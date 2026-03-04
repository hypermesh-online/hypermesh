# Sprint 2.2 Daily Execution Checklist

## Daily Workflow Pattern

```bash
# Start of day
git checkout -b sprint-2.2-day-N
./scripts/verify_unwraps.sh > baseline_day_N.txt

# During work
# Make changes, test frequently
cargo test --lib <module>::

# End of day
cargo build --lib
cargo test --lib
git add .
git commit -m "Day N: [summary]"
git push origin sprint-2.2-day-N
```

---

## Day 1: Critical Unwraps (P0 Modules)

### Morning: CT Storage & DNS Resolver (162 unwraps target)

#### Module 1: ct/storage.rs (26 unwraps)
- [ ] **9:00-9:30** Review unwrap locations
  ```bash
  grep -n "unwrap()" src/ct/storage.rs | head -26
  ```
- [ ] **9:30-11:00** Fix unwraps in ct/storage.rs
  - Pattern: `TempDir::new().unwrap()` → proper error handling
  - Pattern: `.to_str().unwrap()` → `ok_or_else`
  - Pattern: `.await.unwrap()` → `context()`
- [ ] **11:00-11:15** Verify ct/storage.rs
  ```bash
  cargo build --lib
  cargo test --lib ct::storage
  grep -c "unwrap()" src/ct/storage.rs  # Should be 0
  ```

#### Module 2: dns/resolver.rs (21 unwraps)
- [ ] **11:15-12:30** Fix unwraps in dns/resolver.rs
  - Pattern: Lock unwraps → poisoned lock handling
  - Pattern: Parse unwraps → with_context
  - Pattern: Network unwraps → proper error propagation
- [ ] **12:30-12:45** Verify dns/resolver.rs
  ```bash
  cargo test --lib dns::resolver
  grep -c "unwrap()" src/dns/resolver.rs  # Should be 0
  ```

### Afternoon: Crypto Modules (71 unwraps)

#### Module 3: ct/fingerprint_tracker.rs (20 unwraps)
- [ ] **1:30-2:30** Fix unwraps
  - Focus: Async operations, storage access
- [ ] **2:30-2:45** Verify
  ```bash
  cargo test --lib ct::fingerprint_tracker
  ```

#### Module 4: security/alerts.rs (19 unwraps)
- [ ] **2:45-3:45** Fix unwraps
  - Focus: Alert creation, monitoring integration
- [ ] **3:45-4:00** Verify
  ```bash
  cargo test --lib security::alerts
  ```

#### Module 5: crypto/kyber.rs (18 unwraps)
- [ ] **4:00-5:00** Fix unwraps
  - Focus: Key generation, encryption/decryption
  - Use StoqError::CryptoError for domain errors
- [ ] **5:00-5:15** Verify
  ```bash
  cargo test --lib crypto::kyber
  ```

### End of Day 1
- [ ] **5:15-5:45** Run full verification
  ```bash
  ./scripts/verify_unwraps.sh
  cargo test --lib
  git add src/ct/storage.rs src/dns/resolver.rs src/ct/fingerprint_tracker.rs src/security/alerts.rs src/crypto/kyber.rs
  git commit -m "Day 1: Eliminate unwraps in P0 modules (104/162)"
  ```
- [ ] **Target**: ~100+ unwraps eliminated, 0 test regressions

---

## Day 2: Remaining Unwraps (209 unwraps)

### Morning: Complete P0 Modules

#### Module 6-9: crypto/ modules (51 unwraps)
- [ ] **9:00-10:30** crypto/certificate.rs (17 unwraps)
- [ ] **10:30-11:00** crypto/falcon.rs (16 unwraps)
- [ ] **11:00-11:30** crypto/hybrid.rs (15 unwraps)
- [ ] **11:30-12:00** crypto/mod.rs (12 unwraps)

- [ ] **12:00-12:15** Verify all crypto
  ```bash
  cargo test --lib crypto::
  grep -r "unwrap()" src/crypto/ | grep -v test | wc -l  # Should be 0
  ```

### Afternoon: Secondary Modules

#### Module 10-15: dns/ modules (35 unwraps)
- [ ] **1:00-2:00** dns/cache.rs (16 unwraps)
- [ ] **2:00-2:30** dns/cert_validator.rs (14 unwraps)
- [ ] **2:30-3:00** dns/mod.rs (5 unwraps)

