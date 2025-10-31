# STOQ API Migration Guide
**Date**: 2025-10-25
**Purpose**: Replace HTTP REST APIs with STOQ protocol communication
**Status**: 🚧 IN PROGRESS

---

## Executive Summary

This guide documents the migration from HTTP-based APIs to STOQ protocol for all inter-component communication in the Web3 ecosystem. STOQ (pure QUIC over IPv6) provides native transport with better performance, security, and architectural alignment.

---

## Architecture Change

### Before (HTTP)
```
Component A
    ↓ (HTTP/1.1 REST API)
axum/warp server on TCP
    ↓
Network stack
    ↓ (HTTP client - reqwest)
Component B
```

### After (STOQ)
```
Component A
    ↓ (STOQ API - RPC over QUIC)
StoqApiServer
    ↓ (quinn QUIC transport)
Network stack (UDP/IPv6)
    ↓ (StoqApiClient)
Component B
```

---

## STOQ API Framework

### Core Types (stoq/src/api/mod.rs)

**ApiRequest**:
```rust
pub struct ApiRequest {
    pub id: String,              // Request correlation ID
    pub service: String,         // Target service name
    pub method: String,          // Method/endpoint path
    pub payload: Bytes,          // JSON-serialized payload
    pub metadata: HashMap<String, String>,
}
```

**ApiResponse**:
```rust
pub struct ApiResponse {
    pub request_id: String,      // Correlates to request
    pub success: bool,           // Success flag
    pub payload: Bytes,          // JSON-serialized result
    pub error: Option<String>,   // Error message if failed
    pub metadata: HashMap<String, String>,
}
```

**ApiHandler Trait**:
```rust
#[async_trait]
pub trait ApiHandler: Send + Sync {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError>;
    fn path(&self) -> &str;  // Handler registration path
}
```

### Server (StoqApiServer)

**Creation**:
```rust
let transport = Arc::new(StoqTransport::new(config).await?);
let server = Arc::new(StoqApiServer::new(transport));
```

**Register Handlers**:
```rust
server.register_handler(Arc::new(MyHandler::new()));
```

**Start Listening**:
```rust
server.listen().await?;
```

### Client (StoqApiClient)

**Creation**:
```rust
let transport = Arc::new(StoqTransport::new(config).await?);
let client = Arc::new(StoqApiClient::new(transport));
```

**Make API Call**:
```rust
let response: MyResponse = client
    .call("service_name", "method_path", &request_payload)
    .await?;
```

---

## Migration Steps

### Step 1: Implement Handler

Replace HTTP endpoint handler with `ApiHandler` implementation:

**Before (HTTP with axum)**:
```rust
async fn validate_certificate(
    Json(payload): Json<CertificateRequest>,
) -> Result<Json<CertificateResponse>, StatusCode> {
    // Handler logic
}

let app = Router::new()
    .route("/validate", post(validate_certificate));
```

**After (STOQ)**:
```rust
pub struct CertificateValidationHandler {
    service: Arc<ValidationService>,
}

#[async_trait]
impl ApiHandler for CertificateValidationHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        // Deserialize request
        let cert_req: CertificateRequest = serde_json::from_slice(&request.payload)
            .map_err(|e| ApiError::InvalidRequest(e.to_string()))?;

        // Call service
        let result = self.service.validate(cert_req).await
            .map_err(|e| ApiError::HandlerError(e.to_string()))?;

        // Serialize response
        let payload = serde_json::to_vec(&result)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "trustchain/validate_certificate"
    }
}
```

### Step 2: Replace Server Startup

**Before (axum)**:
```rust
let app = Router::new()
    .route("/api/consensus", post(validate_consensus))
    .route("/health", get(health_check));

axum::Server::bind(&addr)
    .serve(app.into_make_service())
    .await?;
```

**After (STOQ)**:
```rust
// Create STOQ transport
let transport_config = TransportConfig {
    bind_address: "[::1]".parse()?,
    port: 9292,
    ..Default::default()
};
let transport = Arc::new(StoqTransport::new(transport_config).await?);

// Create API server
let server = Arc::new(StoqApiServer::new(transport));

// Register handlers
server.register_handler(Arc::new(ConsensusHandler::new(service)));
server.register_handler(Arc::new(HealthHandler));

// Start listening
server.listen().await?;
```

### Step 3: Replace Client Calls

**Before (reqwest)**:
```rust
let client = reqwest::Client::new();
let response = client
    .post("http://trustchain:8080/api/validate")
    .json(&request)
    .send()
    .await?;

let result: ValidationResult = response.json().await?;
```

**After (STOQ)**:
```rust
let transport = Arc::new(StoqTransport::new(config).await?);
let client = Arc::new(StoqApiClient::new(transport));

let result: ValidationResult = client
    .call("trustchain", "validate_certificate", &request)
    .await?;
```

