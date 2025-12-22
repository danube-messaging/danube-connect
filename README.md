# Danube Connect

<div align="center">

![Danube Logo](https://raw.githubusercontent.com/danube-messaging/danube/main/Danube_logo_2.png)

**Connector ecosystem for Danube Messaging**

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

[Documentation](./info/CONNECTORS_README.md) | [Development Guide](./info/connector-development-guide.md) | [Message Patterns](./info/connector-message-patterns.md)

</div>

## Overview

Danube Connect provides a batteries-included connector ecosystem for [Danube Messaging](https://github.com/danube-messaging/danube), enabling seamless integration with external systems without compromising the safety, stability, or binary size of the core broker.

## Features

- 🔌 **Plug-and-Play Connectors** - Ready-to-use integrations for popular systems
- 🦀 **Pure Rust** - Memory-safe, high-performance connector framework
- 🔄 **Bidirectional** - Support for both source and sink connectors
- 📦 **Modular** - Clean separation between framework and connector implementations
- 🚀 **Cloud Native** - Docker-first with Kubernetes support
- 📊 **Observable** - Built-in metrics, tracing, and health checks
- ⚡ **High Performance** - Batching, connection pooling, and parallel processing

## Architecture

```text
External Systems ↔ Connectors ↔ danube-connect-core ↔ danube-client ↔ Danube Broker
```

Connectors run as standalone processes, communicating with Danube brokers via gRPC. This ensures:
- **Isolation**: Connector failures don't impact the broker
- **Scalability**: Horizontal scaling of connectors
- **Flexibility**: Mix and match connectors as needed

## Quick Start

### For Users

Run a connector using Docker:

```bash
docker run -e DANUBE_SERVICE_URL=http://localhost:6650 \
           -e DANUBE_TOPIC=/default/events \
           -e SUBSCRIPTION_NAME=my-sink \
           danube-connect/sink-http:latest
```

### For Developers

Create a new connector:

```bash
# Clone the repository
git clone https://github.com/danube-messaging/danube-connect
cd danube-connect

# Create a new connector
cd connectors
cargo new --bin sink-mydb

# Implement the SinkConnector trait
# See info/connector-development-guide.md for details
```

## Available Connectors

### Sink Connectors (Danube → External)

| Connector | Status | Description |
|-----------|--------|-------------|
| HTTP/Webhook | 🚧 Planned | Universal REST API integration |
| ClickHouse | 🚧 Planned | Real-time analytics ingestion |
| PostgreSQL | 🚧 Planned | Relational database sink |
| Elasticsearch | 🚧 Planned | Search and analytics |

### Source Connectors (External → Danube)

| Connector | Status | Description |
|-----------|--------|-------------|
| MQTT | ✅ Example | IoT device integration ([see example](./examples/source-mqtt/)) |
| PostgreSQL CDC | 🚧 Planned | Change Data Capture |
| File/Directory | 🚧 Planned | File system monitoring |
| Kafka | 🚧 Planned | Kafka topic mirroring |


## Documentation

Complete documentation is available in the `info/` directory:

- **[Start Here: Documentation Index](./info/README.md)** - Overview and navigation
- **[Architecture Document](./info/connectors.md)** - Design philosophy and specifications
- **[Development Guide](./info/connector-development-guide.md)** - Build your first connector
- **[Configuration Guide](./info/unified-configuration-guide.md)** - Configuration patterns and best practices
- **[Message Patterns](./info/connector-message-patterns.md)** - Message handling strategies


## Building from Source

```bash
# Build all crates
cargo build --release

# Run tests
cargo test

# Build a specific connector
cargo build --release -p danube-sink-http
```

## Contributing

We welcome contributions! Here's how you can help:

- **New Connectors**: Implement connectors for popular systems
- **Documentation**: Improve guides and examples
- **Testing**: Add test coverage
- **Bug Reports**: Open issues with detailed information

Please read our [Development Guide](./info/connector-development-guide.md) before contributing.

## License

Apache License 2.0 - See [LICENSE](LICENSE) for details.

## Community

- **GitHub Issues**: [Report bugs or request features](https://github.com/danube-messaging/danube-connect/issues)
- **Danube Docs**: [Official Documentation](https://danube-docs.dev-state.com)
- **Main Project**: [Danube Messaging](https://github.com/danube-messaging/danube)

