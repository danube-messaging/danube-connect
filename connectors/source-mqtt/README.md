# MQTT Source Connector for Danube Connect

A high-performance MQTT source connector that bridges MQTT-based IoT devices with Danube Messaging. Subscribes to MQTT topics and publishes messages to Danube topics with full metadata preservation.

## 📚 Documentation

- **[Quick Start & Testing](../../examples/source-mqtt/)** - Complete example setup with Docker Compose
- **[Development Guide](./DEVELOPMENT.md)** - Build, test, and contribute to the connector

## 🚀 Quick Start

### Running with Docker

```bash
docker run -d \
  --name mqtt-source \
  -v $(pwd)/connector.toml:/etc/connector.toml:ro \
  -e CONNECTOR_CONFIG_PATH=/etc/connector.toml \
  -e DANUBE_SERVICE_URL=http://danube-broker:6650 \
  -e CONNECTOR_NAME=mqtt-source \
  -e MQTT_BROKER_HOST=mosquitto \
  -e MQTT_USERNAME=user \
  -e MQTT_PASSWORD=password \
  danube/source-mqtt:latest
```

**Note:** All structural configuration (topic mappings, QoS, partitions) must be in `connector.toml`. See [Configuration](#configuration) section below.

### Running from Source

```bash
# Clone the repository
cd connectors/source-mqtt

# Build the connector
cargo build --release

# Run with configuration file
CONNECTOR_CONFIG_PATH=/path/to/connector.toml ./target/release/danube-source-mqtt
```

**Want to see it in action?** Check out the [integration testing example](../../examples/source-mqtt/) with a complete Docker Compose setup including MQTT broker, Danube, and sample data publishers.

## ✨ Features

- ✅ **MQTT 3.1.1 Protocol** - Full support via rumqttc client
- ✅ **Wildcard Subscriptions** - `+` (single-level) and `#` (multi-level) patterns
- ✅ **All QoS Levels** - QoS 0 (fire-and-forget), QoS 1 (at-least-once), QoS 2 (exactly-once)
- ✅ **Flexible Topic Routing** - Multiple MQTT patterns → Danube topics with per-topic configuration
- ✅ **Metadata Preservation** - MQTT attributes (topic, QoS, retain, dup) as message attributes
- ✅ **Partitioned Topics** - Per-topic partition configuration for parallel processing
- ✅ **Reliable Dispatch** - Automatic QoS-based reliable delivery to Danube
- ✅ **Authentication** - Username/password and TLS support
- ✅ **High Performance** - Async I/O with TCP_NODELAY enabled

## 🔄 How It Works

```
MQTT Broker → Connector subscribes to topics → Routes to Danube topics
                ↓                                        ↓
       Topic pattern matching                   Message + Metadata
         (wildcards supported)                  (MQTT attributes preserved)
```

**Message Flow:**
1. Connector subscribes to configured MQTT topic patterns (e.g., `sensors/#`, `devices/+/telemetry`)
2. Receives messages from MQTT broker via rumqttc client
3. Matches MQTT topic against configured patterns (first match wins)
4. Routes to corresponding Danube topic with:
   - Original message payload
   - MQTT metadata as attributes (`mqtt.topic`, `mqtt.qos`, `mqtt.retain`, etc.)
   - Per-topic partition and reliability settings
5. Commits offset after successful Danube publish

**Use Cases:** Industrial IoT, smart devices, edge computing, sensor networks, fleet management

## ⚙️ Configuration

### TOML-First Approach

The connector uses **TOML configuration files** as the primary configuration source, with optional environment variable overrides.

**Configuration priority:** `connector.toml` → Environment Variables (overrides)

### Minimal Configuration Example

```toml
# connector.toml
danube_service_url = "http://danube-broker:6650"
connector_name = "mqtt-iot-source"

[mqtt]
broker_host = "mosquitto"
broker_port = 1883
client_id = "danube-connector-1"

# Route sensors/temp/zone1 → /iot/sensors_zone1
[[mqtt.topic_mappings]]
mqtt_topic = "sensors/+/zone1"  # MQTT pattern (wildcards supported)
danube_topic = "/iot/sensors_zone1"  # Format: /{namespace}/{topic}
qos = "AtLeastOnce"  # QoS 1
partitions = 4       # Danube topic partitions

# Route devices telemetry → /iot/device_telemetry
[[mqtt.topic_mappings]]
mqtt_topic = "devices/+/telemetry"
danube_topic = "/iot/device_telemetry"
qos = "AtLeastOnce"
partitions = 2
```

### Key Configuration Fields

**Core Settings:**
- `danube_service_url` - Danube broker URL (required)
- `connector_name` - Unique connector identifier (required)

**MQTT Settings:**
- `mqtt.broker_host` - MQTT broker hostname (required)
- `mqtt.broker_port` - MQTT broker port (default: 1883)
- `mqtt.client_id` - MQTT client ID (required)
- `mqtt.username` / `mqtt.password` - Authentication (optional)
- `mqtt.use_tls` - Enable TLS (default: false)

**Topic Mappings:**
- `mqtt_topic` - MQTT pattern with wildcards (`sensors/#`, `devices/+/data`)
- `danube_topic` - Target Danube topic (`/{namespace}/{topic}`)
- `qos` - MQTT QoS: `"AtMostOnce"` (0), `"AtLeastOnce"` (1), `"ExactlyOnce"` (2)
- `partitions` - Number of Danube topic partitions (0 = non-partitioned)
- `reliable_dispatch` - Override QoS-based default (optional)

### Environment Variable Overrides

Environment variables can override **only secrets and connection URLs**:

| Variable | Description | Example |
|----------|-------------|---------|
| `CONNECTOR_CONFIG_PATH` | Path to TOML config (required) | `/etc/connector.toml` |
| `DANUBE_SERVICE_URL` | Override Danube broker URL | `http://prod-broker:6650` |
| `CONNECTOR_NAME` | Override connector name | `mqtt-production` |
| `MQTT_BROKER_HOST` | Override MQTT broker host | `prod-mqtt.internal` |
| `MQTT_BROKER_PORT` | Override MQTT broker port | `1883` |
| `MQTT_CLIENT_ID` | Override MQTT client ID | `mqtt-prod-1` |
| `MQTT_USERNAME` | MQTT username (secret) | `prod_user` |
| `MQTT_PASSWORD` | MQTT password (secret) | `${VAULT_PASSWORD}` |
| `MQTT_USE_TLS` | Enable TLS | `true` |

**NOT Supported via Environment Variables:**
- Topic mappings (must be in TOML)
- QoS levels (must be in TOML)
- Partition configuration (must be in TOML)
- Retry/processing settings (must be in TOML)

**Why TOML?**
- ✅ Single file contains all routing rules and settings
- ✅ Version control friendly
- ✅ Type-safe deserialization with validation
- ✅ Self-documenting with inline comments

**When to use ENV vars:**
- 🔐 Production secrets (passwords, API keys)
- 🌍 Environment-specific URLs (dev/staging/prod)
- ☸️ Kubernetes ConfigMaps + Secrets

See the [example configuration](../../examples/source-mqtt/connector.toml) for a complete reference.

## 📋 Message Attributes

Each message published to Danube includes MQTT metadata as attributes:

```json
{
  "mqtt.topic": "sensors/temp/zone1",
  "mqtt.qos": "0",
  "mqtt.retain": "false",
  "mqtt.dup": "false",
  "source": "mqtt"
}
```

These attributes are queryable in Danube consumers and useful for filtering, routing, and debugging.

## 🧪 Testing

See the [integration testing example](../../examples/source-mqtt/) for:
- Complete Docker Compose setup
- MQTT broker + Danube + Connector
- Sample message publishers
- Step-by-step testing guide
- Consuming messages with danube-cli

## 🛠️ Development

See [DEVELOPMENT.md](./DEVELOPMENT.md) for:
- Building from source
- Running unit tests
- Code structure
- Contributing guidelines

## 📄 License

Apache-2.0

## 🤝 Contributing

Contributions welcome! See [DEVELOPMENT.md](./DEVELOPMENT.md) for development setup and guidelines.

## 🔗 Built With

- [rumqttc](https://github.com/bytebeamio/rumqtt) - MQTT 3.1.1 client library
- [Danube Messaging](https://github.com/danube-messaging/danube) - High-performance messaging platform
- [tokio](https://tokio.rs) - Async runtime for Rust
