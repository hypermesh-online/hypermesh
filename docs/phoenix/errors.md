# Phoenix SDK Error Handling Guide

This guide covers all error scenarios you may encounter with Phoenix SDK and how to handle them properly.

## Error Types

Phoenix SDK uses `anyhow::Result` for error handling, providing rich context for debugging.

### Common Error Categories

1. **Connection Errors** - Network connectivity issues
2. **Certificate Errors** - TLS/security validation failures
3. **Resource Errors** - System resource limitations
4. **Protocol Errors** - QUIC protocol violations
5. **Configuration Errors** - Invalid settings

## Connection Errors

### ConnectionTimeout

**Error Message**: `Failed to connect to [::1]:9292: Connection timeout`

**Causes**:
- Server not running or not listening on the port
- Firewall blocking the connection
- Network unreachable
- IPv6 not enabled on the system

**Solutions**:
```rust
use std::time::Duration;
use tokio::time::timeout;

// Implement retry with exponential backoff
async fn connect_with_retry(phoenix: &PhoenixTransport, endpoint: &str) -> Result<PhoenixConnection> {
    let mut retry_delay = Duration::from_millis(100);
    let max_retries = 5;

    for attempt in 0..max_retries {
        match timeout(Duration::from_secs(5), phoenix.connect(endpoint)).await {
            Ok(Ok(conn)) => return Ok(conn),
            Ok(Err(e)) if attempt < max_retries - 1 => {
                eprintln!("Connection attempt {} failed: {}", attempt + 1, e);
                tokio::time::sleep(retry_delay).await;
                retry_delay *= 2; // Exponential backoff
            }
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                eprintln!("Connection timeout on attempt {}", attempt + 1);
                if attempt == max_retries - 1 {
                    return Err(anyhow::anyhow!("Connection timeout after {} attempts", max_retries));
                }
            }
        }
    }
    unreachable!()
}
```

### NetworkUnreachable

**Error Message**: `Network unreachable`

**Causes**:
- No route to host
- DNS resolution failure
- IPv6 not configured
- Network interface down

**Solutions**:
```rust
// Check network before connecting
async fn check_network_and_connect(phoenix: &PhoenixTransport, endpoint: &str) -> Result<PhoenixConnection> {
    // Verify IPv6 is available
    if !is_ipv6_available() {
        return Err(anyhow::anyhow!("IPv6 not available on this system"));
    }

    // Try to resolve the endpoint
    let (host, port) = parse_endpoint(endpoint)?;

    // Attempt connection
    phoenix.connect(endpoint).await.map_err(|e| {
        anyhow::anyhow!("Failed to connect to {}:{} - {}", host, port, e)
    })
}

fn is_ipv6_available() -> bool {
    // Check if ::1 is reachable
    std::net::TcpStream::connect_timeout(
        &"[::1]:1".parse().unwrap(),
        std::time::Duration::from_millis(100)
    ).is_err() // Will fail but confirms IPv6 stack exists
}
```

### ConnectionRefused

**Error Message**: `Connection refused`

**Causes**:
- Server not running
- Wrong port number
- Server at capacity
- Firewall rejection

**Solutions**:
```rust
// Handle connection refused with fallback
async fn connect_with_fallback(phoenix: &PhoenixTransport) -> Result<PhoenixConnection> {
    // Try primary endpoint
    match phoenix.connect("[::1]:9292").await {
        Ok(conn) => Ok(conn),
        Err(e) if e.to_string().contains("refused") => {
            eprintln!("Primary server unavailable, trying fallback");
            // Try fallback endpoint
            phoenix.connect("[::1]:9293").await
        }
        Err(e) => Err(e)
    }
}
```

## Certificate Errors

### CertificateValidationFailure

**Error Message**: `Certificate validation failed: self-signed certificate`

**Causes**:
- Self-signed certificates in use
- Certificate expired
- Certificate hostname mismatch
- Missing CA certificates

**Solutions**:
```rust
use phoenix_sdk::phoenix::{PhoenixConfig, PhoenixTransport};

// For development - accept self-signed certificates
async fn create_dev_phoenix() -> Result<PhoenixTransport> {
    let config = PhoenixConfig {
        app_id: "dev-app".to_string(),
        auto_certificates: true,  // Auto-generate self-signed
        ..Default::default()
    };

    PhoenixTransport::with_config(config).await
}

// For production - use proper certificates
async fn create_prod_phoenix() -> Result<PhoenixTransport> {
    // Ensure proper certificates are installed
    verify_certificates()?;

    let config = PhoenixConfig {
        app_id: "prod-app".to_string(),
        auto_certificates: false,  // Use system certificates
        ..Default::default()
    };

    PhoenixTransport::with_config(config).await
}
```

### CertificateExpired

**Error Message**: `Certificate expired`

**Causes**:
- System time incorrect
- Certificate actually expired
- Certificate not yet valid

