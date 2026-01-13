# Phase 1 Verification Plan

## Overview
Three parallel agents are fixing critical issues. This document defines comprehensive verification criteria and test procedures to validate correctness.

## 1. WORKSPACE CONFIGURATION FIX (hypermesh → blockmatrix)

### Success Criteria
- [ ] All `hypermesh` references in Cargo.toml files updated to `blockmatrix`
- [ ] Workspace builds successfully with `cargo build --workspace`
- [ ] No missing dependencies or unresolved module paths
- [ ] All internal imports correctly reference `blockmatrix` instead of `hypermesh`
- [ ] Test suite passes without import errors

### Verification Commands
```bash
# 1.1 Check no remaining hypermesh references in Cargo files
grep -r "hypermesh" --include="Cargo.toml" . | grep -v "keywords\|description\|comment"

# 1.2 Verify workspace builds
cargo clean
cargo build --workspace 2>&1 | tee build_output.log
grep -i "error\|unresolved" build_output.log

# 1.3 Check all member crates build individually
for crate in stoq trustchain caesar catalog blockmatrix; do
    echo "Building $crate..."
    cargo build -p $crate
done

# 1.4 Verify internal imports are correct
grep -r "use hypermesh::" --include="*.rs" .
grep -r "extern crate hypermesh" --include="*.rs" .

# 1.5 Run basic tests
cargo test --workspace --lib
```

### Expected Results
- Zero grep results for hypermesh references (except in descriptions/keywords)
- All crates build without errors
- No unresolved import errors
- Tests compile and run (may fail functionally, but should compile)

### Potential Issues to Watch
- Circular dependencies between crates
- Path dependencies that need updating
- Binary targets that reference old paths
- Documentation files with code examples

---

## 2. AWS CREDENTIALS REMOVAL

### Success Criteria
- [ ] No AWS access keys (AKIA* pattern) in codebase
- [ ] No AWS secret keys in any file
- [ ] Environment variable usage documented in .env.example files
- [ ] CI/CD workflows use proper secret references
- [ ] No hardcoded credentials in any configuration file

### Verification Commands
```bash
# 2.1 Scan for AWS access key patterns
grep -r "AKIA[0-9A-Z]{16}" . --exclude-dir=.git --exclude="*.log"

# 2.2 Scan for common AWS credential patterns
grep -rEi "aws[_-]?(access[_-]?key|secret)" . \
    --exclude-dir=.git \
    --exclude-dir=node_modules \
    --exclude="*.log" \
    --exclude="PHASE1_VERIFICATION_PLAN.md"

# 2.3 Check for base64 encoded credentials
grep -r "[A-Za-z0-9+/]{40}" . --include="*.yml" --include="*.yaml" --include="*.json" --include="*.toml"

# 2.4 Verify .env.example files exist and document needed vars
find . -name ".env.example" -exec echo "Found: {}" \; -exec cat {} \;

# 2.5 Check GitHub workflows for proper secret usage
grep -r "secrets\." .github/workflows/ || echo "No secrets found in workflows"
grep -r "\${{" .github/workflows/ | grep -v "secrets\."

# 2.6 Infrastructure files check
for file in infrastructure/*.sh; do
    echo "Checking $file..."
    grep -E "AKIA|aws_access_key|aws_secret" "$file" || echo "  Clean"
done
```

### Expected Results
- Zero matches for actual AWS credentials
- Only references to GitHub secrets (secrets.AWS_ACCESS_KEY_ID) in workflows
- .env.example files document required environment variables
- No base64 encoded credentials

### Potential Issues to Watch
- Credentials in git history (need BFG or filter-branch if found)
- Credentials in archived/backup files
- Credentials in test fixtures
- Hardcoded S3 bucket names or region configs

---

## 3. PROOF_OF_STATE → PROOF OF STATE RENAMING

### Success Criteria
- [ ] All `Proof of State` references updated to `Proof of State` or appropriate context
- [ ] Module paths updated from `proof_of_state` to `proof_of_state`
- [ ] No broken imports or module references
- [ ] Documentation updated with new terminology
- [ ] Test files and examples updated

