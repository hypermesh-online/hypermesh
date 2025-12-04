# Intelligence Layer Architecture: STOQ Protocol + eBPF + HyperMesh

## Executive Summary

This document defines the **separation of concerns** between three critical layers:
1. **STOQ Protocol** - Standalone transport (like TCP/TLS)
2. **eBPF Layer** - Kernel-level intelligence enforcement
3. **HyperMesh** - Application-level orchestration with intelligence policies

**Key Principle:** STOQ remains a dumb transport protocol that can become an internet standard, while eBPF provides programmable intelligence injection, and HyperMesh defines the intelligence policies.

---

## 1. The Three-Layer Architecture

```
┌─────────────────────────────────────────────┐
│   Layer 3: HyperMesh (Application)          │
│   - Defines intelligence POLICIES            │
│   - Matrix topology, blockchain             │
│   - Proof of State validation logic         │
│   - Asset management, privacy tiers         │
└──────────────────┬──────────────────────────┘
                   │ Policy Configuration
                   │ (via eBPF maps)
                   ↓
┌─────────────────────────────────────────────┐
│   Layer 2: eBPF (Kernel-Level Bridge)       │
│   - Enforces intelligence at kernel level   │
│   - Packet inspection/filtering             │
│   - Protocol validation hooks               │
│   - Performance optimization (zero-copy)    │
└──────────────────┬──────────────────────────┘
                   │ Raw Packets
                   │ (with extension headers)
                   ↓
┌─────────────────────────────────────────────┐
│   Layer 1: STOQ Protocol (Transport)        │
│   - DUMB reliable transport                 │
│   - QUIC over IPv6                          │
│   - Certificate validation (TLS-style)      │
│   - Generic extension header support        │
│   - NO knowledge of PoS, matrix, etc.       │
└─────────────────────────────────────────────┘
```

---

## 2. Design Principles

### 2.1 STOQ Protocol: Standalone Transport

**STOQ must be like TCP/TLS:**

✅ **What STOQ Knows:**
- Packet structure and framing
- QUIC mechanics (reliability, flow control, congestion)
- Certificate-based authentication
- Extension header structure (NOT semantics)
- Connection lifecycle management

❌ **What STOQ Does NOT Know:**
- Proof of State (PoS) **semantics** - Provides hooks, applications define validation logic
- Asset hash **interpretation** - Carries extension headers, applications verify hashes
- Matrix topology **semantics** - Applications define routing, STOQ executes routes
- HyperMesh-specific **business logic** - Validation policies defined by applications

**Key Distinction**: STOQ provides **mechanisms** (extension headers, validation hooks, eBPF acceleration), HyperMesh provides **policies** (what to validate, how to interpret extensions).

**Goal:** STOQ should be usable by ANY application, not just HyperMesh. It could become an RFC-standardized protocol.

### 2.2 eBPF Layer: Programmable Intelligence

**eBPF is the intelligence injection point:**

✅ **eBPF Responsibilities:**
- Intercept STOQ packets at kernel level
- Read HyperMesh policies from userspace (via eBPF maps)
- Validate packets according to policies
- Drop invalid packets BEFORE userspace sees them
- Provide zero-copy, microsecond-level performance

✅ **eBPF Benefits:**
- Security: Malicious packets never reach userspace
- Performance: Kernel-level filtering (10-100x faster)
- Flexibility: Policies can be updated without restarting

### 2.3 HyperMesh: Intelligence Policy Definition

**HyperMesh defines WHAT to validate:**

✅ **HyperMesh Responsibilities:**
- Define validation policies (e.g., "Connection X requires PoS")
- Implement validation logic in eBPF programs
- Load policies into eBPF maps
- Use STOQ as dumb transport
- Remain completely decoupled from STOQ protocol

### 2.4 Current Implementation Reality Check

**STOQ eBPF Status**: STOQ already has eBPF transport acceleration at `/stoq/src/transport/ebpf/`:
- **xdp.rs** (5,769 bytes) - XDP packet filtering for performance
- **af_xdp.rs** (7,042 bytes) - AF_XDP zero-copy socket implementation
- **metrics.rs** (14,333 bytes) - Kernel-level performance metrics collection
- **loader.rs** (10,077 bytes) - Generic eBPF program loading infrastructure
- **mod.rs** (9,407 bytes) - eBPF capabilities detection and management

