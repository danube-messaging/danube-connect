//! Configuration management for connectors.

use crate::{ConnectorError, ConnectorResult};
use danube_client::SubType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

    /// Connector-specific configuration
    #[serde(flatten)]
    pub connector_config: HashMap<String, String>,
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

        // Collect all other env vars for connector-specific config
        let connector_config = env::vars()
            .filter(|(key, _)| {
                !key.starts_with("DANUBE_")
                    && !key.starts_with("CONNECTOR_")
                    && !key.starts_with("SUBSCRIPTION_")
                    && !matches!(
                        key.as_str(),
                        "RELIABLE_DISPATCH"
                            | "MAX_RETRIES"
                            | "RETRY_BACKOFF_MS"
                            | "MAX_BACKOFF_MS"
                            | "BATCH_SIZE"
                            | "BATCH_TIMEOUT_MS"
                            | "POLL_INTERVAL_MS"
                            | "METRICS_PORT"
                            | "LOG_LEVEL"
                            | "RUST_LOG"
                            | "PATH"
                            | "HOME"
                            | "USER"
                            | "SHELL"
                            | "TERM"
                    )
            })
            .collect();

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
            connector_config,
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

    /// Get a string value from connector-specific config
    pub fn get_string(&self, key: &str) -> ConnectorResult<String> {
        self.connector_config
            .get(key)
            .cloned()
            .ok_or_else(|| ConnectorError::config(format!("Missing config key: {}", key)))
    }

    /// Get an optional string value
    pub fn get_optional_string(&self, key: &str) -> Option<String> {
        self.connector_config.get(key).cloned()
    }

    /// Get a parsed value from connector-specific config
    pub fn get<T: std::str::FromStr>(&self, key: &str) -> ConnectorResult<T>
    where
        T::Err: std::fmt::Display,
    {
        let value = self.get_string(key)?;
        value.parse::<T>().map_err(|e| {
            ConnectorError::config(format!("Failed to parse config key {}: {}", key, e))
        })
    }

    /// Get an optional parsed value
    pub fn get_optional<T: std::str::FromStr>(&self, key: &str) -> Option<T>
    where
        T::Err: std::fmt::Display,
    {
        self.get_optional_string(key).and_then(|s| s.parse().ok())
    }

    /// Check if a config key exists
    pub fn contains(&self, key: &str) -> bool {
        self.connector_config.contains_key(key)
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
            connector_config: HashMap::new(),
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

    #[test]
    fn test_config_get_methods() {
        let mut config = ConnectorConfig::default();
        config
            .connector_config
            .insert("KEY1".to_string(), "value1".to_string());
        config
            .connector_config
            .insert("PORT".to_string(), "8080".to_string());

        assert_eq!(config.get_string("KEY1").unwrap(), "value1");
        assert!(config.get_string("KEY2").is_err());

        assert_eq!(
            config.get_optional_string("KEY1"),
            Some("value1".to_string())
        );
        assert_eq!(config.get_optional_string("KEY2"), None);

        assert_eq!(config.get::<u16>("PORT").unwrap(), 8080);
        assert!(config.get::<u16>("KEY1").is_err());

        assert!(config.contains("KEY1"));
        assert!(!config.contains("KEY2"));
    }
}
