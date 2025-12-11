//! # Danube Connect Core
//!
//! Core SDK for building Danube connectors.
//!
//! This library provides the foundational framework for creating connectors that integrate
//! external systems with Danube Messaging. It handles all the complexity of communicating
//! with Danube brokers, managing lifecycles, retries, and observability, allowing connector
//! developers to focus solely on the integration logic.
//!
//! ## Overview
//!
//! Connectors are standalone processes that either:
//! - **Sink**: Consume messages from Danube and write to an external system
//! - **Source**: Read from an external system and publish to Danube
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use danube_connect_core::{SinkConnector, SinkRecord, ConnectorConfig, ConnectorResult};
//! use async_trait::async_trait;
//!
//! pub struct MyConnector;
//!
//! #[async_trait]
//! impl SinkConnector for MyConnector {
//!     async fn initialize(&mut self, config: ConnectorConfig) -> ConnectorResult<()> {
//!         // Setup your connector
//!         Ok(())
//!     }
//!     
//!     async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
//!         // Process message
//!         println!("Got message: {:?}", record.payload());
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ## Features
//!
//! - **Automatic Lifecycle Management**: The runtime handles initialization, message loops, and shutdown
//! - **Built-in Retry Logic**: Configurable exponential backoff for transient failures
//! - **Message Transformation**: Helpers for JSON, binary, and schema-based transformations
//! - **Observability**: Automatic metrics, structured logging, and health checks
//! - **Configuration**: Standard environment variable and file-based configuration

mod config;
mod error;
mod message;
mod metrics;
mod retry;
mod runtime;
mod traits;

// Re-export public API
pub use config::{ConnectorConfig, SubscriptionType};
pub use error::{ConnectorError, ConnectorResult};
pub use message::{SinkRecord, SourceRecord};
pub use metrics::ConnectorMetrics;
pub use retry::{RetryConfig, RetryStrategy};
pub use runtime::{SinkRuntime, SourceRuntime};
pub use traits::{Offset, SinkConnector, SourceConnector};

// Re-export commonly used types from danube-client
pub use danube_client::SubType;
pub use danube_core::message::{MessageID, StreamMessage};

// Version info
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");
