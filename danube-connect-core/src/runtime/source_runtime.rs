//! Source Runtime for External System → Danube connectors
//!
//! Handles polling external systems and publishing messages to Danube topics with
//! dynamic multi-producer management.

use crate::{
    ConnectorConfig, ConnectorError, ConnectorMetrics, ConnectorResult, SourceConnector,
    SourceRecord,
};
use danube_client::{DanubeClient, Producer};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use tracing::{debug, error, info};

/// Configuration for a Danube producer
///
/// Specifies how to create a producer for a specific topic, including partitioning
/// and reliability settings.
#[derive(Debug, Clone)]
pub struct ProducerConfig {
    /// Danube topic name (format: /{namespace}/{topic_name})
    pub topic: String,
    /// Number of partitions (0 = non-partitioned)
    pub partitions: usize,
    /// Use reliable dispatch (WAL + Cloud persistence)
    pub reliable_dispatch: bool,
}

/// Runtime for Source Connectors (External System → Danube)
///
/// Manages multiple producers dynamically, creating them on-demand for each unique
/// destination topic. Supports per-topic configuration for partitions and reliability.
pub struct SourceRuntime<C: SourceConnector> {
    connector: C,
    client: DanubeClient,
    producers: HashMap<String, Producer>, // topic -> producer
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

        // Create Danube client
        let client = DanubeClient::builder()
            .service_url(&config.danube_service_url)
            .build()
            .await
            .map_err(|e| ConnectorError::fatal_with_source("Failed to create Danube client", e))?;

        // Create metrics (topic will be set dynamically per producer)
        let metrics = Arc::new(ConnectorMetrics::new(&config.connector_name, "multi-topic"));
        metrics.set_health(true);

        Ok(Self {
            connector,
            client,
            producers: HashMap::new(), // Start with no producers, create dynamically
            config,
            metrics,
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Initialize tracing/logging
    fn init_tracing(config: &ConnectorConfig) {
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.processing.log_level));

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

        // 2. Producers will be created dynamically on-demand based on SourceRecord.topic
        info!("Ready to create producers dynamically per destination topic");

        // 3. Main polling loop
        info!("Entering main polling loop");
        let poll_interval = Duration::from_millis(self.config.processing.poll_interval_ms);

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

    /// Publish a batch of records to their respective topics
    ///
    /// Records can target different topics, and producers will be created dynamically
    /// as needed. Each record's routing key (if present) will be used for partition selection.
    async fn publish_batch(
        &mut self,
        records: Vec<SourceRecord>,
    ) -> ConnectorResult<Vec<crate::traits::Offset>> {
        let mut offsets = Vec::new();

        for (idx, record) in records.into_iter().enumerate() {
            let start = Instant::now();

            // Use producer config from record if provided, otherwise use defaults
            let producer_cfg = record.producer_config.clone().unwrap_or_else(|| {
                ProducerConfig {
                    topic: record.topic.clone(),
                    partitions: 0,            // Default: non-partitioned
                    reliable_dispatch: false, // Default: non-reliable (Danube default)
                }
            });

            // Get or create producer for this topic
            let producer = self.get_or_create_producer(&producer_cfg).await?;

            // Send message with routing key if present
            let send_result = if let Some(key) = &record.key {
                // Use key-based routing (for partitioned topics - will be used when Danube supports it)
                debug!(
                    "Sending message with key: {} to topic: {}",
                    key, record.topic
                );
                // TODO: Use send_with_key when danube-client supports it
                // For now, key is preserved in SourceRecord but not used in actual send
                producer.send(record.payload, Some(record.attributes)).await
            } else {
                producer.send(record.payload, Some(record.attributes)).await
            };

            match send_result {
                Ok(message_id) => {
                    let duration = start.elapsed();
                    self.metrics.record_processing_time(duration);
                    self.metrics.record_success();
                    debug!("Message sent successfully: {}", message_id);

                    // Create offset
                    let offset = crate::traits::Offset::new(record.topic, idx as u64);
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

        Ok(offsets)
    }

    /// Get or create a producer for a specific topic
    ///
    /// This method manages a cache of producers, creating them on-demand with the
    /// specified configuration (partitions, reliable dispatch, etc.)
    async fn get_or_create_producer(
        &mut self,
        producer_cfg: &ProducerConfig,
    ) -> ConnectorResult<&mut Producer> {
        let topic = &producer_cfg.topic;

        // If producer doesn't exist, create it
        if !self.producers.contains_key(topic) {
            info!(
                "Creating producer for topic: {} (partitions: {}, reliable: {})",
                topic, producer_cfg.partitions, producer_cfg.reliable_dispatch
            );

            // Generate producer name: connector_name-topic_name
            let topic_suffix = topic.replace('/', "-");
            let producer_name = format!("{}-{}", self.config.connector_name, topic_suffix);

            let mut producer_builder = self
                .client
                .new_producer()
                .with_topic(topic)
                .with_name(&producer_name);

            // Add partitions if specified
            if producer_cfg.partitions > 0 {
                producer_builder = producer_builder.with_partitions(producer_cfg.partitions);
            }

            // Add reliable dispatch if requested
            if producer_cfg.reliable_dispatch {
                producer_builder = producer_builder.with_reliable_dispatch();
            }

            let mut producer = producer_builder.build();
            producer.create().await.map_err(|e| {
                ConnectorError::fatal_with_source(
                    format!("Failed to create producer for topic {}", topic),
                    e,
                )
            })?;

            info!("Producer created successfully for topic: {}", topic);
            self.producers.insert(topic.clone(), producer);
        }

        // Return mutable reference to the producer
        Ok(self.producers.get_mut(topic).unwrap())
    }
}
