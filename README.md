# HyperMesh Core

A sovereign distributed computing platform built on a six-layer stack with post-quantum cryptography, 3D matrix topology, and bilateral Proof of State authentication.

**9 crates** | **756 .rs files** | **256,159 lines** | **948 tests** | **0 compiler errors**

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full system design.

---

## Quick Start

```bash
# Clone
git clone https://github.com/hypermesh-online/hypermesh.git
cd hypermesh

# Install dependencies (Ubuntu)
sudo apt install clang lld pkg-config

# Build
cargo check --workspace

# Test
cargo test --workspace
```

## Build Prerequisites

Requires **clang + lld** (not gcc). Configured in `.cargo/config.toml`.

| Required | Purpose |
|----------|---------|
| Rust (stable) | `rustup` — compiler toolchain |
| clang | C compiler + linker |
| lld | LLVM linker (`-fuse-ld=lld`) |
| pkg-config | Native dependency discovery |

| Optional (eBPF) | Purpose |
|------------------|---------|
| libbpf-dev | eBPF program loading (feature-gated) |
| linux-headers | Kernel headers for eBPF |
| llvm | eBPF bytecode compilation |

**Not required**: gcc, openssl/libssl (all crypto is pure Rust), cmake.

<details>
<summary>Install commands by platform</summary>

**Ubuntu / Debian**: `sudo apt install clang lld pkg-config`

**Fedora / RHEL**: `sudo dnf install clang lld pkg-config`

**Arch Linux**: `sudo pacman -S clang lld pkgconf`

**macOS**: `xcode-select --install && brew install llvm pkg-config`

</details>

---

## Crate Overview

| Layer | Crate | Lines | Tests | Phase | Description |
|-------|-------|------:|------:|-------|-------------|
| — | [hypermesh-lib](lib/) | 235 | 0 | alpha | Shared canonical types (single source of truth) |
| 1 | [stoq](stoq/) | 14,039 | 65 | alpha | QUIC/IPv6 transport with eBPF and FALCON-1024 |
| — | [hypermesh-ebpf](hypermesh-ebpf/) | 1,904 | 19 | alpha | Kernel-level packet validation (eBPF) |
| 2 | [trustchain](trustchain/) | 30,027 | 63 | alpha | Certificate Authority, identity, CT logs |
| 3 | [blockmatrix](blockmatrix/) | 129,696 | 624 | alpha | 3D topology, assets, pipeline, blockchain |
| 4 | [catalog](catalog/) | 25,794 | 41 | alpha | Package registry and asset discovery |
| 5 | [caesar](caesar/) | 5,654 | 4 | planning | Payment bridge (optional) |
| — | [gateway](gateway/) | 1,438 | 0 | planning | HTTP/3 entry point |

---

## Project Status

> Source of truth: each crate's `crate-status.toml` file. Auto-synced to `scripts/output/status.ts` via `scripts/sync-status.sh`.

### hypermesh-lib — Shared Types (alpha)

| Done | In Progress | TODO |
|------|-------------|------|
| NodeId, AssetId, NetworkId, ContentHash newtypes | Cross-crate validation helpers | Runtime state unification (all network execution and on-chain ops use Asset typedefs/impls) |
| PrivacyMode struct (2-axis: AccessScope + tracked) | BlockMatrix/TrustChain migration to canonical asset types | Canonical consensus proof types |
| BlockchainScope enum (Device \| Network) | | Shared serialization formats |
| ProofType enum (PoSpace/PoStake/PoWork/PoTime) | | Common test utilities |
| MatrixPosition coordinate type | | Public SDK types for third-party integration |
| PipelineStage, CryptoAlgorithm enums | | |
| HypermeshError unified error type | | |
| Three-pillar asset system (AssetKind + BaseState/AssetStatusTrait + AssetAdapter) | | |

### STOQ — Transport Protocol (alpha)

| Done | In Progress | TODO |
|------|-------------|------|
| QUIC transport over IPv6 | | Protocol-level PoS token validation at line rate |
| Connection pool with health checks | | Adaptive transport tiers |
| FALCON-1024 key generation and signing | | Multi-path QUIC |
| Certificate management (4 strategies) | | |
| Network isolation (PrivacyMode: Anonymous/Private/Public) | | |
| Adaptive congestion control | | |
| Transport metrics collection | | |
| Protocol extension framework (packets/tokens/shards) | | |
| eBPF transport integration with validation hooks | | |
| AF_XDP zero-copy UMEM I/O (kernel-backed) | | |
| PoS validation results fed to eBPF layer | | |
| FalconTrustChainClient with real FALCON-1024 verification | | |
| Tunnel traffic type enforcement | | |

### TrustChain — Identity & Certificates (alpha)

| Done | In Progress | TODO |
|------|-------------|------|
| Certificate Authority with issuance and revocation | Production certificate hardening | HSM key storage integration |
| FALCON-1024 post-quantum signing | CT log federation across nodes | Cross-network CA federation |
| Kyber-1024 key encapsulation | HTTP/3 server integration | Automated certificate rotation |
| Certificate Transparency (Merkle logs, SCTs) | Deployment quality gates | |
| Proof of State four-proof consensus | | |
| Security monitoring and Byzantine detection | | |
| DNS resolver with STOQ transport | | |
| STOQ-based API server | | |

### BlockMatrix — Topology, Assets & Coordination (alpha)

