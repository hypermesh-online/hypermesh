# Documentation Standard

## Principles

1. **Single Source of Truth** - One authoritative location per topic
2. **Component Co-location** - Documentation lives with the code it documents
3. **Clear Lifecycle** - Permanent vs temporary, with explicit archive rules
4. **Anti-Duplication** - Search before create, consolidate always
5. **Automation First** - Generate from code where possible (API docs, type definitions)

## Documentation Types

### Required (Permanent)

These documents are maintained indefinitely and should only be updated, never archived.

| Document | Location | Purpose | Owner |
|----------|----------|---------|-------|
| `README.md` | Project root | Project overview, quick start, installation | Project Lead |
| `CLAUDE.md` | Project root | AI agent instructions, architecture context | Architects |
| `ARCHITECTURE.md` | `docs/technical/` | System architecture, design decisions | Architects |
| `CONTRIBUTING.md` | Project root | Contribution guidelines, coding standards | Project Lead |
| API Reference | `docs/api/` | Generated from code comments | Auto-generated |
| `CHANGELOG.md` | Project root | Version history, release notes | Release Manager |

### Component Documentation

Each major component maintains its own documentation co-located with source code:

```
/blockmatrix/
  ├── README.md              # Component overview, API surface
  ├── ARCHITECTURE.md        # Component-specific architecture
  └── examples/              # Usage examples

/stoq/
  ├── README.md
  ├── PROTOCOL.md            # STOQ protocol specification
  └── examples/

/trustchain/
  ├── README.md
  ├── CERTIFICATE_SPEC.md    # FALCON-1024 CA specification
  └── examples/
```

**Linking**: Component docs are symlinked from `/docs/technical/components/` for discoverability.

### Temporary Documentation (Must Archive)

These documents have a limited lifespan and must be archived when obsolete:

| Type | Purpose | Archive Trigger | Archive Location |
|------|---------|----------------|------------------|
| Deep Analysis | Investigation reports, technical deep-dives | Work complete OR 30 days | `docs/archive/analysis-YYYY/` |
| Sprint Status | Sprint progress, completion reports | Sprint complete | `docs/archive/sprints/sprint-X.X/` |
| Quality Reports | Code quality assessments, security audits | Next assessment OR 90 days | `docs/archive/quality-YYYY-MM/` |
| Migration Guides | One-time migration instructions | Migration complete | `docs/archive/migrations/` |
| RFC/ADR Archive | Superseded architecture decision records | Decision implemented OR rejected | `docs/archive/decisions/` |

**Maximum Age**: 30 days for analysis/status, 90 days for quality reports, archive immediately when obsolete.

### Forbidden Practices

**NEVER create these**:
- Duplicate documentation (same information in multiple locations)
- Timestamp-dated files in project root (`2025_STATUS.md`, `REPORT_JAN_13.md`)
- Manual API documentation (generate from code comments)
- Progress/status markdown files (use PDL system: `mcp__pdl__*` tools)
- Meeting notes in markdown (use `mcp__notepad__*` for temporary work notes)
- TODO lists in markdown (use PDL tasks)

**Why**: These violate Single Source of Truth, create maintenance burden, and become obsolete quickly.

## File Structure

```
/home/persist/repos/projects/web3/
├── README.md                           # Project overview
├── CLAUDE.md                           # AI agent context
├── CONTRIBUTING.md                     # Contribution guidelines
├── CHANGELOG.md                        # Version history
├── LICENSE                             # Project license
│
├── docs/
│   ├── DOCUMENTATION_STANDARD.md       # This file
│   │
│   ├── api/                            # Generated API documentation
│   │   ├── blockmatrix/
│   │   ├── stoq/
│   │   └── trustchain/
│   │
│   ├── technical/                      # Architecture and design
│   │   ├── ARCHITECTURE.md             # System architecture
│   │   ├── PROOF_OF_STATE.md           # Consensus specification
│   │   ├── ASSET_SYSTEM.md             # HyperMesh asset design
│   │   ├── PRIVACY_TIERS.md            # Privacy model specification
│   │   └── components/                 # Symlinks to component docs
│   │       ├── blockmatrix.md -> ../../../blockmatrix/README.md
│   │       ├── stoq.md -> ../../../stoq/README.md
│   │       └── trustchain.md -> ../../../trustchain/README.md
│   │
│   ├── guides/                         # User guides and tutorials
│   │   ├── getting-started.md
│   │   ├── deployment.md
│   │   └── asset-creation.md
│   │
│   └── archive/                        # Historical documentation
│       ├── analysis-2026/
│       ├── sprints/
│       │   ├── sprint-2.1/
│       │   └── sprint-2.2/
│       ├── quality-2026-01/
│       ├── migrations/
│       └── decisions/
│
├── blockmatrix/
│   ├── README.md                       # Component documentation
│   ├── ARCHITECTURE.md
│   └── examples/
│
├── stoq/
│   ├── README.md
│   ├── PROTOCOL.md
│   └── examples/
│
└── trustchain/
    ├── README.md
    ├── CERTIFICATE_SPEC.md
    └── examples/
```

