# Web3 Ecosystem - Development Project Context

## Hard Protocol Requirements

**Canonical source**: `papers/HYPERMESH.md` Section 3 (Protocol Requirements), R1-R14.

**RULE**: Whenever the user says "this is a hard requirement" (or equivalent), check if it's already in `papers/HYPERMESH.md` Section 3. If not, add it as the next R-number. Keep requirements concrete and testable. Update this section's summary when requirements change.

**Current requirements (R1-R14)**:
- **R1** Sovereign genesis with asset instantiation — hardware assessed (not self-reported) as IPv6-addressed assets with Proof of State
- **R2** Four-proof authentication — PoSpace/PoStake/PoWork/PoTime on every state claim, binary authentic/not
- **R3** Pipeline ordering — Compress→Encrypt→Shard→Distribute, whole-blob encryption before sharding
- **R4** Content-addressed dedup — privacy-scoped reference tracking (full in Device/Private, none in Anonymous), tamper detection via BLAKE3 hash
- **R5** Erasure-coded redundancy — k-of-n reconstruction (default 10-of-14 Reed-Solomon)
- **R6** Instruction-based retrieval — shard maps under 1KB, receiver reconstructs
- **R7** Post-quantum storage encryption — Kyber-1024 KEM for data at rest, non-negotiable
- **R8** Cipher suite policy — standard suite mandatory on Public, configurable on Private/Anonymous by agreement, standard is default for all
- **R9** Privacy-scope independence — PrivacyMode and BlockchainScope are independent dimensions, no combo prohibited
- **R10** Universal asset model with transmission — all resources are IPv6-addressed assets; Transmission is first-class for mesh relay; genesis block instantiates initial assets from hardware assessment
- **R11** Bilateral verification — no global consensus/quorum/leader, cost scales with transaction volume not mesh size
- **R12** Shard commitment with swarm scaling — `BLAKE3(sorted placements)` per block; consumers become providers; O(log N) per-node load for N consumers
- **R13** Minimum device spec — 1 Mb/s network, 50GB storage, 4GB RAM, 2-core 1GHz; streaming reconstruction mandatory
- **R14** Adaptive shard sizing — RS parameters scale with asset size, NO post-creation shard splitting (breaks content-addressing hashes), QUIC handles wire chunking, copy-then-redirect migration

---

## 🎯 **Current Status: Single-Node Alpha — Network Sync Not Functional**

**Development Status**: ⚠️ **SINGLE-NODE ALPHA** - Services run individually but are NOT a decentralized mesh
**Repository Status**: ✅ **MONOREPO** - 12 crates in core workspace, 6 services deployed to trust.hypermesh.online
**Implementation Status**: ⚠️ **CRITICAL GAPS** - No real Proof of State (PoS is self-generated), no cross-node sync, certs are rcgen not FALCON-1024, DNS is local-only

---

## 📦 **Repository Architecture**

### **GitHub Organization**: [hypermesh-online](https://github.com/hypermesh-online)

| Component | Repository | Status | Critical Gap |
|-----------|------------|--------|--------------|
| **BlockMatrix** | `/blockmatrix` | ⚠️ **60% (29/48)** | No cross-node sync, no PoS on blocks, shards local-only |
| **Caesar** | `/caesar` | ⚡ **77% (24/31)** | Single-node only, no multi-node packet routing |
| **Caesar-SDK** | `/caesar-sdk` | ⚡ **71% (5/7)** | Trait definitions complete, no external adapters |
| **Catalog** | `/catalog` | ⚠️ **59% (13/22)** | Local-only, DHT not wired to network, OOM bug |
| **Engauge** | `/engauge` | ⚠️ **65% (19/29)** | No real data pipeline from services, metrics are local/mock |
| **Gateway** | `/gateway` | ⚠️ **66% (18/27)** | HTTP/3 proxy works but STOQ bridge incomplete, no real PoS auth |
| **Hypermesh-eBPF** | `/hypermesh-ebpf` | ⚡ **73% (14/19)** | eBPF framework real, PoS validation validates fake proofs |
| **Lib** | `/lib` | ✅ **92% (25/27)** | Types correct, but state proof types exercised with fake data |
| **STOQ** | `/stoq` | ⚡ **72% (21/29)** | QUIC transport genuine, FALCON-1024 signing not integrated |
| **TrustChain** | `/trustchain` | ⚠️ **53% (21/39)** | Single-node CA, certs are rcgen not FALCON, proofs are hardcoded |
| **HyperMesh SDK** | `/hypermesh-sdk` | ⚡ **100% (9/9)** | Alpha — no domain/dashboard/config API yet |
| **UI** | `/ui` | ⚠️ **32% (9/28)** | Components render, no real backend data connections |

