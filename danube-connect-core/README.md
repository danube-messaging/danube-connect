# danube-connect-core

[![Crates.io](https://img.shields.io/crates/v/danube-connect-core.svg)](https://crates.io/crates/danube-connect-core)
[![Docs.rs](https://docs.rs/danube-connect-core/badge.svg)](https://docs.rs/danube-connect-core)
[![License](https://img.shields.io/crates/l/danube-connect-core.svg)](https://github.com/danube-messaging/danube-connect/blob/main/LICENSE)

Core SDK for building high-performance connectors for the [Danube messaging system](https://github.com/danube-messaging/danube).

## Overview

`danube-connect-core` is a production-ready framework for building connectors that integrate Danube with external systems. Whether you're building a sink connector to export messages or a source connector to import data, this SDK provides everything you need.

### Key Features

- **Simple Trait-Based API** - Implement just `SinkConnector` or `SourceConnector` traits
- **Automatic Runtime Management** - Lifecycle handling, message loops, and graceful shutdown
- **Built-in Retry Logic** - Exponential backoff with jitter for resilient integrations
- **Observability** - Prometheus metrics, structured logging, and health checks
- **Message Utilities** - Batching, serialization, and format conversion helpers
- **Configuration Management** - Environment variables, TOML files, and programmatic config
- **Error Handling** - Comprehensive error types and recovery strategies
- **Async/Await** - Built on Tokio for high-performance async operations

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
danube-connect-core = "0.1"
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
```

### Create a Sink Connector

```rust
use async_trait::async_trait;
use danube_connect_core::{
    ConnectorConfig, ConnectorResult, SinkConnector, SinkRecord, SinkRuntime,
};

/// A simple sink connector that prints messages
pub struct PrintSinkConnector {
    message_count: u64,
}

#[async_trait]
impl SinkConnector for PrintSinkConnector {
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()> {
        println!("Initialized: {}", config.connector_name);
        Ok(())
    }

    async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
        self.message_count += 1;
        println!(
            "Message #{}: {} bytes from topic {}",
            self.message_count,
            record.payload_size(),
            record.topic()
        );
        Ok(())
    }

    async fn shutdown(&mut self) -> ConnectorResult<()> {
        println!("Processed {} messages", self.message_count);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> ConnectorResult<()> {
    let config = ConnectorConfig::from_env()?;
    let connector = PrintSinkConnector { message_count: 0 };
    
    let mut runtime = SinkRuntime::new(connector, config).await?;
    runtime.run().await
}
```

### Create a Source Connector

```rust
use async_trait::async_trait;
use danube_connect_core::{
    ConnectorConfig, ConnectorResult, SourceConnector, SourceRecord, SourceRuntime,
};

/// A simple source connector that generates test messages
pub struct TestSourceConnector {
    counter: u64,
}

#[async_trait]
impl SourceConnector for TestSourceConnector {
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()> {
        println!("Initialized: {}", config.connector_name);
        Ok(())
    }

    async fn poll(&mut self) -> ConnectorResult<Vec<SourceRecord>> {
        if self.counter >= 100 {
            return Ok(vec![]);
        }

        self.counter += 1;
        let record = SourceRecord::from_string(
            "/default/test",
            format!("Message #{}", self.counter),
        );

        Ok(vec![record])
    }

    async fn shutdown(&mut self) -> ConnectorResult<()> {
        println!("Generated {} messages", self.counter);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> ConnectorResult<()> {
    let config = ConnectorConfig::from_env()?;
    let connector = TestSourceConnector { counter: 0 };
    
    let mut runtime = SourceRuntime::new(connector, config).await?;
    runtime.run().await
}
```

## Core Concepts

### Traits

- **`SinkConnector`** - Consumes messages from Danube and sends them to an external system
- **`SourceConnector`** - Reads data from an external system and publishes to Danube

### Runtime

- **`SinkRuntime`** - Manages the lifecycle of a sink connector
- **`SourceRuntime`** - Manages the lifecycle of a source connector

Both handle:
- Connection management
- Message polling/processing loops
- Error recovery and retries
- Graceful shutdown
- Signal handling (SIGTERM, SIGINT)

### Configuration

Configure via environment variables:

```bash
# Required
DANUBE_SERVICE_URL=http://localhost:6650
CONNECTOR_NAME=my-connector

# For sink connectors
DANUBE_TOPIC=/my/topic
SUBSCRIPTION_NAME=my-subscription

# For source connectors
DANUBE_TOPIC=/my/topic

# Optional
RELIABLE_DISPATCH=true
MAX_RETRIES=3
RETRY_BACKOFF_MS=1000
BATCH_SIZE=1000
BATCH_TIMEOUT_MS=5000
POLL_INTERVAL_MS=100
```

Or programmatically:

```rust
let config = ConnectorConfig {
    danube_service_url: "http://localhost:6650".to_string(),
    connector_name: "my-connector".to_string(),
    destination_topic: Some("/my/topic".to_string()),
    ..Default::default()
};
```

### Message Types

- **`SinkRecord`** - Message received from Danube (topic, offset, payload, attributes)
- **`SourceRecord`** - Message to publish to Danube (topic, payload, attributes)

### Utilities

The SDK includes helpful utilities in the `utils` module:

- **`Batcher<T>`** - Collect messages with size/timeout-based flushing
- **`HealthChecker`** - Track connector health with failure thresholds
- **`serialization`** - JSON and string conversion helpers

```rust
use danube_connect_core::{Batcher, HealthChecker};
use std::time::Duration;

// Batch messages
let mut batcher = Batcher::new(100, Duration::from_secs(5));
batcher.add(record);
if batcher.should_flush() {
    let batch = batcher.flush();
    // Process batch
}

// Track health
let mut health = HealthChecker::new(3); // Unhealthy after 3 failures
health.record_failure();
if !health.is_healthy() {
    // Handle degraded state
}
```

## Examples

The repository includes working examples:

- **`simple_sink`** - Basic sink connector that prints messages
- **`simple_source`** - Basic source connector that generates test messages

Run them with Docker:

```bash
# Start Danube cluster
cd docker
docker-compose up -d

# Run examples
cargo run --example simple_sink
cargo run --example simple_source
```

## Testing

Test your connector with the included Docker Compose setup:

```bash
# Start the cluster
cd docker
docker-compose up -d

# Run your connector
cargo run --bin my-connector

# Monitor metrics
curl http://localhost:9040/metrics
```

See the [Testing Guide](../docker/TESTING.md) for detailed testing patterns.

## Documentation

- **[Connector Development Guide](../info/connector-development-guide.md)** - Step-by-step guide for building connectors
- **[Architecture Overview](../info/connector-core-architecture.md)** - Deep dive into the SDK design
- **[Message Patterns](../info/connector-message-patterns.md)** - Common message handling patterns
- **[Docker Setup](../docker/README.md)** - Running the test cluster

## Performance

`danube-connect-core` is designed for high-performance integrations:

- **Async/await** - Built on Tokio for efficient async operations
- **Batching** - Process messages in batches for better throughput
- **Connection pooling** - Reuse connections to external systems
- **Metrics** - Monitor performance with Prometheus


## Contributing

Contributions are welcome! Please see the main [danube-connect](https://github.com/danube-messaging/danube-connect) repository for guidelines.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE) for details.

## Related Projects

- **[Danube](https://github.com/danube-messaging/danube)** - The core messaging system
- **[danube-connect](https://github.com/danube-messaging/danube-connect)** - Connector repository
- **[Danube Docs](https://danube-docs.dev-state.com)** - Official documentation
