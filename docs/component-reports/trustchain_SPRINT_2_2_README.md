# Sprint 2.2: Production Quality Focus - Implementation Guide

## Quick Start

```bash
# Day 1: Start unwrap elimination
git checkout -b sprint-2.2-day-1
./scripts/verify_unwraps.sh
# Follow docs/daily_execution_checklist.md

# Daily verification
./scripts/verify_unwraps.sh  # Track unwrap progress
./scripts/verify_todos.sh     # Track TODO progress
cargo test --lib              # Ensure tests pass

# Final verification (Day 7)
./scripts/final_verify.sh
```

---

## Sprint Overview

**Duration**: 7 days
**Objective**: Eliminate production code quality issues

### Targets
- **371 unwrap() calls** → 0 (production code only)
- **31 test failures** → 0 (all passing)
- **26 TODO comments** → 8 implemented, 18 documented

---

## Documentation Structure

### Core Design Document
📄 **SPRINT_2_2_TECHNICAL_DESIGN.md** - Main technical design
- Module priority ordering
- Error handling patterns
- Test fix categorization
- TODO resolution strategy
- Daily verification commands

### Detailed Guides
📄 **docs/error_handling_patterns.md** - Before/after code examples
- 15 common unwrap patterns with solutions
- Error type selection guide
- Testing patterns
- Quick reference table

📄 **docs/test_fix_plan.md** - Test failure analysis
- 6 test categories with root causes
- Fix strategies per category
- Daily execution schedule
- Verification commands

📄 **docs/todo_resolution_plan.md** - TODO implementation plan
- 8 critical TODOs with full implementations
- 18 deferred TODOs with tracking strategy
- Time estimates and dependencies
- Code quality checklist

📄 **docs/daily_execution_checklist.md** - Day-by-day execution
- Hour-by-hour schedules
- Per-task verification commands
- Emergency procedures
- Success criteria per day

---

## Automation Scripts

All scripts are in `./scripts/` and are executable.

### Progress Tracking
```bash
./scripts/verify_unwraps.sh     # Show unwrap count by module
./scripts/verify_todos.sh       # Show TODO categorization
./scripts/final_verify.sh       # Comprehensive quality check
```

### Development Helpers
```bash
./scripts/fix_module_unwraps.sh <module>     # Analyze module unwraps
./scripts/run_test_category.sh <category>    # Run specific test group
```

**Test Categories**: crypto, ct, dns, security, consensus, integration, all

---

## Daily Workflow

### Day 1-2: Unwrap Elimination (371 → 0)
**Focus**: Replace all unwrap() with proper error handling

**Day 1**: Critical modules (162 unwraps)
- ct/storage.rs, dns/resolver.rs, crypto modules
- Pattern: Apply error_handling_patterns.md examples

**Day 2**: Remaining modules (209 unwraps)
- Secondary modules, batch processing
- Verify: 0 unwraps in production code

**Key Command**:
```bash
./scripts/verify_unwraps.sh
```

---

### Day 3-4: Test Fixes (31 → 0)
**Focus**: Fix all failing tests

**Day 3**: Crypto, CT, Misc (15 tests)
- Crypto: Update pqc_kyber API usage
- CT: Fix async timing and storage
- Misc: Float precision, edge cases

**Day 4**: CA, DNS, Integration (16 tests)
- CA: Fix certificate chain setup
- DNS: Add test fixtures
- Integration: Dynamic port allocation

**Key Command**:
```bash
./scripts/run_test_category.sh <category>
cargo test --lib 2>&1 | grep "test result:"
```

---

### Day 5-6: TODO Resolution (26 TODOs)
**Focus**: Implement critical TODOs, document others

**Day 5**: Implement 8 critical TODOs
1. DNS STOQ listener
2. PEM certificate parsing
3. CSR subject extraction
4. CA-signed certificates
5. DNS record parsing
6. SAN extraction
7. Client address logging
8. Certificate chain building

**Day 6**: Document 18 deferred TODOs
- Create GitHub issues
- Add deprecation notices
- Write migration guides

**Key Command**:
```bash
./scripts/verify_todos.sh
```

---

### Day 7: Final Quality Pass
**Focus**: Comprehensive verification

**Checklist**:
- [ ] All verification scripts pass
- [ ] No build warnings
- [ ] Clippy clean
- [ ] Tests pass 10 consecutive times
- [ ] Documentation complete
- [ ] Ready for PR

**Key Command**:
```bash
./scripts/final_verify.sh
```

---

## Success Metrics

### Quantitative
| Metric | Before | Target | Verification |
|--------|--------|--------|--------------|
| Unwraps | 371 | 0 | `./scripts/verify_unwraps.sh` |
| Test Failures | 31 | 0 | `cargo test --lib` |
| Critical TODOs | 8 | 0 | `./scripts/verify_todos.sh` |
| Build Warnings | ~30 | 0 | `cargo build --lib` |
| Clippy Issues | ~50 | 0 | `cargo clippy -- -D warnings` |

### Qualitative
- All errors provide meaningful context
- No silent failures in production paths
- Consistent error handling patterns
- Improved code maintainability
- All deferred work tracked

---

## Error Handling Patterns Quick Reference

### Most Common Patterns

```rust
// 1. Path conversion
path.to_str().ok_or_else(|| anyhow!("Invalid UTF-8"))?

// 2. Lock handling
lock.lock().map_err(|e| anyhow!("Lock poisoned: {}", e))?

// 3. Parsing
value.parse().with_context(|| format!("Invalid value: {}", value))?

// 4. Option to Result
option.ok_or_else(|| anyhow!("Missing required value"))?

// 5. Context addition
async_op().await.context("Operation failed")?
```

