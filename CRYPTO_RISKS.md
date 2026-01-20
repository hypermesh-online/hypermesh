# Cryptography Migration Risk Assessment

## Executive Summary
**Risk Level: CRITICAL** ⚠️
- Mixed RSA/FALCON usage creates security vulnerabilities
- No unified crypto provider leads to inconsistent security
- Certificate incompatibility will break existing deployments
- Kyber implementation incomplete despite claims

## 1. Current Cryptography State

### Active Cryptographic Systems
```
RSA (Legacy) ←→ FALCON-1024 (Quantum-Resistant)
     ↓                ↓
   X.509           Custom Format
     ↓                ↓
   STOQ            BlockMatrix
```

### Mixed Usage Locations

#### RSA Still Active
```rust
// stoq/src/transport/certificates.rs
rustls::SignatureScheme::RSA_PKCS1_SHA256,
rustls::SignatureScheme::RSA_PSS_SHA256,
// ... 7 RSA schemes still supported
```

#### FALCON Partially Implemented
```rust
// stoq/src/transport/falcon/
pub struct FalconEngine {
    variant: FalconVariant::Falcon1024
}
// But TrustChain still uses RSA!
```

#### Kyber Missing
```rust
// Claimed in docs:
"Kyber-1024 encryption for assets"
// Reality: No Kyber implementation found
```

## 2. Security Vulnerabilities

### V1: Downgrade Attacks
**Severity: CRITICAL**
```rust
// Current negotiation allows downgrade
if peer.supports_falcon() {
    use_falcon()
} else {
    fallback_to_rsa()  // VULNERABLE!
}
```
**Risk**: Attackers force RSA, then exploit quantum computer

### V2: Certificate Confusion
**Severity: HIGH**
```rust
// Mixed validation logic
match cert_type {
    X509 => validate_rsa(),     // Old path
    Falcon => validate_falcon(), // New path
    _ => accept_any()           // DANGEROUS!
}
```
**Risk**: Invalid certs accepted during transition

### V3: Incomplete Encryption
**Severity: HIGH**
- Assets claim Kyber encryption
- No Kyber implementation found
- Using AES-256-GCM (not quantum-resistant)
**Risk**: False security claims, vulnerable to quantum attack

### V4: Key Management Chaos
**Severity: MEDIUM**
```rust
// Multiple key stores
/keys/rsa/          // Old keys
/keys/falcon/       // New keys
/keys/mixed/        // Transition keys (!)
```
**Risk**: Key confusion, accidental exposure

## 3. Certificate Compatibility Issues

### Problem Areas

#### TLS Handshake Failures
```
Client (RSA) → Server (FALCON)
Result: CONNECTION_REFUSED
```

#### CA Chain Broken
```
Root CA (RSA)
  ├── Intermediate (RSA)
  └── Leaf (FALCON)  // INVALID CHAIN!
```

#### Cross-System Trust
- STOQ: Supports both RSA and FALCON
- TrustChain: RSA only
- BlockMatrix: Expects FALCON
- Result: Systems can't authenticate

### Impact on Production
1. **New nodes can't join** - Cert validation fails
2. **Existing nodes isolated** - Can't verify new certs
3. **Data in transit vulnerable** - Forced to unencrypted
4. **Consensus breaks** - Nodes can't agree on crypto

## 4. Migration Risks

### Risk 1: Big Bang Migration
**Approach**: Switch everything at once
```rust
// Version 1.0: RSA everywhere
// Version 2.0: FALCON everywhere
```
**Problems**:
- All nodes must update simultaneously
- No rollback possible
- One failure breaks entire network

### Risk 2: Gradual Migration
**Approach**: Support both, migrate slowly
```rust
enum CryptoMode {
    RsaOnly,      // Phase 1
    DualMode,     // Phase 2 (current)
    FalconOnly,   // Phase 3 (target)
}
```
**Problems**:
- Complexity explosion
- Security vulnerabilities in dual mode
- Long transition period (months/years)

### Risk 3: Parallel Networks
**Approach**: Run RSA and FALCON networks separately
**Problems**:
- Double infrastructure cost
- Data synchronization issues
- User confusion

## 5. Missing Test Coverage

### Critical Untested Scenarios

❌ **Crypto Negotiation**
```rust
#[test]
fn test_rsa_to_falcon_upgrade() {
    // NOT IMPLEMENTED
}
```

❌ **Certificate Rotation**
```rust
#[test]
fn test_cert_rotation_during_migration() {
    // NOT IMPLEMENTED
}
```

❌ **Downgrade Prevention**
```rust
#[test]
fn test_prevent_crypto_downgrade() {
    // NOT IMPLEMENTED
}
```

❌ **Mixed Mode Operations**
```rust
#[test]
fn test_mixed_rsa_falcon_cluster() {
    // NOT IMPLEMENTED
}
```

