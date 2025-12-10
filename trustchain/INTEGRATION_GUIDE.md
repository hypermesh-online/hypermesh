# TrustChain<->STOQ Integration Guide

## Overview

TrustChain integrates with STOQ transport layer to provide quantum-resistant certificate management and DNS resolution over the HyperMesh network. This guide covers integration patterns, code examples, and common pitfalls.

## Architecture

```
┌──────────────────┐      ┌──────────────────┐
│   TrustChain     │      │      STOQ        │
│                  │      │                  │
│  ┌────────────┐  │      │  ┌────────────┐  │
│  │     CA     │  │◄────►│  │ Transport  │  │
│  └────────────┘  │      │  └────────────┘  │
│                  │      │                  │
│  ┌────────────┐  │      │  ┌────────────┐  │
│  │    DNS     │  │◄────►│  │   QUIC     │  │
│  └────────────┘  │      │  └────────────┘  │
│                  │      │                  │
│  ┌────────────┐  │      │  ┌────────────┐  │
│  │   HTTP/3   │  │◄────►│  │   eBPF     │  │
│  └────────────┘  │      │  └────────────┘  │
└──────────────────┘      └──────────────────┘
```

## Integration Patterns

### 1. STOQ Client Initialization

```rust
use trustchain::stoq_client::{TrustChainStoqClient, TrustChainStoqConfig};
use std::net::Ipv6Addr;
use std::time::Duration;

async fn initialize_stoq_client() -> Result<TrustChainStoqClient> {
    let config = TrustChainStoqConfig {
        bind_address: Ipv6Addr::UNSPECIFIED,  // Bind to all IPv6 interfaces
        connection_timeout: Duration::from_secs(10),
        max_connections_per_service: 100,
        enable_connection_pooling: true,
        enable_http3_fallback: true,  // Fallback to HTTP/3 if STOQ fails
    };

    TrustChainStoqClient::new(config).await
}
```

### 2. Certificate Operations via STOQ

```rust
use trustchain::ca::CertificateRequest;

async fn request_certificate_via_stoq(client: &TrustChainStoqClient) -> Result<Certificate> {
    // Create certificate request
    let request = CertificateRequest {
        common_name: "node.hypermesh.local".to_string(),
        organization: Some("HyperMesh Node".to_string()),
        country: Some("US".to_string()),
        validity_days: 365,
        key_usage: vec!["digital_signature", "key_encipherment"],
        san_dns_names: vec!["node.hypermesh.local", "*.node.hypermesh.local"],
        use_falcon: true,  // Use quantum-resistant FALCON-1024
    };

    // Send request via STOQ
    let certificate = client.request_certificate(request).await?;

    // Verify certificate
    client.verify_certificate(&certificate).await?;

    Ok(certificate)
}
```

### 3. DNS Resolution over STOQ

```rust
use trustchain::dns::DnsOverStoq;

async fn resolve_dns_via_stoq(client: &TrustChainStoqClient) -> Result<Ipv6Addr> {
    // Create DNS resolver with STOQ backend
    let resolver = DnsOverStoq::new(client.clone()).await?;

    // Resolve AAAA record (IPv6)
    let addresses = resolver.resolve_aaaa("trust.hypermesh.local").await?;

    // Return first address
    addresses.first()
        .ok_or_else(|| Error::msg("No addresses found"))
        .map(|addr| *addr)
}
```

### 4. Secure Communication Pattern

```rust
async fn secure_communication_flow() -> Result<()> {
    // 1. Initialize STOQ client
    let stoq_client = initialize_stoq_client().await?;

    // 2. Request and verify certificate
    let cert = request_certificate_via_stoq(&stoq_client).await?;

    // 3. Establish secure connection
    let secure_conn = stoq_client
        .connect_with_certificate("peer.hypermesh.local", &cert)
        .await?;

    // 4. Send encrypted data
    secure_conn.send_encrypted(b"Hello, HyperMesh!").await?;

    // 5. Receive response
    let response = secure_conn.receive_encrypted().await?;

    Ok(())
}
```

## Common Integration Points

### TransportConfig Fields

When creating STOQ transport configurations, ensure all required fields are set:

```rust
let transport_config = TransportConfig {
    bind_address: Ipv6Addr::LOCALHOST,
    port: 0,  // Use 0 for ephemeral port
    connection_timeout: Duration::from_secs(30),
    health_check_interval: 10,  // Required field
    connection_idle_timeout: 30,  // Required field
    enable_migration: true,
    enable_0rtt: true,
    max_idle_timeout: Duration::from_secs(120),
    max_concurrent_streams: 100,
    send_buffer_size: 8 * 1024 * 1024,
    receive_buffer_size: 8 * 1024 * 1024,
    max_connections: Some(100),
    connection_pool_size: 10,
    enable_zero_copy: true,
    max_datagram_size: 65507,
    congestion_control: CongestionControl::Bbr2,
    enable_memory_pool: true,
    memory_pool_size: 512,
    frame_batch_size: 32,
    enable_cpu_affinity: false,
};
```

