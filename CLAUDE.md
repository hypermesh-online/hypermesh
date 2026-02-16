# Web3 Ecosystem - Development Project Context

## 🎯 **Current Status: ~5-10% Implemented, Core Architecture Phase**

**Development Status**: ⚠️ **EARLY DEVELOPMENT** - Core components in initial implementation phase
**Repository Status**: ✅ **SEPARATED** - 6 repositories at github.com/hypermesh-online/
**Implementation Status**: ⚠️ **FOUNDATIONAL PHASE** - Basic blockchain and asset system partially operational, multi-scope architecture pending

---

## 📦 **Repository Architecture**

### **GitHub Organization**: [hypermesh-online](https://github.com/hypermesh-online)

| Component | Repository | Status | Notes |
|-----------|------------|--------|-------|
| **NGauge** | `/ngauge` | 🚧 Planning | Engagement platform concept |
| **Caesar** | `/caesar` | ⚡ **40% Complete** | HTTP→STOQ migration in progress |
| **Catalog** | `/catalog` | ⚡ **30% Complete** | Asset package registry/template library ONLY - NOT asset manager |
| **BlockMatrix** | `/blockmatrix` | ⚠️ **10% Complete** | Single blockchain per node, multi-scope pending |
| **STOQ** | `/stoq` | ✅ **92% Complete** | QUIC transport with eBPF integration |
| **TrustChain** | `/trustchain` | ✅ **95% Complete** | FALCON-1024 CA production-ready |

### Critical Architectural Note: Block-MATRIX Topology
All components operate within a Block-MATRIX network where each node is a cell in a geospatial matrix (x,y,z coordinates). This enables:
- **Tensor Operations**: Mathematical matrix operations for routing and resource allocation
- **Multi-Scope Blockchain Participation**: Nodes participate in multiple blockchain scopes simultaneously (target architecture)
- **Matrix-Aware Coordination**: Intelligent shard distribution based on topology
- **Network Independence**: Local blockchain runs regardless of network connectivity

**CURRENT STATE**: Single blockchain per node (5-10% implemented)
**TARGET STATE**: Multi-scope blockchain participation (see "Blockchain Architecture" section below)

### **Repository Sync Commands**
```bash
scripts/deploy/sync-repos.sh              # Sync all components
scripts/deploy/sync-repos.sh stoq         # Sync specific component
scripts/deploy/sync-repos.sh --dry-run    # Preview changes
scripts/deploy/deploy-all.sh              # One-command deployment
```

---

## 🔧 **Critical Gaps (Next Priority)**

### **1. Multi-Scope Blockchain Implementation**
- ❌ BlockchainScope abstraction (foundational architecture)
- ❌ User Scope implementation (Phase 1 MVP)
- ❌ Gateway architecture for scope bridging
- ❌ Cross-scope asset transfers
- ❌ Scope-aware consensus rules

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
- ⚡ Multi-node consensus finality testing
- ⚡ Real-world stress testing and profiling
- ⚡ Production deployment hardening

---

## 📋 **Core Architecture (Technical Reference)**

### **Proof of State Four-Proof Consensus System (⚡ 50% Implemented)**
**Location**: `/trustchain/src/consensus/` (primary implementation, re-exported by BlockMatrix)
**Secondary**: `/blockmatrix/src/consensus/` (BlockMatrix-specific consensus orchestration)
**Reference**: Original NKrypt patterns adapted for production

**CRITICAL**: Every asset requires ALL FOUR proofs (not split by type):
- **PoSpace (PoSp)**: WHERE - storage location and physical/network location
- **PoStake (PoSt)**: WHO - ownership, access rights, and economic stake  
- **PoWork (PoWk)**: WHAT/HOW - computational resources and processing
- **PoTime (PoTm)**: WHEN - temporal ordering and timestamp validation

**Combined**: Unified "Consensus Proof" answering WHERE/WHO/WHAT/WHEN for every block/asset

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
- **Public**: Cross-network accessible, full discovery with consensus validation

**Privacy Levels**:
- `Private`: No public access
- `PrivateNetwork`: Specific networks/groups
- `P2P`: Trusted peer sharing
- `PublicNetwork`: Specific public networks  
- `FullPublic`: Maximum CAESAR rewards, full HyperMesh node