**Extension Headers Status**: Already implemented in `/stoq/src/extensions.rs` (458 lines):
- `PacketToken` - Packet tokenization support
- `PacketShard` - Sharding metadata
- `HopInfo` - Multi-hop routing information
- `SeedInfo` - Seeding and mirroring metadata
- Generic `ExtensionHeader` structure matches architecture specification

**Certificate Validation**: `/stoq/src/transport/certificates.rs` includes references to "Proof of State consensus proof validation"

**Implication**: Phase 2 must **refactor** existing code, not create from scratch. The architecture defines the TARGET state and migration path from CURRENT state.

---

## 3. STOQ Protocol Extension Headers

### 3.1 Generic Extension Mechanism

STOQ supports **extensible headers** without interpreting them:

```rust
// STOQ packet structure (stoq/src/protocol/packet.rs)
pub struct StoqPacket {
    // Standard STOQ headers (protocol-defined)
    pub version: u8,                // Protocol version
    pub connection_id: u64,         // Connection identifier
    pub stream_id: u64,             // Stream within connection
    pub flags: u16,                 // Protocol flags
    pub payload_length: u32,        // Payload size in bytes

    // Extension headers (application-defined, STOQ doesn't interpret)
    pub extensions: Vec<ExtensionHeader>,

    // Actual payload
    pub payload: Vec<u8>,
}

// Generic extension header (STOQ knows structure, not semantics)
pub struct ExtensionHeader {
    pub extension_type: u16,        // Application-defined type ID
    pub length: u16,                // Extension data length
    pub data: Vec<u8>,              // Raw extension data (STOQ doesn't parse!)
}
```

**Key Point:** STOQ treats extensions as opaque byte blobs. Only applications (like HyperMesh) interpret them.

### 3.2 HyperMesh Extension Types

HyperMesh defines specific extension types (NOT in STOQ codebase):

```rust
// blockmatrix/src/transport/hypermesh_extensions.rs
pub const EXT_PROOF_OF_STATE: u16 = 0x1000;
pub const EXT_ASSET_HASH: u16 = 0x1001;
pub const EXT_MATRIX_ROUTING: u16 = 0x1002;
pub const EXT_PRIVACY_TIER: u16 = 0x1003;

// Proof of State extension (WHO/WHAT/WHEN/WHERE)
pub struct ProofOfStateHeader {
    pub who: [u8; 32],      // Proof of Stake (identity)
    pub what: [u8; 32],     // Proof of Work (computation)
    pub when: u64,          // Proof of Time (timestamp)
    pub where_: [u8; 16],   // Proof of Space (matrix position)
}

// Asset Hash extension
pub struct AssetHashHeader {
    pub asset_id: [u8; 32],     // Asset identifier
    pub hash: [u8; 32],         // Content hash (BLAKE3)
    pub shard_count: u32,       // Number of shards
}

// Matrix Routing extension
pub struct MatrixRoutingHeader {
    pub source: MatrixCoordinate,       // Origin node position
    pub destination: MatrixCoordinate,  // Target node position
    pub path: Vec<MatrixCoordinate>,    // Routing path through matrix
}
```

---

## 4. Intelligence Flow: End-to-End

### Step-by-Step Packet Flow

