# Danube Connector Message Patterns & Data Flow

## Overview

This document describes common message handling patterns, data transformation strategies, and best practices for working with Danube messages in connectors.

## Understanding Danube's Message Model

### Message Structure

Danube uses a protocol buffer-based message format optimized for high-throughput streaming:

```protobuf
message StreamMessage {
    uint64 request_id = 1;              // Request tracking ID
    MsgID msg_id = 2;                   // Message identifier
    bytes payload = 3;                  // Raw binary payload
    uint64 publish_time = 4;            // Unix timestamp (microseconds)
    string producer_name = 5;           // Producer identifier
    string subscription_name = 6;        // Subscription routing
    map<string, string> attributes = 7; // User-defined metadata
}

message MsgID {
    uint64 producer_id = 1;    // Producer ID within topic
    string topic_name = 2;     // Full topic name (e.g., /default/events)
    string broker_addr = 3;    // Broker address for ack routing
    uint64 topic_offset = 4;   // Monotonic offset within topic
}
```

### Field Semantics

**Client-Populated Fields:**
- `request_id`: Unique per send operation, used for request/response correlation
- `payload`: The actual message data (schema-agnostic bytes)
- `producer_name`: Identifies the producer instance
- `attributes`: Custom key-value metadata (e.g., content-type, correlation-id)

**Broker-Populated Fields:**
- `msg_id`: Complete message identifier with routing information
- `publish_time`: Server-side timestamp for message ordering
- `subscription_name`: Added when delivering to consumers

**Key Properties:**
- `topic_offset` is monotonically increasing per partition
- `msg_id` uniquely identifies a message for acknowledgment
- `attributes` are limited to string key-value pairs (no nested structures)

## Data Flow Patterns

### Pattern 1: Sink Connector (Danube → External System)

```text
┌─────────────────────────────────────────────────────────────┐
│                        Danube Broker                        │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Topic: /default/orders                              │  │
│  │  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐    │  │
│  │  │ Msg 1  │  │ Msg 2  │  │ Msg 3  │  │ Msg 4  │    │  │
│  │  └────────┘  └────────┘  └────────┘  └────────┘    │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────┬───────────────────────────────────────────────┘
              │ gRPC Stream (ReceiveMessages)
              │
┌─────────────▼───────────────────────────────────────────────┐
│           Sink Connector (Consumer)                         │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  1. Receive StreamMessage                            │  │
│  │  2. Transform to external format                     │  │
│  │  3. Write to external system                         │  │
│  │  4. Acknowledge (Ack) back to broker                 │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────┬───────────────────────────────────────────────┘
              │ External Protocol (HTTP, SQL, etc.)
              │
┌─────────────▼───────────────────────────────────────────────┐
│                    External System                          │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Database / API / File System / Message Queue        │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

**Example: HTTP Sink**
```rust
async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
    // 1. Extract payload
    let payload = record.payload();
    
    // 2. Build HTTP request with metadata from attributes
    let request = self.http_client
        .post(&self.target_url)
        .header("X-Danube-Topic", &record.message.msg_id.topic_name)
        .header("X-Danube-Offset", record.message.msg_id.topic_offset)
        .header("Content-Type", "application/json")
        .body(payload.to_vec());
    
    // 3. Send request (retryable operation)
    let response = request.send().await
        .map_err(|e| ConnectorError::Retryable { ... })?;
    
    // 4. Check response
    if response.status().is_success() {
        Ok(()) // Runtime will ack automatically
    } else {
        Err(ConnectorError::Retryable { ... })
    }
}
```

### Pattern 2: Source Connector (External System → Danube)

```text
┌─────────────────────────────────────────────────────────────┐
│                    External System                          │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  New Data: DB Changes, Log Lines, API Events         │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────┬───────────────────────────────────────────────┘
              │ Poll / Stream / CDC
              │
┌─────────────▼───────────────────────────────────────────────┐
│         Source Connector (Producer)                         │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  1. Poll external system for new data                │  │
│  │  2. Transform to Danube SourceRecord                 │  │
│  │  3. Publish to Danube topic                          │  │
│  │  4. Commit offset/checkpoint in external system      │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────┬───────────────────────────────────────────────┘
              │ gRPC (SendMessage)
              │
┌─────────────▼───────────────────────────────────────────────┐
│                      Danube Broker                          │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Topic: /default/postgres-cdc                        │  │
│  │  ┌────────┐  ┌────────┐  ┌────────┐                 │  │
│  │  │ Msg 1  │  │ Msg 2  │  │ Msg 3  │  ...            │  │
│  │  └────────┘  └────────┘  └────────┘                 │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

