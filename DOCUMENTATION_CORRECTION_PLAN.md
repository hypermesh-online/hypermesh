# Documentation Correction Plan - BlockMatrix Architecture

## Executive Summary

After deleting 7 fundamentally wrong files, we need to correct ~60-70% of remaining documentation that misrepresents the BlockMatrix architecture. The key issues are:
1. Missing matrix topology and tensor operation concepts
2. Traditional DNS/CA references instead of Block-MATRIX federated trust
3. STOQ described as just transport vs intelligence layer
4. Missing privacy tier descriptions
5. Missing instruction-based retrieval architecture

## CRITICAL Priority Files (Fix Immediately)

### 1. `/home/persist/repos/projects/web3/README.md`
**Severity**: CRITICAL - Main project entry point
**Problems**:
- Line 28: "FALCON-1024 CA operational" - implies traditional CA
- Line 74: "DNS/TLS → TrustChain" - should be "Traditional trust → Block-MATRIX federated trust"
- Lines 78-87: Describes TrustChain as traditional CA with DNS-over-STOQ
- Missing: Matrix topology concept, tensor operations, instruction-based retrieval
- Missing: Block-MATRIX as the trust layer (not traditional CA)
**Corrections Needed**:
- Replace CA/CT references with Block-MATRIX federated trust
- Add matrix topology explanation in architecture section
- Add tensor operation descriptions for compute orchestration
- Emphasize instruction-based retrieval vs traditional DNS
**Complexity**: Medium

### 2. `/home/persist/repos/projects/web3/CLAUDE.md`
**Severity**: CRITICAL - Project context for development
**Problems**:
- Line 200: "we shouldn't be using HTTP at all" - correct direction but incomplete
- Lines 17-23: Repository descriptions missing matrix topology context
- Lines 78-97: Asset adapter descriptions missing tensor operations
- Lines 137-156: Traditional DNS bootstrap solution
- Missing: Block-MATRIX as primary trust mechanism
**Corrections Needed**:
- Update repository descriptions to include matrix topology
- Add tensor operation context to asset adapters
- Replace DNS bootstrap with Block-MATRIX trust bootstrap
- Add privacy tier matrix (Anonymous|Private|Federated|Public)
**Complexity**: Medium

### 3. `/home/persist/repos/projects/web3/blockmatrix/CLAUDE.md`
**Severity**: CRITICAL - BlockMatrix component context
**Problems**:
- Title uses "HyperMesh" instead of BlockMatrix consistently
- Lines 14-25: Describes traditional cloud infrastructure replacement
- Missing: Matrix topology as core architecture
- Missing: Tensor operations for resource management
- Line 178: "Nexus CLI" reference (should be removed)
**Corrections Needed**:
- Replace HyperMesh references with BlockMatrix where appropriate
- Add matrix topology as fundamental design principle
- Add tensor operation descriptions for resource orchestration
- Remove incorrect CLI references
**Complexity**: Simple

### 4. `/home/persist/repos/projects/web3/stoq/README.md`
**Severity**: CRITICAL - STOQ component documentation
**Problems**:
- Line 3: "Pure transport protocol" - missing intelligence layer aspect
- Lines 8-10: Describes as transport-only like TCP/IP
- Missing: STOQ as intelligence layer with matrix routing
- Missing: Instruction-based packet retrieval
- Missing: Integration with Block-MATRIX trust
**Corrections Needed**:
- Add intelligence layer capabilities beyond transport
- Include matrix-based routing topology
- Add instruction-based retrieval system
- Clarify Block-MATRIX trust integration (not traditional certs)
**Complexity**: Complex

### 5. `/home/persist/repos/projects/web3/trustchain/README.md`
**Severity**: CRITICAL - TrustChain component documentation
**Problems**:
- Entire document assumes traditional CA/CT/DNS model
- Lines 5-7: "CA/CT/DNS core structures"
- Lines 69-72: Traditional DNS services listing
- Missing: Block-MATRIX federated trust model
- Missing: Matrix topology for trust relationships
**Corrections Needed**:
- Complete rewrite from CA/CT/DNS to Block-MATRIX trust
- Add matrix topology for federated trust
- Replace DNS references with instruction-based retrieval
- Add privacy tier trust levels
**Complexity**: Complex

## HIGH Priority Files (Fix Soon)

### 6. `/home/persist/repos/projects/web3/docs/ARCHITECTURE.md`
**Severity**: HIGH - System architecture reference
**Status**: Needs verification of existence and content
**Expected Problems**:
- Likely describes traditional architecture patterns
- Missing matrix topology diagrams
- Missing tensor operation flows
**Complexity**: Complex

