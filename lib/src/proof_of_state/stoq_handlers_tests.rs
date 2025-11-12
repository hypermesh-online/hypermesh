//! Unit tests for HyperMesh Consensus STOQ API handlers

use super::stoq_handlers::*;
use super::validation_service::{
    ConsensusValidationService,
    CertificateValidationRequest,
    FourProofValidationRequest,
    ValidationResult,
    ValidationStatus,
    ValidationMetrics,
    ValidationDetails,
    ProofValidationResults,
    ByzantineFaultToleranceStatus,
    PerformanceStatistics,
    TrustChainCertificateRequest,
    TrustChainConsensusRequirements,
    TrustChainValidationContext,
    TrustChainConsensusProof,
    TrustChainStakeProof,
    TrustChainTimeProof,
    TrustChainSpaceProof,
    TrustChainWorkProof,
    ExternalFourProofSet,
    SpaceProofData,
    StakeProofData,
    WorkProofData,
    TimeProofData,
};

use stoq::api::{ApiRequest, ApiResponse};
use std::sync::Arc;
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use bytes::Bytes;
use anyhow::Result;

/// Mock validation service for testing
struct MockValidationService {
    should_fail: bool,
    is_byzantine: bool,
}

impl MockValidationService {
    fn new() -> Self {
        Self {
            should_fail: false,
            is_byzantine: false,
        }
    }

    fn with_failure() -> Self {
        Self {
            should_fail: true,
            is_byzantine: false,
        }
    }

    fn with_byzantine() -> Self {
        Self {
            should_fail: false,
            is_byzantine: true,
        }
    }

    async fn mock_validate_certificate(&self) -> Result<ValidationResult> {
        if self.should_fail {
            return Err(anyhow::anyhow!("Mock validation failure"));
        }

        let status = if self.is_byzantine {
            ValidationStatus::Invalid {
                failed_proofs: vec!["Byzantine node detected".to_string()],
                reason: "Node exhibits Byzantine behavior".to_string(),
            }
        } else {
            ValidationStatus::Valid
        };

        Ok(ValidationResult {
            result: status,
            proof_hash: Some([0x42; 32]),
            validator_id: "mock-validator".to_string(),
            validated_at: SystemTime::now(),
            metrics: ValidationMetrics {
                validation_time_us: 1000,
                validator_nodes: 3,
                confidence_level: 0.95,
                network_load: 0.5,
            },
            details: ValidationDetails {
                proof_results: ProofValidationResults {
                    space_proof_valid: !self.is_byzantine,
                    stake_proof_valid: !self.is_byzantine,
                    work_proof_valid: !self.is_byzantine,
                    time_proof_valid: !self.is_byzantine,
                    space_proof_confidence: if self.is_byzantine { 0.0 } else { 0.95 },
                    stake_proof_confidence: if self.is_byzantine { 0.0 } else { 0.95 },
                    work_proof_confidence: if self.is_byzantine { 0.0 } else { 0.95 },
                    time_proof_confidence: if self.is_byzantine { 0.0 } else { 0.95 },
                },
                bft_status: ByzantineFaultToleranceStatus {
                    is_byzantine: self.is_byzantine,
                    byzantine_nodes: if self.is_byzantine { vec!["bad-node".to_string()] } else { vec![] },
                    quorum_achieved: !self.is_byzantine,
                    confidence_threshold_met: !self.is_byzantine,
                },
                performance_stats: PerformanceStatistics {
                    proof_validation_us: HashMap::new(),
                    total_validation_us: 1000,
                    network_latency_us: 100,
                    queue_wait_us: 50,
                },
            },
        })
    }

    async fn mock_validate_four_proofs(&self) -> Result<ValidationResult> {
        // Reuse certificate validation logic for simplicity
        self.mock_validate_certificate().await
    }

    async fn mock_get_validation_status(&self, _request_id: &str) -> Result<ValidationResult> {
        // Return a pending status
        Ok(ValidationResult {
            result: ValidationStatus::Pending {
                estimated_completion: SystemTime::now() + Duration::from_secs(10),
            },
            proof_hash: None,
            validator_id: "mock-validator".to_string(),
            validated_at: SystemTime::now(),
            metrics: ValidationMetrics {
                validation_time_us: 0,
                validator_nodes: 0,
                confidence_level: 0.0,
                network_load: 0.5,
            },
            details: ValidationDetails {
                proof_results: ProofValidationResults {
                    space_proof_valid: false,
                    stake_proof_valid: false,
                    work_proof_valid: false,
                    time_proof_valid: false,
                    space_proof_confidence: 0.0,
                    stake_proof_confidence: 0.0,
                    work_proof_confidence: 0.0,
                    time_proof_confidence: 0.0,
                },
                bft_status: ByzantineFaultToleranceStatus {
                    is_byzantine: false,
                    byzantine_nodes: vec![],
                    quorum_achieved: false,
                    confidence_threshold_met: false,
                },
                performance_stats: PerformanceStatistics {
                    proof_validation_us: HashMap::new(),
                    total_validation_us: 0,
                    network_latency_us: 0,
                    queue_wait_us: 0,
                },
            },
        })
    }
}

