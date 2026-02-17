# Sprint 1.2 Step 3: Technical Design - STOQ Stabilization & Hardening

**Sprint Duration**: Days 1-5 (Critical Path)  
**Design Date**: December 9, 2025  
**Design Status**: Ready for Implementation (Step 4)

---

## Executive Summary

This technical design provides implementation-ready specifications for Days 1-5 of Sprint 1.2: fixing compilation errors, removing panic points, patching security vulnerabilities, and enhancing connection pooling. The design prioritizes backward compatibility, incremental changes, and zero regressions.

**Key Principles**:
1. **Incremental refactoring** - One file at a time, test after each change
2. **Backward compatibility** - Maintain existing API surface
3. **Zero regression tolerance** - All tests must pass after each change
4. **Security first** - Patch vulnerabilities before optimization

---

## DAY 1: Compilation Fix Strategy

### 1.1 Root Cause Analysis

**Compilation errors fall into 3 categories**:

#### Category A: Missing/Renamed Modules (16+ errors)
```rust
// ERRORS:
error[E0432]: unresolved import `stoq::performance_monitor`
error[E0432]: unresolved import `stoq::phoenix`
error[E0432]: unresolved import `stoq::monitoring`
error[E0432]: unresolved import `stoq::crypto`
error[E0432]: unresolved import `stoq::errors`
```

**Root Cause**: Tests reference modules that were removed or never existed in lib.rs public API

**Solution**: Add module re-exports in `/stoq/src/lib.rs`:
```rust
// Add to lib.rs after line 54
pub mod errors {
    //! Error types for STOQ transport
    pub use crate::api::ApiError;
    // Add StoqError enum (defined in Day 2)
}

// Re-export existing modules
pub use transport::metrics as performance_monitor;
pub use transport::adaptive as phoenix;

// Crypto is already available via transport::falcon
pub mod crypto {
    pub use crate::transport::falcon::*;
}

// Monitoring will use existing metrics
pub use transport::metrics as monitoring;
```

#### Category B: Struct Field Mismatches (7+ errors)
```rust
// ERRORS:
error[E0560]: struct `TransportConfig` has no field named `bind_addr`
error[E0560]: struct `TransportConfig` has no field named `network_tier`
error[E0560]: struct `TransportConfig` has no field named `adaptive_optimization`
```

**Root Cause**: Tests use old field names from previous TransportConfig API

**Solution**: Add deprecated field aliases with conversion in TransportConfig:
```rust
// In /stoq/src/transport/mod.rs at TransportConfig impl
impl TransportConfig {
    /// Legacy: Deprecated, use bind_address instead
    #[deprecated(since = "0.1.0", note = "use bind_address instead")]
    pub fn with_bind_addr(mut self, addr: Ipv6Addr) -> Self {
        self.bind_address = addr;
        self
    }
    
    /// Legacy: Deprecated, use adapt_to_network_tier instead
    #[deprecated(since = "0.1.0", note = "use adapt_to_network_tier instead")]
    pub fn with_network_tier(mut self, tier: NetworkTier) -> Self {
        self.adapt_to_network_tier(&tier);
        self
    }
}
```

#### Category C: Type Signature Changes (10+ errors)
```rust
// ERRORS:
error[E0599]: no function or associated item named `new_self_signed` found
error[E0599]: no method named `resolve_service` found
error[E0599]: no method named `is_healthy` found
```

**Root Cause**: Methods were renamed or moved to different types

**Solution**: Add compatibility shims in respective implementations

---

### 1.2 Implementation Plan (Day 1)

**File-by-file approach** (3 phases):

#### Phase 1: Fix lib.rs exports (1 hour)
```bash
# Test compilation after each module addition
cargo build --lib
```

**Changes to `/stoq/src/lib.rs`**:
```rust
// Line 18 - Add errors module (temporary, will be proper in Day 2)
pub mod errors {
    pub use crate::api::ApiError;
}

// Line 54 - Add module re-exports
pub use transport::metrics as performance_monitor;
pub use transport::adaptive as phoenix;
pub use transport::metrics as monitoring;

pub mod crypto {
    pub use crate::transport::falcon::*;
}
```

#### Phase 2: Fix TransportConfig compatibility (2 hours)
**File**: `/stoq/src/transport/mod.rs`

Add after line 219 (end of TransportConfig impl):
```rust
impl TransportConfig {
    // Backward compatibility for test suite
    
    #[deprecated(since = "0.1.0")]
    pub fn with_bind_addr(mut self, addr: Ipv6Addr) -> Self {
        self.bind_address = addr;
        self
    }
    
    #[deprecated(since = "0.1.0")]
    pub fn with_max_packet_size(mut self, size: usize) -> Self {
        self.max_datagram_size = size;
        self
    }
    
    #[deprecated(since = "0.1.0")]
    pub fn with_network_tier(mut self, _tier: NetworkTier) -> Self {
        // Tier is now auto-detected, this is a no-op for compatibility
        warn!("with_network_tier is deprecated, tier is auto-detected");
        self
    }
    
    #[deprecated(since = "0.1.0")]
    pub fn enable_network_isolation(mut self, _enable: bool) -> Self {
        // Network isolation is always available, this is a no-op
        warn!("enable_network_isolation is deprecated, always available");
        self
    }
}
```

