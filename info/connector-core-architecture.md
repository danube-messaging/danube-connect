# Danube Connect Core - Shared Library Architecture

## Overview

The `danube-connect-core` library is the foundation for all Danube connectors. It provides a unified framework that abstracts away the complexity of interacting with Danube brokers, allowing connector developers to focus solely on the integration logic with external systems.

## Architecture Principles

### 1. Layered Design

```text
┌─────────────────────────────────────────────┐
│         Connector Implementation            │
│    (implements SinkConnector/SourceConnector)│
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│      danube-connect-core (SDK Layer)        │
│  ┌─────────────────────────────────────┐   │
│  │   Runtime & Lifecycle Management    │   │
│  └─────────────────────────────────────┘   │
│  ┌─────────────────────────────────────┐   │
│  │   Message Transformation Layer      │   │
│  └─────────────────────────────────────┘   │
│  ┌─────────────────────────────────────┐   │
│  │   Retry & Error Handling            │   │
│  └─────────────────────────────────────┘   │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│         danube-client (RPC Layer)           │
│  ┌─────────────────────────────────────┐   │
│  │  Producer/Consumer API              │   │
│  └─────────────────────────────────────┘   │
│  ┌─────────────────────────────────────┐   │
│  │  gRPC Service Clients               │   │
│  │  (Producer/Consumer/Discovery)      │   │
│  └─────────────────────────────────────┘   │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│            Danube Broker Cluster            │
│   (via gRPC/Protobuf - DanubeApi.proto)    │
└─────────────────────────────────────────────┘
```

### 2. Separation of Concerns

The core library is divided into distinct modules, each with a specific responsibility:

| Module | Responsibility |
|--------|---------------|
| `traits.rs` | Defines the `SinkConnector` and `SourceConnector` trait contracts |
| `runtime.rs` | Manages connector lifecycle, main event loops, and graceful shutdown |
| `client_wrapper.rs` | Wraps danube-client and provides high-level operations |
| `message.rs` | Message transformation utilities between Danube and connector formats |
| `config.rs` | Configuration parsing, validation, and environment variable handling |
| `error.rs` | Unified error types and retry classification |
| `retry.rs` | Exponential backoff, circuit breaker, and retry strategies |
| `metrics.rs` | Prometheus metrics and observability hooks |

## Core Components Deep Dive

### 1. Connector Traits (`traits.rs`)

The trait-based design allows for:
- **Type Safety:** Compile-time guarantees for connector implementations
- **Minimal Interface:** Connectors only implement what's necessary
- **Testability:** Easy to mock for unit testing
- **Flexibility:** Optional methods with sensible defaults

#### Key Design Decisions:

**Async Throughout:** All methods are async to support I/O operations without blocking
```rust
#[async_trait]
pub trait SinkConnector: Send + Sync {
    async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()>;
}
```

**Batching Support:** Optional batch processing for high-throughput scenarios
```rust
async fn process_batch(&mut self, records: Vec<SinkRecord>) -> ConnectorResult<()> {
    // Default implementation processes one by one
    // Connectors can override for bulk operations
}
```

**Graceful Shutdown:** Connectors can cleanup resources
```rust
async fn shutdown(&mut self) -> ConnectorResult<()> {
    // Close connections, flush buffers, etc.
}
```

### 2. Runtime & Lifecycle Management (`runtime.rs`)

The runtime is the heart of the connector framework. It manages:

#### For Sink Connectors:

```rust
pub struct SinkRuntime<C: SinkConnector> {
    connector: C,
    client: DanubeClient,
    consumer: Option<Consumer>,
    config: ConnectorConfig,
    metrics: Arc<ConnectorMetrics>,
    shutdown: Arc<AtomicBool>,
}

impl<C: SinkConnector> SinkRuntime<C> {
    pub async fn run(&mut self) -> ConnectorResult<()> {
        // Lifecycle phases:
        // 1. Initialize connector
        // 2. Create Danube consumer and subscribe
        // 3. Enter message processing loop
        // 4. Handle shutdown signal
        // 5. Cleanup and exit
    }
}
```

**Key Features:**

- **Automatic Reconnection:** If connection to broker is lost, runtime handles reconnection with exponential backoff
- **Health Monitoring:** Periodic health checks to both connector and Danube
- **Metrics Collection:** Automatically tracks messages processed, errors, latency
- **Signal Handling:** Responds to SIGTERM/SIGINT for graceful shutdown

#### For Source Connectors:

```rust
pub struct SourceRuntime<C: SourceConnector> {
    connector: C,
    client: DanubeClient,
    producer: Option<Producer>,
    config: ConnectorConfig,
    metrics: Arc<ConnectorMetrics>,
    shutdown: Arc<AtomicBool>,
    // Offset tracking for exactly-once semantics
    offset_manager: OffsetManager,
}

impl<C: SourceConnector> SourceRuntime<C> {
    pub async fn run(&mut self) -> ConnectorResult<()> {
        // Lifecycle phases:
        // 1. Initialize connector and load checkpoints
        // 2. Create Danube producer
        // 3. Enter polling loop
        // 4. Publish batches and commit offsets
        // 5. Handle shutdown and final checkpoint
    }
}
```