```
┌─────────────────────────────────────────────────────────────┐
│ Step 1: HyperMesh Application Defines Policy                │
│                                                               │
│ let policy = Policy {                                         │
│     requires_pos: true,                                       │
│     validate_asset_hash: true,                                │
│     matrix_routing_enabled: true,                             │
│ };                                                            │
│                                                               │
│ ebpf.set_policy(connection_id, policy);                       │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Step 2: HyperMesh Sends Packet via STOQ                      │
│                                                               │
│ let pos_header = ExtensionHeader {                            │
│     extension_type: EXT_PROOF_OF_STATE,                       │
│     length: 72,                                               │
│     data: pos_proof.serialize(),                              │
│ };                                                            │
│                                                               │
│ stoq.send_packet(data, vec![pos_header]);                    │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Step 3: STOQ Protocol Builds Packet (Oblivious to PoS)       │
│                                                               │
│ let packet = StoqPacket {                                     │
│     version: 1,                                               │
│     connection_id: 0x1234,                                    │
│     extensions: vec![pos_header],  // ← Opaque to STOQ       │
│     payload: data,                                            │
│ };                                                            │
│                                                               │
│ kernel.send(packet);  // ← eBPF will intercept               │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Step 4: eBPF Intercepts Packet at Kernel Level               │
│                                                               │
│ #[ebpf_program]                                               │
│ fn hypermesh_packet_filter(ctx: &XdpContext) -> XdpAction {  │
│     let packet = parse_stoq_packet(ctx)?;                    │
│                                                               │
│     // Lookup HyperMesh policy for this connection           │
│     let policy = POLICY_MAP.get(&packet.connection_id)?;     │
│                                                               │
│     if policy.requires_pos {                                  │
│         // Find PoS extension header                          │
│         let pos_ext = packet.extensions.iter()                │
│             .find(|e| e.extension_type == 0x1000)?;           │
│                                                               │
│         // Validate PoS (HyperMesh-specific logic)            │
│         if !validate_pos_in_ebpf(pos_ext.data) {              │
│             return XdpAction::Drop;  // ← Rejected!           │
│         }                                                     │
│     }                                                         │
│                                                               │
│     XdpAction::Pass  // ← Valid packet passes                │
│ }                                                             │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Step 5: STOQ Receives Validated Packet in Userspace          │
│                                                               │
│ // STOQ only sees packets that passed eBPF validation        │
│ let packet = stoq.recv_packet();                              │
│                                                               │
│ // STOQ handles QUIC mechanics (ACKs, retransmission, etc.)  │
│ connection.process_packet(packet);                            │
│                                                               │
│ // Deliver payload to application                            │
│ app.on_data(packet.payload);                                  │
└─────────────────────────────────────────────────────────────┘
```

---

## 5. eBPF Policy Management

### 5.1 Policy Map Structure

```rust
// hypermesh-ebpf/src/policy_maps.rs

#[repr(C)]
pub struct ValidationPolicy {
    pub requires_pos: bool,
    pub validate_asset_hash: bool,
    pub check_matrix_routing: bool,
    pub privacy_tier: u8,          // 0=Anonymous, 1=Private, 2=Federated, 3=Public
    pub max_packet_size: u32,
    pub rate_limit_per_sec: u32,
}

// eBPF map: connection_id → policy
#[map]
static POLICY_MAP: HashMap<u64, ValidationPolicy> = HashMap::with_max_entries(10000, 0);
```

### 5.2 Loading Policies from HyperMesh

```rust
// blockmatrix/src/ebpf/policy_manager.rs

pub struct EbpfPolicyManager {
    ebpf: EbpfProgram,
}

impl EbpfPolicyManager {
    pub fn set_policy(&mut self, connection_id: u64, policy: ValidationPolicy) -> Result<()> {
        // Load policy into eBPF map (kernel space)
        self.ebpf.map_update("POLICY_MAP", &connection_id, &policy)?;
        Ok(())
    }

    pub fn remove_policy(&mut self, connection_id: u64) -> Result<()> {
        self.ebpf.map_delete("POLICY_MAP", &connection_id)?;
        Ok(())
    }
}
```

### 5.3 eBPF Validation Logic

