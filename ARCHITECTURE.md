# HyperMesh Architecture

## Overview

HyperMesh is a sovereign distributed computing platform built from six composable layers. Each layer has a single responsibility, communicates through well-defined interfaces, and can be understood independently.

**8 crates** | **575 Rust files** | **208,787 lines** | **816 tests**

## Design Philosophy

The stack follows a **bottom-up** design. Lower layers know nothing about higher ones. Higher layers depend only on the public interface of the layer immediately below.

- Transport works without knowing what is being transported.
- Identity works without knowing where data is stored.
- Topology works without knowing what assets exist.
- Assets work without knowing how they are paid for.

## The Six-Layer Stack

```
+--------------------------------------------------+
|  6. NGauge        Observability & Metrics          |  [planned]
+--------------------------------------------------+
|  5. Caesar        Payment Bridge (optional)        |  [optional]
+--------------------------------------------------+
|  4. Catalog       Package Registry & Discovery     |  [core]
+--------------------------------------------------+
|  3. BlockMatrix   Topology, Assets & Coordination  |  [core]
+--------------------------------------------------+
|  2. TrustChain    Identity & Certificates          |  [core]
+--------------------------------------------------+
|  1. STOQ          Transport Protocol               |  [core]
+--------------------------------------------------+
|     OS / Kernel   eBPF hooks, IPv6 stack           |
+--------------------------------------------------+
```

A minimal node runs layers 1-4. Caesar is opt-in for paid content. NGauge is planned.

---

## Layer 1 — STOQ (Transport)

**Crate**: `stoq/` | **14,039 lines** | **65 tests** | **Phase**: alpha

### What it does

STOQ is the OS-level transport protocol. It runs QUIC over IPv6 with eBPF kernel integration for packet-level inspection and flow classification.

### Responsibilities

- Connection establishment, stream multiplexing, connection pooling
- Post-quantum transport cryptography (FALCON-1024 signatures, X25519 key exchange)
- BLAKE3 protocol-level token/hash validation
- Network isolation enforcement at the transport level (4 privacy tiers)
- Adaptive congestion control and transport metrics

### What it does NOT do

- Chunking, sharding, compression, deduplication, or edge caching (BlockMatrix)
- Federated trust networking or routing decisions (BlockMatrix)
- Identity management or certificate issuance (TrustChain)

### Interface

Exposes a connection-oriented stream API (`TransportStream`) to TrustChain above.

---

## Layer 2 — TrustChain (Identity)

**Crate**: `trustchain/` | **30,027 lines** | **63 tests** | **Phase**: alpha

### What it does

TrustChain manages decentralized identity through a federated Certificate Authority, FALCON-1024 post-quantum signatures, and Certificate Transparency logs.

### Responsibilities

- Certificate Authority: issuance, revocation, lifecycle management
- FALCON-1024 post-quantum signing and Kyber-1024 key encapsulation
- Certificate Transparency: Merkle logs, SCTs, audit trails
- Proof of State four-proof consensus (WHERE/WHO/WHAT/WHEN)
- DNS protocol resolution (serving signed DNS records over STOQ)
- Security monitoring and Byzantine detection

### What it does NOT do

- Enforce network isolation (STOQ enforces transport isolation)
- Manage federated trust networking (BlockMatrix `network/trust/`)
- Handle DNS record registration/storage (BlockMatrix `dns/`)
- Route, distribute, or deduplicate data (BlockMatrix)

### Interface

Consumes `TransportStream` from STOQ. Exposes authenticated, identity-tagged channels (`TrustedChannel`) to BlockMatrix.

---

## Layer 3 — BlockMatrix (Topology, Assets & Coordination)

**Crate**: `blockmatrix/` | **129,696 lines** | **624 tests** | **Phase**: alpha

### What it does

BlockMatrix is the largest layer. It assigns every node a position in a 3D coordinate space and manages the entire asset lifecycle — from storage to distribution.

### Responsibilities

**Topology**:
- 3D coordinate system (x,y,z node positioning)
- Tensor-weighted path optimization (bandwidth, latency, reliability, load)
- Geospatial clustering (GPS-to-matrix conversion, region-aware placement)
- A* pathfinding and neighbor discovery

**Assets** (everything in the mesh is an asset):
- 6 asset adapters: CPU, GPU, Memory, Storage, Network, Container
- Privacy allocation (5 levels, 4 tiers with user controls)
- NAT-like remote proxy addressing for memory/resources
- Content-addressed storage with hash bucket deduplication

**Distribution pipeline** (exact order):
1. Compression (Brotli)
2. Encryption (Kyber-1024)
3. Sharding (Reed-Solomon)
4. Placement (tensor-based matrix positioning)

**Blockchain**:
- Every-node independent blockchain (starts on boot, no network required)
- Proof of State four-proof validation
- Matrix persistence (WAL, snapshots, recovery)

**Networking** (consensus layer, not transport):
- Federated trust networking (`network/trust/federated.rs`)
- DNS record registration and storage (`dns/`)
- Network membership and multi-network participation
- Privacy tier enforcement at the asset/routing level

### What it does NOT do

- Transport-level encryption or connection management (STOQ)
- Certificate issuance or identity verification (TrustChain)
- Package versioning, dependency resolution, or registry (Catalog)
- Payment processing or reward distribution (Caesar)

### Interface

Consumes `TrustedChannel` from TrustChain. Exposes coordinate-aware topology (`TopologyView`) to Catalog.

---

## Layer 4 — Catalog (Package Registry)

**Crate**: `catalog/` | **25,794 lines** | **41 tests** | **Phase**: alpha

### What it does

