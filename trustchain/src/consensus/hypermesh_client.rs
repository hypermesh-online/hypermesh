//! HyperMesh Consensus Client for TrustChain
//!
//! This module provides the client interface for TrustChain to request
//! consensus validation from HyperMesh. It implements the architectural
//! separation where TrustChain focuses on certificate operations while
//! HyperMesh provides the four-proof consensus validation services.
//!
//! **STOQ Protocol**: Uses STOQ API (QUIC transport) instead of HTTP

use std::sync::Arc;
use std::time::{SystemTime, Duration};
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};

use stoq::{StoqApiClient, transport::{StoqTransport, TransportConfig}};

use crate::ca::CertificateRequest;
use super::{ConsensusProof, ConsensusResult, ConsensusRequirements};

/// HyperMesh consensus validation client
pub struct HyperMeshConsensusClient {
    /// STOQ API client for consensus requests
    stoq_client: Arc<StoqApiClient>,
    /// Client configuration
    config: HyperMeshClientConfig,
    /// Performance metrics
    metrics: Arc<RwLock<ConsensusClientMetrics>>,
}

/// Configuration for HyperMesh consensus client
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HyperMeshClientConfig {
    /// Request timeout for consensus validation
    pub request_timeout: Duration,
    /// Maximum retries for failed requests
    pub max_retries: u32,
    /// Backoff multiplier for retries
    pub retry_backoff: Duration,
    /// Enable consensus caching
    pub enable_caching: bool,
    /// Cache TTL for valid consensus results
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

/// Consensus validation request to HyperMesh
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusValidationRequest {
    /// Certificate request for consensus validation
    pub certificate_request: CertificateRequest,
    /// Required consensus level
    pub consensus_requirements: ConsensusRequirements,
    /// Request ID for tracking
    pub request_id: String,
    /// Request timestamp
    pub timestamp: SystemTime,
    /// Additional validation context
    pub validation_context: ValidationContext,
}

/// Additional context for consensus validation
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

/// Four-proof validation request for complex operations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FourProofValidationRequest {
    /// Proof set to validate
    pub proof_set: FourProofSet,
    /// Operation being validated
    pub operation: String,
    /// Asset or resource identifier
    pub asset_id: String,
    /// Node requesting validation
    pub node_id: String,
    /// Request timestamp
    pub timestamp: SystemTime,
}

