# HyperMesh Consensus Server Requirements

**Document Version**: 1.0
**Date**: 2025-10-30
**Author**: Data Analyst Agent
**Sprint**: Sprint 1, Step 1 (Discovery & Ideation)

---

## Executive Summary

This document specifies the requirements for the HyperMesh consensus server that TrustChain and other external services depend on for four-proof consensus validation. Based on analysis of TrustChain client code (`trustchain/src/consensus/hypermesh_client.rs`) and existing HyperMesh consensus validation service (`hypermesh/src/consensus/validation_service.rs`), this document defines the exact API contract, validation logic, and implementation scope.

**Key Finding**: HyperMesh already has a comprehensive `ConsensusValidationService` implementation with all required structures and conversion logic. The missing piece is the **STOQ API server wrapper** that exposes these validation endpoints over STOQ protocol.

---

## 1. API Contract Specification

TrustChain expects HyperMesh to expose consensus validation via **STOQ API protocol** (QUIC-based RPC), not HTTP.

### 1.1 Primary Endpoints

#### Endpoint 1: `consensus/validate_certificate`

**Purpose**: Validate certificate issuance requests from TrustChain CA

**Request Structure**: `ConsensusValidationRequest` (from TrustChain)
```rust
// File: trustchain/src/consensus/hypermesh_client.rs:77-90
pub struct ConsensusValidationRequest {
    pub certificate_request: CertificateRequest,
    pub consensus_requirements: ConsensusRequirements,
    pub request_id: String,
    pub timestamp: SystemTime,
    pub validation_context: ValidationContext,
}

// File: trustchain/src/ca/mod.rs:129-143
pub struct CertificateRequest {
    pub common_name: String,
    pub san_entries: Vec<String>,
    pub node_id: String,
    pub ipv6_addresses: Vec<std::net::Ipv6Addr>,
    pub consensus_proof: ConsensusProof,
    pub timestamp: SystemTime,
}

// File: trustchain/src/consensus/mod.rs:154-179
pub struct ConsensusRequirements {
    pub minimum_stake: u64,
    pub max_time_offset: Duration,
    pub minimum_storage: u64,
    pub minimum_compute: u64,
    pub byzantine_tolerance: f64,
}

// File: trustchain/src/consensus/hypermesh_client.rs:92-103
pub struct ValidationContext {
    pub ca_id: String,
    pub network_id: String,
    pub certificate_type: CertificateType,
    pub metadata: HashMap<String, String>,
}
```

**Response Structure**: `ConsensusValidationResult`
```rust
// File: trustchain/src/consensus/hypermesh_client.rs:175-190
pub struct ConsensusValidationResult {
    pub result: ConsensusValidationStatus,
    pub proof_hash: Option<[u8; 32]>,
    pub validator_id: String,
    pub validated_at: SystemTime,
    pub metrics: ValidationMetrics,
    pub details: ValidationDetails,
}

// File: trustchain/src/consensus/hypermesh_client.rs:192-202
pub enum ConsensusValidationStatus {
    Valid,
    Invalid { failed_proofs: Vec<String>, reason: String },
    Pending { estimated_completion: SystemTime },
    Error { error_code: String, message: String },
}

// File: trustchain/src/consensus/hypermesh_client.rs:204-214
pub struct ValidationMetrics {
    pub validation_time_us: u64,
    pub validator_nodes: u32,
    pub confidence_level: f64,
    pub network_load: f32,
}

// File: trustchain/src/consensus/hypermesh_client.rs:216-224
pub struct ValidationDetails {
    pub proof_results: ProofValidationResults,
    pub bft_status: ByzantineFaultToleranceStatus,
    pub performance_stats: PerformanceStatistics,
}
```

**TrustChain Client Call**:
```rust
// File: trustchain/src/consensus/hypermesh_client.rs:414-419
let result: ConsensusValidationResult = self.stoq_client
    .call("hypermesh", "consensus/validate_certificate", request)
    .await
    .map_err(|e| anyhow!("STOQ API error sending validation request: {}", e))?;
```

---

#### Endpoint 2: `consensus/validate_proofs`

**Purpose**: Validate four-proof sets for complex operations beyond certificate issuance

**Request Structure**: `FourProofValidationRequest`
```rust
// File: trustchain/src/consensus/hypermesh_client.rs:119-132
pub struct FourProofValidationRequest {
    pub proof_set: FourProofSet,
    pub operation: String,
    pub asset_id: String,
    pub node_id: String,
    pub timestamp: SystemTime,
}

// File: trustchain/src/consensus/hypermesh_client.rs:134-145
pub struct FourProofSet {
    pub space_proof: SpaceProofData,
    pub stake_proof: StakeProofData,
    pub work_proof: WorkProofData,
    pub time_proof: TimeProofData,
}
```

