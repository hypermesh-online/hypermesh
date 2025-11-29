# Documentation Consolidation List

## Files to Remove (Contain False or Unverified Claims)

### High Priority Removals
1. **Files with phantom references** - Update or remove sections referencing:
   - NEXUS_CLI_SPEC.md (doesn't exist)
   - NEXUS_CLI_GUIDE.md (doesn't exist)
   - BOOTSTRAP_ROADMAP.md (doesn't exist)
   - /trustchain/ARCHITECTURE.md (doesn't exist)

2. **Overly optimistic status reports**:
   - Consider archiving test reports claiming unverified performance
   - Remove or update completion percentage claims

## Files to Merge (Eliminate Redundancy)

### Container Documentation
**Merge into**: `docs/components/container_runtime.md`
- CONTAINER_SECURITY_IMPLEMENTATION.md
- CONTAINER_ORCHESTRATION_IMPLEMENTATION_REPORT.md
- BLOCKCHAIN_NATIVE_COMPUTE_IMPLEMENTATION.md

### Test and Quality Reports
**Merge into**: `docs/testing/consolidated_test_status.md`
- TEST_SUMMARY.md
- core/tests/nexus-test-report.md
- core/tests/SPRINT2_TEST_REPORT.md
- core/tests/DNS_CT_TEST_SUITE_SUMMARY.md
- QUALITY_ASSESSMENT_REPORT.md

### Architecture Documentation
**Merge into**: `docs/architecture/current_architecture.md`
- ARCHITECTURE_SEPARATION.md
- core/runtime/docs/ARCHITECTURE_OVERVIEW.md
- blockchain/README.md (if contains architecture info)

### Implementation Summaries
**Merge into**: `docs/implementation_status.md`
- ALM_IMPLEMENTATION_SUMMARY.md
- CONSENSUS_PROOF_INTEGRATION_SUMMARY.md
- ASSET_SYSTEM_COMPLETION.md
- MONITORING_REFACTOR_SUMMARY.md

## Files to Update (Correct False Claims)

### 1. README.md
- Remove any "in development" claims
- Update completion percentages to ~8%
- Add clear "EXPERIMENTAL/RESEARCH" disclaimer
- Remove unverified performance metrics

### 2. benchmarks/mfn/README.md
- Clarify that benchmarks are frameworks, not results
- Remove any claimed performance numbers without evidence

### 3. core/README.md
- Update to reflect actual implementation status
- Remove references to non-existent features

### 4. All CLAUDE.md files
- Already updated main files
- Check for any additional CLAUDE.md files in subdirectories

## Files to Create (New Structure)

### Required New Documentation

1. **docs/architecture/CURRENT_STATE.md**
   - Accurate description of what exists
   - Clear separation of implemented vs planned

2. **docs/benchmarks/VERIFIED_RESULTS.md**
   - Only actual benchmark results with reproduction steps
   - Remove all unverified claims

3. **docs/development/GETTING_STARTED.md**
   - Realistic guide for developers
   - What actually works and can be tested

4. **docs/roadmap/REALISTIC_TIMELINE.md**
   - Based on current velocity
   - No marketing language

## Documentation Standards Going Forward

### Every Documentation File Must:
1. **Status Header**: Clear indicator of implementation status
   ```markdown
   **Status**: 🚧 IN DEVELOPMENT | ✅ IMPLEMENTED | 📋 PLANNED
   **Completion**: X% (based on actual code analysis)
   **Last Verified**: YYYY-MM-DD
   ```

2. **Evidence Links**: Reference actual code for any claims
   ```markdown
   **Claim**: Feature X is implemented
   **Evidence**: See `/src/module/file.rs` lines 100-200
   **Test**: Run `cargo test feature_x` to verify
   ```

3. **Performance Claims**: Must include benchmark command
   ```markdown
   **Performance**: Operation completes in Xms
   **Benchmark**: `cargo bench --bench operation_bench`
   **Environment**: [specify test environment]
   **Output**: [actual benchmark output]
   ```

## Action Items

### Immediate (Day 1)
- [x] Update CLAUDE.md files to reflect reality
- [x] Create REALITY_CHECK.md
- [ ] Add disclaimers to README.md
- [ ] Remove phantom file references

### Short Term (Days 2-3)
- [ ] Merge redundant documentation files
- [ ] Create new documentation structure
- [ ] Update all performance claims with evidence
- [ ] Archive unverified test reports

### Medium Term (Week 1)
- [ ] Complete documentation consolidation
- [ ] Implement documentation validation checks
- [ ] Create automated documentation accuracy tests
- [ ] Establish review process for new claims

## Documentation Validation Checklist

Before any documentation update:
- [ ] Claims backed by code references
- [ ] Performance metrics from actual benchmarks
- [ ] File references verified to exist
- [ ] Status accurately reflects implementation
- [ ] No unsubstantiated marketing language
- [ ] Examples can be executed
- [ ] Dependencies clearly stated
- [ ] Known limitations documented

## Priority Order

1. **CRITICAL**: Remove "~5-7% functional implementation" and "in development" claims
2. **HIGH**: Fix phantom file references
3. **HIGH**: Consolidate test/quality reports
4. **MEDIUM**: Merge architecture documents
5. **MEDIUM**: Create new documentation structure
6. **LOW**: Clean up redundant implementation summaries

## Success Criteria

Documentation cleanup is complete when:
- ✅ No false claims remain in any documentation
- ✅ All referenced files exist and are accurate
- ✅ Performance claims backed by reproducible benchmarks
- ✅ Clear distinction between implemented/in-progress/planned
- ✅ Documentation matches actual codebase state
- ✅ New contributions follow established standards