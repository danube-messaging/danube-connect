# Danube Connector Development Guide

## Quick Start

This guide walks you through creating a new Danube connector from scratch. By the end, you'll have a working connector that integrates an external system with Danube.

## Prerequisites

- Rust 1.70+ installed
- Access to a running Danube cluster (or use Docker Compose setup)
- Basic understanding of async Rust and tokio

## Step 1: Setup Your Connector Project

```bash
# In the danube-connect repository
cd connectors

# Create a new connector (example: MongoDB sink)
cargo new --bin sink-mongodb
cd sink-mongodb
```

### Update Cargo.toml

```toml
[package]
name = "danube-sink-mongodb"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core connector framework
danube-connect-core = { path = "../../danube-connect-core" }
danube-core = { git = "https://github.com/danube-messaging/danube" }

# Async runtime
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# MongoDB driver
mongodb = "2.8"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling
anyhow = "1"
thiserror = "1"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Configuration
config = "0.13"
```

## Step 2: Define Your Connector Configuration

```rust
// src/config.rs

use serde::Deserialize;
use danube_connect_core::ConnectorResult;

#[derive(Debug, Clone, Deserialize)]
pub struct MongoDBSinkConfig {
    // MongoDB connection settings
    pub connection_string: String,
    pub database: String,
    pub collection: String,
    
    // Write options
    pub batch_size: usize,
    pub batch_timeout_ms: u64,
    pub ordered_inserts: bool,
    
    // Schema mapping
    pub id_field: Option<String>,
    pub timestamp_field: Option<String>,
}

impl MongoDBSinkConfig {
    pub fn from_env() -> ConnectorResult<Self> {
        let connection_string = std::env::var("MONGODB_CONNECTION_STRING")
            .map_err(|_| danube_connect_core::ConnectorError::Configuration(
                "MONGODB_CONNECTION_STRING is required".into()
            ))?;
        
        let database = std::env::var("MONGODB_DATABASE")
            .map_err(|_| danube_connect_core::ConnectorError::Configuration(
                "MONGODB_DATABASE is required".into()
            ))?;
        
        let collection = std::env::var("MONGODB_COLLECTION")
            .map_err(|_| danube_connect_core::ConnectorError::Configuration(
                "MONGODB_COLLECTION is required".into()
            ))?;
        
        let batch_size = std::env::var("BATCH_SIZE")
            .unwrap_or_else(|_| "1000".to_string())
            .parse()
            .unwrap_or(1000);
        
        let batch_timeout_ms = std::env::var("BATCH_TIMEOUT_MS")
            .unwrap_or_else(|_| "1000".to_string())
            .parse()
            .unwrap_or(1000);
        
        let ordered_inserts = std::env::var("ORDERED_INSERTS")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);
        
        Ok(Self {
            connection_string,
            database,
            collection,
            batch_size,
            batch_timeout_ms,
            ordered_inserts,
            id_field: std::env::var("ID_FIELD").ok(),
            timestamp_field: std::env::var("TIMESTAMP_FIELD").ok(),
        })
    }
    
    pub fn validate(&self) -> ConnectorResult<()> {
        if self.batch_size == 0 {
            return Err(danube_connect_core::ConnectorError::Configuration(
                "batch_size must be greater than 0".into()
            ));
        }
        Ok(())
    }
}
```

## Step 3: Implement the Connector

### For a Sink Connector (Danube → External System)

