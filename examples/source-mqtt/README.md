# MQTT Source Connector - Example Setup

This example demonstrates the MQTT Source Connector ingesting IoT sensor data from an MQTT broker into Danube.

## 🎯 What This Example Does

- **MQTT Broker** (Mosquitto) - Receives messages from IoT devices
- **Danube Broker** - Messaging platform for data distribution
- **MQTT Connector** - Bridges MQTT topics to Danube topics
- **Test Publisher** - Simulates IoT devices sending telemetry

### Data Flow

```
IoT Sensors (simulated) 
    ↓ MQTT Publish
MQTT Broker (Mosquitto)
    ↓ Subscribe
MQTT Source Connector
    ↓ Publish
Danube Broker (/iot/data topic)
    ↓ Subscribe
Your Applications
```

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
[DEBUG] Publishing to Danube topic: /iot/data
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

### 5. Verify in Danube

```bash
# Check connector health
curl http://localhost:6650/health

# View metrics (if metrics endpoint is available)
curl http://localhost:9090/metrics | grep danube_connector
```

### 6. Stop Everything

```bash
docker-compose down
```

## 📋 Configuration

### Single Configuration File

The connector uses **one TOML file** containing all settings (both core and MQTT-specific).

**Priority:** TOML file → Environment variable overrides

#### Quick Start with TOML

```bash
# Review the config file (everything in one place!)
cat connector.toml

# That's it! One file with all settings
# The docker-compose.yml automatically mounts it

# Start the connector
docker-compose up
```

#### Configuration Structure

**One file, two sections:**

```toml
# Part 1: Core Danube settings (at root level)
danube_service_url = "http://danube-broker:6650"
connector_name = "mqtt-iot-source"
destination_topic = "/iot/data"
max_retries = 3
# ... runtime settings

# Part 2: MQTT-specific settings (under [mqtt])
[mqtt]
broker_host = "mosquitto"
broker_port = 1883
client_id = "danube-connector-1"

# Topic mappings with wildcards
[[mqtt.topic_mappings]]
mqtt_topic = "sensors/+/zone1"
danube_topic = "/iot/sensors/zone1"
qos = 1
```

📖 **See `connector.toml` for the complete, documented configuration**

#### Environment Variable Overrides

Override any TOML setting with environment variables:

```bash
# Override broker host
export MQTT_BROKER_HOST=different-broker
docker-compose up

# Or in docker-compose.yml
environment:
  MQTT_BROKER_HOST: "production-mqtt"
  MQTT_CLIENT_ID: "prod-connector"
```

**Common overrides:**

| Variable | Purpose |
|----------|---------|
| `CONFIG_FILE` | Path to TOML config file |
| `DANUBE_SERVICE_URL` | Override Danube broker URL |
| `MQTT_BROKER_HOST` | Override MQTT broker host |
| `MQTT_USERNAME` / `MQTT_PASSWORD` | Inject secrets |
| `LOG_LEVEL` | Change logging verbosity |

#### ENV-Only Configuration (Alternative)

If you prefer environment variables only (no TOML file):

```bash
# Don't set CONFIG_FILE, and set all required ENV vars
export DANUBE_SERVICE_URL=http://localhost:6650
export CONNECTOR_NAME=mqtt-source
export MQTT_BROKER_HOST=mosquitto
export MQTT_BROKER_PORT=1883
export MQTT_CLIENT_ID=connector-1
export MQTT_TOPICS="sensors/#,devices/+/telemetry"
export MQTT_DANUBE_TOPIC="/iot/data"
export MQTT_QOS=1
```

**Why TOML is Better:**
- ✅ Structured, type-safe configuration
- ✅ Support for complex structures (arrays, nested objects)
- ✅ Self-documenting with comments
- ✅ Version control friendly
- ✅ Validation at startup

### Customizing MQTT Broker

Edit `mosquitto.conf` to change MQTT broker settings:

```conf
# Enable authentication
allow_anonymous false
password_file /mosquitto/config/passwd

# Enable TLS
listener 8883
certfile /mosquitto/config/cert.pem
keyfile /mosquitto/config/key.pem
```

## 🧪 Testing Scenarios

### Scenario 1: High-Volume Sensor Network

Simulate 100 sensors publishing every second:

```bash
# Create test script
cat > test-volume.sh << 'EOF'
#!/bin/bash
for i in {1..100}; do
  docker exec mqtt-example-broker mosquitto_pub \
    -t "sensors/temp/zone$i" \
    -m "{\"temperature\": $((RANDOM % 30 + 10)), \"sensor_id\": $i}" &
done
wait
EOF

chmod +x test-volume.sh
watch -n 1 ./test-volume.sh
```

### Scenario 2: Different QoS Levels

```bash
# QoS 0 - At most once
docker exec mqtt-example-broker mosquitto_pub -t test/qos0 -q 0 -m "qos0 message"

# QoS 1 - At least once
docker exec mqtt-example-broker mosquitto_pub -t test/qos1 -q 1 -m "qos1 message"

# QoS 2 - Exactly once
docker exec mqtt-example-broker mosquitto_pub -t test/qos2 -q 2 -m "qos2 message"
```

### Scenario 3: Retained Messages

