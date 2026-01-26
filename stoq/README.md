# STOQ Protocol - Intelligent Protocol with Protocol-Level Validation

**Status: 🚧 DEVELOPMENT - Intelligence Layer + Transport (Phase 2 Active)**

STOQ is an intelligent protocol that combines QUIC over IPv6 transport with protocol-level validation and matrix-aware routing. Unlike pure transport protocols, STOQ validates Proof of State tokens, verifies asset hashes, provides matrix shard addressing, and enforces privacy tiers at the protocol layer. Features adaptive network tier detection, FALCON-1024 quantum-resistant cryptography, and tensor-aware routing for Block-MATRIX integration.

## ⚡ Architecture Principle

**STOQ is an intelligent protocol with built-in validation** - it provides protocol-level PoS validation, matrix shard addressing, privacy tier enforcement, and tensor-aware routing. Applications (like HyperMesh) leverage STOQ's intelligence layer for validated, privacy-aware, matrix-integrated communication.

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

### Intelligence Layer Features
- **PoS Token Validation**: Protocol-level validation of Proof of State tokens
- **Asset Hash Verification**: Content integrity checks at protocol layer
- **Matrix Shard Addressing**: Provides x,y,z coordinates for Block-MATRIX shard placement
- **Privacy Tier Enforcement**: Different protocol behavior for Anonymous/Private/Federated/Public tiers
- **Tensor-Aware Routing**: Smart routing decisions based on matrix topology and distance calculations

### Core Transport Features
- **Protocol**: QUIC over IPv6 (quinn-based implementation) with intelligence extensions
- **Security**: FALCON-1024 post-quantum cryptography (fully implemented)
- **Adaptive Tiers**: Network performance detection and configuration adaptation
- **Memory Safety**: Eliminated unsafe operations, secure memory management
- **DoS Protection**: Connection limits and 0-RTT replay attack mitigation

### Protocol Extensions Framework
- **PoS Tokenization**: SHA-256 cryptographic validation with Proof of State token verification
- **Matrix Sharding**: Fragmentation/reassembly with matrix coordinate addressing (x,y,z)
- **Tensor Routing**: Multi-hop routing with matrix topology awareness and distance optimization
- **Privacy Extensions**: Protocol behavior adaptation based on privacy tier requirements
- **Extension Integration**: Intelligence layer actively integrated with transport

### Quantum-Resistant Security
- **FALCON-1024**: NIST Post-Quantum Cryptography standard
- **Key Management**: Automatic key generation and rotation
- **Transport Integration**: Handshake-level quantum resistance
- **Security Level**: 256-bit equivalent quantum security

## 🌐 Matrix Integration

STOQ provides deep integration with the Block-MATRIX topology system:

### Matrix-Aware Features
- **Shard Addressing**: Every packet includes matrix coordinates (x,y,z) for optimal shard placement
- **Tensor Routing**: Routes packets based on matrix topology, minimizing hop distance
- **Distance Calculations**: Optimizes paths using Euclidean distance in tensor space
- **Topology Awareness**: Understands matrix structure for efficient data distribution

### Privacy Tiers & Matrix Behavior

**CRITICAL**: Network transport layer is INDEPENDENT from blockchain consensus layer

- **Anonymous**: No coordinate tracking, randomized routing through matrix
  - Can carry private blockchain traffic for maximum security
  - Untraceable packets + encrypted consensus = complete privacy
- **Private**: Direct tensor routing within trusted matrix regions
  - P2P connections between known peers
  - Can carry any blockchain type (private/public/federated)
- **Federated**: Cross-region routing with federation-aware path selection
  - Organization-level networks with controlled membership
  - Supports multi-blockchain participation
- **Public**: Full matrix visibility for optimal global routing
  - Bootstrap via `trust.hypermesh.online` gateway
  - Full transparency for maximum CAESAR rewards

**Privacy Flexibility Examples**:
- Private blockchain + Anonymous network = Maximum security
- Public blockchain + Private network = Open ledger, controlled access
- User's device network + Anonymous transport = Personal cloud with complete privacy

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

### Intelligence Layer Usage
```rust
// Use intelligence extensions
let extensions = DefaultStoqExtensions::new();

// Validate PoS token at protocol level
let pos_token = extensions.validate_pos_token(data)?;

// Create matrix-aware shard with coordinates
let shards = extensions.shard_with_matrix(data, 1024, (x, y, z))?;
let reassembled = extensions.reassemble_from_matrix(shards)?;

// Create intelligent packet with validation
let mut packet = StoqPacket::new(data.into());
packet.pos_token = Some(pos_token);
packet.matrix_coords = Some((x, y, z));
packet.privacy_tier = PrivacyTier::Federated;

// Asset hash verification
packet.asset_hash = extensions.compute_asset_hash(data);
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
- **Intelligence Layer**: Protocol-level PoS validation and matrix integration ✅
- **Quantum Security**: FALCON-1024 cryptography fully implemented ✅
- **Matrix Awareness**: Tensor routing and shard addressing active ✅
- **Privacy Enforcement**: Tier-based protocol behavior implemented ✅
- **Adaptive Networks**: Tier detection and configuration adaptation ✅
- **Memory Safety**: Unsafe operations eliminated, secure by design ✅

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

### Protocol Intelligence Security
- PoS token validation at protocol layer
- Asset hash verification for content integrity
- Matrix coordinate authentication
- Privacy tier enforcement mechanisms
- SHA-256 packet tokenization with PoS integration
- Cryptographic shard verification with matrix addressing
- Tensor-aware hop chain integrity validation

## 🔗 Integration

STOQ provides an intelligent protocol layer for:
- HyperMesh distributed computing with validated asset transfers
- Block-MATRIX tensor topology with coordinate-based routing
- TrustChain certificate authorities with PoS validation
- Privacy-aware networked applications with tier enforcement
- High-performance matrix-integrated systems
- Quantum-resistant communication with protocol-level intelligence

## 📄 License

MIT OR Apache-2.0

---

*STOQ: Intelligent protocol with matrix integration, PoS validation, and quantum resistance - Professional, validated, production-ready.*