fn create_test_certificate_request() -> CertificateValidationRequest {
    CertificateValidationRequest {
        certificate_request: TrustChainCertificateRequest {
            common_name: "test.hypermesh.local".to_string(),
            san_entries: vec!["test.hypermesh.local".to_string()],
            node_id: "test-node-1".to_string(),
            ipv6_addresses: vec![],
            consensus_proof: TrustChainConsensusProof {
                stake_proof: TrustChainStakeProof {
                    stake_amount: 1000,
                    authority_level: 1,
                    stake_holder: "test-holder".to_string(),
                    stake_holder_id: "holder-1".to_string(),
                },
                time_proof: TrustChainTimeProof {
                    block_timestamp: 1234567890,
                    network_time_offset: Duration::from_secs(0),
                    sequence_number: 1,
                },
                space_proof: TrustChainSpaceProof {
                    storage_commitment: vec![0x01, 0x02, 0x03],
                    network_position: vec![0x04, 0x05, 0x06],
                    replication_factor: 3,
                },
                work_proof: TrustChainWorkProof {
                    computational_proof: vec![0x07, 0x08, 0x09],
                    difficulty_target: 100,
                    nonce: 42,
                },
            },
            timestamp: SystemTime::now(),
        },
        consensus_requirements: TrustChainConsensusRequirements {
            minimum_stake: 100,
            max_time_offset: Duration::from_secs(60),
            minimum_storage: 1024,
            minimum_compute: 10,
            byzantine_tolerance: 0.33,
        },
        request_id: "test-request-1".to_string(),
        timestamp: SystemTime::now(),
        validation_context: TrustChainValidationContext {
            ca_id: "trustchain-ca".to_string(),
            network_id: "hypermesh-main".to_string(),
            certificate_type: crate::consensus::validation_service::CertificateType::Server,
            metadata: HashMap::new(),
        },
    }
}

fn create_test_four_proof_request() -> FourProofValidationRequest {
    FourProofValidationRequest {
        proof_set: ExternalFourProofSet {
            space_proof: SpaceProofData {
                storage_commitment: vec![0x01, 0x02],
                network_position: vec![0x03, 0x04],
                proof_data: vec![0x05, 0x06],
            },
            stake_proof: StakeProofData {
                stake_amount: 1000,
                authority_level: 1,
                stake_holder: "test-holder".to_string(),
                proof_data: vec![0x07, 0x08],
            },
            work_proof: WorkProofData {
                computational_proof: vec![0x09, 0x0A],
                difficulty_target: 100,
                proof_data: vec![0x0B, 0x0C],
            },
            time_proof: TimeProofData {
                block_timestamp: 1234567890,
                sequence_number: 1,
                proof_data: vec![0x0D, 0x0E],
            },
        },
        operation: "test-operation".to_string(),
        asset_id: "asset-123".to_string(),
        node_id: "test-node-1".to_string(),
        timestamp: SystemTime::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check_handler() {
        let handler = ConsensusHealthHandler;
        let request = ApiRequest {
            id: "test-health".to_string(),
            service: "test".to_string(),
            method: "consensus/health".to_string(),
            payload: Bytes::new(),
            metadata: HashMap::new(),
        };

        let response = handler.handle(request).await.unwrap();
        assert!(response.success);
        assert!(response.error.is_none());

        // Verify response content
        let health: serde_json::Value = serde_json::from_slice(&response.payload).unwrap();
        assert_eq!(health["status"], "healthy");
        assert_eq!(health["service"], "hypermesh-consensus");
    }

    #[tokio::test]
    async fn test_validation_status_request_serialization() {
        let status_req = StatusRequest {
            request_id: "test-123".to_string(),
        };

        let json = serde_json::to_string(&status_req).unwrap();
        let deserialized: StatusRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.request_id, "test-123");
    }

    #[tokio::test]
    async fn test_certificate_request_serialization() {
        let cert_req = create_test_certificate_request();

        let json = serde_json::to_vec(&cert_req).unwrap();
        let deserialized: CertificateValidationRequest = serde_json::from_slice(&json).unwrap();

        assert_eq!(deserialized.request_id, cert_req.request_id);
        assert_eq!(
            deserialized.certificate_request.common_name,
            cert_req.certificate_request.common_name
        );
    }

    #[tokio::test]
    async fn test_four_proof_request_serialization() {
        let proof_req = create_test_four_proof_request();

        let json = serde_json::to_vec(&proof_req).unwrap();
        let deserialized: FourProofValidationRequest = serde_json::from_slice(&json).unwrap();

        assert_eq!(deserialized.operation, proof_req.operation);
        assert_eq!(deserialized.asset_id, proof_req.asset_id);
        assert_eq!(deserialized.node_id, proof_req.node_id);
    }

    #[tokio::test]
    async fn test_validation_result_serialization() {
        let mock_service = MockValidationService::new();
        let result = mock_service.mock_validate_certificate().await.unwrap();

        let json = serde_json::to_vec(&result).unwrap();
        let deserialized: ValidationResult = serde_json::from_slice(&json).unwrap();

        assert_eq!(deserialized.validator_id, result.validator_id);
        assert!(matches!(deserialized.result, ValidationStatus::Valid));
    }

    #[tokio::test]
    async fn test_byzantine_validation_result() {
        let mock_service = MockValidationService::with_byzantine();
        let result = mock_service.mock_validate_certificate().await.unwrap();

        assert!(matches!(result.result, ValidationStatus::Invalid { .. }));
        assert!(result.details.bft_status.is_byzantine);
        assert!(!result.details.proof_results.space_proof_valid);
    }

    #[tokio::test]
    async fn test_pending_validation_status() {
        let mock_service = MockValidationService::new();
        let result = mock_service.mock_get_validation_status("test").await.unwrap();

        assert!(matches!(result.result, ValidationStatus::Pending { .. }));
        assert!(result.proof_hash.is_none());
    }
}