### Verification Commands
```bash
# 3.1 Check for remaining Proof of State references
grep -r "Proof of State\|proof_of_state" . \
    --exclude-dir=.git \
    --exclude-dir=node_modules \
    --exclude="*.log" \
    --exclude="PHASE1_VERIFICATION_PLAN.md" \
    --exclude="CLAUDE.md"

# 3.2 Verify module paths are updated
find . -type f -name "*.rs" -exec grep -l "mod proof_of_state" {} \;
find . -type f -name "*.rs" -exec grep -l "use.*proof_of_state" {} \;

# 3.3 Check file/directory names
find . -name "*proof_of_state*" -type f
find . -name "*proof_of_state*" -type d

# 3.4 Verify Proof of State module exists and is properly referenced
ls -la lib/src/proof_of_state/
grep -r "use.*proof_of_state" --include="*.rs" .

# 3.5 Check that consensus references are consistent
grep -r "Four-Proof\|four-proof\|4-proof" . --include="*.rs" --include="*.md"

# 3.6 Verify tests still reference correct modules
cargo test --workspace --lib 2>&1 | grep -i "proof_of_state\|unresolved"
```

### Expected Results
- No Proof of State references except in historical context (CLAUDE.md)
- All imports use `proof_of_state` module path
- Consistent terminology throughout codebase
- Tests compile without unresolved module errors

### Potential Issues to Watch
- Frontend JavaScript files with old references
- Documentation in multiple locations
- Test file names that need renaming
- Example code in comments
- External documentation or README files

---

## 4. COMPREHENSIVE BUILD VERIFICATION

### Final Integration Test
```bash
# 4.1 Clean build from scratch
cargo clean
rm -rf target/

# 4.2 Full workspace build with all features
cargo build --workspace --all-features

# 4.3 Run all tests
cargo test --workspace --all-features

# 4.4 Check for warnings
cargo build --workspace 2>&1 | grep -i "warning"

# 4.5 Clippy check for code quality
cargo clippy --workspace --all-features -- -D warnings

# 4.6 Documentation build
cargo doc --workspace --no-deps
```

### Success Matrix
| Component | Build | Tests | No Warnings | Clippy Clean | Docs Build |
|-----------|-------|-------|-------------|--------------|------------|
| stoq      | ✓     | ✓     | ✓           | ✓            | ✓          |
| trustchain| ✓     | ✓     | ✓           | ✓            | ✓          |
| caesar    | ✓     | ✓     | ✓           | ✓            | ✓          |
| catalog   | ✓     | ✓     | ✓           | ✓            | ✓          |
| blockmatrix| ✓    | ✓     | ✓           | ✓            | ✓          |

---

## 5. SECURITY VERIFICATION

### Additional Security Checks
```bash
# 5.1 Check for any remaining sensitive patterns
grep -r "password\|secret\|token\|key" . \
    --include="*.rs" \
    --include="*.toml" \
    --include="*.yml" | \
    grep -v "pub\|struct\|fn\|//"

# 5.2 Verify no localhost/hardcoded IPs
grep -r "127\.0\.0\.1\|localhost" . \
    --include="*.rs" \
    --exclude-dir=tests \
    --exclude-dir=examples

# 5.3 Check for TODO/FIXME security items
grep -r "TODO.*security\|FIXME.*security" . --include="*.rs"
```

---

## Verification Timeline

### Phase 1: Quick Validation (5 minutes)
1. Run credential scans
2. Check for Proof of State references
3. Verify workspace members

### Phase 2: Build Verification (10 minutes)
1. Clean build workspace
2. Individual crate builds
3. Check for compilation errors

### Phase 3: Deep Verification (15 minutes)
1. Run full test suite
2. Clippy analysis
3. Documentation generation
4. Security pattern scanning

### Total Estimated Time: 30 minutes

---

## Rollback Plan

If any verification fails:

1. **Workspace issues**:
   - Revert Cargo.toml changes
   - Fix import paths incrementally

2. **Credential issues**:
   - Immediately rotate any exposed credentials
   - Update secret management
   - Scan git history

3. **Naming issues**:
   - Use sed/awk for bulk corrections
   - Verify with incremental builds

---

## Success Declaration

Phase 1 fixes are considered successful when:
- [ ] All verification commands return expected results
- [ ] Full workspace builds without errors
- [ ] No security vulnerabilities detected
- [ ] All tests compile (functional failures acceptable)
- [ ] Documentation generates without errors

## Monitoring Post-Fix

After fixes are applied:
1. Monitor CI/CD pipeline for failures
2. Check developer feedback on build issues
3. Track any runtime errors in logs
4. Verify no performance degradation