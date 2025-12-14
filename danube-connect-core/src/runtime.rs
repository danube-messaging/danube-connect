//! Runtime for managing connector lifecycle.
//!
//! This module provides runtime implementations for both sink and source connectors:
//! - `SinkRuntime`: Handles Danube → External System (consuming from Danube)
//! - `SourceRuntime`: Handles External System → Danube (producing to Danube)
//!
//! The runtimes handle:
//! - Connector initialization
//! - Danube client setup and connection management
//! - Message processing loops
//! - Retry logic
//! - Health monitoring
//! - Graceful shutdown

mod sink_runtime;
mod source_runtime;

pub use sink_runtime::{ConsumerConfig, SinkRuntime};
pub use source_runtime::{ProducerConfig, SourceRuntime};
