# danube-connect-core

Core SDK for building Danube connectors.

## Overview

`danube-connect-core` is the foundation library for all Danube connectors. It provides:

- **Trait-based API**: Simple `SinkConnector` and `SourceConnector` traits
- **Runtime Management**: Automatic lifecycle handling and message processing loops
- **Message Transformation**: Utilities for converting between formats
- **Error Handling**: Built-in retry logic and error classification
- **Observability**: Automatic metrics, tracing, and health checks
- **Configuration**: Standard config management with environment variables

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
danube-connect-core = "0.1"
```

## Example: Simple Sink Connector

```rust
use danube_connect_core::{
    SinkConnector, SinkRecord, ConnectorConfig, 
    ConnectorResult, SinkRuntime
};
use async_trait::async_trait;

pub struct MyConnector {
    target_url: String,
}

#[async_trait]
impl SinkConnector for MyConnector {
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()> {
        self.target_url = config.get_string("TARGET_URL")?;
        Ok(())
    }
    
    async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
        // Your integration logic here
        println!("Processing: {:?}", record.payload());
        Ok(())
    }
}

#[tokio::main]
async fn main() -> ConnectorResult<()> {
    let config = ConnectorConfig::from_env()?;
    let connector = MyConnector { target_url: String::new() };
    
    let mut runtime = SinkRuntime::new(connector, config).await?;
    runtime.run().await
}
```

## Documentation

See the [Connector Development Guide](../info/connector-development-guide.md) for detailed examples and best practices.

## License

Apache-2.0