**Response Structure**: Same `ConsensusValidationResult` as Endpoint 1

**TrustChain Client Call**:
```rust
// File: trustchain/src/consensus/hypermesh_client.rs:428-433
let result: ConsensusValidationResult = self.stoq_client
    .call("hypermesh", "consensus/validate_proofs", &request)
    .await
    .map_err(|e| anyhow!("STOQ API error sending four-proof validation: {}", e))?;
```

---

#### Endpoint 3: `consensus/validation_status`

**Purpose**: Check status of pending validation requests

**Request Structure**: Simple status query
```rust
// File: trustchain/src/consensus/hypermesh_client.rs:355-362
struct StatusRequest {
    request_id: String,
}
```

**Response Structure**: Same `ConsensusValidationResult` (may have `Pending` status)

**TrustChain Client Call**:
```rust
// File: trustchain/src/consensus/hypermesh_client.rs:364-369
let result: ConsensusValidationResult = self.stoq_client
    .call("hypermesh", "consensus/validation_status", &request)
    .await
    .map_err(|e| anyhow!("STOQ API error checking validation status: {}", e))?;
```

---

## 2. Four-Proof Validation Requirements

### 2.1 Four-Proof Set Components

Every consensus validation requires **ALL FOUR** proofs to answer WHERE/WHO/WHAT/WHEN:

1. **SpaceProof (PoSp)** - WHERE
   - Storage location (`storage_commitment`, `network_position`)
   - Physical/network allocation proof

2. **StakeProof (PoSt)** - WHO
   - Ownership/authority (`stake_amount`, `authority_level`)
   - Access permissions

3. **WorkProof (PoWk)** - WHAT/HOW
   - Computational resources (`computational_proof`, `difficulty_target`)
   - Operation signature

4. **TimeProof (PoTm)** - WHEN
   - Temporal ordering (`block_timestamp`, `sequence_number`)
   - Replay attack prevention

### 2.2 Proof Validation Logic

**Existing Implementation**: `hypermesh/src/consensus/validation_service.rs` already implements:

- **Proof Conversion** (lines 511-613): Converts TrustChain/external proof formats to HyperMesh internal format
- **Consensus Validation** (lines 615-651): Validates proofs through HyperMesh consensus system
- **Byzantine Detection** (lines 638-647): Checks for Byzantine node behavior
- **Result Creation** (lines 653-701): Creates standardized validation results

**Key Validation Steps** (from `validation_service.rs:358-389`):
```rust
// 1. Convert TrustChain proof to HyperMesh format
let hypermesh_proof = self.convert_trustchain_proof(&request.certificate_request.consensus_proof).await?;

// 2. Validate through HyperMesh consensus system
let validation_result = self.validate_consensus_proof(&hypermesh_proof, &node_id, &operation).await?;

// 3. Check Byzantine behavior
let is_byzantine = consensus.is_node_byzantine(&node_id_parsed).await?;

// 4. Create validation result with metrics
let result = self.create_validation_result(validation_result, start_time).await;
```

---

## 3. MVP vs Full Implementation Scope

### 3.1 MVP Scope (Sprint 1 - Essential for TrustChain)

**MUST HAVE**:

1. **STOQ API Server Wrapper**
   - Expose `ConsensusValidationService` methods via STOQ API
   - Implement 3 required endpoints (validate_certificate, validate_proofs, validation_status)
   - Handle STOQ protocol serialization/deserialization

2. **Basic Proof Validation**
   - Use existing `ConsensusValidationService.validate_certificate_request()`
   - Use existing `ConsensusValidationService.validate_four_proof_set()`
   - Return proper validation results with status codes

3. **Minimal Byzantine Detection**
   - Use existing `consensus.is_node_byzantine()` check
   - Return `Invalid` status if Byzantine node detected

4. **Error Handling**
   - Proper STOQ API error responses
   - Validation timeout handling
   - Request tracking for status queries

**Files to Create/Modify**:
- **NEW**: `hypermesh/src/api/consensus_handlers.rs` - STOQ API handlers for consensus endpoints
- **MODIFY**: `hypermesh/src/api/mod.rs` - Register consensus handlers
- **NEW**: `hypermesh/src/bin/consensus-server.rs` - Standalone consensus server binary

### 3.2 Full Implementation Scope (Future Sprints)

**DEFERRED**:

