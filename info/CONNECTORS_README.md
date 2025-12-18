# Danube Connect - Comprehensive Documentation

This directory contains the complete architectural design and documentation for the **Danube Connect** ecosystem - a connector framework for integrating Danube Messaging with external systems.

## 📚 Documentation Index

### 1. [connectors.md](./connectors.md) - Main Architecture Document
**Start here for an overview of the entire Danube Connect ecosystem**

- Executive summary and design philosophy
- Repository structure and workspace layout
- Priority connectors roadmap (HTTP, PostgreSQL CDC, ClickHouse, MQTT, Kafka)
- Deployment strategies with Docker examples
- Technical implementation specifications

**Key Topics:**
- Connector worker pattern and process isolation
- Shared core library responsibilities
- RPC communication with Danube brokers
- Message format and field ownership
- Trait-based connector design

### 2. [UNIFIED_CONFIGURATION_GUIDE.md](./UNIFIED_CONFIGURATION_GUIDE.md) - Configuration Patterns
**For understanding the configuration architecture**

- Single-file configuration pattern
- TOML-first approach with ENV overrides
- Core vs connector-specific settings
- Multiple connector deployment patterns
- Docker and Kubernetes examples

**Key Topics:**
- Unified config struct design
- Environment variable overrides
- Multi-connector deployments
- Configuration best practices

### 3. [connector-development-guide.md](./connector-development-guide.md) - Developer Quickstart
**Step-by-step guide for building your first connector**

- Project setup and dependencies
- Configuration management examples
- Complete MongoDB sink connector implementation
- Testing strategies (unit and integration)
- Docker containerization
- Source connector examples

**Key Topics:**
- Implementing `SinkConnector` trait
- Implementing `SourceConnector` trait
- Batching and buffering patterns
- Error handling best practices
- Building and running connectors

### 4. [connector-message-patterns.md](./connector-message-patterns.md) - Message Handling Guide
**Deep dive into message transformation and data flow**

- Danube message structure and semantics
- Data flow patterns (sink, source, bridge)
- Message transformation strategies (pass-through, JSON, schema-based, batched)
- Using message attributes for routing and metadata
- Error handling patterns (retry, DLQ, partial success)
- Performance optimization techniques

**Key Topics:**
- Understanding `StreamMessage` and `MsgID`
- Field ownership (client vs broker)
- Transformation patterns
- Attribute-based routing
- Batch processing optimization

### 5. [connectors.md](./connectors.md) - Architecture & Design Document
**High-level architecture and design philosophy**

- Executive summary and design philosophy
- Repository structure and workspace layout
- Priority connectors roadmap
- Technical implementation specifications
- Deployment strategies with Docker examples

**Key Topics:**
- Connector worker pattern and process isolation
- Shared core library responsibilities
- Message format and field ownership
- Trait-based connector design
- Docker deployment patterns

## 🏗️ Architecture Overview

```text
┌─────────────────────────────────────────────────────────────────┐
│                    External Systems                             │
│  (PostgreSQL, HTTP APIs, ClickHouse, MQTT, Kafka, etc.)        │
└──────────────────┬──────────────────────────────────────────────┘
                   │ External Protocols
┌──────────────────▼──────────────────────────────────────────────┐
│               Connector Implementations                         │
│  (sink-http, source-postgres, sink-clickhouse, etc.)           │
│  - Implements SinkConnector or SourceConnector traits          │
│  - Business logic for external system integration              │
└──────────────────┬──────────────────────────────────────────────┘
                   │ Uses Connector SDK
┌──────────────────▼──────────────────────────────────────────────┐
│                danube-connect-core                              │
│  - Runtime & lifecycle management                              │
│  - Message transformation utilities                            │
│  - Retry & error handling                                      │
│  - Configuration & observability                               │
└──────────────────┬──────────────────────────────────────────────┘
                   │ Wraps & Manages
┌──────────────────▼──────────────────────────────────────────────┐
│                   danube-client                                 │
│  - Producer/Consumer high-level API                            │
│  - gRPC client implementation                                  │
│  - Connection management & health checks                       │
└──────────────────┬──────────────────────────────────────────────┘
                   │ gRPC/Protobuf (DanubeApi.proto)
┌──────────────────▼──────────────────────────────────────────────┐
│                  Danube Broker Cluster                          │
│  - Topic management & message routing                          │
│  - WAL + Cloud persistence                                     │
│  - Subscription management                                     │
└─────────────────────────────────────────────────────────────────┘
```

