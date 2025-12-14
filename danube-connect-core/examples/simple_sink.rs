//! Simple sink connector example
//!
//! This example demonstrates a minimal sink connector that prints messages to stdout.
//!
//! **NOTE:** This is a simplified example for testing the core library.
//! For production connectors, use the unified TOML+ENV configuration pattern.
//! See `connectors/source-mqtt/` for the recommended implementation.
//!
//! Usage:
//!   DANUBE_SERVICE_URL=http://localhost:6650 \
//!   CONNECTOR_NAME=simple-sink \
//!   DANUBE_TOPIC=/default/test \
//!   SUBSCRIPTION_NAME=simple-sub \
//!   cargo run --example simple_sink

use async_trait::async_trait;
use danube_connect_core::{
    ConnectorConfig, ConnectorResult, ConsumerConfig, ProcessingSettings, RetrySettings,
    SinkConnector, SinkRecord, SinkRuntime, SubscriptionType,
};

/// A simple sink connector that prints messages
struct SimpleSinkConnector {
    message_count: u64,
}

impl SimpleSinkConnector {
    fn new() -> Self {
        Self { message_count: 0 }
    }
}

#[async_trait]
impl SinkConnector for SimpleSinkConnector {
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()> {
        println!("SimpleSinkConnector initialized");
        println!("Configuration: {:?}", config);
        Ok(())
    }

    async fn consumer_configs(&self) -> ConnectorResult<Vec<ConsumerConfig>> {
        // Simple example: consume from one topic
        Ok(vec![ConsumerConfig {
            topic: "/default/test".to_string(),
            consumer_name: "simple-sink-consumer".to_string(),
            subscription: "simple-sink-sub".to_string(),
            subscription_type: SubscriptionType::Exclusive,
        }])
    }

    async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
        self.message_count += 1;

        println!("=== Message #{} ===", self.message_count);
        println!("Topic: {}", record.topic());
        println!("Offset: {}", record.offset());
        println!("Producer: {}", record.producer_name());
        println!("Publish Time: {}", record.publish_time());
        println!("Payload Size: {} bytes", record.payload_size());

        // Try to print as string
        match record.payload_str() {
            Ok(text) => println!("Payload (text): {}", text),
            Err(_) => println!("Payload (binary): {} bytes", record.payload_size()),
        }

        // Print attributes
        if !record.attributes().is_empty() {
            println!("Attributes:");
            for (key, value) in record.attributes() {
                println!("  {} = {}", key, value);
            }
        }

        println!();

        Ok(())
    }

    async fn shutdown(&mut self) -> ConnectorResult<()> {
        println!("SimpleSinkConnector shutting down");
        println!("Total messages processed: {}", self.message_count);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> ConnectorResult<()> {
    // NOTE: This example uses simple ENV-only config for testing.
    // Production connectors should use unified TOML+ENV config (see connectors/source-mqtt/)
    let config = ConnectorConfig::from_env().unwrap_or_else(|_| {
        println!("Using default configuration for testing");
        println!("To use custom settings, set environment variables:");
        println!("  DANUBE_SERVICE_URL (default: http://localhost:6650)");
        println!("  CONNECTOR_NAME (default: simple-sink)");
        println!("  DANUBE_TOPIC (default: /default/test)");
        println!("  SUBSCRIPTION_NAME (default: simple-sink-sub)");
        println!();

        ConnectorConfig {
            danube_service_url: "http://localhost:6650".to_string(),
            connector_name: "simple-sink".to_string(),
            retry: RetrySettings::default(),
            processing: ProcessingSettings::default(),
        }
    });

    // Create connector instance
    let connector = SimpleSinkConnector::new();

    // Create and run runtime
    let mut runtime = SinkRuntime::new(connector, config).await?;
    runtime.run().await
}
