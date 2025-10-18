# Multi-stage build for CST-tree-sitter
# Optimized for production deployment

# Stage 1: Builder
FROM rust:1.75-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /usr/src/cst-tree-sitter

# Copy manifest files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source code
COPY . .

# Clean and rebuild with actual source
RUN touch src/lib.rs && \
    cargo build --release --all-features

# Build binaries
RUN cargo build --release --bins

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash cstuser

# Copy binaries from builder
COPY --from=builder /usr/src/cst-tree-sitter/target/release/benchmark_* /usr/local/bin/

# Create cache directory with proper permissions
RUN mkdir -p /var/cache/cst-tree-sitter && \
    chown -R cstuser:cstuser /var/cache/cst-tree-sitter

# Create config directory
RUN mkdir -p /etc/cst-tree-sitter && \
    chown -R cstuser:cstuser /etc/cst-tree-sitter

# Copy default configuration
COPY configs/default.toml /etc/cst-tree-sitter/config.toml
RUN chown cstuser:cstuser /etc/cst-tree-sitter/config.toml

# Switch to non-root user
USER cstuser

# Set environment variables
ENV RUST_LOG=info
ENV CST_CACHE_DIR=/var/cache/cst-tree-sitter
ENV CST_CONFIG_FILE=/etc/cst-tree-sitter/config.toml

# Expose metrics port
EXPOSE 9090

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:9090/healthz || exit 1

# Default command (can be overridden)
CMD ["benchmark_performance"]

# Labels for metadata
LABEL maintainer="CST-tree-sitter Team"
LABEL version="1.0.0"
LABEL description="High-performance CST caching system with 95% memory reduction"