1. **Advanced Cryptographic Validation**
   - Full PoW difficulty verification with actual hash computation
   - Kyber-1024 encryption validation for space proofs
   - FALCON-1024 signature verification for stake proofs

2. **Multi-Node Consensus**
   - Distribute validation across multiple HyperMesh nodes
   - Achieve Byzantine Fault Tolerance quorum (2f+1)
   - Real consensus confidence scores based on node agreement

3. **Performance Optimization**
   - Validation result caching
   - Parallel proof validation
   - Connection pooling for STOQ clients

4. **Advanced Byzantine Detection**
   - Real-time reputation scoring
   - Pattern-based attack detection
   - Automatic node isolation and recovery

5. **Production Monitoring**
   - Prometheus metrics export
   - Distributed tracing integration
   - Real-time performance dashboards

---

## 4. Dependencies and Existing Code

### 4.1 HyperMesh Modules Required

**ALREADY EXIST**:

- `hypermesh/src/consensus/validation_service.rs` - **Core validation logic** (731 lines)
  - `ConsensusValidationService` struct with all methods
  - TrustChain proof conversion
  - Byzantine detection integration
  - Metrics tracking

- `hypermesh/src/consensus/proof.rs` - **Proof types and validation**
  - `ConsensusProof`, `ProofOfSpace`, `ProofOfStake`, `ProofOfWork`, `ProofOfTime`
  - Proof validation logic

- `hypermesh/src/consensus/mod.rs` - **Consensus engine**
  - `Consensus` struct with `validate_consensus_proof()`
  - Byzantine node tracking

- `stoq/src/api/mod.rs` - **STOQ API framework**
  - `StoqApiServer` for registering handlers
  - `StoqApiClient` for making requests
  - `ApiHandler` trait for implementing endpoints

### 4.2 Import Requirements

**From TrustChain** (for type compatibility):
```rust
use trustchain::consensus::{
    ConsensusValidationRequest, ConsensusValidationResult, ConsensusValidationStatus,
    FourProofValidationRequest, ValidationContext, CertificateType,
};
```

**From STOQ**:
```rust
use stoq::api::{StoqApiServer, ApiHandler, ApiRequest, ApiResponse, ApiError};
use stoq::transport::{StoqTransport, TransportConfig};
```

**From HyperMesh**:
```rust
use hypermesh::consensus::{
    ConsensusValidationService, Consensus, NodeId,
    validation_service::*,
};
```

### 4.3 Type Mapping

The existing `validation_service.rs` already defines duplicate types for compatibility:

- `TrustChainCertificateRequest` maps to TrustChain's `CertificateRequest`
- `TrustChainConsensusProof` maps to TrustChain's `ConsensusProof`
- `ExternalFourProofSet` maps to TrustChain's `FourProofSet`
- `ValidationResult` maps to TrustChain's `ConsensusValidationResult`

**Note**: Consider consolidating these types or using `serde` aliases to avoid duplication.

---

## 5. Implementation Plan

### 5.1 Phase 1: STOQ API Handlers (Sprint 1)

**Step 1**: Create consensus API handler module

**File**: `hypermesh/src/api/consensus_handlers.rs`

```rust
use async_trait::async_trait;
use std::sync::Arc;
use stoq::api::{ApiHandler, ApiRequest, ApiResponse, ApiError};
use crate::consensus::ConsensusValidationService;

pub struct ValidateCertificateHandler {
    validation_service: Arc<ConsensusValidationService>,
}

#[async_trait]
impl ApiHandler for ValidateCertificateHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        // Deserialize ConsensusValidationRequest
        // Call validation_service.validate_certificate_request()
        // Serialize ConsensusValidationResult
        // Return ApiResponse
    }

    fn path(&self) -> &str {
        "consensus/validate_certificate"
    }
}

pub struct ValidateProofsHandler {
    validation_service: Arc<ConsensusValidationService>,
}

#[async_trait]
impl ApiHandler for ValidateProofsHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        // Deserialize FourProofValidationRequest
        // Call validation_service.validate_four_proof_set()
        // Serialize ConsensusValidationResult
        // Return ApiResponse
    }

    fn path(&self) -> &str {
        "consensus/validate_proofs"
    }
}

pub struct ValidationStatusHandler {
    validation_service: Arc<ConsensusValidationService>,
}

#[async_trait]
impl ApiHandler for ValidationStatusHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        // Deserialize StatusRequest
        // Call validation_service.get_validation_status()
        // Serialize ConsensusValidationResult
        // Return ApiResponse
    }

    fn path(&self) -> &str {
        "consensus/validation_status"
    }
}
```

