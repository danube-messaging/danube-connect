//! Configuration for the MQTT Source Connector

use danube_connect_core::{ConnectorConfig, ConnectorResult};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

/// Unified configuration for MQTT Source Connector
///
/// This struct combines core Danube configuration with MQTT-specific settings
/// in a single, easy-to-use configuration file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttSourceConfig {
    /// Core Danube Connect configuration (flattened at root level)
    #[serde(flatten)]
    pub core: ConnectorConfig,

    /// MQTT-specific configuration
    pub mqtt: MqttConfig,
}

impl MqttSourceConfig {
    /// Load configuration from a single TOML file with optional ENV overrides
    ///
    /// Priority: TOML file → Environment variables
    ///
    /// # Example
    ///
    /// ```toml
    /// # connector.toml - Single file for everything
    /// danube_service_url = "http://broker:6650"
    /// connector_name = "mqtt-source"
    ///
    /// [mqtt]
    /// broker_host = "mosquitto"
    /// # ... mqtt settings
    /// ```
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
        config.mqtt.apply_env_overrides();

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
            mqtt: MqttConfig::from_env()?,
        })
    }

    /// Validate all configuration
    pub fn validate(&self) -> ConnectorResult<()> {
        self.core.validate()?;
        self.mqtt.validate()?;
        Ok(())
    }
}

/// MQTT connector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    /// MQTT broker host
    pub broker_host: String,

    /// MQTT broker port
    #[serde(default = "default_port")]
    pub broker_port: u16,

    /// Client ID for MQTT connection
    pub client_id: String,

    /// Username for authentication (optional)
    pub username: Option<String>,

    /// Password for authentication (optional)
    pub password: Option<String>,

    /// Enable TLS/SSL
    #[serde(default)]
    pub use_tls: bool,

    /// Keep alive interval in seconds
    #[serde(default = "default_keep_alive")]
    pub keep_alive_secs: u64,

    /// Connection timeout in seconds
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_secs: u64,

    /// Maximum message size in bytes
    #[serde(default = "default_max_packet_size")]
    pub max_packet_size: usize,

    /// Topic mappings (MQTT topic -> Danube topic)
    pub topic_mappings: Vec<TopicMapping>,

    /// Clean session on connect
    #[serde(default = "default_true")]
    pub clean_session: bool,

    /// Add MQTT metadata as message attributes
    #[serde(default = "default_true")]
    pub include_metadata: bool,
}

fn default_port() -> u16 {
    1883
}

fn default_keep_alive() -> u64 {
    60
}

fn default_connection_timeout() -> u64 {
    30
}

fn default_max_packet_size() -> usize {
    10 * 1024 * 1024 // 10MB
}

fn default_true() -> bool {
    true
}

