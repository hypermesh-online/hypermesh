# STOQ Protocol - Pure QUIC over IPv6 Transport

**Status: ✅ PRODUCTION READY - Pure Transport Protocol**

STOQ is a quantum-resistant QUIC transport protocol with built-in protocol extensions for tokenization, sharding, and post-quantum security. Clean, professional architecture with no application layer contamination.

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

### Pure Transport Layer
- **Protocol**: QUIC over IPv6 (transport only)
- **Security**: FALCON-1024 quantum-resistant cryptography
- **Extensions**: Packet tokenization, sharding, routing, seeding
- **Performance**: Memory pooling, zero-copy, frame batching
- **Integration**: Certificate management with TrustChain

### Protocol Extensions
- **Packet Tokenization**: SHA-256 cryptographic validation with sequence numbers
- **Packet Sharding**: Automatic fragmentation/reassembly with integrity verification
- **Multi-hop Routing**: IPv6-based hop chain tracking for protocol routing
- **Seeding Protocol**: Foundation for distributed packet replication

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

### Key Features
- **Pure Transport**: No application logic contamination
- **Quantum Secure**: FALCON-1024 post-quantum signatures
- **Protocol Extensions**: Real tokenization, sharding, routing protocols
- **Professional Architecture**: Clean separation of concerns
- **Production Ready**: Full test coverage and validation

## 🛡️ Security

### Transport Security
- TLS 1.3 with perfect forward secrecy
- Certificate-based authentication
- 24-hour certificate rotation

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