```rust
// src/connector.rs

use async_trait::async_trait;
use danube_connect_core::{
    SinkConnector, SinkRecord, ConnectorConfig, ConnectorResult, ConnectorError
};
use mongodb::{Client, Collection, bson::{doc, Document}};
use std::time::Duration;
use tracing::{info, warn, error};

pub struct MongoDBSinkConnector {
    config: MongoDBSinkConfig,
    client: Option<Client>,
    collection: Option<Collection<Document>>,
    batch_buffer: Vec<Document>,
    last_flush: tokio::time::Instant,
}

impl MongoDBSinkConnector {
    pub fn new() -> Self {
        Self {
            config: MongoDBSinkConfig::default(),
            client: None,
            collection: None,
            batch_buffer: Vec::new(),
            last_flush: tokio::time::Instant::now(),
        }
    }
    
    async fn flush_batch(&mut self) -> ConnectorResult<()> {
        if self.batch_buffer.is_empty() {
            return Ok(());
        }
        
        let collection = self.collection.as_ref()
            .ok_or_else(|| ConnectorError::Fatal {
                message: "Collection not initialized".into(),
                source: None,
            })?;
        
        let docs_to_insert = std::mem::take(&mut self.batch_buffer);
        let count = docs_to_insert.len();
        
        info!("Flushing batch of {} documents to MongoDB", count);
        
        let result = collection
            .insert_many(docs_to_insert, None)
            .await
            .map_err(|e| ConnectorError::Retryable {
                message: format!("Failed to insert batch: {}", e),
                source: Some(Box::new(e)),
            })?;
        
        info!("Successfully inserted {} documents", result.inserted_ids.len());
        self.last_flush = tokio::time::Instant::now();
        
        Ok(())
    }
    
    fn should_flush(&self) -> bool {
        self.batch_buffer.len() >= self.config.batch_size ||
        self.last_flush.elapsed() > Duration::from_millis(self.config.batch_timeout_ms)
    }
    
    fn message_to_document(&self, record: &SinkRecord) -> ConnectorResult<Document> {
        // Try to parse as JSON first
        let mut doc: Document = if let Ok(json_value) = record.payload_json::<serde_json::Value>() {
            mongodb::bson::to_document(&json_value)
                .map_err(|e| ConnectorError::InvalidData {
                    message: format!("Failed to convert JSON to BSON: {}", e),
                    payload: record.payload().to_vec(),
                })?
        } else {
            // If not JSON, store as raw bytes with metadata
            doc! {
                "payload": mongodb::bson::Binary {
                    subtype: mongodb::bson::spec::BinarySubtype::Generic,
                    bytes: record.payload().to_vec(),
                }
            }
        };
        
        // Add Danube metadata
        doc.insert("_danube_topic", &record.message.msg_id.topic_name);
        doc.insert("_danube_offset", record.message.msg_id.topic_offset as i64);
        doc.insert("_danube_producer", &record.message.producer_name);
        doc.insert("_danube_publish_time", record.message.publish_time as i64);
        
        // Add custom ID field if configured
        if let Some(id_field) = &self.config.id_field {
            if let Some(id_value) = record.message.attributes.get(id_field) {
                doc.insert("_id", id_value);
            }
        }
        
        // Add timestamp field if configured
        if let Some(ts_field) = &self.config.timestamp_field {
            doc.insert(ts_field, record.message.publish_time as i64);
        }
        
        Ok(doc)
    }
}

#[async_trait]
impl SinkConnector for MongoDBSinkConnector {
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()> {
        info!("Initializing MongoDB Sink Connector");
        
        // Load MongoDB-specific configuration
        self.config = MongoDBSinkConfig::from_env()?;
        self.config.validate()?;
        
        info!("Connecting to MongoDB at {}", self.config.connection_string);
        
        // Create MongoDB client
        let client = Client::with_uri_str(&self.config.connection_string)
            .await
            .map_err(|e| ConnectorError::Fatal {
                message: format!("Failed to connect to MongoDB: {}", e),
                source: Some(Box::new(e)),
            })?;
        
        // Get database and collection
        let database = client.database(&self.config.database);
        let collection = database.collection(&self.config.collection);
        
        // Test connection
        database.list_collection_names(None)
            .await
            .map_err(|e| ConnectorError::Fatal {
                message: format!("Failed to list collections: {}", e),
                source: Some(Box::new(e)),
            })?;
        
        info!(
            "Successfully connected to MongoDB database '{}', collection '{}'",
            self.config.database, self.config.collection
        );
        
        self.client = Some(client);
        self.collection = Some(collection);
        self.batch_buffer = Vec::with_capacity(self.config.batch_size);
        
        Ok(())
    }
    
    async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
        // Convert message to MongoDB document
        let document = self.message_to_document(&record)?;
        
        // Add to batch buffer
        self.batch_buffer.push(document);
        
        // Flush if batch is full or timeout reached
        if self.should_flush() {
            self.flush_batch().await?;
        }
        
        Ok(())
    }
    
    async fn process_batch(&mut self, records: Vec<SinkRecord>) -> ConnectorResult<()> {
        // For batching, convert all records first
        for record in records {
            let document = self.message_to_document(&record)?;
            self.batch_buffer.push(document);
            
            // Flush if buffer is full
            if self.batch_buffer.len() >= self.config.batch_size {
                self.flush_batch().await?;
            }
        }
        
        // Flush remaining
        if !self.batch_buffer.is_empty() {
            self.flush_batch().await?;
        }
        
        Ok(())
    }
    
    async fn shutdown(&mut self) -> ConnectorResult<()> {
        info!("Shutting down MongoDB Sink Connector");
        
        // Flush any remaining messages
        if !self.batch_buffer.is_empty() {
            warn!("Flushing {} remaining documents before shutdown", self.batch_buffer.len());
            self.flush_batch().await?;
        }
        
        // Close MongoDB connection
        if let Some(client) = self.client.take() {
            // MongoDB Rust driver doesn't have explicit close, connection is dropped
            drop(client);
        }
        
        info!("MongoDB Sink Connector shutdown complete");
        Ok(())
    }
    
    async fn health_check(&self) -> ConnectorResult<()> {
        // Check if collection is accessible
        let collection = self.collection.as_ref()
            .ok_or_else(|| ConnectorError::Fatal {
                message: "Collection not initialized".into(),
                source: None,
            })?;
        
        // Perform a simple count operation to verify connection
        collection.count_documents(doc! {}, None)
            .await
            .map_err(|e| ConnectorError::Retryable {
                message: format!("Health check failed: {}", e),
                source: Some(Box::new(e)),
            })?;
        
        Ok(())
    }
}
```

