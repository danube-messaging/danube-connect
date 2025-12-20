# Multi-stage build for Qdrant Sink Connector
FROM rust:1.75 as builder

WORKDIR /usr/src/app

# Copy workspace files
COPY Cargo.toml ./
COPY danube-connect-core ./danube-connect-core
COPY connectors/sink-qdrant ./connectors/sink-qdrant

# Build the connector in release mode
WORKDIR /usr/src/app/connectors/sink-qdrant
RUN cargo build --release

# Runtime stage - minimal image
FROM debian:bookworm-slim

# Install CA certificates for HTTPS connections
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder \
    /usr/src/app/target/release/danube-sink-qdrant \
    /usr/local/bin/danube-sink-qdrant

# Run as non-root user for security
RUN useradd -m -u 1000 danube && \
    chown -R danube:danube /usr/local/bin/danube-sink-qdrant

USER danube
WORKDIR /home/danube

# Expose metrics port
EXPOSE 9090

ENTRYPOINT ["danube-sink-qdrant"]