❌ **Kyber Integration**
```rust
#[test]
fn test_kyber_asset_encryption() {
    // NOT IMPLEMENTED - Kyber not even present!
}
```

## 6. Required Tests Before Migration

### Phase 1: Foundation (20 tests)
1. **Dual-mode negotiation** (5 tests)
   - RSA client → FALCON server
   - FALCON client → RSA server
   - Mutual auth scenarios
   - Downgrade prevention
   - Version negotiation

2. **Certificate validation** (5 tests)
   - Mixed chain validation
   - Cross-signing scenarios
   - Revocation during migration
   - Trust anchor updates
   - Emergency rotation

3. **Key management** (5 tests)
   - Dual key generation
   - Key storage isolation
   - Key rotation procedures
   - Backup/recovery
   - HSM integration

4. **Performance impact** (5 tests)
   - FALCON vs RSA benchmarks
   - Memory usage comparison
   - CPU usage under load
   - Network overhead
   - Latency measurements

### Phase 2: Integration (15 tests)
1. **Cross-component** (5 tests)
   - STOQ ↔ TrustChain
   - TrustChain ↔ BlockMatrix
   - Mixed crypto consensus
   - Asset encryption flow
   - DNS with new certs

2. **Migration procedures** (5 tests)
   - Rolling upgrade
   - Rollback scenarios
   - Partial migration
   - Network partition
   - Recovery procedures

3. **Attack resistance** (5 tests)
   - Downgrade attacks
   - Man-in-the-middle
   - Replay attacks
   - Certificate substitution
   - Quantum simulation

### Phase 3: Production (10 tests)
1. **Scale testing** (5 tests)
   - 1000+ node migration
   - Geographic distribution
   - Network latency impact
   - Storage requirements
   - Bandwidth usage

2. **Monitoring** (5 tests)
   - Crypto usage metrics
   - Migration progress
   - Error detection
   - Performance tracking
   - Security alerts

## 7. Implementation Requirements

### Unified Crypto Provider
```rust
pub trait CryptoProvider {
    fn sign(&self, data: &[u8]) -> Signature;
    fn verify(&self, sig: &Signature) -> bool;
    fn encrypt(&self, data: &[u8]) -> Ciphertext;
    fn decrypt(&self, ct: &Ciphertext) -> Vec<u8>;
}

pub struct UnifiedCrypto {
    mode: CryptoMode,
    rsa: Option<RsaProvider>,
    falcon: Option<FalconProvider>,
    kyber: Option<KyberProvider>,  // MUST IMPLEMENT
}
```

### Migration Controller
```rust
pub struct MigrationController {
    current_phase: MigrationPhase,
    nodes: HashMap<NodeId, CryptoCapability>,
    progress: MigrationProgress,
    rollback_point: Option<Checkpoint>,
}
```

### Compatibility Layer
```rust
pub struct CryptoCompat {
    fn negotiate(peer: &Peer) -> CryptoMode;
    fn validate_any(cert: &Certificate) -> bool;
    fn convert_cert(old: RsaCert) -> FalconCert;
}
```

## 8. Recommendations

### Immediate Actions (CRITICAL)
1. **STOP claiming Kyber support** - Implement or remove
2. **Fix dual-mode vulnerabilities** - No automatic downgrade
3. **Unify crypto providers** - Single source of truth
4. **Add migration tests** - Before ANY production use

### Short Term (1-2 weeks)
1. Implement Kyber if keeping claim
2. Create migration controller
3. Add compatibility layer
4. Test certificate rotation

### Medium Term (1 month)
1. Deploy test network with FALCON
2. Practice migration procedures
3. Monitor performance impact
4. Document operations guide

### Long Term (3 months)
1. Complete migration to FALCON
2. Deprecate RSA support
3. Remove compatibility layer
4. Achieve quantum resistance

## Risk Matrix

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Downgrade attack | HIGH | CRITICAL | Disable downgrade |
| Cert incompatibility | CERTAIN | HIGH | Dual-mode support |
| Performance degradation | MEDIUM | MEDIUM | Benchmark first |
| Migration failure | MEDIUM | CRITICAL | Test thoroughly |
| Kyber not implemented | CERTAIN | HIGH | Implement now |
| Network partition | LOW | HIGH | Rollback plan |

## Conclusion

**DO NOT PROCEED** with production deployment until:
1. ✅ Kyber implemented or claims removed
2. ✅ Unified crypto provider created
3. ✅ Migration tests pass
4. ✅ Downgrade attacks prevented
5. ✅ Certificate compatibility verified
6. ✅ Performance impact measured
7. ✅ Rollback procedures tested

**Current State**: Dangerous mixed-crypto environment
**Required State**: Clean migration path with tests
**Estimated Effort**: 4-6 weeks to production-ready