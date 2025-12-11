# Danube Connect Ecosystem - Architecture & Design

**Status:** Draft | **Target:** v0.1 | **Focus:** Architecture & Initial Connectors

## 1. Executive Summary

The goal of **Danube Connect** is to provide a "batteries-included" ecosystem for Danube Messaging without compromising the safety, stability, or binary size of the core broker.

Instead of embedding integrations into the broker (monolithic), we will adopt a **Connect Worker Pattern**. Connectors will be standalone, memory-safe **Rust binaries** that utilize a shared core library to communicate with the Danube Cluster via the **danube-client** RPC protocol.

## 2. Design Philosophy

### 2.1 Isolation & Safety

* **No FFI in Core:** The Broker must remain pure Rust and "dumb." It should not be aware of external systems like AWS, Postgres, or Kafka.
* **Crash Isolation:** A panic in a poorly configured Postgres Connector must never crash the Message Broker. Running connectors as separate processes guarantees this isolation.
* **Process-Level Boundaries:** Each connector runs as an independent process, ensuring fault isolation and independent lifecycle management.

### 2.2 The "Shared Core" Library (`danube-connect-core`)

We will create a library crate (`danube-connect-core`) that acts as the connector SDK. Connector developers will only implement simple Traits (`SourceConnector`, `SinkConnector`). The shared core handles:

**Core Responsibilities:**

* **Danube Client Integration:** Wraps the `danube-client` crate to handle RPC communication with brokers
* **Message Transformation:** Converts between external system formats and Danube's `StreamMessage` protocol buffers
* **Connection Management:** Automatic reconnection, health checks, and broker discovery via the Discovery service
* **Backoff and Retry Logic:** Configurable exponential backoff for failed operations
* **Graceful Shutdowns:** Clean resource cleanup and inflight message handling
* **Metrics and Logging:** Built-in observability with tracing and prometheus metrics
* **Configuration Management:** Standard YAML/TOML config parsing and validation
* **Error Handling:** Unified error types and recovery strategies

## 3. Repository Structure

We will use a **separate repository** from the main broker to allow for faster iteration cycles and community contributions.

**Repository:** `github.com/danube-messaging/danube-connect`

**Workspace Layout:**

```text
/danube-connect                     (Cargo Workspace)
├── Cargo.toml                      (Workspace configuration)
├── README.md                       (Main documentation)
├── LICENSE
│
├── /danube-connect-core            (The Shared SDK Library)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                 (Public API exports)
│   │   ├── traits.rs              (SinkConnector, SourceConnector traits)
│   │   ├── runtime.rs             (Connector lifecycle and main loop)
│   │   ├── client_wrapper.rs      (Danube client integration)
│   │   ├── message.rs             (Message transformation utilities)
│   │   ├── config.rs              (Configuration management)
│   │   ├── error.rs               (Unified error types)
│   │   ├── retry.rs               (Retry and backoff strategies)
│   │   └── metrics.rs             (Observability and metrics)
│   └── examples/                   (Example connector implementations)
│
├── /danube-connect-common          (Shared utilities)
│   ├── src/
│   │   ├── serialization.rs       (JSON, Avro, Protobuf helpers)
│   │   ├── batching.rs            (Message batching utilities)
│   │   └── health.rs              (Health check implementations)
│
└── /connectors                     (Standalone Connector Binaries)
    ├── /sink-http                  (HTTP/Webhook Sink)
    │   ├── Cargo.toml
    │   ├── src/main.rs
    │   ├── config.yaml.example
    │   └── README.md
    │
    ├── /sink-clickhouse            (ClickHouse Analytics Sink)
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── main.rs
    │   │   ├── batch_writer.rs    (Optimized batch writes)
    │   │   └── schema_mapper.rs   (Type conversions)
    │   └── README.md
    │
    ├── /source-postgres            (PostgreSQL CDC Source)
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── main.rs
    │   │   ├── replication.rs     (WAL logical replication)
    │   │   └── snapshot.rs        (Initial table snapshots)
    │   └── README.md
    │
    ├── /source-mqtt                (MQTT IoT Bridge)
    │   └── ...
    │
    └── /bridge-kafka               (Kafka Bridge)
        └── ...
```

-----

## 4\. The First 5 Priority Connectors

### 4.1. HTTP / Webhook Sink ("The Universal Glue")

