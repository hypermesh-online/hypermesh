# BlockMatrix Development Context

## Status: Alpha (~40-50% complete)

See README.md for current feature list and test counts.

## Core Innovation: Block-MATRIX Topology

Each node is a cell in a 3D geospatial matrix (x,y,z). Tensor operations drive routing, resource allocation, and shard placement.

### What's Implemented
- Matrix coordinate system (4 distance metrics, neighbor finding)
- Tensor operations (Vector3D, Matrix3x3, A* pathfinding)
- Every-node blockchain (independent chain from boot, no network required)
- Geospatial integration (GPS-to-matrix, clustering, load balancing)
- Matrix persistence (WAL, snapshots, recovery)
- Asset adapters (CPU, GPU, Memory, Storage, Network, Container)
- Asset pipeline: Compress (Brotli) -> Encrypt (Kyber-1024 KEM) -> Shard (Reed-Solomon 10+4) -> Distribute (tensor)
- Instruction-based retrieval (send shard maps, not data)
- IPv6 asset addressing (AssetAddress, TransferEngine)
- NAT-like remote proxy system
- Privacy-aware resource allocation
- Network sync (SyncManager, ReflectorPool)
- Cross-scope routing (Device <-> Network via gateway)
- Shard rebalancing on node join/leave
- CLI (CommandExecutor with topology/node/asset commands)
- Cross-platform OS abstraction (Linux/Windows/macOS/BSD)
- IPC daemon (JSON-RPC 2.0 over Unix sockets, handler dispatch, DaemonState)
- TOML config system (load/save/merge, CLI overrides, dotpath get/set)
- Domain-as-network (register domain → Network blockchain, hierarchical DNS, invitation tokens)
- Dashboard manifest (dashboard.toml parse/validate, scope-aware access, scaffold)

## Architectural Truths

1. **Local blockchain starts on boot** -- no network required, node is self-sufficient
2. **Node is its own DNS provider** -- bootstraps independently before network registration
3. **PrivacyMode (transport) is independent from BlockchainScope (state proof)** -- any combination valid
4. **Users can create private networks** across their own devices with shared blockchain
5. **trust.hypermesh.online** is the global Public Gateway -- private networks work without it
6. **PoS IS bootstrap authentication, TLS IS encryption** -- self-signed certs + PoS handshake = working bootstrap (see papers/HYPERMESH.md §5.7)
7. **Handshake is flat JSON** -- `connect_to_peer` MUST use NetworkManager path (NOT MatrixStoqIntegration which sends incompatible MatrixMessage format)

## Key Modules

| Module | Purpose |
|--------|---------|
| `matrix/` | Coordinate system, tensor ops, geospatial, pathfinding |
| `blockchain/` | Per-node chain, block validation, sync manager |
| `assets/` | 6 adapters, pipeline, proxy/NAT, privacy allocation |
| `transfer/` | IPv6 asset addressing, TransferEngine |
| `proof_of_state/` | PoS orchestration (re-exports from TrustChain) |
| `distribution/` | PoS validator integration, redistribution |
| `retrieval/` | Instruction-based shard reconstruction |
| `persistence/` | WAL, snapshots, matrix state recovery |
| `network/` | Blockchain integration, membership |
| `dns/` | DNS registration, domain-as-network, hierarchy, invitations |
| `gateway/` | Cross-scope bridge (Device <-> Network) |
| `privacy/` | Privacy mode switching, tier enforcement |
| `cli/` | Command executor, output formatting |
| `os_integration/` | Cross-platform abstraction |
| `container/` | Basic process isolation, cluster management |
| `ipc/` | JSON-RPC 2.0 daemon, Unix socket IPC, config system (NO HTTP — removed) |
| `api/` | STOQ API bridge (ipc_bridge.rs) — exposes IPC handlers over STOQ |
| `dashboard/` | Dashboard manifest, default HTML, scaffold generator |
