//! Configuration for the Qdrant Sink Connector

use danube_connect_core::{ConnectorConfig, ConnectorResult, SubscriptionType};
use serde::{Deserialize, Serialize};
use std::env;

/// Unified configuration for Qdrant Sink Connector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantSinkConfig {
    /// Core Danube Connect configuration (flattened at root level)
    #[serde(flatten)]
    pub core: ConnectorConfig,

    /// Qdrant-specific configuration
    pub qdrant: QdrantConfig,
}

impl QdrantSinkConfig {
    /// Load configuration from a single TOML file with optional ENV overrides
    pub fn load() -> ConnectorResult<Self> {
        // Try to load from file first
        let mut config = if let Ok(config_file) = env::var("CONFIG_FILE") {
            Self::from_file(&config_file)?
        } else {
            // Fallback to environment variables
            Self::from_env()?
        };

        // Apply environment variable overrides
        config.core.apply_env_overrides();
        config.qdrant.apply_env_overrides();

        Ok(config)
    }

    /// Load configuration from a TOML file
    pub fn from_file(path: &str) -> ConnectorResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            danube_connect_core::ConnectorError::config(format!(
                "Failed to read config file {}: {}",
                path, e
            ))
        })?;

        toml::from_str(&content).map_err(|e| {
            danube_connect_core::ConnectorError::config(format!(
                "Failed to parse config file {}: {}",
                path, e
            ))
        })
    }

    /// Load configuration from environment variables
    pub fn from_env() -> ConnectorResult<Self> {
        Ok(Self {
            core: ConnectorConfig::from_env()?,
            qdrant: QdrantConfig::from_env()?,
        })
    }

    /// Validate all configuration
    pub fn validate(&self) -> ConnectorResult<()> {
        self.core.validate()?;
        self.qdrant.validate()?;
        Ok(())
    }
}

/// Qdrant connector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantConfig {
    /// Qdrant server URL (gRPC endpoint)
    pub url: String,

    /// Optional API key for Qdrant Cloud
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Topic mappings: Danube topic → Qdrant collection configuration
    pub topic_mappings: Vec<TopicMapping>,

    /// Global batch size for bulk upserts (10-500)
    /// Can be overridden per topic
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Global batch timeout in milliseconds
    /// Can be overridden per topic
    #[serde(default = "default_batch_timeout")]
    pub batch_timeout_ms: u64,

    /// Timeout for Qdrant operations in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

/// Topic mapping configuration: Danube topic → Qdrant collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicMapping {
    /// Danube topic to consume from (format: /{namespace}/{topic_name})
    pub topic: String,

    /// Subscription name for this topic
    pub subscription: String,

    /// Subscription type (default: Exclusive)
    #[serde(default = "default_subscription_type")]
    pub subscription_type: SubscriptionType,

    /// Target Qdrant collection name
    pub collection_name: String,

    /// Vector dimension (must match embedding model for this topic)
    pub vector_dimension: usize,

    /// Distance metric for this collection
    #[serde(default = "default_distance")]
    pub distance: Distance,

    /// Automatically create collection if it doesn't exist
    #[serde(default = "default_auto_create")]
    pub auto_create_collection: bool,

    /// Include Danube metadata in payload for this topic
    #[serde(default = "default_include_metadata")]
    pub include_danube_metadata: bool,

    /// Topic-specific batch size (overrides global)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<usize>,

    /// Topic-specific batch timeout (overrides global)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_timeout_ms: Option<u64>,
}

impl TopicMapping {
    /// Get effective batch size (topic-specific or global)
    pub fn effective_batch_size(&self, global: usize) -> usize {
        self.batch_size.unwrap_or(global)
    }

    /// Get effective batch timeout (topic-specific or global)
    pub fn effective_batch_timeout(&self, global: u64) -> u64 {
        self.batch_timeout_ms.unwrap_or(global)
    }
}

fn default_distance() -> Distance {
    Distance::Cosine
}

fn default_batch_size() -> usize {
    100
}

fn default_batch_timeout() -> u64 {
    1000
}

fn default_auto_create() -> bool {
    true
}

fn default_include_metadata() -> bool {
    true
}

fn default_timeout() -> u64 {
    30
}

fn default_subscription_type() -> SubscriptionType {
    SubscriptionType::Exclusive
}

/// Distance metric for vector similarity
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Distance {
    /// Cosine similarity (most common for embeddings)
    Cosine,
    /// Euclidean distance
    Euclid,
    /// Dot product
    Dot,
    /// Manhattan distance
    Manhattan,
}

impl Distance {
    pub fn to_qdrant(&self) -> qdrant_client::qdrant::Distance {
        match self {
            Distance::Cosine => qdrant_client::qdrant::Distance::Cosine,
            Distance::Euclid => qdrant_client::qdrant::Distance::Euclid,
            Distance::Dot => qdrant_client::qdrant::Distance::Dot,
            Distance::Manhattan => qdrant_client::qdrant::Distance::Manhattan,
        }
    }
}

