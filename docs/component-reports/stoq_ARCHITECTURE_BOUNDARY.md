# STOQ Architecture Boundary Definition

## Core Principle
STOQ is a **pure transport protocol** - it handles ONLY transport-layer concerns, similar to how TCP/IP operates in the network stack.

## What STOQ IS (Transport Layer)
- ✅ QUIC protocol implementation over IPv6
- ✅ Connection establishment and management
- ✅ Stream multiplexing and flow control
- ✅ Congestion control and loss recovery
- ✅ TLS 1.3 encryption and certificate handling
- ✅ Post-quantum cryptography (FALCON-1024)
- ✅ Packet framing and transmission
- ✅ Network tier detection and adaptation
- ✅ Transport-level metrics (bytes, packets, connections)
- ✅ Protocol extensions (tokenization, sharding, hop routing)

## What STOQ IS NOT (Application Layer)
- ❌ NO application SDKs or frameworks
- ❌ NO business logic or workflows
- ❌ NO monitoring dashboards or UIs
- ❌ NO performance analysis tools
- ❌ NO connection pooling strategies
- ❌ NO retry logic or circuit breakers
- ❌ NO request/response patterns
- ❌ NO serialization formats
- ❌ NO authentication/authorization
- ❌ NO session management

## Layer Comparison

### Network Stack Analogy
```
Application Layer    HTTP, FTP, SMTP      →  HyperMesh, Phoenix SDK
Transport Layer      TCP, UDP             →  STOQ Protocol
Network Layer        IP                   →  IPv6 (used by STOQ)
Link Layer          Ethernet, WiFi        →  OS Network Stack
```

### Practical Examples

#### ✅ CORRECT: Transport Layer (in STOQ)
```rust
// Pure transport - establishing connection
let connection = stoq.connect(endpoint).await?;

// Pure transport - sending raw bytes
connection.send(bytes).await?;

// Pure transport - network adaptation
transport.adapt_config_for_tier(measured_gbps);
```

#### ❌ INCORRECT: Application Layer (NOT in STOQ)
```rust
// Application logic - belongs in HyperMesh
let phoenix = PhoenixSDK::new(stoq);

// Monitoring dashboard - belongs in HyperMesh
let monitor = PerformanceMonitor::new(stoq);

// Connection pooling - belongs in application
let pool = ConnectionPool::new(stoq);
```

## Module Boundaries

### STOQ Modules (Transport Only)
```
stoq/
├── src/
│   ├── transport/       # QUIC implementation
│   │   ├── mod.rs      # Core transport with NetworkTier
│   │   ├── certificates.rs  # TLS certificates
│   │   ├── falcon.rs   # Post-quantum crypto
│   │   ├── streams.rs  # Stream handling
│   │   └── metrics.rs  # Transport metrics only
│   ├── config/         # Transport configuration
│   ├── extensions.rs   # Protocol extensions
│   └── lib.rs         # Public API (transport only)
```

### HyperMesh Modules (Application Layer)
```
hypermesh/
├── src/
│   ├── runtime/
│   │   └── phoenix/    # Phoenix SDK (uses STOQ)
│   ├── monitoring/     # Performance monitoring
│   │   ├── stoq_monitor.rs  # STOQ observer
│   │   └── performance.rs   # Analysis tools
│   └── ...            # Other application features
```

## API Surface

### STOQ Public API (Minimal, Transport-Focused)
```rust
// Core types
pub struct StoqTransport;
pub struct Connection;
pub struct Stream;
pub struct Endpoint;

// Configuration
pub struct TransportConfig;
pub struct StoqConfig;
pub enum NetworkTier;

// Crypto
pub struct FalconEngine;
pub enum FalconVariant;

// Extensions
pub struct PacketToken;
pub struct PacketShard;
```

### What Applications Build on Top
```rust
// In HyperMesh/Phoenix/etc
use stoq::{StoqTransport, Connection};

// Application adds:
- Connection pooling
- Request/response patterns
- Monitoring and observability
- Business logic
- User interfaces
```

## Testing Boundaries

### STOQ Tests (Transport Behavior)
- Connection establishment
- Data transmission
- Stream multiplexing
- Packet loss recovery
- Encryption/decryption
- Network adaptation

### HyperMesh Tests (Application Behavior)
- Phoenix SDK functionality
- Monitoring accuracy
- Dashboard rendering
- Business workflows
- Integration scenarios

## Dependency Rules

### STOQ Dependencies
- ✅ quinn (QUIC implementation)
- ✅ rustls (TLS)
- ✅ ring (cryptography)
- ✅ tokio (async runtime)
- ❌ NO application frameworks
- ❌ NO UI libraries
- ❌ NO business logic libraries

### HyperMesh Dependencies
- ✅ stoq (as transport layer)
- ✅ Any application framework
- ✅ UI/dashboard libraries
- ✅ Business logic libraries

## Migration Checklist

When considering adding code to STOQ, ask:

1. **Is this about moving bytes between endpoints?** → STOQ
2. **Is this about what those bytes mean?** → Application
3. **Would TCP need this feature?** → STOQ
4. **Is this specific to a use case?** → Application
5. **Does this require business context?** → Application

## Enforcement

This boundary is enforced through:
1. Code review requirements
2. Architectural documentation
3. Clear module separation
4. Minimal public API
5. Regular architecture audits

Remember: **STOQ is infrastructure, not application**