**Type:** Sink
**Purpose:** Pushes Danube messages to any external API (Slack, Serverless functions, Legacy apps).

  * **Logic:** Listens to a topic and performs a `POST` request to a configured target URL.
  * **Configuration:**
      * `TARGET_URL`: The endpoint to hit.
      * `RETRY_POLICY`: Linear or Exponential backoff.
      * `TIMEOUT_MS`: Max time to wait for HTTP 200 OK.
  * **Why First?** It immediately makes Danube compatible with Python, Node, and Ruby apps via Webhooks.

### 4.2. Postgres CDC Source ("The Data Liberator")

**Type:** Source
**Purpose:** Streams changes from a SQL database into Danube in real-time (Change Data Capture).

  * **Logic:** Connects to the Postgres Write-Ahead Log (WAL) via Logical Replication.
  * **Output Format:** JSON payload describing the change:
    ```json
    { "op": "INSERT", "table": "users", "after": { "id": 1, "name": "Alice" } }
    ```
  * **Why First?** It unlocks the massive market of companies wanting to move from "Monolith Database" to "Event Driven Architecture."

### 4.3. ClickHouse Sink ("The Speed Layer")

**Type:** Sink
**Purpose:** High-performance ingestion for real-time analytics.

  * **Logic:**
    1.  Consumes messages from Danube.
    2.  **Buffers** them in memory (e.g., up to 10,000 records or 1 second).
    3.  Performs a bulk `INSERT` into ClickHouse via HTTP or Native TCP.
  * **Critical Feature:** **Batching.** Inserting one row at a time is too slow for analytics; the connector *must* buffer.
  * **Why First?** Rust and ClickHouse share a user base (performance obsessions). This creates a "Best in Class" high-speed data stack.

### 4.4. MQTT Source/Bridge ("The IoT Gateway")

**Type:** Source (Bridge)
**Purpose:** Allows IoT devices to write to Danube without implementing the Danube Protocol.

  * **Logic:**
      * Runs an embedded MQTT Server (or connects to one).
      * Maps MQTT topics (e.g., `factory/temp`) to Danube topics (e.g., `/tenant/factory/temp`).
  * **Why First?** IoT generates massive message volume, perfect for Danube's performance, but devices rarely support custom protocols.

### 4.5. Kafka Bridge ("The Migrator")

**Type:** Source & Sink (Bi-directional)
**Purpose:** Risk-free migration for Enterprise users.

  * **Mode A (Import):** Consumes from Kafka `Topic A` -\> Produces to Danube `Topic A`.
  * **Mode B (Export):** Consumes from Danube `Topic B` -\> Produces to Kafka `Topic B`.
  * **Why First?** No enterprise will replace Kafka overnight. They need to run Danube side-by-side. This bridge allows the "Strangler Fig" migration pattern.

-----

## 5. Technical Implementation Specs

### 5.1 Understanding Danube's RPC Communication

Connectors communicate with Danube brokers via **gRPC** using the protocol defined in `danube-core/proto/DanubeApi.proto`. The `danube-client` crate provides a high-level Rust API that wraps these RPC services:

**Key RPC Services:**

* **ProducerService:** Create producers and send messages
  * `CreateProducer(ProducerRequest) -> ProducerResponse`
  * `SendMessage(StreamMessage) -> MessageResponse`
  
* **ConsumerService:** Subscribe and receive messages
  * `Subscribe(ConsumerRequest) -> ConsumerResponse`
  * `ReceiveMessages(ReceiveRequest) -> stream StreamMessage`
  * `Ack(AckRequest) -> AckResponse`
  
* **Discovery:** Topic lookup and partition information
  * `TopicLookup(TopicLookupRequest) -> TopicLookupResponse`
  * `TopicPartitions(TopicLookupRequest) -> TopicPartitionsResponse`
  * `GetSchema(SchemaRequest) -> SchemaResponse`

* **HealthCheck:** Monitor client health
  * `HealthCheck(HealthCheckRequest) -> HealthCheckResponse`

### 5.2 Danube Message Format

The `StreamMessage` is the core data structure for messages flowing through Danube:

```protobuf
message StreamMessage {
    uint64 request_id = 1;              // Unique ID for tracking the message request
    MsgID msg_id = 2;                   // Message identifier with routing info
    bytes payload = 3;                  // The actual message payload
    uint64 publish_time = 4;            // Unix timestamp when published
    string producer_name = 5;           // Name of the producer
    string subscription_name = 6;        // Subscription name for consumer routing
    map<string, string> attributes = 7; // User-defined metadata
}

message MsgID {
    uint64 producer_id = 1;    // Producer identifier within a topic
    string topic_name = 2;     // Topic the message belongs to
    string broker_addr = 3;    // Broker that delivered the message
    uint64 topic_offset = 4;   // Monotonic offset within the topic
}
```