### 7. `/home/persist/repos/projects/web3/BOOTSTRAP_ROADMAP.md`
**Severity**: HIGH - Deployment strategy
**Status**: Referenced but needs content check
**Expected Problems**:
- Traditional DNS bootstrap approach
- Missing Block-MATRIX trust bootstrap
**Complexity**: Medium

### 8. Component lib.rs Files
**Severity**: HIGH - Module-level documentation
**Files to Check**:
- `/blockmatrix/src/lib.rs`
- `/stoq/src/lib.rs`
- `/trustchain/src/lib.rs`
- `/catalog/src/lib.rs`
**Expected Problems**:
- Module docs likely have architecture descriptions
- Missing matrix topology in rustdoc comments
**Complexity**: Simple (doc comments only)

## MEDIUM Priority Files

### 9. Integration and Test Documentation
**Files**:
- `/trustchain/INTEGRATION_TEST_REPORT.md`
- `/trustchain/PRODUCTION_DEPLOYMENT.md`
- `/trustchain/ECOSYSTEM_OVERVIEW.md`
**Problems**: Likely contain traditional infrastructure references
**Complexity**: Simple

### 10. Catalog Documentation
**Files**:
- `/catalog/README.md`
- `/catalog/SHARING_ARCHITECTURE.md`
- `/catalog/DISTRIBUTION_LAYER.md`
**Problems**: May reference traditional distribution models
**Complexity**: Medium

## Correction Guidelines

### What to Add to Every File:
1. **Matrix Topology**: BlockMatrix uses matrix-based resource topology
2. **Tensor Operations**: All compute operations are tensor-based
3. **Privacy Tiers**: Anonymous|Private|Federated|Public
4. **Block-MATRIX Trust**: Federated trust, not traditional CA/DNS
5. **Instruction-Based Retrieval**: Not DNS lookups
6. **Every Node is Blockchain**: Distributed consensus at every level

### What to Remove:
1. Traditional DNS/CA/CT references (except for bootstrap)
2. "Pure transport" descriptions of STOQ
3. HTTP/TCP comparisons that minimize STOQ's intelligence
4. Centralized trust model descriptions
5. Traditional cloud infrastructure analogies

## Implementation Strategy

### Phase 1 (Week 1): CRITICAL Files
- Fix main README.md and CLAUDE.md files
- Correct component-specific CLAUDE.md files
- Update STOQ and TrustChain READMEs

### Phase 2 (Week 2): HIGH Priority
- Fix architecture documentation
- Update lib.rs module docs
- Correct bootstrap roadmap

### Phase 3 (Week 3): MEDIUM Priority
- Update integration/test reports
- Fix catalog documentation
- Clean up remaining references

## Success Metrics
- Zero references to traditional CA/DNS (except bootstrap)
- Matrix topology explained in all architecture sections
- Privacy tiers documented consistently
- STOQ intelligence layer properly described
- Block-MATRIX trust model clear throughout

## Scale of Corrections Needed

### Quantitative Analysis
- **Total files with traditional references**: 272 files
- **Total occurrences**: 3,927 references to DNS/CA/certificates
- **Most affected directories**:
  - `/docs/` - Majority of documentation
  - `/trustchain/` - Entire component based on wrong model
  - `/blockmatrix/` - Mixed references throughout

### Files with Highest Density of Issues
1. `trustchain/COMPLETION_ANALYSIS.md` - 237 occurrences
2. `blockmatrix/core/runtime/docs/DEVELOPER_INTEGRATION_GUIDE.md` - 90 occurrences
3. `docs/TRUSTCHAIN_IMPLEMENTATION_TASKS.md` - 73 occurrences
4. `blockmatrix/core/tests/DNS_CT_TEST_SUITE_SUMMARY.md` - 56 occurrences

## Estimated Effort
- CRITICAL files: 3-4 days
- HIGH priority: 2-3 days
- MEDIUM priority: 2-3 days
- Full correction of all 272 files: 4-6 weeks
- **Recommendation**: Focus on CRITICAL+HIGH first (1-2 weeks), then systematic cleanup

---

**Note**: This plan addresses the fundamental architectural misrepresentation across 272 documentation files with nearly 4,000 incorrect references. The actual BlockMatrix system is a matrix-topology-based distributed compute platform with tensor operations and Block-MATRIX federated trust, not a traditional cloud replacement with DNS/CA infrastructure.