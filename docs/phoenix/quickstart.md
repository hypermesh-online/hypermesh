# Phoenix SDK - Quick Start Guide

Get up and running with Phoenix SDK in 5 minutes.

## Prerequisites

- Rust 1.70+ installed
- IPv6 support on your system
- 4GB+ RAM recommended

## Installation

Add Phoenix SDK to your `Cargo.toml`:

```toml
[dependencies]
phoenix-sdk = { path = "../stoq" }  # Or from crates.io when published
tokio = { version = "1.40", features = ["full"] }
anyhow = "1.0"
```

## Hello Phoenix - Your First Application

### Step 1: Create a Simple Server

```rust
// server.rs
use phoenix_sdk::phoenix::{PhoenixTransport};
use tokio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize Phoenix transport
    let phoenix = PhoenixTransport::new("hello-server").await?;

    println!("Server listening...");

    // Accept incoming connections
    loop {
        let mut conn = phoenix.accept().await?;

        // Handle connection in separate task
        tokio::spawn(async move {
            while let Ok(data) = conn.receive_data().await {
                println!("Received: {} bytes", data.len());

                // Echo back
                let response = format!("Echo: {} bytes received", data.len());
                conn.send_data(response.as_bytes()).await.ok();
            }
        });
    }
}
```

### Step 2: Create a Client

```rust
// client.rs
use phoenix_sdk::phoenix::{PhoenixTransport};
use tokio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize Phoenix transport
    let phoenix = PhoenixTransport::new("hello-client").await?;

    // Connect to server
    let mut conn = phoenix.connect("[::1]:9292").await?;

    // Send data
    let message = b"Hello, Phoenix!";
    conn.send_data(message).await?;
    println!("Sent: {:?}", std::str::from_utf8(message)?);

    // Receive response
    let response = conn.receive_data().await?;
    println!("Received: {:?}", std::str::from_utf8(&response)?);

    Ok(())
}
```

### Step 3: Run Your Application

Terminal 1 - Start the server:
```bash
cargo run --bin server
```

Terminal 2 - Run the client:
```bash
cargo run --bin client
```

## Configuration Options

### High-Performance Mode

Enable optimizations for production workloads:

```rust
use phoenix_sdk::phoenix::{PhoenixBuilder};

let phoenix = PhoenixBuilder::new("my-app")
    .high_performance(true)  // Enable all optimizations
    .max_connections(100)     // Connection pool size
    .port(19292)             // Custom port
    .build()
    .await?;
```

### Custom Configuration

Fine-tune Phoenix for your use case:

```rust
use phoenix_sdk::phoenix::{PhoenixConfig, PhoenixTransport};
use std::net::Ipv6Addr;

let config = PhoenixConfig {
    app_id: "custom-app".to_string(),
    bind_address: Ipv6Addr::LOCALHOST,
    port: 8080,
    high_performance: true,
    max_connections: 50,
    auto_certificates: true,  // Automatic certificate management
};

let phoenix = PhoenixTransport::with_config(config).await?;
```

## Common Patterns

### Request-Response Pattern

```rust
// Server side
let mut conn = phoenix.accept().await?;
let request = conn.receive_data().await?;
let response = process_request(&request);
conn.send_data(&response).await?;

// Client side
let mut conn = phoenix.connect("server:8080").await?;
conn.send_data(&request).await?;
let response = conn.receive_data().await?;
```

### Streaming Pattern

```rust
// Stream data continuously
let mut conn = phoenix.connect("server:8080").await?;

// Producer
for chunk in data_source {
    conn.send_data(&chunk).await?;
}

// Consumer
while let Ok(chunk) = conn.receive_data().await {
    process_chunk(&chunk);
}
```

### Connection Pooling

Phoenix automatically pools connections for efficiency:

```rust
// First connection - creates new
let conn1 = phoenix.connect("server:8080").await?;

// Second connection - may reuse from pool
let conn2 = phoenix.connect("server:8080").await?;

// Connections are automatically returned to pool when dropped
```

## Performance Tips

### 1. Use Bytes for Zero-Copy

```rust
use bytes::Bytes;

// Zero-copy send when possible
let data = Bytes::from_static(b"static data");
conn.send_bytes(data).await?;
```

### 2. Enable Multiplexing for Throughput

```rust
// Use multiple connections for parallel transfers
phoenix.enable_multiplexing("server:8080", 10).await?;

// Send using multiplexed connections
phoenix.send_multiplexed("server:8080", &large_data).await?;
```

### 3. Monitor Performance

```rust
// Get performance statistics
let stats = phoenix.stats().await;
println!("Throughput: {:.2} Gbps", stats.throughput_gbps);
println!("Active connections: {}", stats.active_connections);
println!("Latency: {:.2} ms", stats.latency_ms);
```

## Error Handling

```rust
use phoenix_sdk::phoenix::PhoenixTransport;
use anyhow::Result;

async fn robust_connect(phoenix: &PhoenixTransport, endpoint: &str) -> Result<()> {
    match phoenix.connect(endpoint).await {
        Ok(mut conn) => {
            // Connection successful
            conn.send_data(b"Hello").await?;
            Ok(())
        }
        Err(e) => {
            eprintln!("Connection failed: {}", e);

            // Implement retry logic
            tokio::time::sleep(Duration::from_secs(1)).await;
            robust_connect(phoenix, endpoint).await
        }
    }
}
```

## Real Performance Expectations

Based on actual measurements, expect:

| Environment | Typical Throughput | Latency |
|-------------|-------------------|---------|
| Local Development | 100-500 Mbps | <1ms |
| Same Datacenter | 1-5 Gbps | <5ms |
| Cross-Region | 100-500 Mbps | 20-100ms |
| Internet | 10-100 Mbps | 50-200ms |

**Note**: Performance depends heavily on:
- Network infrastructure quality
- CPU performance (single-thread limited)
- Memory bandwidth
- Concurrent connection count

## Next Steps

- [API Reference](api/README.md) - Detailed API documentation
- [Performance Guide](performance.md) - Optimization techniques
- [Error Handling](errors.md) - Comprehensive error handling
- [Examples](/stoq/examples/) - More example applications

## Troubleshooting

### Connection Refused

```
Error: Failed to connect to [::1]:9292
```
**Solution**: Ensure server is running and IPv6 is enabled on your system.

### Certificate Errors

```
Error: Certificate validation failed
```
**Solution**: Phoenix auto-provisions certificates. Ensure ports aren't blocked by firewall.

### Performance Issues

If throughput is lower than expected:
1. Check network bandwidth limits
2. Verify CPU isn't saturated
3. Increase buffer sizes in configuration
4. Use multiplexing for parallel transfers

## Example Projects

Check the `/stoq/examples/` directory for complete examples:
- `benchmark_real.rs` - Performance benchmarking
- `monitoring_demo.rs` - Monitoring integration
- More examples coming soon!