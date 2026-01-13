# Root Directory Organization Audit - COMPLETE
**Date**: 2026-01-13
**Project**: Web3 Ecosystem (/home/persist/repos/projects/web3)
**Auditor**: Operations Tier 1 Agent (Data Analyst)

---

## Executive Summary

**CRITICAL FINDINGS**:
- Root directory contains **73 files** (target: ~12 essential files)
- **61 files require relocation** to proper directories
- **3 files require deletion** (old logs/PIDs from Sept 2025)
- **Impact**: 83% reduction in root directory clutter (73 → 12 files)

### Issues Found

| Issue | Count | Severity | Action |
|-------|-------|----------|--------|
| Shell scripts in root | 33 | HIGH | Move to /scripts/ |
| Specification files in root | 7 | MEDIUM | Move to /docs/specs/ |
| Report/doc files in root | 15 | MEDIUM | Move to /docs/ or /docs/archive/ |
| Python scripts in root | 3 | MEDIUM | Move to /scripts/ |
| JSON reports in root | 2 | LOW | Move to /docs/archive/ |
| Old log files | 2 | LOW | DELETE (already in .gitignore) |
| Stale PID file | 1 | LOW | DELETE |
| Demo Rust file | 1 | LOW | Move to /tests/ or DELETE |

---

## Current State Analysis

### File Count by Category

| Category | Count | Current Location | Target Location |
|----------|-------|------------------|-----------------|
| **Essential Files** | 8 | Root ✅ | Root (keep) |
| **Primary Scripts** | 3 | Root ✅ | Root (keep) |
| **Shell Scripts** | 33 | Root ❌ | /scripts/ |
| **Specification Files** | 7 | Root ❌ | /docs/specs/ |
| **Strategic Docs** | 6 | Root ⚠️ | /docs/ |
| **Historical Reports** | 9 | Root ❌ | /docs/archive/ |
| **Python Scripts** | 3 | Root ❌ | /scripts/ |
| **JSON Reports** | 2 | Root ❌ | /docs/archive/ |
| **Log Files** | 2 | Root ❌ | DELETE |
| **PID Files** | 1 | Root ❌ | DELETE |
| **Demo Files** | 1 | Root ❌ | /tests/ or DELETE |
| **Directories** | 20 | Root ✅ | Root (keep) |

**Total Files in Root**: 73 → **Target**: 12 files

---

## Essential Files (KEEP IN ROOT - 12 files)

✅ **Core Project Files** (8 files):
1. `README.md` - Project overview and documentation entry point
2. `CLAUDE.md` - AI assistant project context and instructions
3. `Cargo.toml` - Rust workspace configuration
4. `Cargo.lock` - Dependency lock file (version pinning)
5. `.gitignore` - Git exclusion rules
6. `.env.template` - Environment variable template
7. `Makefile` - Build automation and task runner
8. `docker-compose.yml` - Container orchestration configuration

✅ **Configuration** (1 file):
9. `hypermesh-native-config.toml` - HyperMesh native configuration

✅ **Primary Entry Point Scripts** (3 files - keep for convenience):
10. `build-all.sh` - Main build orchestrator
11. `deploy-all.sh` - Main deployment orchestrator
12. `sync-repos.sh` - Repository synchronization tool

**Rationale**: These 3 scripts are primary entry points frequently used from root. All other scripts should be in /scripts/.

---

## Detailed Cleanup Plan

### Phase 1: Delete Old Files (3 files)

**Log Files** (2 files - already in .gitignore):
```bash
rm internet2-server.log  # 362KB, from 2025-09-19
rm server.log            # 186KB, from 2025-09-19
```

**PID Files** (1 file):
```bash
rm internet2-server.pid  # 7 bytes, stale from 2025-11-12
```

**Action Required**: Add `*.pid` to .gitignore
```bash
echo "*.pid" >> .gitignore
```

---

### Phase 2: Move Scripts to /scripts/ (36 files)

#### Build Scripts (9 files)
```bash
git mv build-status.sh scripts/
git mv build-ui.sh scripts/
git mv cargo-build.sh scripts/
git mv fix-build.sh scripts/
git mv fix-dependencies.sh scripts/
git mv fix-trustchain-fields.sh scripts/
git mv demo_extension_loading.sh scripts/
git mv simple-demo.rs scripts/  # OR: rm simple-demo.rs (if obsolete)
```