**Field Ownership:**

* **Client-Created Fields:** `request_id`, `payload`, `producer_name`, `attributes`
* **Broker-Completed Fields:** `msg_id` (fully populated), `publish_time`, `subscription_name`

The `msg_id` is critical for acknowledgment routing, ensuring messages are acked to the correct broker and subscription.

### 5.3 The Connector Trait Definitions

Located in `danube-connect-core/src/traits.rs`.

```rust
use async_trait::async_trait;
use danube_core::message::StreamMessage;
use std::collections::HashMap;

/// Result type for connector operations
pub type ConnectorResult<T> = Result<T, ConnectorError>;

/// Record passed to sink connectors (from Danube -> External System)
#[derive(Debug, Clone)]
pub struct SinkRecord {
    /// The Danube StreamMessage
    pub message: StreamMessage,
    /// The topic partition this message came from
    pub partition: Option<String>,
    /// Additional connector-specific metadata
    pub metadata: HashMap<String, String>,
}

/// Record passed from source connectors (External System -> Danube)
#[derive(Debug, Clone)]
pub struct SourceRecord {
    /// The topic to publish to
    pub topic: String,
    /// The message payload
    pub payload: Vec<u8>,
    /// Optional message attributes/headers
    pub attributes: HashMap<String, String>,
    /// Optional key for partitioned topics
    pub key: Option<String>,
}

/// Trait for implementing Sink Connectors (Danube -> External System)
#[async_trait]
pub trait SinkConnector: Send + Sync {
    /// Initialize the connector with configuration
    /// Called once at startup before processing begins
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()>;
    
    /// Process a single message from Danube
    /// Ok(()) = Ack message. Err(e) = Nack/Retry based on error type
    async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()>;
    
    /// Optional: Process a batch of messages for better throughput
    /// Default implementation calls process() for each record
    async fn process_batch(&mut self, records: Vec<SinkRecord>) -> ConnectorResult<()> {
        for record in records {
            self.process(record).await?;
        }
        Ok(())
    }
    
    /// Optional: Called before shutdown for cleanup
    async fn shutdown(&mut self) -> ConnectorResult<()> {
        Ok(())
    }
    
    /// Optional: Health check implementation
    async fn health_check(&self) -> ConnectorResult<()> {
        Ok(())
    }
}

/// Trait for implementing Source Connectors (External System -> Danube)
#[async_trait]
pub trait SourceConnector: Send + Sync {
    /// Initialize the connector with configuration
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()>;
    
    /// Poll for new data from the external system
    /// Returns a vector of records to publish to Danube
    /// Empty vector means no data available (non-blocking)
    async fn poll(&mut self) -> ConnectorResult<Vec<SourceRecord>>;
    
    /// Optional: Commit offset/checkpoint after successful publish
    /// Called by the runtime after messages are acked by Danube
    async fn commit(&mut self, offsets: Vec<Offset>) -> ConnectorResult<()> {
        Ok(())
    }
    
    /// Optional: Called before shutdown
    async fn shutdown(&mut self) -> ConnectorResult<()> {
        Ok(())
    }
    
    /// Optional: Health check implementation
    async fn health_check(&self) -> ConnectorResult<()> {
        Ok(())
    }
}
```

### 5.4 Message Transformation in the Shared Core

The `danube-connect-core` provides utilities for transforming messages between Danube's format and connector-specific formats:

```rust
// danube-connect-core/src/message.rs

impl SinkRecord {
    /// Convert Danube StreamMessage to SinkRecord
    pub fn from_stream_message(message: StreamMessage, partition: Option<String>) -> Self {
        SinkRecord {
            message,
            partition,
            metadata: HashMap::new(),
        }
    }
    
    /// Get the payload as bytes
    pub fn payload(&self) -> &[u8] {
        &self.message.payload
    }
    
    /// Get the payload as a UTF-8 string (if valid)
    pub fn payload_str(&self) -> ConnectorResult<&str> {
        std::str::from_utf8(&self.message.payload)
            .map_err(|e| ConnectorError::InvalidPayload(e.to_string()))
    }
    
    /// Get the payload as JSON
    pub fn payload_json<T: DeserializeOwned>(&self) -> ConnectorResult<T> {
        serde_json::from_slice(&self.message.payload)
            .map_err(|e| ConnectorError::InvalidPayload(e.to_string()))
    }
    
    /// Access message attributes
    pub fn attributes(&self) -> &HashMap<String, String> {
        &self.message.attributes
    }
    
    /// Get message ID for logging/tracking
    pub fn message_id(&self) -> String {
        format!(
            "topic:{}/producer:{}/offset:{}",
            self.message.msg_id.topic_name,
            self.message.msg_id.producer_id,
            self.message.msg_id.topic_offset
        )
    }
}

impl SourceRecord {
    /// Create a new SourceRecord with payload
    pub fn new(topic: impl Into<String>, payload: Vec<u8>) -> Self {
        SourceRecord {
            topic: topic.into(),
            payload,
            attributes: HashMap::new(),
            key: None,
        }
    }
    
    /// Create from a JSON-serializable object
    pub fn from_json<T: Serialize>(topic: impl Into<String>, data: &T) -> ConnectorResult<Self> {
        let payload = serde_json::to_vec(data)
            .map_err(|e| ConnectorError::SerializationError(e.to_string()))?;
        Ok(Self::new(topic, payload))
    }
    
    /// Add an attribute
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
    
    /// Set the routing key for partitioned topics
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }
}
```

### 5.5 The Connector Runtime

The `danube-connect-core` provides a runtime that manages the connector lifecycle:

```rust
// danube-connect-core/src/runtime.rs

pub struct ConnectorRuntime<C> {
    connector: C,
    client: DanubeClient,
    config: ConnectorConfig,
    shutdown_signal: Arc<AtomicBool>,
}

impl<C: SinkConnector> ConnectorRuntime<C> {
    pub async fn run_sink(&mut self) -> ConnectorResult<()> {
        // 1. Initialize connector
        self.connector.initialize(self.config.clone()).await?;
        
        // 2. Create Danube consumer
        let mut consumer = self.client
            .new_consumer()
            .with_topic(&self.config.source_topic)
            .with_consumer_name(&self.config.connector_name)
            .with_subscription(&self.config.subscription_name)
            .with_subscription_type(self.config.subscription_type)
            .build();
        
        consumer.subscribe().await?;
        
        // 3. Start receiving messages
        let mut message_stream = consumer.receive().await?;
        
        // 4. Main processing loop
        while !self.shutdown_signal.load(Ordering::Relaxed) {
            if let Some(message) = message_stream.recv().await {
                let record = SinkRecord::from_stream_message(message.clone(), None);
                
                // Process with retry logic
                match self.process_with_retry(record).await {
                    Ok(_) => {
                        // Acknowledge successful processing
                        consumer.ack(&message).await?;
                    }
                    Err(e) => {
                        // Handle error based on severity
                        tracing::error!("Failed to process message: {}", e);
                        // Implement dead-letter queue or error topic logic
                    }
                }
            }
        }
        
        // 5. Graceful shutdown
        self.connector.shutdown().await?;
        Ok(())
    }
    
    async fn process_with_retry(&mut self, record: SinkRecord) -> ConnectorResult<()> {
        // Exponential backoff retry logic
        let mut retries = 0;
        let max_retries = self.config.max_retries;
        
        loop {
            match self.connector.process(record.clone()).await {
                Ok(_) => return Ok(()),
                Err(e) if e.is_retryable() && retries < max_retries => {
                    retries += 1;
                    let backoff = self.calculate_backoff(retries);
                    tokio::time::sleep(backoff).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

impl<C: SourceConnector> ConnectorRuntime<C> {
    pub async fn run_source(&mut self) -> ConnectorResult<()> {
        // 1. Initialize connector
        self.connector.initialize(self.config.clone()).await?;
        
        // 2. Create Danube producer
        let mut producer = self.client
            .new_producer()
            .with_topic(&self.config.destination_topic)
            .with_name(&self.config.connector_name)
            .with_reliable_dispatch() // Use reliable mode by default
            .build();
        
        producer.create().await?;
        
        // 3. Main polling loop
        while !self.shutdown_signal.load(Ordering::Relaxed) {
            // Poll for records from external system
            match self.connector.poll().await {
                Ok(records) if !records.is_empty() => {
                    // Publish records to Danube
                    let offsets = self.publish_batch(&mut producer, records).await?;
                    
                    // Commit offsets in source system
                    self.connector.commit(offsets).await?;
                }
                Ok(_) => {
                    // No data, sleep briefly before next poll
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(e) => {
                    tracing::error!("Poll error: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
        
        // 4. Graceful shutdown
        self.connector.shutdown().await?;
        Ok(())
    }
}
```

