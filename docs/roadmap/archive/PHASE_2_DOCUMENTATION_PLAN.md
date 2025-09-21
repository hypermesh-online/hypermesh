# Phase 2: Documentation Consolidation Plan

## Current State Analysis
- **171 markdown files** scattered across the repository
- Multiple overlapping topics and duplicate content
- No clear documentation hierarchy
- Mixed technical/business/development docs

## Consolidation Strategy

### 1. Primary Documentation Structure
```
/docs/
├── README.md                    # Main entry point
├── ARCHITECTURE.md              # Core system architecture
├── DEPLOYMENT.md                # Deployment guide
├── DEVELOPMENT.md               # Development setup
│
├── components/                  # Component-specific docs
│   ├── caesar/                  # Caesar economic system
│   ├── hypermesh/              # HyperMesh asset system
│   ├── stoq/                   # STOQ protocol
│   ├── trustchain/             # TrustChain certificates
│   ├── catalog/                # Catalog VM
│   └── ngauge/                 # NGauge engagement
│
├── guides/                      # User/developer guides
│   ├── getting-started.md
│   ├── api-reference.md
│   └── integration.md
│
├── reports/                     # Historical reports/audits
│   ├── security/
│   ├── performance/
│   └── testing/
│
└── roadmap/                     # Planning documents
    ├── current.md
    └── archive/
```

### 2. Consolidation Categories

#### A. Main Documentation (Keep in root)
- README.md (consolidated from multiple)
- ARCHITECTURE.md (merge with technical docs)
- DEPLOYMENT_GUIDE.md → DEPLOYMENT.md
- CLAUDE.md (project context - keep as-is)

#### B. Component Documentation (Move to /docs/components/)
**Caesar (24 files):**
- Merge deployment/integration docs
- Consolidate economic model docs
- Archive old sprint/phase docs

**HyperMesh (31 files):**
- Merge implementation summaries
- Consolidate hardware adapter docs
- Keep core architecture docs

**STOQ (8 files):**
- Merge performance reports
- Keep protocol architecture
- Consolidate integration docs

**TrustChain (10 files):**
- Merge UI consolidation docs
- Keep architecture doc
- Consolidate implementation docs

#### C. Reports & Audits (Move to /docs/reports/)
- Security audits (5 files)
- Performance reports (4 files)
- Testing results (8 files)
- Validation reports (6 files)

#### D. Redundant/Archive (Consider removal)
- Old sprint documents
- Duplicate READMEs
- Outdated roadmaps
- Temporary analysis files

### 3. Content Preservation Rules
1. **Never delete** unique technical specifications
2. **Always merge** duplicate content, keeping the best version
3. **Archive** historical documents that provide context
4. **Consolidate** scattered information on same topic
5. **Preserve** all API documentation and integration guides

### 4. Execution Steps
1. Create /docs directory structure
2. Start with low-risk moves (reports, guides)
3. Consolidate component docs by system
4. Merge duplicate content carefully
5. Update all cross-references
6. Create navigation index

## Files to Consolidate (Priority Order)

### High Priority (Multiple duplicates)
1. README files (7 instances) → Single comprehensive README
2. Integration docs (12 files) → Unified integration guide
3. Implementation summaries (15 files) → Per-component summaries
4. Testing/validation reports (14 files) → /docs/reports/testing/

### Medium Priority (Related content)
1. Architecture docs (8 files) → Core + component architectures
2. Security reports (5 files) → /docs/reports/security/
3. Performance analyses (4 files) → /docs/reports/performance/
4. Deployment guides (6 files) → Single deployment guide

### Low Priority (Historical/Archive)
1. Sprint documents (4 files) → /docs/roadmap/archive/
2. Phase completion reports (8 files) → /docs/roadmap/archive/
3. Old roadmaps (5 files) → /docs/roadmap/archive/
4. Temporary analyses (6 files) → Review for deletion

## Success Metrics
- Reduce from 171 to ~50 organized files
- Clear navigation hierarchy
- No duplicate content
- Professional documentation structure
- All valuable content preserved
- Improved discoverability