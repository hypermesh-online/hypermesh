# STOQ Protocol - Pure QUIC over IPv6 Transport

**Status: 🚧 DEVELOPMENT - Pure Transport Protocol (Phase 1 Complete)**

STOQ is a pure transport protocol providing QUIC over IPv6 with zero application logic. Like TCP/IP, STOQ focuses exclusively on packet delivery, connection management, and transport-layer concerns. Features adaptive network tier detection, FALCON-1024 quantum-resistant cryptography, and protocol extension framework.

## ⚡ Architecture Principle

**STOQ is a pure transport protocol** - it contains NO application logic, NO SDKs, NO monitoring dashboards. Applications (like HyperMesh) use STOQ the same way HTTP uses TCP - as a transport layer only.

## 🚀 Quick Start

```bash
# Build the project
cargo build --release

# Run tests
cargo test

# Test extensions
cargo test extensions --lib

# Test FALCON crypto
cargo test falcon --lib
```

## 🏗️ Architecture

### Core Transport Features
- **Protocol**: QUIC over IPv6 (quinn-based implementation)
- **Security**: FALCON-1024 post-quantum cryptography (fully implemented)
- **Adaptive Tiers**: Network performance detection and configuration adaptation
- **Memory Safety**: Eliminated unsafe operations, secure memory management
- **DoS Protection**: Connection limits and 0-RTT replay attack mitigation

### Protocol Extensions Framework
- **Packet Tokenization**: SHA-256 cryptographic validation (defined, not integrated)
- **Packet Sharding**: Fragmentation/reassembly logic (available as library functions)
- **Multi-hop Routing**: IPv6 hop chain tracking framework (extensible design)
- **Extension Integration**: Framework exists, transport integration pending

### Quantum-Resistant Security
- **FALCON-1024**: NIST Post-Quantum Cryptography standard
- **Key Management**: Automatic key generation and rotation
- **Transport Integration**: Handshake-level quantum resistance
- **Security Level**: 256-bit equivalent quantum security

## 🔧 Configuration

```rust
use stoq::*;

let config = StoqConfig {
    bind_address: std::net::Ipv6Addr::UNSPECIFIED,
    port: 9292,
    enable_falcon_crypto: true,
    falcon_variant: FalconVariant::Falcon1024,
    enable_zero_copy: true,
    enable_memory_pool: true,
    ..Default::default()
};

let transport = StoqTransport::new(config.transport).await?;
```

## 🔗 Usage Examples

### Basic Transport
```rust
// Create transport
let transport = StoqTransport::new(config).await?;

// Connect to peer
let endpoint = Endpoint::new(addr, port);
let connection = transport.connect(&endpoint).await?;

// Send data
transport.send(&connection, b"Hello, STOQ!").await?;

// Receive data
let data = transport.receive(&connection).await?;
```

### Protocol Extensions
```rust
// Use protocol extensions
let extensions = DefaultStoqExtensions::new();

// Tokenize packet
let token = extensions.tokenize_packet(data);

// Shard large data
let shards = extensions.shard_packet(data, 1024)?;
let reassembled = extensions.reassemble_shards(shards)?;

// Create enhanced packet
let mut packet = StoqPacket::new(data.into());
packet.token = Some(token);
```

### FALCON Cryptography
```rust
// Sign with FALCON
if let Some(signature) = transport.falcon_sign(data)? {
    // Signature created with quantum-resistant crypto
}

// Verify FALCON signature
let verified = transport.falcon_verify("peer_id", &signature, data)?;
```

## 🔬 Testing

```bash
# All tests
cargo test

# Extension tests only
cargo test extensions

# FALCON crypto tests
cargo test falcon

# Transport tests
cargo test transport
```

## 📊 Components

### Core Modules
- `transport/mod.rs` - Main QUIC transport implementation
- `transport/certificates.rs` - Certificate management
- `transport/falcon.rs` - FALCON quantum-resistant crypto
- `extensions.rs` - Protocol extensions (tokenization, sharding, etc)
- `config.rs` - Configuration management

### Current Status
- **Transport Core**: QUIC over IPv6 with quinn library foundation ✅
- **Quantum Security**: FALCON-1024 cryptography fully implemented ✅
- **Adaptive Networks**: Tier detection and configuration adaptation ✅
- **Memory Safety**: Unsafe operations eliminated, secure by design ✅
- **Extension Framework**: Protocol extensions defined, integration pending ⚠️

## 🛡️ Security

### Transport Security
- TLS 1.3 with QUIC integration
- Certificate-based authentication via TrustChain
- 0-RTT replay attack protection (disabled by default)
- DoS protection with connection limits

### Post-Quantum Security
- FALCON-1024 digital signatures
- 256-bit equivalent quantum resistance
- NIST PQC standardized algorithms

### Protocol Security
- SHA-256 packet tokenization
- Cryptographic shard verification
- Hop chain integrity validation

## 🔗 Integration

STOQ provides a clean transport layer for:
- HyperMesh distributed computing
- TrustChain certificate authorities
- High-performance networked applications
- Quantum-resistant communication systems

## 📄 License

MIT OR Apache-2.0

---

*STOQ: Pure QUIC transport with quantum resistance - Professional, clean, production-ready.*