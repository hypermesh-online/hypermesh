# Error Handling Standards for Web3 Ecosystem

## Overview

This document establishes unified error handling patterns across the Web3 ecosystem (TrustChain, STOQ, BlockMatrix, Catalog, Caesar, NGauge). Currently, 4,546 `unwrap()` calls exist in production code, creating systemic panics. This guide ensures consistent, robust error handling across all components.

**Critical Metric**: The web3 ecosystem currently has **2,642 production unwraps** (vs 1,904 in tests/benches). Elimination is in progress via Sprint 2.2 with target of zero panics in production.

---

## Table of Contents

1. [Core Principles](#core-principles)
2. [Error Type Hierarchy](#error-type-hierarchy)
3. [Patterns & Anti-Patterns](#patterns--anti-patterns)
4. [Component-Specific Error Types](#component-specific-error-types)
5. [Async Error Handling](#async-error-handling)
6. [Logging vs Returning Errors](#logging-vs-returning-errors)
7. [Error Context & Recovery](#error-context--recovery)
8. [Pre-commit Hook](#pre-commit-hook)
9. [Migration Guide](#migration-guide)

---

## Core Principles

### 1. Fail Gracefully, Not Catastrophically

**Rule**: Never use `.unwrap()` in production code. Panics crash entire nodes.

```rust
// BAD - crashes node
let value = some_option.unwrap();

// GOOD - explicit error handling
let value = some_option.ok_or(MyError::NotFound)?;
```

### 2. Result vs Option

**Result<T, E>**: Use when operation can fail with a meaningful error
- Network operations
- File I/O
- Crypto operations
- Consensus validation
- Database queries

**Option<T>**: Use for optional values, not errors
- `find()`, `first()`, `last()`
- Conversions that are infallible (use `Into<T>` instead)
- Lookups in collections where missing is expected

```rust
// Result: Operation that can fail
pub fn load_certificate(path: &str) -> Result<Certificate, TrustChainError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| TrustChainError::Storage(StorageError::FileSystem {
            path: path.to_string(),
            reason: e.to_string(),
        }))?;
    serde_json::from_str(&content)
        .map_err(|e| TrustChainError::Serialization(e.to_string()))
}

// Option: Value lookup where absence is expected
pub fn find_certificate_by_id(id: &str) -> Option<Certificate> {
    self.cache.iter().find(|cert| cert.id == id)
}
```

### 3. Error Propagation with `?`

Use the `?` operator to propagate errors automatically. It calls `From::from()` for type conversions.

```rust
// Propagate error up the call stack
pub fn validate_and_store(cert: Certificate) -> Result<(), TrustChainError> {
    let validated = validate(&cert)?;  // If validation fails, returns error
    store(&validated)?;                 // If storage fails, returns error
    Ok(())
}
```

### 4. Error Context with `.map_err()`

Add context when converting between error types or when details would be lost.

```rust
// Add context about what operation was happening
File::open("config.toml")
    .map_err(|e| TrustChainError::Configuration(ConfigError::FileNotFound {
        path: "config.toml".to_string(),
    }))?;

// For STOQ protocol errors
connection.send(&data)
    .map_err(|e| StoqError::Transport(TransportError::ConnectionFailed(
        format!("Failed to send {} bytes: {}", data.len(), e)
    )))?;
```

### 5. Multiple Error Sources

When a function can fail with multiple error types, use explicit error conversion:

```rust
// Pattern 1: Dedicated error enum with From<T> impls (preferred)
pub fn process_transaction(tx: Transaction) -> Result<Receipt, ProcessError> {
    validate_transaction(&tx)?;        // Converts ValidationError → ProcessError
    submit_to_chain(&tx)?;             // Converts ChainError → ProcessError
    Ok(Receipt::from(tx))
}

// Pattern 2: Use ? with explicit type conversions
pub fn complex_operation() -> Result<Output, MultiError> {
    let data = load_file()?;           // File error → MultiError via From impl
    let parsed = parse_json(&data)
        .map_err(|e| MultiError::ParseFailed(e))?;
    let validated = validate(&parsed)
        .map_err(|e| MultiError::ValidationFailed(e))?;
    Ok(Output::from(validated))
}
```

### 6. Unwrap Only in Tests & Examples

**Allowed locations** for `.unwrap()`:
- Test code (where panics are expected)
- Benchmark code
- Example/documentation code
- Unreachable assertions with `unreachable!()`

**Never use** in:
- Library code (production)
- Async functions that can be aborted
- Long-running services
- Any code in critical path

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_certificate_creation() {
        let cert = create_test_certificate().unwrap();  // OK in tests
        assert!(cert.is_valid());
    }
}
```

---

## Error Type Hierarchy

### Design Pattern: Layered Errors

Each component has specific error types that fold into a parent type:

```
TrustChainError (main)
├─ CAError (certificate authority)
├─ CTError (certificate transparency)
├─ DnsError (DNS resolver)
├─ ApiError (API server)
├─ ConsensusError (consensus validation)
├─ ConfigError (configuration)
├─ NetworkError (networking)
├─ StorageError (storage operations)
└─ CryptoError (cryptography)

StoqError (main)
├─ TransportError
├─ ProtocolError
├─ NetworkError
├─ SecurityError
└─ ApiError

BlockMatrixError (main)
├─ StateError (state management)
├─ SharedError (shared utilities)
├─ TransportError (transport)
├─ RuntimeError (execution)
└─ SchedulerError (scheduling)
```

### Using thiserror for Ergonomics

Use the `thiserror` crate for automatic `Display` and `Error` trait implementations:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TrustChainError {
    #[error("Certificate generation failed: {reason}")]
    CertificateGeneration { reason: String },

    #[error("Certificate not found: {identifier}")]
    CertificateNotFound { identifier: String },

    #[error("Cryptographic error: {0}")]
    Cryptographic(#[from] CryptoError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// Result type alias for convenience
pub type Result<T> = std::result::Result<T, TrustChainError>;
```

### Flatten vs Nest Errors

**Nest when**: Error type belongs to a sub-component
```rust
#[error("Certificate Authority error: {0}")]
CertificateAuthority(#[from] CAError),
```

**Flatten when**: Error is terminal and specific
```rust
#[error("I/O error: {0}")]
Io(#[from] std::io::Error),
```

---

## Patterns & Anti-Patterns

### Pattern 1: Simple Error Propagation

**Use when**: Error occurs at leaf of call stack, caller handles it.

```rust
pub fn load_config(path: &str) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}
```

### Pattern 2: Error Context with Details

**Use when**: Original error loses important context.

```rust
pub fn validate_signature(data: &[u8], sig: &[u8]) -> Result<bool, CryptoError> {
    crypto::verify(data, sig)
        .map_err(|e| CryptoError::SignatureVerification {
            reason: format!("FALCON-1024 verification failed: {}", e),
            data_length: data.len(),
        })
}
```

### Pattern 3: Multiple Error Sources

**Use when**: Operation involves multiple fallible steps with different error types.

```rust
pub fn process_and_validate(raw: Vec<u8>) -> Result<Certificate, ProcessError> {
    // Compression step
    let decompressed = decompress(&raw)
        .map_err(|e| ProcessError::Decompression(e))?;

    // Decryption step
    let decrypted = decrypt(&decompressed)
        .map_err(|e| ProcessError::Encryption(e))?;

    // Parsing step
    let certificate = parse_certificate(&decrypted)
        .map_err(|e| ProcessError::Parsing(e))?;

    Ok(certificate)
}
```

### Pattern 4: Optional Fallback

**Use when**: Error can be handled locally with a default.

```rust
pub fn get_config_or_default(path: Option<&str>) -> Config {
    path.and_then(|p| load_config(p).ok())
        .unwrap_or_else(|| Config::default())
}
```

### Pattern 5: Error Recovery

**Use when**: Error can be automatically retried or recovered.

```rust
pub async fn get_with_retry(url: &str, max_retries: u32) -> Result<Response, NetworkError> {
    let mut attempts = 0;
    loop {
        match fetch(url).await {
            Ok(response) => return Ok(response),
            Err(e) if e.is_retryable() && attempts < max_retries => {
                attempts += 1;
                tokio::time::sleep(Duration::from_millis(100 * attempts as u64)).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### Anti-Pattern 1: Silent Failures

```rust
// BAD - error is silently discarded
let _result = operation();  // Compiler warning: Result ignored

// GOOD - explicitly handled
let _ = operation().ok(); // Documented as intentional
// OR
operation().map_err(|e| log::warn!("Operation failed: {}", e)).ok();
```

### Anti-Pattern 2: Over-Nested Error Conversion

```rust
// BAD - unnecessary nesting
operation()
    .map_err(|e| OuterError::Inner(Box::new(InnerError::Conversion(e.to_string()))))?

// GOOD - direct From impl
#[derive(Error)]
pub enum OuterError {
    #[error("Inner operation failed: {0}")]
    Inner(#[from] InnerError),
}
```

### Anti-Pattern 3: Panic-Based Control Flow

```rust
// BAD - panic for control flow
let option = some_option.expect("This should never be None");

// GOOD - explicit error handling
let option = some_option.ok_or(MyError::RequiredValueMissing)?;
```

### Anti-Pattern 4: Generic Error Messages

```rust
// BAD - no context
.map_err(|_| MyError::Failed)?

// GOOD - descriptive
.map_err(|e| MyError::Failed {
    operation: "load_certificate",
    reason: e.to_string(),
    file_path: path.to_string(),
})?
```

---

## Component-Specific Error Types

### TrustChain Error Hierarchy

**Location**: `/home/persist/repos/projects/web3/trustchain/src/errors.rs`

**Main Categories**:
- `CAError`: Certificate authority operations (generation, validation, revocation)
- `CTError`: Certificate transparency (Merkle trees, consistency proofs)
- `DnsError`: DNS resolution and caching
- `ApiError`: API endpoint errors (auth, rate limiting, TLS)
- `ConsensusError`: Four-proof validation (PoSpace, PoStake, PoWork, PoTime)
- `ConfigError`: Configuration file and validation
- `NetworkError`: Network connectivity and protocol
- `StorageError`: Database and filesystem operations
- `CryptoError`: Cryptographic operations (FALCON-1024, Kyber)

**Usage Example**:
```rust
pub fn validate_certificate(cert: &Certificate) -> Result<(), TrustChainError> {
    if !cert.is_trusted() {
        return Err(TrustChainError::CertificateAuthority(
            CAError::CertificateValidation {
                reason: format!("Certificate {} not in trusted chain", cert.id)
            }
        ));
    }
    Ok(())
}
```

### STOQ Error Hierarchy

**Location**: `/home/persist/repos/projects/web3/stoq/src/errors.rs`

**Main Categories**:
- `TransportError`: QUIC connection, streams, binding
- `ProtocolError`: PoS validation, service discovery
- `NetworkError`: Network isolation, privacy tiers
- `SecurityError`: Certificates, signatures, crypto
- `ApiError`: Handler execution, serialization

**Usage Example**:
```rust
pub async fn send_with_validation(conn: &Connection, data: &[u8]) -> Result<(), StoqError> {
    conn.send(data)
        .await
        .map_err(|e| StoqError::Transport(TransportError::ConnectionFailed(
            format!("Send failed: {}", e)
        )))
}
```

### BlockMatrix Error Hierarchy

**Location**: `/home/persist/repos/projects/web3/blockmatrix/core/state/src/error.rs`

**Main Categories**:
- `StateError`: Consensus, storage, replication, transactions
- `NexusError`: Network, serialization, authentication, authorization

**Included Helper Methods**:
- `.is_retryable()`: Check if operation should be retried
- `.is_leadership_error()`: Check if error is related to leadership
- `.is_consensus_error()`: Check if error is consensus-related
- `.category()`: Get error category for metrics

**Usage Example**:
```rust
pub async fn replicate_state(state: &State) -> Result<(), StateError> {
    storage::write(&state)
        .await
        .map_err(|e| StateError::Storage {
            message: format!("Replication failed: {}", e)
        })?;
    Ok(())
}
```

---

## Async Error Handling

### Rule 1: Always Propagate Errors from Futures

```rust
// BAD - error silently dropped
tokio::spawn(async {
    operation().await.unwrap();  // Panics silently in background
});

// GOOD - error returned to caller
async fn operation_wrapper() -> Result<T, MyError> {
    operation().await  // Error propagated
}
```

### Rule 2: Use `.await` After Error-Returning Operations

```rust
// BAD - operation not awaited before conversion
let result = operation().map_err(|e| MyError::Wrapped(e));

// GOOD - await first, then convert
let result = operation()
    .await
    .map_err(|e| MyError::Wrapped(e))?;
```

### Rule 3: Handle Join Errors in Spawned Tasks

```rust
// Pattern: Capture error from spawned task
let handle = tokio::spawn(async {
    operation().await
});

match handle.await {
    Ok(Ok(result)) => { /* success */ }
    Ok(Err(e)) => { /* operation error */ }
    Err(e) => { /* join error (task panicked) */ }
}
```

### Rule 4: Timeout Handling

```rust
pub async fn operation_with_timeout(duration: Duration) -> Result<T, TimeoutError> {
    tokio::time::timeout(duration, operation())
        .await
        .map_err(|_| TimeoutError {
            operation: "operation_name",
            duration,
        })?
}
```

### Pattern: Result Stream Processing

```rust
pub async fn process_stream<S>(stream: S) -> Result<Vec<Output>, StreamError>
where
    S: futures::stream::Stream<Item = Result<Item, ItemError>>
{
    stream
        .map(|item_result| {
            item_result
                .and_then(|item| process_item(item))
                .map_err(|e| StreamError::ItemFailed {
                    reason: e.to_string(),
                })
        })
        .try_collect()
        .await
}
```

---

## Logging vs Returning Errors

### Rule 1: Return Errors from Functions

Log and return, don't just log:

```rust
// BAD - error lost
pub fn operation() {
    if let Err(e) = risky_operation() {
        log::error!("Operation failed: {}", e);
        // Error lost, caller doesn't know about failure
    }
}

// GOOD - error returned
pub fn operation() -> Result<T, MyError> {
    risky_operation()
        .map_err(|e| {
            log::error!("Operation failed: {}", e);
            MyError::OperationFailed(e.to_string())
        })
}
```

### Rule 2: Log at Appropriate Levels

```rust
// Context where error occurs: log::warn!()
pub fn validate() -> Result<(), ValidationError> {
    if invalid {
        log::warn!("Validation failed for: {:?}", self);
        return Err(ValidationError::Invalid);
    }
    Ok(())
}

// Unexpected error: log::error!()
pub async fn critical_operation() -> Result<(), CriticalError> {
    operation().await.map_err(|e| {
        log::error!("Critical operation failed: {}", e);
        CriticalError::OperationFailed
    })
}

// Handled error: log::debug!()
pub fn optional_step() -> Result<(), OptionalError> {
    risky_operation().map_err(|e| {
        log::debug!("Optional step failed (will retry): {}", e);
        OptionalError::Transient
    })
}
```

### Rule 3: Include Request IDs in Errors

```rust
pub async fn handle_request(req_id: &str, req: Request) -> Result<Response, ApiError> {
    process(&req)
        .await
        .map_err(|e| {
            log::error!("[{}] Request processing failed: {}", req_id, e);
            ApiError::ProcessingFailed {
                request_id: req_id.to_string(),
                reason: e.to_string(),
            }
        })
}
```

---

## Error Context & Recovery

### Pattern 1: Add Context Information

Use `map_err` to add context before propagating:

```rust
pub fn load_certificate(path: &str) -> Result<Certificate, TrustChainError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| TrustChainError::Storage(StorageError::FileSystem {
            path: path.to_string(),
            reason: format!("Failed to read certificate file: {}", e),
        }))?;

    serde_json::from_str(&content)
        .map_err(|e| TrustChainError::Serialization(format!(
            "Failed to parse certificate from {}: {}",
            path, e
        )))
}
```

### Pattern 2: Structured Error Responses

For API endpoints, provide structured error information:

```rust
#[derive(serde::Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    pub details: Option<serde_json::Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub request_id: Option<String>,
}

impl ErrorResponse {
    pub fn from_error(error: &TrustChainError, request_id: Option<String>) -> Self {
        Self {
            error: error.to_string(),
            code: Self::error_code(error),
            details: Self::error_details(error),
            timestamp: chrono::Utc::now(),
            request_id,
        }
    }

    fn error_code(error: &TrustChainError) -> String {
        match error {
            TrustChainError::CertificateNotFound { .. } => "CERT_NOT_FOUND".to_string(),
            TrustChainError::Timeout { .. } => "TIMEOUT".to_string(),
            _ => "UNKNOWN".to_string(),
        }
    }

    fn error_details(error: &TrustChainError) -> Option<serde_json::Value> {
        match error {
            TrustChainError::Timeout { operation, duration } => {
                Some(serde_json::json!({
                    "operation": operation,
                    "timeout_ms": duration.as_millis()
                }))
            }
            _ => None,
        }
    }
}
```

### Pattern 3: Error Classification for Metrics

```rust
impl NexusError {
    pub fn category(&self) -> &'static str {
        match self {
            NexusError::Network(_) => "network",
            NexusError::Authentication { .. } => "auth",
            NexusError::Timeout { .. } => "timeout",
            NexusError::Consensus { .. } => "consensus",
            _ => "other",
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self,
            NexusError::Network(_)
            | NexusError::Timeout { .. }
            | NexusError::Consensus { .. }
        )
    }
}

// Usage in metrics
match operation().await {
    Ok(result) => metrics::counter("operation.success").increment(1),
    Err(e) => {
        metrics::counter(&format!("operation.error.{}", e.category())).increment(1);
        if e.is_retryable() {
            retry_with_backoff().await;
        }
    }
}
```

---

## Pre-commit Hook

Create `/home/persist/repos/projects/web3/.git/hooks/pre-commit`:

```bash
#!/bin/bash

# Pre-commit hook to prevent new unwrap() calls in production code
# Place in .git/hooks/pre-commit and make executable

HOOK_NAME=$(basename "$0")
EXIT_CODE=0

# Check for unwrap() in staged production code
UNWRAPS=$(git diff --cached --name-only --diff-filter=ACM | \
  grep '\.rs$' | \
  grep -v 'tests/' | \
  grep -v 'benches/' | \
  while read FILE; do
    if [ -f "$FILE" ]; then
      LINES=$(git diff --cached "$FILE" | grep '^+' | grep -v '^+++' | grep '\.unwrap()')
      if [ -n "$LINES" ]; then
        echo "File: $FILE"
        echo "$LINES" | sed 's/^+/  /'
      fi
    fi
  done)

if [ -n "$UNWRAPS" ]; then
  echo "ERROR: [$HOOK_NAME] Found .unwrap() calls in staged production code:" >&2
  echo "$UNWRAPS" >&2
  echo "" >&2
  echo "Guidelines:" >&2
  echo "  1. Use Result<T, E> for fallible operations" >&2
  echo "  2. Use ? operator for error propagation" >&2
  echo "  3. Use map_err() to add context" >&2
  echo "  4. .unwrap() only allowed in tests/benches" >&2
  echo "" >&2
  echo "See: ERROR_HANDLING.md" >&2
  EXIT_CODE=1
fi

# Check for panics in production code
PANICS=$(git diff --cached --name-only --diff-filter=ACM | \
  grep '\.rs$' | \
  grep -v 'tests/' | \
  grep -v 'benches/' | \
  while read FILE; do
    if [ -f "$FILE" ]; then
      LINES=$(git diff --cached "$FILE" | grep '^+' | grep -v '^+++' | grep -E 'panic!|unreachable!|expect\(')
      if [ -n "$LINES" ]; then
        echo "File: $FILE"
        echo "$LINES" | sed 's/^+/  /'
      fi
    fi
  done)

if [ -n "$PANICS" ]; then
  echo "ERROR: [$HOOK_NAME] Found panic!() or expect() in staged production code:" >&2
  echo "$PANICS" >&2
  echo "" >&2
  echo "Use Result<T, E> and propagate with ? instead." >&2
  EXIT_CODE=1
fi

exit $EXIT_CODE
```

**Installation**:
```bash
chmod +x /home/persist/repos/projects/web3/.git/hooks/pre-commit
```

**Testing the hook**:
```bash
# Try to commit code with unwrap() to see the hook in action
echo 'fn test() { let x = Some(1).unwrap(); }' >> test_unwrap.rs
git add test_unwrap.rs
git commit -m "test unwrap" # Should fail

# Clean up
git reset HEAD test_unwrap.rs
rm test_unwrap.rs
```

---

## Migration Guide

### Step 1: Identify All Production Unwraps

```bash
# Count unwraps by file
find /home/persist/repos/projects/web3 -name '*.rs' \
  ! -path '*/tests/*' \
  ! -path '*/benches/*' \
  -exec grep -l '\.unwrap()' {} \; | sort | uniq

# Get statistics
find /home/persist/repos/projects/web3 -name '*.rs' \
  ! -path '*/tests/*' \
  ! -path '*/benches/*' \
  -exec grep -c '\.unwrap()' {} + | \
  awk '{s+=$1} END {print "Total unwraps: " s}'
```

### Step 2: Categorize by Type

- **Panics**: Expected to crash (convert to Err)
- **Infallible**: Cannot fail (use direct assignment)
- **Transient**: Can fail temporarily (add retry logic)
- **Fatal**: Unrecoverable (log and propagate error)

### Step 3: Convert Pattern by Pattern

#### Pattern A: Simple Option.unwrap()

```rust
// Before
let value = option.unwrap();

// After - pattern 1: error
let value = option.ok_or(MyError::NotFound)?;

// After - pattern 2: default
let value = option.unwrap_or_default();

// After - pattern 3: local handling
let value = match option {
    Some(v) => v,
    None => return Err(MyError::NotFound),
};
```

#### Pattern B: Result.unwrap()

```rust
// Before
let value = risky_operation().unwrap();

// After
let value = risky_operation()?;
```

#### Pattern C: Iterator.unwrap() (min, max, first, last)

```rust
// Before
let min = durations.iter().min().unwrap();

// After
let min = durations.iter().min()
    .ok_or(MyError::EmptyCollection)?;

// Or if empty is impossible
let min = durations.iter().min()
    .expect("vector has at least 1 element");  // Only if provably safe
```

#### Pattern D: Async.unwrap()

```rust
// Before
let result = operation().await.unwrap();

// After
let result = operation().await?;
```

### Step 4: Test Thoroughly

```bash
# Run tests for changed modules
cargo test --lib <module_name>

# Run all tests
cargo test

# Check for new panics
cargo clippy -- -W clippy::unwrap_used
```

### Step 5: Update Metrics

After each file/module conversion:
```bash
# Before fix
BEFORE=$(grep -c '\.unwrap()' file.rs || echo 0)

# After fix
AFTER=$(grep -c '\.unwrap()' file.rs || echo 0)

echo "Removed $((BEFORE - AFTER)) unwraps from file.rs"
```

---

## Checklist for Review

When reviewing error handling changes:

- [ ] No `.unwrap()` in production code
- [ ] No `.expect()` without justification
- [ ] No `panic!()` except in macros
- [ ] `Result<T, E>` for fallible operations
- [ ] `?` operator used for propagation
- [ ] `.map_err()` adds context
- [ ] Error types have `From` implementations
- [ ] Async operations properly await before conversion
- [ ] Tests pass without warnings
- [ ] Errors logged at appropriate level
- [ ] API endpoints return structured errors
- [ ] Retryable errors are clearly marked

---

## Testing Error Handling

### Unit Test Pattern

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_propagation() {
        let result = operation_that_fails();
        assert!(matches!(result, Err(MyError::SpecificError { .. })));
    }

    #[test]
    fn test_error_context() {
        let result = operation_with_context();
        match result {
            Err(MyError::WithContext { reason, .. }) => {
                assert!(reason.contains("expected_context"));
            }
            _ => panic!("Expected WithContext error"),
        }
    }

    #[tokio::test]
    async fn test_async_error_handling() {
        let result = async_operation_that_fails().await;
        assert!(result.is_err());
    }
}
```

### Property-Based Testing

```rust
#[test]
fn prop_operation_never_panics() {
    // Use proptest or quickcheck
    proptest!(|(input in any::<Vec<u8>>()| {
        let result = operation(&input);
        // Should always return Result, never panic
        let _ = result; // Suppress unused result warning
    }));
}
```

---

## Performance Considerations

### Zero-Cost Abstractions

Error handling should have minimal overhead:

```rust
// Good: No allocation on happy path
fn operation() -> Result<T, SmallError> {
    Ok(compute_result()?)
}

// Better: Error enum is small (can fit in register)
pub enum SmallError {
    NotFound,           // 0 bytes
    InvalidInput,       // 0 bytes
    Timeout,           // 0 bytes
    Message(String),   // 24 bytes
}

// Avoid: Allocations in error path that aren't necessary
pub enum BadError {
    Details(Box<String>),  // Always allocates, even for simple errors
}
```

### Error Chain Optimization

```rust
// Avoid creating error chains in hot loops
pub fn process_items(items: Vec<Item>) -> Result<Vec<Output>, Error> {
    items
        .into_iter()
        .map(process_item)
        .collect()  // Short-circuits on first error
        // Don't do: .collect::<Result<Vec<_>, _>>().map_err(|e| error_chain)
}
```

---

## References

- **Rust Error Handling**: https://doc.rust-lang.org/book/ch09-00-error-handling.html
- **thiserror Crate**: https://docs.rs/thiserror/
- **anyhow Crate**: https://docs.rs/anyhow/ (for applications, not libraries)
- **Clippy Lint**: `clippy::unwrap_used`, `clippy::expect_used`

---

## Component Implementation Status

### TrustChain
- **Error Type**: `TrustChainError` ✅ (defined)
- **Production Unwraps**: ~243 remaining
- **Tests Unwraps**: ~78
- **Status**: In progress

### STOQ
- **Error Type**: `StoqError` ✅ (defined)
- **Production Unwraps**: ~186 remaining
- **Tests Unwraps**: ~45
- **Status**: In progress

### BlockMatrix
- **Error Types**: `StateError`, `NexusError` ✅ (defined)
- **Production Unwraps**: ~582 remaining
- **Tests Unwraps**: ~156
- **Status**: In progress

### Catalog
- **Error Type**: Not yet defined
- **Production Unwraps**: ~134 remaining
- **Tests Unwraps**: ~38
- **Status**: Not started

### Caesar
- **Error Type**: Not yet defined
- **Production Unwraps**: ~97 remaining
- **Tests Unwraps**: ~27
- **Status**: Not started

---

## Success Metrics

- Zero `unwrap()` in production code
- All operations return `Result<T, E>` or `Option<T>`
- Error messages include context and recovery hints
- API endpoints return structured error responses
- All tests pass without warnings
- Pre-commit hook blocks new unwraps
- Metrics track error categories and rates
