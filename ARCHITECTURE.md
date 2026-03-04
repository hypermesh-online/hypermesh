# HyperMesh Architecture

## Overview

HyperMesh is a sovereign distributed computing platform. Each node runs as a system-level daemon (like DHCP or DNS) on bare metal or VMs. There are no containers, no orchestrators, no cloud dependencies. Nodes communicate exclusively over STOQ (QUIC + eBPF) with FALCON-1024 quantum-resistant cryptography.

The codebase is organized into 10 Rust crates plus a Svelte UI, layered bottom-up across 9 layers.

**11 workspace members** | **999 files** | **289,405 lines** | **1,885 tests**

## Design Philosophy

Lower layers know nothing about higher ones. Higher layers depend only on the public interface of the layer below.

- Transport works without knowing what is being transported.
- Identity works without knowing where data is stored.
- Topology works without knowing what assets exist.
- Assets work without knowing how they are paid for.

## The Nine-Layer Stack

```
+-----------------------------------------------------------+
|  9. UI             Svelte Dashboard                        |
+-----------------------------------------------------------+
|  8. Gateway        HTTP/3 + STOQ dual-listener (4 roles)   |
+-----------------------------------------------------------+
|  7. Catalog        Asset Package Registry                  |
+-----------------------------------------------------------+
|  6. engauge        Analytics, Metrics & Marketplace        |
+-----------------------------------------------------------+
|  5. Caesar         EVP Gold-Gram Protocol + SDK            |
+-----------------------------------------------------------+
|  4. BlockMatrix    Topology, Assets, Blockchain & Tensor   |
+-----------------------------------------------------------+
|  3. TrustChain     Binary Authentication & Certificates    |
+-----------------------------------------------------------+
|  2. STOQ + eBPF    Transport Protocol + Kernel Hooks       |
+-----------------------------------------------------------+
|  1. hypermesh-lib   Shared Types (canonical)               |
+-----------------------------------------------------------+
```

A minimal node runs layers 1-4. Caesar, engauge, Catalog, and Gateway are optional.

---

## Layer 1 -- hypermesh-lib (Foundation)

**Crate**: `lib/` | **3,198 lines** | **97 tests**

The single source of truth for canonical types shared across all 9 dependent crates:

- `NodeId`, `AssetId`, `NetworkId`, `ContentHash` -- identity newtypes
- `PrivacyMode` struct -- 3 presets: `ANONYMOUS`, `PRIVATE`, `PUBLIC` (2-axis: `AccessScope` + `tracked`)
- `BlockchainScope` -- `Device` | `Network` (binary, not 6-variant)
- `ProofType` -- `PoSpace` | `PoStake` | `PoWork` | `PoTime`
- `MatrixPosition`, `MatrixCoordinate` -- topology primitives
- `SystemAssetKind` -- 9 system asset types (CPU through DNS)
- `PipelineStage`, `CryptoAlgorithm`, `HypermeshError`

No circular dependencies. All other crates depend on hypermesh-lib.

---

## Layer 2 -- STOQ + hypermesh-ebpf (Transport)

### STOQ

**Crate**: `stoq/` | **17,897 lines** | **145 tests** | **Status**: complete

STOQ is the OS-level transport protocol. It runs QUIC over IPv6 with eBPF kernel integration for packet-level inspection and flow classification.

**Responsibilities**:
- Connection establishment, stream multiplexing, connection pooling
- Post-quantum transport cryptography (FALCON-1024 signatures, X25519 key exchange)
- BLAKE3 protocol-level token and hash validation
- Privacy tier enforcement at the transport level (Anonymous/Private/Public)
- Adaptive congestion control (EWMA bandwidth, MTU discovery, loss-based CC)
- Multi-path QUIC with 4 schedulers, reflector pool (heartbeat/quorum sync)

**Does NOT do**: Chunking, sharding, compression, deduplication, identity management, certificate issuance.

### hypermesh-ebpf

**Crate**: `hypermesh-ebpf/` | **8,203 lines** | **152 tests**

Single source of truth for all eBPF packet processing. Three execution paths at XDP:

- **Zero-copy**: AF_XDP direct to STOQ (mmap UMEM, 4-ring setup)
- **Delegate**: XDP_TX to matrix neighbor
- **Local**: XDP_PASS to kernel stack

STOQ is a thin consumer (`StoqEbpfTransport`). BlockMatrix is the policy configurator. Graceful degradation: full eBPF+AF_XDP, eBPF without AF_XDP, or userspace-only.

---

## Layer 3 -- TrustChain (Trust)

**Crate**: `trustchain/` | **33,923 lines** | **95 tests** | **Status**: 95% complete

Manages decentralized identity through a federated Certificate Authority with FALCON-1024 post-quantum signatures.

