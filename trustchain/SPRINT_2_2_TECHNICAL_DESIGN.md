# Sprint 2.2 Technical Design: Production Quality Focus

## Executive Summary
**Sprint Goal**: Eliminate 371 unwrap() calls, fix 31 test failures, clean 26 TODO comments
**Duration**: 7 days
**Approach**: Systematic module-by-module quality improvement with automated verification

---

## Day 1-2: Unwrap Elimination (371 calls)

### Module Priority Order (by density and criticality)

#### Critical Path Modules (Day 1) - 162 unwraps
| Module | Count | Priority | Rationale |
|--------|-------|----------|-----------|
| `ct/storage.rs` | 26 | P0 | Core CT storage, data integrity critical |
| `dns/resolver.rs` | 21 | P0 | DNS resolution, network critical |
| `ct/fingerprint_tracker.rs` | 20 | P0 | Security tracking, audit critical |
| `security/alerts.rs` | 19 | P1 | Security monitoring |
| `crypto/kyber.rs` | 18 | P0 | Quantum-resistant crypto |
| `crypto/certificate.rs` | 17 | P0 | Certificate handling |
| `dns/cache.rs` | 16 | P1 | DNS caching layer |
| `crypto/falcon.rs` | 16 | P0 | Signature verification |
| `bin/trustchain-http3-server.rs` | 16 | P1 | Main server binary |

#### Secondary Modules (Day 2) - 209 unwraps
| Module | Count | Priority | Approach |
|--------|-------|----------|----------|
| `crypto/hybrid.rs` | 15 | P1 | Hybrid encryption |
| `dns/cert_validator.rs` | 14 | P1 | Certificate validation |
| `ct/merkle_log.rs` | 14 | P1 | Merkle tree operations |
| `crypto/mod.rs` | 12 | P2 | Module initialization |
| Remaining 21 modules | 154 | P2 | Standard error handling |

### Error Handling Patterns

#### Pattern 1: Option::unwrap() → Context-aware handling
```rust
// BEFORE (common in tests and initialization)
let storage = CTStorage::new(path.to_str().unwrap()).await.unwrap();

// AFTER
let storage = CTStorage::new(
    path.to_str()
        .ok_or_else(|| anyhow!("Invalid UTF-8 in path"))?
).await
.context("Failed to initialize CT storage")?;
```

#### Pattern 2: Result::unwrap() → Error propagation
```rust
// BEFORE (common in crypto operations)
let key = kyber::generate_keypair().unwrap();

// AFTER
let key = kyber::generate_keypair()
    .map_err(|e| StoqError::CryptoError(format!("Kyber key generation failed: {}", e)))?;
```

#### Pattern 3: Lock unwrap() → Poisoned lock handling
```rust
// BEFORE (common in shared state)
let guard = self.cache.lock().unwrap();

// AFTER
let guard = self.cache.lock()
    .map_err(|e| anyhow!("Cache lock poisoned: {}", e))?;
```

#### Pattern 4: Parse unwrap() → Validation
```rust
// BEFORE
let port: u16 = port_str.parse().unwrap();

// AFTER
let port: u16 = port_str.parse()
    .with_context(|| format!("Invalid port: {}", port_str))?;
```

### Automation Scripts

#### Day 1 Start: Baseline measurement
```bash
#!/bin/bash
# baseline.sh
echo "=== Unwrap Baseline ==="
for module in ct dns crypto security; do
    count=$(grep -r "unwrap()" src/$module --include="*.rs" | grep -v "test" | wc -l)
    echo "$module: $count unwraps"
done
echo "Total: $(grep -r "unwrap()" src/ --include="*.rs" | grep -v "test" | wc -l)"
```

#### Module-specific unwrap fix verification
```bash
#!/bin/bash
# verify_module.sh
MODULE=$1
before=$(git diff HEAD -- src/$MODULE | grep -c "\.unwrap()")
after=$(grep -c "\.unwrap()" src/$MODULE/*.rs 2>/dev/null | grep -v test)
echo "Removed $before unwraps from $MODULE"
echo "Remaining: $after"
cargo build --lib 2>&1 | grep -E "error|warning.*$MODULE"
```

---

## Day 3-4: Test Fixes (31 failures)

### Test Failure Categories

#### Category A: Crypto Test Failures (7 tests) - Day 3 Morning
| Test | Root Cause | Fix Strategy |
|------|------------|--------------|
| `test_kyber_encrypt_decrypt` | Key size mismatch | Update to latest pqc_kyber API |
| `test_hybrid_encryption_decryption` | Initialization order | Fix setup sequence |
| `test_falcon_sign_verify` | Invalid parameters | Use correct security level |

