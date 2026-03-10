# TrustChain

FALCON-1024 Certificate Authority with Proof of State validation, binary authentication, and cross-network CA federation for the HyperMesh ecosystem.

**Status**: 95% Complete (alpha) | 91 files | ~33,900 lines | 95 tests

## Core Architecture

TrustChain implements the HyperMesh trust layer where every node is its own DNS provider first, with optional blockchain registration for CAESAR rewards. DNS names are blockchain assets requiring full Proof of State validation (PoSpace/PoStake/PoWork/PoTime).

### Binary Authentication

Authentication is binary -- pass or fail. There are no trust scores, reputation floats, or graduated levels.

- `BinaryAuthenticator` -- pass/fail with revocation list (replaced `TrustScorer`)
- `StateAuthenticator` trait -- renamed from `ConsensusValidator`
- `PublisherAuthenticator` -- binary publisher validation (no reputation scoring)

### Modules

| Module | Purpose |
|--------|---------|
| **CA** | Certificate Authority with issuance, revocation, `SecurityIntegratedCA` |
| **CT** | Certificate Transparency (RFC 6962 Merkle tree, SCTs, SignedTreeHead) |
| **DNS** | Resolver with STOQ transport, DNS-as-Asset |
| **Proof of State** | Four-proof validation (PoSpace/PoStake/PoWork/PoTime), BLAKE3 content hashing |
| **Security** | Binary authentication, Byzantine detection, monitoring |
| **Rotation** | Certificate rotation scheduler with background task |
| **OCSP** | Responder with FALCON-1024 signed responses |
| **CRL** | Generator and distributor |
| **Threshold** | Shamir Secret Sharing over GF(256), ThresholdSigner wraps FALCON-1024 key splitting |
| **Federation** | Cross-network CA federation (peer management, Full/Conditional/Untrusted trust levels) |
| **HTTP/3** | Server with 8 real endpoint handlers |

### Cryptography
- **FALCON-1024**: Post-quantum signing for all CA operations and protocol signatures
- **Kyber-1024**: Key encapsulation for asset encryption
- **BLAKE3**: Content hashing for Proof of State and certificate transparency

### Privacy Model

PrivacyMode is a struct with 3 presets (transport layer, independent from blockchain state replication):

| PrivacyMode | Validation | Certificate Mode |
|-------------|------------|-----------------|
| **Anonymous** | None | Ephemeral self-signed (no CA/CT) |
| **Private** | Peer attestation | TrustChain-issued (bounded group) |
| **Public** | Full Proof of State | TrustChain-issued (global CA) |

### Proof of State

Every DNS asset and certificate operation requires ALL FOUR proofs:
- **PoSpace (WHERE)**: Network location and topology position in matrix
- **PoStake (WHO)**: Identity, ownership, and economic stake
- **PoWork (WHAT)**: Computational contribution and service provision
- **PoTime (WHEN)**: Temporal ordering and registration timestamp

### Node Bootstrap
1. Create genesis block on boot (no network required)
2. Generate local identity (cryptographic keypair)
3. Initialize local DNS namespace
4. Establish matrix position
5. Node is fully functional for localhost operations
6. Optional: join public network via `trust.hypermesh.online`
7. Optional: register DNS on blockchain for CAESAR rewards

## Quick Start

```bash
cargo build -p trustchain --release
cargo test -p trustchain
```

## Recent Changes
- **Production binary hardening**: Config loading, graceful shutdown, signal handling
- **Recovery commitment**: HKDF-SHA512 + BLAKE3 commitment scheme implemented

## Integration

- **STOQ**: Certificate-based transport authentication, FALCON-1024 handshake signing
- **BlockMatrix**: Asset system integration, DNS-as-Asset registration
- **Caesar**: Economic incentives for DNS resolution services
- **Gateway**: Federation bridge between networks at `trust.hypermesh.online`

## License

Business Source License 1.1
