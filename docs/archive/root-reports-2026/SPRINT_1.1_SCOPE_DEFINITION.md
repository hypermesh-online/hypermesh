# Sprint 1.1 Scope Definition: STOQ Transport Completion
**Sprint Duration**: Week 1 (7 days)
**Sprint Goal**: Complete STOQ transport layer with protocol intelligence and achieve 100% test pass rate

## Discovery Summary
- **Core Transport**: 90% complete, achieving 11+ Gbps performance
- **Protocol Intelligence**: 40% complete, missing critical PoS integration
- **Test Failures**: 5 tests failing due to crypto provider initialization
- **TODOs Identified**: 5 minor TODOs (buffer pool, service discovery, DNS, key tracking, API tests)
- **NO unimplemented!() calls**: False alarm - thorough search found none

## Sprint 1.1 Implementation Scope

### Priority 1: Critical Path (Must Complete) - Days 1-3

#### 1.1 Fix Crypto Provider Initialization (4 hours)
**Files**: `/stoq/src/transport/mod.rs`, `/stoq/src/network_isolation.rs`
**Issue**: rustls CryptoProvider not configured, causing 5 test failures
**Solution**:
- Add `rustls::crypto::aws_lc_rs::default_provider().install_default()` in test setup
- Configure in transport initialization for production
**Dependencies**: None
**Acceptance**: All 5 failing tests pass

#### 1.2 Implement PoS Token Validation at Protocol Layer (12 hours)
**Files**: `/stoq/src/protocol/mod.rs`, `/stoq/src/transport/mod.rs`
**Current State**: Placeholder TODOs at lines 214, 221, 226
**Implementation**:
```rust
// Integration points:
1. Import TrustChain consensus validator
2. Add PoS token validation in handle_stoq_token()
3. Validate shard metadata in handle_sharded_data()
4. Verify signatures in handle_falcon_signature()
```
**Dependencies**: TrustChain library
**Acceptance**: Protocol validates all four proofs (PoSpace, PoStake, PoWork, PoTime)

#### 1.3 TrustChain Integration for Service Discovery (8 hours)
**Files**: `/stoq/src/api/mod.rs` (lines 339, 352)
**Current State**: Hardcoded service endpoints
**Implementation**:
- Replace placeholder `resolve_service()` with TrustChain DNS client
- Integrate with `trustchain::stoq_client::TrustChainStoqClient`
- Support service discovery via DNS-over-QUIC
**Dependencies**: TrustChain DNS module
**Acceptance**: Dynamic service resolution working

### Priority 2: Production Requirements - Days 4-5

#### 2.1 Complete Protocol State Management (6 hours)
**Files**: `/stoq/src/protocol/mod.rs`
**TODOs**: Lines 205-206 (key ID and signed frames tracking)
**Implementation**:
- Implement proper key ID management from TrustChain CA
- Track signed frames for audit trail
- Add state persistence for connection resumption
**Dependencies**: 1.2 (PoS validation)
**Acceptance**: Protocol maintains state across connections

#### 2.2 Integration Testing Suite (8 hours)
**Files**: Create `/stoq/tests/integration/`
**Current State**: Missing comprehensive integration tests
**Implementation**:
- End-to-end STOQ + TrustChain workflow tests
- Multi-node consensus validation tests
- Performance benchmarks (target: 2.95 Gbps minimum)
- Byzantine fault tolerance scenarios
**Dependencies**: 1.1, 1.2, 1.3
**Acceptance**: 90%+ test coverage on critical paths

### Priority 3: Optimizations (Defer if Needed) - Days 6-7

#### 3.1 Buffer Pool Optimization (4 hours)
**File**: `/stoq/src/transport/mod.rs` line 285
**Current State**: Simple buffer allocation
**Implementation**: Shared buffer pool with `Arc<Mutex<Vec<BytesMut>>>`
**Impact**: Low - current implementation works
**Can Defer**: Yes - not a blocker for Sprint 1.1

#### 3.2 STOQ API Integration Tests (4 hours)
**File**: `/stoq/src/api/mod.rs` line 379
**Current State**: TODO comment for tests
**Implementation**: API-level integration test suite
**Can Defer**: Yes - core functionality tested elsewhere

## Deferred to Sprint 1.2 or Later

### Deferred Items (Document WHY)
1. **eBPF Kernel Integration** (40% complete)
   - **Why Deferred**: Requires kernel module deployment, not critical for initial release
   - **Impact**: Performance optimization for 10+ Gbps (current 2.95 Gbps sufficient)