## Lifecycle Rules

### Permanent Documentation

**Rule**: Update only, never delete or archive

**Process**:
1. Changes go through code review
2. Update via PR with rationale
3. Maintain version history in git
4. Breaking changes noted in CHANGELOG.md

**Applies to**: README, CLAUDE, ARCHITECTURE, CONTRIBUTING, API docs, CHANGELOG

### Temporary Documentation

**Rule**: Archive when work complete OR maximum age exceeded

**Process**:
1. **Create**: Place in appropriate temporary location
2. **Monitor**: Track creation date
3. **Archive**: When trigger condition met:
   ```bash
   # Example: Archive sprint 2.2 completion report
   mv docs/SPRINT_2.2_COMPLETE.md docs/archive/sprints/sprint-2.2/completion.md
   ```
4. **Clean**: Remove from main docs/ directory
5. **Link**: Update any references to archived location

**Triggers**:
- Analysis reports: Work complete OR 30 days
- Sprint status: Sprint complete
- Quality reports: Next report published OR 90 days
- Migration guides: Migration complete

### Component Documentation

**Rule**: Maintain with component, symlink for discoverability

**Process**:
1. **Create**: Documentation in component directory
2. **Symlink**: From `/docs/technical/components/` if needed
   ```bash
   ln -s ../../../blockmatrix/README.md docs/technical/components/blockmatrix.md
   ```
3. **Update**: With component code changes
4. **Version**: Tag with component version

## Anti-Duplication Protocol

### Before Creating Any .md File

**Step 1: Search Existing**
```bash
# Search for topic in all markdown files
find /home/persist/repos/projects/web3 -name "*.md" -exec grep -l "topic_keyword" {} \;

# Search archived docs
find /home/persist/repos/projects/web3/docs/archive -name "*.md" -exec grep -l "topic_keyword" {} \;

# Use MCP omni search
# mcp__omni__search({ query: "topic", scope: "notepad" })
```

**Step 2: Evaluate Results**
- **Exists in permanent docs**: Update existing file
- **Exists in component docs**: Update component README
- **Exists in archive**: Create link or reference, don't duplicate
- **Truly new**: Create in correct location per standard

**Step 3: Document Decision**
If creating new file, document in commit message:
- Why new file needed
- What searches were performed
- Why existing docs insufficient

### Consolidation Protocol

**When duplicates found**:

1. **Identify canonical version**: Most recent, most complete, best location
2. **Merge content**: Incorporate unique content from duplicates
3. **Create redirects**: If URLs exist, add redirect comments
4. **Delete duplicates**: Remove after content merged
5. **Update references**: Search and replace all links
6. **Document**: Note consolidation in CHANGELOG.md

**Example**:
```markdown
<!-- This file consolidated 2026-01-13 -->
<!-- Previous locations: /ASSET_DESIGN.md, /docs/ASSETS.md -->
<!-- Canonical location: /docs/technical/ASSET_SYSTEM.md -->
```

### Link, Don't Duplicate

**When multiple locations need same info**:
- **Use relative links**: `[Architecture](docs/technical/ARCHITECTURE.md)`
- **Use symlinks**: For file system references
- **Reference, don't copy**: Cite section, don't duplicate content

## Documentation Review Cadence

### Monthly Review (Automation Preferred)

**Tasks**:
- Archive temporary docs >30 days old
- Check for orphaned docs (no links to them)
- Identify potential duplicates
- Update outdated links

**Script**: `/scripts/docs-monthly-review.sh`

### Quarterly Review (Manual)

**Tasks**:
- Review for duplicate content, consolidate
- Update component documentation for API changes
- Archive quality reports >90 days
- Assess documentation gaps

**Owner**: Documentation Lead

### Release Review (Per Release)

**Tasks**:
- Update all permanent documentation
- Generate updated API docs from code
- Archive release-specific analysis
- Update CHANGELOG.md
- Version-tag documentation

**Owner**: Release Manager

## Enforcement

### CI/CD Checks

**Pre-commit Hook** (`scripts/git-hooks/pre-commit-docs.sh`):
```bash
#!/bin/bash
# Warn on new .md files in forbidden locations
# Fail if timestamp-dated files in root
# Check for duplicate content patterns
```

**CI Pipeline** (`.github/workflows/docs-check.yml`):
- Fail if .md files in root beyond approved list
- Fail if analysis/status docs >30 days old not archived
- Warn on potential duplicate content (fuzzy match)
- Validate all internal markdown links