**Fix Pattern**:
```rust
// Common issue: outdated crypto API usage
// Solution: Update to match latest quantum-safe library versions
use pqc_kyber::{KYBER768_SECRETKEYBYTES, KYBER768_PUBLICKEYBYTES};
```

#### Category B: CT/Certificate Transparency (8 tests) - Day 3 Afternoon
| Test | Root Cause | Fix Strategy |
|------|------------|--------------|
| `test_certificate_logging` | Missing mock setup | Add proper test doubles |
| `test_inclusion_proof` | Merkle tree size | Fix tree initialization |
| `test_fingerprint_tracker` | Async timing | Add proper await points |

**Fix Pattern**:
```rust
// Common issue: Async test timing
// Solution: Proper tokio test harness
#[tokio::test]
async fn test_certificate_logging() {
    // Ensure all async operations complete
    let result = timeout(Duration::from_secs(5), async_op()).await;
}
```

#### Category C: DNS Resolution (6 tests) - Day 4 Morning
| Test | Root Cause | Fix Strategy |
|------|------------|--------------|
| `test_trustchain_domain_resolution` | Hardcoded addresses | Use test fixtures |
| `test_unknown_trustchain_domain` | Missing error case | Add proper error handling |
| `test_stats_update` | Race condition | Add synchronization |

#### Category D: Integration Tests (10 tests) - Day 4 Afternoon
| Test | Root Cause | Fix Strategy |
|------|------------|--------------|
| `test_consensus_validation` | Mock service missing | Implement test doubles |
| `test_security_dashboard` | Port conflicts | Dynamic port allocation |
| `test_secure_certificate_issuance` | Certificate chain | Fix test CA setup |

### Test Fix Automation

```bash
#!/bin/bash
# run_test_category.sh
CATEGORY=$1
case $CATEGORY in
    crypto)
        cargo test --lib crypto:: -- --nocapture
        ;;
    ct)
        cargo test --lib ct:: -- --nocapture
        ;;
    dns)
        cargo test --lib dns:: -- --nocapture
        ;;
    integration)
        cargo test --lib tests:: -- --nocapture
        ;;
esac
```

---

## Day 5-6: TODO Cleanup (26 comments)

### TODO Priority Matrix

#### Critical TODOs (Must Fix) - Day 5
| Location | TODO | Impact | Resolution |
|----------|------|--------|------------|
| `dns/mod.rs:292` | Implement STOQ DNS listener | Blocks DNS functionality | Implement basic listener |
| `crypto/certificate.rs:279` | CA-signed certificates | Security critical | Use proper CA chain |
| `api/stoq_api.rs:135` | PEM parsing | API functionality | Implement with x509-parser |
| `api/stoq_api.rs:188` | Extract CSR subject | Certificate issuance | Parse with openssl crate |

#### Deferred TODOs (Document Only) - Day 6
| Location | TODO | Impact | Action |
|----------|------|--------|--------|
| `consensus/mod.rs:77,89` | Replace with network generation | Future enhancement | Document migration path |
| `dns/stoq_transport.rs:208-209` | Parse actual domain/type | Enhancement | Add to backlog |
| `deployment/quality_gates.rs` | Security gate patterns | Testing only | Keep as documentation |

### TODO Resolution Patterns

#### Pattern 1: Implement Basic Functionality
```rust
// BEFORE
// TODO: Implement proper STOQ DNS service listener
unimplemented!()

// AFTER
pub async fn start_stoq_listener(addr: SocketAddr) -> Result<()> {
    let stoq_client = StoqClient::new(Default::default()).await?;
    stoq_client.listen(addr, handle_dns_request).await?;
    Ok(())
}
```

#### Pattern 2: Extract from Existing Data
```rust
// BEFORE
// TODO: Extract from CSR
common_name: "placeholder.trustchain.local".to_string(),

// AFTER
let csr = X509Req::from_pem(csr_pem)?;
let common_name = csr.subject_name()
    .entries_by_nid(Nid::COMMONNAME)
    .next()
    .ok_or_else(|| anyhow!("Missing CN in CSR"))?
    .data()
    .as_utf8()?
    .to_string();
```

#### Pattern 3: Convert to Tracked Issue
```rust
// BEFORE
// TODO: Replace all calls to this method with generate_from_network()

// AFTER
#[deprecated(since = "0.2.0", note = "Use generate_from_network() - See issue #142")]
```

---

## Day 7: Final Quality Pass

### Verification Checklist