---

## Component Migration Status

### ✅ STOQ Protocol Core
- **Location**: `stoq/src/api/mod.rs`
- **Status**: COMPLETE
- **Features**:
  - StoqApiServer - Accept connections, route to handlers
  - StoqApiClient - Make requests to remote services
  - ApiHandler trait - Handler registration pattern
  - ApiRequest/ApiResponse - Message types
  - Service discovery placeholder (TODO: integrate TrustChain DNS)

### ✅ HyperMesh Consensus API
- **Location**: `hypermesh/src/consensus/stoq_api.rs`
- **Status**: COMPLETE
- **Handlers**:
  - `CertificateValidationHandler` - Validates certificates via consensus
  - `FourProofValidationHandler` - Four-proof validation
  - `HealthCheckHandler` - Health status
- **Replaces**: `hypermesh/src/consensus/api_server.rs` (HTTP warp server)

### ✅ HyperMesh Integration Bridge
- **Location**: `hypermesh/src/integration/stoq_bridge.rs`
- **Status**: COMPLETE
- **Features**:
  - UnifiedStoqBridge - Single bridge for all components
  - Server + Client in one interface
  - Handler registration
  - Service call wrapper
- **Replaces**: `hypermesh/src/integration/api_bridge.rs` (HTTP axum bridge)

### 🚧 TrustChain API (TODO)
- **Location**: `trustchain/src/api/` (needs creation)
- **Status**: NOT STARTED
- **Required Handlers**:
  - Certificate validation
  - Certificate issuance
  - DNS resolution
  - Health check
- **Replaces**:
  - `trustchain/src/bin/simple-server.rs` (HTTP axum)
  - `trustchain/src/bin/standalone-server.rs` (HTTP axum)
  - `trustchain/src/bin/trustchain-server.rs` (HTTP axum)
  - `trustchain/src/api/mod.rs` (HTTP)

### 🚧 Caesar API (TODO)
- **Location**: `caesar/src/api/` (needs creation)
- **Status**: NOT STARTED
- **Required Handlers**:
  - Transaction submission
  - Wallet balance query
  - Economic incentive calculation
- **Replaces**: `caesar/src/lib.rs` (HTTP Router)

### 🚧 HyperMesh Main Server (TODO)
- **Location**: `hypermesh/src/main.rs`
- **Status**: NOT STARTED
- **Changes**: Replace axum Router with STOQ bridge initialization

---

## Service Discovery

### Current (Hardcoded)
```rust
// StoqApiClient::resolve_service()
match service {
    "trustchain" => Endpoint { address: "[::1]:9293".parse()?, ... },
    "hypermesh" => Endpoint { address: "[::1]:9292".parse()?, ... },
    "caesar" => Endpoint { address: "[::1]:9294".parse()?, ... },
    _ => Err(anyhow!("Unknown service")),
}
```

### Future (TrustChain DNS)
```rust
async fn resolve_service(&self, service: &str) -> Result<Endpoint> {
    // Query TrustChain DNS for service SRV record
    let dns_query = format!("_stoq._udp.{}.hypermesh", service);
    let srv_record = trustchain_dns::resolve_srv(&dns_query).await?;

    Ok(Endpoint {
        address: srv_record.target,
        server_name: service.to_string(),
    })
}
```

---

