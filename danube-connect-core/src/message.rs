//! Message transformation utilities.
//!
//! This module provides helper types and methods for transforming messages between
//! Danube's format and connector-specific formats.

use crate::{ConnectorError, ConnectorResult, SchemaType};
use danube_core::message::StreamMessage;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Record passed to sink connectors (from Danube → External System)
///
/// This structure separates user data (payload, attributes) from Danube system metadata,
/// making it more efficient and easier to use in sink connectors.
#[derive(Debug, Clone)]
pub struct SinkRecord {
    /// The actual message payload (raw bytes)
    pub payload: Vec<u8>,
    /// User-defined attributes/properties from producer
    pub attributes: HashMap<String, String>,
    /// Danube metadata for observability and debugging
    pub danube_metadata: DanubeMetadata,
    /// Topic partition (if topic is partitioned)
    pub partition: Option<String>,
}

/// Danube-specific metadata for observability and debugging
#[derive(Debug, Clone)]
pub struct DanubeMetadata {
    /// Topic name
    pub topic: String,
    /// Message offset within topic
    pub offset: u64,
    /// Publish timestamp (microseconds since epoch)
    pub publish_time: u64,
    /// Formatted message ID for logging/debugging
    pub message_id: String,
    /// Producer name (for debugging)
    pub producer_name: String,
}

impl SinkRecord {
    /// Convert Danube StreamMessage to SinkRecord
    pub fn from_stream_message(message: StreamMessage, partition: Option<String>) -> Self {
        let message_id = format!(
            "topic:{}/producer:{}/offset:{}",
            message.msg_id.topic_name, message.msg_id.producer_id, message.msg_id.topic_offset
        );

        Self {
            payload: message.payload,
            attributes: message.attributes,
            danube_metadata: DanubeMetadata {
                topic: message.msg_id.topic_name,
                offset: message.msg_id.topic_offset,
                publish_time: message.publish_time,
                message_id,
                producer_name: message.producer_name,
            },
            partition,
        }
    }

    /// Get the payload as bytes
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Get the payload size in bytes
    pub fn payload_size(&self) -> usize {
        self.payload.len()
    }

    /// Deserialize the payload according to schema type
    ///
    /// This method handles the conversion from raw bytes to the appropriate format
    /// based on the Danube schema type. All sink connectors should use this method
    /// to ensure consistent deserialization across the system.
    ///
    /// # Schema Type Handling
    ///
    /// - **Json**: Deserializes to `serde_json::Value`
    /// - **String**: Deserializes to UTF-8 string, wrapped in `{data: "..."}`
    /// - **Int64**: Deserializes big-endian 8 bytes to i64, wrapped in `{value: N}`
    /// - **Bytes**: Encodes as base64, wrapped in `{data: "base64...", size: N}`
    ///
    /// # Example
    ///
    /// ```ignore
    /// // In any sink connector
    /// let data = record.payload_deserialized(mapping.schema_type)?;
    /// // data is now a serde_json::Value ready to insert
    /// ```
    pub fn payload_deserialized(&self, schema_type: SchemaType) -> ConnectorResult<Value> {
        match schema_type {
            SchemaType::Json => self.payload_json(),
            SchemaType::String => {
                let string_value = self.payload_str()?.to_string();
                Ok(json!({
                    "data": string_value
                }))
            }
            SchemaType::Int64 => {
                let value = self.payload_int64()?;
                Ok(json!({
                    "value": value
                }))
            }
            SchemaType::Bytes => {
                let base64_data = self.payload_base64();
                Ok(json!({
                    "data": base64_data,
                    "size": self.payload_size()
                }))
            }
        }
    }

    /// Get the payload as a UTF-8 string (if valid)
    pub fn payload_str(&self) -> ConnectorResult<&str> {
        std::str::from_utf8(&self.payload).map_err(|e| ConnectorError::InvalidData {
            message: format!("Payload is not valid UTF-8: {}", e),
            payload: self.payload.clone(),
        })
    }

    /// Deserialize the payload as JSON
    pub fn payload_json<T: DeserializeOwned>(&self) -> ConnectorResult<T> {
        serde_json::from_slice(&self.payload).map_err(|e| ConnectorError::InvalidData {
            message: format!("Failed to deserialize JSON: {}", e),
            payload: self.payload.clone(),
        })
    }

    /// Deserialize the payload as Int64 (big-endian 8 bytes)
    pub fn payload_int64(&self) -> ConnectorResult<i64> {
        let payload = self.payload();
        if payload.len() != 8 {
            return Err(ConnectorError::invalid_data(
                format!(
                    "Int64 payload must be exactly 8 bytes, got {}",
                    payload.len()
                ),
                payload.to_vec(),
            ));
        }

        // Deserialize as big-endian i64 (Danube convention)
        Ok(i64::from_be_bytes([
            payload[0], payload[1], payload[2], payload[3], payload[4], payload[5], payload[6],
            payload[7],
        ]))
    }

    /// Encode the payload as base64 (for Bytes schema)
    pub fn payload_base64(&self) -> String {
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, self.payload())
    }

    /// Access message attributes (user-defined properties)
    pub fn attributes(&self) -> &HashMap<String, String> {
        &self.attributes
    }

    /// Get a specific attribute value
    pub fn get_attribute(&self, key: &str) -> Option<&str> {
        self.attributes.get(key).map(|s| s.as_str())
    }

    /// Check if an attribute exists
    pub fn has_attribute(&self, key: &str) -> bool {
        self.attributes.contains_key(key)
    }

    /// Get the topic name
    pub fn topic(&self) -> &str {
        &self.danube_metadata.topic
    }

    /// Get the topic offset
    pub fn offset(&self) -> u64 {
        self.danube_metadata.offset
    }

    /// Get the publish timestamp (microseconds since epoch)
    pub fn publish_time(&self) -> u64 {
        self.danube_metadata.publish_time
    }

    /// Get the producer name
    pub fn producer_name(&self) -> &str {
        &self.danube_metadata.producer_name
    }

    /// Get a formatted message ID string for logging
    pub fn message_id(&self) -> &str {
        &self.danube_metadata.message_id
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
    fn test_sink_record_payload_int64() {
        let value: i64 = 42;
        let mut message = create_test_message();
        message.payload = value.to_be_bytes().to_vec();

        let record = SinkRecord::from_stream_message(message, None);
        assert_eq!(record.payload_int64().unwrap(), 42);
    }

    #[test]
    fn test_sink_record_payload_int64_invalid_length() {
        let mut message = create_test_message();
        message.payload = vec![1, 2, 3]; // Only 3 bytes, not 8

        let record = SinkRecord::from_stream_message(message, None);
        assert!(record.payload_int64().is_err());
    }

    #[test]
    fn test_sink_record_payload_base64() {
        let mut message = create_test_message();
        message.payload = vec![0x01, 0x02, 0x03, 0x04];

        let record = SinkRecord::from_stream_message(message, None);
        let base64 = record.payload_base64();

        // Verify it's valid base64
        assert_eq!(base64, "AQIDBA==");
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