See `docs/error_handling_patterns.md` for 15 complete patterns.

---

## Test Fix Quick Reference

### Common Test Issues

```rust
// 1. Async timing
#[tokio::test]
async fn test() {
    operation().await;
    tokio::time::sleep(Duration::from_millis(10)).await;
}

// 2. Float comparisons
assert!((actual - expected).abs() < 0.001);

// 3. Dynamic ports
let port = get_free_port();

// 4. Service readiness
wait_for_service(addr, Duration::from_secs(5)).await?;
```

See `docs/test_fix_plan.md` for category-specific strategies.

---

## TODO Implementation Quick Reference

### Critical TODOs Template

```rust
// BEFORE
// TODO: Implement feature
unimplemented!()

// AFTER
/// Implement feature with proper error handling
pub async fn feature(&self, param: Type) -> Result<Output> {
    // Validate input
    validate_param(param)?;

    // Perform operation
    let result = operation(param).await
        .context("Operation failed")?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_feature() {
        let result = feature(test_param).await;
        assert!(result.is_ok());
    }
}
```

See `docs/todo_resolution_plan.md` for all 8 implementations.

---

## Git Workflow

### Branch Strategy
```bash
# Create daily branches
git checkout -b sprint-2.2-day-1

# Commit per module/category
git add src/ct/storage.rs
git commit -m "fix: eliminate unwraps in ct/storage.rs"

# Push daily
git push origin sprint-2.2-day-1
```

### Commit Message Convention
```
Day 1: feat: eliminate unwraps in P0 modules
Day 2: feat: complete unwrap elimination (371→0)
Day 3: fix: resolve crypto and CT test failures
Day 4: fix: resolve CA, DNS, and integration tests
Day 5: feat: implement critical TODOs
Day 6: docs: document deferred TODOs
Day 7: chore: final quality verification
```

---

## Troubleshooting

### Build Breaks
```bash
# Reset to last known good state
git stash
cargo build --lib
git stash pop

# Apply changes incrementally
git add -p
```

### Test Flakiness
```bash
# Run multiple times to identify pattern
for i in {1..10}; do cargo test test_name; done

# Add debug logging
RUST_LOG=debug cargo test test_name -- --nocapture
```

### Behind Schedule
1. Review daily_execution_checklist.md
2. Identify time sinks
3. Adjust priorities (focus on critical items)
4. Document decisions in sprint notes
5. Communicate any blockers

---

## Dependencies

### Required Crate Additions (Day 5)
```toml
[dependencies]
x509-parser = "0.15"  # Certificate parsing
pem = "3.0"            # PEM format handling
```

Already present (verify versions):
- anyhow (error handling)
- openssl (CSR parsing)
- trust-dns-proto (DNS messages)
- tokio (async runtime)

---

## Verification Commands Summary

```bash
# Start of day baseline
./scripts/verify_unwraps.sh > baseline_day_N.txt

# After module changes
cargo build --lib
cargo test --lib <module>::

# After test fixes
./scripts/run_test_category.sh <category>

# After TODO implementation
cargo test --lib -- --show-output

# End of day
./scripts/verify_unwraps.sh
./scripts/verify_todos.sh
cargo test --lib
git status

# End of sprint
./scripts/final_verify.sh
```

---

## Support Resources

### Documentation
- `SPRINT_2_2_TECHNICAL_DESIGN.md` - Overall strategy
- `docs/error_handling_patterns.md` - Code examples
- `docs/test_fix_plan.md` - Test strategies
- `docs/todo_resolution_plan.md` - Implementation details
- `docs/daily_execution_checklist.md` - Hour-by-hour guide

### Scripts
- `scripts/verify_unwraps.sh` - Unwrap tracking
- `scripts/verify_todos.sh` - TODO tracking
- `scripts/final_verify.sh` - Comprehensive check
- `scripts/fix_module_unwraps.sh` - Module analysis
- `scripts/run_test_category.sh` - Category testing

### External Resources
- [anyhow docs](https://docs.rs/anyhow) - Error handling
- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [tokio testing](https://tokio.rs/tokio/topics/testing)

---

## Sprint Success Criteria

### Code Quality
- [ ] 0 unwraps in production code
- [ ] 0 test failures
- [ ] 0 critical TODOs
- [ ] 0 build warnings
- [ ] 0 clippy warnings

### Testing
- [ ] All tests pass
- [ ] No flaky tests (10 consecutive passes)
- [ ] Test coverage maintained or improved

### Documentation
- [ ] All changes documented
- [ ] Migration guides for deprecations
- [ ] GitHub issues for deferred work

### Deliverables
- [ ] Final verification report
- [ ] Sprint summary
- [ ] PR ready for review
- [ ] Changelog updated

---

## Contact & Questions

- Review technical design first: `SPRINT_2_2_TECHNICAL_DESIGN.md`
- Check daily checklist: `docs/daily_execution_checklist.md`
- Run verification: `./scripts/final_verify.sh`
- Consult error patterns: `docs/error_handling_patterns.md`

**Remember**: This sprint focuses on production quality. Every change should improve reliability, maintainability, and debuggability.

---

**Sprint 2.2: Production Quality Focus**
*Eliminating unwraps, fixing tests, resolving TODOs - making TrustChain production-ready*