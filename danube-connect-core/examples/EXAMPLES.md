# Running Examples

The examples are designed to work out-of-the-box with sensible defaults, but can also be customized via environment variables.

## Quick Start (No Configuration)

### Simple Sink Connector

```bash
cargo run --example simple_sink
```

This will:
- Connect to `http://localhost:6650` (default Danube broker)
- Subscribe to `/default/test` topic
- Print all received messages to stdout
- Requires a running Danube broker and messages in the topic

### Simple Source Connector

```bash
cargo run --example simple_source
```

This will:
- Connect to `http://localhost:6650` (default Danube broker)
- Generate 100 test messages
- Publish them to `/default/test` topic
- Print each generated message to stdout
- Requires a running Danube broker

## Custom Configuration (Environment Variables)

### Sink Connector with Custom Settings

```bash
DANUBE_SERVICE_URL=http://my-broker:6650 \
CONNECTOR_NAME=my-sink \
DANUBE_TOPIC=/custom/topic \
SUBSCRIPTION_NAME=my-subscription \
cargo run --example simple_sink
```

### Source Connector with Custom Settings

```bash
DANUBE_SERVICE_URL=http://my-broker:6650 \
CONNECTOR_NAME=my-source \
DANUBE_TOPIC=/custom/topic \
cargo run --example simple_source
```

## Available Environment Variables

### Common
- `DANUBE_SERVICE_URL` - Broker address (default: `http://localhost:6650`)
- `CONNECTOR_NAME` - Connector name (default: `simple-sink` or `simple-source`)
- `DANUBE_TOPIC` - Topic to use (default: `/default/test`)

### Sink-specific
- `SUBSCRIPTION_NAME` - Subscription name (default: `simple-sink-sub`)
- `SUBSCRIPTION_TYPE` - Type: `exclusive`, `shared`, or `failover` (default: `exclusive`)

### Runtime Configuration
- `RELIABLE_DISPATCH` - Use reliable dispatch (default: `true`)
- `MAX_RETRIES` - Max retry attempts (default: `3`)
- `RETRY_BACKOFF_MS` - Initial backoff in ms (default: `1000`)
- `BATCH_SIZE` - Batch size (default: `1000`)
- `BATCH_TIMEOUT_MS` - Batch timeout in ms (default: `5000`)
- `POLL_INTERVAL_MS` - Poll interval in ms (default: `100`)

## Testing Without a Broker

The examples will fail to connect if no Danube broker is running. To test the connector logic without a broker, you can:

1. **Mock the Danube client** - Modify the examples to use a mock client
2. **Use Docker Compose** - Run a local Danube broker:
   ```bash
   docker-compose up -d  # Requires docker-compose.yml with Danube
   ```
3. **Write unit tests** - Test the connector trait implementations directly

## Example Output

### Simple Sink
```
Using default configuration for testing
To use custom settings, set environment variables:
  DANUBE_SERVICE_URL (default: http://localhost:6650)
  CONNECTOR_NAME (default: simple-sink)
  DANUBE_TOPIC (default: /default/test)
  SUBSCRIPTION_NAME (default: simple-sink-sub)

SimpleSinkConnector initialized
Configuration: ConnectorConfig { ... }
=== Message #1 ===
Topic: /default/test
Offset: 0
Producer: test-producer
Publish Time: 1702400000
Payload Size: 45 bytes
Payload (text): Test message from producer
Attributes:
  source = test-source
```

### Simple Source
```
Using default configuration for testing
To use custom settings, set environment variables:
  DANUBE_SERVICE_URL (default: http://localhost:6650)
  CONNECTOR_NAME (default: simple-source)
  DANUBE_TOPIC (default: /default/test)

SimpleSourceConnector initialized
Configuration: ConnectorConfig { ... }
Will generate 100 messages
Generated message #1
Generated message #2
...
```

## Stopping Examples

Press `Ctrl+C` to gracefully shutdown the connector. The examples will:
- Call the `shutdown()` method on the connector
- Print shutdown summary
- Exit cleanly