```rust
// hypermesh-ebpf/src/packet_filter.rs

#[xdp]
fn hypermesh_packet_filter(ctx: XdpContext) -> u32 {
    match try_filter(&ctx) {
        Ok(action) => action,
        Err(_) => xdp_action::XDP_DROP,
    }
}

fn try_filter(ctx: &XdpContext) -> Result<u32, ()> {
    // 1. Parse STOQ packet
    let eth = ptr_at::<EthernetHeader>(&ctx, 0)?;
    if eth.ether_type != ETH_P_IPV6 { return Ok(xdp_action::XDP_PASS); }

    let ipv6 = ptr_at::<Ipv6Header>(&ctx, EthernetHeader::LEN)?;
    if ipv6.next_header != IPPROTO_UDP { return Ok(xdp_action::XDP_PASS); }

    let udp = ptr_at::<UdpHeader>(&ctx, EthernetHeader::LEN + Ipv6Header::LEN)?;

    // Parse STOQ header
    let stoq_offset = EthernetHeader::LEN + Ipv6Header::LEN + UdpHeader::LEN;
    let stoq = ptr_at::<StoqHeader>(&ctx, stoq_offset)?;

    // 2. Lookup policy for this connection
    let policy = unsafe { POLICY_MAP.get(&stoq.connection_id) };
    let policy = match policy {
        Some(p) => p,
        None => return Ok(xdp_action::XDP_PASS),  // No policy = pass
    };

    // 3. Validate based on policy
    if policy.requires_pos {
        if !validate_proof_of_state(ctx, stoq_offset)? {
            return Ok(xdp_action::XDP_DROP);  // ← Invalid PoS
        }
    }

    if policy.validate_asset_hash {
        if !validate_asset_hash(ctx, stoq_offset)? {
            return Ok(xdp_action::XDP_DROP);  // ← Invalid hash
        }
    }

    // 4. Packet is valid
    Ok(xdp_action::XDP_PASS)
}

fn validate_proof_of_state(ctx: &XdpContext, offset: usize) -> Result<bool, ()> {
    // Find PoS extension header (type 0x1000)
    let pos_header = find_extension_header(ctx, offset, 0x1000)?;
    if pos_header.is_none() { return Ok(false); }

    // Parse PoS fields
    let pos = ptr_at::<ProofOfStateHeader>(ctx, pos_header.unwrap())?;

    // Validate each proof (simplified for eBPF)
    // Real validation may involve lookups, hashing, etc.
    if pos.when == 0 { return Ok(false); }  // Invalid timestamp
    if pos.who == [0u8; 32] { return Ok(false); }  // Invalid identity

    Ok(true)
}
```

---

## 6. Concrete Repository Structure

### 6.1 Current Directory Layout (December 2025)

```
/home/persist/repos/projects/web3/
├── stoq/                           ← Standalone STOQ protocol (CURRENT)
│   ├── src/
│   │   ├── protocol/              ← Packet format, QUIC
│   │   ├── transport/
│   │   │   └── ebpf/              ← **EXISTING** eBPF (needs refactoring)
│   │   │       ├── mod.rs         ← Capabilities detection (generic)
│   │   │       ├── xdp.rs         ← XDP packet filtering (performance)
│   │   │       ├── af_xdp.rs      ← Zero-copy sockets (transport)
│   │   │       ├── metrics.rs     ← Kernel metrics (monitoring)
│   │   │       └── loader.rs      ← eBPF program loading (generic)
│   │   ├── extensions.rs          ← **EXISTING** Extension headers
│   │   └── certificates.rs        ← Certificate validation (has PoS refs)
│   └── Cargo.toml                 ← Currently NO hypermesh deps ✅
│
└── blockmatrix/                   ← HyperMesh application (CURRENT)
    ├── src/
    │   ├── transport/
    │   │   └── stoq_bridge.rs     ← Uses STOQ as transport
    │   └── integration/
    └── Cargo.toml                 ← Depends on: stoq
```

### 6.2 Target Directory Layout (Phase 2 Completion - March 2026)

```
/home/persist/repos/projects/web3/
├── stoq/                           ← Standalone STOQ protocol (TARGET)
│   ├── src/
│   │   ├── protocol/              ← Core protocol (unchanged)
│   │   ├── transport/
│   │   │   └── ebpf/              ← **GENERIC** eBPF hooks only
│   │   │       ├── mod.rs         ← Capabilities (keep)
│   │   │       ├── hooks.rs       ← Generic validation hooks (new)
│   │   │       ├── af_xdp.rs      ← Zero-copy sockets (keep)
│   │   │       └── loader.rs      ← Program loading (keep)
│   │   ├── extensions.rs          ← Generic extension support (keep)
│   │   └── certificates.rs        ← Generic cert validation (remove PoS)
│   └── Cargo.toml                 ← Still NO hypermesh deps ✅
│
├── hypermesh-ebpf/                ← HyperMesh eBPF layer (NEW - migrated)
│   ├── src/
│   │   ├── packet_filter.rs       ← Migrated from stoq/xdp.rs + HyperMesh logic
│   │   ├── policy_maps.rs         ← Policy configuration (new)
│   │   ├── hypermesh_headers.rs   ← PoS, Asset Hash parsing (new)
│   │   ├── validation.rs          ← PoS validation logic (new)
│   │   └── metrics.rs             ← Migrated from stoq/metrics.rs + HyperMesh metrics
│   └── Cargo.toml                 ← Depends on: stoq (types), aya, libbpf
│
└── blockmatrix/                   ← HyperMesh application (TARGET)
    ├── src/
    │   ├── transport/
    │   │   ├── stoq_adapter.rs    ← STOQ transport (enhanced)
    │   │   └── hypermesh_extensions.rs  ← Extension definitions (new)
    │   ├── ebpf/
    │   │   ├── policy_manager.rs  ← Policy management (new)
    │   │   └── loader.rs          ← Load hypermesh-ebpf programs (new)
    └── Cargo.toml                 ← Depends on: stoq, hypermesh-ebpf
```

