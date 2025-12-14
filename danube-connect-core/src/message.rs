//! Message transformation utilities.
//!
//! This module provides helper types and methods for transforming messages between
//! Danube's format and connector-specific formats.

use crate::{ConnectorError, ConnectorResult};
use danube_core::message::StreamMessage;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;

/// Record passed to sink connectors (from Danube → External System)
#[derive(Debug, Clone)]
pub struct SinkRecord {
    /// The Danube StreamMessage
    pub message: StreamMessage,
    /// The topic partition this message came from (if partitioned)
    pub partition: Option<String>,
    /// Additional connector-specific metadata
    pub metadata: HashMap<String, String>,
}

impl SinkRecord {
    /// Convert Danube StreamMessage to SinkRecord
    pub fn from_stream_message(message: StreamMessage, partition: Option<String>) -> Self {
        Self {
            message,
            partition,
            metadata: HashMap::new(),
        }
    }

    /// Get the payload as bytes
    pub fn payload(&self) -> &[u8] {
        &self.message.payload
    }

    /// Get the payload size in bytes
    pub fn payload_size(&self) -> usize {
        self.message.payload.len()
    }

    /// Get the payload as a UTF-8 string (if valid)
    pub fn payload_str(&self) -> ConnectorResult<&str> {
        std::str::from_utf8(&self.message.payload).map_err(|e| ConnectorError::InvalidData {
            message: format!("Payload is not valid UTF-8: {}", e),
            payload: self.message.payload.clone(),
        })
    }

    /// Get the payload as a String
    pub fn payload_string(&self) -> ConnectorResult<String> {
        self.payload_str().map(|s| s.to_string())
    }

    /// Deserialize the payload as JSON
    pub fn payload_json<T: DeserializeOwned>(&self) -> ConnectorResult<T> {
        serde_json::from_slice(&self.message.payload).map_err(|e| ConnectorError::InvalidData {
            message: format!("Failed to deserialize JSON: {}", e),
            payload: self.message.payload.clone(),
        })
    }

    /// Access message attributes
    pub fn attributes(&self) -> &HashMap<String, String> {
        &self.message.attributes
    }

    /// Get a specific attribute value
    pub fn get_attribute(&self, key: &str) -> Option<&str> {
        self.message.attributes.get(key).map(|s| s.as_str())
    }

    /// Check if an attribute exists
    pub fn has_attribute(&self, key: &str) -> bool {
        self.message.attributes.contains_key(key)
    }

    /// Get the topic name
    pub fn topic(&self) -> &str {
        &self.message.msg_id.topic_name
    }

    /// Get the topic offset
    pub fn offset(&self) -> u64 {
        self.message.msg_id.topic_offset
    }

    /// Get the publish timestamp (microseconds since epoch)
    pub fn publish_time(&self) -> u64 {
        self.message.publish_time
    }

    /// Get the producer name
    pub fn producer_name(&self) -> &str {
        &self.message.producer_name
    }

    /// Get the subscription name (if available)
    pub fn subscription_name(&self) -> Option<&str> {
        self.message.subscription_name.as_deref()
    }

    /// Get the broker address
    pub fn broker_addr(&self) -> &str {
        &self.message.msg_id.broker_addr
    }

    /// Get a formatted message ID string for logging
    pub fn message_id(&self) -> String {
        format!(
            "topic:{}/producer:{}/offset:{}",
            self.message.msg_id.topic_name,
            self.message.msg_id.producer_id,
            self.message.msg_id.topic_offset
        )
    }

    /// Add connector-specific metadata
    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }
}

/// Record passed from source connectors (External System → Danube)
#[derive(Debug, Clone)]
pub struct SourceRecord {
    /// The topic to publish to
    pub topic: String,
    /// The message payload
    pub payload: Vec<u8>,
    /// Optional message attributes/headers
    pub attributes: HashMap<String, String>,
    /// Optional routing key for partitioned topics (will be used when Danube supports it)
    pub key: Option<String>,
    /// Optional producer configuration for this topic (partitions, reliable dispatch)
    /// If not specified, runtime will use default configuration
    pub producer_config: Option<crate::runtime::ProducerConfig>,
}

impl SourceRecord {
    /// Create a new SourceRecord with payload
    pub fn new(topic: impl Into<String>, payload: Vec<u8>) -> Self {
        Self {
            topic: topic.into(),
            payload,
            attributes: HashMap::new(),
            key: None,
            producer_config: None,
        }
    }

    /// Create a SourceRecord from a string payload
    pub fn from_string(topic: impl Into<String>, payload: impl Into<String>) -> Self {
        Self::new(topic, payload.into().into_bytes())
    }

