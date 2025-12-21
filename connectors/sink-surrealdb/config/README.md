# SurrealDB Sink Connector Configuration Guide

Complete reference for configuring the SurrealDB Sink Connector.

## Table of Contents

- [Configuration Methods](#configuration-methods)
- [Core Settings](#core-settings)
- [SurrealDB Connection](#surrealdb-connection)
- [Topic Mappings](#topic-mappings)
- [Schema Types](#schema-types)
- [Batch Processing](#batch-processing)
- [Environment Variables](#environment-variables)
- [Performance Tuning](#performance-tuning)
- [Examples](#examples)

## Configuration Methods

### 1. TOML Configuration File (Recommended)

Best for multi-topic scenarios:

```bash
CONNECTOR_CONFIG_PATH=/path/to/connector.toml danube-sink-surrealdb
```

### 2. Environment Variables

Best for simple single-topic deployments:

```bash
export SURREALDB_URL=ws://localhost:8000
export SURREALDB_TOPIC=/default/events
export SURREALDB_TABLE=events
danube-sink-surrealdb
```

### 3. Hybrid Approach

TOML file with environment variable overrides:

```bash
# Base configuration from file
CONNECTOR_CONFIG_PATH=config.toml \
# Override specific values
SURREALDB_URL=ws://production-db:8000 \
danube-sink-surrealdb
```

## Core Settings

These settings configure the connector instance and Danube connection:

```toml
# Connector instance name (appears in logs and metrics)
connector_name = "surrealdb-sink-production"

# Danube broker service URL
danube_service_url = "http://localhost:6650"

# Prometheus metrics port
metrics_port = 9090
```

### Environment Variable Overrides

| Variable | TOML Key | Default |
|----------|----------|---------|
| `CONNECTOR_NAME` | `connector_name` | `surrealdb-sink` |
| `DANUBE_SERVICE_URL` | `danube_service_url` | `http://localhost:6650` |
| `METRICS_PORT` | `metrics_port` | `9090` |

## SurrealDB Connection

Configure the connection to your SurrealDB instance:

```toml
[surrealdb]
# Connection URL - supports ws:// and http://
# WebSocket (ws://) is recommended for better performance
url = "ws://localhost:8000"

# SurrealDB namespace (isolated environment)
namespace = "production"

# SurrealDB database within the namespace
database = "events"

# Optional authentication
username = "admin"
password = "secure_password"

# Timeouts in seconds
connection_timeout_secs = 30
request_timeout_secs = 30
```

### Connection Protocols

**WebSocket (Recommended):**
```toml
url = "ws://surrealdb:8000"
```
- ✅ Best performance
- ✅ Persistent connection
- ✅ Lower latency

**HTTP:**
```toml
url = "http://surrealdb:8000"
```
- ✅ Firewall-friendly
- ✅ Load balancer compatible
- ⚠️ Higher overhead

### Authentication

**No Authentication:**
```toml
# username and password omitted
```

**Root Authentication:**
```toml
username = "root"
password = "root_password"
```

**Namespace/Database User:**
```toml
username = "app_user"
password = "app_password"
namespace = "app_namespace"
database = "app_database"
```

### Environment Variable Overrides

| Variable | TOML Key | Default |
|----------|----------|---------|
| `SURREALDB_URL` | `surrealdb.url` | `ws://localhost:8000` |
| `SURREALDB_NAMESPACE` | `surrealdb.namespace` | `default` |
| `SURREALDB_DATABASE` | `surrealdb.database` | `default` |
| `SURREALDB_USERNAME` | `surrealdb.username` | - |
| `SURREALDB_PASSWORD` | `surrealdb.password` | - |

## Topic Mappings

Each topic mapping defines how a Danube topic streams to a SurrealDB table.

### Basic Mapping

```toml
[[surrealdb.topic_mappings]]
topic = "/events/user"
subscription = "surrealdb-user-events"
table_name = "user_events"
```

### With Custom Record ID

Use message attributes to specify record IDs (set by producer):

```toml
[[surrealdb.topic_mappings]]
topic = "/orders/created"
subscription = "surrealdb-orders"
table_name = "orders"
schema_type = "Json"
```

**Producer sets attribute:**
```rust
let mut attributes = HashMap::new();
attributes.insert("record_id".to_string(), "ord-12345".to_string());
producer.send(payload, Some(attributes)).await?;
```

**Result in SurrealDB:**
- Record ID: `orders:ord-12345`
- Idempotent: re-processing same message updates existing record

### Auto-Generated IDs

If no `record_id` attribute is present, SurrealDB auto-generates IDs:

```toml
[[surrealdb.topic_mappings]]
topic = "/logs/system"
subscription = "surrealdb-logs"
table_name = "system_logs"
schema_type = "String"
```

**Result:** `system_logs:⟨uuid⟩`

### Metadata Enrichment

Include Danube metadata in each record:

```toml
[[surrealdb.topic_mappings]]
topic = "/events/user"
subscription = "surrealdb-user"
table_name = "user_events"
include_danube_metadata = true  # Default: true
```

**Added Fields:**
```json
{
  "...": "payload fields",
  "_danube_metadata": {
    "danube_topic": "/events/user",
    "danube_offset": 12345,
    "danube_timestamp": "2024-01-01T12:00:00Z",
    "danube_message_id": "msg-abc123"
  }
}
```

**Use Cases:**
- Audit trails
- Debugging
- Event ordering
- Data lineage

### Environment Variables (Single Topic Only)

| Variable | TOML Key | Default |
|----------|----------|---------|
| `SURREALDB_TOPIC` | `topic` | `/default/events` |
| `SURREALDB_SUBSCRIPTION` | `subscription` | `surrealdb-sink` |
| `SURREALDB_TABLE` | `table_name` | `events` |
| `SURREALDB_SCHEMA_TYPE` | `schema_type` | `Json` |
| `SURREALDB_INCLUDE_METADATA` | `include_danube_metadata` | `true` |

## Schema Types

The connector supports Danube's schema system for type-safe data handling:

### Available Schema Types

**Json** (Default):
```toml
schema_type = "Json"
```
Deserializes JSON payloads directly. Most common for structured data.

**String**:
```toml
schema_type = "String"
```
Deserializes UTF-8 strings, wrapped as `{data: "..."}`.

**Int64**:
```toml
schema_type = "Int64"
```
Deserializes 8-byte big-endian integers, wrapped as `{value: N}`.

**Bytes**:
```toml
schema_type = "Bytes"
```
Encodes raw bytes as base64, wrapped as `{data: "base64...", size: N}`.

### Schema Type Examples

**JSON Schema (Default):**
```toml
[[surrealdb.topic_mappings]]
topic = "/events/user"
schema_type = "Json"
```

Producer sends:
```json
{"user_id": "123", "action": "login"}
```

Stored in SurrealDB:
```json
{"user_id": "123", "action": "login"}
```

**String Schema:**
```toml
[[surrealdb.topic_mappings]]
topic = "/logs/system"
schema_type = "String"
```

Producer sends:
```
"System started successfully"
```

Stored in SurrealDB:
```json
{"data": "System started successfully"}
```

**Int64 Schema:**
```toml
[[surrealdb.topic_mappings]]
topic = "/metrics/counter"
schema_type = "Int64"
```

Producer sends: `42` (as 8 bytes)

Stored in SurrealDB:
```json
{"value": 42}
```
3. Flattening applied (if enabled)
4. Metadata added (if enabled)

## Batch Processing

Control throughput and latency with batch configuration.

### Global Batch Settings

Applied to all topics unless overridden:

```toml
[surrealdb]
# Flush when batch reaches this size
batch_size = 100

# Flush at least every N milliseconds
flush_interval_ms = 1000
```

### Per-Topic Batch Settings

Override for specific topics:

```toml
# High-volume topic: larger batches
[[surrealdb.topic_mappings]]
topic = "/iot/sensors"
subscription = "surrealdb-iot"
table_name = "sensor_readings"
batch_size = 500
flush_interval_ms = 5000

# Real-time topic: smaller batches, faster flush
[[surrealdb.topic_mappings]]
topic = "/alerts/critical"
subscription = "surrealdb-alerts"
table_name = "critical_alerts"
batch_size = 10
flush_interval_ms = 100
```

### Batch Behavior

**Flush Triggers:**
1. Batch size reached (`batch_size` records buffered)
2. Flush interval elapsed (`flush_interval_ms` milliseconds since last flush)
3. Connector shutdown (graceful flush of remaining records)

**Example:**
```toml
batch_size = 100
flush_interval_ms = 1000
```

- If 100 records arrive in 500ms → flush at 100 records
- If 50 records arrive in 1000ms → flush at 1000ms
- If 50 records buffered on shutdown → flush immediately

## Environment Variables

### Complete Environment Variable Reference

**Core:**
- `CONNECTOR_NAME` - Connector instance name
- `DANUBE_SERVICE_URL` - Danube broker URL
- `METRICS_PORT` - Prometheus metrics port

**SurrealDB:**
- `SURREALDB_URL` - SurrealDB connection URL
- `SURREALDB_NAMESPACE` - SurrealDB namespace
- `SURREALDB_DATABASE` - SurrealDB database
- `SURREALDB_USERNAME` - Optional username
- `SURREALDB_PASSWORD` - Optional password
- `SURREALDB_CONNECTION_TIMEOUT_SECS` - Connection timeout
- `SURREALDB_REQUEST_TIMEOUT_SECS` - Request timeout

**Topic Mapping (Single Topic Only):**
- `SURREALDB_TOPIC` - Danube topic
- `SURREALDB_SUBSCRIPTION` - Subscription name
- `SURREALDB_TABLE` - SurrealDB table name
- `SURREALDB_RECORD_ID_FIELD` - Payload field for record ID
- `SURREALDB_INCLUDE_METADATA` - Include Danube metadata
- `SURREALDB_BATCH_SIZE` - Batch size
- `SURREALDB_FLUSH_INTERVAL_MS` - Flush interval
- `SURREALDB_FLATTEN_PAYLOAD` - Flatten nested objects

**Note:** For multi-topic configurations, use a TOML file.

## Performance Tuning

### High Throughput (Batch Processing)

Optimize for maximum throughput:

```toml
[surrealdb]
url = "ws://surrealdb:8000"  # WebSocket for speed
batch_size = 1000
flush_interval_ms = 5000
connection_timeout_secs = 60
request_timeout_secs = 60

[[surrealdb.topic_mappings]]
topic = "/logs/application"
batch_size = 1000
flush_interval_ms = 5000
include_danube_metadata = false  # Reduce payload size
```

**Characteristics:**
- ✅ High throughput (10k+ records/sec)
- ⚠️ Higher latency (up to 5 seconds)
- ✅ Lower SurrealDB write load

### Low Latency (Real-Time)

Optimize for minimum latency:

```toml
[surrealdb]
url = "ws://surrealdb:8000"
batch_size = 10
flush_interval_ms = 100
connection_timeout_secs = 10
request_timeout_secs = 10

[[surrealdb.topic_mappings]]
topic = "/events/critical"
batch_size = 1
flush_interval_ms = 10
include_danube_metadata = true
```

**Characteristics:**
- ✅ Low latency (< 100ms)
- ⚠️ Lower throughput
- ⚠️ Higher SurrealDB write load

### Balanced (Production Default)

Balance throughput and latency:

```toml
[surrealdb]
url = "ws://surrealdb:8000"
batch_size = 100
flush_interval_ms = 1000
connection_timeout_secs = 30
request_timeout_secs = 30
```

**Characteristics:**
- ✅ Good throughput (1k-5k records/sec)
- ✅ Reasonable latency (< 1 second)
- ✅ Moderate SurrealDB load

## Examples

### Example 1: Simple Event Streaming

```toml
connector_name = "events-to-surrealdb"
danube_service_url = "http://localhost:6650"

[surrealdb]
url = "ws://localhost:8000"
namespace = "app"
database = "events"

[[surrealdb.topic_mappings]]
topic = "/events/user"
subscription = "surrealdb-events"
table_name = "user_events"
include_danube_metadata = true
```

### Example 2: E-Commerce Orders

```toml
connector_name = "orders-sync"
danube_service_url = "http://danube:6650"

[surrealdb]
url = "ws://surrealdb:8000"
namespace = "ecommerce"
database = "production"
username = "app"
password = "secret"

[[surrealdb.topic_mappings]]
topic = "/orders/created"
subscription = "surrealdb-orders"
table_name = "orders"
schema_type = "Json"
include_danube_metadata = false  # Clean records
batch_size = 100
```

### Example 3: IoT Time-Series Data

```toml
connector_name = "iot-data-sink"
danube_service_url = "http://iot-broker:6650"

[surrealdb]
url = "ws://timeseries-db:8000"
namespace = "iot"
database = "sensors"
batch_size = 500  # High volume
flush_interval_ms = 2000

[[surrealdb.topic_mappings]]
topic = "/iot/temperature"
subscription = "surrealdb-temp"
table_name = "temperature_readings"
schema_type = "Json"
batch_size = 1000  # Override for high volume
flush_interval_ms = 5000
include_danube_metadata = true
```

### Example 4: Multi-Topic with Different Tables

```toml
connector_name = "multi-stream-sink"
danube_service_url = "http://localhost:6650"

[surrealdb]
url = "ws://localhost:8000"
namespace = "analytics"
database = "main"

# User events (JSON)
[[surrealdb.topic_mappings]]
topic = "/events/user"
subscription = "surrealdb-user"
table_name = "user_events"
schema_type = "Json"
batch_size = 200

# System logs (String)
[[surrealdb.topic_mappings]]
topic = "/logs/system"
subscription = "surrealdb-logs"
table_name = "system_logs"
schema_type = "String"
batch_size = 500
flush_interval_ms = 5000

# Metrics (Int64)
[[surrealdb.topic_mappings]]
topic = "/metrics/counter"
subscription = "surrealdb-metrics"
table_name = "metrics"
schema_type = "Int64"
batch_size = 10
flush_interval_ms = 100
```

## Validation

The connector validates configuration on startup:

**Required Fields:**
- `surrealdb.url`
- `surrealdb.namespace`
- `surrealdb.database`
- At least one `topic_mappings` entry

**Per Mapping:**
- `topic` (non-empty)
- `subscription` (non-empty)
- `table_name` (non-empty)

**Error Example:**
```
Failed to load configuration: SURREALDB_URL cannot be empty
```

## Best Practices

1. **Use WebSocket protocol** (`ws://`) for best performance
2. **Specify schema types** matching your Danube topic schemas
3. **Enable metadata** for audit trails and debugging
4. **Use record_id attributes** (set by producer) for idempotent inserts
5. **Tune batch sizes** based on message volume
6. **Monitor metrics** via Prometheus endpoint
7. **Use TOML files** for multi-topic configurations
8. **Secure credentials** - never commit passwords to git

## Troubleshooting

See the main [README.md](../README.md#troubleshooting) for troubleshooting guidance.
