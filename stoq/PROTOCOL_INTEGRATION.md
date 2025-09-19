# STOQ Protocol Integration Guide

This document explains the integration between STOQ transport layer and the protocol message handling system.

## Architecture Overview

The STOQ protocol integration consists of three main layers:

1. **Transport Layer** (`transport/mod.rs`) - QUIC over IPv6 transport
2. **Protocol Layer** (`protocol.rs`) - Structured message handling 
3. **Application Layer** (`server.rs`, `client.rs`) - High-level interfaces

## Integration Points

### Transport → Protocol Integration

The `StoqTransport` class now includes:

```rust
pub struct StoqTransport {
    // ... existing fields
    protocol_handler: Option<Arc<StoqProtocolHandler>>,
}
```

**Key Methods:**
- `set_protocol_handler()` - Wire protocol handler to transport
- `accept()` - Automatically starts protocol message handling for new connections

### Protocol Message Flow

1. **Connection Establishment**
   ```
   Client connects → Transport accepts → Protocol handler attached
   ```

2. **Message Processing**
   ```
   QUIC Stream → Protocol Parser → Message Router → Handler → Response
   ```

3. **Response Flow**
   ```
   Handler Response → Protocol Encoder → QUIC Stream → Client
   ```

## Usage Patterns

### Server-Side Integration

```rust
// Create server with integrated protocol handling
let server = StoqServer::new(config).await?;

// Register message handlers
server.register_handler("echo".to_string(), EchoHandler).await;
server.register_handler("json".to_string(), JsonHandler).await;

// Start server (protocol handling is automatic)
server.start().await?;
```

### Client-Side Integration

```rust
// Create client
let client = StoqClient::new(config).await?;

// Send structured messages
let response: String = client.send_message_with_response(
    &endpoint,
    "echo".to_string(),
    "Hello".to_string()
).await?;

// Access raw transport if needed
client.send_raw_data(&endpoint, &data).await?;
```

## Message Format

### Protocol Message Structure

```rust
struct StoqMessage<T> {
    header: MessageHeader,
    payload: T,
}

struct MessageHeader {
    message_type: String,     // Handler routing key
    version: u8,              // Protocol version
    message_id: u64,          // Request/response correlation
    content_length: u32,      // Payload size
    auth_token: Option<String>, // Authentication
    timestamp: u64,           // Creation time
    compression: Option<CompressionType>, // Optional compression
}
```

### Wire Format

```
[4 bytes: header_length][header_data][payload_data]
```

## Performance Optimizations

### Zero-Copy Operations
- Uses memory pools for buffer reuse
- Direct QUIC datagram sending for small messages
- Stream batching for large data transfers

### Connection Management
- Connection pooling and reuse
- Automatic protocol handler attachment
- Concurrent stream processing

### Hardware Acceleration
- Optional hardware acceleration integration
- Large send offload (LSO) support
- CPU affinity for network threads

## Certificate Authentication

The protocol layer integrates with STOQ's TLS certificate system:

```rust
// Certificate fingerprints automatically extracted from QUIC connections
struct ConnectionInfo {
    connection_id: String,
    remote_address: SocketAddr,
    cert_fingerprint: Option<String>, // From QUIC TLS handshake
    established_at: SystemTime,
}
```

## Error Handling

### Transport Errors
- Connection failures
- Network timeouts
- Certificate validation errors

### Protocol Errors  
- Message parsing failures
- Unknown message types
- Handler execution errors
- Timeout errors

### Recovery Strategies
- Automatic connection retry
- Message timeout handling
- Graceful degradation to raw transport

## Testing

Run the integration example:

```bash
cd stoq
cargo run --example integrated_echo_server
```

This demonstrates:
- Server setup with protocol handlers
- Client message sending/receiving  
- JSON and string message types
- Raw transport access
- Performance statistics

## Implementation Notes

### Thread Safety
- All components are `Send + Sync`
- Uses `Arc` for shared ownership
- `RwLock` and `Mutex` for thread-safe mutation

### Memory Management
- Memory pools for zero-copy operations
- Automatic buffer cleanup
- Connection lifecycle management

### Extensibility
- Generic message handler trait
- Type-safe message routing
- Pluggable authentication
- Configurable compression

## Future Enhancements

1. **Advanced Compression**
   - LZ4 and Gzip compression
   - Adaptive compression thresholds
   - Stream compression for large transfers

2. **Enhanced Authentication**
   - JWT token validation
   - Certificate-based authentication
   - Role-based access control

3. **Monitoring Integration**
   - Prometheus metrics export
   - Distributed tracing support
   - Real-time performance dashboards

4. **Load Balancing**
   - Multiple server endpoints
   - Health check integration
   - Automatic failover

## Migration Guide

### From Raw STOQ Transport

Before:
```rust
let transport = StoqTransport::new(config).await?;
let connection = transport.connect(&endpoint).await?;
transport.send(&connection, raw_data).await?;
```

After:
```rust
let client = StoqClient::new(config).await?;
client.send_message(&endpoint, "message_type".to_string(), structured_data).await?;
```

### From HTTP-based APIs

The protocol integration maintains the request/response pattern familiar from HTTP while providing the performance benefits of QUIC transport and structured message handling.

## Conclusion

The STOQ protocol integration provides a complete solution for high-performance, structured communication over QUIC while maintaining the flexibility to access the raw transport layer when needed. The integration is designed to be performant, type-safe, and easy to use for building distributed systems.