## Step 4: Create the Main Entry Point

```rust
// src/main.rs

mod config;
mod connector;

use connector::MongoDBSinkConnector;
use danube_connect_core::{ConnectorConfig, SinkRuntime, ConnectorResult};
use tracing_subscriber;

#[tokio::main]
async fn main() -> ConnectorResult<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info,danube_connect_core=debug".to_string())
        )
        .init();
    
    tracing::info!("Starting MongoDB Sink Connector");
    
    // Load configuration
    let config = ConnectorConfig::from_env()
        .map_err(|e| {
            tracing::error!("Failed to load configuration: {}", e);
            e
        })?;
    
    tracing::info!("Configuration loaded successfully");
    tracing::info!("Danube Service URL: {}", config.danube_service_url);
    tracing::info!("Topic: {}", config.source_topic.as_ref().unwrap());
    tracing::info!("Subscription: {}", config.subscription_name.as_ref().unwrap());
    
    // Create connector instance
    let connector = MongoDBSinkConnector::new();
    
    // Create and run the runtime
    let mut runtime = SinkRuntime::new(connector, config).await?;
    
    // Run until shutdown signal
    runtime.run().await?;
    
    tracing::info!("MongoDB Sink Connector stopped");
    Ok(())
}
```

## Step 5: Create Configuration Examples

```yaml
# config.example.yaml

# Danube connection settings
danube_service_url: "http://localhost:6650"
connector_name: "mongodb-sink-1"
source_topic: "/default/events"
subscription_name: "mongodb-sink"
subscription_type: "Exclusive"

# Runtime settings
max_retries: 3
retry_backoff_ms: 1000
batch_size: 1000
batch_timeout_ms: 1000

# Metrics and logging
metrics_port: 9090
log_level: "info"

# MongoDB-specific settings (can also use environment variables)
mongodb_connection_string: "mongodb://localhost:27017"
mongodb_database: "myapp"
mongodb_collection: "events"
ordered_inserts: false
id_field: "event_id"
timestamp_field: "timestamp"
```

```dockerfile
# Dockerfile

FROM rust:1.75 as builder

WORKDIR /usr/src/app

# Copy workspace files
COPY Cargo.toml ./
COPY danube-connect-core ./danube-connect-core
COPY connectors/sink-mongodb ./connectors/sink-mongodb

# Build the connector
WORKDIR /usr/src/app/connectors/sink-mongodb
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder \
    /usr/src/app/target/release/danube-sink-mongodb \
    /usr/local/bin/danube-sink-mongodb

# Run as non-root user
RUN useradd -m -u 1000 danube
USER danube

ENTRYPOINT ["danube-sink-mongodb"]
```

## Step 6: Testing Your Connector

### Unit Tests

```rust
// src/connector.rs (add tests module)

#[cfg(test)]
mod tests {
    use super::*;
    use danube_core::message::{MessageID, StreamMessage};
    use std::collections::HashMap;
    
    #[tokio::test]
    async fn test_message_to_document() {
        let connector = MongoDBSinkConnector::new();
        
        let message = StreamMessage {
            request_id: 1,
            msg_id: MessageID {
                producer_id: 100,
                topic_name: "/default/test".to_string(),
                broker_addr: "localhost:6650".to_string(),
                topic_offset: 42,
            },
            payload: b"{\"name\":\"test\",\"value\":123}".to_vec(),
            publish_time: 1234567890,
            producer_name: "test-producer".to_string(),
            subscription_name: Some("test-sub".to_string()),
            attributes: HashMap::new(),
        };
        
        let record = SinkRecord::from_stream_message(message, None);
        let doc = connector.message_to_document(&record).unwrap();
        
        assert_eq!(doc.get_str("_danube_topic").unwrap(), "/default/test");
        assert_eq!(doc.get_i64("_danube_offset").unwrap(), 42);
    }
}
```