### Automated Archival

**Cron Job** (`scripts/docs-auto-archive.sh`):
```bash
# Runs nightly
# Moves docs matching archive triggers to archive/
# Updates references automatically
# Reports actions via notification
```

### PDL Integration

**Status Tracking**: Use `mcp__pdl__step_update` for work status
- NOT markdown files like STATUS.md, PROGRESS.md

**Work Notes**: Use `mcp__notepad__*` for detailed work documentation
- NOT markdown files like WORK_LOG.md, NOTES.md

**Knowledge**: Use `mcp__memory__store` for lasting insights
- NOT markdown files like LEARNINGS.md, INSIGHTS.md

## Documentation Workflow

### Creating New Documentation

1. **Assess Need**: Is this permanent or temporary?
2. **Search First**: Run anti-duplication protocol
3. **Determine Location**: Use file structure rules
4. **Create with Metadata**:
   ```markdown
   ---
   title: Document Title
   created: 2026-01-13
   type: [permanent|temporary|component]
   archive_trigger: [if temporary]
   owner: [team|person]
   ---
   ```
5. **Link Appropriately**: Add to relevant indexes
6. **Document in PDL**: Note creation in step update

### Updating Existing Documentation

1. **Locate Canonical Source**: Follow symlinks, check for redirects
2. **Review Current Content**: Understand existing structure
3. **Make Changes**: Update canonical source only
4. **Update Metadata**: Note last updated date
5. **Validate Links**: Ensure references still valid
6. **Commit with Context**: Explain change in commit message

### Archiving Documentation

1. **Verify Archive Trigger**: Confirm work complete or age exceeded
2. **Create Archive Location**: Follow archive structure
3. **Move File**: `mv` to archive location
4. **Update References**: Search and update all links
5. **Document Archival**: Note in CHANGELOG.md
6. **Notify Stakeholders**: If widely referenced

## Special Cases

### Architecture Decision Records (ADR)

**Location**: `docs/technical/decisions/`
**Format**: `NNNN-title.md` (0001-proof-of-state-four-proofs.md)
**Lifecycle**: Permanent, but superseded ADRs moved to archive
**Status**: Active, Superseded, Deprecated

### API Documentation

**Source**: Rust doc comments (`///`)
**Generation**: `cargo doc --no-deps --document-private-items`
**Location**: `docs/api/` (generated)
**Review**: Updated with each release

### Runbooks and Operational Docs

**Location**: `docs/operations/`
**Type**: Permanent
**Review**: Quarterly, after incidents
**Owner**: Operations team

### Security Documentation

**Location**: `docs/security/` (restricted access if needed)
**Type**: Permanent
**Review**: After security audits
**Owner**: Security team

## Compliance Matrix

| Document Type | Max Age | Required Reviews | Archive Rule |
|---------------|---------|------------------|--------------|
| Analysis Report | 30 days | None | Age OR work complete |
| Sprint Status | Sprint duration | Sprint retro | Sprint complete |
| Quality Report | 90 days | Quarterly review | Next report published |
| RFC/ADR | Permanent | Implementation complete | When superseded → archive |
| API Docs | Per release | Release review | Regenerate, don't archive |
| Architecture | Permanent | Major changes only | Never archive |
| Component README | Permanent | Component changes | Never archive |

## Success Metrics

**Health Indicators**:
- Zero duplicate docs (same content >90% similar)
- No temporary docs >30 days in main docs/
- All component docs have symlinks
- 100% markdown links valid
- No timestamp-dated files in root

**Quality Metrics**:
- Documentation coverage: >80% public API documented
- Freshness: >90% docs updated within 6 months
- Discoverability: <3 clicks to any doc from README

## Questions and Exceptions

**Question**: Where do sprint retrospectives go?
**Answer**: `mcp__notepad__*` during sprint, archive to `docs/archive/sprints/sprint-X.X/retro.md` after

**Question**: What about diagrams and images?
**Answer**: Store with documentation (e.g., `docs/technical/diagrams/`), reference with relative links

**Question**: External documentation (Wiki, Confluence)?
**Answer**: Mirror structure, but this repo is canonical source

**Question**: Generated code examples?
**Answer**: `examples/` in component root, regenerate with tests

**Question**: Versioned documentation?
**Answer**: Git tags for versions, use GitHub Pages or docs.rs for published versions

## References

- [Divio Documentation System](https://documentation.divio.com/)
- [Architecture Decision Records](https://adr.github.io/)
- [Keep a Changelog](https://keepachangelog.com/)
- [Semantic Versioning](https://semver.org/)

---

**Last Updated**: 2026-01-13
**Owner**: Project Lead
**Review Cadence**: Quarterly
**Status**: Active