/// Complete four-proof set for validation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FourProofSet {
    /// WHERE: Storage location and network position
    pub space_proof: SpaceProofData,
    /// WHO: Ownership and access rights
    pub stake_proof: StakeProofData,
    /// WHAT/HOW: Computational work and processing
    pub work_proof: WorkProofData,
    /// WHEN: Temporal ordering and timing
    pub time_proof: TimeProofData,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpaceProofData {
    pub storage_commitment: u64,
    pub network_position: String,
    pub allocation_proof: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StakeProofData {
    pub stake_amount: u64,
    pub authority_level: u64,
    pub access_permissions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkProofData {
    pub computational_proof: Vec<u8>,
    pub difficulty_target: u32,
    pub operation_signature: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeProofData {
    pub block_timestamp: u64,
    pub sequence_number: u64,
    pub temporal_proof: Vec<u8>,
}

/// Consensus validation result from HyperMesh
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusValidationResult {
    /// Validation result
    pub result: ConsensusValidationStatus,
    /// Consensus proof hash
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
pub enum ConsensusValidationStatus {
    /// All four proofs validated successfully
    Valid,
    /// One or more proofs failed validation
    Invalid { failed_proofs: Vec<String>, reason: String },
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
    /// Consensus confidence level (0.0 - 1.0)
    pub confidence_level: f64,
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
    pub consensus_latency_ms: u64,
    pub throughput_ops_per_sec: f64,
    pub network_overhead_bytes: u64,
}

/// Client performance metrics
#[derive(Clone, Debug, Default)]
pub struct ConsensusClientMetrics {
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

impl HyperMeshConsensusClient {
    /// Create new HyperMesh consensus client with STOQ transport
    pub async fn new(config: HyperMeshClientConfig) -> Result<Self> {
        info!("Initializing HyperMesh consensus client (STOQ protocol)");

        // Create STOQ transport for client
        let transport_config = TransportConfig::default();
        let transport = Arc::new(StoqTransport::new(transport_config).await?);

        // Create STOQ API client
        let stoq_client = Arc::new(StoqApiClient::new(transport));

        Ok(Self {
            stoq_client,
            config,
            metrics: Arc::new(RwLock::new(ConsensusClientMetrics::default())),
        })
    }

    /// Validate certificate request through HyperMesh consensus
    pub async fn validate_certificate_request(
        &self,
        request: &CertificateRequest,
        requirements: &ConsensusRequirements,
    ) -> Result<ConsensusValidationResult> {
        let start_time = std::time::Instant::now();
        
        debug!("Validating certificate request with HyperMesh consensus: {}", request.common_name);

        // Create validation request
        let validation_request = ConsensusValidationRequest {
            certificate_request: request.clone(),
            consensus_requirements: requirements.clone(),
            request_id: format!("trustchain-{}-{}", 
                SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_millis(),
                request.common_name),
            timestamp: SystemTime::now(),
            validation_context: ValidationContext {
                ca_id: "trustchain-ca".to_string(),
                network_id: "hypermesh-production".to_string(),
                certificate_type: CertificateType::TLS,
                metadata: std::collections::HashMap::new(),
            },
        };

        // Send validation request with retries
        let result = self.send_validation_request_with_retry(validation_request).await?;

        // Update metrics
        self.update_metrics(start_time, &result).await;

        debug!("Certificate validation completed: {:?}", result.result);
        Ok(result)
    }

    /// Validate four-proof set for complex operations
    pub async fn validate_four_proofs(
        &self,
        proof_set: &FourProofSet,
        operation: &str,
        asset_id: &str,
        node_id: &str,
    ) -> Result<ConsensusValidationResult> {
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
        let result = self.send_four_proof_validation_request(validation_request).await?;

        // Update metrics
        self.update_metrics(start_time, &result).await;

        debug!("Four-proof validation completed: {:?}", result.result);
        Ok(result)
    }

    /// Check consensus validation status for pending requests
    pub async fn check_validation_status(&self, request_id: &str) -> Result<ConsensusValidationResult> {
        debug!("Checking validation status for request: {}", request_id);

        #[derive(Serialize)]
        struct StatusRequest {
            request_id: String,
        }

        let request = StatusRequest {
            request_id: request_id.to_string(),
        };

        // Call HyperMesh validation status handler via STOQ
        let result: ConsensusValidationResult = self.stoq_client
            .call("hypermesh", "consensus/validation_status", &request)
            .await
            .map_err(|e| anyhow!("STOQ API error checking validation status: {}", e))?;

        Ok(result)
    }

    /// Get client performance metrics
    pub async fn get_metrics(&self) -> ConsensusClientMetrics {
        self.metrics.read().await.clone()
    }

    /// Reset client metrics
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        *metrics = ConsensusClientMetrics::default();
    }

    // Internal: Send validation request with retry logic
    async fn send_validation_request_with_retry(
        &self,
        request: ConsensusValidationRequest,
    ) -> Result<ConsensusValidationResult> {
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
        request: &ConsensusValidationRequest,
    ) -> Result<ConsensusValidationResult> {
        // Call HyperMesh consensus validation handler via STOQ
        let result: ConsensusValidationResult = self.stoq_client
            .call("hypermesh", "consensus/validate_certificate", request)
            .await
            .map_err(|e| anyhow!("STOQ API error sending validation request: {}", e))?;

        Ok(result)
    }

    // Internal: Send four-proof validation request via STOQ
    async fn send_four_proof_validation_request(
        &self,
        request: FourProofValidationRequest,
    ) -> Result<ConsensusValidationResult> {
        // Call HyperMesh four-proof validation handler via STOQ
        let result: ConsensusValidationResult = self.stoq_client
            .call("hypermesh", "consensus/validate_proofs", &request)
            .await
            .map_err(|e| anyhow!("STOQ API error sending four-proof validation: {}", e))?;

        Ok(result)
    }

    // Internal: Update performance metrics
    async fn update_metrics(&self, start_time: std::time::Instant, result: &ConsensusValidationResult) {
        let mut metrics = self.metrics.write().await;
        
        metrics.total_requests += 1;
        
        match result.result {
            ConsensusValidationStatus::Valid => {
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
            metrics.avg_latency_us = (metrics.avg_latency_us * (metrics.total_requests - 1) + latency_us) / metrics.total_requests;
        }

        metrics.last_updated = Some(SystemTime::now());
    }
}

/// Trait for consensus validation service
pub trait ConsensusValidationService {
    /// Validate certificate request with consensus
    async fn validate_certificate_request(
        &self,
        request: &CertificateRequest,
        requirements: &ConsensusRequirements,
    ) -> Result<ConsensusValidationResult>;

    /// Validate four-proof set for complex operations
    async fn validate_four_proofs(
        &self,
        proof_set: &FourProofSet,
        operation: &str,
        asset_id: &str,
        node_id: &str,
    ) -> Result<ConsensusValidationResult>;
}

impl ConsensusValidationService for HyperMeshConsensusClient {
    async fn validate_certificate_request(
        &self,
        request: &CertificateRequest,
        requirements: &ConsensusRequirements,
    ) -> Result<ConsensusValidationResult> {
        self.validate_certificate_request(request, requirements).await
    }

    async fn validate_four_proofs(
        &self,
        proof_set: &FourProofSet,
        operation: &str,
        asset_id: &str,
        node_id: &str,
    ) -> Result<ConsensusValidationResult> {
        self.validate_four_proofs(proof_set, operation, asset_id, node_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ConsensusProof;

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
        let client = HyperMeshConsensusClient::new(config).await.unwrap();
        
        let metrics = client.get_metrics().await;
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_validations, 0);
    }

    #[test]
    fn test_four_proof_set_creation() {
        let proof_set = FourProofSet {
            space_proof: SpaceProofData {
                storage_commitment: 1024,
                network_position: "hypermesh://proxy/test".to_string(),
                allocation_proof: vec![1, 2, 3, 4],
            },
            stake_proof: StakeProofData {
                stake_amount: 5000,
                authority_level: 100,
                access_permissions: vec!["read".to_string(), "write".to_string()],
            },
            work_proof: WorkProofData {
                computational_proof: vec![5, 6, 7, 8],
                difficulty_target: 16,
                operation_signature: "test-operation".to_string(),
            },
            time_proof: TimeProofData {
                block_timestamp: 1000,
                sequence_number: 1,
                temporal_proof: vec![9, 10, 11, 12],
            },
        };

        assert_eq!(proof_set.space_proof.storage_commitment, 1024);
        assert_eq!(proof_set.stake_proof.stake_amount, 5000);
    }
}