# Safety Report: Code Cleanup Risk Assessment

## Executive Summary
**Overall Risk Level: MEDIUM-HIGH**
- Julia removal: **LOW RISK** (isolated to adapters)
- Doc cleanup: **LOW RISK** (mostly archived/duplicate content)
- Duplicate consolidation: **MEDIUM RISK** (requires careful merging)
- Crypto migration: **HIGH RISK** (mixed RSA/FALCON usage)

## 1. Julia Code Removal Analysis

### Current State
- **43 public APIs** in Julia modules
- **3 external imports** outside Julia directory
- **NO production dependencies** - only used in language adapters

### Safe to Remove
✅ `/blockmatrix/src/catalog/vm/julia/` - Self-contained module
✅ `JuliaAdapter` in language adapters - Optional language support
✅ `JuliaValidator` in catalog - Syntax validation only

### Dependencies Found
- `blockmatrix/src/catalog/vm/mod.rs`: Exports JuliaVM (can be removed)
- `blockmatrix/src/catalog/vm/languages/adapters/julia.rs`: Main adapter
- `catalog/src/validation/validators.rs`: JuliaValidator for syntax

### Migration Path
1. Remove Julia modules first
2. Update language adapter registry
3. Remove validator exports
4. Clean up test references

**Risk Level: LOW** - Julia is fully isolated in adapter pattern

## 2. Documentation Cleanup Analysis

### Safe to Remove (Archives)
✅ `/docs/archive/` - 50+ duplicate reports
✅ `/docs/completion-snapshots-2025/` - Outdated snapshots
✅ `/blockmatrix/docs/archive/obsolete-reports/` - Marked obsolete

### Must Preserve
⚠️ `/docs/architecture/` - Critical design docs
⚠️ `/docs/specs/` - API specifications
⚠️ `/api/API_SPECIFICATIONS.md` - Active API docs
⚠️ `README.md` files - User documentation

### Documentation Dependencies
- UI components reference API docs
- Caesar wallet references asset specifications
- Integration guides actively used

**Risk Level: LOW** - Archives are clearly separated

## 3. Duplicate Code Consolidation

### High-Risk Duplicates Found
⚠️ **Multiple DNS implementations**:
- `/trustchain/src/dns/dns_over_quic.rs` (deprecated)
- `/trustchain/src/dns/dns_over_stoq.rs` (new)
- `/blockmatrix/src/dns/` (separate implementation)

⚠️ **Asset system duplicates**:
- `/blockmatrix/src/assets/`
- `/catalog/src/assets.rs`
- `/caesar/src/assets/`

⚠️ **Consensus implementations**:
- `/lib/src/proof_of_state/` (16K lines)
- `/trustchain/src/consensus/`
- `/blockmatrix/src/consensus/`

### Critical Merging Required
1. DNS: Migrate all to dns_over_stoq
2. Assets: Unify under BlockMatrix asset system
3. Consensus: Consolidate to lib/proof_of_state

**Risk Level: MEDIUM** - Requires careful API preservation

## 4. Breaking Changes Detected

### Public API Changes
⚠️ **JuliaVM removal affects**:
- Language adapter factory patterns
- VM execution context APIs
- Catalog package validation

⚠️ **DNS consolidation breaks**:
- Direct DNS-over-QUIC clients
- Hardcoded DNS resolver paths
- Certificate validation flows

### External Dependencies
- UI components expect catalog endpoints
- Caesar wallet uses asset APIs
- STOQ references certificate managers

### Mitigation Strategy
1. Add deprecation warnings first
2. Provide migration guides
3. Update all internal references
4. Version bump (1.0 → 2.0)

**Risk Level: MEDIUM** - Manageable with proper versioning

## 5. Cryptography Migration Risks

### CRITICAL: Mixed Cryptography Usage
🔴 **RSA still active in**:
- `/stoq/src/transport/certificates.rs` - RSA signature schemes
- `/gateway/tests/endpoint_validation.rs` - RSA test configs
- X.509 certificate parsing uses RSA validation

🟡 **FALCON-1024 partially implemented**:
- `/stoq/src/transport/falcon/` - Engine implemented
- `/trustchain/` - Not fully migrated
- Transport params support both

🟡 **Kyber usage inconsistent**:
- Asset encryption claims Kyber
- Actual implementation missing in some paths
- No unified crypto provider

### Certificate Compatibility Issues
- Current certs use RSA/ECDSA
- FALCON certs not universally supported
- Need dual-mode operation during transition

**Risk Level: HIGH** - Security-critical migration

## Recommendations

### Immediate Actions Required
1. **Create comprehensive test suite BEFORE refactoring**
2. **Document all public APIs being changed**
3. **Implement crypto provider abstraction**
4. **Add feature flags for gradual migration**

### Phased Approach
**Phase 1**: Remove Julia (LOW RISK)
**Phase 2**: Clean docs (LOW RISK)
**Phase 3**: Add missing tests (CRITICAL)
**Phase 4**: Consolidate duplicates (MEDIUM RISK)
**Phase 5**: Migrate crypto (HIGH RISK)

### Do NOT Proceed Without
- [ ] Full integration test suite
- [ ] Crypto migration tests
- [ ] API compatibility layer
- [ ] Rollback plan

## Conclusion
**Can proceed with**: Julia removal, documentation cleanup
**Must delay until tested**: Duplicate consolidation, crypto migration
**Critical gap**: Test coverage at 60%, need 90%+ before major refactoring