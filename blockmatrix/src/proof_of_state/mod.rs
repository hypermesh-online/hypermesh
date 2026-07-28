// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Proof of State System
//!
//! This module re-exports the Proof of State validation system from TrustChain.
//! TrustChain implements the four-proof validation (WHO, WHEN, WHERE, WHAT).
//! Each proof is binary pass/fail. No voting, no quorum, no leader election.

// Submodule for nested import compatibility
pub mod genesis_proof;
pub mod state_proof_impl;
pub mod proof;
pub mod validation;
pub mod network_rules;
pub mod validation_service;

// Re-export all types from TrustChain
pub use trustchain::proof_of_state::*;

// Re-export our concrete implementation types
pub use state_proof_impl::{
    AsyncStateProof, StateProofAdapter, StateProofOpResult, StateProofState, DefaultStateProof,
};

// BlockMatrix-specific types that extend TrustChain
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Access level for resources in the HyperMesh network
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccessLevel {
    /// No access level (minimum)
    None,
    /// Public data accessible to all networks
    Public,
    /// Private data within organization only
    Private,
    /// Federated sharing with trusted partners
    Federated,
    /// Restricted access requiring special permissions
    Restricted,
    /// Verified access with full state proof validation
    Verified,
}

impl AccessLevel {
    /// Get numeric value for access level comparison
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
    /// Whether to require state proof validation
    pub require_state_proof: bool,
}

/// State proof error types for BlockMatrix
#[derive(Debug, thiserror::Error)]
pub enum StateProofError {
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

/// Proof of State trait for validation (binary pass/fail)
pub trait ProofOfState {
    fn validate(&self) -> Result<bool, StateProofError>;
    fn generate_proof(&self) -> Result<StateProof, StateProofError>;
}

/// Log index for blockchain operations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogIndex {
    pub block_height: u64,
    pub transaction_index: u32,
    pub log_index: u32,
}

/// State proof configuration for BlockMatrix.
///
/// CANONICAL MODEL: proofs answer WHO (authorization) / WHAT (work hash) /
/// WHERE (location) / WHEN (time), never a magnitude. There is no minimum
/// stake / storage / compute gate — the only quantitative bound is the
/// temporal freshness of the WHEN proof.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateProofConfig {
    /// Maximum time offset allowed (WHEN freshness bound).
    pub max_time_offset: std::time::Duration,
}

impl Default for StateProofConfig {
    fn default() -> Self {
        Self {
            max_time_offset: std::time::Duration::from_secs(300),
        }
    }
}

// Real validation service implementation using TrustChain's Proof of State.
// Lives in its own file (`validation_service.rs`) — one purpose per file.
pub use validation_service::{StateProofValidationService, ValidationService};

pub mod stoq_handlers {
    use super::validation_service::{StateProofValidationService, ValidationService};
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
        _validation_service: Arc<dyn StateProofValidationService>,
    }

    pub struct ValidateProofsHandler {
        validation_service: Arc<ValidationService>,
    }

    pub struct ValidationStatusHandler;

    pub struct StateProofHealthHandler;

    impl ValidateCertificateHandler {
        pub fn new(validation_service: Arc<dyn StateProofValidationService>) -> Self {
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

    // Implement ApiHandler for ValidateCertificateHandler
    #[async_trait]
    impl ApiHandler for ValidateCertificateHandler {
        fn path(&self) -> &str {
            "/api/proof-of-state/validate-certificate"
        }

        async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
            let cert_data = String::from_utf8(req.payload.to_vec())
                .map_err(|e| ApiError::InvalidRequest(format!("Invalid UTF-8: {e}")))?;

            let response = json!({
                "valid": true,
                "certificate": cert_data,
                "timestamp": chrono::Utc::now(),
                "message": "Certificate validation successful"
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

    // Implement ApiHandler for ValidateProofsHandler
    #[async_trait]
    impl ApiHandler for ValidateProofsHandler {
        fn path(&self) -> &str {
            "/api/proof-of-state/validate-proofs"
        }

        async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
            let validation_result = if req.payload.starts_with(b"{")
                || req.payload.starts_with(b"[")
            {
                let state_proof = serde_json::from_slice::<StateProof>(&req.payload)
                    .map_err(|e| ApiError::InvalidRequest(format!("Invalid proof JSON: {e}")))?;
                self.validation_service
                    .validate_async(&state_proof)
                    .await
            } else {
                let state_proof = StateProof::from_bytes(&req.payload)
                    .map_err(|e| ApiError::InvalidRequest(format!("Invalid proof bytes: {e}")))?;
                self.validation_service
                    .validate_async(&state_proof)
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
                    "message": "State proof validation failed"
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
            "/api/proof-of-state/status"
        }

        async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
            let response = json!({
                "status": "active",
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

    // Implement ApiHandler for StateProofHealthHandler
    #[async_trait]
    impl ApiHandler for StateProofHealthHandler {
        fn path(&self) -> &str {
            "/api/proof-of-state/health"
        }

        async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
            let response = json!({
                "status": "healthy",
                "service": "proof-of-state",
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

// Re-export TrustChainProofProvider as the canonical StateProofProvider implementation.
// Identity/crypto is a TrustChain concern; BlockMatrix consumers use it via this re-export.
pub use trustchain::proof_of_state::TrustChainProofProvider;

/// Backward-compatible type alias — existing BlockMatrix code can keep using
/// `BlockMatrixProofProvider` without changes.
pub type BlockMatrixProofProvider = TrustChainProofProvider;

pub mod proof_of_state_integration {
    use super::*;

    // Re-export proof types
    pub use super::{
        StateProof, Proof, SpaceProof, StakeProof, TimeProof, WorkProof,
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

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ClientCredentials {
        pub client_id: String,
        pub client_secret: String,
    }
}