**Responsibilities**:
- Certificate Authority: issuance, revocation (OCSP/CRL), lifecycle management
- FALCON-1024 post-quantum signing
- Certificate Transparency: Merkle logs, SCTs, audit trails
- Binary authentication (pass/fail, no trust scoring or float-based levels)
- Threshold crypto (Shamir SSS over GF(256), wraps FALCON-1024 key splitting)
- Federation: FederatedCA peers with Full/Conditional/Untrusted trust levels (policy gates)
- DNS protocol resolution (serving signed DNS records over STOQ)
- Security monitoring and Byzantine detection

**Does NOT do**: Enforce network isolation (STOQ), manage topology (BlockMatrix), distribute data (BlockMatrix).

---

## Layer 4 -- BlockMatrix (Matrix)

**Crate**: `blockmatrix/` | **128,404 lines** | **821 tests** | **Status**: alpha

The largest crate. Assigns every node a position in a 3D coordinate space (the Block-MATRIX) and manages the entire asset lifecycle.

### Topology

- 3D coordinate system (x,y,z node positioning)
- Tensor-weighted path optimization (bandwidth, latency, reliability, load)
- Geospatial clustering (GPS-to-matrix conversion, region-aware placement)
- A* pathfinding and neighbor discovery

### Assets (everything in the mesh is an asset)

- 6 asset adapters: CPU, GPU, Memory, Storage, Network, Container
- Privacy allocation with user controls (resource percentages, concurrent limits, duration)
- NAT-like remote proxy addressing for memory and resources
- Content-addressed storage with hash bucket deduplication

### IPv6 Asset Addressing

Every asset gets a globally unique `AssetAddress` derived from its `AssetId`. The `TransferEngine` handles asset transfers across the mesh using IPv6-compatible addressing. A 10-node transfer simulation validates the addressing and routing under realistic conditions.

### Data Pipeline (exact order)

1. **Compress** -- Brotli streaming (levels 1-11)
2. **Encrypt** -- Kyber-1024 KEM, AES-256-GCM whole-blob encryption (not per-shard)
3. **Shard** -- Reed-Solomon erasure coding (10 data + 4 parity)
4. **Distribute** -- Tensor-based placement at calculated matrix positions

### Blockchain

- Every node runs an independent Device blockchain (starts on boot, no network required)
- Optional Network blockchain (synchronized via reflector/swarm mode)
- Proof of State four-proof validation (PoSpace/PoStake/PoWork/PoTime)
- Matrix persistence (WAL, snapshots, recovery)

### Networking (state proof layer, not transport)

- Federated trust networking
- DNS record registration and storage
- Network membership and multi-network participation
- Privacy-aware routing at the asset level

**Does NOT do**: Transport-level encryption (STOQ), certificate issuance (TrustChain), package versioning (Catalog), payment processing (Caesar).

---

## Layer 5 -- Caesar + caesar-sdk (Economy)

### Caesar

**Crate**: `caesar/` | **12,753 lines** | **220 tests** | **Status**: complete

The EVP (Economic Value Protocol) gold-gram payment protocol. Tracks value through CaesPackets with state machine transitions and conservation invariants.

**Responsibilities**:
- CaesPacket state machine: Minted, InTransit, Delivered, Settling, Settled, Expired, Refunded, Dissolved
- Governor: PID controller for fee adjustment, FeeCaps, NetworkMetrics
- Settlement: AcceptanceCriteria, fee distribution, conservation enforcement
- Conservation: Input = Output + Fees + Demurrage (circuit breaker, full audit)
- OracleFeed trait for pluggable gold price feeds
- STOQ API: 5 handlers wired to real CaesarProtocol

### caesar-sdk

**Crate**: `caesar-sdk/` | **1,039 lines** | **2 tests**

Extracted SDK traits with zero Caesar-internal dependencies:
- `IngressAdapter` (7 methods), `EgressAdapter` (5 methods)
- `MeshCreditAdapter` reference implementation
- Public mocks for SDK consumers

---

## Layer 6 -- engauge (Analytics)

**Crate**: `engauge/` | **5,758 lines** | **135 tests** | **Status**: complete

Engagement analytics and resource marketplace. 10 modules:

- **receipt, metrics, compliance** -- measurement and audit trail
- **organic_detection** -- distinguish real vs. artificial engagement
- **throttle, capacity** -- rate limiting and resource management
- **trending** -- trend detection algorithms
- **streaming** -- MetricsFrame protocol (4 payloads: Capacity/Congestion/Routing/Economic), differential privacy (Laplace noise)
- **routing_intel** -- RoutingAdvisor + PathAdvisor traits for tensor weight modification
- **marketplace** -- ResourcePool, LeaseContract lifecycle, PricingEngine (4 tier multipliers)

Privacy-aware: Anonymous shares nothing, Private shares Capacity+Congestion within federation, Public shares all 4 payloads mesh-wide.

---

