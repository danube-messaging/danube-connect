# SurrealDB Sink Connector

Stream events from Danube into [SurrealDB](https://surrealdb.com/) - the ultimate multi-model database for modern applications.

## Overview

The SurrealDB Sink Connector enables real-time data streaming from Danube topics to SurrealDB tables. Built entirely in Rust for maximum performance and zero JVM overhead.

### Key Features

- ✅ **Multi-Model Support** - Store events as documents, graphs, or relational data
- ✅ **Schema-Aware** - Supports JSON, String, Int64, and Bytes schema types
- ✅ **Multi-Topic Routing** - Route different topics to different tables with independent configurations
- ✅ **Configurable Batching** - Optimize throughput with per-topic batch sizes and flush intervals
- ✅ **Custom Record IDs** - Use message attributes for idempotent inserts or auto-generate
- ✅ **Metadata Enrichment** - Optionally include Danube metadata (topic, offset, timestamp)
- ✅ **Zero-Copy Performance** - Rust-to-Rust with WebSocket protocol
- ✅ **Production Ready** - Health checks, metrics, graceful shutdown

### Use Cases

- **Real-Time Analytics** - Stream user events, IoT sensor data, or system logs
- **Event Sourcing** - Store event streams with full audit trails
- **Graph Relationships** - Build real-time social graphs or recommendation systems
- **Time-Series Data** - Collect metrics, logs, and sensor readings
- **Operational Databases** - Sync data from microservices into SurrealDB

## Quick Start

### Prerequisites

- Danube broker running (see [Danube docs](https://github.com/danrusei/danube))
- SurrealDB instance (see [SurrealDB docs](https://surrealdb.com/docs))
- Rust 1.75+ (for building from source)

### Running with Docker

```bash
docker run -d \
  --name surrealdb-sink \
  -e DANUBE_SERVICE_URL=http://danube-broker:6650 \
  -e SURREALDB_URL=ws://surrealdb:8000 \
  -e SURREALDB_NAMESPACE=default \
  -e SURREALDB_DATABASE=default \
  -e SURREALDB_TOPIC=/default/events \
  -e SURREALDB_SUBSCRIPTION=surrealdb-sink \
  -e SURREALDB_TABLE=events \
  danube/sink-surrealdb:latest
```

### Running from Source

```bash
# Clone the repository
git clone https://github.com/danrusei/danube-connect.git
cd danube-connect/connectors/sink-surrealdb

# Build
cargo build --release

# Run with environment variables
export DANUBE_SERVICE_URL=http://localhost:6650
export SURREALDB_URL=ws://localhost:8000
export SURREALDB_NAMESPACE=default
export SURREALDB_DATABASE=default
export SURREALDB_TOPIC=/default/events
export SURREALDB_SUBSCRIPTION=surrealdb-sink
export SURREALDB_TABLE=events

./target/release/danube-sink-surrealdb
```

### Running with Configuration File

```bash
# Create config file
cp config/connector.toml my-config.toml
# Edit configuration...

# Run with config file
CONNECTOR_CONFIG_PATH=my-config.toml ./target/release/danube-sink-surrealdb
```

## Configuration

### Environment Variables (Single Topic)

For simple single-topic use cases, configure via environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `CONNECTOR_NAME` | Connector instance name | `surrealdb-sink` |
| `DANUBE_SERVICE_URL` | Danube broker URL | `http://localhost:6650` |
| `METRICS_PORT` | Prometheus metrics port | `9090` |
| `SURREALDB_URL` | SurrealDB connection URL (ws:// or http://) | `ws://localhost:8000` |
| `SURREALDB_NAMESPACE` | SurrealDB namespace | `default` |
| `SURREALDB_DATABASE` | SurrealDB database | `default` |
| `SURREALDB_USERNAME` | Optional username | - |
| `SURREALDB_PASSWORD` | Optional password | - |
| `SURREALDB_TOPIC` | Danube topic to consume | `/default/events` |
| `SURREALDB_SUBSCRIPTION` | Subscription name | `surrealdb-sink` |
| `SURREALDB_TABLE` | SurrealDB table name | `events` |
| `SURREALDB_SCHEMA_TYPE` | Schema type (Json/String/Int64/Bytes) | `Json` |
| `SURREALDB_BATCH_SIZE` | Records per batch | `100` |
| `SURREALDB_FLUSH_INTERVAL_MS` | Max time between flushes (ms) | `1000` |
| `SURREALDB_INCLUDE_METADATA` | Include Danube metadata | `true` |

### TOML Configuration (Multi-Topic)

For multi-topic routing and advanced features, use a TOML configuration file:

```toml
connector_name = "surrealdb-sink"
danube_service_url = "http://localhost:6650"
metrics_port = 9090

[surrealdb]
url = "ws://localhost:8000"
namespace = "production"
database = "events"
username = "admin"
password = "password"

batch_size = 100
flush_interval_ms = 1000

# Route user events to user_events table (JSON)
[[surrealdb.topic_mappings]]
topic = "/events/user"
subscription = "surrealdb-user"
table_name = "user_events"
schema_type = "Json"
include_danube_metadata = true
batch_size = 200

# Route IoT data to sensor_readings table (JSON)
[[surrealdb.topic_mappings]]
topic = "/iot/sensors"
subscription = "surrealdb-iot"
table_name = "sensor_readings"
schema_type = "Json"
include_danube_metadata = true
batch_size = 500
flush_interval_ms = 2000
```

See [config/](./config/) directory for complete examples.

## Schema Types

The connector supports Danube's schema system for type-safe data handling:

### JSON Schema (Default)

Most common for structured data:

```toml
schema_type = "Json"
```

**Input:**
```json
{
  "user_id": "user-123",
  "event_type": "login",
  "timestamp": "2024-01-01T12:00:00Z"
}
```

**Stored as-is in SurrealDB**

### String Schema

For plain text messages:

```toml
schema_type = "String"
```

**Input:** `"System started successfully"`

**Stored:** `{"data": "System started successfully"}`

### Int64 Schema

For numeric counters/metrics:

```toml
schema_type = "Int64"
```

**Input:** `42` (8 bytes, big-endian)

**Stored:** `{"value": 42}`

### Bytes Schema

For binary data:

```toml
schema_type = "Bytes"
```

**Input:** Raw bytes `[0x01, 0x02, 0x03]`

**Stored:** `{"data": "AQID", "size": 3}` (base64 encoded)

## Metadata Enrichment

Optionally include Danube metadata for audit trails:

```toml
include_danube_metadata = true
```

**Adds metadata to each record:**

```json
{
  "user_id": "user-123",
  "event_type": "login",
  "_danube_metadata": {
    "danube_topic": "/events/user",
    "danube_offset": 12345,
    "danube_timestamp": "2024-01-01T12:00:01Z",
    "danube_message_id": "topic:/events/user/producer:1/offset:12345"
  }
}
```

**Benefits:**
- Message traceability
- Event ordering
- Debugging
- Data lineage


## Record ID Management

### Auto-Generated IDs (Default)

SurrealDB generates unique IDs automatically:

```rust
// Producer doesn't set record_id attribute
producer.send(payload, None).await?;
```

**Result:** `events:⟨uuid⟩`

### Custom Record IDs (Idempotent)

Set `record_id` attribute in the producer:

```rust
let mut attributes = HashMap::new();
attributes.insert("record_id".to_string(), "order-12345".to_string());
producer.send(payload, Some(attributes)).await?;
```

**Result:** `orders:order-12345`

**Benefits:**
- Idempotent inserts (reprocessing updates existing record)
- Predictable record IDs
- Easy lookups
- Natural keys from source systems

## Performance Tuning

### Batch Size

Control throughput vs latency:

```toml
# High throughput (batch more records)
batch_size = 500
flush_interval_ms = 5000

# Low latency (flush frequently)
batch_size = 10
flush_interval_ms = 100
```

**Recommendations:**
- **High-volume streams**: `batch_size = 500-1000`
- **Real-time analytics**: `batch_size = 50-100`
- **Transactional data**: `batch_size = 10-20`

### Connection Protocol

Use WebSocket for better performance:

```toml
url = "ws://surrealdb:8000"  # Recommended
# url = "http://surrealdb:8000"  # HTTP fallback
```

### Per-Topic Optimization

Different topics can have different performance profiles:

```toml
# Logs: high volume, larger batches
[[surrealdb.topic_mappings]]
topic = "/logs"
batch_size = 1000
flush_interval_ms = 5000

# Orders: low volume, fast flush
[[surrealdb.topic_mappings]]
topic = "/orders"
batch_size = 10
flush_interval_ms = 100
```

## Monitoring

### Prometheus Metrics

The connector exposes metrics on port `9090` (configurable):

```bash
curl http://localhost:9090/metrics
```

**Key Metrics:**
- `danube_connector_records_processed_total` - Total records processed
- `danube_connector_batches_flushed_total` - Total batches flushed
- `danube_connector_errors_total` - Total errors encountered
- `danube_connector_batch_size` - Current batch size histogram
- `danube_connector_flush_duration_seconds` - Flush duration histogram

### Health Checks

Check connector health:

```bash
# Via metrics endpoint
curl http://localhost:9090/health

# Via logs
docker logs -f surrealdb-sink
```

## SurrealDB Integration

### Query Inserted Data

```sql
-- Get all events
SELECT * FROM events;

-- Query with metadata
SELECT * FROM events 
WHERE _danube_metadata.danube_topic = '/events/user' 
ORDER BY _danube_metadata.danube_timestamp DESC 
LIMIT 10;

-- Use custom record IDs
SELECT * FROM events:order-12345;
```

### Graph Relationships

Build relationships from event streams:

```sql
-- Create user and event relationship
RELATE user:$user_id->generated->events:$event_id 
SET timestamp = time::now();
```

### Time-Series Queries

Leverage SurrealDB's time-series capabilities:

```sql
SELECT 
  time::group(_danube_metadata.danube_timestamp, '1h') AS hour,
  count() AS event_count
FROM events
WHERE event_type = 'login'
GROUP BY hour
ORDER BY hour DESC;
```

## Troubleshooting

### Connection Errors

**Symptom:** `Failed to connect to SurrealDB`

**Solutions:**
- Verify SurrealDB is running: `docker ps | grep surrealdb`
- Check URL format: `ws://host:port` or `http://host:port`
- Test connection: `surreal sql --endpoint ws://localhost:8000`

### Authentication Errors

**Symptom:** `SurrealDB authentication failed`

**Solutions:**
- Verify credentials are correct
- Check namespace/database permissions
- Ensure user has write access to target tables

### Batch Insert Failures

**Symptom:** `Failed to insert record`

**Solutions:**
- Check SurrealDB logs for schema validation errors
- Verify record ID uniqueness if using `record_id_field`
- Ensure payload is valid JSON

### Performance Issues

**Symptom:** High latency or lag

**Solutions:**
- Increase `batch_size` for better throughput
- Use WebSocket protocol (`ws://`) instead of HTTP
- Monitor SurrealDB resource usage
- Consider sharding across multiple connector instances

## Examples

See the [examples/](../../examples/) directory for complete examples:

- **Single Topic** - Simple event streaming
- **Multi-Topic** - Multiple topics to different tables
- **Graph Events** - Building relationship graphs
- **Time-Series** - IoT sensor data streaming

## Building

```bash
# Build release binary
cargo build --release

# Run tests
cargo test

# Build Docker image
docker build -t danube/sink-surrealdb:latest .
```

## Architecture

```
┌─────────────────┐
│ Danube Broker   │
│  Topic: /events │
└────────┬────────┘
         │ Stream messages
         ▼
┌─────────────────┐
│ SurrealDB Sink  │
│   Connector     │
│  - Batch        │
│  - Transform    │
│  - Route        │
└────────┬────────┘
         │ Insert records
         ▼
┌─────────────────┐
│   SurrealDB     │
│  Multi-Model DB │
│  - Documents    │
│  - Graphs       │
│  - Time-Series  │
└─────────────────┘
```

## Contributing

Contributions are welcome! Please see the main [danube-connect](https://github.com/danrusei/danube-connect) repository.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
- MIT License ([LICENSE-MIT](../../LICENSE-MIT))

at your option.

## Resources

- [SurrealDB Documentation](https://surrealdb.com/docs)
- [Danube Broker](https://github.com/danrusei/danube)
- [Danube Connect Framework](https://github.com/danrusei/danube-connect)
- [SurrealDB Rust SDK](https://docs.rs/surrealdb/latest/surrealdb/)