### 6.3 Dependency Graph

```
blockmatrix (HyperMesh)
    ├── depends on: stoq (transport)
    └── depends on: hypermesh-ebpf (validation)

hypermesh-ebpf
    └── depends on: stoq (packet types only, no runtime)

stoq
    └── depends on: NOTHING HyperMesh-specific
```

---

## 7. API Examples

### 7.1 STOQ API (Standalone Protocol)

```rust
// Example: Using STOQ without HyperMesh

use stoq::{StoqConnection, ExtensionHeader};

// Connect using STOQ (like a TCP socket)
let mut conn = StoqConnection::connect("::1:4433").await?;

// Send data with custom extensions
let my_extension = ExtensionHeader {
    extension_type: 0x2000,  // My application's extension
    length: 8,
    data: vec![1, 2, 3, 4, 5, 6, 7, 8],
};

conn.send_with_extensions(b"Hello, STOQ!", vec![my_extension]).await?;

// Receive data (extensions preserved)
let (data, extensions) = conn.recv().await?;
println!("Received: {:?}", data);
```

### 7.2 HyperMesh API (Using STOQ + eBPF)

```rust
// Example: HyperMesh using STOQ with intelligence

use blockmatrix::transport::{StoqAdapter, HyperMeshExtensions};
use blockmatrix::ebpf::EbpfPolicyManager;

// Create STOQ connection
let mut adapter = StoqAdapter::connect("::1:4433").await?;

// Configure eBPF policy for this connection
let mut ebpf = EbpfPolicyManager::new()?;
ebpf.set_policy(adapter.connection_id(), ValidationPolicy {
    requires_pos: true,
    validate_asset_hash: true,
    check_matrix_routing: false,
    privacy_tier: 3,  // Public tier
    max_packet_size: 65536,
    rate_limit_per_sec: 1000,
})?;

// Send with Proof of State
let pos = ProofOfState {
    who: node_identity,
    what: compute_proof,
    when: current_timestamp(),
    where_: matrix_position,
};

adapter.send_with_pos(b"HyperMesh data", pos).await?;

// Receive (eBPF already validated PoS before it reached userspace)
let data = adapter.recv().await?;  // Guaranteed valid by eBPF
```

---

## 8. Performance Characteristics

### 8.1 Without eBPF (Userspace Validation)

```
Packet arrives → Network stack → STOQ userspace → Validation → Application
                                  ↑
                            10-50 µs overhead
```

**Problems:**
- Every packet reaches userspace (context switches)
- Invalid packets waste CPU cycles
- Vulnerable to DoS (malicious packets processed)

### 8.2 With eBPF (Kernel Validation)

```
Packet arrives → eBPF filter → STOQ userspace → Application
                  ↑
              0.5-2 µs overhead

(Invalid packets dropped at kernel level, never reach userspace)
```

**Benefits:**
- 10-100x faster validation (kernel vs userspace)
- Invalid packets never waste userspace CPU
- DoS protection (kernel drops bad packets)
- Zero-copy packet processing

### 8.3 Benchmark Results (Expected)

