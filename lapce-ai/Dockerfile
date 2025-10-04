# Dockerfile for Titan Semantic Search Service
# Multi-stage build for optimized production image

# Stage 1: Builder
# Production-ready Docker image for lapce-ai-rust IPC server
# Optimized for <10μs latency and >1M msg/sec throughput
FROM rust:1.75-slim-bullseye as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code

# Copy actual source code
COPY src ./src
COPY examples ./examples

# Build the actual binary
RUN cargo build --release --bin lapce-ai-server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 lapce && \
    mkdir -p /var/run/lapce-ai /etc/lapce-ai /dev/shm && \
    chown -R lapce:lapce /var/run/lapce-ai /etc/lapce-ai /dev/shm

# Copy binary and config
COPY --from=builder /app/target/release/lapce-ai-server /usr/local/bin/
COPY --from=builder /app/deploy/config.toml /etc/lapce-ai/config.toml

# Set permissions
RUN chmod +x /usr/local/bin/lapce-ai-server

USER lapce

# Expose metrics port
EXPOSE 9090

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:9090/health || exit 1

# Environment variables
ENV RUST_LOG=info
ENV LAPCE_AI_CONFIG=/etc/lapce-ai/config.toml

# Run the server
CMD ["lapce-ai-server", "--config", "/etc/lapce-ai/config.toml"]

# Copy default configuration
COPY --chown=lapce:lapce lapce-ipc.toml /etc/lapce/config.toml

# Switch to non-root user
USER lapce

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD test -S /var/run/lapce/ipc.sock || exit 1

# Expose metrics port
EXPOSE 9090

# Volume for socket and logs
VOLUME ["/var/run/lapce", "/var/log/lapce"]

# Environment variables
ENV IPC_SOCKET_PATH=/var/run/lapce/ipc.sock \
    RUST_LOG=info \
    IPC_MAX_CONNECTIONS=2000

# Entrypoint
ENTRYPOINT ["/usr/local/bin/lapce-ipc-server"]
# Run the semantic search server
CMD ["lapce-ai-rust", "serve", "--host", "0.0.0.0", "--port", "8080"]