#### Deployment Scripts (5 files)
```bash
git mv deploy-http3-stack.sh scripts/
git mv deploy-hypermesh.sh scripts/
git mv start-all-services.sh scripts/
git mv start-http3-production.sh scripts/
git mv stop-http3-production.sh scripts/
```

#### HTTP3 Testing/Management Scripts (10 files)
```bash
git mv check-http3-health.sh scripts/
git mv health-check.sh scripts/
git mv monitor-http3-stack.sh scripts/
git mv run-http3-servers.sh scripts/
git mv start-http3-servers.sh scripts/
git mv stop-http3-servers.sh scripts/
git mv test-http3-servers.sh scripts/
git mv validate-http3-health.sh scripts/
git mv verify-http3-startup.sh scripts/
git mv setup-local-dns.sh scripts/
```

#### Test Scripts (9 files)
```bash
git mv run_integration.sh scripts/
git mv run-tests.sh scripts/
git mv test_hardware_api.sh scripts/
git mv test-integration.sh scripts/
git mv test_trustchain_api.sh scripts/
git mv validate_performance.sh scripts/
git mv verify_phase1_fixes.sh scripts/
git mv measure_real_performance.sh scripts/
```

#### Python Security Scripts (3 files)
```bash
git mv security_audit.py scripts/
git mv security_remediation.py scripts/
git mv security_test_suite.py scripts/
```

---

### Phase 3: Move Specifications to /docs/specs/ (7 files)

```bash
git mv CERTIFICATE_ARCHITECTURE.spec docs/specs/
git mv INITIATIVE_1_TRUSTCHAIN.spec docs/specs/
git mv INITIATIVE_2_STOQ.spec docs/specs/
git mv INITIATIVE_3_CATALOG.spec docs/specs/
git mv INITIATIVE_4_HYPERMESH.spec docs/specs/
git mv INITIATIVE_5_REPOSITORY_CLEANUP.spec docs/specs/
git mv PROOF_OF_STATE_CONSENSUS_INTEGRATION.spec docs/specs/
```

---

### Phase 4: Move Strategic Documentation to /docs/ (6 files)

**Active Reference Documentation** (NOT archived):
```bash
# Check for duplicate first
if [ -f docs/INFRASTRUCTURE.md ]; then
  echo "WARNING: docs/INFRASTRUCTURE.md already exists - review before moving"
  # Compare files and decide which to keep
else
  git mv INFRASTRUCTURE.md docs/
fi

git mv BLOCKMATRIX_IMPLEMENTATION_ROADMAP.md docs/
git mv INTELLIGENCE_LAYER_ARCHITECTURE.md docs/
git mv PDL_ROADMAP.md docs/
git mv METRICS_DASHBOARD.md docs/
git mv REQUIRED_ENV_VARS.md docs/
```

---

### Phase 5: Archive Historical Reports to /docs/archive/ (12 files)

**Completed/Historical Documentation**:
```bash
git mv ALPN_FIX_SUMMARY.md docs/archive/
git mv DEDUPLICATION_CONSENSUS_REPORT.md docs/archive/
git mv DOCUMENTATION_CORRECTION_PLAN.md docs/archive/
git mv PERFORMANCE_CLAIMS_VALIDATION_REPORT.md docs/archive/
git mv PHASE1_VERIFICATION_PLAN.md docs/archive/
git mv QUALITY_REVIEW_DOCUMENTATION_VS_CODE.md docs/archive/
git mv SPRINT_1.1_SCOPE_DEFINITION.md docs/archive/
git mv SPRINT_2.3_COMPLETE.md docs/archive/
git mv STOQ_VERIFICATION_REPORT.md docs/archive/
git mv TECHNICAL_DESIGN_STOQ_INTEGRATION.md docs/archive/

# JSON Reports
git mv security_audit_report.json docs/archive/
git mv security_test_report.json docs/archive/
```

---

## Recommended Root Directory Structure (AFTER CLEANUP)