## Testing

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handler() {
        let handler = MyHandler::new();

        let request = ApiRequest {
            id: "test-1".to_string(),
            service: "test".to_string(),
            method: "test_method".to_string(),
            payload: serde_json::to_vec(&TestPayload { ... })?.into(),
            metadata: HashMap::new(),
        };

        let response = handler.handle(request).await.unwrap();
        assert!(response.success);
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_client_server() {
    // Start server
    let server_config = TransportConfig {
        bind_address: "[::1]".parse()?,
        port: 9999,
        ..Default::default()
    };
    let server_transport = Arc::new(StoqTransport::new(server_config).await?);
    let server = Arc::new(StoqApiServer::new(server_transport));
    server.register_handler(Arc::new(TestHandler));

    tokio::spawn(async move {
        server.listen().await.unwrap();
    });

    // Client request
    let client_config = TransportConfig::default();
    let client_transport = Arc::new(StoqTransport::new(client_config).await?);
    let client = Arc::new(StoqApiClient::new(client_transport));

    let result: TestResponse = client
        .call("test", "test_method", &TestRequest { ... })
        .await?;

    assert_eq!(result.status, "ok");
}
```

---

## Performance Comparison

### HTTP/1.1 (axum/warp)
- **Connection Setup**: TCP handshake (1-2 RTT) + TLS handshake (1-2 RTT) = 2-4 RTT
- **Head-of-Line Blocking**: Yes (TCP)
- **Multiplexing**: Limited (HTTP/1.1 persistent connections)
- **0-RTT Resumption**: No
- **Connection Migration**: No
- **Protocol Overhead**: HTTP headers (text-based)

### STOQ (QUIC/IPv6)
- **Connection Setup**: 0-RTT or 1-RTT (with resumption)
- **Head-of-Line Blocking**: No (stream-level independence)
- **Multiplexing**: Full (unlimited concurrent streams)
- **0-RTT Resumption**: Yes (configurable)
- **Connection Migration**: Yes (IP/port changes)
- **Protocol Overhead**: Binary protocol, minimal overhead

**Expected Improvement**: 2-4x lower latency, 10-20% higher throughput

---

## Security Comparison

### HTTP/1.1 + TLS
- **Transport Security**: TLS 1.2/1.3
- **Certificate Validation**: Standard X.509 PKI
- **Quantum Resistance**: No

### STOQ
- **Transport Security**: QUIC with TLS 1.3 built-in
- **Certificate Validation**: TrustChain federated CA
- **Quantum Resistance**: FALCON-1024 post-quantum signatures (optional)
- **Perfect Forward Secrecy**: Yes
- **Replay Protection**: Built into QUIC

---

## Migration Checklist

### For Each Component:

- [ ] Create `stoq_api.rs` module
- [ ] Implement `ApiHandler` for each endpoint
- [ ] Create STOQ server initialization
- [ ] Register all handlers
- [ ] Replace HTTP client calls with STOQ client
- [ ] Update configuration (remove HTTP ports, add STOQ address)
- [ ] Remove HTTP dependencies from `Cargo.toml`
- [ ] Comment out old HTTP modules
- [ ] Add integration tests
- [ ] Update documentation

### HyperMesh:
- [x] Consensus API (`consensus/stoq_api.rs`)
- [x] Integration bridge (`integration/stoq_bridge.rs`)
- [ ] Main server (`main.rs`)
- [ ] API module (`api/mod.rs`)
- [ ] Integration tests

### TrustChain:
- [ ] Certificate API
- [ ] DNS API
- [ ] Simple server replacement
- [ ] Standalone server replacement
- [ ] Integration tests

### Caesar:
- [ ] Transaction API
- [ ] Wallet API
- [ ] Economic API
- [ ] Integration tests

---

## Common Pitfalls

### 1. Bidirectional Streams
**Problem**: QUIC streams are unidirectional by default
**Solution**: Use `connection.open_bi()` for request/response pattern

### 2. Service Discovery
**Problem**: Hardcoded addresses don't scale
**Solution**: Integrate TrustChain DNS resolution (future work)

### 3. Serialization Format
**Problem**: Mixing JSON and bincode
**Solution**: Use bincode for ApiRequest/ApiResponse envelope, JSON for payloads

### 4. Error Handling
**Problem**: Losing error context across STOQ boundary
**Solution**: Use ApiError enum with detailed error messages

### 5. Connection Pooling
**Problem**: Creating new connection per request
**Solution**: StoqApiClient caches connections by service name

---

## Debugging

### Enable Tracing
```rust
tracing_subscriber::fmt()
    .with_env_filter("stoq=debug,hypermesh=debug")
    .init();
```

### Common Issues

**"Handler not found"**:
- Check handler registration path matches client method call
- Verify handler was registered before server started

**"Serialization error"**:
- Ensure request/response types implement Serialize/Deserialize
- Check payload types match between client and server

**"Transport error: Connection refused"**:
- Verify server is listening on correct IPv6 address/port
- Check firewall allows UDP traffic
- Ensure service discovery resolves to correct address

---

## Next Steps

1. **Complete HyperMesh Migration** - Main server and API module
2. **Implement TrustChain STOQ API** - All certificate/DNS services
3. **Implement Caesar STOQ API** - Economic transaction services
4. **Service Discovery Integration** - Replace hardcoded addresses with TrustChain DNS
5. **Performance Testing** - Compare HTTP vs STOQ latency/throughput
6. **Production Deployment** - Roll out to staging environment

---

## Resources

- STOQ Protocol: `/home/persist/repos/projects/web3/stoq/`
- STOQ API: `/home/persist/repos/projects/web3/stoq/src/api/mod.rs`
- HyperMesh Consensus STOQ: `/home/persist/repos/projects/web3/hypermesh/src/consensus/stoq_api.rs`
- Integration Bridge: `/home/persist/repos/projects/web3/hypermesh/src/integration/stoq_bridge.rs`
- HTTP Removal Report: `/home/persist/repos/projects/web3/HTTP_REMOVED.md`

---

**Migration Status**: Framework complete, component migration in progress
**Target Completion**: Migration framework ready for rollout