### 5.6 Deployment Strategy (Docker)

Users will compose their system using Docker. Each connector is a container.

```yaml
# Example docker-compose.yml
services:
  # Danube broker cluster
  danube-broker:
    image: danube-messaging/broker:latest
    ports:
      - "6650:6650"  # RPC port
    environment:
      ETCD_ENDPOINTS: "etcd:2379"
      STORAGE_PROVIDER: "s3"
      AWS_REGION: "us-east-1"
    depends_on:
      - etcd
      - minio
  
  etcd:
    image: quay.io/coreos/etcd:latest
    ports:
      - "2379:2379"
  
  minio:
    image: minio/minio:latest
    ports:
      - "9000:9000"
    environment:
      MINIO_ROOT_USER: admin
      MINIO_ROOT_PASSWORD: password
  
  # Postgres CDC Source Connector
  connector-postgres-cdc:
    image: danube-connect/source-postgres:v0.1
    environment:
      # Danube connection
      DANUBE_SERVICE_URL: "http://danube-broker:6650"
      DANUBE_TOPIC: "/default/postgres-changes"
      CONNECTOR_NAME: "postgres-cdc-1"
      
      # PostgreSQL connection
      PG_HOST: "postgres-db"
      PG_PORT: "5432"
      PG_DATABASE: "myapp"
      PG_USER: "replicator"
      PG_PASSWORD: "secret"
      
      # CDC configuration
      SLOT_NAME: "danube_cdc_slot"
      PUBLICATION_NAME: "danube_publication"
      TABLE_WHITELIST: "users,orders,products"
      
      # Runtime configuration
      POLL_INTERVAL_MS: "100"
      BATCH_SIZE: "1000"
    depends_on:
      - danube-broker
      - postgres-db
  
  # ClickHouse Analytics Sink Connector
  connector-clickhouse-sink:
    image: danube-connect/sink-clickhouse:v0.1
    environment:
      # Danube connection (consumer)
      DANUBE_SERVICE_URL: "http://danube-broker:6650"
      DANUBE_TOPIC: "/default/analytics-events"
      SUBSCRIPTION_NAME: "clickhouse-sink"
      SUBSCRIPTION_TYPE: "Exclusive"
      CONNECTOR_NAME: "clickhouse-sink-1"
      
      # ClickHouse connection
      CH_HOST: "clickhouse"
      CH_PORT: "9000"
      CH_DATABASE: "analytics"
      CH_TABLE: "events"
      CH_USER: "default"
      CH_PASSWORD: ""
      
      # Batching configuration
      BATCH_SIZE: "10000"
      BATCH_TIMEOUT_MS: "1000"
      FLUSH_INTERVAL_MS: "5000"
    depends_on:
      - danube-broker
      - clickhouse
  
  # HTTP Webhook Sink Connector
  connector-http-sink:
    image: danube-connect/sink-http:v0.1
    environment:
      DANUBE_SERVICE_URL: "http://danube-broker:6650"
      DANUBE_TOPIC: "/default/notifications"
      SUBSCRIPTION_NAME: "webhook-sink"
      CONNECTOR_NAME: "webhook-1"
      
      # HTTP configuration
      TARGET_URL: "https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
      HTTP_METHOD: "POST"
      TIMEOUT_MS: "5000"
      
      # Retry configuration
      MAX_RETRIES: "3"
      RETRY_BACKOFF_MS: "1000"
    depends_on:
      - danube-broker
```

## 6\. Roadmap

1.  **Phase 1: Foundation (Weeks 1-2)**

      * Initialize `danube-connect` repository.
      * Implement `core` crate (Runtime, Traits, Error Handling).
      * CI/CD pipeline setup.

2.  **Phase 2: The Proof of Concept (Weeks 3-4)**

      * Build **HTTP Sink**.
      * Prove the end-to-end flow: Producer -\> Broker -\> HTTP Sink -\> RequestBin.

3.  **Phase 3: The Heavy Lifters (Month 2)**

      * Build **Postgres Source** (CDC).
      * Build **Kafka Bridge**.

4.  **Phase 4: Optimization (Month 3)**

      * Implement **ClickHouse Sink** with batching logic.
      * Benchmark memory usage of connectors.