#### Module 16-20: ct/ modules (23 unwraps)
- [ ] **3:00-4:00** ct/merkle_log.rs (14 unwraps)
- [ ] **4:00-4:30** ct/mod.rs (9 unwraps)

#### Module 21-25: Remaining modules (100 unwraps)
- [ ] **4:30-5:30** Batch process remaining low-density files
  ```bash
  for file in $(grep -r "unwrap()" src/ --include="*.rs" | grep -v test | cut -d: -f1 | sort -u); do
    echo "Fixing $file..."
    # Apply patterns from docs/error_handling_patterns.md
  done
  ```

### End of Day 2
- [ ] **5:30-6:00** Full verification
  ```bash
  ./scripts/verify_unwraps.sh
  cargo build --release  # Check release build too
  cargo test --lib
  grep -r "unwrap()" src/ --include="*.rs" | grep -v test | wc -l  # Should be 0
  git commit -m "Day 2: Complete unwrap elimination (371→0)"
  ```
- [ ] **Target**: 0 unwraps in production code, all tests passing

---

## Day 3: Test Fixes Part 1 (15 tests)

### Morning: Crypto Tests (5 tests)

#### Crypto API Updates
- [ ] **9:00-9:30** Update pqc_kyber usage
  ```bash
  # Check current API version
  cargo tree | grep pqc_kyber
  # Update tests to match
  ```
- [ ] **9:30-10:30** Fix crypto tests
  - [ ] test_kyber_encrypt_decrypt_roundtrip
  - [ ] test_large_data_encryption
  - [ ] test_hybrid_encryption_decryption
  - [ ] test_kyber_encrypt_decrypt

- [ ] **10:30-10:45** Verify crypto tests
  ```bash
  cargo test --lib crypto:: -- --show-output
  ```

### Late Morning: CT Tests (6 tests)

#### Certificate Transparency Fixes
- [ ] **10:45-11:00** Create test storage helper
  ```rust
  async fn setup_test_storage() -> CTStorage { ... }
  ```
- [ ] **11:00-12:00** Fix CT tests
  - [ ] test_certificate_logging
  - [ ] test_certificate_verification
  - [ ] test_get_entries_range
  - [ ] test_inclusion_proof
  - [ ] test_log_stats
  - [ ] test_domain_fingerprints

### Afternoon: Miscellaneous Tests (4 tests)

- [ ] **1:00-1:30** test_stats_update (DNS)
  - Fix: Use atomic operations for stats
- [ ] **1:30-2:00** test_timing_statistics (monitoring)
  - Fix: Float precision comparisons
- [ ] **2:00-2:30** test_remaining_tokens (rate limiter)
  - Fix: Float epsilon comparisons
- [ ] **2:30-3:00** test_sanitize_input (validation)
  - Fix: Edge case handling

### End of Day 3
- [ ] **3:00-3:30** Verification
  ```bash
  ./scripts/run_test_category.sh crypto
  ./scripts/run_test_category.sh ct
  cargo test --lib 2>&1 | grep "test result:"
  # Target: 15 tests fixed, 16 remaining
  git commit -m "Day 3: Fix crypto, CT, and misc tests (15/30)"
  ```

---

## Day 4: Test Fixes Part 2 (15 tests)

### Morning: CA Tests (6 tests)

#### Certificate Authority Setup
- [ ] **9:00-9:30** Create test CA helper
  ```rust
  async fn create_test_ca() -> CertificateAuthority { ... }
  async fn generate_test_root_ca(config: &CAConfig) -> Result<()> { ... }
  ```
- [ ] **9:30-12:00** Fix CA tests
  - [ ] test_ca_creation
  - [ ] test_certificate_issuance
  - [ ] test_certificate_validation
  - [ ] test_security_integrated_ca_creation
  - [ ] test_secure_certificate_issuance
  - [ ] test_mandatory_state proof_disabled

- [ ] **12:00-12:15** Verify CA tests
  ```bash
  cargo test --lib ca:: -- --show-output
  ```

### Afternoon: DNS & Integration Tests (9 tests)

