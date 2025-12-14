//! Configuration management for connectors.

use crate::{ConnectorError, ConnectorResult};
use danube_client::SubType;
use serde::{Deserialize, Serialize};
use std::env;

/// Subscription type for configuration (mirrors SubType but with Serialize/Deserialize)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionType {
    Exclusive,
    Shared,
    FailOver,
}

impl From<SubscriptionType> for SubType {
    fn from(st: SubscriptionType) -> Self {
        match st {
            SubscriptionType::Exclusive => SubType::Exclusive,
            SubscriptionType::Shared => SubType::Shared,
            SubscriptionType::FailOver => SubType::FailOver,
        }
    }
}

/// Main configuration for connectors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorConfig {
    /// Danube broker service URL
    pub danube_service_url: String,

    /// Connector name (must be unique)
    pub connector_name: String,

    /// Source topic for sink connectors
    pub source_topic: Option<String>,

    /// Subscription name for sink connectors
    pub subscription_name: Option<String>,

    /// Subscription type for sink connectors
    pub subscription_type: Option<SubscriptionType>,

    /// Destination topic for source connectors
    pub destination_topic: Option<String>,

    /// Use reliable dispatch (WAL + Cloud persistence)
    pub reliable_dispatch: bool,

    /// Maximum number of retries for failed operations
    pub max_retries: u32,

    /// Base backoff duration in milliseconds
    pub retry_backoff_ms: u64,

    /// Maximum backoff duration in milliseconds
    pub max_backoff_ms: u64,

    /// Batch size for batch processing
    pub batch_size: usize,

    /// Batch timeout in milliseconds
    pub batch_timeout_ms: u64,

    /// Poll interval in milliseconds for source connectors
    pub poll_interval_ms: u64,

    /// Metrics export port
    pub metrics_port: u16,

    /// Log level
    pub log_level: String,
}

impl ConnectorConfig {
    /// Load configuration from environment variables
    ///
    /// Environment variables:
    /// - `DANUBE_SERVICE_URL`: Required, Danube broker URL
    /// - `CONNECTOR_NAME`: Required, unique connector name
    /// - `DANUBE_TOPIC`: Topic name (source for sink, destination for source)
    /// - `SUBSCRIPTION_NAME`: Subscription name for sink connectors
    /// - `SUBSCRIPTION_TYPE`: Subscription type (Exclusive, Shared, Failover)
    /// - `RELIABLE_DISPATCH`: Enable reliable dispatch (default: true)
    /// - `MAX_RETRIES`: Maximum retries (default: 3)
    /// - `RETRY_BACKOFF_MS`: Base backoff in ms (default: 1000)
    /// - `MAX_BACKOFF_MS`: Max backoff in ms (default: 30000)
    /// - `BATCH_SIZE`: Batch size (default: 1000)
    /// - `BATCH_TIMEOUT_MS`: Batch timeout in ms (default: 1000)
    /// - `POLL_INTERVAL_MS`: Poll interval in ms (default: 100)
    /// - `METRICS_PORT`: Metrics port (default: 9090)
    /// - `LOG_LEVEL`: Log level (default: info)
    ///
    /// All other environment variables are added to `connector_config`
    pub fn from_env() -> ConnectorResult<Self> {
        let danube_service_url = env::var("DANUBE_SERVICE_URL")
            .map_err(|_| ConnectorError::config("DANUBE_SERVICE_URL is required"))?;

        let connector_name = env::var("CONNECTOR_NAME")
            .map_err(|_| ConnectorError::config("CONNECTOR_NAME is required"))?;

        // Topic can be source or destination depending on connector type
        let topic = env::var("DANUBE_TOPIC").ok();

        let subscription_name = env::var("SUBSCRIPTION_NAME").ok();

        let subscription_type =
            env::var("SUBSCRIPTION_TYPE")
                .ok()
                .and_then(|s| match s.to_lowercase().as_str() {
                    "exclusive" => Some(SubscriptionType::Exclusive),
                    "shared" => Some(SubscriptionType::Shared),
                    "failover" => Some(SubscriptionType::FailOver),
                    _ => None,
                });

        let reliable_dispatch = env::var("RELIABLE_DISPATCH")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);

