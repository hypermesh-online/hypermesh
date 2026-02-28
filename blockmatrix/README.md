# Block-MATRIX - Distributed Computing Node

**Status: Alpha | 380 files | ~128K lines | 821 tests | 97% feature-complete**

Block-MATRIX is the core node daemon for the HyperMesh ecosystem. Every device runs one. It implements a 3D mesh topology where each node is a cell at (x,y,z) coordinates, with tensor operations driving routing, resource allocation, and shard placement. Everything on the node is an Asset -- CPU, GPU, memory, storage, network interfaces, containers -- addressed via IPv6 and registered on a local blockchain that starts immediately on boot.

## Architecture

```
Node Boot
  |-> Genesis block (no network required)
  |-> Matrix position assigned (x,y,z)
  |-> Asset adapters register hardware
  |-> DNS namespace initialized (node-local)
  |-> Optional: join Network chain via reflector pooling
```

**Dual-scope blockchain**: Device chain runs always (local-only). Network chain syncs across participating nodes via reflector pooling. PrivacyMode (transport) is independent from BlockchainScope (consensus).

## Key Features

- **Block-MATRIX topology** -- 3D coordinate system, tensor math (Vector3D, Matrix3x3, A* pathfinding), geospatial GPS conversion and clustering
- **Universal asset system** -- 6 adapters (CPU, GPU, Memory, Storage, Network, Container) with privacy-aware allocation
- **Asset pipeline** -- Compress (Brotli) -> Encrypt (Kyber-1024 KEM) -> Shard (Reed-Solomon 10+4) -> Distribute (tensor-based placement)
- **Instruction-based retrieval** -- send shard maps (<1KB), not files; client assembly with 4 fallback strategies
- **IPv6 asset addressing** -- `fd48:4d00` prefix encoding matrix coords, content fingerprint, and shard sub-address
- **Transfer engine** -- PoS-authenticated asset transfers with blockchain receipts and state proofs
- **Remote proxy/NAT** -- IPv6-like addressing for resource access across nodes, trust-based proxy selection
- **Privacy allocation** -- Anonymous/Private/Public presets with per-resource user controls
- **Cross-scope routing** -- ScopeAwareRouter, gateway node selection, TransactionRouter with tensor dot-product pathfinding
- **Shard rebalancing** -- join/leave/failure triggers, octant diversity, cooldown throttling
- **Network sync** -- SyncManager, ReflectorPool, SyncDispatcher, BlockTransport trait
- **eBPF security** -- XDP attachment, policy enforcement, Privacy-eBPF bridge with sync_to_kernel
- **Cross-platform** -- OsAbstraction for Linux/macOS/BSD/Windows with hardware detection
- **BLAKE3** content hashing throughout, FALCON-1024 for protocol signing

## Modules

| Module | Purpose |
|--------|---------|
| `blockchain` | Device-scope chain, block production, validation |
| `consensus` | Proof of State orchestration (PoSpace/PoStake/PoWork/PoTime) |
| `assets` | Adapters, pipeline (compress/encrypt/shard/distribute), proxy, privacy |
| `matrix` | Tensor operations, geospatial, coordinate system |
| `transfer` | TransferEngine, IPv6 AssetAddress, state proofs |
| `verification` | PoSPing spatial verification, shard commitment |
| `retrieval` | Instruction-based retrieval, client assembly |
| `distribution` | Shard placement, rebalancing, PoS validation |
| `network` | Peer transport, blockchain integration |
| `gateway` | Cross-scope transfers, ScopeBridge lock/transfer/unlock |
| `persistence` | WAL, incremental snapshots, recovery |
| `container` | Process isolation, cluster management, health monitoring |
| `cli` | Topology queries, node management, asset operations |
| `dns` | Node-local DNS, blockchain asset registration |
| `privacy` | PrivacyMode enforcement, eBPF bridge |
| `security` | eBPF manager, syscall tracing, ExtensionValidator |
| `platform` | User contribution, resource sharing controls |
| `os_integration` | Cross-platform abstraction layer |
| `intelligence` | Routing intelligence integration |

## Quick Start

```bash
# Build
cargo build -p blockmatrix --release

# Run tests (use module filter -- full --lib hangs)
cargo test -p blockmatrix -- blockchain::
cargo test -p blockmatrix -- assets::
cargo test -p blockmatrix -- matrix::

# Run the node
cargo run -p blockmatrix --release
```

## Usage

```rust
use blockmatrix::matrix::MatrixFoundation;
use blockmatrix::assets::core::AssetId;
use hypermesh_lib::MatrixPosition;

// Initialize matrix node at position
let position = MatrixPosition { x: 10, y: 20, z: 5 };
let foundation = MatrixFoundation::new(position);

// Assets are registered on the local blockchain
let asset_id = AssetId::new("cpu-0");
```

## Dependencies

- `hypermesh-lib` -- canonical shared types (NodeId, AssetId, PrivacyMode, BlockchainScope)
- `hypermesh-ebpf` -- eBPF orchestrator for packet processing and policy enforcement
- `blake3` -- content hashing
- `pqcrypto-kyber` -- Kyber-1024 asset encryption
- `tokio` -- async runtime

## License

MIT OR Apache-2.0