```bash
# Publish retained message (new subscribers get last value)
docker exec mqtt-example-broker mosquitto_pub \
  -t sensors/config -r \
  -m '{"interval": 60, "enabled": true}'

# Subscribe and immediately receive the retained message
docker exec mqtt-example-broker mosquitto_sub -t sensors/config -C 1
```

### Scenario 4: Wildcard Subscriptions

The connector subscribes to:
- `sensors/#` - All sensor topics (multi-level wildcard)
- `devices/+/telemetry` - All device telemetry (single-level wildcard)

Test matching:
```bash
# These MATCH sensors/#
mosquitto_pub -t sensors/temp -m "test"           # ✓
mosquitto_pub -t sensors/temp/zone1 -m "test"     # ✓
mosquitto_pub -t sensors/a/b/c/d -m "test"        # ✓

# These DON'T match sensors/#
mosquitto_pub -t devices/temp -m "test"           # ✗

# These MATCH devices/+/telemetry
mosquitto_pub -t devices/device1/telemetry -m "test"   # ✓
mosquitto_pub -t devices/device2/telemetry -m "test"   # ✓

# These DON'T match devices/+/telemetry
mosquitto_pub -t devices/telemetry -m "test"           # ✗ (missing middle level)
mosquitto_pub -t devices/d1/t/telemetry -m "test"      # ✗ (too many levels)
```

## 📊 Monitoring

### View Connector Metrics

```bash
# Connector logs
docker logs mqtt-example-connector 2>&1 | grep -E "(messages|processed|error)"

# MQTT broker stats
docker exec mqtt-example-broker mosquitto_sub -t '$SYS/#' -v
```

### Performance Metrics

Expected performance:
- **Throughput**: 50K+ messages/sec (1KB payloads)
- **Latency**: <5ms (MQTT → Danube)
- **Memory**: ~15-20MB

Check resource usage:
```bash
docker stats mqtt-example-connector
```

## 🔍 Troubleshooting

### Connector Won't Start

```bash
# Check logs
docker logs mqtt-example-connector

# Common issues:
# 1. Danube broker not ready - wait for healthcheck
docker-compose ps

# 2. MQTT connection failed - check mosquitto logs
docker logs mqtt-example-broker

# 3. Network issues - restart stack
docker-compose down && docker-compose up
```

### No Messages Received

```bash
# 1. Verify MQTT subscription
docker exec mqtt-example-broker mosquitto_sub -t '#' -v

# 2. Check connector is subscribing
docker logs mqtt-example-connector | grep "Subscribing"

# 3. Publish test message directly
docker exec mqtt-example-broker mosquitto_pub -t test -m "hello"
docker logs mqtt-example-connector | grep "Received MQTT message"
```

### Connector Disconnects

```bash
# Check MQTT keep-alive settings
# Increase keep-alive in .env:
MQTT_KEEP_ALIVE_SECS=120

# Check network stability
docker network inspect mqtt-example_danube-mqtt-network
```

## 🎓 Learning Resources

### Understanding MQTT Topics

```
sensors/temp/zone1
└─┬──┘ └─┬┘ └─┬─┘
  │      │    └── Level 3: Specific zone
  │      └────── Level 2: Sensor type
  └───────────── Level 1: Category
```

Wildcards:
- `+` matches ONE level: `sensors/+/zone1` → `sensors/temp/zone1` ✓
- `#` matches ALL remaining: `sensors/#` → `sensors/temp/zone1/data` ✓

### MQTT QoS Levels

- **QoS 0**: Fire and forget (fastest, least reliable)
- **QoS 1**: At least once (good balance) ← **Recommended**
- **QoS 2**: Exactly once (slowest, most reliable)

## 📁 Files in This Example

```
examples/source-mqtt/
├── docker-compose.yml    # Complete stack definition
├── .env.example          # Configuration template
├── mosquitto.conf        # MQTT broker configuration
└── README.md            # This file
```

## 🔄 Next Steps

1. **Customize Topics** - Edit `MQTT_TOPICS` in docker-compose.yml
2. **Add Authentication** - Configure username/password in mosquitto.conf
3. **Enable TLS** - Add certificates to mosquitto configuration
4. **Scale Up** - Run multiple connector instances for different topics
5. **Integrate** - Connect your real IoT devices to the MQTT broker

## 📚 Related Documentation

- [MQTT Connector Documentation](../../connectors/source-mqtt/README.md)
- [MQTT Protocol Specification](https://mqtt.org/)
- [Eclipse Mosquitto Docs](https://mosquitto.org/documentation/)
- [Danube Documentation](https://github.com/danube-messaging/danube)

## 💡 Production Considerations

Before going to production:
- [ ] Enable MQTT authentication
- [ ] Configure TLS/SSL
- [ ] Set up monitoring and alerting
- [ ] Configure connector retry policies
- [ ] Plan topic naming convention
- [ ] Set up log aggregation
- [ ] Implement health checks in your infrastructure

## 🆘 Getting Help

- Check connector logs: `docker logs mqtt-example-connector`
- Test MQTT broker: `mosquitto_sub -h localhost -t '#' -v`
- Verify Danube: `curl http://localhost:6650/health`
- GitHub Issues: https://github.com/danube-messaging/danube-connect/issues

Happy streaming! 🚀
