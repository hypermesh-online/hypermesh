# STOQ Monitoring - Built-in Protocol Monitoring

## Overview

STOQ now includes comprehensive built-in monitoring capabilities without any external dependencies like Prometheus. The monitoring system provides real-time metrics collection, protocol-level tracking, and health monitoring for the STOQ transport protocol.

## Features

### 1. **Native Metrics Collection**
- No external dependencies required
- Lightweight, lock-free atomic counters
- Minimal performance impact
- Thread-safe metrics recording

### 2. **Comprehensive Metrics**

#### Transport Metrics
- Bytes sent/received
- Active and total connections
- Throughput (Gbps)
- Latency (average, P50, P95, P99)

#### Protocol Metrics
- Packet tokenization
- Packet sharding/reassembly
- Hop routing
- Seed distribution

#### Performance Metrics
- Peak throughput tracking
- Zero-copy operations count
- Memory pool efficiency
- Frame batching statistics

#### Error Tracking
- Connection failures
- Packet drops
- Sharding/reassembly errors
- Token validation failures

### 3. **Monitoring API**

```rust
use stoq::{StoqMonitor, MonitoringAPI};

// Create monitor
let mut monitor = StoqMonitor::new(transport);

// Get current metrics
let metrics = monitor.get_metrics();

// Get health status
let health = monitor.get_health();

// Get dashboard summary
let summary = monitor.get_summary();

// Export as JSON
let json = monitor.export_json();
```

### 4. **Integration with Nexus UI**

The monitoring system is designed to integrate seamlessly with the Nexus UI dashboard:

```rust
// Expose metrics endpoint for Nexus
async fn metrics_endpoint(monitor: Arc<Mutex<StoqMonitor>>) -> Json<MetricsSnapshot> {
    let mut monitor = monitor.lock().await;
    Json(monitor.get_metrics())
}
```

## Architecture

### Metrics Collection Flow

```
Transport Operations → Atomic Counters → Metrics Aggregation → API/Export
                    ↓                  ↓                    ↓
                Protocol Events    Latency Tracking    Dashboard Display
```

### Key Components

1. **TransportMetrics** (`transport/metrics.rs`)
   - Core metrics collection with atomic operations
   - Protocol-specific event recording
   - Latency percentile tracking

2. **StoqMonitor** (`monitoring.rs`)
   - High-level monitoring interface
   - Historical metrics storage
   - Health status calculation
   - JSON export capabilities

3. **Protocol Integration** (`extensions.rs`)
   - Automatic metrics recording during protocol operations
   - Zero overhead when metrics not enabled
   - Comprehensive event tracking

## Performance Impact

The monitoring system is designed for minimal performance impact:

- **Atomic Operations**: Lock-free counters for basic metrics
- **Sampling**: Configurable sampling for latency tracking
- **Lazy Aggregation**: Metrics computed on-demand
- **Memory Bounded**: Fixed-size history buffers

## Usage Examples

### Basic Monitoring

```rust
// Initialize transport with monitoring
let transport = Arc::new(StoqTransport::new(config).await?);
let mut monitor = StoqMonitor::new(transport.clone());

// Periodic metrics collection
loop {
    sleep(Duration::from_secs(10)).await;

    let metrics = monitor.get_metrics();
    println!("Throughput: {:.2} Gbps", metrics.throughput_gbps);
    println!("Active connections: {}", metrics.active_connections);
}
```

### Health Monitoring

```rust
let health = monitor.get_health();

match health.level {
    HealthLevel::Healthy => println!("System healthy"),
    HealthLevel::Warning => {
        println!("Warning: {}", health.issues.join(", "));
    }
    HealthLevel::Critical => {
        println!("CRITICAL: {}", health.issues.join(", "));
        // Trigger alerts
    }
}
```

### Protocol Metrics

```rust
let protocol_metrics = transport.get_protocol_metrics();

println!("Tokenization rate: {} packets/sec",
         protocol_metrics.packets_tokenized as f64 / elapsed);
println!("Sharding efficiency: {:.1}%",
         protocol_metrics.shards_reassembled as f64 /
         protocol_metrics.packets_sharded as f64 * 100.0);
```

## Testing

Run the monitoring tests:

```bash
cargo test monitoring_test
cargo run --example monitoring_demo
```

## Future Enhancements

1. **Grafana Integration**: Export Prometheus-compatible metrics format
2. **Alerting**: Built-in threshold-based alerting
3. **Distributed Tracing**: Correlation IDs for request tracking
4. **Custom Metrics**: User-defined metric collection
5. **Performance Profiling**: CPU/memory usage tracking

## Configuration

Monitoring can be configured through the `StoqMonitor` constructor:

```rust
let monitor = StoqMonitor::with_config(
    transport,
    Duration::from_secs(5),  // Collection interval
    1000,                     // Max history size
);
```

## API Reference

See the generated documentation:

```bash
cargo doc --open --no-deps -p stoq
```

## Conclusion

STOQ's built-in monitoring provides comprehensive protocol-level observability without external dependencies, ensuring high performance while maintaining full visibility into transport operations.