### Critical Architectural Note: Block-MATRIX Topology
All components operate within a Block-MATRIX network where each node is a cell in a geospatial matrix (x,y,z coordinates). This enables:
- **Tensor Operations**: Mathematical matrix operations for routing and resource allocation
- **Dual-Scope Blockchain**: Device (local) and Network (synced) blockchain scopes
- **Matrix-Aware Coordination**: Intelligent shard distribution based on topology
- **Network Independence**: Local blockchain runs regardless of network connectivity

**CURRENT STATE**: Single blockchain per node — Device scope only (5-10% implemented)
**TARGET STATE**: Device + Network blockchain scopes (see "Blockchain Architecture" section below)

### **Repository Sync Commands**
```bash
scripts/deploy/sync-repos.sh              # Sync all components
scripts/deploy/sync-repos.sh stoq         # Sync specific component
scripts/deploy/sync-repos.sh --dry-run    # Preview changes
scripts/deploy/deploy-all.sh              # One-command deployment
```

---

## 🔧 **Critical Gaps (Next Priority)**

### **1. Network Scope Blockchain Implementation**
- ❌ Network scope sync (reflector/swarm mode)
- ✅ Gateway architecture for Device-to-Network bridging (implemented in `/gateway/`)
- ❌ Cross-network asset transfers
- ❌ Reflector pooling for Network chain synchronization

### **2. Integration and Testing**
- ⚡ Component integration tests needed (components work individually)
- ⚡ End-to-end workflow validation required
- ⚡ Multi-node testing under development
- ✅ Byzantine fault tolerance code implemented (needs verification)

### **3. Production Infrastructure**
- ❌ CI/CD pipelines not configured
- ⚡ Storage backends implemented, optimization needed
- ❌ Load balancing and auto-scaling not deployed
- ⚡ Monitoring eBPF integration implemented in STOQ

### **4. Performance Optimization**
- ⚡ STOQ transport optimization (current: 2.95 Gbps, target: adaptive tiers)
- ⚡ Multi-node Proof of State verification testing
- ⚡ Real-world stress testing and profiling
- ⚡ Production deployment hardening

---

## 📋 **Core Architecture (Technical Reference)**

### **Proof of State Four-Proof System (⚡ 50% Implemented)**
**Location**: `/trustchain/src/proof_of_state/` (primary implementation, re-exported by BlockMatrix)
**Secondary**: `/blockmatrix/src/proof_of_state/` (BlockMatrix-specific state proof orchestration)
**Reference**: Original NKrypt patterns adapted for production

**CRITICAL**: Every asset requires ALL FOUR proofs (not split by type):
- **PoSpace (PoSp)**: WHERE - storage location and physical/network location
- **PoStake (PoSt)**: WHO - ownership, access rights, and economic stake
- **PoWork (PoWk)**: WHAT/HOW - computational resources and processing
- **PoTime (PoTm)**: WHEN - temporal ordering and timestamp validation

**Combined**: Unified "State Proof" answering WHERE/WHO/WHAT/WHEN for every block/asset

### **HyperMesh Asset System (⚡ 60% Implemented)**
**Location**: `/blockmatrix/src/assets/` (asset management library)
**Integration**: BlockMatrix (`/blockmatrix/`) orchestration layer

**Universal Requirements**:
- Everything in BlockMatrix is an Asset (containers, CPU, GPU, RAM, storage, services)
- ✅ Universal AssetId system with blockchain registration - IMPLEMENTED
- ✅ AssetAdapter trait for specialized handling - IMPLEMENTED
- ✅ Remote proxy addressing (NAT-like for memory) - FULLY IMPLEMENTED

