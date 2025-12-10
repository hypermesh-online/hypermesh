# Test Fix Plan - Sprint 2.2

## Overview
**Total Test Failures**: 30 tests
**Test Suite**: 185 passed, 30 failed
**Success Rate**: 86% → Target: 100%

---

## Category A: Crypto Tests (5 failures) - Day 3 Morning (2 hours)

### Failed Tests
1. `crypto::kyber::tests::test_kyber_encrypt_decrypt_roundtrip`
2. `crypto::kyber::tests::test_large_data_encryption`
3. `crypto::hybrid::tests::test_hybrid_encryption_decryption`
4. `crypto::tests::test_kyber_encrypt_decrypt`
5. `crypto::falcon::tests::test_falcon_sign_verify` (if exists)

### Root Cause Analysis
**Primary Issue**: Kyber API changes in pqc_kyber crate
- Key size mismatches between test data and current API
- Outdated function signatures
- Changed constant names

### Fix Strategy
```rust
// OLD API (failing)
use pqc_kyber::*;
let (public_key, secret_key) = keypair(&mut rng).unwrap();

// NEW API (correct)
use pqc_kyber::{
    KYBER768_SECRETKEYBYTES,
    KYBER768_PUBLICKEYBYTES,
    KYBER768_CIPHERTEXTBYTES,
};
let keys = keypair(&mut rng)?;
```

### Verification
```bash
cargo test --lib crypto:: -- --nocapture
```

---

## Category B: Certificate Transparency (6 failures) - Day 3 Afternoon (3 hours)

### Failed Tests
1. `ct::tests::test_certificate_logging`
2. `ct::tests::test_certificate_verification`
3. `ct::tests::test_get_entries_range`
4. `ct::tests::test_inclusion_proof`
5. `ct::tests::test_log_stats`
6. `ct::fingerprint_tracker::tests::test_domain_fingerprints`

### Root Cause Analysis
**Primary Issues**:
1. **Async timing**: Tests not properly awaiting async operations
2. **Storage initialization**: Mock storage not properly set up
3. **Merkle tree state**: Tests expecting specific tree states

### Fix Strategy

#### Issue 1: Async Timing
```rust
// BEFORE
#[test]
fn test_certificate_logging() {
    let entry = create_test_entry();
    log.add_entry(entry); // Missing await
    assert_eq!(log.size(), 1);
}

// AFTER
#[tokio::test]
async fn test_certificate_logging() {
    let entry = create_test_entry();
    log.add_entry(entry).await?;
    // Add small delay for background tasks
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(log.size().await, 1);
}
```

#### Issue 2: Storage Mock Setup
```rust
// Add proper test storage initialization
async fn setup_test_storage() -> CTStorage {
    let temp_dir = TempDir::new().unwrap();
    CTStorage::new(temp_dir.path().to_str().unwrap())
        .await
        .expect("Failed to create test storage")
}
```

### Verification
```bash
cargo test --lib ct:: -- --show-output
```

---

## Category C: Certificate Authority (6 failures) - Day 4 Morning (3 hours)

### Failed Tests
1. `ca::tests::test_certificate_validation`
2. `ca::tests::test_ca_creation`
3. `ca::tests::test_certificate_issuance`
4. `ca::security_integration::tests::test_security_integrated_ca_creation`
5. `ca::security_integration::tests::test_mandatory_consensus_disabled`
6. `ca::security_integration::tests::test_secure_certificate_issuance`

### Root Cause Analysis
**Primary Issues**:
1. **Certificate chain setup**: Test CA not properly initialized
2. **Consensus mocking**: Security integration requires consensus mocks
3. **Key material**: Test certificates using wrong key types

### Fix Strategy

#### Issue 1: Test CA Setup
```rust
async fn create_test_ca() -> CertificateAuthority {
    let config = CAConfig {
        root_cert_path: "test_data/root.pem".into(),
        root_key_path: "test_data/root.key".into(),
        cert_validity_days: 365,
        ..Default::default()
    };

    // Generate test root if doesn't exist
    if !Path::new(&config.root_cert_path).exists() {
        generate_test_root_ca(&config).await?;
    }

    CertificateAuthority::new(config).await?
}
```