**Example: PostgreSQL CDC Source**
```rust
async fn poll(&mut self) -> ConnectorResult<Vec<SourceRecord>> {
    // 1. Poll PostgreSQL replication stream
    let changes = self.replication_stream
        .next_batch(self.config.batch_size)
        .await?;
    
    let mut records = Vec::new();
    
    // 2. Transform each change to SourceRecord
    for change in changes {
        let payload = serde_json::to_vec(&json!({
            "op": change.operation, // INSERT, UPDATE, DELETE
            "table": change.table_name,
            "before": change.old_values,
            "after": change.new_values,
            "lsn": change.lsn,
            "timestamp": change.timestamp,
        }))?;
        
        let record = SourceRecord::new(&self.config.destination_topic, payload)
            .with_attribute("source", "postgres")
            .with_attribute("table", &change.table_name)
            .with_attribute("op", &change.operation)
            .with_key(&change.primary_key); // For partitioning
        
        records.push(record);
    }
    
    Ok(records)
}

async fn commit(&mut self, offsets: Vec<Offset>) -> ConnectorResult<()> {
    // 3. Commit LSN position in PostgreSQL
    if let Some(last_offset) = offsets.last() {
        self.replication_stream
            .commit_lsn(last_offset.value)
            .await?;
    }
    Ok(())
}
```

### Pattern 3: Bridge Connector (Bidirectional)

Bridge connectors facilitate migration or integration between messaging systems:

```text
┌──────────────┐  Source Mode   ┌──────────────┐  Sink Mode    ┌──────────────┐
│   Kafka      │  ─────────────> │   Bridge     │  ────────────> │   Danube     │
│   Cluster    │                 │  Connector   │                │   Cluster    │
│              │ <───────────────│              │ <──────────────│              │
└──────────────┘   Sink Mode     └──────────────┘  Source Mode   └──────────────┘
```

**Use Cases:**
- **Migration:** Gradually move workloads from Kafka to Danube
- **Integration:** Connect Danube to existing Kafka ecosystems
- **Hybrid:** Run both systems during transition periods

## Message Transformation Patterns

### 1. Pass-Through (Minimal Transformation)

Best for: Binary protocols, already-serialized data

```rust
async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
    // Direct write without transformation
    self.external_system.write(record.payload()).await?;
    Ok(())
}
```

### 2. JSON Transformation

Best for: REST APIs, document databases, analytics systems

```rust
async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
    // Deserialize Danube payload
    let data: MyDataType = record.payload_json()?;
    
    // Enrich with Danube metadata
    let enriched = EnrichedData {
        data,
        metadata: Metadata {
            topic: record.message.msg_id.topic_name.clone(),
            offset: record.message.msg_id.topic_offset,
            timestamp: record.message.publish_time,
            producer: record.message.producer_name.clone(),
        },
    };
    
    // Write to external system
    self.external_system.insert(&enriched).await?;
    Ok(())
}
```

### 3. Schema-Based Transformation

Best for: Strong typing requirements, Avro/Protobuf schemas

```rust
async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
    // Get schema from attributes or Danube schema registry
    let schema_id = record.message.attributes
        .get("schema_id")
        .ok_or(ConnectorError::InvalidData { ... })?;
    
    let schema = self.schema_registry.get_schema(schema_id).await?;
    
    // Deserialize using schema
    let value = schema.deserialize(record.payload())?;
    
    // Transform to target system's schema
    let target_value = self.transform_schema(value, &schema)?;
    
    // Write
    self.external_system.write(&target_value).await?;
    Ok(())
}
```

### 4. Batched Transformation with Aggregation

Best for: Analytics databases (ClickHouse, Snowflake), time-series data

```rust
pub struct BatchingSinkConnector {
    batch: Vec<TransformedRecord>,
    batch_start: Instant,
}

async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
    // Transform single record
    let transformed = self.transform_record(record)?;
    self.batch.push(transformed);
    
    // Flush conditions
    let should_flush = 
        self.batch.len() >= self.config.batch_size ||
        self.batch_start.elapsed() > self.config.batch_timeout;
    
    if should_flush {
        self.flush_batch().await?;
    }
    
    Ok(())
}

async fn flush_batch(&mut self) -> ConnectorResult<()> {
    if self.batch.is_empty() {
        return Ok(());
    }
    
    // Bulk insert for better performance
    self.external_system
        .bulk_insert(&self.batch)
        .await?;
    
    self.batch.clear();
    self.batch_start = Instant::now();
    
    Ok(())
}
```

## Handling Message Attributes

Danube's `attributes` field provides flexible metadata transport:

### Common Attribute Patterns

```rust
// Content-Type detection
let content_type = record.message.attributes
    .get("content-type")
    .map(|s| s.as_str())
    .unwrap_or("application/octet-stream");

match content_type {
    "application/json" => self.handle_json(record).await?,
    "application/xml" => self.handle_xml(record).await?,
    "text/plain" => self.handle_text(record).await?,
    _ => self.handle_binary(record).await?,
}

// Correlation ID for distributed tracing
if let Some(correlation_id) = record.message.attributes.get("correlation-id") {
    tracing::span!(
        tracing::Level::INFO,
        "process_message",
        correlation_id = %correlation_id
    );
}

// Custom routing keys
if let Some(customer_id) = record.message.attributes.get("customer-id") {
    // Route to customer-specific processing
    self.route_to_customer_pipeline(customer_id, record).await?;
}

// Schema evolution
if let Some(schema_version) = record.message.attributes.get("schema-version") {
    let transformer = self.get_transformer_for_version(schema_version)?;
    transformer.transform(record).await?;
}
```

