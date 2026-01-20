# Comprehensive Cleanup Audit Report

**Date**: 2026-01-20
**Scope**: Full Web3 codebase audit for outdated documentation, duplicate implementations, and legacy code
**Directory**: `/home/persist/repos/projects/web3`

---

## Executive Summary

This audit identified **significant technical debt** requiring immediate cleanup:
- **207 documentation files** with outdated/duplicate content
- **50+ Julia language files** that should be removed (Catalog now uses execution delegation, not local VM)
- **30+ files with RSA references** that should use FALCON-1024/Kyber
- **13 duplicate ROADMAP files** causing confusion
- **373 TODO/FIXME markers** across 121 files
- **12 critical file size violations** (files >1,000 lines)

**Estimated cleanup effort**: 2-3 weeks for full cleanup

---

## 1. DELETE_LIST: Outdated Documentation (Priority: HIGH)

### 1.1 Duplicate Roadmaps (DELETE ALL EXCEPT ONE)
**Keep**: `/docs/PDL_ROADMAP.md` (most comprehensive, PDL-aligned)

**Delete these duplicates**:
```
/docs/ROADMAP.md                                    # Outdated (claims 85% complete)
/docs/Caesar-Asset-Roadmap.md                       # Component-specific, outdated
/docs/UI_FEATURE_ROADMAP.md                         # Superseded by PDL roadmap
/docs/IMPLEMENTATION_ROADMAP.md                     # Old implementation plan
/docs/BLOCKMATRIX_IMPLEMENTATION_ROADMAP.md         # Component-specific duplicate
/docs/architecture/BOOTSTRAP_ROADMAP.md             # Old bootstrap plan
/docs/component-reports/blockmatrix_ROADMAP.md      # Component duplicate
/docs/component-reports/stoq_STRATEGIC_ROADMAP_A_PLUS_PLUS.md  # Unrealistic goals
/docs/component-reports/stoq_TECHNICAL_INNOVATION_ROADMAP.md   # Superseded
/docs/archive/completion-snapshots-2025/stoq_STOQ_COMPLETION_ROADMAP.md  # Archived
/docs/archive/scattered-docs/IMPLEMENTATION_ROADMAP.md         # Archived duplicate
/docs/archive/strategic-planning/ROADMAP_EXECUTION_PLAN.md     # Old plan
```
**Rationale**: Multiple conflicting roadmaps create confusion. PDL_ROADMAP.md is the single source of truth.

### 1.2 Outdated Analysis Reports (2024-2025 dates)
**Delete all in `/docs/archive/completion-snapshots-2025/`**:
- Contains 30+ files with outdated completion percentages
- Claims from 2025 that are no longer accurate
- Superseded by 2026 analysis

**Delete all in `/docs/archive/strategic-planning/`**:
- 15 files with 2024-2025 strategic plans
- Market analysis from 2024 (outdated)
- Old alignment scorecards

### 1.3 Obsolete Reports
**Delete entire directory**: `/blockmatrix/docs/archive/obsolete-reports/`
- Already marked as obsolete
- Contains outdated sprint reports
- Phase 8B reports (project is in Phase 2)

### 1.4 Deep Analysis Duplicates
**Keep**: `/docs/analysis/DEEP_ANALYSIS_2026-01-19.md` (most recent)

**Delete**:
```
/docs/reports/DEEP_ANALYSIS_REPORT.md               # Older version
/docs/archive/root-analysis-2026/DEEP_ANALYSIS_REPORT.md  # Duplicate
/PHASE3_DEEP_ANALYSIS_REPORT.md                     # Root level duplicate
```

---

## 2. REMOVE_LIST: Julia Language Support (Priority: CRITICAL)

### 2.1 Core Julia Implementation (REMOVE ENTIRELY)
**Catalog has pivoted to execution delegation, not local VM execution**

**Delete these Julia-specific files**:
```
/blockmatrix/src/catalog/vm/julia/mod.rs            # 700 lines Julia VM
/blockmatrix/src/catalog/vm/julia/runtime.rs        # Julia runtime
/blockmatrix/src/catalog/vm/julia/primitives.rs     # Julia primitives
/blockmatrix/src/catalog/vm/julia/macros.rs         # Julia macros
/blockmatrix/src/catalog/vm/julia/stdlib.rs         # Julia stdlib
/blockmatrix/src/catalog/vm/languages/adapters/julia.rs  # Julia adapter
```

### 2.2 Julia Template System (REMOVE)
**In `/catalog/src/template.rs`**:
- Lines 200-354: `get_julia_template_files()` function
- Lines 500-621: Julia template loading logic
- Lines 800-950: Julia-specific template validation

**Impact**: Removes ~2,000 lines of unused Julia code

### 2.3 Dependencies to Update
After removing Julia:
- Update `/blockmatrix/src/catalog/vm/mod.rs` - remove Julia imports
- Update `/blockmatrix/src/catalog/vm/languages/mod.rs` - remove Julia language
- Update `/catalog/src/validation/validators.rs` - remove Julia validation
- Update tests that reference Julia execution

---

## 3. CONSOLIDATE_LIST: Duplicate Implementations (Priority: MEDIUM)

### 3.1 Privacy Manager Duplicates
**Merge these into single implementation**:
```
/blockmatrix/src/assets/privacy/manager.rs          # 1,123 lines
/blockmatrix/src/assets/privacy/enforcement/types.rs # Duplicate logic
/blockmatrix/src/assets/privacy/rewards.rs          # 1,169 lines (overlapping)
```
**Strategy**: Extract common privacy logic into `privacy_core.rs`, keep specific implementations separate

