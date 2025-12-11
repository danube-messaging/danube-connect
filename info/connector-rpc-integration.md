# Danube Connector RPC Integration Guide

## Overview

This document provides detailed information about how Danube connectors integrate with the Danube broker cluster via RPC (gRPC), leveraging the `danube-client` library. Understanding this integration is crucial for connector developers who need to optimize performance or debug connection issues.

## RPC Protocol Stack

```text
┌─────────────────────────────────────────────────────────────┐
│                  Connector Implementation                   │
│            (Implements SinkConnector/SourceConnector)       │
└─────────────────┬───────────────────────────────────────────┘
                  │ Uses Traits & Helper Methods
┌─────────────────▼───────────────────────────────────────────┐
│              danube-connect-core                            │
│  (Runtime, Message Transformation, Error Handling)          │
└─────────────────┬───────────────────────────────────────────┘
                  │ Wraps & Manages
┌─────────────────▼───────────────────────────────────────────┐
│                danube-client                                │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Producer API    │ Consumer API   │ Discovery API    │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │         Connection Manager & Health Checks           │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              gRPC Client (tonic)                     │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────┬───────────────────────────────────────────┘
                  │ gRPC over HTTP/2
┌─────────────────▼───────────────────────────────────────────┐
│                  Danube Broker                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ ProducerService │ ConsumerService │ Discovery        │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Key RPC Services

### 1. Discovery Service

Used for initial broker discovery and topic metadata lookup.

#### TopicLookup

**Purpose:** Find which broker serves a specific topic

```rust
// Proto definition
service Discovery {
    rpc TopicLookup(TopicLookupRequest) returns (TopicLookupResponse);
}

message TopicLookupRequest {
    uint64 request_id = 1;
    string topic = 2;
}

message TopicLookupResponse {
    enum LookupType {
        Redirect = 0;  // Connect to different broker
        Connect  = 1;  // Connect to this broker
        Failed   = 2;  // Topic doesn't exist or error
    }
    uint64 request_id = 3;
    LookupType response_type = 4;
    string brokerServiceUrl = 5;
}
```

**Client Usage:**
```rust
// danube-client automatically handles lookup
let client = DanubeClient::builder()
    .service_url("http://localhost:6650")  // Initial broker
    .build()
    .await?;

// Behind the scenes, client performs topic lookup
// and connects to the correct broker
let mut producer = client
    .new_producer()
    .with_topic("/default/events")  // Triggers lookup
    .build();
```

**Connector Implications:**
- Connectors don't need to manually handle broker discovery
- `danube-connect-core` wraps the lookup logic
- Automatic reconnection if broker changes

#### TopicPartitions

**Purpose:** Get list of partitions for a partitioned topic

```rust
rpc TopicPartitions(TopicLookupRequest) returns (TopicPartitionsResponse);

message TopicPartitionsResponse {
    uint64 request_id = 1;
    repeated string partitions = 2;  // ["/default/topic-part-0", "/default/topic-part-1", ...]
}
```

**Usage in Connectors:**
```rust
// For source connectors producing to partitioned topics
let partitions = client.lookup_service
    .topic_partitions(&service_url, "/default/orders")
    .await?;

// Create one producer per partition for parallel writes
for partition in partitions {
    let producer = client.new_producer()
        .with_topic(&partition)
        .build();
    producers.push(producer);
}
```

### 2. ProducerService (Source Connectors)

Used by source connectors to publish messages to Danube.

#### CreateProducer

**Purpose:** Register a new producer with the broker

```rust
service ProducerService {
    rpc CreateProducer(ProducerRequest) returns (ProducerResponse);
}

message ProducerRequest { 
    uint64 request_id = 1;
    string producer_name = 2;
    string topic_name = 3;
    Schema schema = 4;
    ProducerAccessMode producer_access_mode = 5;  // Shared or Exclusive
    DispatchStrategy dispatch_strategy = 6;        // NonReliable or Reliable
}

message ProducerResponse {
    uint64 request_id = 1;
    uint64 producer_id = 2;  // Assigned by broker, used in message IDs
    string producer_name = 3;
}
```

**Client Wrapper:**
```rust
// danube-client provides builder pattern
let mut producer = client
    .new_producer()
    .with_topic("/default/events")
    .with_name("postgres-cdc-connector")
    .with_reliable_dispatch()  // WAL + Cloud persistence
    .build();

