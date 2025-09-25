# STOQ: Pure QUIC over IPv6 Transport Protocol

## Overview
STOQ is a professional QUIC transport protocol with quantum-resistant cryptography and protocol extensions. It provides clean, efficient data transport with built-in tokenization, sharding, and post-quantum security for production environments.

## Current Status
✅ **PRODUCTION READY**: Pure transport protocol with FALCON-1024 quantum resistance and protocol extensions

## Architecture

### Core Components

#### Pure Transport Layer
- **Protocol**: QUIC over IPv6 (transport layer only)
- **Security**: TLS 1.3 + FALCON-1024 post-quantum crypto
- **Features**: 0-RTT resumption, connection migration, memory pooling
- **Performance**: High-performance concurrent connections
- **Implementation**: Quinn (Rust QUIC library) with custom extensions

#### Protocol Extensions
- **Packet Tokenization**: SHA-256 cryptographic validation with sequence tracking
- **Packet Sharding**: Automatic fragmentation and reassembly with integrity verification
- **Multi-hop Routing**: IPv6-based hop chain tracking for protocol-level routing
- **Seeding Protocol**: Distributed packet replication foundation

#### Quantum-Resistant Security
- **Algorithm**: FALCON-1024 (NIST PQC standard)
- **Security Level**: 256-bit equivalent quantum resistance
- **Key Management**: Automatic generation, rotation, and caching
- **Integration**: Transport-level handshake signatures

#### Certificate Management
- **Provider**: TrustChain integration
- **Rotation**: Automatic 24-hour certificate lifecycle
- **Validation**: Real-time certificate transparency integration
- **Mode**: Production-ready with IPv6-only enforcement

## Protocol Specification

### Transport Frame Structure
```
STOQ Extension Frame:
┌──────────────┬────────────┬─────────────┬──────────────┐
│ Extension ID │ Length     │ Metadata    │ Payload      │
│ (1 byte)     │ (4 bytes)  │ (variable)  │ (variable)   │
└─────────────┴──────────┴───────────┴──────────┘

Extension Types:
0x01 - PACKET_TOKEN (tokenization)
0x02 - PACKET_SHARD (fragmentation/reassembly)
0x03 - HOP_INFO (multi-hop routing)
0x04 - SEED_INFO (packet replication)
0x05 - FALCON_SIGNATURE (quantum-resistant signature)
```

### FALCON Integration
```
QUIC Handshake with FALCON:
Client                          Server
  │                               │
  ├──── TLS ClientHello ─────────→│
  │     + FALCON Extension        │
  │                               │
  │←─── TLS ServerHello ──────────│
  │     + FALCON Public Key       │
  │                               │
  ├──── FALCON Signature ────────→│
  │     (Handshake Data)          │
  │                               │
  │←──── Verification ────────────│
  │      + Connection OK          │
```

### Protocol Extension Flow
```
Extension Processing:
1. Receive raw QUIC packet
2. Check for STOQ extensions
3. Process tokenization (if present)
4. Handle sharding/reassembly
5. Update hop information
6. Execute seeding protocol
7. Verify FALCON signatures
8. Deliver to application
```

## Implementation Details

### Core Modules
- **`transport/mod.rs`**: Main QUIC transport implementation
- **`transport/falcon.rs`**: FALCON-1024 quantum-resistant cryptography
- **`transport/certificates.rs`**: TrustChain certificate management
- **`extensions.rs`**: Protocol extensions (tokenization, sharding, etc.)
- **`config.rs`**: Configuration and defaults

### Protocol Extensions

#### Packet Tokenization
- **Algorithm**: SHA-256 with sequence numbers
- **Purpose**: Cryptographic packet validation
- **Implementation**: `DefaultStoqExtensions::tokenize_packet()`
- **Validation**: Automatic integrity verification

#### Packet Sharding
- **Algorithm**: Automatic fragmentation with hash integrity
- **Purpose**: Transport-level packet fragmentation
- **Implementation**: `DefaultStoqExtensions::shard_packet()`
- **Reassembly**: Cryptographically verified reconstruction

#### Multi-hop Routing
- **Protocol**: IPv6-based hop chain tracking
- **Purpose**: Transport-level routing information
- **Implementation**: `HopInfo` with timestamps and metadata
- **Validation**: Hop chain integrity verification

