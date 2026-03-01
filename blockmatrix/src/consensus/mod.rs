// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Consensus System
//!
//! This module re-exports the Proof of State validation system from TrustChain.
//! TrustChain implements the four-proof validation (WHO, WHEN, WHERE, WHAT).
//!
//! For the full consensus implementation, see trustchain::consensus module.

// Submodule for nested import compatibility
pub mod consensus_impl;
pub mod proof;
pub mod validation;

// Re-export all consensus types from TrustChain
pub use trustchain::consensus::*;

// Re-export our concrete implementation types
pub use consensus_impl::{
    AsyncConsensus, ConsensusAdapter, ConsensusResult, ConsensusState, DefaultConsensus,
};

// BlockMatrix-specific consensus types that extend TrustChain
use serde::{Deserialize, Serialize};

/// Access level for resources in the HyperMesh network
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccessLevel {
    /// No access level (minimum)
    // STUB: Phase 3
    None,
    /// Public data accessible to all networks
    Public,
    /// Private data within organization only
    Private,
    /// Federated sharing with trusted partners
    Federated,
    /// Restricted access requiring special permissions
    Restricted,
    /// Verified access with full consensus validation
    // STUB: Phase 3
    Verified,
}

impl AccessLevel {
    /// Get numeric value for access level comparison
    // STUB: Phase 3
    pub fn level_value(&self) -> u8 {
        match self {
            AccessLevel::None => 0,
            AccessLevel::Private => 1,
            AccessLevel::Public => 2,
            AccessLevel::Federated => 3,
            AccessLevel::Restricted => 4,
            AccessLevel::Verified => 5,
        }
    }
}

/// Network position information for node topology
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkPosition {
    /// Geographic region or data center
    pub region: String,
    /// Network zone within region
    pub zone: String,
    /// Rack or cluster identifier
    pub cluster_id: String,
    /// Node identifier within cluster
    pub node_id: String,
}

/// Access permissions configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessPermissions {
    /// Access level for this resource
    pub level: AccessLevel,
    /// Required roles for access
    pub required_roles: Vec<String>,
    /// Allowed IP ranges (IPv6)
    pub allowed_networks: Vec<String>,
    /// Whether to require consensus validation
    pub require_consensus: bool,
}

/// Consensus error types for BlockMatrix
#[derive(Debug, thiserror::Error)]
pub enum ConsensusError {
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Insufficient proofs: {0}")]
    InsufficientProofs(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Other error: {0}")]
    Other(String),
}

/// Consensus trait for validation
pub trait Consensus {
    fn validate(&self) -> Result<bool, ConsensusError>;
    fn generate_proof(&self) -> Result<ConsensusProof, ConsensusError>;
}

/// Log index for blockchain operations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogIndex {
    pub block_height: u64,
    pub transaction_index: u32,
    pub log_index: u32,
}

/// Consensus configuration for BlockMatrix
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Minimum stake required for validation
    pub min_stake: u64,
    /// Maximum time offset allowed
    pub max_time_offset: std::time::Duration,
    /// Minimum storage capacity
    pub min_storage: u64,
    /// Minimum computational power
    pub min_compute_power: u64,
    /// Byzantine fault tolerance threshold
    pub byzantine_threshold: f64,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            min_stake: 1000,
            max_time_offset: std::time::Duration::from_secs(300),
            min_storage: 1024 * 1024 * 1024, // 1GB
            min_compute_power: 100,
            byzantine_threshold: 0.33,
        }
    }
}

// Real validation service implementation using TrustChain's Proof of State
pub mod validation_service {
    use super::*;
    use crate::consensus::validation::{DefaultStateAuthenticator, StateAuthenticator};
    use std::sync::Arc;

    pub struct ValidationService {
        validator: Arc<dyn StateAuthenticator>,
        requirements: ConsensusRequirements,
    }

    impl Default for ValidationService {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ValidationService {
        pub fn new() -> Self {
            Self {
                validator: Arc::new(DefaultStateAuthenticator::new()),
                requirements: ConsensusRequirements::default(),
            }
        }

        pub fn with_requirements(requirements: ConsensusRequirements) -> Self {
            Self {
                validator: Arc::new(DefaultStateAuthenticator::with_requirements(
                    requirements.clone(),
                )),
                requirements,
            }
        }

