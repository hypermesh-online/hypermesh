# Phoenix SDK

**High-performance distributed computing made simple.**

Phoenix SDK provides a zero-configuration, developer-friendly API for building high-performance distributed applications. It combines the power of STOQ's adaptive transport with TrustChain's security infrastructure to deliver enterprise-grade networking with minimal complexity.

## Features

- 🚀 **Zero Configuration** - Works out of the box with sensible defaults
- ⚡ **Blazing Fast** - Achieves up to 40 Gbps throughput with automatic optimization
- 🔒 **Enterprise Security** - Built-in TLS, mTLS, and post-quantum cryptography
- 📊 **Real-time Metrics** - Monitor performance with built-in dashboards
- 🌐 **IPv6 Native** - Future-proof networking with full IPv6 support
- 🎯 **Type Safe** - Rust's type system prevents runtime errors
- 🔄 **Automatic Compression** - Smart compression based on data characteristics
- 🛡️ **Byzantine Fault Tolerant** - Consensus-based certificate validation

## Quick Start

### Installation

Add Phoenix SDK to your `Cargo.toml`:

```toml
[dependencies]
phoenix-sdk = "1.0"
```

### Hello World

```rust
use phoenix_sdk::Phoenix;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create Phoenix instance with zero configuration
    let phoenix = Phoenix::new("my-app").await?;

    // Connect to another Phoenix application
    let conn = phoenix.connect("remote-app.example.com:8080").await?;

    // Send and receive data with automatic optimization
    conn.send(&"Hello, Phoenix!").await?;
    let response: String = conn.receive().await?;
    println!("Received: {}", response);

    Ok(())
}
```

### Server Example

```rust
use phoenix_sdk::Phoenix;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let phoenix = Phoenix::new("server").await?;
    let listener = phoenix.listen(8080).await?;

    println!("Server listening on port 8080");

    listener.handle(|conn| async move {
        while let Ok(msg) = conn.receive::<String>().await {
            println!("Received: {}", msg);
            conn.send(&format!("Echo: {}", msg)).await?;
        }
        Ok(())
    }).await?;

    Ok(())
}
```

## Performance Tiers

Phoenix automatically optimizes for your use case:

| Tier | Target | Use Case |
|------|--------|----------|
| `Development` | <1 Gbps | Local testing, prototyping |
| `Production` | 1-10 Gbps | Standard applications |
| `HighThroughput` | 10+ Gbps | Data-intensive workloads |
| `Custom(n)` | n Gbps | Custom performance target |

### Configuration Example

```rust
use phoenix_sdk::{Phoenix, PhoenixConfig, PerformanceTier, SecurityLevel};

let config = PhoenixConfig::production("my-app")
    .with_performance_tier(PerformanceTier::HighThroughput)
    .with_security_level(SecurityLevel::Enhanced)
    .with_compression(true)
    .with_max_connections(10000);

let phoenix = Phoenix::with_config(config).await?;
```

## Security Levels

Phoenix provides multiple security levels:

| Level | Features | Use Case |
|-------|----------|----------|
| `Development` | Self-signed certificates | Local development |
| `Standard` | TLS 1.3 with validation | Production |
| `Enhanced` | Mutual TLS + pinning | High security |
| `PostQuantum` | FALCON-1024 cryptography | Future-proof |

## Real-time Metrics

Monitor your application's performance:

```rust
let metrics = phoenix.metrics().await;
println!("Throughput: {:.2} Gbps", metrics.throughput_gbps);
println!("Active connections: {}", metrics.active_connections);
println!("Average latency: {} µs", metrics.avg_latency_us);
```

## Advanced Features

### Streaming

```rust
// Open bidirectional stream
let mut stream = conn.open_stream().await?;

// Stream data
stream.send(&data).await?;
let response = stream.receive::<MyType>().await?;
```

### Connection Metrics

```rust
let metrics = conn.metrics();
println!("RTT: {:?}", metrics.round_trip_time);
println!("Bytes sent: {}", metrics.bytes_sent);
```

### Custom Serialization

Phoenix uses `bincode` by default but supports any Serde-compatible format:

```rust
#[derive(Serialize, Deserialize)]
struct MyData {
    id: u64,
    message: String,
}

conn.send(&MyData {
    id: 1,
    message: "Hello".to_string(),
}).await?;
```

## Examples

The `examples/` directory contains complete applications:

- `chat_server.rs` / `chat_client.rs` - Real-time chat application
- `file_transfer.rs` - High-performance file transfer
- `microservice.rs` - Microservice communication
- `metrics_dashboard.rs` - Live metrics dashboard

Run examples with:

```bash
cargo run --example chat_server
cargo run --example chat_client
```

## Architecture

Phoenix SDK is built on top of:

- **STOQ Protocol** - Adaptive QUIC-based transport (1-40 Gbps)
- **TrustChain** - Certificate authority with consensus validation
- **Post-Quantum Crypto** - FALCON-1024 and Kyber for quantum resistance

## Performance

Benchmarked on standard cloud hardware:

- **Throughput**: Up to 40 Gbps
- **Latency**: <100 µs (local), <1 ms (regional)
- **Connections**: 100,000+ concurrent
- **CPU Usage**: <5% at 1 Gbps
- **Memory**: ~100 MB base + 1 KB per connection

## Documentation

- [API Reference](https://docs.phoenix.dev/api)
- [Performance Guide](https://docs.phoenix.dev/performance)
- [Security Guide](https://docs.phoenix.dev/security)
- [Deployment Guide](https://docs.phoenix.dev/deployment)

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Phoenix SDK is dual-licensed under MIT and Apache 2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) for details.

## Support

- Documentation: https://docs.phoenix.dev
- GitHub Issues: https://github.com/hypermesh-online/phoenix-sdk/issues
- Discord: https://discord.gg/phoenix-sdk

---

**Phoenix SDK**: *Distributed computing as simple as HTTP, performance of bare metal, security of a bank vault.*