**Solutions**:
```rust
// Check and handle certificate expiry
fn check_certificate_validity() -> Result<()> {
    let cert_path = "/path/to/cert.pem";

    // In real implementation, parse certificate and check dates
    // For now, just check file exists and is recent
    let metadata = std::fs::metadata(cert_path)?;
    let modified = metadata.modified()?;
    let age = std::time::SystemTime::now().duration_since(modified)?;

    if age > std::time::Duration::from_secs(86400 * 30) { // 30 days
        return Err(anyhow::anyhow!("Certificate may be expired"));
    }

    Ok(())
}
```

## Resource Errors

### TooManyConnections

**Error Message**: `Maximum connection limit reached`

**Causes**:
- Connection pool exhausted
- System file descriptor limit
- Application connection limit

**Solutions**:
```rust
use std::sync::Arc;
use tokio::sync::Semaphore;

// Implement connection limiting
struct ConnectionManager {
    phoenix: PhoenixTransport,
    semaphore: Arc<Semaphore>,
}

impl ConnectionManager {
    fn new(phoenix: PhoenixTransport, max_connections: usize) -> Self {
        Self {
            phoenix,
            semaphore: Arc::new(Semaphore::new(max_connections)),
        }
    }

    async fn connect(&self, endpoint: &str) -> Result<PhoenixConnection> {
        // Acquire permit before connecting
        let _permit = self.semaphore.acquire().await?;

        self.phoenix.connect(endpoint).await
    }
}
```

### OutOfMemory

**Error Message**: `Failed to allocate memory for buffer`

**Causes**:
- Buffer size too large
- Memory leak
- System memory exhausted

**Solutions**:
```rust
// Use smaller buffers for memory-constrained environments
async fn create_low_memory_phoenix() -> Result<PhoenixTransport> {
    let config = PhoenixConfig {
        app_id: "low-mem-app".to_string(),
        high_performance: false,  // Smaller buffers
        max_connections: 10,      // Limit connections
        ..Default::default()
    };

    PhoenixTransport::with_config(config).await
}

// Monitor memory usage
async fn send_with_memory_check(conn: &mut PhoenixConnection, data: &[u8]) -> Result<()> {
    let mem_before = get_memory_usage();

    if mem_before > 1024 * 1024 * 1024 { // 1GB threshold
        return Err(anyhow::anyhow!("Memory usage too high: {} MB", mem_before / 1024 / 1024));
    }

    conn.send_data(data).await?;

    let mem_after = get_memory_usage();
    if mem_after > mem_before * 2 {
        eprintln!("Warning: Memory usage doubled during send");
    }

    Ok(())
}

fn get_memory_usage() -> usize {
    // Platform-specific memory query
    // This is a simplified example
    100 * 1024 * 1024 // 100MB placeholder
}
```

## Protocol Errors

### StreamReset

**Error Message**: `Stream reset by peer`

**Causes**:
- Peer closed stream unexpectedly
- Protocol violation
- Timeout on peer side

**Solutions**:
```rust
// Handle stream resets gracefully
async fn send_with_stream_recovery(conn: &mut PhoenixConnection, data: &[u8]) -> Result<()> {
    let mut retries = 3;

    loop {
        match conn.send_data(data).await {
            Ok(()) => return Ok(()),
            Err(e) if e.to_string().contains("reset") && retries > 0 => {
                eprintln!("Stream reset, retrying... ({} left)", retries);
                retries -= 1;
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### InvalidPacket

**Error Message**: `Invalid packet format`

**Causes**:
- Corrupted data
- Version mismatch
- Incompatible protocol

**Solutions**:
```rust
// Validate data before sending
fn validate_packet(data: &[u8]) -> Result<()> {
    if data.is_empty() {
        return Err(anyhow::anyhow!("Cannot send empty packet"));
    }

    if data.len() > 10 * 1024 * 1024 { // 10MB limit
        return Err(anyhow::anyhow!("Packet too large: {} bytes", data.len()));
    }

    Ok(())
}

async fn send_validated(conn: &mut PhoenixConnection, data: &[u8]) -> Result<()> {
    validate_packet(data)?;
    conn.send_data(data).await
}
```

## Configuration Errors

### InvalidConfiguration

**Error Message**: `Invalid configuration: port must be between 1 and 65535`

**Causes**:
- Invalid port number
- Invalid IPv6 address
- Conflicting settings

**Solutions**:
```rust
// Validate configuration before use
fn validate_config(config: &PhoenixConfig) -> Result<()> {
    // Validate port
    if config.port > 65535 {
        return Err(anyhow::anyhow!("Invalid port: {}", config.port));
    }

    // Validate connection limit
    if config.max_connections == 0 {
        return Err(anyhow::anyhow!("max_connections must be > 0"));
    }

    if config.max_connections > 10000 {
        eprintln!("Warning: {} connections may exhaust resources", config.max_connections);
    }

    Ok(())
}