Catalog is the package registry. It defines asset specifications and provides discovery for mesh resources. Catalog is a **registry**, not an execution environment — assets belong to their owners on BlockMatrix nodes.

### Responsibilities

- Asset package types, metadata, and YAML-based specs
- Registry operations: publish, install, search
- Semantic versioning and dependency resolution
- Template generation framework
- HyperMesh execution delegation (strategy-based placement)
- Scripting engine (syntax validation only — no local execution)

### What it does NOT do

- Store or own assets (BlockMatrix)
- Execute code locally (delegates to HyperMesh nodes)
- Handle transport, identity, or topology concerns

### Interface

Consumes `TopologyView` from BlockMatrix for discovery routing and TrustChain identity for access control. Exposes `AssetEvent` to Caesar.

---

## Layer 5 — Caesar (Payment Bridge) [Optional]

**Crate**: `caesar/` | **5,654 lines** | **4 tests** | **Phase**: planning

### What it does

Caesar is a payment bridge for paid content hosting and contract execution. It connects external payment systems to the mesh.

### Responsibilities

- Token economics and wallet management
- Transaction processing and staking
- Exchange rate engine
- Cross-chain bridge types (8 networks: BTC, ETH, SOL, etc.)
- Reward calculation framework

### Why it's optional

Caesar has **zero dependency** from the core protocol. Removing it changes nothing about STOQ, TrustChain, BlockMatrix, or Catalog. It exists solely for assets that require payment.

---

## Layer 6 — NGauge (Observability) [Planned]

Not yet implemented. Will provide unified metrics, distributed tracing, health monitoring, and alerting across all layers.

---

## Supporting Crates

### hypermesh-lib (Shared Types)

**Crate**: `lib/` | **235 lines** | **Phase**: alpha

The single source of truth for canonical types shared across all crates:
- `NodeId`, `AssetId` — identity newtypes
- `NetworkPrivacyTier` — Anonymous | P2P | Federated | Public
- `BlockchainScope` — Device | Network
- `ProofType` — PoSpace | PoStake | PoWork | PoTime
- `MatrixPosition`, `MatrixCoordinate` — topology primitives
- `PipelineStage`, `CryptoAlgorithm`, `HypermeshError`

All 7 other crates depend on hypermesh-lib. No circular dependencies.

### hypermesh-ebpf (Kernel Integration)

**Crate**: `hypermesh-ebpf/` | **1,904 lines** | **19 tests** | **Phase**: alpha

Userspace eBPF validation framework consumed by STOQ:
- PoS header parsing and asset hash verification (BLAKE3)
- Matrix routing path validation
- Policy map management
- XDP program loading via `aya` (feature-gated)

### Gateway (HTTP/3 Entry Point)

**Crate**: `gateway/` | **1,438 lines** | **Phase**: planning

HTTP/3 gateway for `trust.hypermesh.online` and federated entry points:
- QUIC/HTTP3 server setup (quinn + h3)
- Router with path-based backend selection
- Circuit breaker and retry logic

---

## Cross-Layer Interfaces

| Boundary | Interface | Data Exchanged |
|---|---|---|
| OS to STOQ | eBPF program hooks | Raw packets, flow metadata |
| STOQ to TrustChain | `TransportStream` | Authenticated byte streams |
| TrustChain to BlockMatrix | `TrustedChannel` | Identity-tagged, privacy-classified channels |
| BlockMatrix to Catalog | `TopologyView` | Coordinate positions, tensor-weighted paths |
| Catalog to Caesar | `AssetEvent` | Asset allocation and release events |
| All to NGauge | `MetricEmitter` | Counters, histograms, trace spans |

## Key Architectural Concepts

### Proof of State (Four-Proof Consensus)

Every asset requires ALL FOUR proofs:
- **PoSpace (WHERE)**: Storage location and physical/network position
- **PoStake (WHO)**: Ownership, access rights, economic stake
- **PoWork (WHAT/HOW)**: Computational resources and processing
- **PoTime (WHEN)**: Temporal ordering and timestamp validation

This is bilateral verification, not global consensus. The mesh scales without consensus bottlenecks.

### Privacy Independence

Network privacy tiers (transport) and blockchain scopes (consensus) are independent dimensions:

| | Anonymous Transport | Private Transport | Public Transport |
|---|---|---|---|
| **Device Scope** | Local chain, untraceable | Local chain, group-visible | Local chain, fully visible |
| **Network Scope** | Synced state, untraceable | Synced state, group-visible | Synced state, fully visible |

### Instruction-Based Retrieval

Traditional: Send raw data to receiver.
BlockMatrix: Send shard map instructions. Receiver queries matrix positions, fetches shards from nearest nodes, reconstructs locally.

### Every Node = Own Blockchain

Each node's blockchain starts immediately on boot with a unique genesis block. No network connectivity required. Network participation is optional — a node is fully functional for local operations from the moment of creation.

## Crate Dependency Graph

```
hypermesh-lib (canonical types)
    |
    +-- stoq (transport)
    |     |
    |     +-- hypermesh-ebpf (kernel validation)
    |
    +-- trustchain (identity)
    |     |
    |     +-- stoq
    |
    +-- blockmatrix (topology + assets)
    |     |
    |     +-- trustchain
    |     +-- stoq
    |     +-- hypermesh-ebpf
    |
    +-- catalog (registry)
    |     |
    |     +-- blockmatrix
    |     +-- trustchain
    |
    +-- caesar (payments)
    |     |
    |     +-- blockmatrix
    |
    +-- gateway (HTTP/3 entry)
          |
          +-- trustchain
          +-- stoq
```
