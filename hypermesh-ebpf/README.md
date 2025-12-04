# HyperMesh eBPF Intelligence Layer

**Sprint 2.1: Intelligence Layer Separation**

## Overview

`hypermesh-ebpf` provides kernel-level enforcement of HyperMesh intelligence policies. This crate implements HyperMesh-specific eBPF programs that validate Proof of State, Asset Hashes, Matrix Routing, and Privacy Tiers at microsecond latency.

## Architecture Principle

**STOQ provides MECHANISMS, HyperMesh provides POLICIES, eBPF enforces AT KERNEL LEVEL.**

```
┌─────────────────────────────────────────────┐
│   HyperMesh Application                     │
│   - Defines validation POLICIES              │
│   - Matrix topology, blockchain             │
│   - Proof of State semantics                │
└──────────────────┬──────────────────────────┘
                   │ Policy Configuration
                   ↓
┌─────────────────────────────────────────────┐
│   hypermesh-ebpf (THIS CRATE)               │
│   - Enforces policies at kernel level      │
│   - Packet filtering with PoS validation   │
│   - Asset hash verification                │
│   - Matrix routing compliance              │
└──────────────────┬──────────────────────────┘
                   │ Extension Headers
                   ↓
┌─────────────────────────────────────────────┐
│   STOQ Protocol                             │
│   - Generic transport (like TCP/TLS)       │
│   - Carries extension headers               │
│   - NO knowledge of HyperMesh semantics    │
└─────────────────────────────────────────────┘
```

## Key Components

### 1. **HyperMesh Extension Headers** (`hypermesh_headers.rs`)

Defines HyperMesh-specific extension types carried in STOQ packets:

- **Proof of State** (0x1000): WHO/WHAT/WHEN/WHERE consensus proofs
- **Asset Hash** (0x1001): Content integrity validation
- **Matrix Routing** (0x1002): Topology-aware routing paths
- **Privacy Tier** (0x1003): Access control enforcement

### 2. **Policy Maps** (`policy_maps.rs`)

Userspace-to-kernel policy configuration:

```rust
let policy = ValidationPolicy {
    requires_pos: true,
    validate_asset_hash: true,
    check_matrix_routing: true,
    privacy_tier: 2, // Federated
    max_packet_size: 9000,
    rate_limit_per_sec: 100,
};

policy_manager.set_policy(connection_id, policy);
```

### 3. **Packet Filtering** (`packet_filter.rs`)

XDP packet filtering with HyperMesh intelligence:

- Kernel-level packet inspection
- Drop invalid packets before userspace sees them
- Zero-copy redirect for validated packets
- Microsecond-level performance

### 4. **Validation Logic** (`validation.rs`)

Implements HyperMesh-specific validation:

- **Proof of State Validator**: Four-proof consensus validation
- **Asset Hash Validator**: BLAKE3 hash verification
- **Shard Set Validator**: Multi-part asset integrity

### 5. **Metrics** (`metrics.rs`)

Intelligence-specific metrics:

- Proof of State validation rates
- Asset hash verification statistics
- Matrix routing performance
- Privacy tier enforcement metrics

## Usage

### Basic Setup

```rust
use hypermesh_ebpf::{HyperMeshEbpf, ValidationPolicy};

// Create eBPF manager
let mut ebpf = HyperMeshEbpf::new()?;

// Configure policies
ebpf.policy_manager_mut()
    .set_default_policy(ValidationPolicy::strict());

// Attach to network interface (requires root privileges)
#[cfg(feature = "kernel-attach")]
ebpf.attach("eth0")?;

// Get metrics
let metrics = ebpf.get_metrics();
println!("{}", metrics);
```

### Validation

```rust
use hypermesh_ebpf::{ProofOfStateValidator, AssetHashValidator};

// Validate Proof of State
let validator = ProofOfStateValidator::default();
validator.validate(&proof_header)?;

// Validate asset hash
AssetHashValidator::validate(&asset_header, payload)?;
```

## STOQ Independence

**Critical**: STOQ protocol has **ZERO** HyperMesh dependencies after Sprint 2.1 refactoring.

- STOQ treats extension headers as opaque byte blobs
- STOQ provides validation hooks, applications implement validators
- STOQ can be used by ANY application, not just HyperMesh

## Features

- **`default`**: Userspace validation only (no kernel attachment)
- **`kernel-attach`**: Enable actual eBPF program loading (requires privileges)

## Requirements

- Rust 1.70+
- Linux kernel 4.18+ (for AF_XDP support)
- CAP_NET_ADMIN capability (for eBPF attachment)
- libbpf-dev (for kernel attachment)

## Status

**Sprint 2.1 Complete**: ✅ Intelligence layer separated from transport

- ✅ HyperMesh extension headers defined
- ✅ Policy configuration infrastructure
- ✅ Validation logic implemented (userspace)
- ✅ Metrics collection framework
- ✅ STOQ independence verified (zero deps)

**Future Work (Sprint 2.2+)**:
- Actual eBPF XDP program implementation
- Kernel-level validation enforcement
- Performance benchmarking vs userspace

## Architecture Compliance

### ✅ STOQ Remains Generic Transport
- Zero HyperMesh dependencies in STOQ
- Extension headers are opaque to STOQ
- Can become internet standard (RFC-ready)

### ✅ Clear Separation of Concerns
- **STOQ**: Reliable transport, eBPF infrastructure
- **hypermesh-ebpf**: Validation policies, intelligence enforcement
- **HyperMesh**: Policy definitions, extension semantics

### ✅ Reusable by Other Applications
- Other apps can implement their own eBPF validators
- STOQ validation hooks are generic
- No HyperMesh lock-in

## License

MIT OR Apache-2.0

## Author

HyperMesh Team