// Builder pattern with validation
async fn create_validated_phoenix(app_id: &str, port: u16) -> Result<PhoenixTransport> {
    if port == 0 || port > 65535 {
        return Err(anyhow::anyhow!("Invalid port: {}", port));
    }

    PhoenixBuilder::new(app_id)
        .port(port)
        .build()
        .await
}
```

## Error Recovery Patterns

### Circuit Breaker Pattern

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

struct CircuitBreaker {
    failures: AtomicUsize,
    last_failure: tokio::sync::Mutex<Option<Instant>>,
    threshold: usize,
    timeout: Duration,
}

impl CircuitBreaker {
    fn new(threshold: usize, timeout: Duration) -> Self {
        Self {
            failures: AtomicUsize::new(0),
            last_failure: tokio::sync::Mutex::new(None),
            threshold,
            timeout,
        }
    }

    async fn call<F, T>(&self, f: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        // Check if circuit is open
        if self.is_open().await {
            return Err(anyhow::anyhow!("Circuit breaker is open"));
        }

        // Try the operation
        match f.await {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(e)
            }
        }
    }

    async fn is_open(&self) -> bool {
        let failures = self.failures.load(Ordering::Relaxed);
        if failures >= self.threshold {
            // Check timeout
            let last = self.last_failure.lock().await;
            if let Some(time) = *last {
                return time.elapsed() < self.timeout;
            }
        }
        false
    }

    fn on_success(&self) {
        self.failures.store(0, Ordering::Relaxed);
    }

    async fn on_failure(&self) {
        self.failures.fetch_add(1, Ordering::Relaxed);
        *self.last_failure.lock().await = Some(Instant::now());
    }
}
```

### Bulkhead Pattern

```rust
use tokio::sync::Semaphore;

struct Bulkhead {
    semaphore: Arc<Semaphore>,
}

impl Bulkhead {
    fn new(capacity: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(capacity)),
        }
    }

    async fn execute<F, T>(&self, f: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        let _permit = self.semaphore.acquire().await
            .map_err(|_| anyhow::anyhow!("Bulkhead capacity exceeded"))?;

        f.await
    }
}
```

## Logging and Debugging

### Enable Detailed Logging

```rust
use tracing::{info, debug, error, warn};

// Initialize tracing
fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter("phoenix_sdk=debug,stoq=debug")
        .init();
}

// Log errors with context
async fn connect_with_logging(phoenix: &PhoenixTransport, endpoint: &str) -> Result<PhoenixConnection> {
    info!("Attempting connection to {}", endpoint);

    match phoenix.connect(endpoint).await {
        Ok(conn) => {
            info!("Successfully connected to {}", endpoint);
            Ok(conn)
        }
        Err(e) => {
            error!("Failed to connect to {}: {}", endpoint, e);

            // Add more context for debugging
            debug!("Error details: {:?}", e);
            debug!("Phoenix config: {:?}", phoenix.config());

            Err(e)
        }
    }
}
```

## Common Error Messages Reference

| Error | Likely Cause | Quick Fix |
|-------|-------------|-----------|
| `Connection timeout` | Server down/unreachable | Check server status, firewall |
| `Connection refused` | Wrong port/server not running | Verify endpoint, start server |
| `Certificate validation failed` | Self-signed cert | Use auto_certificates for dev |
| `Stream reset by peer` | Protocol error/timeout | Implement retry logic |
| `Maximum connections reached` | Pool exhausted | Increase max_connections |
| `Network unreachable` | No route/IPv6 disabled | Check network config |
| `Invalid endpoint format` | Malformed address | Use `[ipv6]:port` format |
| `Out of memory` | Large buffers/leak | Reduce buffer size, check leaks |

## Best Practices

1. **Always handle errors explicitly** - Don't use `unwrap()` in production
2. **Implement retry logic** - Network operations can fail transiently
3. **Use timeouts** - Prevent indefinite blocking
4. **Log errors with context** - Include relevant state information
5. **Validate input** - Check configuration and data before use
6. **Monitor resources** - Track memory and connection usage
7. **Implement circuit breakers** - Prevent cascading failures
8. **Test error paths** - Ensure error handling works correctly

## Testing Error Handling

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_timeout() {
        let phoenix = PhoenixTransport::new("test").await.unwrap();

        // Connect to non-existent server
        let result = phoenix.connect("[::1]:99999").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("refused") ||
                result.unwrap_err().to_string().contains("invalid"));
    }

    #[tokio::test]
    async fn test_retry_logic() {
        let phoenix = PhoenixTransport::new("test").await.unwrap();

        let conn = connect_with_retry(&phoenix, "[::1]:9292").await;

        // Should fail after max retries if server not running
        assert!(conn.is_err() || conn.is_ok());
    }
}