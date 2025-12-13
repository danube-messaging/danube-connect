# Danube Connect Examples

This directory contains complete, runnable examples for each connector. Each example includes:
- 🐳 Docker Compose setup with all required services
- 📝 Configuration templates
- 🧪 Test scripts and sample data
- 📖 Quick start guide

## 🚀 Quick Start

Run any example with the universal test script:

```bash
# Start an example
./scripts/test-connector.sh source-mqtt start

# View logs
./scripts/test-connector.sh source-mqtt logs

# Run test publisher
./scripts/test-connector.sh source-mqtt test

# Stop the example
./scripts/test-connector.sh source-mqtt stop
```

## 📋 Available Examples

### Source Connectors

#### [MQTT Source](./source-mqtt/)
**Status:** ✅ Available  
**Use Case:** IoT sensor data ingestion, telemetry collection, edge device integration

Bridges MQTT topics to Danube topics. Perfect for Industrial IoT, smart cities, connected vehicles, and edge computing scenarios.

```bash
./scripts/test-connector.sh source-mqtt start
```

**Features:**
- Wildcard topic subscriptions (`+`, `#`)
- QoS 0, 1, 2 support
- Metadata preservation
- TLS/authentication support

---

#### PostgreSQL CDC Source
**Status:** 🚧 Coming Soon  
**Use Case:** Database change data capture, event-driven architecture, CQRS

Streams PostgreSQL database changes to Danube using logical replication.

---

### Sink Connectors

#### ClickHouse Sink
**Status:** 🚧 Coming Soon  
**Use Case:** Real-time analytics, data warehousing, OLAP queries

Writes Danube messages to ClickHouse for high-performance analytics.

---

#### HTTP Webhook Sink
**Status:** 🚧 Coming Soon  
**Use Case:** External API integration, webhook delivery, event forwarding

Sends Danube messages to HTTP endpoints with retry and error handling.

---

### Bridge Connectors

#### Kafka Bridge
**Status:** 🚧 Coming Soon  
**Use Case:** Kafka migration, hybrid architectures, legacy system integration

Bidirectional bridge between Kafka and Danube topics.

---

## 📁 Example Structure

Each example follows a standardized structure:

```
examples/<connector-name>/
├── README.md              # Quick start guide and documentation
├── docker-compose.yml     # Complete stack (Danube + external systems + connector)
├── .env.example           # Configuration template with all options
├── <configs>              # External system configurations
└── test-*.sh              # Test scripts for data generation
```

## 🎯 Example Features

All examples are:
- **Self-contained** - Everything needed to run in one directory
- **Production-like** - Configurations similar to real deployments
- **Well-documented** - Clear README with troubleshooting
- **Tested** - Include test data generators
- **Consistent** - Follow the same structure across all connectors

## 🛠️ Working with Examples

### Starting an Example

```bash
cd examples/<connector-name>
docker-compose up
```

Or use the helper script:
```bash
./scripts/test-connector.sh <connector-name> start
```

### Viewing Logs

```bash
cd examples/<connector-name>
docker-compose logs -f <service-name>

# Or for the connector specifically
docker-compose logs -f <connector-name>-connector
```

### Stopping an Example

```bash
cd examples/<connector-name>
docker-compose down

# Or with cleanup
docker-compose down -v  # Also removes volumes
```

### Customizing Configuration

```bash
cd examples/<connector-name>

# Copy template
cp .env.example .env

# Edit configuration
nano .env

# Restart with new config
docker-compose down
docker-compose up
```

## 🧪 Testing Workflow

1. **Start the example**
   ```bash
   ./scripts/test-connector.sh source-mqtt start
   ```

2. **Watch the logs**
   ```bash
   docker-compose logs -f mqtt-connector
   ```

3. **Generate test data**
   ```bash
   ./test-publisher.sh
   ```

4. **Verify data flow**
   - Check connector logs for "processed" messages
   - Query Danube broker for published messages
   - Verify downstream consumers receive data

5. **Stop when done**
   ```bash
   ./scripts/test-connector.sh source-mqtt stop
   ```

## 📊 Monitoring

Each example includes:
- **Health checks** - Docker Compose healthcheck configurations
- **Logs** - Structured logging with appropriate levels
- **Metrics** - Prometheus-compatible metrics (where applicable)

View all container status:
```bash
cd examples/<connector-name>
docker-compose ps
```

## 🐛 Troubleshooting

### Container Won't Start

```bash
# Check logs
docker-compose logs <service-name>

# Check Docker resources
docker system df

# Restart everything
docker-compose down && docker-compose up
```

### Port Already in Use

```bash
# Find process using port
lsof -i :1883  # Example for MQTT

# Change port in docker-compose.yml or .env
```

### Network Issues

```bash
# Inspect network
docker network inspect <example>_<network-name>

# Recreate network
docker-compose down
docker network prune
docker-compose up
```

## 🔧 Development

To modify an example:

1. Edit files in `examples/<connector-name>/`
2. Test changes: `docker-compose down && docker-compose up`
3. Verify functionality
4. Update documentation in README.md

To create a new example, copy an existing one and modify:

```bash
cp -r examples/source-mqtt examples/source-new-connector
cd examples/source-new-connector
# Edit docker-compose.yml, README.md, configs, etc.
```

## 📚 Additional Resources

- [Connector Development Guide](../info/connector-development-guide.md)
- [Message Patterns](../info/connector-message-patterns.md)
- [RPC Integration](../info/connector-rpc-integration.md)
- [Danube Documentation](https://github.com/danube-messaging/danube)

## 🆘 Getting Help

- Check the example's README.md for specific troubleshooting
- View connector logs for errors
- Review Docker Compose healthcheck status
- Open an issue: https://github.com/danube-messaging/danube-connect/issues

## 🤝 Contributing

We welcome new examples! To contribute:

1. Create a new example following the standard structure
2. Include comprehensive documentation
3. Add test scripts
4. Verify everything works end-to-end
5. Submit a pull request

Make sure your example includes:
- [ ] Complete docker-compose.yml with healthchecks
- [ ] Detailed README.md with quick start
- [ ] .env.example with all configuration options
- [ ] Test script or sample data
- [ ] Troubleshooting section

---

**Need to add your connector example?** Follow the structure above and submit a PR!