// This calls CreateProducer RPC
producer.create().await?;
```

**Key Points:**
- `producer_id` is unique per topic and used in message identification
- `reliable_dispatch` ensures messages are persisted to WAL and cloud storage
- `non_reliable_dispatch` is faster but messages may be lost

#### SendMessage

**Purpose:** Send a single message to the topic

```rust
rpc SendMessage(StreamMessage) returns (MessageResponse);

message StreamMessage {
    uint64 request_id = 1;
    MsgID msg_id = 2;              // Partially filled by client
    bytes payload = 3;
    uint64 publish_time = 4;       // Set by broker
    string producer_name = 5;
    string subscription_name = 6;   // Set by broker for consumers
    map<string, string> attributes = 7;
}

message MessageResponse {
    uint64 request_id = 1;  // Matches request for correlation
}
```

**Client Usage:**
```rust
// Simple send
let message_id = producer.send(
    b"hello world".to_vec(),
    None  // Optional attributes
).await?;

// With attributes
let mut attrs = HashMap::new();
attrs.insert("content-type".to_string(), "application/json".to_string());
attrs.insert("source".to_string(), "postgres".to_string());

let message_id = producer.send(data, Some(attrs)).await?;
```

**Message Flow:**
1. Client creates `StreamMessage` with partial `msg_id`
2. Broker completes `msg_id` fields (topic_offset, publish_time)
3. Message is appended to topic
4. Broker returns `MessageResponse` with request_id

**Connector Pattern:**
```rust
// In SourceConnector::poll()
async fn poll(&mut self) -> ConnectorResult<Vec<SourceRecord>> {
    let data = self.fetch_from_external_system().await?;
    
    Ok(vec![
        SourceRecord::new("/default/events", data)
            .with_attribute("source", "my-connector")
    ])
}

// Runtime handles the actual SendMessage RPC
// and tracks acknowledgments
```

### 3. ConsumerService (Sink Connectors)

Used by sink connectors to receive messages from Danube.

#### Subscribe

**Purpose:** Create a consumer and attach to a subscription

```rust
service ConsumerService {
    rpc Subscribe(ConsumerRequest) returns (ConsumerResponse);
}

message ConsumerRequest {
    enum SubscriptionType {
        Exclusive = 0;  // Single consumer
        Shared = 1;     // Load-balanced across multiple consumers
        Failover = 2;   // Active-standby pattern
    }
    uint64 request_id = 1;
    string topic_name = 2;
    string consumer_name = 3;
    string subscription = 4;
    SubscriptionType subscription_type = 5;
}

message ConsumerResponse {
    uint64 request_id = 1;
    uint64 consumer_id = 2;  // Assigned by broker
    string consumer_name = 3;
}
```

**Client Usage:**
```rust
let mut consumer = client
    .new_consumer()
    .with_topic("/default/events")
    .with_consumer_name("http-sink-1")
    .with_subscription("http-webhooks")
    .with_subscription_type(SubType::Exclusive)
    .build();

// This calls Subscribe RPC
consumer.subscribe().await?;
```

**Subscription Types Impact:**

| Type | Use Case | Connector Pattern |
|------|----------|-------------------|
| Exclusive | Single instance processing | Development, single-threaded sinks |
| Shared | Load balancing across replicas | Production deployments, horizontal scaling |
| Failover | High availability with standby | Mission-critical sinks requiring failover |

#### ReceiveMessages (Streaming)

**Purpose:** Streaming RPC that pushes messages to consumer

```rust
rpc ReceiveMessages(ReceiveRequest) returns (stream StreamMessage);

message ReceiveRequest {
    uint64 request_id = 1;
    uint64 consumer_id = 2;
}
```

**Client Usage:**
```rust
// Start receiving messages (returns mpsc channel)
let mut message_stream = consumer.receive().await?;

