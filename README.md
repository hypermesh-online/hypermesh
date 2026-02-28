# HyperMesh Core

A sovereign distributed computing platform with post-quantum cryptography, Block-MATRIX topology, and bilateral Proof of State authentication.

**11 crates** | **999 files** | **289,405 lines** | **1,885 tests** | **0 compiler errors**

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full system design.

---

## Quick Start

```bash
# Clone
git clone https://github.com/hypermesh-online/core.git
cd core

# Install dependencies (Ubuntu)
sudo apt install clang lld pkg-config

# Build
cargo check --workspace
cargo test --workspace
cargo build --workspace --release
```

## Build Prerequisites

Requires **clang + lld** (not gcc). Configured in `.cargo/config.toml`.

| Required | Purpose |
|----------|---------|
| Rust (stable) | `rustup` -- compiler toolchain |
| clang | C compiler + linker |
| lld | LLVM linker (`-fuse-ld=lld`) |
| pkg-config | Native dependency discovery |

| Optional (eBPF) | Purpose |
|------------------|---------|
| libbpf-dev | eBPF program loading (feature-gated) |
| linux-headers | Kernel headers for eBPF |
| llvm | eBPF bytecode compilation |

**Not required**: gcc, openssl/libssl (all crypto is pure Rust), cmake, Docker.

<details>
<summary>Install commands by platform</summary>

**Ubuntu / Debian**: `sudo apt install clang lld pkg-config`

**Fedora / RHEL**: `sudo dnf install clang lld pkg-config`

**Arch Linux**: `sudo pacman -S clang lld pkgconf`

**macOS**: `xcode-select --install && brew install llvm pkg-config`

</details>

---

## Crate Overview

| Crate | Lines | Tests | Completion | Description |
|-------|------:|------:|-----------:|-------------|
| [hypermesh-lib](lib/) | 3,198 | 97 | 68% | Shared canonical types -- single source of truth |
| [stoq](stoq/) | 17,897 | 145 | 100% | QUIC/IPv6 transport with eBPF and FALCON-1024 |
| [hypermesh-ebpf](hypermesh-ebpf/) | 8,203 | 152 | 100% | Kernel-level eBPF packet processing (XDP + AF_XDP) |
| [trustchain](trustchain/) | 33,923 | 95 | 100% | FALCON-1024 Certificate Authority, identity, CT logs |
| [blockmatrix](blockmatrix/) | 128,404 | 821 | 97% | Block-MATRIX node -- topology, assets, pipeline, blockchain |
| [catalog](catalog/) | 27,932 | 52 | 100% | Asset package registry with DHT distribution |
| [caesar](caesar/) | 12,753 | 220 | 94% | Gold-denominated Ephemeral Value Protocol |
| [caesar-sdk](caesar-sdk/) | 1,039 | 2 | 83% | UPI adapter traits for Caesar payment rails |
| [gateway](gateway/) | 7,028 | 155 | 100% | HTTP/3 + STOQ gateway (4 roles, *.hypermesh.online) |
| [engauge](engauge/) | 5,758 | 135 | 100% | Analytics, capacity metrics, and streaming |
| [ui](ui/) | 43,270 | 11 | 44% | TypeScript/React dashboard UI |

---

## Installation

HyperMesh runs as system-level daemons. See `systemd/` for service units.

---

## Key Concepts

### Proof of State

Every asset requires four proofs -- PoSpace (WHERE), PoStake (WHO), PoWork (WHAT/HOW), PoTime (WHEN). Bilateral verification, not global consensus.

### Block-MATRIX Topology

Each node is a cell in a 3D geospatial matrix (x, y, z coordinates). Tensor operations drive routing, resource allocation, and shard placement. Nodes discover neighbors through matrix adjacency.

### Every Node = Own Blockchain

Starts on boot with a unique genesis block. No network connectivity required. Network participation is optional.

### Privacy Model (Two Independent Dimensions)

**PrivacyMode** (transport layer via STOQ) -- a struct with three presets:
- **Anonymous**: Unbounded, untracked
- **Private**: Bounded, tracked
- **Public**: Unbounded, tracked