### Adding Attributes in Source Connectors

```rust
async fn poll(&mut self) -> ConnectorResult<Vec<SourceRecord>> {
    let records = self.fetch_from_external_system().await?;
    
    records.into_iter()
        .map(|data| {
            SourceRecord::new(&self.topic, data.payload)
                // Standard attributes
                .with_attribute("content-type", "application/json")
                .with_attribute("source", "postgres-cdc")
                
                // Business context
                .with_attribute("tenant-id", &data.tenant_id)
                .with_attribute("region", &data.region)
                
                // Tracing
                .with_attribute("trace-id", &Uuid::new_v4().to_string())
                
                // Schema information
                .with_attribute("schema-version", "v2.1")
                .with_attribute("schema-id", &data.schema_id)
        })
        .collect()
}
```

## Error Handling Patterns

### Pattern 1: Retry with Exponential Backoff

For transient failures (network issues, rate limits):

```rust
async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
    match self.write_to_external(record).await {
        Ok(_) => Ok(()),
        Err(e) if e.is_timeout() || e.is_connection_error() => {
            // Runtime will retry with exponential backoff
            Err(ConnectorError::Retryable {
                message: format!("Transient error: {}", e),
                source: Some(Box::new(e)),
            })
        }
        Err(e) => {
            // Fatal error, don't retry
            Err(ConnectorError::Fatal {
                message: format!("Permanent error: {}", e),
                source: Some(Box::new(e)),
            })
        }
    }
}
```

### Pattern 2: Dead Letter Queue

For invalid messages that should be skipped:

```rust
async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
    match self.validate_and_transform(record.clone()) {
        Ok(data) => {
            self.external_system.write(&data).await?;
            Ok(())
        }
        Err(e) if e.is_validation_error() => {
            // Send to DLQ topic for later inspection
            self.dlq_producer
                .send_to_dlq(record, &e)
                .await?;
            
            // Return Ok to ack the message
            Ok(())
        }
        Err(e) => Err(e),
    }
}
```

### Pattern 3: Partial Success in Batches

For batch operations where some records succeed:

```rust
async fn process_batch(&mut self, records: Vec<SinkRecord>) -> ConnectorResult<()> {
    let results = self.external_system
        .bulk_insert(&records)
        .await?;
    
    // Check for partial failures
    let failed_records: Vec<_> = results.iter()
        .zip(records.iter())
        .filter_map(|(result, record)| {
            if let Err(e) = result {
                Some((record.clone(), e))
            } else {
                None
            }
        })
        .collect();
    
    if !failed_records.is_empty() {
        // Retry failed records individually
        for (record, error) in failed_records {
            tracing::warn!("Retrying failed record: {}", error);
            self.process(record).await?;
        }
    }
    
    Ok(())
}
```

## Performance Optimization Patterns

### 1. Connection Pooling

```rust
pub struct OptimizedSinkConnector {
    connection_pool: Pool<ConnectionManager>,
}

async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()> {
    self.connection_pool = Pool::builder()
        .max_size(config.pool_size)
        .connection_timeout(Duration::from_secs(5))
        .build(ConnectionManager::new(config.connection_string))
        .await?;
    
    Ok(())
}
```

### 2. Async Batching with Timeout

```rust
use tokio::time::{timeout, Duration};

async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
    self.batch.push(record);
    
    // Non-blocking check for batch conditions
    if self.should_flush() {
        self.flush_batch().await?;
    }
    
    Ok(())
}

// Background flush task
async fn start_flush_timer(&self) {
    loop {
        tokio::time::sleep(self.config.flush_interval).await;
        if !self.batch.is_empty() {
            self.flush_batch().await.ok();
        }
    }
}
```

### 3. Parallel Processing

For independent operations:

```rust
async fn process_batch(&mut self, records: Vec<SinkRecord>) -> ConnectorResult<()> {
    // Process records in parallel
    let futures: Vec<_> = records.into_iter()
        .map(|record| self.process_single(record))
        .collect();
    
    let results = futures::future::join_all(futures).await;
    
    // Check for any failures
    for result in results {
        result?;
    }
    
    Ok(())
}
```

## Best Practices Summary

1. **Leverage Attributes:** Use message attributes for routing, tracing, and metadata
2. **Batch When Possible:** Aggregate writes for better throughput
3. **Handle Errors Gracefully:** Distinguish retryable vs fatal errors
4. **Use Structured Logging:** Include message IDs and offsets in logs
5. **Implement Health Checks:** Verify external system connectivity
6. **Monitor Performance:** Track latency, throughput, and error rates
7. **Plan for Shutdown:** Flush buffers and close connections cleanly
8. **Test with Real Data:** Validate transformation logic with production-like payloads
9. **Document Schema:** Clearly specify expected payload formats
10. **Version Attributes:** Support schema evolution via version attributes

## References

- [Danube Message Architecture](https://danube-docs.dev-state.com/architecture/messages/)
- [Connector Core Architecture](./connector-core-architecture.md)
- [Connector Development Guide](./connector-development-guide.md)
- [danube-client API Documentation](../danube-client/README.md)
