# Error Handling Patterns for Sprint 2.2

## Overview
This document provides specific before/after patterns for eliminating unwrap() calls in the TrustChain codebase.

---

## Pattern 1: Path to String Conversion

### Before (Common in file operations)
```rust
let storage = CTStorage::new(temp_dir.path().to_str().unwrap()).await.unwrap();
```

### After
```rust
let storage = CTStorage::new(
    temp_dir.path()
        .to_str()
        .ok_or_else(|| anyhow!("Invalid UTF-8 in path"))?
).await
.context("Failed to initialize CT storage")?;
```

### Rationale
- `to_str()` returns `Option<&str>` because paths may contain invalid UTF-8
- `ok_or_else` converts Option to Result with meaningful error
- `context()` adds operation-specific context for debugging

---

## Pattern 2: Mutex/RwLock Unwrap

### Before (Common in shared state)
```rust
let mut cache = self.cache.lock().unwrap();
cache.insert(key, value);
```

### After
```rust
let mut cache = self.cache.lock()
    .map_err(|e| anyhow!("Cache lock poisoned: {}", e))?;
cache.insert(key, value);
```

### Rationale
- Lock poisoning can occur if a thread panics while holding the lock
- Proper error handling allows recovery or clean shutdown
- Error message indicates which lock failed

---

## Pattern 3: String Parsing

### Before (Common in config/network code)
```rust
let port: u16 = port_str.parse().unwrap();
let addr = SocketAddr::new(ip, port);
```

### After
```rust
let port: u16 = port_str.parse()
    .with_context(|| format!("Invalid port number: {}", port_str))?;
let addr = SocketAddr::new(ip, port);
```

### Rationale
- User input should never cause panics
- Error message includes the invalid input for debugging
- `with_context` is ideal for adding dynamic error details

---

## Pattern 4: Option::unwrap in Business Logic

### Before
```rust
fn get_certificate(&self, fingerprint: &str) -> Certificate {
    self.cache.get(fingerprint).unwrap()
}
```

### After
```rust
fn get_certificate(&self, fingerprint: &str) -> Result<Certificate> {
    self.cache.get(fingerprint)
        .ok_or_else(|| anyhow!("Certificate not found: {}", fingerprint))
}
```

### Rationale
- Missing data is not an exceptional condition in many cases
- Caller can decide how to handle missing certificates
- Function signature now accurately represents possible outcomes

---

## Pattern 5: Crypto Operations

### Before (Common in key generation)
```rust
let keypair = kyber::generate_keypair().unwrap();
let ciphertext = kyber::encrypt(&public_key, &plaintext).unwrap();
```

### After
```rust
let keypair = kyber::generate_keypair()
    .map_err(|e| StoqError::CryptoError(format!("Kyber key generation failed: {}", e)))?;

let ciphertext = kyber::encrypt(&public_key, &plaintext)
    .map_err(|e| StoqError::CryptoError(format!("Kyber encryption failed: {}", e)))?;
```

### Rationale
- Crypto failures are rare but must be handled (RNG failures, invalid keys)
- Use domain-specific error type (StoqError)
- Distinguish between different crypto operation failures

---

## Pattern 6: JSON/Serialization

### Before
```rust
let config: Config = serde_json::from_str(&json_str).unwrap();
let serialized = serde_json::to_string(&config).unwrap();
```

### After
```rust
let config: Config = serde_json::from_str(&json_str)
    .context("Failed to parse configuration JSON")?;

let serialized = serde_json::to_string(&config)
    .context("Failed to serialize configuration")?;
```

### Rationale
- Malformed JSON should not panic the server
- Serialization can fail on deeply nested structures
- Context helps identify which configuration failed

---

## Pattern 7: Network Address Parsing

### Before
```rust
let addr: SocketAddr = addr_str.parse().unwrap();
```

### After
```rust
let addr: SocketAddr = addr_str.parse()
    .with_context(|| format!("Invalid socket address: {}", addr_str))?;
```

### Rationale
- Network configuration often comes from external sources
- Invalid addresses should be configuration errors, not panics
- Include the invalid input in error message

---

## Pattern 8: Certificate Parsing

### Before
```rust
let cert = Certificate::from_pem(pem_data).unwrap();
let fingerprint = cert.fingerprint().unwrap();
```

### After
```rust
let cert = Certificate::from_pem(pem_data)
    .context("Failed to parse PEM certificate")?;

let fingerprint = cert.fingerprint()
    .context("Failed to compute certificate fingerprint")?;
```

### Rationale
- Certificates can be malformed or corrupted
- Fingerprint calculation can fail on invalid ASN.1
- Clear error messages for debugging certificate issues

---

## Pattern 9: Time/Duration Operations

### Before
```rust
let duration = SystemTime::now().duration_since(start_time).unwrap();
```

### After
```rust
let duration = SystemTime::now()
    .duration_since(start_time)
    .map_err(|_| anyhow!("System time moved backwards"))?;
```

### Rationale
- System time can go backwards (NTP adjustments, VM snapshots)
- Time-based logic must handle this gracefully
- Error indicates a specific time-related issue