impl QdrantConfig {
    /// Load configuration from environment variables (backward compatible single topic)
    pub fn from_env() -> ConnectorResult<Self> {
        let url = env::var("QDRANT_URL").map_err(|_| {
            danube_connect_core::ConnectorError::config("QDRANT_URL is required")
        })?;

        let api_key = env::var("QDRANT_API_KEY").ok();

        // Backward compatibility: single topic configuration via env vars
        let topic = env::var("QDRANT_TOPIC").unwrap_or_else(|_| "/default/vectors".to_string());
        let subscription = env::var("QDRANT_SUBSCRIPTION")
            .unwrap_or_else(|_| "qdrant-sink-sub".to_string());

        let collection_name = env::var("QDRANT_COLLECTION").map_err(|_| {
            danube_connect_core::ConnectorError::config("QDRANT_COLLECTION is required")
        })?;

        let vector_dimension = env::var("QDRANT_VECTOR_DIMENSION")
            .map_err(|_| {
                danube_connect_core::ConnectorError::config("QDRANT_VECTOR_DIMENSION is required")
            })?
            .parse()
            .map_err(|_| {
                danube_connect_core::ConnectorError::config(
                    "QDRANT_VECTOR_DIMENSION must be a valid number",
                )
            })?;

        let distance = env::var("QDRANT_DISTANCE")
            .ok()
            .and_then(|s| match s.to_lowercase().as_str() {
                "cosine" => Some(Distance::Cosine),
                "euclid" => Some(Distance::Euclid),
                "dot" => Some(Distance::Dot),
                "manhattan" => Some(Distance::Manhattan),
                _ => None,
            })
            .unwrap_or(Distance::Cosine);

        let batch_size = env::var("QDRANT_BATCH_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);

        let batch_timeout_ms = env::var("QDRANT_BATCH_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1000);

        let auto_create_collection = env::var("QDRANT_AUTO_CREATE_COLLECTION")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);

        let include_danube_metadata = env::var("QDRANT_INCLUDE_DANUBE_METADATA")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);

        let timeout_secs = env::var("QDRANT_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        // Create single topic mapping from env vars
        let topic_mapping = TopicMapping {
            topic,
            subscription,
            subscription_type: SubscriptionType::Exclusive,
            collection_name,
            vector_dimension,
            distance,
            auto_create_collection,
            include_danube_metadata,
            batch_size: None,
            batch_timeout_ms: None,
        };

        Ok(Self {
            url,
            api_key,
            topic_mappings: vec![topic_mapping],
            batch_size,
            batch_timeout_ms,
            timeout_secs,
        })
    }

    /// Apply environment variable overrides to Qdrant configuration
    pub fn apply_env_overrides(&mut self) {
        if let Ok(val) = env::var("QDRANT_URL") {
            self.url = val;
        }
        if let Ok(val) = env::var("QDRANT_API_KEY") {
            self.api_key = Some(val);
        }
        // Topic-specific overrides would need to be handled per mapping
        // For simplicity, global settings can be overridden here
    }

    /// Validate the configuration
    pub fn validate(&self) -> ConnectorResult<()> {
        if self.url.is_empty() {
            return Err(danube_connect_core::ConnectorError::config(
                "Qdrant URL cannot be empty",
            ));
        }

        if self.topic_mappings.is_empty() {
            return Err(danube_connect_core::ConnectorError::config(
                "At least one topic mapping is required",
            ));
        }

        // Validate each topic mapping
        for (idx, mapping) in self.topic_mappings.iter().enumerate() {
            if mapping.topic.is_empty() {
                return Err(danube_connect_core::ConnectorError::config(
                    format!("Topic mapping {} has empty topic", idx),
                ));
            }

            if mapping.collection_name.is_empty() {
                return Err(danube_connect_core::ConnectorError::config(
                    format!("Topic mapping {} has empty collection name", idx),
                ));
            }

            if mapping.vector_dimension == 0 {
                return Err(danube_connect_core::ConnectorError::config(
                    format!("Topic mapping {} has zero vector dimension", idx),
                ));
            }

            if mapping.subscription.is_empty() {
                return Err(danube_connect_core::ConnectorError::config(
                    format!("Topic mapping {} has empty subscription", idx),
                ));
            }
        }

        if self.batch_size == 0 {
            return Err(danube_connect_core::ConnectorError::config(
                "Batch size must be greater than 0",
            ));
        }

        Ok(())
    }

    /// Create Qdrant client configuration
    pub fn qdrant_client_config(&self) -> qdrant_client::config::QdrantConfig {
        let mut builder = qdrant_client::config::QdrantConfig::from_url(&self.url);

        if let Some(ref api_key) = self.api_key {
            builder.set_api_key(api_key);
        }

        builder.set_timeout(std::time::Duration::from_secs(self.timeout_secs));

        builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let mut config = QdrantConfig {
            url: "http://localhost:6334".to_string(),
            api_key: None,
            topic_mappings: vec![TopicMapping {
                topic: "/default/vectors".to_string(),
                subscription: "qdrant-sink-sub".to_string(),
                subscription_type: SubscriptionType::Exclusive,
                collection_name: "test_collection".to_string(),
                vector_dimension: 1536,
                distance: Distance::Cosine,
                auto_create_collection: true,
                include_danube_metadata: true,
                batch_size: None,
                batch_timeout_ms: None,
            }],
            batch_size: 100,
            batch_timeout_ms: 1000,
            timeout_secs: 30,
        };

        assert!(config.validate().is_ok());

        // Test empty URL
        config.url = "".to_string();
        assert!(config.validate().is_err());

        // Test empty topic mappings
        config.url = "http://localhost:6334".to_string();
        config.topic_mappings = vec![];
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_distance_conversion() {
        assert_eq!(
            Distance::Cosine.to_qdrant(),
            qdrant_client::qdrant::Distance::Cosine
        );
        assert_eq!(
            Distance::Euclid.to_qdrant(),
            qdrant_client::qdrant::Distance::Euclid
        );
    }
}