| Operation | Without eBPF | With eBPF | Improvement |
|-----------|--------------|-----------|-------------|
| PoS Validation | 50 µs | 2 µs | 25x |
| Asset Hash Check | 30 µs | 1 µs | 30x |
| Invalid Packet Drop | 40 µs | 0.5 µs | 80x |
| Throughput (pps) | 200k | 2M | 10x |

---

## 9. Security Model

### 9.1 Defense in Depth

```
Layer 1: eBPF (Kernel)
    ├── Drop malformed STOQ packets
    ├── Enforce rate limits
    ├── Validate PoS signatures
    └── Check asset hashes
           ↓ (Only valid packets pass)
Layer 2: STOQ (Userspace)
    ├── TLS certificate validation
    ├── Connection state management
    └── QUIC security (encryption, replay protection)
           ↓ (Authenticated, encrypted data)
Layer 3: HyperMesh (Application)
    ├── Blockchain consensus validation
    ├── Matrix routing verification
    └── Privacy tier enforcement
```

### 9.2 Attack Mitigation

| Attack Vector | Without eBPF | With eBPF |
|---------------|--------------|-----------|
| **Invalid PoS flood** | Userspace CPU exhaustion | Kernel drops instantly |
| **Malformed packets** | Parser vulnerabilities | Rejected at kernel |
| **Connection DoS** | Rate limiting in app | Kernel-level limits |
| **Hash collision attacks** | Userspace verification | eBPF pre-validation |

---

## 10. Migration Path: From Current to Target

### Current State (December 2025 - Phase 1 Complete)

**What Exists:**
- ✅ STOQ as standalone protocol (`/stoq/`)
- ✅ STOQ eBPF transport acceleration (`/stoq/src/transport/ebpf/` - 5 files, 46KB)
- ✅ Extension header framework (`/stoq/src/extensions.rs`)
- ✅ BlockMatrix Phase 1 foundation (matrix, tensor, blockchain, geospatial, persistence)
- ⚠️ STOQ has some PoS references in `certificates.rs` (needs cleanup)

**What Needs Work:**
- ❌ Clear separation: STOQ eBPF (generic) vs HyperMesh eBPF (intelligence)
- ❌ Dedicated `hypermesh-ebpf` crate doesn't exist
- ❌ HyperMesh-specific validation logic mixed into STOQ

### Phase 2.1: eBPF Refactoring & Separation (Weeks 13-14)

**Goal**: Extract HyperMesh intelligence from STOQ, create clean boundaries

**Tasks**:
1. **Audit STOQ eBPF code** (`/stoq/src/transport/ebpf/`)
   - Identify: What's generic transport vs HyperMesh-specific
   - Decision matrix:
     - `mod.rs` (capabilities detection) → **Keep in STOQ** (generic)
     - `xdp.rs` (XDP filtering) → **Split**: Generic hooks in STOQ, validation logic → hypermesh-ebpf
     - `af_xdp.rs` (zero-copy sockets) → **Keep in STOQ** (transport-level)
     - `metrics.rs` (kernel metrics) → **Split**: Generic metrics in STOQ, HyperMesh metrics → hypermesh-ebpf
     - `loader.rs` (eBPF loading) → **Keep in STOQ** (generic infrastructure)

2. **Create `hypermesh-ebpf` crate**
   - Extract HyperMesh-specific logic from STOQ
   - Implement: `packet_filter.rs`, `policy_maps.rs`, `hypermesh_headers.rs`, `validation.rs`
   - Migrate: HyperMesh-specific parts of `xdp.rs` and `metrics.rs`

3. **Clean up STOQ**
   - Remove PoS validation logic from `/stoq/src/transport/certificates.rs`
   - Keep only generic validation hooks
   - Document: STOQ provides mechanisms, applications provide policies

4. **Define HyperMesh extension types**
   - Create `/blockmatrix/src/transport/hypermesh_extensions.rs`
   - Define: `EXT_PROOF_OF_STATE`, `EXT_ASSET_HASH`, `EXT_MATRIX_ROUTING`, `EXT_PRIVACY_TIER`
   - Implement serialization/deserialization for PoS, Asset Hash, Matrix Routing headers

**Deliverables**:
- ✅ Clean STOQ with generic eBPF hooks only
- ✅ New `hypermesh-ebpf` crate with HyperMesh intelligence
- ✅ HyperMesh extension type definitions
- ✅ Migration complete, zero duplication