**Asset Adapter Pattern** (✅ All Implemented):
```rust
// CPU Asset Adapter - PoWk validation, time-based scheduling
pub struct CpuAssetAdapter; // IMPLEMENTED

// GPU Asset Adapter - FALCON-1024 quantum security, NAT-like GPU memory
pub struct GpuAssetAdapter; // IMPLEMENTED

// Memory Asset Adapter - NAT-like memory addressing with proxy translation
pub struct MemoryAssetAdapter; // FULLY IMPLEMENTED (PoSp proofs, user controls)

// Storage Asset Adapter - Sharding, encryption, PoSp storage commitment
pub struct StorageAssetAdapter; // IMPLEMENTED (Kyber encryption, content-aware)

// Network & Container Adapters - Resource isolation and orchestration
pub struct NetworkAssetAdapter; // IMPLEMENTED
pub struct ContainerAssetAdapter; // IMPLEMENTED
```

**Asset allocation uses tensor operations on the Block-MATRIX:**
- CPU/GPU/Memory allocation calculated using matrix position and neighbor capabilities
- Tensor-based routing for optimal resource placement
- Geospatial awareness in allocation decisions

### **Privacy-Aware Resource Allocation (✅ Implemented)**
**Location**: `/blockmatrix/src/assets/privacy/`

**Privacy Allocation Types** (Protocol/Asset/Network levels):
- **Anonymous**: No identity tracking, privacy-first sharing
- **Private**: Internal network only, no external access
- **Federated**: Trusted network groups with selective sharing
- **Public**: Cross-network accessible, full discovery with Proof of State validation

**Privacy Levels**:
- `Private`: No public access
- `PrivateNetwork`: Specific networks/groups
- `P2P`: Trusted peer sharing
- `PublicNetwork`: Specific public networks  
- `FullPublic`: Maximum CAESAR rewards, full HyperMesh node

**User Controls** (All Mandatory):
- Resource allocation percentages (0-100% per resource type)
- Concurrent usage limits, rewards configuration, duration limits
- State proof requirements (which proofs: PoSp/PoSt/PoWk/PoTm)
- Remote proxy settings (NAT-like addressing preferences)

### **Remote Proxy/NAT System (✅ Implemented)**
**Location**: `/blockmatrix/src/assets/proxy/` (NAT translation and routing)

**Implementation Status**:
- ✅ **NAT-like addressing for memory/resources** - FULLY IMPLEMENTED
- ✅ Global proxy addresses (IPv6-like addressing for BlockMatrix ecosystem)
- ✅ Trust-based proxy selection using PoSt (Proof of Stake) validation
- ✅ Federated trust integration with TrustChain certificate hierarchy
- ✅ Sharded data access through encrypted/sharded pools
- ✅ User-configurable privacy-aware proxy selection and routing

**Key Files**:
- `/blockmatrix/src/assets/adapters/memory.rs` - NAT-like memory addressing (lines 48-96)
- `/blockmatrix/src/assets/proxy/nat_translation.rs` - Address translation logic
- `/blockmatrix/src/assets/proxy/routing.rs` - Proxy forwarding and selection

## Blockchain Architecture

### Dual-Scope Blockchain Architecture (TARGET ARCHITECTURE)

**BlockchainScope is binary**: Device (local) | Network (synced). PrivacyMode (Anonymous/Private/Public) handles all participation semantics independently at the transport layer.

#### Current Implementation Status: ~5-10%
**What EXISTS today**:
- ✅ Single blockchain per node (Device scope only)
- ✅ Multi-network participation (Anonymous/Private/Public networks via STOQ)
- ✅ Basic Proof of State validation (four proofs: PoSpace/PoStake/PoWork/PoTime)
- ✅ Asset system with blockchain registration
- ❌ Network scope sync (reflector/swarm mode NOT implemented)
- ✅ Gateway crate implemented (HTTP/3 + STOQ bridge, scope routing, federation, 20/20 features)
- ❌ Cross-network asset transfers (NOT implemented)

**Key File Status**:
- `/blockmatrix/src/blockchain/` - Single blockchain implementation (Device scope only)
- `/blockmatrix/src/proof_of_state/` - Basic PoS validation, no scope awareness

