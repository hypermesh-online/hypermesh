# SURGICAL DOCUMENTATION COMPRESSION PLAN

## CURRENT STATE ASSESSMENT

**Scattered Documentation Identified:**
- 33 files in hypermesh/ subdirectory
- 17 files in caesar/.claude/ context
- 8 files in docs/archive/claude-context/
- Significant redundancy and outdated content

## COMPRESSION STRATEGY

### PRESERVE (Essential - Keep in working directories)
1. **Core Project Files**:
   - `/README.md` - Main project overview (KEEP)
   - `/CLAUDE.md` - Project context (KEEP)
   - `/hypermesh/README.md` - Component overview (KEEP)
   - `/hypermesh/CLAUDE.md` - Component context (KEEP)

2. **Active Documentation**:
   - `/hypermesh/docs/` - Structured docs directory (KEEP)

### ARCHIVE (Move to docs/archive/)
3. **Redundant READMEs**:
   - Multiple component README files with duplicate content
   - Interface/examples README files with minimal content

4. **Session/Report Files**:
   - Test reports: nexus-test-report.md, SPRINT2_TEST_REPORT.md
   - Integration reports: MFN_INTEGRATION_TESTING_REPORT.md
   - Implementation status files
   - TODO.md files

5. **Historical/Context Files**:
   - CHANGELOG.md files (multiple versions)
   - Getting started guides (outdated)
   - Deployment guides (superseded by main docs)

### ELIMINATE (Delete completely)
6. **Obsolete Files**:
   - Duplicate context files in caesar/.claude/
   - Session continuity/command files
   - Node_modules documentation (npm dependencies)

## SURGICAL EXECUTION PLAN

**Phase 1**: Archive redundant documentation
**Phase 2**: Eliminate obsolete context files  
**Phase 3**: Consolidate remaining essential docs
**Phase 4**: Update cross-references

## TARGET STATE

**Working Directory Files** (<10 essential):
- Project root: README.md, CLAUDE.md
- Component roots: hypermesh/README.md, hypermesh/CLAUDE.md
- Structured docs: hypermesh/docs/ (consolidated)

**Archive Directory**: 
- docs/archive/scattered-docs/ (consolidated redundant content)
- docs/archive/claude-context/ (historical context)

## SUCCESS METRICS

Reduce from 58+ scattered files to <10 essential working files while preserving all functional information.