| Done | In Progress | TODO |
|------|-------------|------|
| 3D coordinate system (x,y,z positioning) | Asset pipeline reorder (Compress-Encrypt-Shard) | Network sync + reflector pooling |
| Tensor math library (Vector3D, Matrix3x3, A*) | Kyber-1024 for asset encryption | Container runtime with isolation |
| Every-node blockchain (independent chains) | Instruction-based retrieval system | Multi-node production deployment |
| Geospatial module (GPS conversion, clustering) | OS integration layer (Linux/macOS/Windows) | Dynamic shard rebalancing |
| Asset adapters (CPU/GPU/Memory/Storage/Network/Container) | | |
| Privacy allocation (5 levels, 4 tiers) | | |
| Matrix persistence (WAL, snapshots, recovery) | | |
| Proof of State four-proof validation | | |

### Catalog — Package Registry (alpha)

| Done | In Progress | TODO |
|------|-------------|------|
| Asset package types and metadata | STOQ transport for distribution | Full asset marketplace |
| Asset registry with publish/install/search | Content-addressed storage (DHT) | Peer-to-peer package sharing |
| Template generation framework | TrustChain security integration | Consensus proof validation for packages |
| Asset validation pipeline | | Asset SDK for third-party developers |
| Semantic versioning and dependency resolution | | Asset transaction integration with Caesar |
| HyperMesh execution delegation | | catalog.hypermesh.online clearnet registry access |
| Scripting engine (syntax validation) | | |
| Canonical asset type integration with lib↔blockmatrix compat layer | | |

### Caesar — Payment Bridge (planning)

| Done | In Progress | TODO |
|------|-------------|------|
| Token economics configuration | Banking provider STOQ migration | Live multi-chain bridge (BTC/ETH/SOL) |
| Wallet creation and balance tracking | Analytics engine integration | Fiat payment processing |
| Transaction processing engine | STOQ API server endpoints | Reward distribution system |
| Reward calculation framework | | Gold peg stabilization mechanism |
| Staking manager with APY | | Actual stake storage implementation |
| Exchange rate engine | | Balance lookup service (actual implementation) |
| Cross-chain bridge types (8 networks) | | |

### HyperMesh eBPF — Kernel Integration (alpha)

| Done | In Progress | TODO |
|------|-------------|------|
| Unified HyperMeshEbpf orchestrator API | | Kernel-space PoS validation at line rate |
| XDP program management and packet filtering | | Hardware offload support (smart NICs) |
| AF_XDP zero-copy UMEM I/O (4-ring buffers, sendto/poll) | | Multi-queue AF_XDP load balancing |
| eBPF program compiler and kernel loader | | |
| Validation hooks (certificate, packet, extension) | | |
| PoS header parsing and enhanced validation | | |
| Asset hash verification (BLAKE3) | | |
| Matrix routing path validation | | |
| Policy map management with BPF map sync | | |
| Privacy tier eBPF enforcement | | |
| C kernel XDP program (HyperMesh extension headers) | | |
| Unified intelligence + transport metrics | | |
| AF_XDP frame allocator with batch operations | | |

### Gateway — HTTP/3 Entry Point (planning)

| Done | In Progress | TODO |
|------|-------------|------|
| QUIC/HTTP3 server setup (quinn + h3) | Request handling (h3 API fix needed) | Federated gateway mesh |
| TLS certificate loading (PEM/DER) | HTTP/3 backend proxying | STOQ protocol bridge |
| Connection pool with health checks | | Load balancing across backends |
| Router with path-based backend selection | | Rate limiting and DDoS protection |
| Circuit breaker and retry logic | | |
| CORS middleware and request logging | | |

---

## TODO Summary

| Category | Count |
|----------|------:|
| Done | 57 |
| In Progress | 18 |
| TODO | 26 |

**Priority TODOs** (cross-cutting):
- [ ] Network scope sync + reflector pooling (Device ↔ Network synchronization)
- [ ] Asset pipeline reorder (Compression → Encryption → Sharding → Distribution)
- [ ] Protocol-level PoS token validation (STOQ + eBPF)
- [ ] Multi-node production deployment
- [ ] STOQ adaptive transport tiers
- [ ] Kernel eBPF program loading
- [ ] HSM key storage for TrustChain
- [ ] Full asset marketplace (Catalog)
- [ ] Live multi-chain bridge (Caesar)
- [ ] Runtime state unification (all network execution uses Asset typedefs/impls)

---

## Key Concepts

**Proof of State**: Every asset requires four proofs — PoSpace (WHERE), PoStake (WHO), PoWork (WHAT/HOW), PoTime (WHEN). Bilateral verification, not global consensus.

**Every Node = Own Blockchain**: Starts on boot with unique genesis block. No network required. Network participation is optional.

**Privacy Independence**: Network tiers (Anonymous/P2P/Federated/Public) and blockchain scopes (Device/User/Group/Org/Federation/Public) are independent dimensions. Any combination is valid.

**Instruction-Based Retrieval**: Send shard map instructions, not raw data. Receiver fetches shards from nearest matrix nodes and reconstructs locally.

**Distribution Pipeline**: Compression (Brotli) → Encryption (Kyber-1024) → Sharding (Reed-Solomon) → Placement (tensor-based).

**Three-Pillar Asset System**: Every asset has a Kind (two-level classification: system or user-defined), a Status (programmable state machine where domain states map to infrastructure BaseState), and an Adapter (fully programmable runtime interface with lifecycle hooks, command/query dispatch, and self-describing capabilities). Defined canonically in hypermesh-lib, used by all crates.

---

## Contributing

- Files < 500 lines, functions < 50 lines, nesting < 3 levels
- No `.unwrap()` or `panic!()` in production code (enforced by pre-commit hook)
- All shared types go in `hypermesh-lib` — no duplicate type definitions
- IPv6-only networking throughout
- Update `crate-status.toml` when feature status changes, then run `./scripts/sync-status.sh`

## License

MIT — See [LICENSE](LICENSE)