**User Controls** (All Mandatory):
- Resource allocation percentages (0-100% per resource type)
- Concurrent usage limits, rewards configuration, duration limits
- Consensus requirements (which proofs: PoSp/PoSt/PoWk/PoTm)
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

### Multi-Scope Blockchain Participation (TARGET ARCHITECTURE)

**CRITICAL DISTINCTION**: Network privacy tiers (Anonymous/P2P/Federated/Public) are TRANSPORT layer concerns. Blockchain scopes are CONSENSUS layer concerns. These are independent dimensions.

#### Current Implementation Status: ~5-10%
**What EXISTS today**:
- ✅ Single blockchain per node (local Device scope only)
- ✅ Multi-network participation (Anonymous/P2P/Federated/Public networks via STOQ)
- ✅ Basic Proof of State consensus (four proofs: PoSpace/PoStake/PoWork/PoTime)
- ✅ Asset system with blockchain registration
- ❌ Multi-blockchain-per-node capability (NOT implemented)
- ❌ BlockchainScope abstraction (does NOT exist)
- ❌ Gateway architecture for scope bridging (NOT implemented)
- ❌ Cross-scope transfers (NOT implemented)

**Key File Status**:
- `/blockmatrix/src/blockchain/` - Single blockchain implementation (Device scope only)
- `/blockmatrix/src/consensus/` - Basic PoS consensus, no scope awareness
- No `blockchain_scope.rs` or similar scope management files exist

#### Target Multi-Scope Architecture (FUTURE VISION)

**Six Blockchain Scope Types**:
1. **Device Scope**: Single device, local-only blockchain (CURRENT STATE)
2. **User Scope**: User's devices share blockchain (PHASE 1 TARGET)
3. **Group Scope**: Small trusted groups (friends, family)
4. **Organization Scope**: Companies, teams, departments
5. **Federation Scope**: Multi-org collaboration networks
6. **Public Scope**: Global public blockchain (trust.hypermesh.online)

**Multi-Blockchain Participation Model**:
- Node participates in multiple blockchain scopes simultaneously
- Each scope has independent blockchain with own consensus rules
- Pluggable consensus per scope (Byzantine for Device/User, PoS for Public)
- Gateway nodes bridge scopes, route cross-scope transfers
- Hierarchical routing between scopes

**Per-Asset Distribution Policies**:
- Assets declare which scopes they participate in
- Privacy settings determine scope visibility
- Cross-scope transfers require proof of state in both scopes
- Scope-aware shard placement based on matrix topology

#### Key Distinctions (Eliminate Confusion)

**Network Privacy Tiers** (TRANSPORT layer via STOQ):
- Anonymous, P2P, Federated, Public
- Controls packet tracking and identity disclosure
- Independent of blockchain scope

**Blockchain Scopes** (CONSENSUS layer):
- Device, User, Group, Org, Federation, Public
- Controls who participates in consensus
- Independent of network privacy

**Example Combinations**:
- User Scope blockchain over Anonymous network = Private family devices, untraceable packets
- Public Scope blockchain over Federated network = Open ledger, controlled network access
- Group Scope blockchain over Public network = Friends-only consensus, tracked routing

#### Gateway Architecture (FUTURE)

**trust.hypermesh.online as Public Scope Gateway**:
- Entry point for Public Scope blockchain participation
- Routes requests to appropriate scope gateways
- NAT traversal for devices behind firewalls
- Blockchain state replication vs resource location distinction

**Gateway Nodes**:
- Bridge between blockchain scopes
- Validate cross-scope transfers
- Maintain partial state from multiple scopes
- Route based on matrix topology and scope membership

#### Remote Access Model

**Blockchain State Replication**:
- User Scope: Devices replicate shared blockchain state
- Gateway caches recent blocks, routes queries to authoritative nodes
- Full replication for small scopes, partial for large scopes

**Resource Location vs Blockchain Access**:
- Blockchain access: Query gateway for block/transaction data
- Resource access: NAT traversal through proxy system (separate concern)
- Assets registered in blockchain, resources located via matrix topology

### Node Bootstrap Architecture (CURRENT + FUTURE)