#### Two Blockchain Scope Types

1. **Device**: Single device, local-only blockchain. Always running from boot. No network required.
2. **Network**: Synchronized across participating nodes via reflector/swarm mode. Requires connectivity.

**Why only two**: PrivacyMode handles all participation semantics. A "Group" is a Private network with specific peers. An "Organization" is a bigger Private network. Sub-federation is nesting -- same protocol at every level. The blockchain either syncs with a network or it does not.

**Participation Model**:
- Node runs Device chain always (local, independent)
- Node optionally joins one or more Network chains (synced via reflector pooling)
- Each Network chain has independent Proof of State rules
- Gateway nodes bridge Device-to-Network and cross-network transfers
- PrivacyMode controls transport behavior independently of chain scope

**Per-Asset Distribution Policies**:
- Assets declare which chains they participate in (Device-only or Network-synced)
- PrivacyMode settings determine transport behavior and visibility
- Cross-network transfers require proof of state in both chains
- Matrix-topology-aware shard placement

#### Key Distinctions (Eliminate Confusion)

**PrivacyMode** (TRANSPORT layer via STOQ):
- Anonymous (open, untracked), Private (bounded, tracked), Public (open, tracked)
- Controls packet tracking and identity disclosure
- Independent of blockchain scope

**BlockchainScope** (STATE REPLICATION layer):
- Device (local-only), Network (synced across nodes)
- Controls whether chain state is replicated
- Independent of transport privacy

**Example Combinations**:
- Device chain + Anonymous transport = fully isolated, untraceable node
- Device chain + Private transport = local chain visible to bounded group
- Network chain + Anonymous transport = synced swarm, untraceable packets
- Network chain + Private transport = synced group with identity (family, company)
- Network chain + Public transport = open synced ledger, full transparency

#### Gateway Architecture (IMPLEMENTED — `/gateway/` crate, 100%)

**trust.hypermesh.online as Network Gateway** (4 roles):
1. **Clearnet Bootstrap**: HTTP/3 at trust.hypermesh.online for initial STOQ connection info + bootstrap tokens
2. **Clearnet Inbound Proxy**: HTTP/3 access to HyperMesh dashboards (resource dashboard, engauge panel)
3. **Outbound Proxy**: Bridge HyperMesh resources to non-HyperMesh clearnet endpoints
4. **Inter-Network Gateway**: Bridge between federated/private/public HyperMesh networks (STOQ-to-STOQ)

**Implementation**:
- Dual-listener: HTTP/3 (port 8443) + STOQ (port 8444)
- TLS via TlsProvider (File/TrustChain/SelfSigned certificate sources)
- PoS authentication + bootstrap token flow
- Cross-scope routing (Device↔Network) via ScopeRouter
- Federation bridge with trust levels (Full/Conditional/Untrusted)
- Rate limiting (token bucket per-IP/identity/global) + DDoS protection
- Load balancing (RoundRobin/LeastConnections/WeightedRoundRobin/HealthAware)
- Multi-domain SNI routing (*.hypermesh.online)
- Privacy-mode-aware forwarding (Anonymous/Private/Public)

**Gateway Nodes**:
- Bridge between Device and Network chains
- Validate cross-network transfers
- Maintain partial state from multiple Network chains
- Route based on matrix topology and network membership

#### Remote Access Model

**Blockchain State Replication**:
- Network scope: Nodes replicate shared blockchain state via reflector pooling
- Gateway caches recent blocks, routes queries to authoritative nodes
- Full replication for small networks, partial for large networks

**Resource Location vs Blockchain Access**:
- Blockchain access: Query gateway for block/transaction data
- Resource access: NAT traversal through proxy system (separate concern)
- Assets registered in blockchain, resources located via matrix topology

### Node Bootstrap Architecture (CURRENT + FUTURE)

#### Bootstrap Sequence (Resolves Chicken-and-Egg)

A circular dependency exists: STOQ connection → TLS cert → TrustChain CA → DNS → blockchain → STOQ connection. HyperMesh breaks this by separating encryption from authentication:

**Phase 0 — Bootstrap (IMPLEMENTED):**
- Node generates self-signed cert via local TrustChain (`NetworkType::P2P` → `local://trustchain`)
- QUIC/TLS provides encryption only (`AcceptAllVerifier` for non-Public strategies)
- After QUIC connects, nodes exchange flat JSON handshake with `pos_token` field
- Bilateral PoS validation is the trust mechanism (not TLS certificates)
- **Key**: PoS IS authentication. TLS IS encryption. They are independent.

**Phase 1 — Connected (IMPLEMENTED):**
- Nodes that pass PoS handshake are trusted peers
- DNS records registered as blockchain assets, propagated via block sync
- Blocks propagate to peers, shards distribute to matrix neighbors

**Phase 2 — Full PKI (FUTURE):**
- TrustChain-issued certs replace self-signed after PoS trust is established
- Reflector/gateway nodes become distributed CAs
- DNS resolution via blockchain instead of local HashMap

#### Local Blockchain Lifecycle
**Current Behavior (Single Blockchain)**:
- Local BlockMatrix blockchain begins with genesis block on boot
- No network connectivity required for blockchain to exist
- Blockchain runs independently of network participation mode
- Node is fully functional for localhost operations from moment of creation

**Future Behavior (Device + Network)**:
- Device chain starts immediately on boot (as today)
- Network chains joined after connectivity is established
- Node queries gateway/reflector to discover and join Network chains
- Synchronizes blockchain state for each joined Network chain via reflector pooling

### Node-as-DNS-Provider First
**Critical Difference from Traditional Systems:**
- Each node is its OWN DNS provider BEFORE network registration
- No upstream dependency (no 8.8.8.8 needed for local operations)
- Node bootstraps independently, THEN chooses to register with network
- For PUBLIC network: `trust.hypermesh.online` serves as global gateway

### DNS-as-Asset with Blockchain Registration
DNS registration is NOT a simple service - it's an ASSET requiring full Proof of State:
- **PoSpace (WHERE)**: Node's position in matrix + storage commitment
- **PoStake (WHO)**: Ownership, economic stake in the name
- **PoWork (WHAT)**: Computational proof of registration work
- **PoTime (WHEN)**: Temporal ordering, prevents replay attacks

DNS names are blockchain assets earning CAESAR rewards.

### **Domain/Namespace Goals**
**Target Resolution**:
- `http3://hypermesh` → HyperMesh global dashboard
- `http3://caesar` → Caesar wallet/exchange
- `http3://trust` → TrustChain management  
- `http3://assets` → HyperMesh asset management

**Domain-as-Network (IMPLEMENTED)**: Domain registration creates Network-scope blockchains. Hierarchical DNS resolution walks parent chains. Invitation tokens use BLAKE3-HMAC.

### **Catalog: Asset Package Manager with Execution Delegation (⚠️ 30% Complete)**
**Catalog Architecture**:
- ✅ **Pure Asset Package Manager**: Manages asset packages (definitions, versioning, distribution)
- ✅ **Execution Delegation Framework**: Delegates execution to HyperMesh infrastructure (no local VM)
- ✅ **Asset SDK**: Plugin development and asset creation tools complete
- ✅ **Syntax Validation**: Julia/Lua/WASM syntax validation (not execution)
- ✅ **HyperMesh Integration**: Resource allocation through Asset Adapters
- ⚡ **State Proof Validation**: Integration in progress
- ✅ **Network Address**: catalog.hypermesh.online (via TrustChain DNS)

**Execution Model**:
- Catalog packages assets with metadata and resource requirements
- Asset execution happens on remote HyperMesh nodes (not locally)
- Resources allocated through BlockMatrix Asset Adapters (CPU/GPU/Memory/Storage)
- TrustChain certificate-based security for network operations

## STOQ: Intelligent Protocol, Not Just Transport

**STOQ is NOT just QUIC wrapper** - it provides protocol-level intelligence:
- **PoS Token Validation**: Validates Proof of State at protocol layer (not application)
- **Asset Hash Validation**: Verifies content integrity at protocol layer
- **Shard Addressing**: Provides matrix positions for shard placement
- **Privacy Tier Enforcement**: Different behavior for Anonymous vs Public connections
- **Protocol-Level Routing**: Smart routing decisions based on matrix topology