## 🚀 Quick Start

### For Connector Users

1. **Choose a connector** from the available implementations
2. **Configure via environment variables**:
   ```bash
   export DANUBE_SERVICE_URL="http://localhost:6650"
   export DANUBE_TOPIC="/default/events"
   export SUBSCRIPTION_NAME="my-sink"
   ```
3. **Run with Docker**:
   ```bash
   docker run -e DANUBE_SERVICE_URL=... danube-connect/sink-http:latest
   ```

### For Connector Developers

1. **Read** [connector-development-guide.md](./connector-development-guide.md)
2. **Create** a new connector project using the template
3. **Implement** the `SinkConnector` or `SourceConnector` trait
4. **Test** with your local Danube cluster
5. **Contribute** back to the community!

## 🎯 Key Design Principles

### 1. **Isolation & Safety**
- Connectors run as separate processes
- No FFI or unsafe code in core broker
- Crash in connector doesn't affect broker

### 2. **Minimal Interface**
- Developers implement 2-3 trait methods
- SDK handles all infrastructure concerns
- Focus on business logic, not plumbing

### 3. **Performance First**
- Built-in batching support
- Connection pooling
- Parallel processing capabilities
- Zero-copy where possible

### 4. **Observability Built-in**
- Prometheus metrics out of the box
- Structured logging with tracing
- Health check endpoints
- Automatic error tracking

### 5. **Cloud Native**
- 12-factor app configuration
- Docker-first deployment
- Horizontal scalability
- Kubernetes-ready

## 📊 Connector Types

### Sink Connectors (Danube → External)

| Connector | Status | Description |
|-----------|--------|-------------|
| HTTP/Webhook | Planned | Universal REST API integration |
| ClickHouse | Planned | Real-time analytics ingestion |
| PostgreSQL | Planned | Transactional database sink |
| Elasticsearch | Planned | Search and analytics |
| S3/Object Storage | Planned | Data lake integration |
| Redis | Planned | Cache population |

### Source Connectors (External → Danube)

| Connector | Status | Description |
|-----------|--------|-------------|
| MQTT | ✅ Example Available | IoT device integration (see examples/source-mqtt) |
| PostgreSQL CDC | 🚧 Planned | Change Data Capture from Postgres |
| MySQL CDC | 🚧 Planned | Change Data Capture from MySQL |
| File/Directory | 🚧 Planned | File system monitoring |
| HTTP Polling | 🚧 Planned | REST API polling |
| Kafka | 🚧 Planned | Kafka topic mirroring |

### Bridge Connectors (Bidirectional)

| Connector | Status | Description |
|-----------|--------|-------------|
| Kafka Bridge | Planned | Bidirectional Kafka integration |
| RabbitMQ Bridge | Planned | AMQP protocol bridging |

## 🛠️ Development Workflow

```text
1. Design Phase
   ├── Define external system integration requirements
   ├── Choose connector type (Sink/Source/Bridge)
   └── Plan message transformation strategy

2. Implementation Phase
   ├── Create connector project structure
   ├── Implement connector trait
   ├── Add configuration handling
   ├── Implement transformation logic
   └── Add error handling

3. Testing Phase
   ├── Unit tests for transformation logic
   ├── Integration tests with real external system
   ├── Performance testing with load
   └── Failure scenario testing

4. Deployment Phase
   ├── Create Dockerfile
   ├── Build Docker image
   ├── Deploy with docker-compose or Kubernetes
   └── Monitor metrics and logs

5. Maintenance Phase
   ├── Monitor connector health
   ├── Update dependencies
   ├── Add features based on feedback
   └── Performance optimization
```