impl MqttConfig {
    /// Load configuration from environment variables
    ///
    /// Environment variables:
    /// - `MQTT_BROKER_HOST`: Required, MQTT broker hostname
    /// - `MQTT_BROKER_PORT`: Optional, broker port (default: 1883)
    /// - `MQTT_CLIENT_ID`: Required, client identifier
    /// - `MQTT_USERNAME`: Optional, authentication username
    /// - `MQTT_PASSWORD`: Optional, authentication password
    /// - `MQTT_USE_TLS`: Optional, enable TLS (default: false)
    /// - `MQTT_TOPICS`: Required, comma-separated MQTT topics to subscribe
    /// - `MQTT_DANUBE_TOPIC`: Optional, target Danube topic (uses MQTT topic if not set)
    /// - `MQTT_QOS`: Optional, QoS level 0-2 (default: 1)
    pub fn from_env() -> ConnectorResult<Self> {
        let broker_host = std::env::var("MQTT_BROKER_HOST").map_err(|_| {
            danube_connect_core::ConnectorError::config("MQTT_BROKER_HOST is required")
        })?;

        let broker_port = std::env::var("MQTT_BROKER_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1883);

        let client_id = std::env::var("MQTT_CLIENT_ID").map_err(|_| {
            danube_connect_core::ConnectorError::config("MQTT_CLIENT_ID is required")
        })?;

        let username = std::env::var("MQTT_USERNAME").ok();
        let password = std::env::var("MQTT_PASSWORD").ok();

        let use_tls = std::env::var("MQTT_USE_TLS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(false);

        let keep_alive_secs = std::env::var("MQTT_KEEP_ALIVE_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60);

        let connection_timeout_secs = std::env::var("MQTT_CONNECTION_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        let max_packet_size = std::env::var("MQTT_MAX_PACKET_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10 * 1024 * 1024);

        let clean_session = std::env::var("MQTT_CLEAN_SESSION")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);

        let include_metadata = std::env::var("MQTT_INCLUDE_METADATA")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);

        // Parse topic mappings
        let mqtt_topics = std::env::var("MQTT_TOPICS")
            .map_err(|_| danube_connect_core::ConnectorError::config("MQTT_TOPICS is required"))?;

        let danube_topic = std::env::var("MQTT_DANUBE_TOPIC").ok();

        let qos_level = std::env::var("MQTT_QOS")
            .ok()
            .and_then(|s| s.parse::<u8>().ok())
            .unwrap_or(1);

        let qos = match qos_level {
            0 => QoS::AtMostOnce,
            1 => QoS::AtLeastOnce,
            2 => QoS::ExactlyOnce,
            _ => QoS::AtLeastOnce,
        };

        // Create topic mappings
        let topic_mappings: Vec<TopicMapping> = mqtt_topics
            .split(',')
            .map(|topic| {
                let mqtt_topic = topic.trim().to_string();
                let danube_topic = danube_topic.clone().unwrap_or_else(|| {
                    format!(
                        "/mqtt{}",
                        mqtt_topic.replace('#', "all").replace('+', "any")
                    )
                });

                TopicMapping {
                    mqtt_topic,
                    danube_topic,
                    qos,
                }
            })
            .collect();

        if topic_mappings.is_empty() {
            return Err(danube_connect_core::ConnectorError::config(
                "At least one topic mapping is required",
            ));
        }

        Ok(Self {
            broker_host,
            broker_port,
            client_id,
            username,
            password,
            use_tls,
            keep_alive_secs,
            connection_timeout_secs,
            max_packet_size,
            topic_mappings,
            clean_session,
            include_metadata,
        })
    }

    /// Apply environment variable overrides to MQTT configuration
    pub fn apply_env_overrides(&mut self) {
        if let Ok(val) = env::var("MQTT_BROKER_HOST") {
            self.broker_host = val;
        }
        if let Ok(val) = env::var("MQTT_BROKER_PORT") {
            if let Ok(port) = val.parse() {
                self.broker_port = port;
            }
        }
        if let Ok(val) = env::var("MQTT_CLIENT_ID") {
            self.client_id = val;
        }
        if let Ok(val) = env::var("MQTT_USERNAME") {
            self.username = Some(val);
        }
        if let Ok(val) = env::var("MQTT_PASSWORD") {
            self.password = Some(val);
        }
        if let Ok(val) = env::var("MQTT_USE_TLS") {
            if let Ok(b) = val.parse() {
                self.use_tls = b;
            }
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> ConnectorResult<()> {
        if self.broker_host.is_empty() {
            return Err(danube_connect_core::ConnectorError::config(
                "broker_host cannot be empty",
            ));
        }

        if self.client_id.is_empty() {
            return Err(danube_connect_core::ConnectorError::config(
                "client_id cannot be empty",
            ));
        }

        if self.topic_mappings.is_empty() {
            return Err(danube_connect_core::ConnectorError::config(
                "At least one topic mapping is required",
            ));
        }

        for mapping in &self.topic_mappings {
            if mapping.mqtt_topic.is_empty() {
                return Err(danube_connect_core::ConnectorError::config(
                    "MQTT topic cannot be empty",
                ));
            }
            if mapping.danube_topic.is_empty() {
                return Err(danube_connect_core::ConnectorError::config(
                    "Danube topic cannot be empty",
                ));
            }
        }

        Ok(())
    }

    /// Get MQTT connection options
    pub fn mqtt_options(&self) -> rumqttc::MqttOptions {
        let mut options =
            rumqttc::MqttOptions::new(&self.client_id, &self.broker_host, self.broker_port);

        options.set_keep_alive(Duration::from_secs(self.keep_alive_secs));
        options.set_clean_session(self.clean_session);
        options.set_max_packet_size(self.max_packet_size, self.max_packet_size);

        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            options.set_credentials(username, password);
        }

        options
    }
}

/// MQTT Quality of Service level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum QoS {
    /// At most once delivery
    AtMostOnce = 0,
    /// At least once delivery
    AtLeastOnce = 1,
    /// Exactly once delivery
    ExactlyOnce = 2,
}

impl From<QoS> for rumqttc::QoS {
    fn from(qos: QoS) -> Self {
        match qos {
            QoS::AtMostOnce => rumqttc::QoS::AtMostOnce,
            QoS::AtLeastOnce => rumqttc::QoS::AtLeastOnce,
            QoS::ExactlyOnce => rumqttc::QoS::ExactlyOnce,
        }
    }
}

/// Topic mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicMapping {
    /// MQTT topic pattern (supports wildcards: +, #)
    pub mqtt_topic: String,
    /// Target Danube topic
    pub danube_topic: String,
    /// QoS level for subscription
    #[serde(default = "default_qos")]
    pub qos: QoS,
}

fn default_qos() -> QoS {
    QoS::AtLeastOnce
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qos_conversion() {
        assert_eq!(
            rumqttc::QoS::from(QoS::AtMostOnce),
            rumqttc::QoS::AtMostOnce
        );
        assert_eq!(
            rumqttc::QoS::from(QoS::AtLeastOnce),
            rumqttc::QoS::AtLeastOnce
        );
        assert_eq!(
            rumqttc::QoS::from(QoS::ExactlyOnce),
            rumqttc::QoS::ExactlyOnce
        );
    }

    #[test]
    fn test_config_validation() {
        let mut config = MqttConfig {
            broker_host: "localhost".to_string(),
            broker_port: 1883,
            client_id: "test-client".to_string(),
            username: None,
            password: None,
            use_tls: false,
            keep_alive_secs: 60,
            connection_timeout_secs: 30,
            max_packet_size: 1024 * 1024,
            topic_mappings: vec![TopicMapping {
                mqtt_topic: "sensors/#".to_string(),
                danube_topic: "/mqtt/sensors".to_string(),
                qos: QoS::AtLeastOnce,
            }],
            clean_session: true,
            include_metadata: true,
        };

        assert!(config.validate().is_ok());

        // Test empty broker host
        config.broker_host = "".to_string();
        assert!(config.validate().is_err());

        // Test empty topic mappings
        config.broker_host = "localhost".to_string();
        config.topic_mappings = vec![];
        assert!(config.validate().is_err());
    }
}