**Testing**: Verify STOQ can be used independently (no HyperMesh dependencies)

### Phase 2.2: eBPF Policy Integration (Weeks 15-16)

**Goal**: Integrate HyperMesh eBPF with BlockMatrix

**Tasks**:
1. **Create `EbpfPolicyManager` in BlockMatrix**
   - Implement policy loading into eBPF maps
   - Per-connection policy configuration
   - Runtime policy updates

2. **Implement validation in eBPF**
   - PoS validation at kernel level
   - Asset hash verification
   - Matrix routing checks

3. **Integrate with STOQ transport**
   - STOQ provides generic hooks
   - HyperMesh eBPF implements validation
   - Test: Invalid packets dropped at kernel

**Deliverables**:
- ✅ `EbpfPolicyManager` operational
- ✅ PoS validation in kernel (XDP/TC)
- ✅ BlockMatrix uses STOQ + hypermesh-ebpf

**Testing**: Validate PoS packets pass, invalid packets drop at kernel

### Phase 2.3: Complete Intelligence Layer (Weeks 17-18)

**Goal**: Full intelligence layer with all validation types

**Tasks**:
1. **Implement all validation types**
   - Asset hash verification in eBPF
   - Matrix routing validation
   - Privacy tier enforcement
   - Rate limiting per policy

2. **Performance optimization**
   - Zero-copy packet processing
   - JIT compilation of eBPF programs
   - Benchmark: kernel vs userspace validation

3. **Documentation and testing**
   - Complete eBPF program documentation
   - Comprehensive integration tests
   - Performance benchmarks

**Deliverables**:
- ✅ All validation types implemented
- ✅ Performance targets met (10-100x improvement)
- ✅ Complete test coverage

**Success Criteria**:
- STOQ remains standalone (can be used without HyperMesh)
- HyperMesh intelligence in dedicated crate
- All validation at kernel level
- Performance: 2M packets/sec with validation

---

## 11. Future: STOQ as Internet Standard

### 11.1 Path to RFC

With proper separation, STOQ could become an IETF standard:

**RFC Draft Structure:**
```
1. Abstract: "STOQ: Secure Transport over QUIC with Extensible Headers"
2. Introduction: Modern transport needs
3. Protocol Specification:
   - Packet format (MUST)
   - Extension mechanism (MUST)
   - Certificate validation (MUST)
   - Connection lifecycle (MUST)
4. Extension Registry: IANA-managed extension type IDs
5. Security Considerations
6. IANA Considerations: Extension type allocation
```

**Extension Type Registry (like TLS extensions):**
```
0x0000-0x0FFF: Reserved (IETF)
0x1000-0x1FFF: HyperMesh (Proof of State, etc.)
0x2000-0x2FFF: Future Application A
0x3000-0x3FFF: Future Application B
...
```

### 11.2 Adoption Path

1. **Open Source STOQ** (independent of HyperMesh)
2. **Implement in multiple languages** (Rust, Go, C, Python)
3. **Demonstrate performance benefits** over traditional stacks
4. **Submit Internet-Draft** to IETF
5. **Community adoption** (non-HyperMesh applications use STOQ)

---

## 12. Conclusion

**Key Takeaways:**

1. **STOQ = Dumb Transport**
   - Knows packet structure, NOT application semantics
   - Can be used by any application (not just HyperMesh)
   - Path to becoming internet standard

2. **eBPF = Intelligence Enforcement**
   - Kernel-level validation (10-100x faster)
   - Programmable per-connection policies
   - Security: drops invalid packets before userspace

3. **HyperMesh = Intelligence Definition**
   - Defines validation policies (PoS, Asset Hash, etc.)
   - Implements validation logic in eBPF programs
   - Completely decoupled from STOQ protocol

**This architecture enables:**
- ✅ STOQ to become an internet standard
- ✅ HyperMesh intelligence without coupling to transport
- ✅ Kernel-level performance (microsecond validation)
- ✅ Flexibility (policies can change without protocol changes)
- ✅ Security (defense in depth at kernel + userspace)