## 📈 Performance Benchmarks

**Target Metrics** (to be validated):

| Metric | Target | Notes |
|--------|--------|-------|
| Sink Throughput | 50K msgs/sec | With batching enabled |
| Source Throughput | 30K msgs/sec | CDC workloads |
| End-to-end Latency | < 10ms | Non-reliable mode |
| End-to-end Latency | < 100ms | Reliable mode with persistence |
| Memory Usage | < 100MB | Per connector instance |

## 🔧 Configuration Reference

### Common Environment Variables

```bash
# Danube connection
DANUBE_SERVICE_URL="http://localhost:6650"
CONNECTOR_NAME="my-connector-1"

# For Sink Connectors
DANUBE_TOPIC="/default/source-topic"
SUBSCRIPTION_NAME="my-subscription"
SUBSCRIPTION_TYPE="Exclusive"  # or Shared, Failover

# For Source Connectors  
DANUBE_TOPIC="/default/destination-topic"
DISPATCH_STRATEGY="Reliable"  # or NonReliable

# Runtime configuration
MAX_RETRIES=3
RETRY_BACKOFF_MS=1000
BATCH_SIZE=1000
BATCH_TIMEOUT_MS=1000
POLL_INTERVAL_MS=100

# Observability
METRICS_PORT=9090
LOG_LEVEL="info"
RUST_LOG="info,danube_connect_core=debug"
```

## 🐛 Troubleshooting

### Common Issues

1. **Connection Refused**
   - Verify Danube broker is running
   - Check firewall and network settings
   - Ensure correct service URL format

2. **Topic Not Found**
   - Create topic via Danube admin CLI
   - Check topic name format (must start with /)

3. **High Latency**
   - Enable batching if not already
   - Check network latency to broker
   - Profile connector for bottlenecks

4. **Message Loss**
   - Ensure using `Reliable` dispatch strategy
   - Check acknowledgment logic
   - Verify external system writes are durable

5. **Memory Issues**
   - Reduce batch size
   - Check for memory leaks in connector code
   - Monitor with metrics

## 🤝 Contributing

We welcome contributions! Areas where we need help:

- **New Connectors:** Implement connectors for popular systems
- **Documentation:** Improve guides and examples
- **Testing:** Add test coverage and integration tests
- **Performance:** Optimize hot paths and reduce overhead
- **Features:** Dead-letter queues, schema evolution, exactly-once semantics

## 📞 Support & Community

- **Documentation:** You're reading it!
- **GitHub Issues:** Report bugs and request features
- **Discord/Slack:** Join the Danube community (link TBD)
- **Email:** support@danube-messaging.io (if applicable)

## 🗺️ Roadmap

### Phase 1: Foundation (Q1 2024)
- ✅ Design architecture and documentation
- ⏳ Implement `danube-connect-core` library
- ⏳ Create connector project templates
- ⏳ Setup CI/CD pipeline

### Phase 2: Initial Connectors (Q2 2024)
- ⏳ HTTP/Webhook Sink
- ⏳ PostgreSQL CDC Source
- ⏳ ClickHouse Sink

### Phase 3: Ecosystem Growth (Q3-Q4 2024)
- ⏳ MQTT Bridge
- ⏳ Kafka Bridge
- ⏳ Additional database connectors
- ⏳ Monitoring dashboard

### Phase 4: Enterprise Features (2025)
- Schema registry integration
- Exactly-once semantics
- Dynamic configuration
- Multi-tenancy support

## 📄 License

Danube Connect follows the same license as Danube Messaging (check main repository).

## 🙏 Acknowledgments

- **Danube Team:** For building an amazing messaging platform
- **Community Contributors:** For feedback and contributions
- **Inspiration:** Apache Kafka Connect, Debezium, and other connector frameworks

---

**Ready to build your first connector?** Start with the [connector-development-guide.md](./connector-development-guide.md)!
