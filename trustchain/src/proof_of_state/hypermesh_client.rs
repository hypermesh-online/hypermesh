// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh State Proof Client for TrustChain
//!
//! This module provides the client interface for TrustChain to request
//! state proof validation from HyperMesh. It implements the architectural
//! separation where TrustChain focuses on certificate operations while
//! HyperMesh provides the four-proof state proof validation services.
//!
//! **STOQ Protocol**: Uses STOQ API (QUIC transport) instead of HTTP

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use stoq::{
    transport::{StoqTransport, TransportConfig},
    StoqApiClient,
};

use super::StateRequirements;
use crate::ca::CertificateRequest;

/// HyperMesh Proof of State validation client
pub struct HyperMeshStateProofClient {
    /// STOQ API client for state proof requests
    stoq_client: Arc<StoqApiClient>,
    /// Client configuration
    config: HyperMeshClientConfig,
    /// Performance metrics
    metrics: Arc<RwLock<StateProofClientMetrics>>,
}

/// Configuration for HyperMesh Proof of State client
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HyperMeshClientConfig {
    /// Request timeout for state proof validation
    pub request_timeout: Duration,
    /// Maximum retries for failed requests
    pub max_retries: u32,
    /// Backoff multiplier for retries
    pub retry_backoff: Duration,
    /// Enable state proof caching
    pub enable_caching: bool,
    /// Cache TTL for valid state proof results
    pub cache_ttl: Duration,
}

impl Default for HyperMeshClientConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_backoff: Duration::from_millis(500),
            enable_caching: true,
            cache_ttl: Duration::from_secs(300), // 5 minutes
        }
    }
}

impl HyperMeshClientConfig {
    /// Production configuration for HyperMesh integration
    pub fn production(_hypermesh_endpoint: String) -> Self {
        Self {
            request_timeout: Duration::from_secs(60),
            max_retries: 5,
            retry_backoff: Duration::from_secs(1),
            enable_caching: true,
            cache_ttl: Duration::from_secs(600), // 10 minutes
        }
    }

    /// Localhost testing configuration
    pub fn localhost_testing() -> Self {
        Self::default()
    }
}

/// State proof validation request to HyperMesh
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateProofValidationRequest {
    /// Certificate request for state proof validation
    pub certificate_request: CertificateRequest,
    /// Required state proof level
    pub state_requirements: StateRequirements,
    /// Request ID for tracking
    pub request_id: String,
    /// Request timestamp
    pub timestamp: SystemTime,
    /// Additional validation context
    pub validation_context: ValidationContext,
}

/// Additional context for state proof validation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationContext {
    /// TrustChain CA identifier
    pub ca_id: String,
    /// Network identifier
    pub network_id: String,
    /// Certificate type being requested
    pub certificate_type: CertificateType,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CertificateType {
    /// Standard TLS certificate
    TLS,
    /// Code signing certificate
    CodeSigning,
    /// Client authentication certificate
    ClientAuth,
    /// Root CA certificate
    RootCA,
    /// Intermediate CA certificate
    IntermediateCA,
}

/// Four-proof validation request for complex operations.
///
/// The proof set is the canonical [`hypermesh_lib::proof::StateProof`] — the
/// single source of truth for the four-proof model (WHERE/WHO/WHAT/WHEN). It
/// carries authorization (StakeProof = FALCON identity binding, no magnitude),
/// a work hash (WorkProof = BLAKE3 of work done, no difficulty), a location
/// (SpaceProof = WHERE, capacity is descriptive), and a timestamp (TimeProof).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FourProofValidationRequest {
    /// Canonical four-proof set to validate.
    pub proof_set: hypermesh_lib::proof::StateProof,
    /// Operation being validated
    pub operation: String,
    /// Asset or resource identifier
    pub asset_id: String,
    /// Node requesting validation
    pub node_id: String,
    /// Request timestamp
    pub timestamp: SystemTime,
}

