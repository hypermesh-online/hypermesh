# Phoenix SDK API Reference

Complete API documentation for Phoenix SDK with all public types, methods, and traits.

## Core Modules

### `phoenix` Module
The main Phoenix SDK module containing all public APIs.

```rust
use phoenix_sdk::phoenix::{
    PhoenixTransport,
    PhoenixConnection,
    PhoenixConfig,
    PhoenixBuilder,
    PerformanceMetrics,
};
```

## Primary Types

### `PhoenixTransport`
The main transport interface for Phoenix SDK applications.

```rust
pub struct PhoenixTransport {
    // Internal implementation
}
```

**Key Methods**:
- [`new()`](#phoenixtransportnew) - Create with default configuration
- [`with_config()`](#phoenixtransportwith_config) - Create with custom configuration
- [`connect()`](#phoenixtransportconnect) - Connect to a remote endpoint
- [`accept()`](#phoenixtransportaccept) - Accept incoming connections
- [`stats()`](#phoenixtransportstats) - Get performance statistics
- [`enable_multiplexing()`](#phoenixtransportenable_multiplexing) - Enable connection multiplexing
- [`send_multiplexed()`](#phoenixtransportsend_multiplexed) - Send using multiplexed connections
- [`shutdown()`](#phoenixtransportshutdown) - Gracefully shutdown transport

### `PhoenixConnection`
Represents an active connection to a remote peer.

```rust
pub struct PhoenixConnection {
    // Internal implementation
}
```

**Key Methods**:
- [`send_data()`](#phoenixconnectionsend_data) - Send byte data
- [`send_bytes()`](#phoenixconnectionsend_bytes) - Send Bytes (zero-copy when possible)
- [`receive_data()`](#phoenixconnectionreceive_data) - Receive data
- [`stream_data()`](#phoenixconnectionstream_data) - Stream data continuously
- [`metrics()`](#phoenixconnectionmetrics) - Get connection metrics
- [`is_active()`](#phoenixconnectionis_active) - Check if connection is active
- [`endpoint()`](#phoenixconnectionendpoint) - Get endpoint address
- [`close()`](#phoenixconnectionclose) - Close the connection

### `PhoenixConfig`
Configuration structure for Phoenix SDK.

```rust
#[derive(Debug, Clone)]
pub struct PhoenixConfig {
    pub app_id: String,
    pub bind_address: Ipv6Addr,
    pub port: u16,
    pub high_performance: bool,
    pub max_connections: usize,
    pub auto_certificates: bool,
}
```

**Fields**:
- `app_id` - Application identifier for logging/metrics
- `bind_address` - IPv6 address to bind to (default: `::`)
- `port` - Port number (0 for dynamic)
- `high_performance` - Enable performance optimizations
- `max_connections` - Maximum concurrent connections
- `auto_certificates` - Automatic certificate provisioning

### `PhoenixBuilder`
Builder pattern for creating configured Phoenix transports.

```rust
pub struct PhoenixBuilder {
    // Internal configuration
}
```

**Methods**:
- [`new()`](#phoenixbuildernew) - Create new builder
- [`bind_address()`](#phoenixbuilderbind_address) - Set bind address
- [`port()`](#phoenixbuilderport) - Set port
- [`high_performance()`](#phoenixbuilderhigh_performance) - Enable/disable high performance
- [`max_connections()`](#phoenixbuildermax_connections) - Set connection limit
- [`auto_certificates()`](#phoenixbuilderauto_certificates) - Enable/disable auto certificates
- [`build()`](#phoenixbuilderbuild) - Build the PhoenixTransport

### `PerformanceMetrics`
Performance statistics for Phoenix SDK.

```rust
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub throughput_gbps: f64,
    pub latency_ms: f64,
    pub active_connections: usize,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub zero_copy_operations: u64,
}
```

## Method Documentation

### PhoenixTransport Methods

#### `PhoenixTransport::new()`
Create a new Phoenix transport with default configuration.

```rust
pub async fn new(app_id: impl Into<String>) -> Result<Self>
```

**Parameters**:
- `app_id` - Application identifier

**Returns**: `Result<PhoenixTransport>`

**Example**:
```rust
let phoenix = PhoenixTransport::new("my-app").await?;
```

**Performance**: Initialization takes <100ms including certificate generation.

#### `PhoenixTransport::with_config()`
Create Phoenix transport with custom configuration.

```rust
pub async fn with_config(config: PhoenixConfig) -> Result<Self>
```

**Parameters**:
- `config` - Custom configuration

**Returns**: `Result<PhoenixTransport>`

**Example**:
```rust
let config = PhoenixConfig {
    app_id: "custom-app".to_string(),
    port: 8080,
    high_performance: true,
    ..Default::default()
};
let phoenix = PhoenixTransport::with_config(config).await?;
```

#### `PhoenixTransport::connect()`
Connect to a remote Phoenix endpoint.

```rust
pub async fn connect(&self, endpoint: &str) -> Result<PhoenixConnection>
```

**Parameters**:
- `endpoint` - Remote endpoint (format: `"host:port"` or `"[ipv6]:port"`)

**Returns**: `Result<PhoenixConnection>`

**Connection Pooling**: Automatically reuses existing connections when available.

**Example**:
```rust
let conn = phoenix.connect("[::1]:9292").await?;
let conn2 = phoenix.connect("example.com:8080").await?;
```

**Performance**:
- New connection: 5-10ms
- Pooled connection: <1ms

#### `PhoenixTransport::accept()`
Accept an incoming connection.

```rust
pub async fn accept(&self) -> Result<PhoenixConnection>
```

**Returns**: `Result<PhoenixConnection>`

**Blocks**: Until a connection is available

**Example**:
```rust
loop {
    let conn = phoenix.accept().await?;
    tokio::spawn(handle_connection(conn));
}
```

#### `PhoenixTransport::stats()`
Get current performance statistics.

```rust
pub async fn stats(&self) -> PerformanceMetrics
```

**Returns**: Current performance metrics

**Example**:
```rust
let metrics = phoenix.stats().await;
println!("Throughput: {:.2} Gbps", metrics.throughput_gbps);
```

#### `PhoenixTransport::enable_multiplexing()`
Enable connection multiplexing for improved throughput.

```rust
pub async fn enable_multiplexing(&self, endpoint: &str, connections: usize) -> Result<()>
```

**Parameters**:
- `endpoint` - Target endpoint
- `connections` - Number of parallel connections

**Example**:
```rust
phoenix.enable_multiplexing("server:8080", 10).await?;
```

#### `PhoenixTransport::send_multiplexed()`
Send data using multiplexed connections.

```rust
pub async fn send_multiplexed(&self, endpoint: &str, data: &[u8]) -> Result<()>
```

**Parameters**:
- `endpoint` - Target endpoint
- `data` - Data to send

**Requires**: `enable_multiplexing()` called first

**Example**:
```rust
phoenix.send_multiplexed("server:8080", &large_data).await?;
```

#### `PhoenixTransport::shutdown()`
Gracefully shutdown the transport.

```rust
pub async fn shutdown(&self)
```

**Effect**: Closes all connections and releases resources

**Example**:
```rust
phoenix.shutdown().await;
```

### PhoenixConnection Methods

#### `PhoenixConnection::send_data()`
Send data over the connection.

```rust
pub async fn send_data(&mut self, data: &[u8]) -> Result<()>
```

**Parameters**:
- `data` - Byte data to send

**Example**:
```rust
conn.send_data(b"Hello, World!").await?;
```

#### `PhoenixConnection::send_bytes()`
Send Bytes directly (enables zero-copy when possible).

```rust
pub async fn send_bytes(&mut self, data: Bytes) -> Result<()>
```

**Parameters**:
- `data` - Bytes to send

**Zero-Copy**: Attempted when platform supports it

**Example**:
```rust
use bytes::Bytes;
let data = Bytes::from_static(b"Static data");
conn.send_bytes(data).await?;
```

#### `PhoenixConnection::receive_data()`
Receive data from the connection.

```rust
pub async fn receive_data(&mut self) -> Result<Bytes>
```

**Returns**: Received data as Bytes

**Blocks**: Until data is available

**Example**:
```rust
let data = conn.receive_data().await?;
println!("Received {} bytes", data.len());
```

#### `PhoenixConnection::stream_data()`
Stream data continuously using a closure.

```rust
pub async fn stream_data<F>(&mut self, data_source: F) -> Result<()>
where
    F: FnMut() -> Option<Vec<u8>>
```

**Parameters**:
- `data_source` - Closure that provides data chunks

**Example**:
```rust
let mut counter = 0;
conn.stream_data(|| {
    counter += 1;
    if counter <= 10 {
        Some(vec![counter; 1024])
    } else {
        None
    }
}).await?;
```

#### `PhoenixConnection::metrics()`
Get connection-specific metrics.

```rust
pub async fn metrics(&self) -> ConnectionMetrics
```

**Returns**: Metrics for this connection

**Example**:
```rust
let metrics = conn.metrics().await;
println!("Sent: {} bytes", metrics.bytes_sent);
```

#### `PhoenixConnection::is_active()`
Check if the connection is still active.

```rust
pub fn is_active(&self) -> bool
```

**Returns**: `true` if connection is active

**Example**:
```rust
if conn.is_active() {
    conn.send_data(b"ping").await?;
}
```

#### `PhoenixConnection::endpoint()`
Get the remote endpoint address.

```rust
pub fn endpoint(&self) -> &str
```

**Returns**: Endpoint string

**Example**:
```rust
println!("Connected to: {}", conn.endpoint());
```

#### `PhoenixConnection::close()`
Close the connection.

```rust
pub fn close(&self)
```

**Effect**: Immediately closes the connection

**Example**:
```rust
conn.close();
```

### PhoenixBuilder Methods

#### `PhoenixBuilder::new()`
Create a new builder instance.

```rust
pub fn new(app_id: impl Into<String>) -> Self
```

**Parameters**:
- `app_id` - Application identifier

**Example**:
```rust
let builder = PhoenixBuilder::new("my-app");
```

#### `PhoenixBuilder::bind_address()`
Set the bind address.

```rust
pub fn bind_address(mut self, addr: Ipv6Addr) -> Self
```

**Parameters**:
- `addr` - IPv6 address to bind to

**Example**:
```rust
builder.bind_address(Ipv6Addr::LOCALHOST)
```

#### `PhoenixBuilder::port()`
Set the port number.

```rust
pub fn port(mut self, port: u16) -> Self
```

**Parameters**:
- `port` - Port number (0 for dynamic)

**Example**:
```rust
builder.port(8080)
```

#### `PhoenixBuilder::high_performance()`
Enable or disable high-performance mode.

```rust
pub fn high_performance(mut self, enabled: bool) -> Self
```

**Parameters**:
- `enabled` - Enable performance optimizations

**Effect**: Enables zero-copy, memory pooling, larger buffers

**Example**:
```rust
builder.high_performance(true)
```

#### `PhoenixBuilder::max_connections()`
Set maximum concurrent connections.

```rust
pub fn max_connections(mut self, max: usize) -> Self
```

**Parameters**:
- `max` - Maximum connection count

**Example**:
```rust
builder.max_connections(100)
```

#### `PhoenixBuilder::auto_certificates()`
Enable or disable automatic certificate management.

```rust
pub fn auto_certificates(mut self, enabled: bool) -> Self
```

**Parameters**:
- `enabled` - Enable auto certificates

**Example**:
```rust
builder.auto_certificates(true)
```

#### `PhoenixBuilder::build()`
Build the Phoenix transport instance.

```rust
pub async fn build(self) -> Result<PhoenixTransport>
```

**Returns**: `Result<PhoenixTransport>`

**Example**:
```rust
let phoenix = PhoenixBuilder::new("app")
    .port(8080)
    .high_performance(true)
    .build()
    .await?;
```

## Error Types

Phoenix SDK uses `anyhow::Error` for error handling with context messages.

Common error scenarios:
- `ConnectionTimeout` - Server unreachable
- `CertificateValidation` - Certificate trust failure
- `NetworkUnreachable` - Network configuration issue
- `InvalidEndpoint` - Malformed endpoint string
- `ConnectionClosed` - Connection terminated

## Thread Safety

All Phoenix SDK types are thread-safe:
- `PhoenixTransport`: `Send + Sync + Clone`
- `PhoenixConnection`: `Send + Sync`
- `PhoenixConfig`: `Send + Sync + Clone`
- `PerformanceMetrics`: `Send + Sync + Clone`

## Memory Management

Phoenix SDK manages memory efficiently:
- Connection pooling reduces allocations
- Memory pools for frequent allocations
- Zero-copy operations when possible
- Automatic cleanup on drop

## Platform Support

- **Linux**: Full support, best performance
- **macOS**: Full support, good performance
- **Windows**: Full support, moderate performance
- **WebAssembly**: Planned (not yet implemented)

## Version Compatibility

Phoenix SDK follows semantic versioning:
- **1.0.x**: Current stable API
- **0.x**: Pre-release versions (breaking changes possible)