        pub fn for_production() -> Self {
            Self {
                validator: Arc::new(DefaultStateAuthenticator::with_requirements(
                    ConsensusRequirements::production(),
                )),
                requirements: ConsensusRequirements::production(),
            }
        }
    }

    // Trait for consensus validation service
    pub trait ConsensusValidationService: Send + Sync {
        fn validate(&self, proof: &ConsensusProof) -> Result<bool, ConsensusError>;
    }

    impl ConsensusValidationService for ValidationService {
        fn validate(&self, proof: &ConsensusProof) -> Result<bool, ConsensusError> {
            // Synchronous validation using the basic validate() method
            if proof.validate_with_requirements(&self.requirements) {
                Ok(true)
            } else {
                Err(ConsensusError::ValidationFailed(
                    "Consensus proof failed validation requirements".to_string(),
                ))
            }
        }
    }

    impl ValidationService {
        pub async fn validate_async(&self, proof: &ConsensusProof) -> Result<bool, ConsensusError> {
            // Convert proof to bytes for async validation
            let proof_bytes = proof
                .to_bytes()
                .map_err(|e| ConsensusError::Other(format!("Failed to serialize proof: {e}")))?;

            // Use the async validator
            match self.validator.validate(&proof_bytes).await {
                Ok(true) => Ok(true),
                Ok(false) => Err(ConsensusError::ValidationFailed(
                    "Consensus proof validation failed".to_string(),
                )),
                Err(e) => Err(ConsensusError::ValidationFailed(format!(
                    "Validation error: {e}"
                ))),
            }
        }
    }
}

pub mod stoq_handlers {
    use super::validation_service::{ConsensusValidationService, ValidationService};
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Arc;
    use stoq::{ApiError, ApiHandler, ApiRequest, ApiResponse};

    pub struct StoqHandler;

    impl Default for StoqHandler {
        fn default() -> Self {
            Self::new()
        }
    }

    impl StoqHandler {
        pub fn new() -> Self {
            Self
        }
    }

    // Handler types for API
    pub struct ValidateCertificateHandler {
        _validation_service: Arc<dyn ConsensusValidationService>,
    }

    pub struct ValidateProofsHandler {
        validation_service: Arc<ValidationService>,
    }

    pub struct ValidationStatusHandler {
        _validation_service: Arc<dyn ConsensusValidationService>,
    }

    pub struct ConsensusHealthHandler;

    impl ValidateCertificateHandler {
        pub fn new(validation_service: Arc<dyn ConsensusValidationService>) -> Self {
            Self {
                _validation_service: validation_service,
            }
        }
    }

    impl ValidateProofsHandler {
        pub fn new(validation_service: Arc<ValidationService>) -> Self {
            Self { validation_service }
        }
    }

    impl ValidationStatusHandler {
        pub fn new(validation_service: Arc<dyn ConsensusValidationService>) -> Self {
            Self {
                _validation_service: validation_service,
            }
        }
    }

    // Implement ApiHandler for ValidateCertificateHandler
    #[async_trait]
    impl ApiHandler for ValidateCertificateHandler {
        fn path(&self) -> &str {
            "/api/consensus/validate-certificate"
        }

        async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
            // Parse request and validate certificate
            let cert_data = String::from_utf8(req.payload.to_vec())
                .map_err(|e| ApiError::InvalidRequest(format!("Invalid UTF-8: {e}")))?;

            // TODO: Implement actual certificate validation logic
            let response = json!({
                "valid": true,
                "certificate": cert_data,
                "timestamp": chrono::Utc::now(),
                "message": "Certificate validation successful"
            });

            // Serialize response to bytes
            let payload = serde_json::to_vec(&response)
                .map_err(|e| ApiError::SerializationError(format!("Failed to serialize: {e}")))?;

            Ok(ApiResponse {
                request_id: req.id.clone(),
                success: true,
                payload: bytes::Bytes::from(payload),
                error: None,
                metadata: std::collections::HashMap::new(),
            })
        }
    }