#### Issue 2: Consensus Mocking
```rust
// Create mock consensus validator for tests
struct MockConsensusValidator;

impl ConsensusValidator for MockConsensusValidator {
    async fn validate_proof(&self, _proof: &ConsensusProof) -> Result<bool> {
        Ok(true) // Always pass in tests
    }
}
```

### Verification
```bash
cargo test --lib ca:: -- --show-output
```

---

## Category D: DNS Tests (4 failures) - Day 4 Afternoon (2 hours)

### Failed Tests
1. `dns::tests::test_trustchain_domain_resolution`
2. `dns::tests::test_unknown_trustchain_domain`
3. `dns::resolver::tests::test_stats_update`
4. `dns::cache::tests::test_cache_expiration` (potential)

### Root Cause Analysis
**Primary Issues**:
1. **Hardcoded addresses**: Tests expect specific IP addresses
2. **Cache timing**: Race conditions in cache expiration tests
3. **Stats synchronization**: Metrics updates not atomic

### Fix Strategy

#### Issue 1: DNS Test Fixtures
```rust
// Use test fixtures instead of hardcoded values
const TEST_TRUSTCHAIN_DOMAIN: &str = "test.trustchain.local";
const TEST_IPV6: Ipv6Addr = Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1);

async fn setup_test_resolver() -> DnsResolver {
    let mut resolver = DnsResolver::new(Default::default()).await?;
    // Pre-populate with test data
    resolver.add_static_entry(TEST_TRUSTCHAIN_DOMAIN, TEST_IPV6).await;
    resolver
}
```

#### Issue 2: Stats Synchronization
```rust
// Use Arc<AtomicU64> for stats instead of plain counters
pub struct ResolverStats {
    queries: Arc<AtomicU64>,
    cache_hits: Arc<AtomicU64>,
}

impl ResolverStats {
    pub fn increment_queries(&self) {
        self.queries.fetch_add(1, Ordering::Relaxed);
    }
}
```

### Verification
```bash
cargo test --lib dns:: -- --show-output
```

---

## Category E: Integration Tests (5 failures) - Day 4 Late Afternoon (2 hours)

### Failed Tests
1. `tests::test_trustchain_security_initialization`
2. `tests::test_security_dashboard`
3. `tests::test_secure_certificate_issuance`
4. `tests::test_consensus_validation`

### Root Cause Analysis
**Primary Issues**:
1. **Port conflicts**: Multiple tests binding to same ports
2. **Service dependencies**: Tests require multiple services running
3. **Timing**: Services not fully initialized before tests run

### Fix Strategy

#### Issue 1: Dynamic Port Allocation
```rust
use std::net::TcpListener;

fn get_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[tokio::test]
async fn test_security_dashboard() {
    let port = get_free_port();
    let config = SecurityConfig {
        dashboard_port: port,
        ..Default::default()
    };
    // Rest of test
}
```

#### Issue 2: Service Readiness
```rust
async fn wait_for_service(addr: SocketAddr, timeout: Duration) -> Result<()> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if TcpStream::connect(addr).await.is_ok() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(anyhow!("Service not ready after {:?}", timeout))
}

#[tokio::test]
async fn test_secure_certificate_issuance() {
    let service = spawn_ca_service(config).await;
    wait_for_service(service.addr(), Duration::from_secs(5)).await?;
    // Now run test
}
```

### Verification
```bash
cargo test --lib tests:: -- --test-threads=1 --show-output
```

---

## Category F: Miscellaneous (4 failures) - Throughout Days 3-4

### Failed Tests
1. `api::rate_limiter::tests::test_remaining_tokens`
2. `api::tests::test_certificate_request_deserialization`
3. `config::tests::test_config_file_operations`
4. `monitoring::metrics::tests::test_timing_statistics`
5. `validation::tests::test_sanitize_input`
6. `consensus::hypermesh_client::tests::test_client_metrics`
7. `consensus::proof::tests::test_time_proof_serialization`