#### Phase 3: Fix method compatibility (3 hours)

**File**: `/stoq/src/api/service_discovery.rs`
```rust
// Add after line 158
impl ServiceDiscovery {
    /// Backward compatibility: resolve_service -> resolve
    #[deprecated(since = "0.1.0", note = "use resolve instead")]
    pub fn resolve_service(&self, name: &str) -> Option<ServiceEndpoint> {
        self.resolve(name)
    }
}
```

**File**: `/stoq/src/transport/certificates.rs`
```rust
// Add after CertificateManager impl
impl CertificateManager {
    /// Backward compatibility for tests
    #[deprecated(since = "0.1.0", note = "use generate_self_signed instead")]
    pub fn new_self_signed() -> Result<Self> {
        Self::generate_self_signed()
    }
}
```

**File**: `/stoq/src/transport/mod.rs` (Connection impl)
```rust
// Add health check method for connection pool (Day 5 full impl)
impl Connection {
    /// Check if connection is still active (basic version)
    pub fn is_active(&self) -> bool {
        !self.connection.is_closed()
    }
    
    /// Check if connection is healthy (will enhance in Day 5)
    pub fn is_healthy(&self) -> bool {
        self.is_active()
    }
}
```

#### Phase 4: Fix NetworkTier variants (2 hours)

**Problem**: Tests use old NetworkTier variants (Auto, Lan, Wan, Metro, Satellite, Anonymous)

**File**: `/stoq/src/transport/mod.rs` (after NetworkTier enum)
```rust
impl NetworkTier {
    // Backward compatibility for tests
    
    #[deprecated(since = "0.1.0", note = "use from_gbps(1.0) instead")]
    pub fn auto() -> Self {
        NetworkTier::Standard { gbps: 1.0 }
    }
    
    #[deprecated(since = "0.1.0", note = "use from_gbps(10.0) instead")]
    pub fn lan() -> Self {
        NetworkTier::Enterprise { gbps: 10.0 }
    }
    
    #[deprecated(since = "0.1.0", note = "use from_gbps(0.1) instead")]
    pub fn wan() -> Self {
        NetworkTier::Home { mbps: 100.0 }
    }
    
    #[deprecated(since = "0.1.0", note = "use from_gbps(0.5) instead")]
    pub fn metro() -> Self {
        NetworkTier::Home { mbps: 500.0 }
    }
    
    #[deprecated(since = "0.1.0", note = "use from_gbps(0.001) instead")]
    pub fn satellite() -> Self {
        NetworkTier::Slow { mbps: 1.0 }
    }
    
    #[deprecated(since = "0.1.0", note = "network isolation is now in NetworkIsolationManager")]
    pub fn anonymous() -> Self {
        NetworkTier::Slow { mbps: 10.0 }
    }
}
```

### 1.3 Verification Strategy (Day 1)

**After EACH phase**:
```bash
# Incremental compilation check
cargo build --lib
cargo build --all-targets 2>&1 | tee compilation_errors.log

# Count remaining errors
grep "^error\[" compilation_errors.log | wc -l
```

**End of Day 1 Success Criteria**:
```bash
cargo build --all-targets  # Zero errors
cargo test --no-run       # Zero compilation errors
cargo bench --no-run      # Zero compilation errors
```

---

## DAY 2: Error Handling Refactor Part 1

### 2.1 Error Type Hierarchy

**Create new file**: `/stoq/src/errors.rs`

