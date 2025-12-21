//! Configuration module for SurrealDB Sink Connector
//!
//! This module handles all configuration aspects including:
//! - SurrealDB connection settings (URL, namespace, database, credentials)
//! - Topic-to-table mappings with per-table configurations
//! - Batch processing and performance tuning
//! - Environment variable overrides

use danube_connect_core::{ConnectorConfig, ConnectorError, ConnectorResult, SchemaType};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;

/// Complete configuration for the SurrealDB Sink Connector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurrealDBSinkConfig {
    /// Core connector configuration (Danube connection, etc.)
    #[serde(flatten)]
    pub core: ConnectorConfig,

    /// SurrealDB-specific configuration
    pub surrealdb: SurrealDBConfig,
}

/// SurrealDB-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurrealDBConfig {
    /// SurrealDB connection URL (e.g., "ws://localhost:8000", "http://localhost:8000")
    pub url: String,

    /// SurrealDB namespace
    pub namespace: String,

    /// SurrealDB database
    pub database: String,

    /// Optional username for authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// Optional password for authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    /// Connection timeout in seconds
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_secs: u64,

    /// Request timeout in seconds
    #[serde(default = "default_request_timeout")]
    pub request_timeout_secs: u64,

    /// Topic mappings: Danube topics → SurrealDB tables
    #[serde(default)]
    pub topic_mappings: Vec<TopicMapping>,

    /// Global batch size (can be overridden per topic)
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Global flush interval in milliseconds
    #[serde(default = "default_flush_interval_ms")]
    pub flush_interval_ms: u64,
}

/// Mapping from a Danube topic to a SurrealDB table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicMapping {
    /// Danube topic to consume from
    pub topic: String,

    /// Danube subscription name
    pub subscription: String,

    /// SurrealDB table name to insert into
    pub table_name: String,

    /// Include Danube metadata in records (topic, offset, timestamp)
    #[serde(default = "default_include_metadata")]
    pub include_danube_metadata: bool,

    /// Custom batch size for this topic (overrides global)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<usize>,

    /// Custom flush interval for this topic (overrides global)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flush_interval_ms: Option<u64>,

    /// Schema type for this topic (must match Danube topic schema)
    /// Determines how to interpret and insert the payload
    #[serde(default)]
    pub schema_type: SchemaType,
}

// Default value functions
fn default_connection_timeout() -> u64 {
    30
}

fn default_request_timeout() -> u64 {
    30
}

fn default_batch_size() -> usize {
    100
}

fn default_flush_interval_ms() -> u64 {
    1000
}

fn default_include_metadata() -> bool {
    true
}

impl SurrealDBSinkConfig {
    /// Load configuration from TOML file or environment variables
    pub fn load() -> ConnectorResult<Self> {
        // Try to load from config file first
        if let Ok(config_path) = env::var("CONNECTOR_CONFIG_PATH") {
            Self::from_file(&config_path)
        } else {
            // Fall back to environment variables
            Self::from_env()
        }
    }

    /// Load configuration from a TOML file
    pub fn from_file(path: &str) -> ConnectorResult<Self> {
        let contents = fs::read_to_string(path).map_err(|e| {
            ConnectorError::config(format!("Failed to read config file '{}': {}", path, e))
        })?;

        let mut config: Self = toml::from_str(&contents)
            .map_err(|e| ConnectorError::config(format!("Failed to parse TOML config: {}", e)))?;

        // Apply environment variable overrides
        config.apply_env_overrides()?;

        Ok(config)
    }