    /// Create a SourceRecord from a JSON-serializable object
    pub fn from_json<T: Serialize>(topic: impl Into<String>, data: &T) -> ConnectorResult<Self> {
        let payload =
            serde_json::to_vec(data).map_err(|e| ConnectorError::Serialization(e.to_string()))?;
        Ok(Self::new(topic, payload))
    }

    /// Add an attribute
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Add multiple attributes
    pub fn with_attributes(mut self, attrs: HashMap<String, String>) -> Self {
        self.attributes.extend(attrs);
        self
    }

    /// Set the routing key for partitioned topics
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Set the producer configuration for this record's destination topic
    pub fn with_producer_config(mut self, config: crate::runtime::ProducerConfig) -> Self {
        self.producer_config = Some(config);
        self
    }

    /// Get the payload size in bytes
    pub fn size(&self) -> usize {
        self.payload.len()
    }

    /// Get the payload as a string slice (if valid UTF-8)
    pub fn payload_str(&self) -> ConnectorResult<&str> {
        std::str::from_utf8(&self.payload).map_err(|e| {
            ConnectorError::Serialization(format!("Payload is not valid UTF-8: {}", e))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use danube_core::message::MessageID;
    use serde::{Deserialize, Serialize};

    fn create_test_message() -> StreamMessage {
        StreamMessage {
            request_id: 1,
            msg_id: MessageID {
                producer_id: 100,
                topic_name: "/default/test".to_string(),
                broker_addr: "localhost:6650".to_string(),
                topic_offset: 42,
            },
            payload: b"test payload".to_vec(),
            publish_time: 1234567890,
            producer_name: "test-producer".to_string(),
            subscription_name: Some("test-sub".to_string()),
            attributes: HashMap::new(),
        }
    }

    #[test]
    fn test_sink_record_basic() {
        let message = create_test_message();
        let record = SinkRecord::from_stream_message(message, None);

        assert_eq!(record.payload(), b"test payload");
        assert_eq!(record.payload_size(), 12);
        assert_eq!(record.topic(), "/default/test");
        assert_eq!(record.offset(), 42);
        assert_eq!(record.producer_name(), "test-producer");
    }

    #[test]
    fn test_sink_record_payload_str() {
        let message = create_test_message();
        let record = SinkRecord::from_stream_message(message, None);

        assert_eq!(record.payload_str().unwrap(), "test payload");
        assert_eq!(record.payload_string().unwrap(), "test payload");
    }

    #[test]
    fn test_sink_record_json() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct TestData {
            name: String,
            value: i32,
        }

        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        let mut message = create_test_message();
        message.payload = serde_json::to_vec(&data).unwrap();

        let record = SinkRecord::from_stream_message(message, None);
        let decoded: TestData = record.payload_json().unwrap();

        assert_eq!(decoded, data);
    }

    #[test]
    fn test_sink_record_attributes() {
        let mut message = create_test_message();
        message
            .attributes
            .insert("key1".to_string(), "value1".to_string());

        let record = SinkRecord::from_stream_message(message, None);

        assert_eq!(record.get_attribute("key1"), Some("value1"));
        assert_eq!(record.get_attribute("key2"), None);
        assert!(record.has_attribute("key1"));
        assert!(!record.has_attribute("key2"));
    }

    #[test]
    fn test_source_record_basic() {
        let record = SourceRecord::new("/default/events", b"test".to_vec());

        assert_eq!(record.topic, "/default/events");
        assert_eq!(record.payload, b"test");
        assert_eq!(record.size(), 4);
        assert!(record.attributes.is_empty());
        assert!(record.key.is_none());
    }

    #[test]
    fn test_source_record_from_string() {
        let record = SourceRecord::from_string("/default/events", "test message");

        assert_eq!(record.payload_str().unwrap(), "test message");
    }

    #[test]
    fn test_source_record_from_json() {
        #[derive(Serialize)]
        struct TestData {
            name: String,
            value: i32,
        }

        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        let record = SourceRecord::from_json("/default/events", &data).unwrap();

        let json: serde_json::Value = serde_json::from_slice(&record.payload).unwrap();
        assert_eq!(json["name"], "test");
        assert_eq!(json["value"], 42);
    }

    #[test]
    fn test_source_record_builder() {
        let record = SourceRecord::new("/default/events", b"test".to_vec())
            .with_attribute("source", "test-connector")
            .with_attribute("version", "1.0")
            .with_key("user-123");

        assert_eq!(
            record.attributes.get("source"),
            Some(&"test-connector".to_string())
        );
        assert_eq!(record.attributes.get("version"), Some(&"1.0".to_string()));
        assert_eq!(record.key, Some("user-123".to_string()));
    }
}