---

## Pattern 10: Array Indexing

### Before
```rust
let first_entry = entries[0].unwrap();
```

### After
```rust
let first_entry = entries.first()
    .ok_or_else(|| anyhow!("No entries found in log"))?;
```

### Rationale
- Empty collections are common in real-world scenarios
- `first()` is more idiomatic than indexing for optional access
- Error message clarifies what was missing

---

## Pattern 11: Channel Send/Recv

### Before
```rust
tx.send(message).unwrap();
let message = rx.recv().unwrap();
```

### After
```rust
tx.send(message)
    .map_err(|_| anyhow!("Channel receiver dropped"))?;

let message = rx.recv()
    .map_err(|_| anyhow!("Channel sender dropped or closed"))?;
```

### Rationale
- Channels can close if other end drops (common in shutdown scenarios)
- Proper error handling allows clean shutdown
- Distinguishes send vs recv failures

---

## Pattern 12: File I/O

### Before
```rust
let content = fs::read_to_string(path).unwrap();
fs::write(path, data).unwrap();
```

### After
```rust
let content = fs::read_to_string(path)
    .with_context(|| format!("Failed to read file: {}", path.display()))?;

fs::write(path, data)
    .with_context(|| format!("Failed to write file: {}", path.display()))?;
```

### Rationale
- File operations fail for many reasons (permissions, disk full, etc.)
- Include path in error for debugging
- `path.display()` handles non-UTF-8 paths safely

---

## Pattern 13: Async Join Handles

### Before
```rust
let result = handle.await.unwrap();
```

### After
```rust
let result = handle.await
    .map_err(|e| anyhow!("Task panicked: {}", e))?;
```

### Rationale
- Tasks can panic, causing join to fail
- Propagate panic info for debugging
- Allows graceful handling of worker failures

---

## Pattern 14: Environment Variables

### Before
```rust
let var = env::var("CONFIG_PATH").unwrap();
```

### After
```rust
let var = env::var("CONFIG_PATH")
    .context("CONFIG_PATH environment variable not set")?;
```

### Rationale
- Environment variables may not be set
- Configuration errors should be clear
- Context indicates which variable is missing

---

## Pattern 15: Collection Operations

### Before
```rust
let max = values.iter().max().unwrap();
```

### After
```rust
let max = values.iter().max()
    .ok_or_else(|| anyhow!("Cannot find max of empty collection"))?;
```

### Rationale
- Many collection methods return `Option` for empty cases
- Empty collections should be handled explicitly
- Error message clarifies the operation that failed

---

## Testing Patterns

### For Test Code (acceptable to keep some unwraps)
```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_storage() {
        // Setup can use unwrap for brevity
        let temp_dir = TempDir::new().unwrap();

        // But assertions should be explicit
        let result = storage.get_entry(0).await;
        assert!(result.is_ok(), "Failed to get entry: {:?}", result.err());

        let entry = result.unwrap();
        assert_eq!(entry.sequence, 0);
    }
}
```

### Rationale for Test Unwraps
- Test setup failures should fail fast
- Unwraps in tests are caught during development
- Focus on testing business logic, not error paths
- Still prefer `expect()` with messages for clarity

---

## Error Type Selection Guide

### Use `anyhow::Result` when:
- Application/binary code
- Error details are for debugging only
- Multiple error types from different libraries
- No need for programmatic error handling

### Use `StoqError` when:
- Library code that others will use
- Callers need to handle specific error cases
- Domain-specific error semantics
- Error recovery is possible

### Example: Hybrid Approach
```rust
// Library function
pub fn validate_certificate(cert: &Certificate) -> Result<(), StoqError> {
    // Use domain-specific errors
    if cert.is_expired() {
        return Err(StoqError::CertificateExpired);
    }
    Ok(())
}

// Application code
async fn load_and_validate(path: &Path) -> anyhow::Result<Certificate> {
    let pem = fs::read_to_string(path)
        .context("Failed to read certificate file")?; // anyhow for context

    let cert = Certificate::from_pem(&pem)
        .map_err(|e| anyhow!("Invalid PEM: {}", e))?; // Convert to anyhow

    validate_certificate(&cert)
        .map_err(|e| anyhow!("Validation failed: {}", e))?; // Convert StoqError

    Ok(cert)
}
```

---

## Quick Reference

| Original | Replacement | Use Case |
|----------|-------------|----------|
| `.unwrap()` | `.context("...")?` | Add context to errors |
| `.unwrap()` | `.ok_or_else(\|\| anyhow!("..."))?` | Option to Result |
| `.unwrap()` | `.map_err(\|e\| ...)?` | Transform error type |
| `.unwrap()` | `.with_context(\|\| format!("...", var))?` | Dynamic error context |
| `.expect("msg")` | `.context("msg")?` | Better propagation |

---

## Common Imports Needed

```rust
use anyhow::{Context, Result, anyhow};
use crate::errors::StoqError; // For domain-specific errors
```

Most modules already have these imports; if not, add to the top of the file.