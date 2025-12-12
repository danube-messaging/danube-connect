# Build stage
FROM rust:1.75 as builder

WORKDIR /usr/src/app

# Copy workspace configuration
COPY Cargo.toml ./
COPY danube-connect-core ./danube-connect-core

# Copy connector source
COPY connectors/source-mqtt ./connectors/source-mqtt

# Build the connector
WORKDIR /usr/src/app/connectors/source-mqtt
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install CA certificates for HTTPS/TLS connections
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder \
    /usr/src/app/target/release/danube-source-mqtt \
    /usr/local/bin/danube-source-mqtt

# Create non-root user
RUN useradd -m -u 1000 danube && \
    chown -R danube:danube /usr/local/bin/danube-source-mqtt

USER danube

# Set environment defaults
ENV RUST_LOG=info
ENV LOG_LEVEL=info

ENTRYPOINT ["danube-source-mqtt"]