#### Morning: Automated Verification
```bash
#!/bin/bash
# final_verify.sh

echo "=== Production Quality Metrics ==="

# Unwrap count (should be 0 in non-test code)
UNWRAPS=$(grep -r "unwrap()" src/ --include="*.rs" | grep -v "test" | wc -l)
echo "Unwraps remaining: $UNWRAPS (target: 0)"

# Test results
TEST_OUTPUT=$(cargo test --lib 2>&1)
PASSED=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= passed)')
FAILED=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= failed)')
echo "Tests: $PASSED passed, $FAILED failed (target: 0 failed)"

# TODO count
TODOS=$(grep -rn "TODO\|FIXME" src/ --include="*.rs" | grep -v "test" | wc -l)
echo "TODOs remaining: $TODOS (target: 0 critical)"

# Compilation
cargo build --release 2>&1 | grep -c "warning"
echo "Build warnings: $(cargo build 2>&1 | grep -c warning) (target: 0)"

# Clippy
cargo clippy -- -D warnings 2>&1 | grep -E "error|warning" | wc -l
echo "Clippy issues: $(cargo clippy 2>&1 | grep -c "warning") (target: 0)"
```

#### Afternoon: Manual Review
1. **Code Review Checklist**:
   - [ ] All error messages provide context
   - [ ] No silent failures
   - [ ] Logging at appropriate levels
   - [ ] Error types are specific (not generic anyhow)

2. **Test Coverage Analysis**:
   ```bash
   cargo tarpaulin --out Html --output-dir coverage/
   # Review coverage/index.html
   ```

3. **Security Scan**:
   ```bash
   cargo audit
   cargo outdated --depth 1
   ```

---

## Time Estimates

### Daily Breakdown

| Day | Task | Hours | Modules | Verification |
|-----|------|-------|---------|--------------|
| **Day 1** | Critical unwraps (P0) | 8 | ct/, crypto/ core | 162 → 0 unwraps |
| **Day 2** | Secondary unwraps | 8 | dns/, remaining | 209 → 0 unwraps |
| **Day 3** | Crypto/CT tests | 8 | 15 test fixes | cargo test crypto ct |
| **Day 4** | DNS/Integration tests | 8 | 16 test fixes | cargo test --lib |
| **Day 5** | Critical TODOs | 8 | 8 implementations | grep TODO critical |
| **Day 6** | Remaining TODOs | 8 | 18 defer/document | All TODOs addressed |
| **Day 7** | Quality verification | 8 | Full codebase | All metrics pass |

### Buffer Time Allocation
- Each day includes 1 hour buffer for unexpected issues
- Day 7 is entirely buffer/polish time
- Can parallelize test fixes if ahead of schedule

---

## Risk Mitigation

### Risk 1: Cascading Error Changes
**Mitigation**: Fix errors module by module, run tests after each module

### Risk 2: Breaking API Changes
**Mitigation**: Maintain error type compatibility, use #[deprecated] for transitions

### Risk 3: Performance Regression
**Mitigation**: Benchmark critical paths before/after error handling changes
```bash
cargo bench --bench trustchain_bench > before.txt
# After changes
cargo bench --bench trustchain_bench > after.txt
diff before.txt after.txt
```

### Risk 4: Hidden Unwraps in Macros
**Mitigation**: Search for macro-generated unwraps
```bash
cargo expand | grep -n "unwrap()"
```

---

## Success Metrics

### Quantitative Goals
- **Unwraps**: 371 → 0 (excluding tests)
- **Test Failures**: 31 → 0
- **Critical TODOs**: 8 → 0 (18 documented/deferred)
- **Build Warnings**: Current → 0
- **Clippy Issues**: Current → 0

### Qualitative Goals
- All errors provide meaningful context
- No silent failures in production paths
- Consistent error handling patterns
- Improved code maintainability

---

## Daily Verification Commands

```bash
# Day 1-2: After unwrap fixes
./verify_unwraps.sh

# Day 3-4: After test fixes
cargo test --lib -- --show-output | tee test_results.txt
grep "test result:" test_results.txt

# Day 5-6: After TODO cleanup
./verify_todos.sh

# Day 7: Final verification
./final_verify.sh
cargo clippy -- -D warnings
cargo fmt -- --check
```

---

## Implementation Notes

1. **Commit Strategy**: One commit per module group for easy rollback
2. **Testing**: Run tests after each module to catch regressions early
3. **Documentation**: Update inline docs when changing error handling
4. **Review**: Self-review each module before moving to next

This design provides a clear, actionable plan for the 7-day sprint with specific patterns, automation, and verification at each step.