    /// Load configuration from environment variables (backward compatibility)
    pub fn from_env() -> ConnectorResult<Self> {
        let topic = env::var("SURREALDB_TOPIC").unwrap_or_else(|_| "/default/events".to_string());
        let subscription =
            env::var("SURREALDB_SUBSCRIPTION").unwrap_or_else(|_| "surrealdb-sink".to_string());
        let table_name = env::var("SURREALDB_TABLE").unwrap_or_else(|_| "events".to_string());

        // Create single topic mapping from env vars
        let topic_mapping = TopicMapping {
            topic,
            subscription,
            table_name,
            include_danube_metadata: env::var("SURREALDB_INCLUDE_METADATA")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            batch_size: env::var("SURREALDB_BATCH_SIZE")
                .ok()
                .and_then(|v| v.parse().ok()),
            flush_interval_ms: env::var("SURREALDB_FLUSH_INTERVAL_MS")
                .ok()
                .and_then(|v| v.parse().ok()),
            schema_type: env::var("SURREALDB_SCHEMA_TYPE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(SchemaType::Json),
        };

        let config = SurrealDBSinkConfig {
            core: ConnectorConfig {
                connector_name: env::var("CONNECTOR_NAME")
                    .unwrap_or_else(|_| "surrealdb-sink".to_string()),
                danube_service_url: env::var("DANUBE_SERVICE_URL")
                    .unwrap_or_else(|_| "http://localhost:6650".to_string()),
                retry: Default::default(),
                processing: Default::default(),
            },
            surrealdb: SurrealDBConfig {
                url: env::var("SURREALDB_URL")
                    .unwrap_or_else(|_| "ws://localhost:8000".to_string()),
                namespace: env::var("SURREALDB_NAMESPACE")
                    .unwrap_or_else(|_| "default".to_string()),
                database: env::var("SURREALDB_DATABASE").unwrap_or_else(|_| "default".to_string()),
                username: env::var("SURREALDB_USERNAME").ok(),
                password: env::var("SURREALDB_PASSWORD").ok(),
                connection_timeout_secs: env::var("SURREALDB_CONNECTION_TIMEOUT_SECS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(default_connection_timeout()),
                request_timeout_secs: env::var("SURREALDB_REQUEST_TIMEOUT_SECS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(default_request_timeout()),
                topic_mappings: vec![topic_mapping],
                batch_size: env::var("SURREALDB_BATCH_SIZE")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(default_batch_size()),
                flush_interval_ms: env::var("SURREALDB_FLUSH_INTERVAL_MS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(default_flush_interval_ms()),
            },
        };

        Ok(config)
    }

    /// Apply environment variable overrides to configuration
    fn apply_env_overrides(&mut self) -> ConnectorResult<()> {
        // Core overrides
        if let Ok(name) = env::var("CONNECTOR_NAME") {
            self.core.connector_name = name;
        }
        if let Ok(url) = env::var("DANUBE_SERVICE_URL") {
            self.core.danube_service_url = url;
        }

        // SurrealDB overrides
        if let Ok(url) = env::var("SURREALDB_URL") {
            self.surrealdb.url = url;
        }
        if let Ok(ns) = env::var("SURREALDB_NAMESPACE") {
            self.surrealdb.namespace = ns;
        }
        if let Ok(db) = env::var("SURREALDB_DATABASE") {
            self.surrealdb.database = db;
        }
        if let Ok(username) = env::var("SURREALDB_USERNAME") {
            self.surrealdb.username = Some(username);
        }
        if let Ok(password) = env::var("SURREALDB_PASSWORD") {
            self.surrealdb.password = Some(password);
        }

        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> ConnectorResult<()> {
        // Validate SurrealDB URL
        if self.surrealdb.url.is_empty() {
            return Err(ConnectorError::config("SURREALDB_URL cannot be empty"));
        }

        // Validate namespace and database
        if self.surrealdb.namespace.is_empty() {
            return Err(ConnectorError::config(
                "SURREALDB_NAMESPACE cannot be empty",
            ));
        }
        if self.surrealdb.database.is_empty() {
            return Err(ConnectorError::config("SURREALDB_DATABASE cannot be empty"));
        }

        // Validate topic mappings
        if self.surrealdb.topic_mappings.is_empty() {
            return Err(ConnectorError::config(
                "At least one topic mapping is required",
            ));
        }

        for mapping in &self.surrealdb.topic_mappings {
            if mapping.topic.is_empty() {
                return Err(ConnectorError::config("Topic name cannot be empty"));
            }
            if mapping.subscription.is_empty() {
                return Err(ConnectorError::config("Subscription name cannot be empty"));
            }
            if mapping.table_name.is_empty() {
                return Err(ConnectorError::config("Table name cannot be empty"));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let mut config = SurrealDBSinkConfig {
            core: ConnectorConfig {
                connector_name: "test".to_string(),
                danube_service_url: "http://localhost:6650".to_string(),
                retry: Default::default(),
                processing: Default::default(),
            },
            surrealdb: SurrealDBConfig {
                url: "ws://localhost:8000".to_string(),
                namespace: "test".to_string(),
                database: "test".to_string(),
                username: None,
                password: None,
                connection_timeout_secs: 30,
                request_timeout_secs: 30,
                topic_mappings: vec![TopicMapping {
                    topic: "/test/topic".to_string(),
                    subscription: "test-sub".to_string(),
                    table_name: "events".to_string(),
                    include_danube_metadata: true,
                    batch_size: None,
                    flush_interval_ms: None,
                    schema_type: SchemaType::Json,
                }],
                batch_size: 100,
                flush_interval_ms: 1000,
            },
        };

        assert!(config.validate().is_ok());

        // Test empty URL
        config.surrealdb.url = "".to_string();
        assert!(config.validate().is_err());
        config.surrealdb.url = "ws://localhost:8000".to_string();

        // Test empty topic mappings
        config.surrealdb.topic_mappings.clear();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_default_values() {
        assert_eq!(default_batch_size(), 100);
        assert_eq!(default_flush_interval_ms(), 1000);
        assert_eq!(default_connection_timeout(), 30);
        assert_eq!(default_request_timeout(), 30);
        assert!(default_include_metadata());
    }
}