// Process messages asynchronously
while let Some(message) = message_stream.recv().await {
    println!("Received: {:?}", message);
    
    // Process message...
    
    // Acknowledge
    consumer.ack(&message).await?;
}
```

**Message Flow:**
1. Client sends `ReceiveRequest` with consumer_id
2. Broker opens a streaming response
3. Broker pushes messages as they arrive (or from backlog)
4. Client processes and acknowledges
5. Stream remains open until client disconnects or shutdown

**Ordering Guarantees:**
- **Non-Partitioned Topics:** Messages delivered in publish order
- **Partitioned Topics:** Messages delivered in order per partition
- **Shared Subscriptions:** No ordering guarantees across consumers

#### Ack (Acknowledgment)

**Purpose:** Acknowledge successful message processing

```rust
rpc Ack(AckRequest) returns (AckResponse);

message AckRequest {
    uint64 request_id = 1;
    MsgID msg_id = 2;            // Identifies the message
    string subscription_name = 3; // Routes ack to correct subscription
}

message AckResponse {
    uint64 request_id = 1;
}
```

**Client Usage:**
```rust
// After successfully processing a message
consumer.ack(&message).await?;
```

**Acknowledgment Semantics:**
- **At-Least-Once Delivery:** Messages are redelivered if not acked
- **Ack Timeout:** Broker has configurable timeout for unacked messages
- **Negative Ack:** Can explicitly nack for immediate redelivery (future feature)

**Connector Pattern:**
```rust
async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
    // Write to external system
    self.external_system.write(record.payload()).await?;
    
    // If this returns Ok(()), runtime automatically acks
    // If Err, runtime retries or nacks based on error type
    Ok(())
}
```

### 4. HealthCheck Service

**Purpose:** Verify client connection is alive

```rust
service HealthCheck {
    rpc HealthCheck(HealthCheckRequest) returns (HealthCheckResponse);
}

message HealthCheckRequest {
    enum ClientType {
        Producer = 0;
        Consumer = 1;
    }
    uint64 request_id = 1;
    ClientType client = 2;
    uint64 id = 3;  // producer_id or consumer_id
}

message HealthCheckResponse {
    enum ClientStatus {
        OK = 0;
        CLOSE = 1;  // Client should disconnect
    }
    ClientStatus status = 1;
}
```

**Client Usage:**
```rust
// Periodic health checks (handled automatically by danube-client)
tokio::spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;
        
        if let Err(e) = producer.health_check().await {
            // Reconnect logic
            reconnect().await;
        }
    }
});
```

## Connection Management

### Connection Lifecycle

```text
┌─────────────────────────────────────────────────────────────┐
│                    1. Initial Connection                    │
│  Client → Discovery Service (TopicLookup)                   │
│  Response: brokerServiceUrl                                 │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│              2. Producer/Consumer Registration              │
│  Client → ProducerService (CreateProducer)                  │
│         OR ConsumerService (Subscribe)                      │
│  Response: producer_id / consumer_id                        │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│                3. Message Streaming/Sending                 │
│  Producer: SendMessage RPC calls                            │
│  Consumer: ReceiveMessages streaming RPC                    │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│                  4. Health Monitoring                       │
│  Periodic HealthCheck RPCs                                  │
│  Automatic reconnection on failures                         │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│                    5. Graceful Shutdown                     │
│  Client closes gRPC channels                                │
│  Broker cleans up resources                                 │
└─────────────────────────────────────────────────────────────┘
```

### Retry and Backoff

The `danube-client` includes built-in retry logic:

```rust
// In danube-client/src/retry_manager.rs
pub struct RetryManager {
    max_retries: u32,
    base_backoff_ms: u64,
    max_backoff_ms: u64,
}

impl RetryManager {
    pub async fn retry_with_backoff<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) if attempt < self.max_retries => {
                    attempt += 1;
                    let backoff = self.calculate_backoff(attempt);
                    tokio::time::sleep(backoff).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}
```

**Connector Configuration:**
```rust
let client = DanubeClient::builder()
    .service_url("http://localhost:6650")
    .build()
    .await?;

let producer = client
    .new_producer()
    .with_topic("/default/events")
    .with_options(ProducerOptions {
        max_retries: 5,
        base_backoff_ms: 100,
        max_backoff_ms: 5000,
    })
    .build();