**Step 2**: Register handlers with STOQ API server

**File**: `hypermesh/src/api/mod.rs`

```rust
pub mod consensus_handlers;

pub async fn create_consensus_api_server(
    transport: Arc<StoqTransport>,
    validation_service: Arc<ConsensusValidationService>,
) -> Result<StoqApiServer> {
    let server = StoqApiServer::new(transport);

    // Register consensus handlers
    server.register_handler(Arc::new(
        consensus_handlers::ValidateCertificateHandler::new(validation_service.clone())
    ));
    server.register_handler(Arc::new(
        consensus_handlers::ValidateProofsHandler::new(validation_service.clone())
    ));
    server.register_handler(Arc::new(
        consensus_handlers::ValidationStatusHandler::new(validation_service.clone())
    ));

    Ok(server)
}
```

**Step 3**: Create standalone consensus server binary

**File**: `hypermesh/src/bin/consensus-server.rs`

```rust
use hypermesh::consensus::{Consensus, ConsensusValidationService};
use hypermesh::api::create_consensus_api_server;
use stoq::transport::{StoqTransport, TransportConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize consensus system
    let consensus = Arc::new(Consensus::new(...).await?);

    // Create validation service
    let validation_service = Arc::new(
        ConsensusValidationService::new(consensus, node_id, config).await?
    );

    // Create STOQ transport
    let transport_config = TransportConfig::default();
    let transport = Arc::new(StoqTransport::new(transport_config).await?);

    // Create and start API server
    let api_server = create_consensus_api_server(transport, validation_service).await?;
    api_server.listen().await?;

    Ok(())
}
```

### 5.2 Phase 2: Testing and Validation

**Integration Tests**:

1. Test certificate validation request flow
2. Test four-proof validation request flow
3. Test validation status queries
4. Test Byzantine node rejection
5. Test error handling and timeouts

**Test File**: `hypermesh/src/consensus/tests/api_integration_tests.rs`

### 5.3 Phase 3: Documentation and Deployment

1. Update `hypermesh/README.md` with consensus server usage
2. Create deployment guide for consensus server
3. Add Prometheus metrics endpoint
4. Create systemd service file for production deployment

---

## 6. Configuration Requirements

### 6.1 Consensus Server Configuration

```toml
# config/consensus-server.toml
[server]
node_id = "hypermesh-consensus-1"
bind_address = "::"  # IPv6 all interfaces
port = 9292          # STOQ default port

[validation]
max_concurrent_validations = 100
validation_timeout_secs = 30
byzantine_tolerance = 0.33
min_confidence_level = 0.80
enable_detailed_logging = true

[consensus]
# Consensus engine configuration
# (uses existing consensus config)

[transport]
# STOQ transport configuration
# (uses existing STOQ config)
```

### 6.2 TrustChain Client Configuration

```toml
# TrustChain already has this configured
[hypermesh_client]
hypermesh_endpoint = "localhost:9292"  # For testing
# hypermesh_endpoint = "hypermesh.hypermesh.online:9292"  # For production
request_timeout_secs = 60
max_retries = 5
retry_backoff_secs = 1
enable_caching = true
cache_ttl_secs = 600
```

---

## 7. Performance Targets

### 7.1 MVP Targets (Achievable with existing code)

- **Validation Latency**: < 100ms average (single-node validation)
- **Throughput**: 100 validations/second
- **Availability**: 99.9% uptime
- **Error Rate**: < 0.1% validation errors

### 7.2 Production Targets (Future)

- **Validation Latency**: < 50ms average (multi-node consensus)
- **Throughput**: 1000 validations/second
- **Availability**: 99.99% uptime
- **Byzantine Tolerance**: 33% malicious nodes

---

## 8. Security Considerations

### 8.1 MVP Security

1. **STOQ Transport Security**: QUIC provides TLS 1.3 encryption automatically
2. **Byzantine Detection**: Use existing `is_node_byzantine()` check
3. **Request Validation**: Validate all incoming request fields
4. **Rate Limiting**: Basic request rate limiting per client

### 8.2 Production Security (Future)

1. **Certificate Validation**: Verify TrustChain client certificates
2. **Advanced Byzantine Detection**: Machine learning-based anomaly detection
3. **Audit Logging**: Log all validation requests and results
4. **DDoS Protection**: Advanced rate limiting and traffic shaping

---

## 9. File References

### Primary Analysis Sources

