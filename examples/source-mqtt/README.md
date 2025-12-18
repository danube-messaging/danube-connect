# MQTT Source Connector - Integration Testing

This example demonstrates end-to-end integration testing of the MQTT Source Connector, showing how MQTT messages flow into Danube.

## 🎯 What This Tests

- MQTT broker → Connector → Danube broker pipeline
- Topic mapping and wildcards
- Message transformation and metadata
- QoS handling

### Data Flow

```
MQTT Publisher (test messages)
    ↓
Mosquitto MQTT Broker
    ↓ Subscribe
MQTT Source Connector
    ↓ Publish  
Danube Broker (/iot/sensors topic)
    ↓ danube-cli consumer
Your terminal
```

## 📁 Files in This Example

- **`docker-compose.yml`** - Orchestrates the complete test stack (etcd, Danube, Mosquitto, connector)
- **`connector.toml`** - MQTT connector configuration with topic mappings and settings
- **`danube_broker.yml`** - Danube broker configuration
- **`mosquitto.conf`** - MQTT broker configuration (listeners, logging)
- **`.env.example`** - Template for environment variable overrides
- **`test-publisher.sh`** - Automated test script to publish sample MQTT messages
- **`README.md`** - This file (integration testing guide)

## 🚀 Quick Start (5 Minutes)

### Prerequisites

- Docker & Docker Compose
- 8GB RAM recommended
- Ports available: 1883, 2379, 6650, 9001

### 1. Start Everything

```bash
cd examples/source-mqtt
docker-compose up
```

This starts:
- etcd (Danube's metadata store)
- Danube broker
- Mosquitto MQTT broker
- MQTT source connector
- Test message publisher

### 2. Watch the Logs

```bash
# Watch connector logs
docker logs -f mqtt-example-connector

# Watch test publisher
docker logs -f mqtt-example-publisher

# Watch MQTT broker
docker logs -f mqtt-example-broker
```

You should see messages flowing:
```
[INFO] Received MQTT message: topic=sensors/temp/zone1, qos=1, size=67
[DEBUG] Publishing to Danube topic: /iot/sensors
[INFO] Message successfully published
```

### 3. Publish Your Own Messages

```bash
# Temperature reading
docker exec mqtt-example-broker mosquitto_pub \
  -t sensors/temp/zone2 \
  -m '{"temperature": 25.5, "unit": "celsius"}'

# Device telemetry
docker exec mqtt-example-broker mosquitto_pub \
  -t devices/mydevice/telemetry \
  -m '{"battery": 87, "signal": 95}'

# Pressure sensor
docker exec mqtt-example-broker mosquitto_pub \
  -t sensors/pressure/factory1 \
  -m '{"pressure": 101.3, "unit": "kPa"}'
```

### 4. Subscribe to MQTT Topics

```bash
# See all sensor messages
docker exec mqtt-example-broker mosquitto_sub -t 'sensors/#' -v

# See specific device
docker exec mqtt-example-broker mosquitto_sub -t 'devices/+/telemetry' -v
```

### 5. Consume from Danube

To verify messages are reaching Danube, consume them using **danube-cli**.

**Download danube-cli:**
- GitHub Releases: https://github.com/danube-messaging/danube/releases
- Documentation: https://danube-docs.dev-state.com/danube_clis/danube_cli/consumer/

**Consume messages:**
```bash
# In a new terminal, consume from the Danube topic
danube-cli consumer \
  --server-addr http://localhost:6650 \
  --topic /iot/sensors \
  --subscription test-sub \
  --subscription-type exclusive

# You should see MQTT messages appearing in real-time:
# Topic: /iot/sensors
# Payload: {"temperature": 23.5, "sensor_id": "zone1"}
# Attributes: mqtt.topic=sensors/temp/zone1, mqtt.qos=1
```

**Note:** Danube topics use the format `/namespace/topic` (exactly 2 segments).

### 6. Stop Everything

```bash
docker-compose down
```

## 📋 Configuration

The example uses `connector.toml` with topic mappings:

```toml
[[mqtt.topic_mappings]]
mqtt_topic = "sensors/#"           # MQTT wildcard pattern
danube_topic = "/iot/sensors"      # Danube topic (format: /namespace/topic)
qos = 1
partitions = 0
```

## 🧪 Testing

### Test 1: Basic Message Flow

```bash
# Publish to MQTT
docker exec mqtt-example-broker mosquitto_pub -t sensors/temp -m '{"value": 23.5}'

# Verify in connector logs
docker logs mqtt-example-connector | grep "Received MQTT message"

# Consume from Danube
danube-cli consumer --server-addr http://localhost:6650 --topic /iot/sensors --subscription test
```

### Test 2: Wildcard Subscriptions

The connector subscribes to:
- `sensors/#` - All sensor topics (multi-level wildcard)
- `devices/+/telemetry` - All device telemetry (single-level wildcard)

```bash
# Publish to different MQTT topics
docker exec mqtt-example-broker mosquitto_pub -t sensors/temp -m "test1"
docker exec mqtt-example-broker mosquitto_pub -t sensors/pressure -m "test2" 
docker exec mqtt-example-broker mosquitto_pub -t devices/telemetry -m "test3"  # Won't match

# All matching messages go to /iot/sensors in Danube
danube-cli consumer --server-addr http://localhost:6650 --topic /iot/sensors --subscription test
```

### Test 3: Verify Metadata

```bash
# Publish with QoS
docker exec mqtt-example-broker mosquitto_pub -t sensors/test -q 1 -m "data"

# Consume and check attributes (mqtt.topic, mqtt.qos, source=mqtt)
danube-cli consumer --server-addr http://localhost:6650 --topic /iot/sensors --subscription test --show-attributes
```

### Test 4: Automated Load Testing

Use `test-publisher.sh` to continuously publish sample messages:

```bash
# Make script executable
chmod +x test-publisher.sh

# Start automated publishing (sends temp, humidity, pressure, telemetry every 5s)
./test-publisher.sh

# Output:
# [10:30:15] Published batch #1: temp=22°C, humidity=65%, pressure=1013hPa, battery=78%
# [10:30:20] Published batch #2: temp=25°C, humidity=58%, pressure=1009hPa, battery=82%
# ...

# In another terminal, consume from Danube
danube-cli consumer --server-addr http://localhost:6650 --topic /iot/sensors --subscription load-test

# Press Ctrl+C to stop the publisher
```

This script simulates:
- Temperature sensors (`sensors/temp/zone1`)
- Humidity sensors (`sensors/humidity/zone1`)
- Pressure sensors (`sensors/pressure/factory1`)
- Device telemetry (`devices/device001/telemetry`)

## 🔍 Troubleshooting

```bash
# Verify MQTT broker is running
docker exec mqtt-example-broker mosquitto_sub -t '#' -v

# Check connector logs
docker logs mqtt-example-connector

# Check Danube broker
curl http://localhost:6650/health

# Restart if needed
docker-compose restart mqtt-example-connector
```

## 📚 Related Documentation

- [MQTT Connector Development Guide](../../connectors/source-mqtt/DEVELOPMENT.md)
- [MQTT Connector README](../../connectors/source-mqtt/README.md)
- [danube-cli Documentation](https://danube-docs.dev-state.com/danube_clis/danube_cli/consumer/)
- [danube-cli Releases](https://github.com/danube-messaging/danube/releases)