## Network Scope Implementation Roadmap

### Phase 1: Network Sync (MVP Target)
**Goal**: Enable nodes to join and synchronize a shared Network chain
- Implement Network scope with reflector/swarm sync
- Reflector pooling for chain state propagation
- Use case: User's devices sharing resources via Private network
- Use case: Friends/family group via Private network with specific peers

**Files to Create**:
- `/blockmatrix/src/blockchain_scope.rs` - Device | Network scope abstraction
- `/blockmatrix/src/gateway/scope_bridge.rs` - Device-to-Network bridging

### Phase 2: Cross-Network Operations
**Goal**: Enable transfers and interactions across Network chains
- Cross-network asset transfers with dual proof of state
- NAT traversal for behind-firewall devices
- TrustChain certificate-based cross-network trust
- CAESAR reward distribution for public Network participation

### Phase 3: Nested Networks
**Goal**: Sub-federation support via nested Private networks
- Nested Network chains (same protocol at every level)
- Hierarchical routing between nested networks
- Gateway nodes bridge parent/child networks
- Use case: Department networks within organization network

## Four Privacy Tiers (Network-Level Behavior)

| Tier | Validation | Signing | Tracking | Rewards |
|------|-----------|---------|----------|---------|
| **Anonymous** | None | No | No | None |
| **Private P2P** | Peer-only | Optional | Minimal | Low |
| **Federated** | Network-level | Yes | Network-only | Medium |
| **Public** | Full PoS | Yes | Full transparency | Maximum |

## Privacy Flexibility Matrix (CRITICAL UNDERSTANDING)

**Network layer (transport) is COMPLETELY INDEPENDENT from blockchain layer (state replication):**

**Transport Layer** (STOQ PrivacyMode):
- Anonymous (open, untracked), Private (bounded, tracked), Public (open, tracked)
- Controls packet tracking and communication privacy

**State Replication Layer** (BlockchainScope):
- Device (local-only), Network (synced)
- Controls whether chain state is replicated across nodes

**Example Combinations**:
- **Device chain + Anonymous transport** = Fully isolated, untraceable node
- **Device chain + Private transport** = Local chain visible to bounded group
- **Network chain + Anonymous transport** = Synced swarm, untraceable packets
- **Network chain + Private transport** = Synced group with identity (family, company)
- **Network chain + Public transport** = Open synced ledger, full transparency

**Real-world example (Current + Future)**:
- **Today**: Single device runs Device chain over any PrivacyMode
- **Phase 1**: User's devices join Network chain, communicate over Anonymous STOQ transport
- **Result**: Complete privacy (synced chain + untraceable packets), no external entity can see blockchain OR communication

## Revolutionary Distribution: Instruction-Based Retrieval

**Traditional**: Send raw data → Receiver
**Block-MATRIX**: Send instructions → Receiver queries matrix positions → Gets shards → Reconstructs

**Benefits**:
- Bandwidth efficiency (send KB instructions, not GB files)
- Distributed load (receiver pulls from multiple matrix positions)
- Resilience (if one node fails, query other positions)
- Deduplication (shared shards referenced once, used many times)

## Data Processing Pipeline (EXACT ORDER)

**Compression → Encryption → Sharding → Distribution**

1. **Compression First**: Brotli streaming compression (levels 1-11) on raw data
2. **Encryption Second**: Kyber-1024 quantum-resistant encryption of the compressed blob (NOT per-shard, NOT AES wrapping)
3. **Sharding Third**: Reed-Solomon erasure coding (10+4) splits the encrypted blob into matrix-aware shards
4. **Distribution Fourth**: Tensor-based placement at calculated matrix positions

**Current Implementation GAP**: Code currently does Compress→Shard→Encrypt (wrong order, per-shard AES-256-GCM). Needs reorder to Compress→Encrypt→Shard and replacement of AES-256-GCM with Kyber-1024. Brotli compression is correct. Reed-Solomon sharding is correct. FALCON-1024 is for STOQ protocol signing. Kyber-1024 is for asset encryption.

**Bucket Deduplication**: Hash buckets mapped to matrix positions prevent duplicate storage while maintaining redundancy through matrix topology.

