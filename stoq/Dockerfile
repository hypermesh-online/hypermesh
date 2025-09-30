# Multi-stage build for STOQ Protocol
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Build dependencies (cached layer)
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy source code
COPY src ./src
COPY examples ./examples
COPY benches ./benches

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
RUN useradd -m -u 1000 -s /bin/bash stoq

# Copy binary from builder
COPY --from=builder /app/target/release/stoq /usr/local/bin/stoq

# Create config directory
RUN mkdir -p /etc/stoq && chown stoq:stoq /etc/stoq

# Switch to non-root user
USER stoq

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD stoq health || exit 1

# Expose QUIC port (UDP)
EXPOSE 6001/udp

# Set resource limits
ENV RUST_LOG=info
ENV STOQ_MAX_CONNECTIONS=10000
ENV STOQ_MEMORY_POOL_SIZE=1073741824

# Run STOQ server
ENTRYPOINT ["stoq"]
CMD ["server", "--config", "/etc/stoq/config.toml"]