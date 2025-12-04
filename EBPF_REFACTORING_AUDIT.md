# eBPF Refactoring Audit Report
**Sprint 2.1: Intelligence Layer Separation**
**Date:** 2025-12-04
**Auditor:** Operations Tier 1 Agent

---

## Executive Summary

This audit analyzes the existing STOQ eBPF implementation to determine what stays in STOQ (transport mechanisms) versus what moves to `hypermesh-ebpf` (intelligence policies).

**Key Finding:** STOQ eBPF code is already well-separated. Most files are generic transport acceleration and can remain in STOQ. Only minor HyperMesh-specific references need extraction.

---

## Audit Results: File-by-File Analysis

### 1. `/stoq/src/transport/ebpf/mod.rs` (9,407 bytes)

**Decision:** ✅ **KEEP** (Generic capabilities detection)

**Rationale:**
- Pure infrastructure: Capability detection, XDP/AF_XDP availability checks
- No HyperMesh-specific logic found
- Generic eBPF initialization and lifecycle management
- Kernel version parsing, CAP_NET_ADMIN checks - all transport-level

**HyperMesh References:** NONE

**Actions Required:** NONE

---

### 2. `/stoq/src/transport/ebpf/xdp.rs` (5,769 bytes)

**Decision:** ⚠️ **SPLIT** (Generic structure + HyperMesh filtering)

**Rationale:**
- **KEEP in STOQ:**
  - XDP program attachment/detachment infrastructure (lines 26-159)
  - Generic packet filtering hooks (lines 133-142)
  - Statistics collection infrastructure (lines 144-158)
  - XDP action types (Drop/Pass/Redirect) - generic

- **MOVE to hypermesh-ebpf:**
  - HyperMesh-specific filter rules (if any implemented)
  - Connection tracking with HyperMesh semantics
  - Proof of State validation logic (NOT YET IMPLEMENTED in current code)

**Current Code Analysis:**
- File is a **placeholder implementation** ("simulated", "not yet implemented")
- No actual HyperMesh intelligence present - just structure
- `XdpFilterConfig` is generic (QUIC port filtering, IPv6-only, packet size limits)

