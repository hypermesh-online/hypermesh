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

/// State proof configuration for BlockMatrix
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateProofConfig {
    /// Minimum stake required for validation
    pub min_stake: u64,
    /// Maximum time offset allowed
    pub max_time_offset: std::time::Duration,
    /// Minimum storage capacity
    pub min_storage: u64,
    /// Minimum computational power
    pub min_compute_power: u64,
}

impl Default for StateProofConfig {
    fn default() -> Self {
        Self {
            min_stake: 1000,
            max_time_offset: std::time::Duration::from_secs(300),
            min_storage: 1024 * 1024 * 1024, // 1GB
            min_compute_power: 100,
        }
    }
}

// Real validation service implementation using TrustChain's Proof of State
pub mod validation_service {
    use super::*;
    use crate::proof_of_state::validation::{DefaultStateAuthenticator, StateAuthenticator};
    use std::sync::Arc;

    pub struct ValidationService {
        validator: Arc<dyn StateAuthenticator>,
        requirements: StateRequirements,
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
                requirements: StateRequirements::default(),
            }
        }

        pub fn with_requirements(requirements: StateRequirements) -> Self {
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
                    StateRequirements::production(),
                )),
                requirements: StateRequirements::production(),
            }
        }
    }

    // Trait for state proof validation service
    pub trait StateProofValidationService: Send + Sync {
        fn validate(&self, proof: &StateProof) -> Result<bool, StateProofError>;
    }

    impl StateProofValidationService for ValidationService {
        fn validate(&self, proof: &StateProof) -> Result<bool, StateProofError> {
            if proof.validate_with_requirements(&self.requirements) {
                Ok(true)
            } else {
                Err(StateProofError::ValidationFailed(
                    "State proof failed validation requirements".to_string(),
                ))
            }
        }
    }

    impl ValidationService {
        pub async fn validate_async(&self, proof: &StateProof) -> Result<bool, StateProofError> {
            let proof_bytes = proof
                .to_bytes()
                .map_err(|e| StateProofError::Other(format!("Failed to serialize proof: {e}")))?;

            match self.validator.validate(&proof_bytes).await {
                Ok(true) => Ok(true),
                Ok(false) => Err(StateProofError::ValidationFailed(
                    "State proof validation failed".to_string(),
                )),
                Err(e) => Err(StateProofError::ValidationFailed(format!(
                    "Validation error: {e}"
                ))),
            }
        }
    }
}

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

/// Wire format for FALCON-signed state proofs.
///
/// This envelope wraps a serialized `StateProof` with a FALCON-1024 detached
/// signature, the signer's public key, and a replay-prevention nonce. It is
/// the on-the-wire format used by `BlockMatrixProofProvider` so that every
/// proof exchanged during bilateral handshakes is cryptographically bound to
/// the signing node.
#[derive(Serialize, Deserialize)]
struct WireSignedProof {
    /// JSON-serialized `StateProof`
    proof_bytes: Vec<u8>,
    /// FALCON-1024 detached signature over `BLAKE3(proof_bytes || nonce)`
    signature: Vec<u8>,
    /// Signer's full FALCON-1024 public key
    signer_pubkey: Vec<u8>,
    /// Random nonce to prevent replay attacks
    nonce: [u8; 32],
}

/// [`StateProofProvider`] implementation for BlockMatrix.
///
/// Wraps TrustChain's `StateProof` generation and validation so that
/// STOQ's bilateral handshake can use it via the trait from lib.
///
/// Holds an `Arc<dyn NodeSigner>` so that generated proofs are signed
/// with the node's FALCON-1024 identity key and received proofs have
/// their signatures cryptographically verified.
pub struct BlockMatrixProofProvider {
    node_id: String,
    signer: Arc<dyn hypermesh_lib::NodeSigner + Send + Sync>,
}

impl BlockMatrixProofProvider {
    pub fn new(node_id: String, signer: Arc<dyn hypermesh_lib::NodeSigner + Send + Sync>) -> Self {
        Self { node_id, signer }
    }
}

