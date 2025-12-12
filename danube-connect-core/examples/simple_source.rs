//! Simple source connector example
//!
//! This example demonstrates a minimal source connector that generates test messages.
//!
//! Usage:
//!   DANUBE_SERVICE_URL=http://localhost:6650 \
//!   CONNECTOR_NAME=simple-source \
//!   DANUBE_TOPIC=/default/test \
//!   cargo run --example simple_source

use async_trait::async_trait;
use danube_connect_core::{
    ConnectorConfig, ConnectorResult, SourceConnector, SourceRecord, SourceRuntime,
};
use std::time::Duration;

/// A simple source connector that generates test messages
struct SimpleSourceConnector {
    counter: u64,
    max_messages: u64,
}

impl SimpleSourceConnector {
    fn new(max_messages: u64) -> Self {
        Self {
            counter: 0,
            max_messages,
        }
    }
}

#[async_trait]
impl SourceConnector for SimpleSourceConnector {
    async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()> {
        println!("SimpleSourceConnector initialized");
        println!("Configuration: {:?}", config);
        println!("Will generate {} messages", self.max_messages);
        Ok(())
    }

    async fn poll(&mut self) -> ConnectorResult<Vec<SourceRecord>> {
        // Check if we've reached the limit
        if self.counter >= self.max_messages {
            // No more messages to send
            tokio::time::sleep(Duration::from_secs(1)).await;
            return Ok(vec![]);
        }

        // Generate a batch of messages
        let mut records = Vec::new();
        let batch_size = 5.min(self.max_messages - self.counter);

        for i in 0..batch_size {
            self.counter += 1;

            let message = format!(
                "Test message #{} - Timestamp: {}",
                self.counter,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            );

            let record = SourceRecord::from_string("/default/test", message)
                .with_attribute("source", "simple-source-connector")
                .with_attribute("message_number", self.counter.to_string())
                .with_attribute("batch_index", i.to_string());

            records.push(record);

            println!("Generated message #{}", self.counter);
        }

        // Simulate some processing time
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(records)
    }

    async fn shutdown(&mut self) -> ConnectorResult<()> {
        println!("SimpleSourceConnector shutting down");
        println!("Total messages generated: {}", self.counter);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> ConnectorResult<()> {
    // Load configuration from environment, with sensible defaults for testing
    let config = ConnectorConfig::from_env().unwrap_or_else(|_| {
        println!("Using default configuration for testing");
        println!("To use custom settings, set environment variables:");
        println!("  DANUBE_SERVICE_URL (default: http://localhost:6650)");
        println!("  CONNECTOR_NAME (default: simple-source)");
        println!("  DANUBE_TOPIC (default: /default/test)");
        println!();

        ConnectorConfig {
            danube_service_url: "http://localhost:6650".to_string(),
            connector_name: "simple-source".to_string(),
            source_topic: None,
            subscription_name: None,
            subscription_type: None,
            destination_topic: Some("/default/test".to_string()),
            reliable_dispatch: true,
            max_retries: 3,
            retry_backoff_ms: 1000,
            max_backoff_ms: 30000,
            batch_size: 1000,
            batch_timeout_ms: 5000,
            poll_interval_ms: 100,
            metrics_port: 9091,
            log_level: "info".to_string(),
            connector_config: Default::default(),
        }
    });

    // Create connector instance (generate 100 messages)
    let connector = SimpleSourceConnector::new(100);

    // Create and run runtime
    let mut runtime = SourceRuntime::new(connector, config).await?;
    runtime.run().await
}