        let max_retries = env::var("MAX_RETRIES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3);

        let retry_backoff_ms = env::var("RETRY_BACKOFF_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1000);

        let max_backoff_ms = env::var("MAX_BACKOFF_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30000);

        let batch_size = env::var("BATCH_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1000);

        let batch_timeout_ms = env::var("BATCH_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1000);

        let poll_interval_ms = env::var("POLL_INTERVAL_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);

        let metrics_port = env::var("METRICS_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(9090);

        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        Ok(Self {
            danube_service_url,
            connector_name,
            source_topic: topic.clone(),
            subscription_name,
            subscription_type,
            destination_topic: topic,
            reliable_dispatch,
            max_retries,
            retry_backoff_ms,
            max_backoff_ms,
            batch_size,
            batch_timeout_ms,
            poll_interval_ms,
            metrics_port,
            log_level,
        })
    }

    /// Load configuration from a TOML file
    pub fn from_file(path: &str) -> ConnectorResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            ConnectorError::config(format!("Failed to read config file {}: {}", path, e))
        })?;

        toml::from_str(&content).map_err(|e| {
            ConnectorError::config(format!("Failed to parse config file {}: {}", path, e))
        })
    }

    /// Apply environment variable overrides to core configuration
    ///
    /// This is a helper method for connectors to apply ENV overrides
    /// after loading from TOML. The core library itself doesn't load files.
    pub fn apply_env_overrides(&mut self) {
        if let Ok(val) = env::var("DANUBE_SERVICE_URL") {
            self.danube_service_url = val;
        }
        if let Ok(val) = env::var("CONNECTOR_NAME") {
            self.connector_name = val;
        }
        if let Ok(val) = env::var("DANUBE_TOPIC") {
            self.source_topic = Some(val.clone());
            self.destination_topic = Some(val);
        }
        if let Ok(val) = env::var("RELIABLE_DISPATCH") {
            if let Ok(b) = val.parse() {
                self.reliable_dispatch = b;
            }
        }
        if let Ok(val) = env::var("MAX_RETRIES") {
            if let Ok(n) = val.parse() {
                self.max_retries = n;
            }
        }
        if let Ok(val) = env::var("POLL_INTERVAL_MS") {
            if let Ok(n) = val.parse() {
                self.poll_interval_ms = n;
            }
        }
        if let Ok(val) = env::var("METRICS_PORT") {
            if let Ok(n) = val.parse() {
                self.metrics_port = n;
            }
        }
        if let Ok(val) = env::var("LOG_LEVEL") {
            self.log_level = val;
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> ConnectorResult<()> {
        if self.danube_service_url.is_empty() {
            return Err(ConnectorError::config("danube_service_url cannot be empty"));
        }

        if self.connector_name.is_empty() {
            return Err(ConnectorError::config("connector_name cannot be empty"));
        }

        if self.max_retries > 100 {
            return Err(ConnectorError::config("max_retries too high (max 100)"));
        }

        if self.batch_size == 0 {
            return Err(ConnectorError::config("batch_size must be > 0"));
        }

        Ok(())
    }
}

impl Default for ConnectorConfig {
    fn default() -> Self {
        Self {
            danube_service_url: "http://localhost:6650".to_string(),
            connector_name: "default-connector".to_string(),
            source_topic: None,
            subscription_name: None,
            subscription_type: None,
            destination_topic: None,
            reliable_dispatch: true,
            max_retries: 3,
            retry_backoff_ms: 1000,
            max_backoff_ms: 30000,
            batch_size: 1000,
            batch_timeout_ms: 1000,
            poll_interval_ms: 100,
            metrics_port: 9090,
            log_level: "info".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ConnectorConfig::default();
        assert_eq!(config.danube_service_url, "http://localhost:6650");
        assert_eq!(config.connector_name, "default-connector");
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.batch_size, 1000);
    }

    #[test]
    fn test_config_validation() {
        let mut config = ConnectorConfig::default();
        assert!(config.validate().is_ok());

        config.danube_service_url = "".to_string();
        assert!(config.validate().is_err());

        config.danube_service_url = "http://localhost:6650".to_string();
        config.batch_size = 0;
        assert!(config.validate().is_err());
    }
}