**BlockchainScope** (consensus layer) -- binary:
- **Device**: Local-only chain, always running from boot
- **Network**: Synchronized across nodes via reflector pooling

These are independent dimensions. Any combination is valid: Device + Anonymous, Network + Private, etc.

### Asset Pipeline

**Compress -> Encrypt -> Shard -> Distribute** (exact order):
1. **Brotli** streaming compression (levels 1-11)
2. **Kyber-1024** quantum-resistant encryption (KEM + AES-GCM, whole-blob)
3. **Reed-Solomon** erasure coding (10+4 shards)
4. **Tensor-based** placement at calculated matrix positions

### Instruction-Based Retrieval

Nodes send shard map instructions (under 1 KB), not raw data. The receiver queries matrix positions, fetches shards from nearest nodes, and reconstructs locally.

### IPv6 Asset Addressing

Every asset gets a unique IPv6 address (`fd48:4d00` prefix) encoding matrix coordinates, content fingerprint, and shard sub-addressing. The TransferEngine handles PoS-authenticated transfers with blockchain receipts.

### Cryptography

| Algorithm | Purpose |
|-----------|---------|
| FALCON-1024 | Protocol signing (TrustChain CA, STOQ handshake) |
| Kyber-1024 | Asset encryption (KEM + AES-GCM) |
| BLAKE3 | All content hashing |

SHA-256 is used only for X.509 certificate fingerprints (standard requirement) and OCI image digests.

### Three-Pillar Asset System

Every asset has a **Kind** (two-level classification: system or user-defined), a **Status** (programmable state machine where domain states map to BaseState), and an **Adapter** (runtime interface with lifecycle hooks, command/query dispatch, and self-describing capabilities). Defined canonically in hypermesh-lib, used by all crates.

---

## Project Status

> Source of truth: each crate's `crate-status.toml` file.
> Auto-synced to `scripts/output/status.ts` via `scripts/sync-status.sh`.

### What Works

- **STOQ**: QUIC transport, FALCON-1024 signing, adaptive tiers, multi-path QUIC, reflector pools
- **TrustChain**: Full CA with issuance/revocation, CT logs, OCSP, CRL, threshold crypto, federation
- **BlockMatrix**: Asset adapters (6 types), asset pipeline, instruction-based retrieval, IPv6 addressing, network sync, gateway architecture, cross-scope routing, shard rebalancing, CLI, container runtime, cross-platform OS abstraction
- **Catalog**: Full package registry with DHT, P2P sharing, TrustChain security, Caesar rewards, STOQ API
- **Caesar**: EVP protocol with state machine, Governor PID controller, settlement, conservation law, gold oracle, STOQ API
- **Gateway**: 4 roles (bootstrap, inbound proxy, outbound proxy, inter-network), rate limiting, load balancing, federation bridge
- **engauge**: Receipts, capacity metrics, streaming protocol, differential privacy, routing intelligence, marketplace
- **eBPF**: XDP packet processing, AF_XDP zero-copy, BPF map policy sync, multi-queue load balancing, hardware offload detection

### Remaining Work

- Network scope blockchain sync (reflector/swarm mode for multi-node consensus)
- Cross-network asset transfers with dual proof of state
- Live multi-chain Caesar bridges (BTC/ETH/SOL)
- CI/CD pipelines and production deployment
- End-to-end integration testing across all crates
- UI completion (live data connections, native desktop via Tauri)

---

## Contributing

- Files < 500 lines, functions < 50 lines, nesting < 3 levels
- No `.unwrap()` or `panic!()` in production code (enforced by pre-commit hook)
- All shared types go in `hypermesh-lib` -- no duplicate type definitions
- IPv6-only networking throughout
- BLAKE3 for all hashing (no SHA-256 except X.509 fingerprints)
- Update `crate-status.toml` when feature status changes, then run `./scripts/sync-status.sh`

## License

MIT -- See [LICENSE](LICENSE)
