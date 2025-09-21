# STOQ Protocol Documentation

## Overview
STOQ (Secure Transport Optimization with QUIC) is the next-generation transport protocol replacing TCP/IP with quantum-resistant, Byzantine fault-tolerant communication designed for the Internet 2.0 era.

## Architecture

### Core Features
- **QUIC-based Transport**: Built on QUIC for low-latency, multiplexed streams
- **IPv6-only**: Native IPv6 with no IPv4 fallback
- **TrustChain Integration**: Automatic certificate management and validation
- **Byzantine Tolerance**: Resilient to 33% malicious nodes
- **CDN Capabilities**: Edge caching, routing, and content distribution

### Protocol Stack
```
Application Layer (HTTP/3)
     ↓
STOQ Protocol Layer
     ↓
QUIC Transport (with TrustChain)
     ↓
IPv6 Network Layer
```

## Implementation Details

### Transport Features
- **Stream Multiplexing**: Multiple concurrent streams per connection
- **0-RTT Connection**: Resume connections without handshake overhead
- **Connection Migration**: Seamless IP address changes
- **Loss Recovery**: Advanced congestion control and retransmission

### Security
- **TLS 1.3**: Modern encryption with forward secrecy
- **Certificate Pinning**: TrustChain certificate validation
- **Quantum Resistance**: Post-quantum cryptography ready
- **Byzantine Detection**: Real-time malicious node identification

### Performance
- **Current**: 2.95 Gbps throughput (bottleneck identified)
- **Target**: 40 Gbps for production deployment
- **Latency**: <1ms for local connections
- **Connections**: 100K+ concurrent streams supported

## Performance Optimization Plan

### Identified Bottlenecks
1. **QUIC Implementation**: Current Rust QUIC library limitations
2. **Buffer Management**: Suboptimal memory allocation
3. **CPU Affinity**: Lack of core pinning for network threads
4. **Kernel Bypass**: Not utilizing DPDK/XDP for packet processing

### Optimization Roadmap
1. **Phase 1**: Replace QUIC library with optimized implementation
2. **Phase 2**: Implement zero-copy buffer management
3. **Phase 3**: Add DPDK/XDP kernel bypass
4. **Phase 4**: Hardware offloading (NIC acceleration)

## API Reference

### Client API
```rust
// Initialize STOQ client
let client = StoqClient::new(config)?;

// Connect to endpoint
let connection = client.connect("http3://destination")?;

// Send data
connection.send(data).await?;
```

### Server API
```rust
// Create STOQ server
let server = StoqServer::bind("[::]:443")?;

// Accept connections
while let Some(conn) = server.accept().await {
    handle_connection(conn);
}
```

## Integration

### With TrustChain
- Automatic certificate provisioning
- Certificate rotation every 24 hours
- Certificate Transparency validation
- DNS-over-QUIC resolution

### With HyperMesh
- Transport for asset operations
- Secure remote procedure calls
- Distributed consensus messages
- Resource allocation commands

## Deployment
- **Development**: Self-signed certificates for testing
- **Staging**: Let's Encrypt integration
- **Production**: HSM-backed certificate management

## Status
- ✅ Core protocol implemented
- ✅ TrustChain integration complete
- ✅ IPv6-only enforcement
- ⚠️ Performance optimization required (2.95 Gbps → 40 Gbps)

## References
- [Performance Analysis](./PERFORMANCE.md)
- [Protocol Specification](./PROTOCOL.md)
- [Integration Guide](./INTEGRATION.md)