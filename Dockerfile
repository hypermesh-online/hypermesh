# Multi-stage Dockerfile for Nexus Hypermesh
ARG RUST_VERSION=1.75
FROM rust:${RUST_VERSION}-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    clang \
    llvm \
    pkg-config \
    libssl-dev \
    libclang-dev \
    libbpf-dev \
    linux-headers-generic \
    && rm -rf /var/lib/apt/lists/*

# Set work directory
WORKDIR /usr/src/nexus

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY core/ ./core/
COPY interface/ ./interface/

# Build core components
WORKDIR /usr/src/nexus/core
RUN cargo build --release --workspace

# Build interface components
WORKDIR /usr/src/nexus/interface/phase2-c2
RUN cargo build --release --workspace

# Runtime stage
FROM ubuntu:22.04 as runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    libbpf1 \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd -r nexus \
    && useradd -r -g nexus nexus

# Create directories
RUN mkdir -p /etc/nexus /var/lib/nexus /var/log/nexus \
    && chown -R nexus:nexus /etc/nexus /var/lib/nexus /var/log/nexus

# Copy binaries from builder stage
COPY --from=builder /usr/src/nexus/core/target/release/nexus-* /usr/local/bin/
COPY --from=builder /usr/src/nexus/interface/phase2-c2/target/release/nexus-cli /usr/local/bin/
COPY --from=builder /usr/src/nexus/interface/phase2-c2/target/release/nexus-api-server /usr/local/bin/

# Copy configuration templates
COPY deploy/config/ /etc/nexus/

# Set up entrypoint
COPY deploy/scripts/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Switch to non-root user
USER nexus

# Expose ports
EXPOSE 7777/udp 8080/tcp 9090/tcp

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

# Default command
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["nexus-coordinator"]

# Development stage (optional)
FROM builder as development

RUN cargo install cargo-watch cargo-audit cargo-tarpaulin

WORKDIR /usr/src/nexus

# Install additional development tools
RUN apt-get update && apt-get install -y \
    gdb \
    strace \
    tcpdump \
    netstat-nat \
    && rm -rf /var/lib/apt/lists/*

CMD ["cargo", "watch", "-x", "run"]

# Test stage
FROM builder as test

WORKDIR /usr/src/nexus/core

# Run tests
RUN cargo test --workspace --release

# Generate coverage report
RUN cargo tarpaulin --out Html --output-dir /usr/src/nexus/coverage

# Benchmark stage  
FROM builder as benchmark

WORKDIR /usr/src/nexus/core

# Run benchmarks
RUN cargo bench --workspace