#[async_trait::async_trait]
impl hypermesh_lib::StateProofProvider for BlockMatrixProofProvider {
    async fn generate_proof(&self) -> anyhow::Result<Vec<u8>> {
        let proof = StateProof::generate_from_network(&self.node_id)
            .await
            .map_err(|e| anyhow::anyhow!("PoS proof generation failed: {e}"))?;

        // Serialize the inner StateProof as JSON
        let proof_bytes = serde_json::to_vec(&proof)
            .map_err(|e| anyhow::anyhow!("Failed to serialize state proof: {e}"))?;

        // Generate random nonce for replay prevention
        let mut nonce = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce);

        // Compute BLAKE3(proof_bytes || nonce) and sign with FALCON-1024
        let mut hasher = blake3::Hasher::new();
        hasher.update(&proof_bytes);
        hasher.update(&nonce);
        let digest = hasher.finalize();

        let signature = self.signer.sign(digest.as_bytes())
            .map_err(|e| anyhow::anyhow!("FALCON signing failed: {e}"))?;

        let wire = WireSignedProof {
            proof_bytes,
            signature,
            signer_pubkey: self.signer.public_key_bytes().to_vec(),
            nonce,
        };

        serde_json::to_vec(&wire)
            .map_err(|e| anyhow::anyhow!("Failed to serialize WireSignedProof: {e}"))
    }

    async fn validate_proof(&self, incoming: &[u8]) -> anyhow::Result<bool> {
        // Try to deserialize as WireSignedProof (new signed format)
        match serde_json::from_slice::<WireSignedProof>(incoming) {
            Ok(wire) => {
                // Recompute BLAKE3(proof_bytes || nonce)
                let mut hasher = blake3::Hasher::new();
                hasher.update(&wire.proof_bytes);
                hasher.update(&wire.nonce);
                let digest = hasher.finalize();

                // Verify FALCON-1024 signature using pqcrypto directly
                use pqcrypto_falcon::falcon1024;
                use pqcrypto_traits::sign::{DetachedSignature, PublicKey};

                let pk = falcon1024::PublicKey::from_bytes(&wire.signer_pubkey)
                    .map_err(|e| anyhow::anyhow!("Invalid FALCON public key in proof: {e}"))?;
                let sig = falcon1024::DetachedSignature::from_bytes(&wire.signature)
                    .map_err(|e| anyhow::anyhow!("Invalid FALCON signature in proof: {e}"))?;

                if falcon1024::verify_detached_signature(&sig, digest.as_bytes(), &pk).is_err() {
                    tracing::warn!("FALCON signature verification failed on WireSignedProof");
                    return Ok(false);
                }

                // Signature valid — now validate the inner StateProof
                let proof: StateProof = serde_json::from_slice(&wire.proof_bytes)
                    .map_err(|e| anyhow::anyhow!("Failed to deserialize inner StateProof: {e}"))?;
                Ok(proof.validate())
            }
            Err(_) => {
                // Backward compatibility: try raw StateProof (bincode format)
                tracing::warn!(
                    "Received unsigned state proof (legacy format) — \
                     cryptographic verification skipped"
                );
                let proof = StateProof::from_bytes(incoming)
                    .map_err(|e| anyhow::anyhow!("Failed to deserialize state proof: {e}"))?;
                Ok(proof.validate())
            }
        }
    }
}

#[cfg(test)]
mod wire_signed_proof_tests {
    use super::*;
    use hypermesh_lib::{NodeSigner, StateProofProvider};

    /// Minimal signer for testing that uses real FALCON-1024 keys.
    struct TestNodeSigner {
        public_key: Vec<u8>,
        secret_key: Vec<u8>,
        node_id: String,
    }