## HyperMesh Coordination Intelligence

**Matrix-Aware Shard Distribution**:
- Shards placed based on matrix topology (x,y,z) calculations
- Tensor operations determine optimal placement
- Geographic and network proximity considered
- Load balancing through matrix mathematics
- Self-healing through matrix neighbor discovery

---

## 🎯 **Next Actions (Context for Resumption)**

### **Immediate Priority (Network Scope Implementation)**
1. **BlockchainScope Abstraction**: Device | Network scope (`/blockmatrix/src/blockchain_scope.rs`)
2. **Network Sync MVP**: Reflector/swarm mode for Network chain synchronization (Phase 1)
3. **Cross-Network Asset Transfers**: Dual proof-of-state validation across scopes
4. **Integration Testing**: End-to-end workflow validation across components
5. **Performance Optimization**: STOQ transport tuning (2.95 Gbps → adaptive tiers)
6. ✅ **IPC Architecture**: JSON-RPC 2.0 daemon with Unix socket server/client — COMPLETE
7. ✅ **Domain Naming**: Domain-as-network registration, hierarchy, BLAKE3-HMAC invitations — COMPLETE
8. ✅ **Dashboard SDK**: Scope-aware dashboard serving, manifest system, default dashboards — COMPLETE

### **Key Files for Development**

**Network Scope Blockchain (TO BE CREATED)**:
- `/blockmatrix/src/blockchain_scope.rs` - Device | Network scope abstraction
- `/blockmatrix/src/gateway/scope_bridge.rs` - Device-to-Network bridging

**Existing Implementation**:
- `/blockmatrix/src/blockchain/` - Single blockchain implementation (Device scope only)
- `/blockmatrix/src/proof_of_state/` - State proof orchestration (needs scope awareness)
- `/trustchain/src/proof_of_state/` - Core Proof of State implementation (primary)
- `/blockmatrix/src/assets/adapters/` - Asset adapters (CPU/GPU/Memory/Storage/Network/Container)
- `/blockmatrix/src/assets/proxy/` - Remote proxy/NAT system
- `/stoq/src/transport/mod.rs` - QUIC transport with eBPF
- `/trustchain/` - FALCON-1024 CA (production-ready)
- `/catalog/` - Asset package manager (compiles with warnings)
- `/gateway/` - HTTP/3 + STOQ gateway (100% complete, 20 modules, 194 tests)
- `/blockmatrix/src/ipc/` - IPC daemon, client, protocol, config, handler modules
- `/blockmatrix/src/dashboard/` - Dashboard manifest, default content, scaffold
- `/blockmatrix/src/dns/domain.rs` - Domain-as-network registration
- `/blockmatrix/src/dns/invitation.rs` - BLAKE3-HMAC invitation tokens
- `/hypermesh-sdk/` - Typed Rust SDK for daemon IPC API
- `/gateway/src/dashboard_server.rs` - Scope-aware dashboard serving

### **Architecture Decisions Made**

**Core Architecture** (Implemented):
- ✅ **Block-MATRIX Topology**: Each node is matrix cell (x,y,z) with tensor operations
- ✅ **Node-as-DNS-Provider First**: Self-sufficient bootstrap, no upstream dependency
- ✅ **DNS-as-Asset**: Requires full Proof of State, blockchain-registered
- ✅ **STOQ Protocol Intelligence**: PoS validation, shard addressing at protocol layer
- ⚡ **Compression→Encryption→Sharding→Distribution**: Whole-blob Kyber-1024 encryption before sharding (code currently wrong order + AES-256-GCM, needs fix)
- ✅ **Instruction-Based Retrieval**: Send maps not files
- ✅ **Matrix-Aware Coordination**: Tensor operations for resource allocation
- ✅ Separate protocols (TrustChain, STOQ, Catalog) from BlockMatrix
- ✅ Everything is a BlockMatrix Asset with remote NAT-like addressing
- ✅ IPv6-only networking throughout ecosystem
- ✅ Four-proof Proof of State (PoSpace, PoStake, PoWork, PoTime) for all operations
- ✅ Quantum-resistant cryptography (FALCON-1024, Kyber)

