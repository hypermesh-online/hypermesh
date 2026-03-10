# STOQ Protocol

Intelligent QUIC-based transport protocol with protocol-level validation, eBPF acceleration, and matrix-aware routing for the HyperMesh ecosystem.

**Status**: 100% Complete (alpha) | 60 files | ~17,900 lines | 145 tests

STOQ is not a simple QUIC wrapper. It provides protocol-level intelligence: Proof of State token validation, asset hash verification (BLAKE3), matrix shard addressing, privacy enforcement, and tensor-aware routing -- all at the transport layer.

## Architecture

**STOQ provides MECHANISMS. HyperMesh provides POLICIES. eBPF enforces at kernel level.**

### Intelligence Layer
- **PoS Token Validation**: Two-stage fast/full validation at protocol level, privacy-mode-aware, cached, rate-limited
- **Asset Hash Verification**: BLAKE3 content integrity checks at protocol layer
- **Matrix Shard Addressing**: x,y,z coordinates for Block-MATRIX shard placement
- **Privacy Enforcement**: Different protocol behavior per PrivacyMode (Anonymous/Private/Public)
- **Tensor-Aware Routing**: Smart routing based on matrix topology and distance calculations

### Transport
- **Protocol**: QUIC over IPv6 (quinn-based) with intelligence extensions
- **Crypto**: FALCON-1024 post-quantum signing (handshake + protocol signatures)
- **Adaptive Tiers**: EWMA bandwidth estimation, MTU discovery, loss-based congestion control selection
- **Multi-Path QUIC**: 4 schedulers (scope/privacy/federation policy, bandwidth-weighted, redundant mode)
- **Reflector Pool**: Heartbeat/health tracking, quorum detection, sync protocol, MatrixMessage bridge
- **eBPF Acceleration**: AF_XDP zero-copy I/O via `StoqEbpfTransport` thin consumer wrapper

### Certificate Management (Two Modes)
- **Anonymous**: Ephemeral per-connection self-signed certs (no CA, no CT)
- **Authenticated**: TrustChain-issued certs with configurable endpoint (P2P: `local://trustchain`, Federated: `quic://{gateway}`, Public: `quic://trust.hypermesh.online`)

### Privacy Model

PrivacyMode is a struct with 3 presets -- transport layer is independent from blockchain state replication:

| PrivacyMode | Tracking | Routing | Use Case |
|-------------|----------|---------|----------|
| **Anonymous** | Untracked | Randomized through matrix | Maximum privacy |
| **Private** | Tracked | Direct tensor routing within trusted regions | Bounded groups |
| **Public** | Tracked | Full matrix visibility, optimal global routing | Maximum CAESAR rewards |

**Privacy Flexibility**: Any PrivacyMode can carry any BlockchainScope (Device or Network). Examples:
- Device chain + Anonymous transport = fully isolated, untraceable node
- Network chain + Anonymous transport = synced swarm, untraceable packets
- Network chain + Public transport = open synced ledger, full transparency

## Quick Start

```bash
cargo build -p stoq --release
cargo test -p stoq
```

## Protocol Extensions
- **PoS Tokenization**: BLAKE3 cryptographic validation with Proof of State token verification
- **Matrix Sharding**: Fragmentation/reassembly with matrix coordinate addressing (x,y,z)
- **Tensor Routing**: Multi-hop routing with matrix topology awareness
- **Engauge METRICS Frame**: Custom frame type (0xfe000007) for streaming MetricsFrame payloads (feature-gated)

## Recent Changes
- **Local FALCON-1024 PoS verification**: PoS validator verifies FALCON-1024 signatures locally (no TrustChain client needed)
- **E2E bilateral handshake**: Verified end-to-end between two real nodes

## Security
- TLS 1.3 with QUIC integration
- Certificate-based authentication via TrustChain
- FALCON-1024 digital signatures (256-bit equivalent quantum resistance)
- 0-RTT replay attack protection (disabled by default)
- eBPF validation hooks (CertificateValidator + PacketValidator registered with hypermesh-ebpf orchestrator)
- BLAKE3 packet hashing and asset hash verification

## License

Business Source License 1.1
