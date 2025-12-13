# MQTT Source Connector - Development Guide

This guide is for developers who want to modify or extend the MQTT source connector.

## 🛠️ Development Setup

### Prerequisites

- Rust 1.75+ with cargo
- Docker (for running MQTT broker during development)
- MQTT client tools (optional, for testing)

### Clone and Build

```bash
cd connectors/source-mqtt

# Check compilation
cargo check

# Run tests
cargo test

# Build release binary
cargo build --release
```

## 📁 Project Structure

```
connectors/source-mqtt/
├── src/
│   ├── main.rs          # Entry point, runtime initialization
│   ├── connector.rs     # Core connector implementation
│   └── config.rs        # Configuration management
├── Cargo.toml           # Dependencies
├── Dockerfile           # Container image
├── README.md            # User documentation
└── DEVELOPMENT.md       # This file
```

## 🏗️ Architecture

### Components

1. **Main Runtime** (`main.rs`)
   - Initializes tracing/logging
   - Loads ConnectorConfig
   - Creates SourceRuntime
   - Runs until shutdown signal

2. **Connector Implementation** (`connector.rs`)
   - Implements `SourceConnector` trait
   - Spawns MQTT event loop in background task
   - Uses channel for thread-safe message passing
   - Converts MQTT messages to SourceRecords

3. **Configuration** (`config.rs`)
   - Loads from environment variables
   - Validates settings
   - Provides MQTT client options

### Thread Model

```
Main Thread                      Background Task
    |                                  |
    |-- initialize()                   |
    |   |-- spawn_event_loop() ------> |
    |                                  |-- rumqttc::EventLoop::poll()
    |                                  |-- Convert Publish → SourceRecord
    |                                  |-- Send via channel
    |                                  |
    |<- Receive from channel           |
    |-- poll() returns Vec<SourceRecord>
    |
```

### Key Design Decisions

**Why background task for event loop?**
- `rumqttc::EventLoop` is `!Sync` (not thread-safe)
- Can't be used directly in async trait methods
- Solution: Spawn task + use mpsc channel

**Why static helper methods?**
- Background task needs to own data
- Static methods avoid self reference issues
- Topic map and config cloned to task

**Why batch receiving in poll()?**
- Reduces syscall overhead
- Better throughput for high-volume scenarios
- Limits batch to 100 to avoid blocking

## 🧪 Testing

### Unit Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocov --test-threads=1

# Run specific test
cargo test test_topic_matching
```

### Integration Testing

See the [example setup](../../examples/source-mqtt/README.md) for end-to-end testing.

Quick local test:

```bash
# Terminal 1: Start MQTT broker
docker run -p 1883:1883 eclipse-mosquitto:2

# Terminal 2: Run connector (requires Danube running)
export DANUBE_SERVICE_URL=http://localhost:6650
export CONNECTOR_NAME=mqtt-dev
export MQTT_BROKER_HOST=localhost
export MQTT_CLIENT_ID=dev-connector
export MQTT_TOPICS=test/#
export RUST_LOG=debug

cargo run

# Terminal 3: Publish test message
mosquitto_pub -h localhost -t test/hello -m "world"
```

### Debugging

Enable verbose logging:

```bash
# All debug logs
export RUST_LOG=debug

# Connector-specific trace logs
export RUST_LOG=danube_source_mqtt=trace

# With timestamps
export RUST_LOG=danube_source_mqtt=trace
cargo run 2>&1 | ts '[%Y-%m-%d %H:%M:%S]'
```

## 🐛 Common Issues

### "Channel closed" Error

**Symptom**: `MQTT event loop channel closed` error

**Cause**: Background task crashed

**Fix**: Check event loop logs for panic:
```bash
export RUST_BACKTRACE=1
cargo run
```

### Messages Not Received

**Check list**:
1. MQTT broker running? `nc -zv localhost 1883`
2. Topic subscription successful? Check logs for "Subscribing to MQTT topic"
3. Wildcard patterns correct? Test with `mosquitto_sub -t '#' -v`
4. QoS level supported? Broker may downgrade QoS

### High Memory Usage

**Potential causes**:
1. Channel buffer full - Increase buffer size or speed up poll()
2. Message backlog - Check Danube publish latency
3. Large messages - Reduce `max_packet_size`

## 📊 Performance Tuning

### Throughput Optimization

```rust
// Increase channel buffer
let (message_tx, message_rx) = mpsc::channel(10000); // Default: 1000

// Increase batch size in poll()
if records.len() >= 1000 { break; } // Default: 100

// Reduce poll timeout
tokio::time::timeout(Duration::from_millis(10), ...) // Default: 100ms
```

### Latency Optimization

- Use QoS 0 (if acceptable)
- Reduce keep-alive interval
- Minimize message transformation
- Optimize Danube publish settings

### Memory Optimization

- Reduce channel buffer size
- Limit max_packet_size
- Use clean_session=true
- Process messages immediately

## 🚀 Building for Production

### Optimized Build

```bash
# Release build with all optimizations
cargo build --release

# Check binary size
ls -lh target/release/danube-source-mqtt

# Strip debug symbols (done automatically via Cargo.toml)
```

### Docker Build

```bash
# From repository root
docker build -t danube-source-mqtt:latest \
  -f connectors/source-mqtt/Dockerfile .

# Multi-arch build
docker buildx build --platform linux/amd64,linux/arm64 \
  -t danube-source-mqtt:latest \
  -f connectors/source-mqtt/Dockerfile .
```

## 📝 Code Style

This project follows standard Rust conventions:

```bash
# Format code
cargo fmt

# Lint
cargo clippy -- -D warnings

# Check for common mistakes
cargo clippy --all-targets --all-features
```

## 🤝 Contributing

1. Create a feature branch
2. Make your changes
3. Add tests for new functionality
4. Run `cargo fmt` and `cargo clippy`
5. Update documentation
6. Submit a pull request

## 📚 References

- [rumqttc Documentation](https://docs.rs/rumqttc/)
- [MQTT 3.1.1 Specification](https://docs.oasis-open.org/mqtt/mqtt/v3.1.1/mqtt-v3.1.1.html)
- [Danube Connect Core](../../danube-connect-core/)
- [SourceConnector Trait](../../danube-connect-core/src/traits.rs)

## 🆘 Getting Help

- Check existing issues: https://github.com/danube-messaging/danube-connect/issues
- Read the connector development guide: [../../info/connector-development-guide.md](../../info/connector-development-guide.md)
- Review other connectors for patterns