```rust
//! Comprehensive error types for STOQ transport

use std::io;
use thiserror::Error;

/// Primary error type for STOQ operations
#[derive(Debug, Error)]
pub enum StoqError {
    /// Transport layer errors (QUIC, socket, network)
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),
    
    /// Protocol layer errors (framing, encoding, handshake)
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    
    /// Connection errors (establishment, closure, timeout)
    #[error("Connection error: {0}")]
    Connection(#[from] ConnectionError),
    
    /// Cryptographic errors (FALCON, certificates, signing)
    #[error("Cryptographic error: {0}")]
    Crypto(#[from] CryptoError),
    
    /// I/O errors (file, socket, system)
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    
    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),
    
    /// Invalid state errors
    #[error("Invalid state: {0}")]
    InvalidState(String),
}

/// Transport layer specific errors
#[derive(Debug, Error)]
pub enum TransportError {
    #[error("Failed to bind to address: {0}")]
    BindFailed(String),
    
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Connection closed unexpectedly")]
    ConnectionClosed,
    
    #[error("Connection timeout after {0:?}")]
    Timeout(std::time::Duration),
    
    #[error("QUIC error: {0}")]
    QuicError(String),
    
    #[error("Socket error: {0}")]
    SocketError(String),
}

/// Protocol layer specific errors
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Invalid frame type: {0}")]
    InvalidFrame(u8),
    
    #[error("Frame encoding failed: {0}")]
    EncodingFailed(String),
    
    #[error("Frame decoding failed: {0}")]
    DecodingFailed(String),
    
    #[error("Handshake failed: {0}")]
    HandshakeFailed(String),
    
    #[error("Invalid protocol version: {0}")]
    InvalidVersion(String),
    
    #[error("Shard reassembly failed: {0}")]
    ShardReassemblyFailed(String),
    
    #[error("Token validation failed: {0}")]
    TokenValidationFailed(String),
}

/// Connection specific errors
#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("Connection pool exhausted")]
    PoolExhausted,
    
    #[error("Connection not found: {0}")]
    NotFound(String),
    
    #[error("Stream error: {0}")]
    StreamError(String),
    
    #[error("Connection health check failed: {0}")]
    HealthCheckFailed(String),
}

/// Cryptographic operation errors
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("FALCON key generation failed: {0}")]
    KeyGenerationFailed(String),
    
    #[error("FALCON signing failed: {0}")]
    SigningFailed(String),
    
    #[error("FALCON verification failed: {0}")]
    VerificationFailed(String),
    
    #[error("Certificate generation failed: {0}")]
    CertificateGenerationFailed(String),
    
    #[error("Certificate validation failed: {0}")]
    CertificateValidationFailed(String),
    
    #[error("Invalid key format: {0}")]
    InvalidKeyFormat(String),
}

// Conversion from quinn errors
impl From<quinn::ConnectionError> for StoqError {
    fn from(err: quinn::ConnectionError) -> Self {
        StoqError::Transport(TransportError::QuicError(err.to_string()))
    }
}

impl From<quinn::ConnectError> for StoqError {
    fn from(err: quinn::ConnectError) -> Self {
        StoqError::Transport(TransportError::ConnectionFailed(err.to_string()))
    }
}

// Conversion from bincode errors
impl From<bincode::Error> for StoqError {
    fn from(err: bincode::Error) -> Self {
        StoqError::Serialization(err.to_string())
    }
}

// Conversion from anyhow for migration period
impl From<anyhow::Error> for StoqError {
    fn from(err: anyhow::Error) -> Self {
        StoqError::InvalidState(err.to_string())
    }
}

/// Type alias for Results using StoqError
pub type Result<T> = std::result::Result<T, StoqError>;
```

**Add to `/stoq/src/lib.rs`** (replace temporary errors module from Day 1):
```rust
pub mod errors;
pub use errors::{StoqError, TransportError, ProtocolError, ConnectionError, CryptoError};
```

### 2.2 Refactoring Strategy

**Prioritize by risk** (highest impact first):

#### Priority 1: Core Protocol (14 unwraps) - 3 hours
**File**: `/stoq/src/protocol/mod.rs`

**Unwraps at lines**: 513, 517, 533, 536, 540, 566, 571, 574

**Strategy**:
1. Change function signatures to return `Result<T, StoqError>`
2. Replace unwrap() with `?` operator
3. Add error context with `.map_err()`

**Example transformation**:
```rust
// BEFORE (line 513):
let encoded = handler.encode_token_frame(&token).unwrap();

// AFTER:
let encoded = handler.encode_token_frame(&token)
    .map_err(|e| ProtocolError::EncodingFailed(format!("token frame: {}", e)))?;
```

**Full refactor for encode_token_frame**:
```rust
// Change signature:
pub fn encode_token_frame(&self, token: &PacketToken) -> Result<Bytes, StoqError> {
    let frame = StoqFrame::Token {
        token_id: token.token_id.clone(),
        data: token.clone(),
    };
    
    frame.encode()
        .map_err(|e| ProtocolError::EncodingFailed(format!("token frame: {}", e)).into())
}
```

#### Priority 2: Network Isolation (9 unwraps) - 2 hours
**File**: `/stoq/src/network_isolation.rs`

**Strategy**: Same as Priority 1

#### Priority 3: Service Discovery (8 unwraps) - 1 hour
**File**: `/stoq/src/api/service_discovery.rs`

**Lines**: 232, 348, 351, 354, 357, 366, 372, 404