#### DNS Tests (4 tests)
- [ ] **1:00-1:30** Create DNS test fixtures
  ```rust
  const TEST_TRUSTCHAIN_DOMAIN: &str = "test.trustchain.local";
  async fn setup_test_resolver() -> DnsResolver { ... }
  ```
- [ ] **1:30-3:00** Fix DNS tests
  - [ ] test_trustchain_domain_resolution
  - [ ] test_unknown_trustchain_domain
  - [ ] test_config_file_operations
  - [ ] test_certificate_request_deserialization

#### Integration Tests (5 tests)
- [ ] **3:00-3:30** Add dynamic port allocation
  ```rust
  fn get_free_port() -> u16 { ... }
  async fn wait_for_service(addr: SocketAddr) -> Result<()> { ... }
  ```
- [ ] **3:30-5:00** Fix integration tests
  - [ ] test_trustchain_security_initialization
  - [ ] test_security_dashboard
  - [ ] test_state proof_validation
  - [ ] test_client_metrics
  - [ ] test_time_proof_serialization

### End of Day 4
- [ ] **5:00-5:30** Full test verification
  ```bash
  cargo test --lib -- --test-threads=1 --show-output
  cargo test --lib 2>&1 | grep "test result:"
  # Target: 0 test failures
  git commit -m "Day 4: Complete test fixes (30/30 fixed)"
  ```

---

## Day 5: Critical TODO Implementation (8 TODOs)

### Morning: DNS & API (4 TODOs)

- [ ] **9:00-11:00** DNS STOQ Listener (Priority 1)
  - Implement `start_stoq_listener()`
  - Add test: `test_stoq_listener`
  - Verify: DNS-over-STOQ works

- [ ] **11:00-12:30** PEM Parsing (Priority 2)
  - Add dependencies: x509-parser, pem
  - Implement `parse_certificate_pem()`
  - Test: `test_certificate_parsing`

### Afternoon: Certificate Handling (4 TODOs)

- [ ] **1:30-3:00** CSR Subject Extraction (Priority 3)
  - Implement `parse_csr()`
  - Extract CN and SANs
  - Test: `test_csr_parsing`

- [ ] **3:00-5:00** CA Signing (Priority 4)
  - Implement `sign_with_ca()`
  - Update `issue_certificate()`
  - Test: `test_ca_signed_certificate`

### Evening: Cleanup (3 TODOs)

- [ ] **5:00-5:30** DNS Record Parsing (Priority 5)
  - Implement `parse_dns_query()`
  - Implement `serialize_dns_response()`

- [ ] **5:30-6:00** SAN Extraction (Priority 6)
  - Implement `extract_san_entries()`

- [ ] **6:00-6:15** Client Address (Priority 7)
  - Update STOQ handler to extract peer addr

### End of Day 5
- [ ] **6:15-6:30** Verification
  ```bash
  ./scripts/verify_todos.sh
  cargo test --lib
  # Target: 8 critical TODOs resolved
  git commit -m "Day 5: Implement 8 critical TODOs"
  ```

---

## Day 6: TODO Documentation & Cleanup (18 TODOs)

### Morning: Documentation

- [ ] **9:00-10:00** Chain Building (Priority 8)
  - Implement `build_certificate_chain()`
  - Test full chain verification

- [ ] **10:00-11:00** Create Migration Guides
  - [ ] `docs/state proof_MIGRATION.md`
  - Document deprecation of dummy proof generation
  - Explain migration to network-based generation

- [ ] **11:00-12:00** Create GitHub Issues
  - [ ] Issue: Merkle tree algorithm upgrade
  - [ ] Issue: S3 storage integration
  - [ ] Issue: CA metrics collection
  - Link all deferred TODOs to issues

### Afternoon: Code Cleanup

- [ ] **1:00-2:00** Update Deferred TODOs
  - Replace generic TODOs with issue references
  - Add deprecation notices where needed
  - Ensure all TODOs have context

- [ ] **2:00-3:00** Documentation Pass
  - Update inline docs for new implementations
  - Add examples for complex functions
  - Document workarounds

- [ ] **3:00-4:00** Quality Checks
  ```bash
  cargo doc --no-deps
  cargo clippy -- -D warnings
  cargo fmt -- --check
  ```

