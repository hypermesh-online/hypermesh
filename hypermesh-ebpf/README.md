# HyperMesh eBPF

Unified eBPF intelligence layer -- the single source of truth for all packet processing in the HyperMesh ecosystem. Enforces Proof of State, asset hash verification, matrix routing, and privacy policies at kernel level with microsecond latency.

**Status**: 100% Complete (alpha) | 19 files | ~8,200 lines | 152 tests

## Architecture

```
+-----------------------------------------------+
|   HyperMesh Application                       |
|   - Defines validation POLICIES                |
|   - Matrix topology, blockchain               |
|   - Proof of State semantics                  |
+------------------------+-----------------------+
                         | Policy Configuration
                         v
+-----------------------------------------------+
|   hypermesh-ebpf (THIS CRATE)                  |
|   - Enforces policies at kernel level          |
|   - Packet filtering with PoS validation       |
|   - Asset hash verification (BLAKE3)           |
|   - Matrix routing compliance                  |
+------------------------+-----------------------+
                         | Extension Headers
                         v
+-----------------------------------------------+
|   STOQ Protocol                                |
|   - Generic transport (like TCP/TLS)           |
|   - Carries extension headers                  |
|   - NO knowledge of HyperMesh semantics        |
+-----------------------------------------------+
```

**Consumer pattern**: STOQ is a thin consumer (`StoqEbpfTransport`, ~131 LOC wrapper). BlockMatrix is a configurator (policy setter). This crate is the single source of truth.

## Key Components

### HyperMeshEbpf Orchestrator
Central API for eBPF lifecycle management:
- `attach_xdp()` -- attach to network interface
- `create_af_xdp_socket()` -- zero-copy I/O setup
- `set_privacy_tier()` / `set_routing_rule()` / `register_asset_hash()` -- policy control
- Capability detection and state management

### Three-Path Graceful Degradation
1. **Full**: eBPF + AF_XDP (kernel-level filtering + zero-copy I/O)
2. **Partial**: eBPF without AF_XDP (kernel filtering, userspace I/O)
3. **Userspace**: No kernel attachment (pure userspace validation)

### PacketDecision Routing
- `Pass` -- deliver to local stack (XDP_PASS)
- `Redirect { socket }` -- zero-copy to AF_XDP socket for STOQ consumption
- `Forward { next_hop }` -- XDP_TX to matrix neighbor node
- `Drop { reason }` -- reject at kernel level

### AF_XDP Zero-Copy I/O
Real zero-copy via direct libc syscalls:
- mmap UMEM allocation
- 4-ring setup (fill/completion/rx/tx)
- Frame allocator with batch operations
- Multi-queue load balancing (RoundRobin/LeastLoaded/FlowHash)
- Drop-based cleanup

### C Kernel XDP Program (`hypermesh_xdp.c`, ~457 lines)
- 4 BPF maps for policy/PoS/asset/xsk state
- HyperMesh extension header parsing (magic `0x484D`)
- Structural PoS validation (difficulty, algorithm indicator, TTL-based cache)
- Full crypto (FALCON-1024/Ed25519/ECDSA) deferred to userspace

### HyperMesh Extension Headers
- **Proof of State** (0x1000): WHO/WHAT/WHEN/WHERE consensus proofs
- **Asset Hash** (0x1001): BLAKE3 content integrity
- **Matrix Routing** (0x1002): Topology-aware routing paths (IPv6 + matrix position)
- **Privacy Tier** (0x1003): Access control enforcement

### Validation Hooks
- STOQ registers `CertificateValidator` + `PacketValidator`
- BlockMatrix registers `ExtensionValidator`
- All registered via `set_validation_hooks()`

### Policy Sync
`sync_to_kernel()` serializes `ValidationPolicy` to 32-byte little-endian format matching the C struct `policy_value`.

### Hardware Offload
- Driver detection (mlx5_core/nfp/bnxt_en)
- `OffloadPolicy`: Disabled | Opportunistic | Required
- Automatic fallback to native XDP

## Features

- **`default`**: Userspace validation only (no kernel attachment)
- **`kernel-attach`**: Real eBPF program loading via aya/libc (requires CAP_NET_ADMIN)
- **`ebpf-loader`**: C program compilation via build.rs

## Requirements

- Linux kernel 4.18+ (AF_XDP support)
- CAP_NET_ADMIN capability (eBPF attachment)
- libbpf-dev (kernel attachment)

## Quick Start

```bash
cargo build -p hypermesh-ebpf --release
cargo test -p hypermesh-ebpf
```

## License

Business Source License 1.1
