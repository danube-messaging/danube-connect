# Running Examples

The examples are designed to work out-of-the-box with sensible defaults, but can also be customized via environment variables.

## Prerequisites

### Start Danube Broker

Before running any examples, you need a running Danube broker. Use the provided Docker Compose setup:

```bash
# From the project root, navigate to the docker folder
cd docker

# Start the Danube broker
docker-compose up -d

# Verify the broker is running
docker-compose ps
```

The broker will be available at `http://localhost:6650`.

To stop the broker:
```bash
cd docker
docker-compose down
```

## Quick Start (No Configuration)

**Important:** Run the **source** example first, as it creates the topic on the broker. Then run the sink example to consume the messages.

### Step 1: Simple Source Connector (Run First)

```bash
# Terminal 1 - Start the source connector
cargo run --example simple_source
```

This will:
- Connect to `http://localhost:6650` (default Danube broker)
- Create the `/default/test` topic (if it doesn't exist)
- Generate 100 test messages
- Publish them to `/default/test` topic
- Print each generated message to stdout
- Requires a running Danube broker

### Step 2: Simple Sink Connector (Run Second)

```bash
# Terminal 2 - Start the sink connector
cargo run --example simple_sink
```

This will:
- Connect to `http://localhost:6650` (default Danube broker)
- Subscribe to `/default/test` topic
- Print all received messages to stdout
- Requires a running Danube broker and the topic to exist (created by source)

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