```

### Connection Pooling

For high-throughput connectors:

```rust
// Multiple producers for parallel publishing
let mut producers = Vec::new();
for i in 0..num_workers {
    let producer = client
        .new_producer()
        .with_topic(&topic)
        .with_name(&format!("worker-{}", i))
        .build();
    
    producer.create().await?;
    producers.push(producer);
}

// Round-robin across producers
let producer = &producers[message_idx % producers.len()];
producer.send(payload, attributes).await?;
```

## Performance Considerations

### 1. Batching Messages

While `SendMessage` RPC is per-message, connectors can optimize with internal buffering:

```rust
pub struct BatchedProducer {
    producer: Producer,
    batch: Vec<Vec<u8>>,
    batch_size: usize,
}

impl BatchedProducer {
    pub async fn send(&mut self, payload: Vec<u8>) -> Result<()> {
        self.batch.push(payload);
        
        if self.batch.len() >= self.batch_size {
            self.flush().await?;
        }
        
        Ok(())
    }
    
    pub async fn flush(&mut self) -> Result<()> {
        // Send all messages in parallel
        let futures: Vec<_> = self.batch.drain(..)
            .map(|payload| self.producer.send(payload, None))
            .collect();
        
        futures::future::try_join_all(futures).await?;
        Ok(())
    }
}
```

### 2. Message Size Optimization

```rust
// Compress large payloads
fn compress_payload(data: &[u8]) -> Vec<u8> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

// Send with compression indicator
let compressed = compress_payload(&large_payload);
let mut attrs = HashMap::new();
attrs.insert("compression".to_string(), "gzip".to_string());

producer.send(compressed, Some(attrs)).await?;
```

### 3. Partition Affinity

For ordered processing:

```rust
// Consistent hashing to ensure same key goes to same partition
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn get_partition(key: &str, num_partitions: usize) -> usize {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    (hasher.finish() % num_partitions as u64) as usize
}

// Route to specific partition
let partition_idx = get_partition(&customer_id, partitions.len());
let producer = &producers[partition_idx];
producer.send(payload, attributes).await?;
```

## Debugging RPC Issues

### Enable gRPC Logging

```bash
RUST_LOG=debug,tonic=trace,h2=debug cargo run
```

### Common Issues and Solutions

| Issue | Symptom | Solution |
|-------|---------|----------|
| Connection refused | "Failed to connect to broker" | Verify broker is running and URL is correct |
| Topic not found | "TOPIC_NOT_FOUND error" | Create topic via admin CLI first |
| Authentication failed | "Unauthorized" error | Check API key configuration |
| Timeout | "Deadline exceeded" | Increase timeout or check network latency |
| Stream closed | "Stream ended unexpectedly" | Check broker logs, enable auto-reconnect |

### Inspecting Messages

```rust
// Enable message tracing
let producer = client
    .new_producer()
    .with_topic("/default/events")
    .build();

// Log every message
let result = producer.send(payload.clone(), attrs.clone()).await;
tracing::info!(
    topic = "/default/events",
    payload_size = payload.len(),
    result = ?result,
    "Message sent"
);
```

## Security Considerations

### Authentication

```rust
// Using API key authentication
let client = DanubeClient::builder()
    .service_url("http://localhost:6650")
    .with_api_key("your-api-key")  // Future feature
    .build()
    .await?;
```

### TLS/mTLS

```rust
// Using TLS
let client = DanubeClient::builder()
    .service_url("https://danube.example.com:6651")
    .with_tls_config(tls_config)  // Future feature
    .build()
    .await?;
```

## Summary

**Key Takeaways:**

1. **Abstraction:** `danube-connect-core` wraps `danube-client` so connector developers don't interact with RPC directly
2. **Discovery:** Automatic broker discovery and topic lookup
3. **Reliability:** Built-in retry logic and health checking
4. **Streaming:** Consumer uses long-lived streaming RPC for efficient message delivery
5. **Acknowledgment:** Explicit ack model ensures at-least-once delivery
6. **Performance:** Batching and parallel sends can significantly improve throughput

**References:**
- [DanubeApi.proto](../danube-core/proto/DanubeApi.proto) - Full protocol definition
- [danube-client source](../danube-client/src/) - Client implementation
- [Connector Core Architecture](./connector-core-architecture.md)
