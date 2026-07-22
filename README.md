# HyperMesh Core

A sovereign distributed computing platform with post-quantum cryptography, Block-MATRIX topology, and bilateral Proof of State authentication.

**12 workspace crates** (+ UI) | **1,035 Rust files** | **361,784 lines** | **4,553 tests** | **0 compiler errors**

---

## Run a Node

```bash
# Build (requires Rust + clang + lld)
git clone https://github.com/hypermesh-online/hypermesh.git
cd hypermesh/core
cargo build --release --target x86_64-unknown-linux-musl -p blockmatrix --bin hypermesh

# Run — joins the public test network via trust.hypermesh.online
./target/x86_64-unknown-linux-musl/release/hypermesh \
  --name yourname \
  --privacy public \
  --network-id trustnet-test \
  --stoq-port 9292 \
  --bootstrap "[::ffff:35.208.78.211]:9292" \
  connect public --foreground
```

Your node creates its own blockchain, assesses hardware, generates a FALCON-1024 identity, connects via STOQ/QUIC, and completes a bilateral Proof of State handshake — no external dependencies required.

## What You Can Do

```bash
# Store a file (Compress → Encrypt → Shard → Distribute)
hypermesh store myfile.pdf

# Share with a peer by name
hypermesh share send abc123 --with alice

# Check your inbox for shared files
hypermesh share inbox

# Accept a shared file
hypermesh share accept inv-001

# Fetch a shared file
hypermesh fetch abc123 -o downloaded.pdf

# Send an encrypted message
hypermesh message send --to alice "Hey, check out the file I shared"

# Read your messages
hypermesh message inbox

# Register a DNS name
hypermesh dns register myname ::1

# Resolve a peer
hypermesh dns resolve alice

# Ping a peer (bilateral PoS handshake RTT)
hypermesh ping alice

# View node status
hypermesh status
```

## Build Prerequisites

Requires **clang + lld** (not gcc). Configured in `.cargo/config.toml`.

| Required | Purpose |
|----------|---------|
| Rust (stable) | `rustup` — compiler toolchain |
| clang | C compiler + linker |
| lld | LLVM linker (`-fuse-ld=lld`) |
| pkg-config | Native dependency discovery |

**Not required**: gcc, openssl/libssl (all crypto is pure Rust), cmake, Docker.

<details>
<summary>Install commands by platform</summary>

**Ubuntu / Debian**: `sudo apt install clang lld pkg-config`

**Fedora / RHEL**: `sudo dnf install clang lld pkg-config`

**Arch Linux**: `sudo pacman -S clang lld pkgconf`

**macOS**: `xcode-select --install && brew install llvm pkg-config`

For musl static builds (deployment): `rustup target add x86_64-unknown-linux-musl`

</details>

---

## Architecture

### Crate Overview

| Crate | Description |
|-------|-------------|
| [hypermesh-lib](lib/) | Shared canonical types — single source of truth for all crates |
| [stoq](stoq/) | QUIC/IPv6 transport with eBPF acceleration, post-quantum key exchange |
| [hypermesh-ebpf](hypermesh-ebpf/) | Kernel-level XDP + AF_XDP zero-copy packet processing |
| [trustchain](trustchain/) | FALCON-1024 CA, threshold Shamir SSS, federation, OCSP/CRL |
| [blockmatrix](blockmatrix/) | Block-MATRIX node — topology, assets, pipeline, blockchain, IPC daemon |
| [catalog](catalog/) | Asset type registry with DHT, type publishing, STOQ API |
| [caesar](caesar/) | Gold-denominated Ephemeral Value Protocol |
| [caesar-sdk](caesar-sdk/) | UPI adapter traits for Caesar payment rails |

> Banking provider integrations (Stripe/Plaid/Square) are not yet live; the caesar-sdk UPI adapter trait is the integration point.
| [gateway](gateway/) | HTTP/3 + STOQ gateway, DNS-over-HTTPS, domain routing |
| [ngauge](ngauge/) | Analytics, capacity metrics, routing intelligence, marketplace |
| [hypermesh-sdk](hypermesh-sdk/) | Typed Rust SDK for daemon IPC API |
| [hypermesh-ffi](hypermesh-ffi/) | C FFI bindings for cross-language integration |
| [ui](ui/) | React 19 dashboard — 55 connected components, zero mock data |