## Layer 7 -- Catalog (Registry)

**Crate**: `catalog/` | **27,932 lines** | **52 tests** | **Status**: alpha

Asset package registry. Defines asset specifications and provides discovery. Catalog is a **registry and package manager**, not a marketplace and not an execution environment.

**Responsibilities**:
- Asset package types, metadata, versioning, dependency resolution
- Registry operations: publish, install, search
- DHT-based distributed discovery
- STOQ transport and TrustChain security integration
- Template generation framework
- Caesar reward integration (30% publications, 30% refs, 25% validation, 15% maintenance)

**Does NOT do**: Store or execute assets (BlockMatrix), handle payments (Caesar), act as a marketplace.

---

## Layer 8 -- Gateway (Entry Point)

**Crate**: `gateway/` | **7,028 lines** | **155 tests** | **Status**: complete

HTTP/3 + STOQ dual-listener for `trust.hypermesh.online`. 4 operational roles:

1. **Clearnet Bootstrap**: HTTP/3 at port 8443 for initial STOQ connection info + bootstrap tokens
2. **Inbound Proxy**: HTTP/3 access to HyperMesh dashboards (resource dashboard, engauge panel)
3. **Outbound Proxy**: Bridge HyperMesh resources to non-HyperMesh clearnet endpoints
4. **Inter-Network**: STOQ-to-STOQ bridge between federated/private/public networks

Features: TLS (File/TrustChain/SelfSigned), PoS authentication, cross-scope routing (Device to Network), federation bridge, rate limiting (token bucket), load balancing (4 strategies), multi-domain SNI routing.

---

## Layer 9 -- UI (Dashboard)

**Crate**: `ui/` | **43,270 lines** | **11 tests**

Svelte-based dashboard for node management, asset browsing, and network visualization.

---

## Cryptography

| Purpose | Algorithm |
|---------|-----------|
| Protocol signing (TrustChain CA, STOQ handshake) | FALCON-1024 |
| Asset encryption (whole-blob before sharding) | Kyber-1024 (KEM then AES-256-GCM) |
| Content hashing (all content, blockchain, verification) | BLAKE3 |
| X.509 certificate fingerprints, OCI digests | SHA-256 (industry standard) |

---

## Privacy Independence

`PrivacyMode` (transport layer) and `BlockchainScope` (state proof layer) are independent dimensions:

- **PrivacyMode**: `ANONYMOUS` (unbounded, untracked) | `PRIVATE` (bounded, tracked) | `PUBLIC` (unbounded, tracked)
- **BlockchainScope**: `Device` (local-only, always running) | `Network` (synchronized via reflector)

| | Anonymous Transport | Private Transport | Public Transport |
|---|---|---|---|
| **Device Scope** | Local chain, untraceable | Local chain, group-visible | Local chain, fully visible |
| **Network Scope** | Synced state, untraceable | Synced state, group-visible | Synced state, fully visible |

Any combination is valid. They are configured independently.

---

## Proof of State (Four-Proof Authentication)

Every asset requires ALL FOUR proofs -- this is binary authentication (authentic or not), not trust scoring:

- **PoSpace (WHERE)**: Storage location and physical/network position
- **PoStake (WHO)**: Ownership, access rights, economic stake
- **PoWork (WHAT/HOW)**: Computational resources and processing
- **PoTime (WHEN)**: Temporal ordering and timestamp validation

Combined into a unified State Proof answering WHERE/WHO/WHAT/WHEN. Bilateral verification -- no voting, no quorum, no leader election.

---

## Instruction-Based Retrieval

Traditional: Send raw data to receiver.
BlockMatrix: Send shard map instructions (~748 bytes). Receiver queries matrix positions, fetches shards from nearest nodes, reconstructs locally. This gives bandwidth efficiency, distributed load, resilience, and deduplication.

---

## Deployment Model

Each HyperMesh node runs as a **systemd service** on the host OS. No Docker, no Kubernetes, no cloud orchestration.

- Device blockchain starts on boot (no network required)
- Network blockchains joined after connectivity is established
- Each node is its own DNS provider before network registration

---

## Crate Dependency Graph

```
hypermesh-lib (canonical types, 9 dependents)
    |
    +-- hypermesh-ebpf (kernel validation)
    |
    +-- stoq (transport)
    |     |
    |     +-- hypermesh-ebpf
    |     +-- engauge (optional, feature-gated)
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
    |     +-- stoq
    |     +-- caesar
    |
    +-- caesar (payments)
    |     |
    |     +-- stoq
    |     +-- caesar-sdk
    |     +-- engauge (optional, feature-gated)
    |
    +-- caesar-sdk (SDK traits)
    |
    +-- engauge (analytics)
    |
    +-- gateway (HTTP/3 + STOQ entry)
          |
          +-- stoq
```

All arrows point upward from hypermesh-lib. No circular dependencies.
