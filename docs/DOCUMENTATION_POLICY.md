# Documentation Policy

**Version**: 1.0
**Effective Date**: 2026-01-13
**Owner**: Project Maintainers

## Purpose

This policy establishes clear rules for documentation creation, maintenance, and lifecycle management to prevent bloat, duplication, and organizational decay.

---

## Core Principles

### 1. Single Source of Truth (SSOT)
- **Every concept has ONE authoritative document**
- Before creating new documentation, search existing docs
- Update existing docs rather than creating alternatives
- Consolidate duplicate information immediately

### 2. Location Standards

| Document Type | Location | Examples |
|--------------|----------|----------|
| **Project Overview** | Root `README.md` | Project description, quick start |
| **Architecture** | Root `ARCHITECTURE.md` | System design, component relationships |
| **Component Docs** | Component root `README.md` | Component-specific functionality |
| **API Documentation** | `docs/api/` | API specifications, endpoints |
| **Development Guides** | `docs/guides/` | Setup, contributing, testing guides |
| **Design Documents** | `docs/design/` | RFCs, design proposals (active) |
| **Historical Archives** | `docs/archive/YYYY/` | Completed analyses, old reports |

### 3. Anti-Duplication Protocol

**SEARCH BEFORE CREATE**:
```bash
# Check existing documentation
grep -r "concept_name" docs/ *.md
find . -name "*keyword*.md"

# Search archives
find docs/archive/ -name "*keyword*.md"
```

**UPDATE BEFORE DUPLICATE**:
- Found existing doc → Update it
- Found outdated doc → Refresh it
- Found archived doc → Extract relevant parts into active doc

**NEVER CREATE**:
- `_simple.*`, `_fixed.*`, `_new.*`, `_v2.*` variants
- Duplicate guides (e.g., multiple "Getting Started")
- Temporary analysis files in root (use `docs/archive/`)

---

## Document Lifecycle

### Active Phase
- **Location**: Root or `docs/`
- **Maintenance**: Keep current, update regularly
- **Examples**: README.md, ARCHITECTURE.md, API docs

### Completed/Historical Phase
- **Trigger**: Analysis complete, sprint finished, snapshot taken
- **Action**: Move to `docs/archive/YYYY/`
- **Examples**: Completion reports, status snapshots, audits

### Archive Organization
```
docs/
├── archive/
│   ├── 2025/
│   │   ├── completion-snapshots/       # Component completion reports
│   │   ├── performance-analyses/       # Performance studies
│   │   └── security-audits/           # Security assessments
│   └── 2026/
│       ├── root-analysis/             # Root-level analyses
│       └── planning-docs/             # Historical planning
```

### Archive Policy
- Archive after 3 months of no updates
- Archive when superseded by new documentation
- Archive completion/status reports immediately after sprint end
- Preserve git history when archiving (use `git mv`)

---

## Documentation Types

### 1. Production Documentation (Active)
**Audience**: Users, developers, contributors
**Lifecycle**: Permanent, updated continuously
**Location**: Root or `docs/`

- `README.md` - Project overview, quick start
- `ARCHITECTURE.md` - System design
- `CONTRIBUTING.md` - Contribution guide
- `docs/api/` - API specifications
- `docs/guides/` - How-to guides

### 2. Analysis/Reports (Temporary)
**Audience**: Team, stakeholders
**Lifecycle**: Archive after completion
**Location**: `docs/archive/YYYY/`

- Completion reports (`*_COMPLETION_*.md`)
- Status snapshots (`*_STATUS_*.md`)
- Audit reports (`*_AUDIT_*.md`)
- Performance analyses (`*_PERFORMANCE_*.md`)

### 3. Design Documents (Active → Archive)
**Audience**: Architects, senior developers
**Lifecycle**: Active during design, archive when implemented
**Location**: `docs/design/` → `docs/archive/YYYY/design/`

- RFCs, design proposals
- Architecture decision records (ADRs)

---

## Quality Standards

### Minimum Requirements
- Clear title and purpose statement
- Table of contents for docs >200 lines
- Last updated date
- Owner/maintainer identified
- Links validated (no broken references)

### Prohibited Content
- Duplicate information (consolidate instead)
- Placeholder/stub documents (complete or delete)
- Outdated information (update or archive)
- Temporary analysis in root directory (use `docs/archive/`)
- Personal notes/scratchpads (use notepad MCP tool)

### Review Triggers
- Quarterly documentation audit
- Before major releases
- When creating new related documentation
- After component completion

---

## Enforcement

### Pre-Commit Checks
- No duplicate filenames (case-insensitive)
- No files matching `*_v2.*`, `*_simple.*`, `*_fixed.*`
- No `*_COMPLETION_*.md` or `*_STATUS_*.md` in root
- Archive files only in `docs/archive/`

### Violation Remediation
1. **Duplicate Found**: Consolidate immediately, delete duplicate
2. **Outdated Doc**: Update or archive within 1 week
3. **Misplaced File**: Move to correct location
4. **Bloat Detected**: Archive or delete within 2 weeks

### Audit Schedule
- **Monthly**: Check for duplicates and outdated docs
- **Quarterly**: Full documentation structure review
- **Annually**: Archive cleanup (delete archives >3 years old)

---

## Tools and Commands

### Search for Duplicates
```bash
# Find similar filenames
find . -name "*.md" -type f | sort | uniq -d

# Search content similarity
grep -r "unique phrase" docs/ *.md

# Check file sizes (detect bloat)
find docs/ -name "*.md" -size +50k -exec ls -lh {} \;
```

### Archive Files
```bash
# Preserve git history
git mv OLD_FILE.md docs/archive/2026/

# Bulk archive with prefix
find . -name "*_COMPLETION_*.md" -exec git mv {} docs/archive/2025/completion-snapshots/ \;
```

### Validate Links
```bash
# Check for broken markdown links
grep -r "\[.*\](.*)" docs/ *.md | grep -v "http" | while read line; do
  # Validate file references
done
```

---

## Maintenance Responsibilities

### Document Owners
- Keep documentation current
- Review quarterly for accuracy
- Archive when no longer active
- Consolidate duplicate information

### Project Maintainers
- Enforce documentation policy
- Conduct quarterly audits
- Approve new documentation structure
- Manage archive lifecycle

### Contributors
- Follow location standards
- Search before creating
- Update existing docs when possible
- Mark outdated content for review

---

## Revision History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-01-13 | Initial policy creation after bloat audit |

---

## Related Documents

- `CONTRIBUTING.md` - Contribution guidelines
- `docs/archive/2026/root-analysis/DOCUMENTATION_BLOAT_AUDIT_2026.md` - Initial audit that prompted this policy