Language SDKs: [TypeScript](sdk/typescript/) | [Python](sdk/python/) | [Go](sdk/go/) | [C#](sdk/csharp/) | [C++](sdk/cpp/)

### Key Concepts

**Proof of State** — Every asset requires four proofs: PoSpace (WHERE), PoStake (WHO), PoWork (WHAT), PoTime (WHEN). Bilateral verification — no voting, no quorum, no leader election. Something's either authentic or it isn't.

**Block-MATRIX Topology** — Each node is a cell in a 3D geospatial matrix (x,y,z). Tensor operations drive routing, shard placement, and resource allocation.

**Every Node = Own Blockchain** — Starts on boot with a unique genesis block. No network required. Network participation is optional (Device scope = local, Network scope = synced).

**Privacy Model** — Two independent dimensions:
- **PrivacyMode** (transport): Anonymous | Private | Public
- **BlockchainScope** (state): Device | Network
- Any combination is valid. Device + Anonymous = fully isolated. Network + Public = open ledger.

**Asset Pipeline** — Compress → Encrypt → Shard → Distribute:
- Compression: Zstd (large/binary), Brotli (small text), auto-skip (video/audio)
- Encryption: Kyber-1024 KEM → BLAKE3-HKDF per-segment → AES-256-GCM
- Sharding: Reed-Solomon adaptive erasure coding
- Segment-oriented: supports torrent, streaming, and random-access byte range

**Torrent-Model Propagation** — Blocks propagate to content-interested peers only:
- Device scope: no propagation
- Private: full replication to all peers
- Public: reflectors + spatial neighbors + active consumers
- Anonymous: consumers only

**Post-Quantum Cryptography**:
| Algorithm | Purpose |
|-----------|---------|
| FALCON-1024 | Signing (identity, handshake, share invites, messages) |
| Kyber-1024 | Encryption (assets, messages, key wrapping) |
| X25519MLKEM768 | QUIC key exchange (hybrid post-quantum) |
| BLAKE3 | All content hashing |

### P2P File Sharing

Files are encrypted with Kyber-1024, sharded with Reed-Solomon, and distributed across the mesh. Share invitations are FALCON-1024 signed envelopes containing the shard map + a Kyber-wrapped decryption key — only the intended recipient can decrypt.

### Private Messaging

Messages are Kyber-1024 encrypted for the recipient, FALCON-1024 signed by the sender, and registered as blockchain assets. Threading via `reply_to`. All through CLI (`hypermesh message`) or the web dashboard.

### Distributed CA

TrustChain CA with Shamir Secret Sharing (3-of-5 threshold) for FALCON-1024 key splitting. Federation peers hold key shares. CRL/OCSP propagation across federated CAs.

### Intelligence Loop

ngauge monitors network health and feeds back into routing:
- MetricsReporter → ngauge ingestion (every 30s)
- SwarmDemandTracker → SwarmAnalytics (every 10s)
- RoutingIntelligence → PropagationWeights (every 15s)
- ReplicationTrigger checks against live analytics
- 9-test simulation harness (10-50 nodes)

---

## Deployment

### Static Binary (musl)

```bash
cargo build --release --target x86_64-unknown-linux-musl -p blockmatrix --bin hypermesh
# Produces ~11MB static-pie ELF with zero runtime dependencies
```

### Deploy to Server

```bash
scp target/x86_64-unknown-linux-musl/release/hypermesh user@server:/usr/local/bin/
# Copy systemd unit
scp systemd/blockmatrix.service user@server:/etc/systemd/system/
sudo systemctl enable --now blockmatrix
```

### trust.hypermesh.online

The public bootstrap node at `35.208.78.211:9292` (GCP, Debian 12). Runs as a reflector with `--network-id trustnet-test`.

---

## Project Status

> Source of truth: each crate's `crate-status.toml`. Auto-synced via `./scripts/sync-status.sh`.

### What Works

- **P2P file sharing** with post-quantum encrypted invitations
- **Private messaging** (Kyber encrypted, FALCON signed, threaded)
- **Streaming asset pipeline** (segment-oriented, torrent/streaming/random-access)
- **Torrent-model block propagation** (content-interested peers only)
- **Intelligence loop** (ngauge → routing weights → replication triggers)
- **Cross-network asset transfers** (blockchain lock/register/release entries)
- **Threshold CA** (Shamir SSS for FALCON-1024 key splitting)
- **Distributed CA** (federation key shares, CRL/OCSP propagation)
- **Identity distribution** (key rotation via blockchain, split-brain detection)
- **Browser namespace** (DNS-over-HTTPS, Host→DNS resolution, dynamic certs)
- **Catalog type publishing** (Message, Invitation, Document types with JSON Schema)
- **Web dashboard** (55 connected components, zero mock data, all real IPC hooks)
- **Network sync** (block propagation, gossip, cross-genesis, E2E verified on trust.hypermesh.online)
- **5 language SDKs** (TypeScript, Python, Go, C#, C++) + C FFI bindings

### Remaining Work

- CI/CD pipelines
- ARM cross-compilation (aarch64-unknown-linux-musl)
- Docker image + install script
- Storybook stories + Vitest unit tests for UI
- Multi-node E2E integration test suite
- Production security audit

---

## Contributing

- Files < 500 lines, functions < 50 lines, nesting < 3 levels
- No `.unwrap()` or `panic!()` in production code (enforced by pre-commit hook)
- All shared types go in `hypermesh-lib` — no duplicate type definitions
- IPv6-only networking throughout
- BLAKE3 for all hashing (no SHA-256 except X.509 fingerprints)
- Update `crate-status.toml` when feature status changes, then run `./scripts/sync-status.sh`

See [CONTRIBUTING.md](CONTRIBUTING.md) for full guidelines.

## License

Business Source License 1.1 — See [LICENSE](LICENSE)