#### Local Blockchain Lifecycle
**Current Behavior (Single Blockchain)**:
- Local BlockMatrix blockchain begins with genesis block on boot
- No network connectivity required for blockchain to exist
- Blockchain runs independently of network participation mode
- Node is fully functional for localhost operations from moment of creation

**Future Behavior (Multi-Scope)**:
- Device Scope blockchain starts immediately on boot (as today)
- User/Group/Org/Federation/Public scopes joined after network connection
- Node queries gateway to discover and join appropriate scopes
- Synchronizes blockchain state for each joined scope

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

### **Catalog: Asset Package Manager with Execution Delegation (⚠️ 30% Complete)**
**Catalog Architecture**:
- ✅ **Pure Asset Package Manager**: Manages asset packages (definitions, versioning, distribution)
- ✅ **Execution Delegation Framework**: Delegates execution to HyperMesh infrastructure (no local VM)
- ✅ **Asset SDK**: Plugin development and asset creation tools complete
- ✅ **Syntax Validation**: Julia/Lua/WASM syntax validation (not execution)
- ✅ **HyperMesh Integration**: Resource allocation through Asset Adapters
- ⚡ **Consensus Proof Validation**: Integration in progress
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

## Multi-Scope Implementation Roadmap

### Phase 1: User Scope (MVP Target)
**Goal**: Enable user's devices to share blockchain
- Implement BlockchainScope abstraction
- Device Scope → User Scope migration
- Shared blockchain state across user's devices
- Private federated system isolated from public network
- Use case: Personal cloud with all devices sharing resources

**Files to Create**:
- `/blockmatrix/src/blockchain_scope.rs` - Scope abstraction and management
- `/blockmatrix/src/scope/user.rs` - User scope implementation
- `/blockmatrix/src/scope/device.rs` - Refactor existing blockchain to Device scope
- `/blockmatrix/src/gateway/scope_bridge.rs` - Cross-scope communication

### Phase 2: Group/Organization Scopes
**Goal**: Small trusted groups and organizational networks
- Group Scope for friends/family trusted networks
- Organization Scope for companies and teams
- Federated trust integration with TrustChain
- Scope-aware consensus rules

### Phase 3: Public Scope Participation
**Goal**: Integration with trust.hypermesh.online gateway
- Public Scope blockchain via global gateway
- NAT traversal for behind-firewall devices
- Cross-scope asset transfers
- CAESAR reward distribution for public participation

### Phase 4: Flexible Topology
**Goal**: Configurable flat/nested/hybrid scope architectures
- Flat: All scopes at same level
- Nested: Hierarchical scopes (Device → User → Group → Org → Federation → Public)
- Hybrid: Mix of flat and nested based on use case

## Four Privacy Tiers (Network-Level Behavior)

| Tier | Validation | Signing | Tracking | Rewards |
|------|-----------|---------|----------|---------|
| **Anonymous** | None | No | No | None |
| **Private P2P** | Peer-only | Optional | Minimal | Low |
| **Federated** | Network-level | Yes | Network-only | Medium |
| **Public** | Full PoS | Yes | Full transparency | Maximum |

## Privacy Flexibility Matrix (CRITICAL UNDERSTANDING)

**Network layer (transport) is COMPLETELY INDEPENDENT from blockchain layer (consensus):**

**Transport Layer** (STOQ network privacy tiers):
- Anonymous, P2P, Federated, Public
- Controls packet tracking and communication privacy

**Consensus Layer** (Blockchain scopes):
- Device, User, Group, Org, Federation, Public
- Controls who participates in blockchain consensus

**Example Combinations**:
- **User Scope blockchain on Anonymous network** = Family devices sharing private blockchain, untraceable packets
- **Public Scope blockchain on Federated network** = Open ledger accessible to world, controlled network membership
- **Group Scope blockchain on Public network** = Friends-only consensus with full packet tracking
- **Device Scope blockchain on Public network** = Single device, full public participation

**Real-world example (Current + Future)**:
- **Today**: Single device runs Device Scope blockchain over any network privacy tier
- **Phase 1**: User's devices share User Scope blockchain, communicate over Anonymous STOQ network
- **Result**: Complete privacy (private consensus + untraceable packets), no external entity can see blockchain OR communication

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