**Blockchain Architecture** (Clarified):
- ✅ **Current State**: Single blockchain per node (Device scope only)
- ✅ **Target State**: Device (local) + Network (synced) dual-scope model
- ✅ **Layer Separation**: PrivacyMode (transport) ≠ BlockchainScope (state replication)
- ❌ **Network Scope Sync**: Reflector/swarm mode NOT yet implemented
- ❌ **BlockchainScope Abstraction**: Device | Network pending

**Privacy Architecture** (Clarified):
- ✅ **Three PrivacyModes**: Anonymous | Private | Public (STOQ transport layer)
- ✅ **Two Blockchain Scopes**: Device | Network (state replication layer)
- ✅ **Privacy Flexibility Matrix**: PrivacyMode ≠ BlockchainScope (independent dimensions)
- ✅ **Transport + State Replication Independence**: Any PrivacyMode can carry any BlockchainScope

---

### **Removed Components & Features**
- ❌ **Julia Language Support**: REMOVED - Execution delegation replaces local VM need
- ❌ **Traditional Databases**: REMOVED - All storage is asset-based through BlockMatrix
- ❌ **RSA Cryptography**: REMOVED - FALCON-1024 for protocol, Kyber for asset encryption
- ❌ **HTTP/REST APIs**: REMOVED - Everything runs through STOQ protocol
- ❌ **Lua VM Integration**: REMOVED - Remote execution on HyperMesh nodes only

**Current Phase**: Foundation development with 5-10% implementation complete
**Next Milestone**: Implement Network scope blockchain sync (reflector/swarm mode)

**Critical Understanding**:
- **Current State**: Single blockchain per node (Device scope only), Network scope not yet implemented
- **PrivacyMode vs BlockchainScope**: PrivacyMode (Anonymous/Private/Public) is TRANSPORT layer, BlockchainScope (Device/Network) is STATE REPLICATION layer
- **Layer Independence**: PrivacyMode and BlockchainScope are independent dimensions
- **Target Architecture**: Nodes run Device chain always + optionally join Network chains via reflector pooling
- The Block-MATRIX topology IS the trust mechanism - position in matrix determines trust relationships
- STOQ provides protocol-level intelligence, not just transport
- Everything runs through STOQ - no HTTP, no traditional networking
- Matrix operations (tensor math) drive all routing and resource decisions

---

## 📊 **Crate Status Tracking (Single Source of Truth)**

### How It Works
Each Rust crate has a `crate-status.toml` file that is the **single source of truth** for feature status. A sync script reads these files and generates TypeScript data files for the website.

### Files
- `<crate>/crate-status.toml` - Feature status per crate (8 crates)
- `scripts/sync-status.ts` - Reads toml files, counts code metrics, generates output
- `scripts/sync-status.sh` - Shell wrapper for the sync script
- `scripts/output/status.ts` - Auto-generated: feature status per crate
- `scripts/output/stats.ts` - Auto-generated: code metrics per crate (files, lines, tests)

### Managing Feature Status

When a feature changes status, update the relevant `crate-status.toml`:

```toml
[crate]
id = "stoq"
name = "STOQ Protocol"
description = "..."
phase = "alpha"  # planning | alpha | beta | stable

[features.working]
items = [
    "Feature that works",
]

[features.in_development]
items = [
    "Feature being built",
]

[features.planned]
items = [
    "Feature not started",
]
```

Note: `completion` percentage is auto-computed from `working / total * 100`.

### Workflow
1. **Feature completed**: Move from `in_development` to `working` in the crate's toml
2. **New feature started**: Move from `planned` to `in_development`
3. **New feature identified**: Add to `planned`
4. **Update phase**: Change `phase` field as crate matures
5. **Run sync**: `./scripts/sync-status.sh` (also runs automatically on git push)

### Auto-Sync
A git pre-push hook automatically runs the sync script and amends the commit with updated output files. The website at `../public/` imports these generated files directly.

### Rules
- **NEVER edit** `scripts/output/status.ts` or `scripts/output/stats.ts` directly
- **ALWAYS edit** the `crate-status.toml` in the relevant crate directory
- Keep feature descriptions concise (one line each)
- Update `completion` percentage when significant progress is made