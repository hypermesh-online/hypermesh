# Multi-stage build for TrustChain CA
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy source code
COPY src ./src
COPY config ./config

# Build release binary
RUN touch src/main.rs
RUN cargo build --release --locked

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash trustchain

# Copy binary from builder
COPY --from=builder /app/target/release/trustchain-server /usr/local/bin/trustchain-server

# Copy default configuration
COPY --from=builder /app/config /etc/trustchain

# Create certificate storage directory
RUN mkdir -p /var/lib/trustchain/certs && \
    chown -R trustchain:trustchain /var/lib/trustchain

# Switch to non-root user
USER trustchain

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8443/health || exit 1

# Expose ports
EXPOSE 8443/tcp  # HTTPS API
EXPOSE 8853/udp  # DNS over TLS
EXPOSE 6002/udp  # QUIC

# Environment variables
ENV RUST_LOG=info
ENV TRUSTCHAIN_DATA_DIR=/var/lib/trustchain
ENV TRUSTCHAIN_CONFIG=/etc/trustchain/production.toml

# Run TrustChain server
ENTRYPOINT ["trustchain-server"]
CMD ["--config", "/etc/trustchain/production.toml"]