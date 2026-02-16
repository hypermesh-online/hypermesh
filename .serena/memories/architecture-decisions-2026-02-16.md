# HyperMesh Architecture Decisions - 2026-02-16

## User-Confirmed Decisions

### Dead Code (DELETE)
- blockmatrix/protocols/stoq/ - old duplicate of stoq/ crate
- blockmatrix/core/ - 10 dead subcrates (Nexus architecture, Raft/PBFT, Ed25519 - all superseded)
- blockmatrix/src/mfn/ - 4-layer polyglot stack (Zig/Rust/Go), completely disconnected
- blockmatrix/benchmarks/mfn/ - MFN benchmarks
- catalog/src/registry_old.rs - superseded by registry/ module
- tests/integration_stubs.rs - 16 sleep-only fake tests
- All orphaned backup files

### Shared Types: lib/ is canonical
- Add lib/ to workspace as hypermesh-lib
- All shared types (NodeId, AssetId, PrivacyTier, ProofType, etc.) defined once in lib/
- All crates depend on lib/ for common types

### Caesar Intent (Critical Business Logic)
- Gold-pegged (+/-20% per gram) interop bridge token with demurrage + reward
- Liquidity/volatility of physical exchange rate factors fiscal calculations using standard deviation
- Multi-chain: BTC, ETH, Solana, 0x, Tether, XRP, Cosmos, Cardano, Doge
- Bridges: LayerZero, Hyperlink, Stargate, Hyperlane
- Fiat: Stripe, Square, Plaid
- Already on Sepolia: 0x7dcfc3f620634a7de2d065fad5a20c3a9092269b
- NGauge = wallet + middleware + marketing hub + action space for real-world execution

### Gateway Architecture
- Gateway serves trustchain.hypermesh.online (owned, not deployed)
- Each federated network can have its own gateway
- P2P networks work without gateway (private)
- STOQ works standalone without HyperMesh/BlockMatrix

### eBPF: Priority Implementation
- Real AF_XDP/XDP integration is priority (not placeholder)

### TrustChain CA: Single Implementation
- production_certificate_authority.rs is canonical
- Remove duplicates from certificate_authority.rs and mod.rs

### Catalog: Package Registry + Distribution
- Discovery, validation, distribution via STOQ
- Execution on BlockMatrix nodes
- Use FALCON-1024 signing (not ED25519)

## Realistic Completion Estimates
- STOQ: ~70% | TrustChain: ~75% | BlockMatrix: ~5% | Catalog: ~20%
- Caesar: ~15% | Gateway: ~15% | lib: 0% | hypermesh-ebpf: ~30%