#### Seeding Protocol
- **Protocol**: Distributed packet replication foundation
- **Purpose**: Protocol-level packet distribution
- **Implementation**: `SeedInfo` with node selection and priority
- **Replication**: Configurable replication factors

### Security Features

#### Transport Security
- **TLS**: 1.3 with perfect forward secrecy
- **Certificates**: TrustChain integration with 24-hour rotation
- **IPv6**: Mandatory IPv6-only networking
- **Validation**: Real-time certificate transparency

#### Post-Quantum Security
- **FALCON-1024**: NIST PQC standardized algorithm
- **Key Size**: 1793 bytes public, 2305 bytes private
- **Signature Size**: 1330 bytes per signature
- **Security Level**: 256-bit equivalent quantum resistance

#### Protocol Security
- **Tokenization**: SHA-256 cryptographic validation
- **Sharding**: Integrity verification on reassembly
- **Hop Validation**: Cryptographic hop chain verification
- **Extension Validation**: All extensions cryptographically secured

## Usage Examples

### Basic Transport
```rust
use stoq::*;

// Create transport configuration
let config = TransportConfig {
    bind_address: std::net::Ipv6Addr::UNSPECIFIED,
    port: 9292,
    enable_falcon_crypto: true,
    falcon_variant: FalconVariant::Falcon1024,
    ..Default::default()
};

// Initialize transport
let transport = StoqTransport::new(config).await?;

// Connect and send data
let endpoint = Endpoint::new(addr, port);
let connection = transport.connect(&endpoint).await?;
transport.send(&connection, b"Hello STOQ!").await?;
```

### Protocol Extensions
```rust
// Initialize extensions
let extensions = DefaultStoqExtensions::new();

// Tokenize packet
let token = extensions.tokenize_packet(data);
assert!(extensions.validate_token(data, &token));

// Shard large data
let shards = extensions.shard_packet(data, 1024)?;
let reassembled = extensions.reassemble_shards(shards)?;
assert_eq!(reassembled, data);

// Enhanced packet with extensions
let mut packet = StoqPacket::new(data.into());
packet.token = Some(token);
let serialized = packet.serialize()?;
```

### FALCON Cryptography
```rust
// Access FALCON transport
if let Some(falcon_transport) = transport.falcon_transport() {
    let falcon = falcon_transport.read();

    // Sign handshake data
    let signature = falcon.sign_handshake_data(handshake_data)?;

    // Verify signature
    let is_valid = falcon.verify_handshake_signature(
        "peer_key_id", &signature, handshake_data
    )?;
}

// Transport-level signing
let signature = transport.falcon_sign(data)?;
if let Some(sig) = signature {
    let verified = transport.falcon_verify("peer", &sig, data)?;
}
```

## Integration Points

### HyperMesh Integration
- **Asset Transport**: Clean transport layer for HyperMesh assets
- **Remote Proxy**: NAT-like addressing support for HyperMesh resources
- **Consensus Transport**: Reliable transport for consensus messages
- **Certificate Integration**: TrustChain certificates for authentication

### TrustChain Integration
- **Certificate Management**: Automatic certificate lifecycle
- **DNS Integration**: Transport for DNS-over-STOQ
- **CA Communication**: Secure communication with certificate authorities
- **Trust Verification**: Real-time trust chain validation

### Production Deployment
- **Clean Architecture**: Pure transport with no application contamination
- **High Performance**: Memory pooling, zero-copy, frame batching
- **Quantum Resistant**: Future-proof against quantum computing threats
- **Professional Quality**: Full test coverage and production validation

## Testing

### Core Tests
```bash
# All tests
cargo test

# Extension-specific tests
cargo test extensions --lib

# FALCON cryptography tests
cargo test falcon --lib

# Transport layer tests
cargo test transport --lib
```

### Test Coverage
- **Extensions**: Tokenization, sharding, routing, seeding
- **FALCON**: Key generation, signing, verification, transport integration
- **Transport**: QUIC connectivity, certificate management, performance
- **Integration**: Cross-component functionality and error handling

---

**STOQ Protocol**: Production-ready pure QUIC transport with quantum-resistant security and protocol extensions.

*Status: ✅ Production Ready*
*Version: 1.0.0*
*Last Updated: January 2025*