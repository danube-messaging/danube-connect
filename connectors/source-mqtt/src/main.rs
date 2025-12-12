//! MQTT Source Connector for Danube Connect
//!
//! This connector subscribes to MQTT topics and publishes messages to Danube topics.
//! Perfect for IoT use cases where devices publish telemetry via MQTT.

mod config;
mod connector;

use connector::MqttSourceConnector;
use danube_connect_core::{ConnectorConfig, ConnectorResult, SourceRuntime};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> ConnectorResult<()> {
    // Initialize logging
    init_tracing();

    tracing::info!("Starting MQTT Source Connector");
    tracing::info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = ConnectorConfig::from_env().map_err(|e| {
        tracing::error!("Failed to load configuration: {}", e);
        e
    })?;

    tracing::info!("Configuration loaded successfully");
    tracing::info!("Connector: {}", config.connector_name);
    tracing::info!("Danube URL: {}", config.danube_service_url);
    tracing::info!(
        "Destination Topic: {}",
        config
            .destination_topic
            .as_ref()
            .unwrap_or(&"<not set>".to_string())
    );

    // Create connector instance
    let connector = MqttSourceConnector::new();

    // Create and run the runtime
    let mut runtime = SourceRuntime::new(connector, config).await?;

    // Run until shutdown signal
    runtime.run().await?;

    tracing::info!("MQTT Source Connector stopped");
    Ok(())
}

/// Initialize tracing/logging
fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,danube_source_mqtt=debug"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();
}
