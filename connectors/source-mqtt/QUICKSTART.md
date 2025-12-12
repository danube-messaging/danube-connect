# MQTT Source Connector - Quick Start Guide

## 🚀 Get Started in 5 Minutes

### 1. Setup Local Test Environment

```bash
# Start MQTT broker
docker run -d -p 1883:1883 --name mosquitto eclipse-mosquitto:2

# Start Danube broker (assuming you have it set up)
# See: https://github.com/danube-messaging/danube
```

### 2. Configure the Connector

```bash
cd connectors/source-mqtt

# Copy example configuration
cp config.example.env .env

# Edit .env with your settings
export DANUBE_SERVICE_URL="http://localhost:6650"
export CONNECTOR_NAME="mqtt-test"
export MQTT_BROKER_HOST="localhost"
export MQTT_CLIENT_ID="danube-mqtt-1"
export MQTT_TOPICS="sensors/#,devices/+/telemetry"
export MQTT_DANUBE_TOPIC="/iot/data"
```

### 3. Run the Connector

```bash
# Build and run
cargo run --release

# Or use the binary directly
../../target/release/danube-source-mqtt
```

### 4. Test It!

Publish test messages to MQTT:

```bash
# Install mosquitto clients if needed
# Ubuntu/Debian: sudo apt-get install mosquitto-clients
# macOS: brew install mosquitto

# Publish a JSON message
mosquitto_pub -h localhost -t sensors/temp/zone1 \
  -m '{"temperature": 23.5, "unit": "celsius", "timestamp": 1234567890}'

# Publish to a device telemetry topic
mosquitto_pub -h localhost -t devices/device001/telemetry \
  -m '{"battery": 85, "signal_strength": 95}'

# Publish multiple messages
for i in {1..10}; do
  mosquitto_pub -h localhost -t sensors/temp/zone$i \
    -m "{\"temperature\": $((RANDOM % 30 + 10)), \"zone\": $i}"
  sleep 1
done
```

### 5. Verify Messages in Danube

```bash
# Using danube-client CLI (if available)
danube-cli topic stats /iot/data

# Check connector logs
tail -f logs/mqtt-connector.log
```

## 🐳 Docker Quick Start

### Using Docker Compose

```bash
# Start everything (Mosquitto + Danube + Connector)
docker-compose -f docker-compose.example.yml up

# Publish test messages
docker exec -it mqtt-publisher sh -c "
  mosquitto_pub -h mosquitto -t sensors/test \
    -m '{\"value\": 42, \"timestamp\": $(date +%s)}'
"

# View connector logs
docker logs -f mqtt-source-connector
```

### Build Docker Image

```bash
# From project root
docker build -t danube-connect/source-mqtt:latest \
  -f connectors/source-mqtt/Dockerfile .

# Run the image
docker run --rm \
  -e DANUBE_SERVICE_URL=http://host.docker.internal:6650 \
  -e CONNECTOR_NAME=mqtt-docker \
  -e MQTT_BROKER_HOST=host.docker.internal \
  -e MQTT_CLIENT_ID=danube-docker-1 \
  -e MQTT_TOPICS="test/#" \
  danube-connect/source-mqtt:latest
```

## 📊 Common Use Cases

### IoT Sensor Network

```bash
export MQTT_TOPICS="factory/+/sensors/+,warehouse/+/sensors/+"
export MQTT_DANUBE_TOPIC="/industrial/sensors"
export MQTT_QOS=1
```

Subscribe to all sensors across factory and warehouse with QoS 1 for reliable delivery.

### Smart Home Integration

```bash
export MQTT_TOPICS="home/+/+/state,home/+/+/command"
export MQTT_DANUBE_TOPIC="/smarthome/events"
export MQTT_CLEAN_SESSION=true
```

Bridge smart home device states and commands to Danube for processing.

### Vehicle Telemetry

```bash
export MQTT_TOPICS="vehicles/+/location,vehicles/+/diagnostics"
export MQTT_DANUBE_TOPIC="/fleet/telemetry"
export MQTT_INCLUDE_METADATA=true
```

Capture vehicle location and diagnostic data with full metadata preservation.

## 🔍 Troubleshooting

### Connector won't start

```bash
# Check environment variables
env | grep -E '(MQTT|DANUBE|CONNECTOR)'

# Verify MQTT broker is accessible
telnet localhost 1883

# Check Danube broker
curl http://localhost:6650/health
```

### No messages received

```bash
# Enable debug logging
export RUST_LOG=debug,danube_source_mqtt=trace
cargo run

# Test MQTT subscription directly
mosquitto_sub -h localhost -t '#' -v

# Verify topic patterns match
mosquitto_pub -h localhost -t sensors/test -m "test"
```

### Connection issues

```bash
# For TLS connections
export MQTT_USE_TLS=true
export MQTT_BROKER_PORT=8883

# For authenticated connections
export MQTT_USERNAME=your-username
export MQTT_PASSWORD=your-password
```

## 📚 Next Steps

- Read the full [README.md](./README.md) for detailed configuration
- Check out [example configurations](./config.example.env)
- Explore [message patterns](../../info/connector-message-patterns.md)
- Deploy to production with [Kubernetes](./README.md#kubernetes-deployment)

## 💡 Tips

1. **Start with QoS 1** for a good balance of reliability and performance
2. **Use wildcard topics** (`#`, `+`) to reduce configuration complexity
3. **Enable metadata** to preserve MQTT context in Danube messages
4. **Monitor metrics** at `http://localhost:9090/metrics`
5. **Test with mosquitto_sub** before deploying to production

## 🆘 Need Help?

- Check logs: `RUST_LOG=debug cargo run`
- View metrics: `curl http://localhost:9090/metrics`
- GitHub Issues: https://github.com/danube-messaging/danube-connect/issues
- Documentation: https://danube-docs.dev-state.com

Happy streaming! 🚀