```
/home/persist/repos/projects/web3/
├── .env.template                ✅ Environment template
├── .gitignore                   ✅ Git exclusions
├── build-all.sh                 ✅ Primary build script
├── Cargo.lock                   ✅ Rust dependencies
├── Cargo.toml                   ✅ Rust workspace
├── CLAUDE.md                    ✅ AI context
├── deploy-all.sh                ✅ Primary deploy script
├── docker-compose.yml           ✅ Container config
├── hypermesh-native-config.toml ✅ HyperMesh config
├── Makefile                     ✅ Task automation
├── README.md                    ✅ Project overview
├── sync-repos.sh                ✅ Repo sync tool
│
├── api/                         [Directory - API components]
├── backups/                     [Directory - Backup storage]
├── blockmatrix/                 [Directory - BlockMatrix component]
├── caesar/                      [Directory - Caesar component]
├── catalog/                     [Directory - Catalog VM component]
├── certs/                       [Directory - Certificate storage]
├── data/                        [Directory - Runtime data]
├── docs/                        [Directory - Documentation]
├── gateway/                     [Directory - Gateway component]
├── hypermesh-ebpf/              [Directory - eBPF integration]
├── infrastructure/              [Directory - Infrastructure configs]
├── lib/                         [Directory - Shared libraries]
├── scripts/                     [Directory - ALL scripts (39 files)]
├── stoq/                        [Directory - STOQ component]
├── systemd/                     [Directory - Systemd configs]
├── target/                      [Directory - Build artifacts]
├── test-results/                [Directory - Test outputs]
├── tests/                       [Directory - Test suites]
├── trustchain/                  [Directory - TrustChain component]
└── ui/                          [Directory - UI components]
```

**Result**: 12 files + 20 directories in root (clean, organized, maintainable)

---

## Post-Cleanup Verification

### Verification Commands

```bash
# 1. Verify root directory file count
cd /home/persist/repos/projects/web3
ls -1 | grep -v "/$" | wc -l
# Expected: 12 files

# 2. Verify scripts directory
ls -1 scripts/ | wc -l
# Expected: ~39 files (3 existing + 36 moved)

# 3. Verify docs/specs
ls -1 docs/specs/ | wc -l
# Expected: ~10 files (3 existing + 7 moved)

# 4. Verify docs/archive
ls -1 docs/archive/ | grep -E "\.(md|json)$" | wc -l
# Expected: +12 new files

# 5. Verify docs/ strategic docs
ls -1 docs/*.md | wc -l
# Expected: +6 new files (if no duplicates)

# 6. Verify no log/pid files in root
ls -1 *.log *.pid 2>/dev/null | wc -l
# Expected: 0 files

# 7. Git status should show moves (not deletes/adds)
git status
# Expected: "renamed: old_path -> new_path" for all moved files
```

### Script Reference Updates Required

**Files that may reference moved scripts**:
1. `Makefile` - Check for hardcoded script paths
2. `build-all.sh` - May call other scripts
3. `deploy-all.sh` - May call other scripts
4. `.github/workflows/*` - CI/CD may reference scripts
5. Documentation files - May link to scripts

**Update Pattern**:
- Old: `./script-name.sh`
- New: `./scripts/script-name.sh`

---

## Risk Assessment

### LOW RISK ✅
- Moving scripts (no code changes, just file locations)
- Moving documentation files (no functional impact)
- Deleting old log/PID files (already ignored by git)
- Moving specs (documentation only)

### MEDIUM RISK ⚠️
- Scripts may have hardcoded relative paths to other scripts
- CI/CD pipelines may reference old script locations
- Documentation may link to old file paths
- Makefile targets may use old paths

### MITIGATION STRATEGIES
1. **Use `git mv`** to preserve file history
2. **Test all moved scripts** after relocation (run from new location)
3. **Update Makefile** to reference new script paths
4. **Search codebase** for hardcoded script references:
   ```bash
   grep -r "deploy-http3-stack.sh" .
   grep -r "security_audit.py" .
   ```
5. **Create temporary symlinks** if scripts break:
   ```bash
   ln -s scripts/script-name.sh script-name.sh
   ```
6. **Run full test suite** after cleanup
7. **Commit in phases** (delete logs → move scripts → move docs)

---

## Implementation Timeline

### Phase 1: Safety First (5 minutes)
- Remove 3 log/PID files
- Add `*.pid` to .gitignore
- Commit: "chore: remove old log and PID files"

### Phase 2: Move Scripts (15 minutes)
- Move 36 files to /scripts/
- Update Makefile/build-all.sh references
- Test critical scripts
- Commit: "refactor: organize scripts into /scripts/ directory"

### Phase 3: Move Specifications (5 minutes)
- Move 7 .spec files to /docs/specs/
- Commit: "docs: organize specifications into /docs/specs/"

### Phase 4: Move Documentation (10 minutes)
- Move 6 strategic docs to /docs/
- Move 12 historical reports to /docs/archive/
- Commit: "docs: organize documentation hierarchy"

### Phase 5: Verification (10 minutes)
- Run verification commands
- Test build/deploy scripts
- Run test suite
- Final review