**Note**: These are ALL in test code (under #[cfg(test)])
**Action**: Convert test unwraps to expect() with descriptive messages:

```rust
// BEFORE:
let trustchain = discovery.resolve("trustchain").unwrap();

// AFTER:
let trustchain = discovery.resolve("trustchain")
    .expect("trustchain service should be registered in test");
```

### 2.3 Testing Strategy (Day 2)

**After EACH file refactor**:
```bash
# Compile check
cargo build --lib

# Run affected tests
cargo test --lib protocol::  # After protocol/mod.rs
cargo test --lib network_isolation::  # After network_isolation.rs

# Full test suite
cargo test --lib
```

**Track progress**:
```bash
# Count remaining unwraps in production code (exclude tests)
grep -r "unwrap()\|expect(" src/ | grep -v "#\[cfg(test)\]" -A 5 -B 5 | wc -l
```

**End of Day 2 Success Criteria**:
- 30+ unwraps removed from production code
- All existing tests still pass
- Zero compilation errors

---

## DAY 3: Error Handling Refactor Part 2

### 3.1 Remaining Files (3 hours)

#### File 1: `/stoq/src/extensions.rs` (4 unwraps)
Lines: 426, 429, 445, 455

```rust
// BEFORE (line 426):
let shards = extensions.shard_packet(data, max_shard_size).unwrap();

// AFTER:
let shards = extensions.shard_packet(data, max_shard_size)
    .map_err(|e| ProtocolError::EncodingFailed(format!("sharding: {}", e)))?;
```

**Change shard_packet signature**:
```rust
pub fn shard_packet(&self, data: Bytes, max_shard_size: usize) 
    -> Result<Vec<PacketShard>, StoqError>
```

#### File 2: `/stoq/src/protocol/pos_validator.rs` (4 unwraps)
Lines: 441, 452, 463, 471

**Note**: All in test code
**Action**: Convert to expect() with descriptive messages

#### File 3: `/stoq/src/protocol/frames.rs` (4 unwraps)
Lines: 582, 583, 609, 610

```rust
// Change encode/decode signatures:
impl StoqFrame {
    pub fn encode(&self) -> Result<Bytes, StoqError> {
        bincode::serialize(self)
            .map(|v| Bytes::from(v))
            .map_err(|e| ProtocolError::EncodingFailed(e.to_string()).into())
    }
    
    pub fn decode(data: Bytes) -> Result<Self, StoqError> {
        bincode::deserialize(&data)
            .map_err(|e| ProtocolError::DecodingFailed(e.to_string()).into())
    }
}
```

#### File 4: `/stoq/src/protocol/parameters.rs` (1 unwrap)
Line: 326

#### File 5: `/stoq/src/protocol/handshake.rs` (1 unwrap)
Line: 469

#### File 6: `/stoq/src/transport/falcon.rs` (1 unwrap)

#### File 7: `/stoq/src/transport/mod.rs` (1 unwrap)

#### File 8: `/stoq/src/transport/ebpf/metrics.rs` (2 unwraps)

#### File 9: `/stoq/src/transport/ebpf/loader.rs` (4 unwraps)

### 3.2 Test Suite Updates (4 hours)

**Update test assertions** for new error types:

```rust
// BEFORE:
assert!(result.is_ok());

// AFTER:
assert!(result.is_ok(), "expected success, got error: {:?}", result.err());

// BEFORE:
let value = function().unwrap();

// AFTER:
let value = function()
    .expect("function should succeed in test");
```

**Add error case tests**:
```rust
#[test]
fn test_decode_invalid_frame() {
    let invalid_data = Bytes::from_static(b"invalid");
    let result = StoqFrame::decode(invalid_data);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        StoqError::Protocol(ProtocolError::DecodingFailed(_)) => {},
        e => panic!("expected DecodingFailed, got {:?}", e),
    }
}
```

### 3.3 Verification (Day 3)

**Full test suite**:
```bash
cargo test --all-targets
cargo test --doc
```

**Unwrap audit**:
```bash
# Should be ZERO in production code
rg "\.unwrap\(\)" src/ --type rust | grep -v "test" | wc -l
rg "\.expect\(" src/ --type rust | grep -v "test" | wc -l
```

**End of Day 3 Success Criteria**:
- Zero unwrap() in production code (src/, excluding #[cfg(test)])
- Zero expect() in production code (src/, excluding #[cfg(test)])
- All tests pass
- No compilation warnings about unused Result types

---

## DAY 4: Security Patches

### 4.1 Vulnerability Assessment

**4 vulnerabilities identified**:

#### Vulnerability 1: RSA Marvin Attack (CRITICAL)
```
Crate:     rsa 0.9.8
ID:        RUSTSEC-2023-0071
Severity:  5.9 (medium)
Solution:  No fixed upgrade available
```

**Mitigation Strategy**:
1. **Remove direct rsa usage** - STOQ uses FALCON-1024 as primary crypto
2. **Audit usage**: `rg "use rsa" src/`
3. **Replace with FALCON** for all signing/verification
4. **Keep rsa for cert generation only** (non-critical path)

**Action**: Add warning in documentation, plan migration to pure FALCON

#### Vulnerability 2: pqcrypto-dilithium unmaintained
```
Crate:     pqcrypto-dilithium 0.5.0
ID:        RUSTSEC-2024-0380
Severity:  Warning (unmaintained)
Solution:  Migrate to pqcrypto-mldsa
```

**Migration Plan**:
1. **Check if used**: `rg "dilithium" src/` (likely not used, pulled by pqcrypto)
2. **Pin pqcrypto-falcon only**: Remove general pqcrypto dependency
3. **Direct dependency on pqcrypto-falcon**: Avoid unmaintained transitive deps

**Cargo.toml change**:
```toml
# BEFORE:
pqcrypto = { workspace = true }
pqcrypto-falcon = { workspace = true }

# AFTER:
# pqcrypto = { workspace = true }  # REMOVED - pulls unmaintained deps
pqcrypto-falcon = { workspace = true }
pqcrypto-traits = { workspace = true }
```

#### Vulnerability 3: pqcrypto-kyber unmaintained
```
Crate:     pqcrypto-kyber 0.8.1
ID:        RUSTSEC-2024-0381
Severity:  Warning (unmaintained)
Solution:  Migrate to pqcrypto-mlkem
```

**Same mitigation as Vulnerability 2**: Remove general pqcrypto dependency

#### Vulnerability 4: rustls-pemfile (potential)
```
Crate:     rustls-pemfile 2.2.0
ID:        RUSTSEC-2025-0134
Severity:  Unknown (check advisory)
```

**Check for upgrade**:
```bash
cargo update -p rustls-pemfile --dry-run
```

**If upgrade available**: Update to latest
**If breaking changes**: Create compatibility shim

### 4.2 Dependency Upgrade Plan (3 hours)

**Incremental upgrade strategy** (one at a time):

#### Step 1: Remove pqcrypto general dependency
```bash
# Edit Cargo.toml
# Remove: pqcrypto = { workspace = true }
# Keep:   pqcrypto-falcon = { workspace = true }

cargo build --lib
cargo test --lib
```

#### Step 2: Update rustls-pemfile
```bash
cargo update -p rustls-pemfile
cargo build --lib
cargo test transport::certificates  # Test cert parsing
```

#### Step 3: Audit rsa usage
```bash
rg "use rsa" src/
rg "use ring" src/  # Check if ring can replace rsa
```

**Create migration plan** for rsa removal (may defer to Phase 2)

### 4.3 Security Code Review (3 hours)

**Focus areas**:

#### Area 1: Certificate validation
**File**: `/stoq/src/transport/certificates.rs`

**Review checklist**:
- [ ] Certificate expiration checked
- [ ] Certificate chain validation
- [ ] Hostname verification (SNI)
- [ ] No self-signed in production mode
- [ ] Proper error handling for invalid certs

#### Area 2: FALCON crypto operations
**File**: `/stoq/src/transport/falcon.rs`

**Review checklist**:
- [ ] Key generation uses proper entropy
- [ ] Private keys never logged
- [ ] Signature verification before trust
- [ ] No hardcoded keys
- [ ] Side-channel resistance (constant-time ops)

#### Area 3: Connection handshake
**File**: `/stoq/src/protocol/handshake.rs`

**Review checklist**:
- [ ] Replay attack prevention
- [ ] Handshake timeout enforced
- [ ] Version downgrade protection
- [ ] No unauthenticated data accepted
- [ ] Proper state machine (no invalid transitions)

### 4.4 Security Testing (1 hour)

**Add security test suite**:

**File**: `/stoq/tests/security_hardening.rs`
```rust
#[tokio::test]
async fn test_reject_expired_certificate() {
    // Generate cert with past expiration
    // Attempt connection
    // Verify rejection
}

#[tokio::test]
async fn test_reject_invalid_falcon_signature() {
    // Create invalid signature
    // Attempt verification
    // Verify rejection
}

#[tokio::test]
async fn test_handshake_timeout() {
    // Initiate handshake
    // Don't respond
    // Verify timeout after max duration
}

#[tokio::test]
async fn test_connection_rate_limiting() {
    // Attempt rapid connections
    // Verify rate limiting kicks in
}
```

### 4.5 Verification (Day 4)

```bash
# Zero critical vulnerabilities
cargo audit

# All tests pass including security tests
cargo test

# No new warnings
cargo clippy -- -D warnings
```

**End of Day 4 Success Criteria**:
- `cargo audit` shows zero critical vulnerabilities
- Security test suite passes
- Certificate validation hardened
- FALCON crypto operations reviewed
- Migration plan for rsa documented

---

## DAY 5: Connection Pool Enhancement

### 5.1 Health Check Implementation (3 hours)

**Goal**: Verify connection is alive before reuse

**File**: `/stoq/src/transport/mod.rs`

#### Add health check logic to Connection:
```rust
impl Connection {
    /// Check if connection is active (basic liveness)
    pub fn is_active(&self) -> bool {
        !self.connection.is_closed()
    }
    
    /// Perform comprehensive health check
    pub async fn health_check(&self) -> Result<(), StoqError> {
        // Check 1: Connection not closed
        if self.connection.is_closed() {
            return Err(ConnectionError::HealthCheckFailed(
                "connection closed".to_string()
            ).into());
        }
        
        // Check 2: No recent errors
        let stats = self.connection.stats();
        if stats.path.lost_packets > 100 {
            return Err(ConnectionError::HealthCheckFailed(
                format!("high packet loss: {}", stats.path.lost_packets)
            ).into());
        }
        
        // Check 3: RTT not excessive (>1s indicates network issues)
        if stats.path.rtt > std::time::Duration::from_secs(1) {
            return Err(ConnectionError::HealthCheckFailed(
                format!("high RTT: {:?}", stats.path.rtt)
            ).into());
        }
        
        // Check 4: Can open new stream (actual liveness test)
        match tokio::time::timeout(
            std::time::Duration::from_millis(100),
            self.connection.open_bi()
        ).await {
            Ok(Ok(_stream)) => Ok(()),
            Ok(Err(e)) => Err(ConnectionError::HealthCheckFailed(
                format!("stream open failed: {}", e)
            ).into()),
            Err(_) => Err(ConnectionError::HealthCheckFailed(
                "stream open timeout".to_string()
            ).into()),
        }
    }
}
```

#### Update connect() to use health checks:
```rust
// In StoqTransport::connect() at line 714
if let Some(mut pool) = self.connection_pool.get_mut(&pool_key) {
    while let Some(pooled_conn) = pool.pop() {
        // BEFORE: if pooled_conn.is_active()
        // AFTER:
        if pooled_conn.is_active() {
            // Quick health check before reuse
            match pooled_conn.health_check().await {
                Ok(_) => {
                    debug!("Reusing healthy pooled connection");
                    self.metrics.record_pool_hit();
                    return Ok(pooled_conn);
                },
                Err(e) => {
                    debug!("Evicting unhealthy connection: {}", e);
                    self.metrics.record_pool_eviction();
                    // Continue to next connection in pool
                }
            }
        } else {
            self.metrics.record_pool_eviction();
        }
    }
}
```

### 5.2 Pool Metrics (2 hours)

**Add metrics to TransportMetrics**:

**File**: `/stoq/src/transport/metrics.rs`

```rust
// Add to TransportMetrics struct:
pub struct TransportMetrics {
    // ... existing fields ...
    
    // Connection pool metrics
    pool_hits: AtomicU64,
    pool_misses: AtomicU64,
    pool_evictions: AtomicU64,
    pool_size: AtomicUsize,
    connection_reuse_count: AtomicU64,
}

impl TransportMetrics {
    pub fn record_pool_hit(&self) {
        self.pool_hits.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_pool_miss(&self) {
        self.pool_misses.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_pool_eviction(&self) {
        self.pool_evictions.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn update_pool_size(&self, size: usize) {
        self.pool_size.store(size, Ordering::Relaxed);
    }
    
    pub fn record_connection_reuse(&self) {
        self.connection_reuse_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn pool_stats(&self) -> PoolStats {
        let hits = self.pool_hits.load(Ordering::Relaxed);
        let misses = self.pool_misses.load(Ordering::Relaxed);
        let total = hits + misses;
        
        PoolStats {
            hits,
            misses,
            evictions: self.pool_evictions.load(Ordering::Relaxed),
            size: self.pool_size.load(Ordering::Relaxed),
            reuse_count: self.connection_reuse_count.load(Ordering::Relaxed),
            hit_rate: if total > 0 { hits as f64 / total as f64 } else { 0.0 },
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub size: usize,
    pub reuse_count: u64,
    pub hit_rate: f64,
}
```

**Add to StoqTransport**:
```rust
impl StoqTransport {
    pub fn pool_stats(&self) -> PoolStats {
        self.metrics.pool_stats()
    }
}
```

### 5.3 LRU Eviction Policy (2 hours)

**Current**: Simple Vec (LIFO)  
**Target**: LRU with timestamp tracking

**Add to Connection struct**:
```rust
pub struct Connection {
    // ... existing fields ...
    last_used: Arc<AtomicU64>,  // Unix timestamp in millis
}

impl Connection {
    fn touch(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.last_used.store(now, Ordering::Relaxed);
    }
    
    pub fn last_used_timestamp(&self) -> u64 {
        self.last_used.load(Ordering::Relaxed)
    }
}
```

**Replace Vec with sorted structure**:

**Create new file**: `/stoq/src/transport/connection_pool.rs`
```rust
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use parking_lot::Mutex;
use crate::transport::Connection;

/// LRU connection pool with health checking
pub struct ConnectionPool {
    connections: Mutex<Vec<PooledConnection>>,
    max_size: usize,
    max_idle_time: std::time::Duration,
}

struct PooledConnection {
    connection: Arc<Connection>,
    last_used: u64,
}

impl ConnectionPool {
    pub fn new(max_size: usize, max_idle_time: std::time::Duration) -> Self {
        Self {
            connections: Mutex::new(Vec::with_capacity(max_size)),
            max_size,
            max_idle_time,
        }
    }
    
    pub fn get(&self) -> Option<Arc<Connection>> {
        let mut conns = self.connections.lock();
        
        // Evict expired connections
        let now = current_timestamp();
        let max_idle_ms = self.max_idle_time.as_millis() as u64;
        conns.retain(|pc| (now - pc.last_used) < max_idle_ms);
        
        // Find most recently used connection (last in vec after sort)
        if let Some(pooled) = conns.pop() {
            Some(pooled.connection)
        } else {
            None
        }
    }
    
    pub fn put(&self, connection: Arc<Connection>) {
        let mut conns = self.connections.lock();
        
        // Don't exceed max size
        if conns.len() >= self.max_size {
            // Remove least recently used (first in vec)
            conns.remove(0);
        }
        
        let pooled = PooledConnection {
            connection,
            last_used: current_timestamp(),
        };
        
        conns.push(pooled);
        
        // Sort by last_used (LRU at front, MRU at back)
        conns.sort_by_key(|pc| pc.last_used);
    }
    
    pub fn size(&self) -> usize {
        self.connections.lock().len()
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
```

**Update StoqTransport to use ConnectionPool**:
```rust
// Replace: connection_pool: Arc<DashMap<String, Vec<Arc<Connection>>>>
// With:    connection_pool: Arc<DashMap<String, ConnectionPool>>

impl StoqTransport {
    pub fn new(config: TransportConfig) -> Result<Self> {
        // ... existing code ...
        
        let connection_pool = Arc::new(DashMap::new());
        
        // ... rest of initialization ...
    }
    
    pub async fn connect(&self, endpoint: &Endpoint) -> Result<Arc<Connection>> {
        let pool_key = format!("{}:{}", endpoint.address, endpoint.port);
        
        // Get or create pool for this endpoint
        let pool = self.connection_pool
            .entry(pool_key.clone())
            .or_insert_with(|| {
                ConnectionPool::new(
                    self.config.connection_pool_size,
                    self.config.max_idle_timeout,
                )
            });
        
        // Try to get from pool
        if let Some(pooled_conn) = pool.get() {
            if pooled_conn.health_check().await.is_ok() {
                self.metrics.record_pool_hit();
                return Ok(pooled_conn);
            }
            self.metrics.record_pool_eviction();
        }
        
        self.metrics.record_pool_miss();
        
        // Create new connection (existing logic)
        // ... existing connection creation code ...
    }
    
    pub fn return_to_pool(&self, endpoint: &Endpoint, connection: Arc<Connection>) {
        let pool_key = format!("{}:{}", endpoint.address, endpoint.port);
        
        if let Some(pool) = self.connection_pool.get(&pool_key) {
            pool.put(connection);
            self.metrics.update_pool_size(pool.size());
        }
    }
}
```

### 5.4 Integration Tests (1 hour)

**File**: `/stoq/tests/connection_pool_test.rs`

```rust
#[tokio::test]
async fn test_connection_reuse() {
    let config = TransportConfig::default();
    let transport = StoqTransport::new(config).await.unwrap();
    
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9292);
    
    // Create first connection
    let conn1 = transport.connect(&endpoint).await.unwrap();
    let conn1_id = conn1.id();
    
    // Return to pool
    transport.return_to_pool(&endpoint, conn1);
    
    // Get connection again
    let conn2 = transport.connect(&endpoint).await.unwrap();
    
    // Should be same connection
    assert_eq!(conn1_id, conn2.id());
    
    // Verify metrics
    let stats = transport.pool_stats();
    assert_eq!(stats.hits, 1);
}

#[tokio::test]
async fn test_pool_eviction_unhealthy() {
    let config = TransportConfig::default();
    let transport = StoqTransport::new(config).await.unwrap();
    
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9292);
    
    let conn = transport.connect(&endpoint).await.unwrap();
    transport.return_to_pool(&endpoint, conn);
    
    // Simulate connection becoming unhealthy (close it)
    // (implementation depends on how to simulate this)
    
    // Try to get from pool
    let new_conn = transport.connect(&endpoint).await.unwrap();
    
    // Should have created new connection (old one evicted)
    let stats = transport.pool_stats();
    assert_eq!(stats.evictions, 1);
}

#[tokio::test]
async fn test_pool_lru_eviction() {
    let mut config = TransportConfig::default();
    config.connection_pool_size = 2;  // Small pool
    
    let transport = StoqTransport::new(config).await.unwrap();
    
    let ep1 = Endpoint::new(Ipv6Addr::LOCALHOST, 9292);
    let ep2 = Endpoint::new(Ipv6Addr::LOCALHOST, 9293);
    let ep3 = Endpoint::new(Ipv6Addr::LOCALHOST, 9294);
    
    // Fill pool
    let c1 = transport.connect(&ep1).await.unwrap();
    let c2 = transport.connect(&ep2).await.unwrap();
    
    transport.return_to_pool(&ep1, c1);
    transport.return_to_pool(&ep2, c2);
    
    // Add third connection (should evict LRU)
    let c3 = transport.connect(&ep3).await.unwrap();
    transport.return_to_pool(&ep3, c3);
    
    // Verify pool size limited
    assert_eq!(transport.pool_stats().size, 2);
}
```

### 5.5 Verification (Day 5)

```bash
# All tests pass
cargo test

# Pool tests specifically
cargo test connection_pool

# Verify metrics working
cargo test pool_stats
```

**End of Day 5 Success Criteria**:
- Connection health checks implemented
- Pool metrics tracked (hits, misses, evictions)
- LRU eviction policy working
- Integration tests pass
- Pool hit rate visible in metrics

---

## Risk Mitigation & Rollback Strategy

### Risk 1: Error handling breaks functionality
**Probability**: Medium  
**Impact**: High

**Mitigation**:
1. Incremental refactoring (one file at a time)
2. Test after each file
3. Keep old code in git history
4. Use feature flags for major changes

**Rollback**:
```bash
git revert <commit-hash>  # Revert specific file
cargo test --all-targets  # Verify rollback works
```

### Risk 2: Security patches introduce regressions
**Probability**: Low-Medium  
**Impact**: High

**Mitigation**:
1. Update one dependency at a time
2. Run full test suite after each update
3. Check for breaking changes in CHANGELOG
4. Have version pinning ready

**Rollback**:
```toml
# Cargo.toml - pin to old version
rustls-pemfile = "=2.2.0"  # Exact version
```

### Risk 3: Connection pool performance degradation
**Probability**: Low  
**Impact**: Medium

**Mitigation**:
1. Benchmark before/after
2. Keep simple Vec as fallback
3. Make LRU optional via config

**Rollback**:
```rust
// Config option to disable LRU
pub struct TransportConfig {
    pub enable_lru_pooling: bool,  // Default: true
}
```

### Risk 4: Time overrun
**Probability**: Medium  
**Impact**: Medium

**Mitigation**:
1. Track time spent per task
2. Cut scope if Day 1-3 take too long
3. Day 6-7 are optional (defer to Phase 2)

**Contingency**:
- Days 1-3: MUST complete (compilation + error handling)
- Day 4: SHOULD complete (security)
- Day 5: NICE TO HAVE (can ship without LRU)

---

## Validation & Testing Checkpoints

### After Day 1:
```bash
cargo build --all-targets
cargo test --no-run
cargo bench --no-run
# Expected: Zero compilation errors
```

### After Day 2:
```bash
cargo build --lib
cargo test --lib
rg "\.unwrap\(\)" src/ | grep -v test | wc -l  # Should be <23
# Expected: 30+ unwraps removed, tests pass
```

### After Day 3:
```bash
cargo test --all-targets
rg "\.unwrap\(\)|\.expect\(" src/ | grep -v test | wc -l  # Should be 0
# Expected: Zero unwraps in production, all tests pass
```

### After Day 4:
```bash
cargo audit
cargo test
cargo clippy -- -D warnings
# Expected: Zero critical vulnerabilities, tests pass, no warnings
```

### After Day 5:
```bash
cargo test connection_pool
cargo test pool_stats
# Expected: Pool tests pass, metrics working
```

---

## Success Criteria Summary

### DAY 1 (MUST HAVE):
- ✅ All code compiles (cargo build --all-targets)
- ✅ Tests compile (cargo test --no-run)
- ✅ Benchmarks compile (cargo bench --no-run)

### DAY 2 (MUST HAVE):
- ✅ StoqError enum defined and integrated
- ✅ 30+ unwraps removed from critical files
- ✅ All existing tests pass

### DAY 3 (MUST HAVE):
- ✅ Zero unwraps in production code
- ✅ Zero expect in production code
- ✅ All tests pass
- ✅ Error case tests added

### DAY 4 (MUST HAVE):
- ✅ cargo audit shows zero critical vulnerabilities
- ✅ pqcrypto general dependency removed
- ✅ rustls-pemfile updated
- ✅ Security test suite added
- ✅ Certificate/crypto code reviewed

### DAY 5 (SHOULD HAVE):
- ✅ Connection health checks implemented
- ✅ Pool metrics (hits/misses/evictions) tracked
- ✅ LRU eviction policy working
- ✅ Integration tests pass
- ✅ Pool stats accessible

---

## Implementation Handoff to @developer

This design is implementation-ready. Each section provides:
1. **Exact file locations** to edit
2. **Line numbers** for changes
3. **Code samples** showing before/after
4. **Testing strategy** for verification
5. **Rollback plans** for risk mitigation

**Next Steps**:
1. Review this design (Step 3 complete)
2. Implement Day 1 (Step 4: Development begins)
3. Follow incremental approach (test after each change)
4. Report blockers immediately if encountered

**Estimated Implementation Time**:
- Day 1: 8 hours (compilation fixes)
- Day 2: 8 hours (error handling part 1)
- Day 3: 8 hours (error handling part 2 + tests)
- Day 4: 8 hours (security patches)
- Day 5: 8 hours (connection pool)
- **TOTAL**: 40 hours (5 days @ 8 hours/day)

**Critical Path**: Days 1-3 MUST complete. Days 4-5 SHOULD complete but can compress if needed.

---

**Design Complete. Ready for Implementation.**
