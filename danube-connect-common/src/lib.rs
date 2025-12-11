//! Common utilities for Danube connectors.
//!
//! This crate provides shared functionality that can be used across multiple connectors:
//! - Serialization helpers (JSON, Avro, etc.)
//! - Batching utilities
//! - Health check implementations

pub mod batching;
pub mod health;
pub mod serialization;

// Re-export commonly used types
pub use batching::Batcher;
pub use health::HealthChecker;