**Total Estimated Time**: 45 minutes

---

## Cleanup Execution Script

**Complete automation** (review before running):

```bash
#!/bin/bash
# Root Directory Cleanup Script
# Generated: 2026-01-13

set -e  # Exit on error
cd /home/persist/repos/projects/web3

echo "=== Phase 1: Delete old files ==="
rm -f internet2-server.log server.log internet2-server.pid
echo "*.pid" >> .gitignore
git add .gitignore
git commit -m "chore: remove old log/PID files, update .gitignore"

echo "=== Phase 2: Move scripts to /scripts/ ==="
# Build scripts
git mv build-status.sh build-ui.sh cargo-build.sh fix-build.sh fix-dependencies.sh fix-trustchain-fields.sh demo_extension_loading.sh scripts/

# Deployment scripts
git mv deploy-http3-stack.sh deploy-hypermesh.sh start-all-services.sh start-http3-production.sh stop-http3-production.sh scripts/

# HTTP3 scripts
git mv check-http3-health.sh health-check.sh monitor-http3-stack.sh run-http3-servers.sh start-http3-servers.sh stop-http3-servers.sh test-http3-servers.sh validate-http3-health.sh verify-http3-startup.sh setup-local-dns.sh scripts/

# Test scripts
git mv run_integration.sh run-tests.sh test_hardware_api.sh test-integration.sh test_trustchain_api.sh validate_performance.sh verify_phase1_fixes.sh measure_real_performance.sh scripts/

# Python scripts
git mv security_audit.py security_remediation.py security_test_suite.py scripts/

# Demo file (or delete if obsolete)
git mv simple-demo.rs scripts/

git commit -m "refactor: organize scripts into /scripts/ directory"

echo "=== Phase 3: Move specifications to /docs/specs/ ==="
git mv CERTIFICATE_ARCHITECTURE.spec INITIATIVE_1_TRUSTCHAIN.spec INITIATIVE_2_STOQ.spec INITIATIVE_3_CATALOG.spec INITIATIVE_4_HYPERMESH.spec INITIATIVE_5_REPOSITORY_CLEANUP.spec PROOF_OF_STATE_CONSENSUS_INTEGRATION.spec docs/specs/
git commit -m "docs: organize specifications into /docs/specs/"

echo "=== Phase 4: Move documentation ==="
# Strategic docs to /docs/
if [ ! -f docs/INFRASTRUCTURE.md ]; then
  git mv INFRASTRUCTURE.md docs/
fi
git mv BLOCKMATRIX_IMPLEMENTATION_ROADMAP.md INTELLIGENCE_LAYER_ARCHITECTURE.md PDL_ROADMAP.md METRICS_DASHBOARD.md REQUIRED_ENV_VARS.md docs/

# Historical reports to /docs/archive/
git mv ALPN_FIX_SUMMARY.md DEDUPLICATION_CONSENSUS_REPORT.md DOCUMENTATION_CORRECTION_PLAN.md PERFORMANCE_CLAIMS_VALIDATION_REPORT.md PHASE1_VERIFICATION_PLAN.md QUALITY_REVIEW_DOCUMENTATION_VS_CODE.md SPRINT_1.1_SCOPE_DEFINITION.md SPRINT_2.3_COMPLETE.md STOQ_VERIFICATION_REPORT.md TECHNICAL_DESIGN_STOQ_INTEGRATION.md docs/archive/

git mv security_audit_report.json security_test_report.json docs/archive/

git commit -m "docs: organize documentation hierarchy"

echo "=== Phase 5: Verification ==="
echo "Files in root: $(ls -1 | grep -v '/$' | wc -l)"
echo "Scripts in /scripts/: $(ls -1 scripts/ | wc -l)"
echo "Specs in /docs/specs/: $(ls -1 docs/specs/ | wc -l)"

echo "=== Cleanup Complete ==="
git log --oneline -4
```

---

## Summary

**Impact**: Root directory organization improved by 83% (73 → 12 files)

**Benefits**:
- ✅ Clear project structure
- ✅ Easy navigation
- ✅ Better maintainability
- ✅ Professional organization
- ✅ Reduced cognitive load
- ✅ Improved onboarding for new developers

**Next Steps**:
1. Review this audit report
2. Execute cleanup script (or run commands manually)
3. Test all critical scripts and build processes
4. Update documentation references
5. Verify CI/CD pipelines still work
6. Commit and push changes

---

**Audit Complete** | Generated by Operations Tier 1 Agent | 2026-01-13
