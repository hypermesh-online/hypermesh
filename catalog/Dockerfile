# Multi-stage build for Catalog VM Service
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
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
COPY benches ./benches

# Build release binary with all optimizations
RUN touch src/main.rs
RUN RUSTFLAGS="-C target-cpu=native" cargo build --release --locked

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash catalog

# Copy binary from builder
COPY --from=builder /app/target/release/catalog /usr/local/bin/catalog

# Create necessary directories
RUN mkdir -p /var/lib/catalog/cache && \
    mkdir -p /etc/catalog && \
    chown -R catalog:catalog /var/lib/catalog /etc/catalog

# Switch to non-root user
USER catalog

# Health check
HEALTHCHECK --interval=20s --timeout=3s --start-period=5s --retries=3 \
    CMD catalog status || exit 1

# Expose service port
EXPOSE 7001/tcp

# Environment variables for performance tuning
ENV RUST_LOG=info
ENV CATALOG_CACHE_SIZE=2147483648
ENV CATALOG_MAX_WORKERS=8
ENV CATALOG_ENABLE_JIT=true

# Run Catalog service
ENTRYPOINT ["catalog"]
CMD ["serve", "--bind", "0.0.0.0:7001"]