/// State proof validation result from HyperMesh
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateProofValidationResult {
    /// Validation result
    pub result: StateProofValidationStatus,
    /// State proof hash
    pub proof_hash: Option<[u8; 32]>,
    /// HyperMesh validator node ID
    pub validator_id: String,
    /// Validation timestamp
    pub validated_at: SystemTime,
    /// Validation metrics
    pub metrics: ValidationMetrics,
    /// Additional details
    pub details: ValidationDetails,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StateProofValidationStatus {
    /// All four proofs validated successfully
    Valid,
    /// One or more proofs failed validation
    Invalid {
        failed_proofs: Vec<String>,
        reason: String,
    },
    /// Validation is still pending
    Pending { estimated_completion: SystemTime },
    /// Validation failed due to system error
    Error { error_code: String, message: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationMetrics {
    /// Time taken for validation (microseconds)
    pub validation_time_us: u64,
    /// Number of nodes that participated in validation
    pub validator_nodes: u32,
    /// Whether all four proofs passed (binary)
    pub all_proofs_valid: bool,
    /// Network load during validation
    pub network_load: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationDetails {
    /// Individual proof validation results
    pub proof_results: ProofValidationResults,
    /// Byzantine fault tolerance status
    pub bft_status: ByzantineFaultToleranceStatus,
    /// Performance statistics
    pub performance_stats: PerformanceStatistics,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofValidationResults {
    pub space_proof_valid: bool,
    pub stake_proof_valid: bool,
    pub work_proof_valid: bool,
    pub time_proof_valid: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ByzantineFaultToleranceStatus {
    pub byzantine_nodes_detected: u32,
    pub fault_tolerance_maintained: bool,
    pub recovery_action_taken: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceStatistics {
    pub state_proof_latency_ms: u64,
    pub throughput_ops_per_sec: f64,
    pub network_overhead_bytes: u64,
}

/// Client performance metrics
#[derive(Clone, Debug, Default)]
pub struct StateProofClientMetrics {
    /// Total validation requests sent
    pub total_requests: u64,
    /// Successful validations
    pub successful_validations: u64,
    /// Failed validations
    pub failed_validations: u64,
    /// Average request latency (microseconds)
    pub avg_latency_us: u64,
    /// Cache hit rate
    pub cache_hit_rate: f64,
    /// Last update timestamp
    pub last_updated: Option<SystemTime>,
}

impl HyperMeshStateProofClient {
    /// Create new HyperMesh Proof of State client with STOQ transport
    pub async fn new(config: HyperMeshClientConfig) -> Result<Self> {
        info!("Initializing HyperMesh Proof of State client (STOQ protocol)");

        // Create STOQ transport for client (port 0 = OS-assigned to avoid conflicts)
        let transport_config = TransportConfig {
            port: 0,
            ..Default::default()
        };
        let transport = Arc::new(StoqTransport::new(transport_config).await?);

        // Create STOQ API client
        let stoq_client = Arc::new(StoqApiClient::new(transport));

        Ok(Self {
            stoq_client,
            config,
            metrics: Arc::new(RwLock::new(StateProofClientMetrics::default())),
        })
    }

    /// Validate certificate request through HyperMesh Proof of State
    pub async fn validate_certificate_request(
        &self,
        request: &CertificateRequest,
        requirements: &StateRequirements,
    ) -> Result<StateProofValidationResult> {
        let start_time = std::time::Instant::now();

        debug!(
            "Validating certificate request with HyperMesh Proof of State: {}",
            request.common_name
        );

        // Create validation request
        let validation_request = StateProofValidationRequest {
            certificate_request: request.clone(),
            state_requirements: requirements.clone(),
            request_id: format!(
                "trustchain-{}-{}",
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)?
                    .as_millis(),
                request.common_name
            ),
            timestamp: SystemTime::now(),
            validation_context: ValidationContext {
                ca_id: "trustchain-ca".to_string(),
                network_id: "hypermesh-production".to_string(),
                certificate_type: CertificateType::TLS,
                metadata: std::collections::HashMap::new(),
            },
        };

        // Send validation request with retries
        let result = self
            .send_validation_request_with_retry(validation_request)
            .await?;

        // Update metrics
        self.update_metrics(start_time, &result).await;

        debug!("Certificate validation completed: {:?}", result.result);
        Ok(result)
    }

    /// Validate the canonical four-proof set for complex operations
    pub async fn validate_four_proofs(
        &self,
        proof_set: &hypermesh_lib::proof::StateProof,
        operation: &str,
        asset_id: &str,
        node_id: &str,
    ) -> Result<StateProofValidationResult> {
        let start_time = std::time::Instant::now();

        debug!("Validating four-proof set for operation: {}", operation);

        // Create four-proof validation request
        let validation_request = FourProofValidationRequest {
            proof_set: proof_set.clone(),
            operation: operation.to_string(),
            asset_id: asset_id.to_string(),
            node_id: node_id.to_string(),
            timestamp: SystemTime::now(),
        };

        // Send four-proof validation request
        let result = self
            .send_four_proof_validation_request(validation_request)
            .await?;

        // Update metrics
        self.update_metrics(start_time, &result).await;

        debug!("Four-proof validation completed: {:?}", result.result);
        Ok(result)
    }

    /// Check state proof validation status for pending requests
    pub async fn check_validation_status(
        &self,
        request_id: &str,
    ) -> Result<StateProofValidationResult> {
        debug!("Checking validation status for request: {}", request_id);

        #[derive(Serialize)]
        struct StatusRequest {
            request_id: String,
        }

        let request = StatusRequest {
            request_id: request_id.to_string(),
        };

        // Call HyperMesh validation status handler via STOQ
        let result: StateProofValidationResult = self
            .stoq_client
            .call("hypermesh", "state_proof/validation_status", &request)
            .await
            .map_err(|e| anyhow!("STOQ API error checking validation status: {e}"))?;

        Ok(result)
    }

    /// Get client performance metrics
    pub async fn get_metrics(&self) -> StateProofClientMetrics {
        self.metrics.read().await.clone()
    }

    /// Reset client metrics
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        *metrics = StateProofClientMetrics::default();
    }

    // Internal: Send validation request with retry logic
    async fn send_validation_request_with_retry(
        &self,
        request: StateProofValidationRequest,
    ) -> Result<StateProofValidationResult> {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            match self.send_validation_request(&request).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    warn!("Validation request attempt {} failed: {}", attempt + 1, e);
                    last_error = Some(e);

                    if attempt < self.config.max_retries {
                        let backoff = self.config.retry_backoff * (2_u32.pow(attempt));
                        tokio::time::sleep(backoff).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("All validation attempts failed")))
    }

    // Internal: Send single validation request via STOQ
    async fn send_validation_request(
        &self,
        request: &StateProofValidationRequest,
    ) -> Result<StateProofValidationResult> {
        // Call HyperMesh Proof of State validation handler via STOQ
        let result: StateProofValidationResult = self
            .stoq_client
            .call("hypermesh", "state_proof/validate_certificate", request)
            .await
            .map_err(|e| anyhow!("STOQ API error sending validation request: {e}"))?;

        Ok(result)
    }

    // Internal: Send four-proof validation request via STOQ
    async fn send_four_proof_validation_request(
        &self,
        request: FourProofValidationRequest,
    ) -> Result<StateProofValidationResult> {
        // Call HyperMesh four-proof validation handler via STOQ
        let result: StateProofValidationResult = self
            .stoq_client
            .call("hypermesh", "state_proof/validate_proofs", &request)
            .await
            .map_err(|e| anyhow!("STOQ API error sending four-proof validation: {e}"))?;

        Ok(result)
    }

    // Internal: Update performance metrics
    async fn update_metrics(
        &self,
        start_time: std::time::Instant,
        result: &StateProofValidationResult,
    ) {
        let mut metrics = self.metrics.write().await;

        metrics.total_requests += 1;

        match result.result {
            StateProofValidationStatus::Valid => {
                metrics.successful_validations += 1;
            }
            _ => {
                metrics.failed_validations += 1;
            }
        }

        let latency_us = start_time.elapsed().as_micros() as u64;

        // Update rolling average latency
        if metrics.total_requests == 1 {
            metrics.avg_latency_us = latency_us;
        } else {
            metrics.avg_latency_us = (metrics.avg_latency_us * (metrics.total_requests - 1)
                + latency_us)
                / metrics.total_requests;
        }

        metrics.last_updated = Some(SystemTime::now());
    }
}

/// Trait for state proof validation service
#[allow(async_fn_in_trait)]
pub trait StateProofValidationService {
    /// Validate certificate request with state proof
    async fn validate_certificate_request(
        &self,
        request: &CertificateRequest,
        requirements: &StateRequirements,
    ) -> Result<StateProofValidationResult>;

    /// Validate the canonical four-proof set for complex operations
    async fn validate_four_proofs(
        &self,
        proof_set: &hypermesh_lib::proof::StateProof,
        operation: &str,
        asset_id: &str,
        node_id: &str,
    ) -> Result<StateProofValidationResult>;
}

impl StateProofValidationService for HyperMeshStateProofClient {
    async fn validate_certificate_request(
        &self,
        request: &CertificateRequest,
        requirements: &StateRequirements,
    ) -> Result<StateProofValidationResult> {
        self.validate_certificate_request(request, requirements)
            .await
    }

    async fn validate_four_proofs(
        &self,
        proof_set: &hypermesh_lib::proof::StateProof,
        operation: &str,
        asset_id: &str,
        node_id: &str,
    ) -> Result<StateProofValidationResult> {
        self.validate_four_proofs(proof_set, operation, asset_id, node_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_creation() {
        let config = HyperMeshClientConfig::default();
        assert!(config.request_timeout > Duration::ZERO);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_production_config() {
        let config = HyperMeshClientConfig::production("hypermesh.example.com".to_string());
        assert_eq!(config.max_retries, 5);
        assert!(config.request_timeout > Duration::from_secs(30));
    }

    #[tokio::test]
    async fn test_client_metrics() {
        let config = HyperMeshClientConfig::localhost_testing();
        let client = HyperMeshStateProofClient::new(config).await.expect("test: async operation");

        let metrics = client.get_metrics().await;
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_validations, 0);
    }

    #[test]
    fn test_four_proof_set_creation() {
        use hypermesh_lib::proof::{SpaceProof, StakeProof, StateProof, TimeProof, WorkProof};
        use std::time::Duration;

        // Canonical four-proof set: authorization (WHO) + work hash (WHAT) +
        // location (WHERE) + time (WHEN). No stake magnitude, no difficulty.
        let proof_set = StateProof::new(
            StakeProof::new("test-owner".to_string(), "owner-identity-id".to_string()),
            TimeProof::new(Duration::from_secs(0)),
            SpaceProof::new(
                "test-node".to_string(),
                "hypermesh://proxy/test".to_string(),
                1024,
            ),
            WorkProof::from_work(
                "test-owner".to_string(),
                "test-operation".to_string(),
                b"the registration work",
            ));

        // PoStake answers WHO via a bound identity — never a magnitude.
        assert!(proof_set.stake_proof.is_structurally_valid());
        assert_eq!(proof_set.stake_proof.stake_holder_id, "owner-identity-id");
        // PoWork answers WHAT via a non-zero content hash — never a difficulty.
        assert!(proof_set.work_proof.is_structurally_valid());
        // PoSpace answers WHERE via a bound location — capacity is descriptive.
        assert!(proof_set.space_proof.is_structurally_valid());
    }
}