2. **Advanced Adaptive Optimization**
   - **Why Deferred**: Current tier-based adaptation working well
   - **Impact**: Minor performance improvements

3. **Full Certificate Rotation Automation**
   - **Why Deferred**: 24-hour rotation interval sufficient for Sprint 1.1
   - **Impact**: Operational enhancement, not functional blocker

## Integration Contract with TrustChain

### Required TrustChain Modules
```rust
use trustchain::{
    consensus::{ConsensusProof, ConsensusValidator},
    validation::ProofValidator,
    stoq_client::{TrustChainStoqClient, ServiceEndpoint},
    ca::SecurityIntegratedCA,
};
```

### Integration Points
1. **Certificate Validation**: `/stoq/src/transport/certificates.rs`
   - Use `SecurityIntegratedCA` for cert issuance
   - Validate with consensus proofs

2. **PoS Token Validation**: `/stoq/src/protocol/mod.rs`
   - Call `ConsensusValidator::validate_proof()`
   - Check all four proofs (Space, Stake, Work, Time)

3. **Service Discovery**: `/stoq/src/api/mod.rs`
   - Use `TrustChainStoqClient::resolve_service()`
   - Cache results with TTL

## Test Strategy

### Fix Failing Tests (Priority 1)
1. Add crypto provider initialization to test setup
2. Fix import paths in phase5 tests
3. Update test expectations for new validation

### New Test Requirements
- PoS validation unit tests (4 proofs)
- TrustChain integration tests
- Service discovery tests
- Multi-node consensus tests
- Performance benchmarks (2.95 Gbps baseline)

### Target Coverage
- **Critical Path**: 100% (transport, protocol, validation)
- **Integration**: 90% (STOQ + TrustChain workflows)
- **Overall**: 85%+ line coverage

## Success Criteria (13-Point Quality Gates)

1. ✅ **Zero unimplemented!()** - Already complete
2. ✅ **All tests passing** - Fix 5 failures, add new tests
3. ✅ **PoS validation integrated** - All 4 proofs validated at protocol layer
4. ✅ **TrustChain integrated** - Service discovery and cert validation
5. ✅ **Performance maintained** - 2.95 Gbps minimum (current: 11+ Gbps)
6. ✅ **No hardcoded endpoints** - Dynamic service discovery
7. ✅ **Protocol state management** - Proper key ID and frame tracking
8. ✅ **Error handling** - No silent failures, proper logging
9. ✅ **Security validation** - Consensus proofs on all operations
10. ✅ **Documentation** - Updated API docs and integration guide
11. ✅ **Memory safety** - No unsafe operations, proper cleanup
12. ✅ **Backward compatibility** - Existing clients continue working
13. ✅ **Production ready** - Can deploy to trust.hypermesh.online

## Task Breakdown & Timeline

### Day 1-2: Foundation (16 hours)
- [ ] Fix crypto provider (4h) - BLOCKER
- [ ] Import TrustChain modules (2h)
- [ ] Implement PoS validation hooks (10h)

### Day 3-4: Integration (16 hours)
- [ ] TrustChain service discovery (8h)
- [ ] Protocol state management (6h)
- [ ] Fix remaining test failures (2h)

### Day 5-6: Testing (16 hours)
- [ ] Integration test suite (8h)
- [ ] Performance benchmarks (4h)
- [ ] Multi-node testing (4h)

### Day 7: Polish & Verification (8 hours)
- [ ] Documentation updates (2h)
- [ ] Final test run (2h)
- [ ] Performance validation (2h)
- [ ] Security audit (2h)

## Total Effort Estimate: 56 hours (7 days @ 8 hours/day)

## Definition of Done
- [ ] All 5 test failures fixed
- [ ] PoS token validation implemented and tested
- [ ] TrustChain integration complete (certs + DNS)
- [ ] Service discovery replacing hardcoded endpoints
- [ ] 100% test pass rate achieved
- [ ] Performance baseline maintained (2.95+ Gbps)
- [ ] Integration tests covering critical paths
- [ ] Documentation updated with integration guide
- [ ] Code review completed
- [ ] Ready for deployment to trust.hypermesh.online

## Risk Mitigation
- **Risk**: TrustChain integration complexity
  - **Mitigation**: Use existing `TrustChainStoqClient` abstraction
- **Risk**: Performance degradation from validation
  - **Mitigation**: Async validation, connection pooling
- **Risk**: Breaking existing clients
  - **Mitigation**: Maintain backward compatibility flags

## Next Sprint Preview (1.2)
- eBPF kernel module deployment
- Advanced adaptive optimization
- Full certificate rotation automation
- Production deployment to trust.hypermesh.online