### Root Cause Analysis
**Mixed Issues**:
- Rate limiter: Float precision in token calculations
- Deserialization: JSON schema changes
- Config: File I/O in tests (cleanup issues)
- Metrics: Timing precision issues
- Validation: Edge case handling
- Consensus: Serialization format changes

### Fix Strategy

#### Rate Limiter Float Precision
```rust
// BEFORE
assert_eq!(limiter.remaining_tokens(), 5.0);

// AFTER
assert!((limiter.remaining_tokens() - 5.0).abs() < 0.001);
```

#### Deserialization Tests
```rust
// Update test JSON to match current schema
let json = r#"{
    "common_name": "test.trustchain.local",
    "subject_alt_names": ["alt.trustchain.local"],
    "key_type": "FALCON1024"
}"#;
```

---

## Test Execution Plan

### Day 3 Schedule
| Time | Category | Tests | Focus |
|------|----------|-------|-------|
| 9-11 AM | Crypto | 5 | API updates, key sizes |
| 11-1 PM | CT Tests | 6 | Async timing, storage |
| 2-4 PM | Buffer | - | Catch-up, verification |

### Day 4 Schedule
| Time | Category | Tests | Focus |
|------|----------|-------|-------|
| 9-12 PM | CA Tests | 6 | Certificate chains, mocks |
| 1-3 PM | DNS Tests | 4 | Fixtures, synchronization |
| 3-5 PM | Integration | 5 | Port allocation, timing |
| 5-6 PM | Misc | 4 | Quick fixes |

---

## Verification Checklist

### After Each Category Fix
```bash
# Run just that category
cargo test --lib <category>:: -- --show-output

# Check overall progress
cargo test --lib 2>&1 | grep "test result:"

# Look for new failures
cargo test --lib 2>&1 | grep FAILED
```

### End of Day 3
- [ ] All crypto tests passing
- [ ] All CT tests passing
- [ ] Build still successful
- [ ] No new test failures introduced

### End of Day 4
- [ ] All CA tests passing
- [ ] All DNS tests passing
- [ ] All integration tests passing
- [ ] All misc tests passing
- [ ] Full test suite: 0 failures

---

## Common Test Patterns

### Pattern: Async Test Setup
```rust
#[tokio::test]
async fn test_name() {
    // Setup
    let resource = setup_test_resource().await;

    // Execute
    let result = resource.operation().await;

    // Verify
    assert!(result.is_ok());

    // Cleanup
    resource.cleanup().await;
}
```

### Pattern: Timeout for Flaky Tests
```rust
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_name() {
    let result = timeout(
        Duration::from_secs(5),
        async_operation()
    ).await;

    assert!(result.is_ok(), "Test timed out");
}
```

### Pattern: Shared Test State
```rust
// Use once_cell for shared test resources
use once_cell::sync::Lazy;

static TEST_CA: Lazy<Arc<CertificateAuthority>> = Lazy::new(|| {
    Arc::new(setup_test_ca())
});
```

---

## Automation Scripts

### Run All Tests by Category
```bash
#!/bin/bash
for category in crypto ct ca dns integration; do
    echo "=== Testing $category ==="
    cargo test --lib ${category}:: -- --show-output
    if [ $? -ne 0 ]; then
        echo "❌ $category tests failed"
        exit 1
    fi
done
echo "✅ All categories pass"
```

### Track Progress
```bash
#!/bin/bash
# Track test fix progress
TOTAL=30
CURRENT=$(cargo test --lib 2>&1 | grep -oP '\d+(?= failed)' | tail -1)
FIXED=$((TOTAL - CURRENT))
echo "Progress: $FIXED/$TOTAL fixed ($(( FIXED * 100 / TOTAL ))%)"
```

---

## Success Criteria

- [ ] All 30 test failures resolved
- [ ] No new test failures introduced
- [ ] Test execution time < 45 seconds
- [ ] No flaky tests (pass 10 consecutive runs)
- [ ] All tests use proper async/await
- [ ] No hardcoded ports or addresses
- [ ] Proper test cleanup (no resource leaks)