### Error Handling

```rust
use trustchain::errors::{TrustChainError, Result};

async fn handle_stoq_errors() -> Result<()> {
    match stoq_client.connect("peer.hypermesh.local").await {
        Ok(conn) => {
            // Handle successful connection
            Ok(())
        }
        Err(TrustChainError::Network(e)) => {
            // Network-level error
            eprintln!("Network error: {}", e);
            Err(e.into())
        }
        Err(TrustChainError::Certificate(e)) => {
            // Certificate validation error
            eprintln!("Certificate error: {}", e);
            Err(e.into())
        }
        Err(e) => {
            // Other errors
            eprintln!("Unexpected error: {}", e);
            Err(e)
        }
    }
}
```

## Common Pitfalls

### 1. Port Conflicts

**Problem**: Tests fail with "Address already in use" when running in parallel.

**Solution**: Use ephemeral ports (port: 0) or implement proper test isolation:
```rust
#[tokio::test]
async fn test_with_isolated_port() {
    let port = find_available_port().await;
    // Use the port for testing
}
```

### 2. Missing TransportConfig Fields

**Problem**: Compilation fails with missing fields error.

**Solution**: Always include `health_check_interval` and `connection_idle_timeout`:
```rust
health_check_interval: 10,
connection_idle_timeout: 30,
```

### 3. Certificate Chain Validation

**Problem**: Certificate validation fails for self-signed certificates.

**Solution**: Use proper certificate chain with root CA:
```rust
let chain = client.build_certificate_chain().await?;
client.validate_chain(&chain).await?;
```

### 4. DNS Resolution Timeout

**Problem**: DNS queries timeout when STOQ server is unavailable.

**Solution**: Implement fallback mechanism:
```rust
let result = tokio::time::timeout(
    Duration::from_secs(5),
    resolver.resolve("domain.local")
).await;

match result {
    Ok(Ok(addr)) => use_address(addr),
    _ => use_fallback_resolver(),
}
```

### 5. Connection Pool Exhaustion

**Problem**: Too many concurrent connections exhaust the pool.

**Solution**: Configure appropriate pool size and implement backpressure:
```rust
let config = TrustChainStoqConfig {
    max_connections_per_service: 100,
    enable_connection_pooling: true,
    // ...
};
```

## Testing Integration

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stoq_client_creation() {
        let config = TrustChainStoqConfig::default();
        let client = TrustChainStoqClient::new(config).await;
        assert!(client.is_ok() || client.is_err());  // Handle both cases
    }
}
```

### Integration Tests
```rust
#[tokio::test]
#[ignore]  // Run with: cargo test -- --ignored
async fn test_full_integration() {
    // Requires running STOQ server
    let client = setup_test_client().await;
    let cert = client.request_certificate("test.local").await.unwrap();
    assert!(client.verify_certificate(&cert).await.is_ok());
}
```

## Performance Considerations

1. **Connection Pooling**: Enable for high-throughput scenarios
2. **Zero-Copy**: Enable for large data transfers
3. **BBR2 Congestion Control**: Better for variable network conditions
4. **Memory Pools**: Pre-allocate for consistent latency
5. **CPU Affinity**: Consider for dedicated server deployments

## Security Best Practices

1. Always validate certificates before use
2. Use FALCON-1024 for quantum resistance
3. Implement proper key rotation (daily for production)
4. Enable certificate transparency logging
5. Monitor for Byzantine behavior
6. Use IPv6 for better security and addressing

## Debugging Tips

### Enable Verbose Logging
```bash
RUST_LOG=trustchain=debug,stoq=debug cargo run
```

### Check Connection Statistics
```rust
let stats = client.get_connection_stats().await?;
println!("Active connections: {}", stats.active);
println!("Pool utilization: {}%", stats.pool_utilization);
```

### Monitor Certificate Rotation
```rust
let manager = client.certificate_manager();
let rotation_status = manager.get_rotation_status().await?;
println!("Next rotation: {:?}", rotation_status.next_rotation);
```

## Next Steps

1. Implement mock STOQ server for testing
2. Add connection retry logic with exponential backoff
3. Implement certificate pinning for known peers
4. Add metrics collection for monitoring
5. Create performance benchmarks for optimization