### 3. Message Transformation (`message.rs`)

#### The Transformation Layer

Danube uses a binary `Vec<u8>` payload with optional attributes. Connectors need to:
- **Decode:** Extract typed data from Danube messages
- **Encode:** Convert external system data to Danube format
- **Enrich:** Add metadata and attributes

**Provided Utilities:**

```rust
// Sink Record - from Danube to external system
impl SinkRecord {
    // Binary access
    pub fn payload(&self) -> &[u8];
    
    // UTF-8 string access
    pub fn payload_str(&self) -> ConnectorResult<&str>;
    
    // JSON deserialization
    pub fn payload_json<T: DeserializeOwned>(&self) -> ConnectorResult<T>;
    
    // Attribute access
    pub fn get_attribute(&self, key: &str) -> Option<&str>;
    
    // Message metadata
    pub fn topic(&self) -> &str;
    pub fn offset(&self) -> u64;
    pub fn publish_time(&self) -> u64;
    pub fn producer_name(&self) -> &str;
}

// Source Record - from external system to Danube
impl SourceRecord {
    // Builders
    pub fn new(topic: String, payload: Vec<u8>) -> Self;
    pub fn from_json<T: Serialize>(topic: String, data: &T) -> ConnectorResult<Self>;
    
    // Fluent API for building records
    pub fn with_attribute(self, key: String, value: String) -> Self;
    pub fn with_key(self, key: String) -> Self;
}
```

#### Schema Support

Connectors can leverage Danube's schema registry:

```rust
// For typed messages
let schema = Schema::new("user_events", SchemaType::Json(schema_json));

// Producer creation with schema
let producer = client
    .new_producer()
    .with_schema(schema)
    .build();

// Consumer can fetch schema
let schema = client.get_schema(topic).await?;
```

### 4. Error Handling & Retry Logic (`error.rs`, `retry.rs`)

#### Error Classification

```rust
pub enum ConnectorError {
    // Retryable errors - transient failures
    Retryable {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    // Fatal errors - requires restart or config change
    Fatal {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    // Invalid data - message should be skipped or sent to DLQ
    InvalidData {
        message: String,
        payload: Vec<u8>,
    },
    
    // Configuration error - detected at startup
    Configuration(String),
}

impl ConnectorError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, ConnectorError::Retryable { .. })
    }
}
```

#### Retry Strategy

The core provides configurable retry logic:

```rust
pub struct RetryStrategy {
    max_retries: u32,
    base_delay_ms: u64,
    max_delay_ms: u64,
    multiplier: f64,
    jitter: bool,
}

impl RetryStrategy {
    pub fn exponential_backoff(max_retries: u32) -> Self;
    pub fn linear_backoff(max_retries: u32) -> Self;
    pub fn fixed_delay(max_retries: u32, delay_ms: u64) -> Self;
}
```

**Runtime Retry Behavior:**

```rust
async fn process_with_retry(&mut self, record: SinkRecord) -> ConnectorResult<()> {
    let mut attempt = 0;
    
    loop {
        match self.connector.process(record.clone()).await {
            Ok(()) => {
                self.metrics.record_success();
                return Ok(());
            }
            Err(e) if e.is_retryable() && attempt < self.config.max_retries => {
                attempt += 1;
                let delay = self.retry_strategy.calculate_delay(attempt);
                
                tracing::warn!(
                    "Retry attempt {} after {:?} - error: {}",
                    attempt, delay, e
                );
                
                self.metrics.record_retry();
                tokio::time::sleep(delay).await;
            }
            Err(ConnectorError::InvalidData { payload, .. }) => {
                // Send to dead-letter queue if configured
                self.send_to_dlq(record, e).await?;
                return Ok(()); // Don't block on bad data
            }
            Err(e) => {
                self.metrics.record_error();
                return Err(e);
            }
        }
    }
}
```

### 5. Configuration Management (`config.rs`)

Connectors are configured via:
1. **Environment Variables** (12-factor app style)
2. **YAML/TOML files**
3. **Command-line arguments**

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectorConfig {
    // Danube connection
    pub danube_service_url: String,
    pub connector_name: String,
    
    // Sink-specific
    pub source_topic: Option<String>,
    pub subscription_name: Option<String>,
    pub subscription_type: Option<SubType>,
    
    // Source-specific
    pub destination_topic: Option<String>,
    pub dispatch_strategy: Option<DispatchStrategy>,
    
    // Runtime configuration
    pub max_retries: u32,
    pub retry_backoff_ms: u64,
    pub batch_size: usize,
    pub batch_timeout_ms: u64,
    pub poll_interval_ms: u64,
    
    // Observability
    pub metrics_port: u16,
    pub log_level: String,
    
    // Connector-specific config (flattened)
    #[serde(flatten)]
    pub connector_config: HashMap<String, String>,
}

impl ConnectorConfig {
    /// Load from environment variables with CONNECTOR_ prefix
    pub fn from_env() -> ConnectorResult<Self>;
    
    /// Load from YAML file
    pub fn from_file(path: &str) -> ConnectorResult<Self>;
    