**Actions Required:**
1. Keep entire file in STOQ (it's already generic)
2. Create `hypermesh-ebpf/src/packet_filter.rs` with HyperMesh-specific XDP logic
3. HyperMesh eBPF will implement actual validation, STOQ provides hooks

---

### 3. `/stoq/src/transport/ebpf/af_xdp.rs` (7,042 bytes)

**Decision:** ✅ **KEEP** (Pure transport - zero-copy sockets)

**Rationale:**
- AF_XDP is a **generic Linux kernel feature** for zero-copy packet I/O
- No application logic whatsoever
- UMEM configuration, ring buffers, socket management - all transport-level
- Could be used by ANY application requiring high-performance networking

**HyperMesh References:** NONE

**Actions Required:** NONE

---

### 4. `/stoq/src/transport/ebpf/metrics.rs` (14,333 bytes)

**Decision:** ⚠️ **SPLIT** (Generic metrics + HyperMesh metrics)

**Rationale:**
- **KEEP in STOQ:**
  - Packet-level metrics (packet counts, bytes, size distribution) - lines 29-59
  - Connection-level metrics (active connections, state tracking) - lines 61-87
  - Latency metrics (min/max/avg/percentiles, histogram) - lines 89-123
  - CPU metrics (utilization, interrupts, context switches) - lines 125-138
  - Memory metrics (UMEM pages, ring buffer utilization) - lines 140-153
  - All collection infrastructure (lines 156-360)

- **MOVE to hypermesh-ebpf:**
  - Proof of State validation metrics (validation counts, failures)
  - Asset hash verification metrics
  - Matrix routing metrics
  - Privacy tier metrics
  - HyperMesh-specific performance counters

**Current Code Analysis:**
- Current metrics are **100% generic transport metrics**
- No HyperMesh-specific metrics exist yet
- File can stay in STOQ, add new file in hypermesh-ebpf for intelligence metrics

**Actions Required:**
1. Keep entire file in STOQ (already generic)
2. Create `hypermesh-ebpf/src/metrics.rs` for intelligence-specific metrics
3. HyperMesh metrics will extend STOQ metrics, not replace them

---

### 5. `/stoq/src/transport/ebpf/loader.rs` (10,077 bytes)

**Decision:** ✅ **KEEP** (Generic infrastructure)

**Rationale:**
- Pure eBPF program loading infrastructure
- Clang compilation, bytecode loading, verification
- Could load ANY eBPF program (STOQ or HyperMesh)
- No application-specific logic

**HyperMesh References:**
- Line 184: `#define STOQ_PORT 9292` in generated XDP source (generic transport)
- Lines 186-323: Generated XDP program is **generic STOQ filtering** (IPv6, UDP, port 9292)
- Connection tracking is generic (src/dst IP/port only, no PoS)

**Actions Required:**
1. Keep entire file in STOQ
2. HyperMesh eBPF programs will use this loader infrastructure

---

### 6. `/stoq/src/transport/certificates.rs` (905 lines)

**Decision:** ⚠️ **CLEAN UP** (Remove PoS semantics, keep generic validation)

**HyperMesh References Found:**
- **Line 8:** Comment mentions "Proof of State consensus proof validation"
- **Line 93-94:** `consensus_proof: Option<Vec<u8>>` field in `StoqNodeCertificate`
- **Line 147:** `consensus_proof: &[u8]` parameter in `request_certificate()`
- **Line 200:** `consensus_proof` in certificate request JSON
- **Line 250:** `consensus_proof: Some(consensus_proof.to_vec())`
- **Line 749:** `generate_real_consensus_proof()` method (lines 768-785)
- **Lines 770-782:** PoS proof generation with comments about "four-proof consensus system"

**Actions Required:**
1. **REMOVE:** Lines 8, 93-94, 147, 200, 250, 749, 768-785 (PoS-specific)
2. **REPLACE:** Generic validation hook trait
3. **KEEP:** All X.509 certificate validation, TLS integration, TrustChain CT log validation

**Rationale:**
- STOQ should validate certificates like TLS does (standard X.509)
- Proof of State is HyperMesh application logic, not transport concern
- Applications can implement custom validation via hook trait

---

## Summary Statistics

| File | Size (bytes) | Decision | HyperMesh References | Actions |
|------|--------------|----------|---------------------|---------|
| `mod.rs` | 9,407 | ✅ KEEP | 0 | None |
| `xdp.rs` | 5,769 | ✅ KEEP (already generic) | 0 | Create hypermesh-ebpf equivalent |
| `af_xdp.rs` | 7,042 | ✅ KEEP | 0 | None |
| `metrics.rs` | 14,333 | ✅ KEEP (already generic) | 0 | Create hypermesh-ebpf metrics |
| `loader.rs` | 10,077 | ✅ KEEP | 0 (STOQ_PORT is transport) | None |
| `certificates.rs` | ~23,000 | ⚠️ CLEAN UP | 8 references | Remove PoS, add hooks |

**Total eBPF Code:** 46,628 bytes (5 files)
**Code to Keep in STOQ:** 46,628 bytes (100%)
**Code to Remove:** ~100 lines in certificates.rs (PoS references)
**Code to Create:** New `hypermesh-ebpf` crate with HyperMesh intelligence

---

## Architecture Compliance Check

### ✅ STOQ Remains Generic Transport
- No PoS validation logic in eBPF layer
- Extension headers are opaque byte blobs
- Certificate validation uses standard X.509
- eBPF provides hooks, applications provide policies

### ✅ Clear Separation of Concerns
- **STOQ provides:** Packet structure, reliability, eBPF acceleration infrastructure
- **hypermesh-ebpf provides:** Validation policies, intelligence enforcement
- **HyperMesh provides:** Policy definitions, extension header semantics

### ✅ STOQ Can Become Internet Standard
- Zero HyperMesh dependencies after cleanup
- Usable by any application requiring high-performance transport
- Extension mechanism allows custom intelligence injection

---

## Next Steps (Implementation Order)

1. ✅ **Create `hypermesh-ebpf` crate** with dependencies on `stoq` (types only)
2. ✅ **Define HyperMesh extension types** in `blockmatrix/src/transport/hypermesh_extensions.rs`
3. ✅ **Implement HyperMesh eBPF programs** in `hypermesh-ebpf/src/`
4. ⚠️ **Clean up `certificates.rs`** - Remove PoS, add validation hooks
5. ✅ **Add generic validation hooks** in `stoq/src/transport/ebpf/hooks.rs`
6. ✅ **Verify STOQ independence** - Build without HyperMesh
7. ✅ **Update BlockMatrix** to use `hypermesh-ebpf` crate

---

## Verification Criteria

- [ ] STOQ Cargo.toml has zero HyperMesh dependencies
- [ ] STOQ builds successfully standalone
- [ ] `hypermesh-ebpf` depends on `stoq` (types only), `aya`, `libbpf-sys`
- [ ] BlockMatrix uses both STOQ (transport) and hypermesh-ebpf (intelligence)
- [ ] Zero code duplication between STOQ and hypermesh-ebpf
- [ ] All validation logic in hypermesh-ebpf, not STOQ
- [ ] Generic validation hooks allow other applications to use STOQ

---

**Audit Complete:** STOQ eBPF code is already well-separated. Minimal refactoring required.
