//! Runtime for managing connector lifecycle.
//!
//! The runtime handles:
//! - Connector initialization
//! - Danube client setup and connection management
//! - Message processing loops
//! - Retry logic
//! - Health monitoring
//! - Graceful shutdown

use crate::{
    ConnectorConfig, ConnectorError, ConnectorMetrics, ConnectorResult, RetryConfig, RetryStrategy,
    SinkConnector, SinkRecord, SourceConnector, SourceRecord,
};
use danube_client::{Consumer, DanubeClient, Producer, SubType};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use tracing::{debug, error, info, warn};

/// Runtime for Sink Connectors (Danube → External System)
pub struct SinkRuntime<C: SinkConnector> {
    connector: C,
    client: DanubeClient,
    consumer: Option<Consumer>,
    config: ConnectorConfig,
    metrics: Arc<ConnectorMetrics>,
    retry_strategy: RetryStrategy,
    shutdown: Arc<AtomicBool>,
}

impl<C: SinkConnector> SinkRuntime<C> {
    /// Create a new sink runtime
    pub async fn new(connector: C, config: ConnectorConfig) -> ConnectorResult<Self> {
        // Validate configuration
        config.validate()?;

        // Initialize tracing
        Self::init_tracing(&config);

        info!("Initializing Sink Runtime");
        info!("Connector: {}", config.connector_name);
        info!("Danube URL: {}", config.danube_service_url);
        info!("Topic: {:?}", config.source_topic);
        info!("Subscription: {:?}", config.subscription_name);

        // Create Danube client
        let client = DanubeClient::builder()
            .service_url(&config.danube_service_url)
            .build()
            .await
            .map_err(|e| ConnectorError::fatal_with_source("Failed to create Danube client", e))?;

        // Create metrics
        let topic = config.source_topic.clone().unwrap_or_default();
        let metrics = Arc::new(ConnectorMetrics::new(&config.connector_name, &topic));
        metrics.set_health(true);

        // Create retry strategy
        let retry_strategy = RetryStrategy::new(RetryConfig::new(
            config.max_retries,
            config.retry_backoff_ms,
            config.max_backoff_ms,
        ));

        Ok(Self {
            connector,
            client,
            consumer: None,
            config,
            metrics,
            retry_strategy,
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Initialize tracing/logging
    fn init_tracing(config: &ConnectorConfig) {
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level));

        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    /// Run the sink connector
    pub async fn run(&mut self) -> ConnectorResult<()> {
        info!("Starting Sink Runtime");

        // Setup shutdown handler
        let shutdown = self.shutdown.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for ctrl-c");
            info!("Received shutdown signal");
            shutdown.store(true, Ordering::Relaxed);
        });

        // 1. Initialize connector
        info!("Initializing connector");
        self.connector.initialize(self.config.clone()).await?;
        info!("Connector initialized successfully");

        // 2. Create and subscribe consumer
        let source_topic = self.config.source_topic.as_ref().ok_or_else(|| {
            ConnectorError::config("source_topic is required for sink connectors")
        })?;

        let subscription_name = self.config.subscription_name.as_ref().ok_or_else(|| {
            ConnectorError::config("subscription_name is required for sink connectors")
        })?;

        let subscription_type = self
            .config
            .subscription_type
            .clone()
            .map(|st| st.into())
            .unwrap_or(SubType::Exclusive);

        info!("Creating consumer");
        let mut consumer = self
            .client
            .new_consumer()
            .with_topic(source_topic)
            .with_consumer_name(&self.config.connector_name)
            .with_subscription(subscription_name)
            .with_subscription_type(subscription_type)
            .build();

        consumer
            .subscribe()
            .await
            .map_err(|e| ConnectorError::fatal_with_source("Failed to subscribe to topic", e))?;
        info!("Consumer subscribed successfully");

        // 3. Start receiving messages
        let mut message_stream = consumer
            .receive()
            .await
            .map_err(|e| ConnectorError::fatal_with_source("Failed to start message stream", e))?;
        info!("Message stream started");

        self.consumer = Some(consumer);

        // 4. Main processing loop
        info!("Entering main processing loop");
        while !self.shutdown.load(Ordering::Relaxed) {
            if let Some(message) = message_stream.recv().await {
                self.metrics.record_received();

                let record = SinkRecord::from_stream_message(message.clone(), None);

                debug!(
                    "Processing message: topic={}, offset={}",
                    record.topic(),
                    record.offset()
                );

                // Process with retry logic
                match self.process_with_retry(record).await {
                    Ok(_) => {
                        // Acknowledge successful processing
                        if let Some(ref mut consumer) = self.consumer {
                            if let Err(e) = consumer.ack(&message).await {
                                error!("Failed to acknowledge message: {}", e);
                            } else {
                                self.metrics.record_success();
                                debug!("Message acknowledged");
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to process message after retries: {}", e);
                        self.metrics.record_error(&format!("{:?}", e));
                        // TODO: Implement dead-letter queue logic
                    }
                }
            }
        }

        // 5. Graceful shutdown
        info!("Shutting down connector");
        self.connector.shutdown().await?;
        self.metrics.set_health(false);
        info!("Sink Runtime stopped");

        Ok(())
    }

    /// Process a record with retry logic
    async fn process_with_retry(&mut self, record: SinkRecord) -> ConnectorResult<()> {
        let start = Instant::now();
        let mut attempt = 0;

        loop {
            match self.connector.process(record.clone()).await {
                Ok(_) => {
                    let duration = start.elapsed();
                    self.metrics.record_processing_time(duration);
                    return Ok(());
                }
                Err(e) if e.is_retryable() && self.retry_strategy.should_retry(attempt) => {
                    attempt += 1;
                    self.metrics.record_retry();

                    let backoff = self.retry_strategy.calculate_backoff(attempt);
                    warn!(
                        "Retry attempt {} after {:?} - error: {}",
                        attempt, backoff, e
                    );

                    tokio::time::sleep(backoff).await;
                }
                Err(e) if e.is_invalid_data() => {
                    // Skip invalid messages
                    warn!("Skipping invalid message: {}", e);
                    return Ok(());
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }
}

/// Runtime for Source Connectors (External System → Danube)
pub struct SourceRuntime<C: SourceConnector> {
    connector: C,
    client: DanubeClient,
    producer: Option<Producer>,
    config: ConnectorConfig,
    metrics: Arc<ConnectorMetrics>,
    shutdown: Arc<AtomicBool>,
}

impl<C: SourceConnector> SourceRuntime<C> {
    /// Create a new source runtime
    pub async fn new(connector: C, config: ConnectorConfig) -> ConnectorResult<Self> {
        // Validate configuration
        config.validate()?;

        // Initialize tracing
        Self::init_tracing(&config);

        info!("Initializing Source Runtime");
        info!("Connector: {}", config.connector_name);
        info!("Danube URL: {}", config.danube_service_url);
        info!("Destination Topic: {:?}", config.destination_topic);

        // Create Danube client
        let client = DanubeClient::builder()
            .service_url(&config.danube_service_url)
            .build()
            .await
            .map_err(|e| ConnectorError::fatal_with_source("Failed to create Danube client", e))?;

        // Create metrics
        let topic = config.destination_topic.clone().unwrap_or_default();
        let metrics = Arc::new(ConnectorMetrics::new(&config.connector_name, &topic));
        metrics.set_health(true);

        Ok(Self {
            connector,
            client,
            producer: None,
            config,
            metrics,
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Initialize tracing/logging
    fn init_tracing(config: &ConnectorConfig) {
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level));

        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    /// Run the source connector
    pub async fn run(&mut self) -> ConnectorResult<()> {
        info!("Starting Source Runtime");

        // Setup shutdown handler
        let shutdown = self.shutdown.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for ctrl-c");
            info!("Received shutdown signal");
            shutdown.store(true, Ordering::Relaxed);
        });

        // 1. Initialize connector
        info!("Initializing connector");
        self.connector.initialize(self.config.clone()).await?;
        info!("Connector initialized successfully");

        // 2. Create producer
        let destination_topic = self.config.destination_topic.as_ref().ok_or_else(|| {
            ConnectorError::config("destination_topic is required for source connectors")
        })?;

        info!("Creating producer");
        let mut producer = self
            .client
            .new_producer()
            .with_topic(destination_topic)
            .with_name(&self.config.connector_name);

        // Use reliable dispatch if configured
        if self.config.reliable_dispatch {
            producer = producer.with_reliable_dispatch();
        }

        let mut producer = producer.build();
        producer
            .create()
            .await
            .map_err(|e| ConnectorError::fatal_with_source("Failed to create producer", e))?;
        info!("Producer created successfully");

        self.producer = Some(producer);

        // 3. Main polling loop
        info!("Entering main polling loop");
        let poll_interval = Duration::from_millis(self.config.poll_interval_ms);

        while !self.shutdown.load(Ordering::Relaxed) {
            match self.connector.poll().await {
                Ok(records) if !records.is_empty() => {
                    info!("Polled {} records", records.len());
                    self.metrics.record_batch_size(records.len());

                    // Publish records
                    match self.publish_batch(records).await {
                        Ok(offsets) => {
                            // Commit offsets
                            if let Err(e) = self.connector.commit(offsets).await {
                                error!("Failed to commit offsets: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to publish batch: {}", e);
                            self.metrics.record_error(&format!("{:?}", e));
                        }
                    }
                }
                Ok(_) => {
                    // No data, sleep briefly
                    tokio::time::sleep(poll_interval).await;
                }
                Err(e) => {
                    error!("Poll error: {}", e);
                    self.metrics.record_error(&format!("{:?}", e));
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }

        // 4. Graceful shutdown
        info!("Shutting down connector");
        self.connector.shutdown().await?;
        self.metrics.set_health(false);
        info!("Source Runtime stopped");

        Ok(())
    }

    /// Publish a batch of records
    async fn publish_batch(
        &mut self,
        records: Vec<SourceRecord>,
    ) -> ConnectorResult<Vec<crate::traits::Offset>> {
        let mut offsets = Vec::new();

        if let Some(ref mut producer) = self.producer {
            for (idx, record) in records.iter().enumerate() {
                let start = Instant::now();

                match producer
                    .send(record.payload.clone(), Some(record.attributes.clone()))
                    .await
                {
                    Ok(_) => {
                        let duration = start.elapsed();
                        self.metrics.record_processing_time(duration);
                        self.metrics.record_success();

                        // Create offset
                        let offset = crate::traits::Offset::new(record.topic.clone(), idx as u64);
                        offsets.push(offset);
                    }
                    Err(e) => {
                        error!("Failed to publish message: {}", e);
                        return Err(ConnectorError::retryable_with_source(
                            "Failed to publish batch",
                            e,
                        ));
                    }
                }
            }
        }

        Ok(offsets)
    }
}
