# Phoenix SDK Documentation

## Overview

Phoenix SDK is a developer-friendly API layer built on top of the STOQ transport protocol, providing simplified access to high-performance distributed networking capabilities. This documentation reflects the **actual implemented functionality** as of the current codebase.

## Documentation Index

### Getting Started
- [Quick Start Guide](quickstart.md) - Get running in 5 minutes
- [Installation Guide](installation.md) - Setup and configuration
- [Migration Guide](migration.md) - Upgrading from raw STOQ

### API Documentation
- [API Reference](api/README.md) - Complete API documentation
- [Phoenix Transport](api/transport.md) - Core transport API
- [Phoenix Connection](api/connection.md) - Connection management
- [Phoenix Builder](api/builder.md) - Configuration builder

### Developer Guides
- [Performance Guide](performance.md) - Real performance metrics and optimization
- [Error Handling](errors.md) - Common errors and solutions
- [Integration Guide](integration.md) - Integrating with existing systems
- [Security Guide](security.md) - Security considerations

### Architecture
- [Architecture Overview](architecture.md) - System design and components
- [STOQ Integration](stoq-integration.md) - How Phoenix uses STOQ
- [Network Protocols](protocols.md) - Protocol details

### Community
- [Contributing Guide](../CONTRIBUTING.md) - How to contribute
- [Ecosystem Overview](ecosystem.md) - Related projects
- [Roadmap](roadmap.md) - Future development plans

## Current Implementation Status

### ✅ Implemented Features

#### Core Functionality
- **Phoenix Transport API** - Simplified wrapper around STOQ transport
- **Connection Management** - Automatic pooling and reuse
- **IPv6-Only Networking** - Full IPv6 support, no IPv4 fallback
- **Certificate Management** - Automatic certificate provisioning
- **Performance Monitoring** - Real-time metrics collection

#### API Features
- **PhoenixTransport** - Main transport interface
- **PhoenixConnection** - Connection wrapper with metrics
- **PhoenixBuilder** - Fluent configuration API
- **PhoenixConfig** - Configuration structure
- **PerformanceMetrics** - Performance statistics

#### Transport Features
- **QUIC Protocol** - Using quinn library for QUIC implementation
- **Connection Pooling** - Reuse connections for efficiency
- **Stream Multiplexing** - Multiple streams per connection
- **Zero-Copy Operations** - When supported by platform
- **Memory Pooling** - Reduced allocation overhead

### 🚧 In Development

#### Phoenix-Specific Features
- **Automatic Load Balancing** - Distribute across multiple connections
- **Smart Routing** - Optimal path selection
- **Resilience Features** - Automatic failover and recovery
- **Compression** - Transparent data compression

#### Integration Features
- **FFI Bindings** - C/C++ integration
- **WebAssembly Support** - Browser compatibility
- **Python Bindings** - Python SDK
- **gRPC Integration** - Protocol buffer support

### ❌ Not Yet Implemented

#### Advanced Features
- **Hardware Acceleration** - DPDK/io_uring support
- **Quantum-Resistant Crypto** - FALCON integration pending
- **Multi-Path Transport** - Simultaneous multi-path
- **Global Load Balancing** - Cross-region optimization

## Performance Reality Check

### Measured Performance (Actual)
Based on real benchmark results from `/stoq/examples/benchmark_real.rs`:

| Metric | Measured Value | Test Conditions |
|--------|---------------|-----------------|
| **Peak Throughput** | 0.4-0.5 Gbps | Local loopback, single connection |
| **Connection Setup** | <10ms | Local network |
| **Stream Creation** | <1ms | Existing connection |
| **Memory Usage** | ~50-100MB | Per connection with buffers |
| **CPU Usage** | 10-20% | Single core at peak |

### Performance Factors
- **Network Infrastructure** - Limited by actual network capacity
- **CPU Performance** - Single-thread bottlenecks in QUIC
- **Memory Bandwidth** - Copy operations impact throughput
- **Kernel Overhead** - System call overhead for networking

### Honest Performance Expectations

#### Development Environment
- **Local Testing**: 100-500 Mbps typical
- **Docker/Containers**: 50-200 Mbps due to overlay networking
- **VM Environment**: 100-300 Mbps depending on hypervisor

#### Production Environment
- **Same Datacenter**: 1-5 Gbps achievable
- **Cross-Region**: 100-500 Mbps (latency-limited)
- **Internet**: 10-100 Mbps (bandwidth-limited)

## Architecture Overview

Phoenix SDK is structured as a three-layer architecture:

```
┌─────────────────────────────────────┐
│         Phoenix SDK API             │  Developer-facing API
├─────────────────────────────────────┤
│      STOQ Transport Protocol        │  QUIC-based transport
├─────────────────────────────────────┤
│        Network Layer (IPv6)         │  IPv6-only networking
└─────────────────────────────────────┘
```

### Component Relationships
- **Phoenix SDK** (`/stoq/src/phoenix.rs`) - High-level API
- **STOQ Transport** (`/stoq/src/transport/`) - Transport implementation
- **Quinn QUIC** - Underlying QUIC protocol implementation
- **Rustls** - TLS implementation for security

## Quality Commitment

This documentation is committed to:

1. **Accuracy** - All documented features are implemented and tested
2. **Honesty** - Performance metrics are measured, not theoretical
3. **Completeness** - All public APIs are documented
4. **Clarity** - Examples work and are tested
5. **Maintenance** - Documentation stays current with code

## Getting Help

- **Documentation**: You're reading it!
- **Examples**: `/stoq/examples/` directory
- **Tests**: `/stoq/tests/` for usage patterns
- **Issues**: GitHub issue tracker for bugs/features

## License

Phoenix SDK is part of the Web3 ecosystem project. See LICENSE file for details.