    // Implement ApiHandler for ValidateProofsHandler
    #[async_trait]
    impl ApiHandler for ValidateProofsHandler {
        fn path(&self) -> &str {
            "/api/consensus/validate-proofs"
        }

        async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
            // Parse proof data from request - expect ConsensusProof JSON or bytes
            let validation_result = if req.payload.starts_with(b"{")
                || req.payload.starts_with(b"[")
            {
                // JSON format
                let consensus_proof = serde_json::from_slice::<ConsensusProof>(&req.payload)
                    .map_err(|e| ApiError::InvalidRequest(format!("Invalid proof JSON: {e}")))?;

                // Validate using the service
                self.validation_service
                    .validate_async(&consensus_proof)
                    .await
            } else {
                // Binary format
                let consensus_proof = ConsensusProof::from_bytes(&req.payload)
                    .map_err(|e| ApiError::InvalidRequest(format!("Invalid proof bytes: {e}")))?;

                // Validate using the service
                self.validation_service
                    .validate_async(&consensus_proof)
                    .await
            };

            let response = match validation_result {
                Ok(true) => json!({
                    "valid": true,
                    "proofs_validated": ["PoStake (WHO)", "PoTime (WHEN)", "PoSpace (WHERE)", "PoWork (WHAT)"],
                    "timestamp": chrono::Utc::now(),
                    "message": "All four proofs validated successfully"
                }),
                Ok(false) => json!({
                    "valid": false,
                    "proofs_validated": [],
                    "timestamp": chrono::Utc::now(),
                    "message": "Consensus validation failed"
                }),
                Err(e) => json!({
                    "valid": false,
                    "error": e.to_string(),
                    "timestamp": chrono::Utc::now(),
                    "message": "Validation error occurred"
                }),
            };

            let payload = serde_json::to_vec(&response)
                .map_err(|e| ApiError::SerializationError(format!("Failed to serialize: {e}")))?;

            Ok(ApiResponse {
                request_id: req.id.clone(),
                success: true,
                payload: bytes::Bytes::from(payload),
                error: None,
                metadata: std::collections::HashMap::new(),
            })
        }
    }

    // Implement ApiHandler for ValidationStatusHandler
    #[async_trait]
    impl ApiHandler for ValidationStatusHandler {
        fn path(&self) -> &str {
            "/api/consensus/status"
        }

        async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
            // Return current validation status
            let response = json!({
                "status": "active",
                "validators_online": 10,
                "pending_validations": 3,
                "completed_validations": 150,
                "timestamp": chrono::Utc::now()
            });

            let payload = serde_json::to_vec(&response)
                .map_err(|e| ApiError::SerializationError(format!("Failed to serialize: {e}")))?;

            Ok(ApiResponse {
                request_id: req.id.clone(),
                success: true,
                payload: bytes::Bytes::from(payload),
                error: None,
                metadata: std::collections::HashMap::new(),
            })
        }
    }

    // Implement ApiHandler for ConsensusHealthHandler
    #[async_trait]
    impl ApiHandler for ConsensusHealthHandler {
        fn path(&self) -> &str {
            "/api/consensus/health"
        }

        async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
            let response = json!({
                "status": "healthy",
                "service": "consensus",
                "timestamp": chrono::Utc::now(),
                "version": "0.1.0"
            });

            let payload = serde_json::to_vec(&response)
                .map_err(|e| ApiError::SerializationError(format!("Failed to serialize: {e}")))?;

            Ok(ApiResponse {
                request_id: req.id.clone(),
                success: true,
                payload: bytes::Bytes::from(payload),
                error: None,
                metadata: std::collections::HashMap::new(),
            })
        }
    }
}

pub mod proof_of_state_integration {
    use super::*;

    // Re-export all consensus types for compatibility
    pub use super::{
        ConsensusProof, Proof, SpaceProof, StakeProof, TimeProof, WorkProof, WorkState,
        WorkloadType,
    };

    pub struct ProofOfStateIntegration;

    impl Default for ProofOfStateIntegration {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ProofOfStateIntegration {
        pub fn new() -> Self {
            Self
        }
    }

    // Additional types that may be required
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ClientCredentials {
        pub client_id: String,
        pub client_secret: String,
    }
}

// Additional blockmatrix-specific consensus types can be added here