### End of Day 6
- [ ] **4:00-4:30** Final TODO verification
  ```bash
  ./scripts/verify_todos.sh
  # All TODOs should be:
  # - Implemented, OR
  # - Documented with issue reference, OR
  # - Marked as intentional (quality gates)
  git commit -m "Day 6: Document and track remaining TODOs"
  ```

---

## Day 7: Final Quality Pass

### Morning: Automated Verification

- [ ] **9:00-9:30** Run all verification scripts
  ```bash
  ./scripts/verify_unwraps.sh
  ./scripts/verify_todos.sh
  ./scripts/final_verify.sh
  ```

- [ ] **9:30-10:30** Fix any remaining issues
  - Address script findings
  - Fix last-minute warnings
  - Clean up any missed items

- [ ] **10:30-11:30** Performance Check
  ```bash
  cargo build --release
  cargo bench  # If benchmarks exist
  # Check for performance regressions
  ```

### Late Morning: Code Quality

- [ ] **11:30-12:00** Clippy deep dive
  ```bash
  cargo clippy --all-targets --all-features -- -D warnings
  ```

- [ ] **12:00-12:30** Format and documentation
  ```bash
  cargo fmt
  cargo doc --no-deps --document-private-items
  ```

### Afternoon: Integration Testing

- [ ] **1:00-2:00** Full test suite (multiple runs)
  ```bash
  for i in {1..10}; do
    echo "Run $i"
    cargo test --lib -- --test-threads=1
    if [ $? -ne 0 ]; then
      echo "Flaky test detected in run $i"
      break
    fi
  done
  ```

- [ ] **2:00-3:00** Security audit
  ```bash
  cargo audit
  cargo outdated --depth 1
  # Review and address vulnerabilities
  ```

- [ ] **3:00-4:00** Manual code review
  - Review all changes made during sprint
  - Check for missed patterns
  - Verify error messages are helpful

### End of Day 7: Sprint Completion

- [ ] **4:00-4:30** Generate final report
  ```bash
  ./scripts/final_verify.sh > SPRINT_2_2_FINAL_REPORT.txt
  ```

- [ ] **4:30-5:00** Prepare sprint summary
  - [ ] Document accomplishments
  - [ ] List any deferred items
  - [ ] Note lessons learned
  - [ ] Update sprint status in PDL

- [ ] **5:00-5:30** Create PR
  ```bash
  git checkout main
  git merge sprint-2.2-day-7
  git push origin main
  # Create PR with summary
  ```

### Success Criteria Checklist
- [ ] **Unwraps**: 371 → 0 in production code
- [ ] **Tests**: 30 failures → 0 failures
- [ ] **TODOs**: 8 critical → all implemented
- [ ] **TODOs**: 18 deferred → all documented
- [ ] **Warnings**: Build clean (0 warnings)
- [ ] **Clippy**: All recommendations addressed
- [ ] **Tests**: No flaky tests (10 consecutive passes)
- [ ] **Documentation**: All changes documented

---

## Daily Checkpoint Questions

### End of Each Day Ask:
1. Are all commits pushed?
2. Did tests pass before committing?
3. Are there any blockers for tomorrow?
4. Did we meet the day's target metrics?
5. Any unexpected issues to document?

### Red Flags (Stop and Reassess):
- More than 2 hours stuck on single unwrap pattern
- Tests failing after "simple" change
- Build time increased significantly
- New warnings appearing
- Performance regression detected

---

## Emergency Procedures

### If Build Breaks:
1. Git stash current changes
2. Verify main branch builds
3. Apply changes incrementally
4. Identify breaking change
5. Fix or revert

### If Tests Become Flaky:
1. Run test 10 times
2. Identify failure pattern
3. Add debug logging
4. Check for race conditions
5. Add proper synchronization

### If Behind Schedule:
1. Review time spent per module
2. Identify time sinks
3. Adjust remaining priorities
4. Document decisions
5. Communicate blockers

---

## Tools Quick Reference

```bash
# Check current status
./scripts/verify_unwraps.sh
./scripts/verify_todos.sh

# Run specific test category
./scripts/run_test_category.sh crypto

# Module-specific unwrap analysis
./scripts/fix_module_unwraps.sh ct/storage

# Final comprehensive check
./scripts/final_verify.sh

# Format and lint
cargo fmt
cargo clippy -- -D warnings

# Documentation
cargo doc --open
```