    /// Validate configuration
    pub fn validate(&self) -> ConnectorResult<()>;
}
```

### 6. Observability (`metrics.rs`)

Built-in metrics using Prometheus:

```rust
pub struct ConnectorMetrics {
    // Message counters
    messages_received: Counter,
    messages_processed: Counter,
    messages_failed: Counter,
    messages_retried: Counter,
    
    // Latency histograms
    processing_duration: Histogram,
    danube_publish_duration: Histogram,
    
    // Gauges
    inflight_messages: Gauge,
    connector_health: Gauge,
    
    // Connector-specific labels
    connector_name: String,
    topic: String,
}

impl ConnectorMetrics {
    pub fn record_received(&self);
    pub fn record_success(&self);
    pub fn record_error(&self);
    pub fn record_retry(&self);
    pub fn record_processing_time(&self, duration: Duration);
}
```

**Exposed Metrics:**

- `danube_connector_messages_received_total{connector, topic}`
- `danube_connector_messages_processed_total{connector, topic}`
- `danube_connector_messages_failed_total{connector, topic, error_type}`
- `danube_connector_processing_duration_seconds{connector, quantile}`
- `danube_connector_inflight_messages{connector}`

## Integration with danube-client

The core library wraps `danube-client` to provide:

### 1. Automatic Topic Discovery

```rust
// Connectors don't need to manually lookup brokers
let partitions = self.client.lookup_service
    .topic_partitions(&service_url, &topic)
    .await?;
```

### 2. Health Check Integration

```rust
// Periodic health checks ensure connection is alive
pub async fn check_connection_health(&self) -> ConnectorResult<()> {
    match self.producer.as_ref() {
        Some(producer) => {
            // Use danube-client's health check RPC
            producer.health_check().await
                .map_err(|e| ConnectorError::Retryable {
                    message: format!("Health check failed: {}", e),
                    source: Some(Box::new(e)),
                })
        }
        None => Err(ConnectorError::Fatal {
            message: "Producer not initialized".to_string(),
            source: None,
        }),
    }
}
```

### 3. Reliable Dispatch for Source Connectors

```rust
// Source connectors use reliable dispatch by default
let mut producer = self.client
    .new_producer()
    .with_topic(&self.config.destination_topic)
    .with_name(&self.config.connector_name)
    .with_reliable_dispatch() // Ensures messages are persisted to WAL + Cloud
    .build();
```

### 4. Subscription Management for Sink Connectors

```rust
// Sink connectors can use different subscription types
let mut consumer = self.client
    .new_consumer()
    .with_topic(&self.config.source_topic)
    .with_consumer_name(&self.config.connector_name)
    .with_subscription(&self.config.subscription_name)
    .with_subscription_type(SubType::Exclusive) // or Shared, Failover
    .build();
```

## Usage Example: Implementing a Simple Connector

```rust
use danube_connect_core::{
    SinkConnector, SinkRecord, ConnectorConfig, ConnectorResult, ConnectorError
};

pub struct HttpSinkConnector {
    target_url: String,
    http_client: reqwest::Client,
}

#[async_trait]
impl SinkConnector for HttpSinkConnector {
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()> {
        self.target_url = config.connector_config
            .get("TARGET_URL")
            .ok_or(ConnectorError::Configuration("TARGET_URL required".into()))?
            .clone();
        
        self.http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| ConnectorError::Fatal {
                message: format!("Failed to create HTTP client: {}", e),
                source: Some(Box::new(e)),
            })?;
        
        Ok(())
    }
    
    async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
        let response = self.http_client
            .post(&self.target_url)
            .body(record.payload().to_vec())
            .send()
            .await
            .map_err(|e| ConnectorError::Retryable {
                message: format!("HTTP request failed: {}", e),
                source: Some(Box::new(e)),
            })?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            Err(ConnectorError::Retryable {
                message: format!("HTTP {} from {}", response.status(), self.target_url),
                source: None,
            })
        }
    }
}

// main.rs
#[tokio::main]
async fn main() -> ConnectorResult<()> {
    let config = ConnectorConfig::from_env()?;
    let connector = HttpSinkConnector::new();
    
    // The runtime handles everything else!
    let mut runtime = danube_connect_core::SinkRuntime::new(connector, config).await?;
    runtime.run().await
}
```

## Benefits of the Shared Core Approach

1. **Consistency:** All connectors behave predictably
2. **Reduced Boilerplate:** Connector code is 90% business logic, 10% infrastructure
3. **Automatic Updates:** Bug fixes and improvements benefit all connectors
4. **Testing Support:** Mock implementations for unit tests
5. **Performance:** Optimized retry, batching, and connection pooling
6. **Observability:** Standardized metrics and logging

## Future Enhancements

- **Dead Letter Queue:** Automatic DLQ support for failed messages
- **Schema Evolution:** Automatic schema compatibility checking
- **Exactly-Once Semantics:** Transactional support for source connectors
- **Dynamic Configuration:** Hot-reload of connector configuration
- **Multi-Tenancy:** Support for multiple topics/subscriptions in one connector