**Next Steps:**
- Phase 2.1: Separate STOQ as standalone protocol
- Phase 2.2: Implement eBPF validation layer
- Phase 2.3: Integrate with HyperMesh Phase 1 foundation

---

## 13. Existing Code Migration Strategy

### 13.1 STOQ eBPF Code Audit

**File**: `/stoq/src/transport/ebpf/mod.rs` (9,407 bytes)
- **Content**: eBPF capabilities detection, feature flags
- **Decision**: ✅ **KEEP IN STOQ** - Generic transport-level capability detection
- **Action**: No changes needed

**File**: `/stoq/src/transport/ebpf/xdp.rs` (5,769 bytes)
- **Content**: XDP packet filtering, drop/pass decisions
- **Decision**: ⚠️ **SPLIT** - Generic hooks stay, HyperMesh logic moves
- **Action**:
  - Keep: Generic XDP program structure, packet parsing
  - Move: HyperMesh-specific validation logic → `hypermesh-ebpf/packet_filter.rs`

**File**: `/stoq/src/transport/ebpf/af_xdp.rs` (7,042 bytes)
- **Content**: AF_XDP zero-copy socket implementation
- **Decision**: ✅ **KEEP IN STOQ** - Pure transport optimization
- **Action**: No changes needed

**File**: `/stoq/src/transport/ebpf/metrics.rs` (14,333 bytes)
- **Content**: Kernel-level performance metrics collection
- **Decision**: ⚠️ **SPLIT** - Generic metrics stay, HyperMesh metrics move
- **Action**:
  - Keep: Packet counts, throughput, latency (transport-level)
  - Move: PoS validation metrics, asset hash metrics → `hypermesh-ebpf/metrics.rs`

**File**: `/stoq/src/transport/ebpf/loader.rs` (10,077 bytes)
- **Content**: eBPF program loading infrastructure
- **Decision**: ✅ **KEEP IN STOQ** - Generic eBPF program management
- **Action**: No changes needed

### 13.2 Certificate Validation Cleanup

**File**: `/stoq/src/transport/certificates.rs`
- **Issue**: Contains reference to "Proof of State consensus proof validation"
- **Decision**: ⚠️ **REMOVE PoS** - Keep only generic certificate validation
- **Action**:
  - Remove: PoS-specific validation logic
  - Keep: Standard X.509 certificate validation, TLS integration
  - Add: Generic validation hook that HyperMesh can implement

### 13.3 Extension Headers Status

**File**: `/stoq/src/extensions.rs` (458 lines)
- **Content**: Generic extension header framework with existing types
- **Decision**: ✅ **KEEP IN STOQ** - Already correct architecture
- **Existing Types**: `PacketToken`, `PacketShard`, `HopInfo`, `SeedInfo`
- **Action**: HyperMesh adds new types: `ProofOfState`, `AssetHash`, `MatrixRouting`, `PrivacyTier`

### 13.4 Migration Checklist

**Before Phase 2.1:**
- [x] Audit all STOQ eBPF code (complete)
- [ ] Document: What stays vs what moves
- [ ] Create migration test plan

**During Phase 2.1:**
- [ ] Create `/hypermesh-ebpf/` crate skeleton
- [ ] Move HyperMesh logic from `/stoq/src/transport/ebpf/xdp.rs`
- [ ] Move HyperMesh metrics from `/stoq/src/transport/ebpf/metrics.rs`
- [ ] Clean up `/stoq/src/transport/certificates.rs` (remove PoS)
- [ ] Add generic validation hooks to STOQ

**After Phase 2.1:**
- [ ] Verify STOQ builds without HyperMesh dependencies
- [ ] Test STOQ independently (no HyperMesh code)
- [ ] Verify HyperMesh eBPF integrates cleanly
- [ ] Performance benchmark: no regression from refactoring

**Success Criteria**:
- ✅ STOQ Cargo.toml has zero HyperMesh dependencies
- ✅ STOQ can be used by non-HyperMesh applications
- ✅ All HyperMesh intelligence in `hypermesh-ebpf` crate
- ✅ Zero code duplication between STOQ and hypermesh-ebpf
- ✅ Performance maintained or improved after refactoring

---

**Document Version:** 1.1
**Date:** December 4, 2025
**Status:** Architecture Specification (Updated with Current Implementation)