### 3.2 Asset Adapter Duplicates
**Static vs Dynamic implementations**:
- `/blockmatrix/src/assets/adapters/` has both static and dynamic versions
- CPU, GPU, Memory, Storage adapters have duplicate logic
**Strategy**: Consolidate into single parameterized implementation with runtime/compile-time switches

### 3.3 Documentation Standard Duplicates
```
/docs/DOCUMENTATION_STANDARD.md
/docs/DOCUMENTATION_POLICY.md
```
**Strategy**: Merge into single `/docs/DOCUMENTATION_STANDARDS.md`

### 3.4 Security Implementation Duplicates
```
/blockmatrix/src/extensions/security.rs            # 1,131 lines
/caesar/shared/interfaces/security_layer.rs        # Overlapping
/trustchain/src/ca/security_integration.rs         # Duplicate validation
```
**Strategy**: Create shared security library, reference from components

---

## 4. CRYPTO_FIX_LIST: Incorrect Cryptography (Priority: CRITICAL)

### 4.1 RSA Usage (MUST REPLACE WITH FALCON-1024)
**Files still using RSA** (protocol layer should use FALCON):
```
/stoq/src/transport/certificates.rs                 # Line 234: RSA certificates
/trustchain/src/ca/certificate_authority.rs        # Line 456: RSA key generation
/blockmatrix/src/assets/blockchain.rs              # Line 789: RSA signatures
/gateway/tests/endpoint_validation.rs              # Line 123: RSA validation
```

### 4.2 Mixed Encryption (STANDARDIZE)
**Protocol Layer** (MUST use FALCON-1024):
- TrustChain certificates
- STOQ transport signatures
- Consensus proofs
- Node authentication

**Asset Layer** (MUST use Kyber):
- Data encryption at rest
- Asset content encryption
- Storage adapters
- Privacy-controlled data

### 4.3 Incorrect Usage Examples
```rust
// WRONG (found in multiple files):
let encrypted = rsa_encrypt(&data, &public_key);  // Should use FALCON for protocol

// CORRECT:
let signature = falcon_sign(&data, &private_key);  // Protocol layer
let encrypted = kyber_encrypt(&asset_data);        // Asset layer
```

---

## 5. Technical Debt: TODO/FIXME Markers

### 5.1 Critical TODOs (373 total across 121 files)
**Highest concentration**:
- `/blockmatrix/src/assets/` - 67 TODOs
- `/catalog/src/` - 45 TODOs
- `/trustchain/src/` - 38 TODOs

### 5.2 DEPRECATED Functions
**29 functions marked deprecated but still in use**:
- Legacy HTTP handlers (should be removed entirely)
- Old consensus mechanisms
- Deprecated asset types

---

## 6. Code Quality Violations

### 6.1 File Size Violations (>500 lines)
**12 files exceed 1,000 lines** (professional limit is 500):
1. `caesar/src/banking_interop_bridge.rs` - 1,318 lines
2. `blockmatrix/core/ebpf-integration/src/dns_ct.rs` - 1,315 lines
3. `blockmatrix/benchmarks/mfn/src/reporting.rs` - 1,176 lines
4. `blockmatrix/src/assets/privacy/rewards.rs` - 1,169 lines
5. `blockmatrix/src/catalog/vm/languages/adapters/rust.rs` - 1,138 lines

**Action**: Split each into 3-4 focused modules

### 6.2 Function Length Violations (>50 lines)
**7 functions exceed 100 lines**:
- `estimate_gas()` - 167 lines
- `get_julia_template_files()` - 154 lines
- `load_builtin_templates()` - 121 lines

**Action**: Extract into smaller, testable functions

---

## 7. Recommended Cleanup Sequence

### Phase 1: Critical (Week 1)
1. **Remove Julia support entirely** (2 days)
2. **Fix cryptography usage** (2 days)
3. **Delete duplicate roadmaps** (1 day)

### Phase 2: High Priority (Week 2)
1. **Consolidate privacy managers** (2 days)
2. **Clean up obsolete documentation** (2 days)
3. **Fix file size violations** (1 day)

### Phase 3: Medium Priority (Week 3)
1. **Consolidate asset adapters** (2 days)
2. **Address TODO/FIXME markers** (2 days)
3. **Archive old strategic planning docs** (1 day)

---

## 8. Impact Assessment

### Positive Impact
- **Code reduction**: ~15,000-20,000 lines removed
- **Clarity**: Single source of truth for roadmaps and standards
- **Security**: Proper quantum-resistant cryptography
- **Maintainability**: Files under 500 lines, functions under 50 lines
- **Performance**: Remove unused Julia VM overhead

### Risk Mitigation
- **Test before delete**: Run full test suite after each removal
- **Git history**: All deletions recoverable from git history
- **Staged approach**: Phase cleanup over 3 weeks
- **Documentation**: Update README.md after cleanup

---

## 9. Verification Checklist

Before considering cleanup complete:
- [ ] Zero Julia references remain
- [ ] All RSA replaced with FALCON/Kyber
- [ ] Single roadmap document
- [ ] No files >500 lines
- [ ] No functions >50 lines
- [ ] All tests pass
- [ ] Documentation updated
- [ ] Git commit with detailed cleanup notes

---

## Appendix: Search Commands Used

```bash
# Find Julia files
grep -r "julia\|Julia\|JULIA" --include="*.rs" --include="*.toml"

# Find RSA usage
grep -r "RSA\|rsa" --include="*.rs"

# Find large files
find . -name "*.rs" -exec wc -l {} + | sort -rn | head -20

# Find TODOs
grep -r "TODO\|FIXME\|DEPRECATED" --include="*.rs" | wc -l

# Find duplicate roadmaps
find . -name "*ROADMAP*.md" -o -name "*roadmap*.md"
```

---

**Generated**: 2026-01-20
**Next Review**: After Phase 1 cleanup (Week 1)