### Integration Tests

```rust
// tests/integration_test.rs

use danube_connect_core::{ConnectorConfig, SinkRuntime};
use mongodb::{Client, bson::doc};

#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored
async fn test_mongodb_sink_integration() {
    // Setup: Start Danube and MongoDB containers
    // This would use testcontainers or docker-compose
    
    // Create test messages in Danube
    // ...
    
    // Run connector for a few seconds
    // ...
    
    // Verify messages in MongoDB
    let mongo_client = Client::with_uri_str("mongodb://localhost:27017")
        .await
        .unwrap();
    
    let collection = mongo_client
        .database("test")
        .collection::<mongodb::bson::Document>("events");
    
    let count = collection.count_documents(doc! {}, None).await.unwrap();
    assert!(count > 0);
}
```

## Step 7: Build and Run

```bash
# Build the connector
cargo build --release

# Set environment variables
export DANUBE_SERVICE_URL="http://localhost:6650"
export DANUBE_TOPIC="/default/events"
export SUBSCRIPTION_NAME="mongodb-sink"
export CONNECTOR_NAME="mongodb-sink-1"
export MONGODB_CONNECTION_STRING="mongodb://localhost:27017"
export MONGODB_DATABASE="myapp"
export MONGODB_COLLECTION="events"
export RUST_LOG="info"

# Run the connector
./target/release/danube-sink-mongodb
```

### Using Docker

```bash
# Build Docker image
docker build -t danube-connect/sink-mongodb:latest .

# Run with docker-compose
docker-compose up -d
```

## Best Practices

### 1. Error Handling

- Use `ConnectorError::Retryable` for transient failures (network issues, timeouts)
- Use `ConnectorError::Fatal` for configuration errors or permanent failures
- Use `ConnectorError::InvalidData` for malformed messages

### 2. Batching

- Implement `process_batch()` for high-throughput scenarios
- Use configurable batch sizes and timeouts
- Flush on shutdown to avoid data loss

### 3. Configuration

- Always validate configuration at startup
- Use environment variables for sensitive data (passwords, API keys)
- Provide sensible defaults

### 4. Observability

- Log important events (initialization, errors, batch flushes)
- Use structured logging with tracing
- Implement health checks
- Expose connector-specific metrics

### 5. Resource Management

- Implement proper shutdown handling
- Close connections and flush buffers
- Use connection pooling when applicable

### 6. Testing

- Write unit tests for message transformation logic
- Create integration tests with real external systems
- Use test containers for reproducible testing

## Source Connector Example

For completeness, here's a simple source connector structure:

```rust
use async_trait::async_trait;
use danube_connect_core::{
    SourceConnector, SourceRecord, ConnectorConfig, 
    ConnectorResult, Offset
};

pub struct FileSourceConnector {
    file_path: String,
    position: u64,
}

#[async_trait]
impl SourceConnector for FileSourceConnector {
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()> {
        self.file_path = config.connector_config
            .get("FILE_PATH")
            .ok_or_else(|| ConnectorError::Configuration("FILE_PATH required".into()))?
            .clone();
        
        // Load last checkpoint
        self.position = load_checkpoint().await?;
        
        Ok(())
    }
    
    async fn poll(&mut self) -> ConnectorResult<Vec<SourceRecord>> {
        // Read lines from file starting at position
        let lines = read_lines_from(&self.file_path, self.position).await?;
        
        let mut records = Vec::new();
        for (line, offset) in lines {
            let record = SourceRecord::new("/default/file-events", line.into_bytes())
                .with_attribute("source", "file")
                .with_attribute("offset", offset.to_string());
            
            records.push(record);
        }
        
        Ok(records)
    }
    
    async fn commit(&mut self, offsets: Vec<Offset>) -> ConnectorResult<()> {
        // Save checkpoint
        if let Some(last_offset) = offsets.last() {
            self.position = last_offset.value;
            save_checkpoint(self.position).await?;
        }
        Ok(())
    }
}
```

## Next Steps

1. **Optimize Performance:** Profile your connector and optimize hot paths
2. **Add Metrics:** Implement connector-specific Prometheus metrics
3. **Documentation:** Write comprehensive README with examples
4. **CI/CD:** Setup automated builds and tests
5. **Contribute:** Submit your connector to the danube-connect repository

## Resources

- [Danube Documentation](https://danube-docs.dev-state.com)
- [danube-client Examples](https://github.com/danube-messaging/danube/tree/main/danube-client/examples)
- [Connector Core API Docs](../danube-connect-core/README.md)
- [Community Discord](#) - Ask questions and share your connector!