1. **TrustChain Client**: `/home/persist/repos/projects/web3/trustchain/src/consensus/hypermesh_client.rs`
   - Lines 77-90: `ConsensusValidationRequest`
   - Lines 119-174: `FourProofValidationRequest` and `FourProofSet`
   - Lines 175-246: `ConsensusValidationResult` and related types
   - Lines 284-318: `validate_certificate_request()` method
   - Lines 320-349: `validate_four_proofs()` method
   - Lines 352-371: `check_validation_status()` method

2. **HyperMesh Validation Service**: `/home/persist/repos/projects/web3/hypermesh/src/consensus/validation_service.rs`
   - Lines 265-338: `ConsensusValidationService` struct
   - Lines 357-389: `validate_certificate_request()` implementation
   - Lines 391-421: `validate_four_proof_set()` implementation
   - Lines 511-561: TrustChain proof conversion logic
   - Lines 615-651: Consensus proof validation logic

3. **STOQ API Framework**: `/home/persist/repos/projects/web3/stoq/src/api/mod.rs`
   - Lines 17-45: `ApiRequest` and `ApiResponse` types
   - Lines 77-84: `ApiHandler` trait
   - Lines 86-249: `StoqApiServer` implementation
   - Lines 251-373: `StoqApiClient` implementation

4. **TrustChain Consensus Types**: `/home/persist/repos/projects/web3/trustchain/src/consensus/mod.rs`
   - Lines 24-152: `ConsensusProof` and proof components
   - Lines 154-202: `ConsensusRequirements`
   - Lines 204-228: `ConsensusResult`

### Supporting Files

- `/home/persist/repos/projects/web3/hypermesh/src/consensus/proof.rs` - Proof validation logic
- `/home/persist/repos/projects/web3/trustchain/src/consensus/proof.rs` - TrustChain proof types
- `/home/persist/repos/projects/web3/trustchain/src/ca/mod.rs` - Certificate request types

---

## 10. Next Steps

### For Developer Agent (Sprint 1, Step 4)

1. **Create consensus API handlers** (`hypermesh/src/api/consensus_handlers.rs`)
   - Implement `ValidateCertificateHandler`
   - Implement `ValidateProofsHandler`
   - Implement `ValidationStatusHandler`

2. **Integrate with STOQ API server** (`hypermesh/src/api/mod.rs`)
   - Add `create_consensus_api_server()` function
   - Register all handlers

3. **Create consensus server binary** (`hypermesh/src/bin/consensus-server.rs`)
   - Initialize consensus system
   - Start STOQ API server
   - Add graceful shutdown

4. **Write integration tests** (`hypermesh/src/consensus/tests/api_integration_tests.rs`)
   - Test all three endpoints
   - Test error scenarios
   - Test Byzantine rejection

### Verification Checklist

- [ ] All three endpoints respond correctly to TrustChain requests
- [ ] Proof validation uses existing `ConsensusValidationService` logic
- [ ] Byzantine nodes are rejected
- [ ] Error responses follow STOQ API format
- [ ] Performance meets MVP targets (< 100ms, 100 req/sec)
- [ ] No duplicates or stub implementations
- [ ] All types properly serialized via STOQ protocol

---

## Appendix A: Type Compatibility Matrix

| TrustChain Type | HyperMesh Type | Conversion Required |
|-----------------|----------------|---------------------|
| `ConsensusValidationRequest` | `CertificateValidationRequest` | Yes - via `convert_trustchain_proof()` |
| `FourProofValidationRequest` | `FourProofValidationRequest` | Yes - via `convert_external_proof_set()` |
| `ConsensusValidationResult` | `ValidationResult` | Direct mapping |
| `ConsensusProof` | `ConsensusProof` | Yes - format conversion |
| `FourProofSet` | `ExternalFourProofSet` | Yes - structure mapping |

---

## Appendix B: Error Code Reference

| Error Code | Description | HTTP Equivalent |
|------------|-------------|-----------------|
| `CONSENSUS_NOT_INITIALIZED` | Consensus system not ready | 503 Service Unavailable |
| `INVALID_PROOF_FORMAT` | Proof deserialization failed | 400 Bad Request |
| `BYZANTINE_NODE_DETECTED` | Requesting node is Byzantine | 403 Forbidden |
| `VALIDATION_TIMEOUT` | Validation took too long | 504 Gateway Timeout |
| `INTERNAL_CONSENSUS_ERROR` | Consensus system error | 500 Internal Server Error |
| `REQUEST_NOT_FOUND` | Validation request ID not found | 404 Not Found |

---

**End of Requirements Document**

**Prepared for**: Developer Agent (Step 4 Implementation)
**Review Status**: Ready for implementation
**Estimated Implementation Time**: 8-12 hours (MVP scope)