### **Immediate Priority (Multi-Scope Implementation)**
1. **BlockchainScope Abstraction**: Core multi-scope architecture (`/blockmatrix/src/blockchain_scope.rs`)
2. **User Scope MVP**: Enable user's devices to share blockchain (Phase 1)
3. **Gateway Architecture**: Cross-scope communication and bridging
4. **Integration Testing**: End-to-end workflow validation across components
5. **Performance Optimization**: STOQ transport tuning (2.95 Gbps → adaptive tiers)

### **Key Files for Development**

**Multi-Scope Blockchain (TO BE CREATED)**:
- `/blockmatrix/src/blockchain_scope.rs` - Scope abstraction and management
- `/blockmatrix/src/scope/device.rs` - Device scope (refactor existing blockchain)
- `/blockmatrix/src/scope/user.rs` - User scope implementation
- `/blockmatrix/src/scope/group.rs` - Group scope implementation
- `/blockmatrix/src/scope/org.rs` - Organization scope implementation
- `/blockmatrix/src/scope/federation.rs` - Federation scope implementation
- `/blockmatrix/src/scope/public.rs` - Public scope implementation
- `/blockmatrix/src/gateway/scope_bridge.rs` - Cross-scope communication

**Existing Implementation**:
- `/blockmatrix/src/blockchain/` - Single blockchain implementation (Device scope only)
- `/blockmatrix/src/consensus/` - Consensus orchestration (needs scope awareness)
- `/trustchain/src/consensus/` - Core Proof of State implementation (primary)
- `/blockmatrix/src/assets/adapters/` - Asset adapters (CPU/GPU/Memory/Storage/Network/Container)
- `/blockmatrix/src/assets/proxy/` - Remote proxy/NAT system
- `/stoq/src/transport/mod.rs` - QUIC transport with eBPF
- `/trustchain/` - FALCON-1024 CA (production-ready)
- `/catalog/` - Asset package manager (compiles with warnings)

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
- ✅ Four-proof consensus (PoSpace, PoStake, PoWork, PoTime) for all operations
- ✅ Quantum-resistant cryptography (FALCON-1024, Kyber)

**Blockchain Architecture** (Clarified):
- ✅ **Current State**: Single blockchain per node (Device Scope only)
- ✅ **Target State**: Multi-scope blockchain participation (6 scopes)
- ✅ **Layer Separation**: Network privacy (transport) ≠ Blockchain scope (consensus)
- ❌ **Multi-Blockchain-Per-Node**: NOT yet implemented (5-10% complete)
- ❌ **BlockchainScope Abstraction**: Foundational architecture pending

**Privacy Architecture** (Clarified):
- ✅ **Four Network Privacy Tiers**: Anonymous | Private P2P | Federated | Public (STOQ transport layer)
- ✅ **Six Blockchain Scopes**: Device | User | Group | Org | Federation | Public (consensus layer)
- ✅ **Privacy Flexibility Matrix**: Network privacy ≠ Blockchain scope (independent dimensions)
- ✅ **Transport + Consensus Independence**: Any network tier can carry any blockchain scope

---

### **Removed Components & Features**
- ❌ **Julia Language Support**: REMOVED - Execution delegation replaces local VM need
- ❌ **Traditional Databases**: REMOVED - All storage is asset-based through BlockMatrix
- ❌ **RSA Cryptography**: REMOVED - FALCON-1024 for protocol, Kyber for asset encryption
- ❌ **HTTP/REST APIs**: REMOVED - Everything runs through STOQ protocol
- ❌ **Lua VM Integration**: REMOVED - Remote execution on HyperMesh nodes only

**Current Phase**: Foundation development with 5-10% implementation complete
**Next Milestone**: Implement multi-scope blockchain architecture (BlockchainScope abstraction + User Scope MVP)

**Critical Understanding**:
- **Current State**: Single blockchain per node (Device Scope only), NOT multi-scope
- **Network vs Blockchain**: Privacy tiers (Anonymous/P2P/Federated/Public) are TRANSPORT layer, Blockchain scopes (Device/User/Group/Org/Federation/Public) are CONSENSUS layer
- **Layer Independence**: Network privacy and blockchain scope are independent dimensions
- **Target Architecture**: Nodes will participate in multiple blockchain scopes simultaneously
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