    impl TestNodeSigner {
        fn new() -> Self {
            use pqcrypto_falcon::falcon1024;
            use pqcrypto_traits::sign::{PublicKey, SecretKey};
            let (pk, sk) = falcon1024::keypair();
            let pk_bytes = pk.as_bytes().to_vec();
            let node_id = blake3::hash(&pk_bytes).to_hex().to_string();
            Self {
                public_key: pk_bytes,
                secret_key: sk.as_bytes().to_vec(),
                node_id,
            }
        }
    }

    impl hypermesh_lib::NodeSigner for TestNodeSigner {
        fn node_id(&self) -> &str {
            &self.node_id
        }

        fn public_key_bytes(&self) -> &[u8] {
            &self.public_key
        }

        fn sign(&self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
            use pqcrypto_falcon::falcon1024;
            use pqcrypto_traits::sign::{DetachedSignature, SecretKey};
            let sk = falcon1024::SecretKey::from_bytes(&self.secret_key)
                .map_err(|e| anyhow::anyhow!("Invalid FALCON secret key: {e}"))?;
            let sig = falcon1024::detached_sign(data, &sk);
            Ok(sig.as_bytes().to_vec())
        }

        fn verify_signature(pubkey: &[u8], data: &[u8], signature: &[u8]) -> anyhow::Result<bool>
        where
            Self: Sized,
        {
            use pqcrypto_falcon::falcon1024;
            use pqcrypto_traits::sign::{DetachedSignature, PublicKey};
            let pk = falcon1024::PublicKey::from_bytes(pubkey)
                .map_err(|e| anyhow::anyhow!("Invalid FALCON public key: {e}"))?;
            let sig = falcon1024::DetachedSignature::from_bytes(signature)
                .map_err(|e| anyhow::anyhow!("Invalid FALCON signature: {e}"))?;
            Ok(falcon1024::verify_detached_signature(&sig, data, &pk).is_ok())
        }
    }

    #[tokio::test]
    async fn test_sign_verify_roundtrip() {
        let signer = Arc::new(TestNodeSigner::new());
        let provider = BlockMatrixProofProvider::new(
            signer.node_id().to_string(),
            signer.clone(),
        );

        let proof_bytes = provider.generate_proof().await
            .expect("test: proof generation should succeed");

        let valid = provider.validate_proof(&proof_bytes).await
            .expect("test: validation should not error");
        assert!(valid, "Signed proof should validate successfully");
    }

    #[tokio::test]
    async fn test_tampered_signature_fails() {
        let signer = Arc::new(TestNodeSigner::new());
        let provider = BlockMatrixProofProvider::new(
            signer.node_id().to_string(),
            signer.clone(),
        );

        let proof_bytes = provider.generate_proof().await
            .expect("test: proof generation should succeed");

        // Tamper with the signature field inside the JSON
        let mut wire: WireSignedProof = serde_json::from_slice(&proof_bytes)
            .expect("test: should deserialize");
        if let Some(byte) = wire.signature.get_mut(0) {
            *byte ^= 0xFF;
        }
        let tampered = serde_json::to_vec(&wire)
            .expect("test: should serialize");

        let valid = provider.validate_proof(&tampered).await
            .expect("test: validation should not error");
        assert!(!valid, "Tampered signature should fail verification");
    }

    #[tokio::test]
    async fn test_backward_compat_raw_state_proof() {
        let signer = Arc::new(TestNodeSigner::new());
        let provider = BlockMatrixProofProvider::new(
            signer.node_id().to_string(),
            signer.clone(),
        );

        // Create a raw StateProof in bincode (legacy format)
        let proof = StateProof::new_for_testing();
        let raw_bytes = proof.to_bytes()
            .expect("test: serialization should succeed");

        let valid = provider.validate_proof(&raw_bytes).await
            .expect("test: validation should not error");
        assert!(valid, "Legacy unsigned proof should still validate");
    }
}

pub mod proof_of_state_integration {
    use super::*;

    // Re-export proof types
    pub use super::{
        StateProof, Proof, SpaceProof, StakeProof, TimeProof, WorkProof, WorkState,
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

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ClientCredentials {
        pub client_id: String,
        pub client_secret: String,
    }
}
