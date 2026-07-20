// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Proof of State - Bilateral Binary Authentication for TrustChain
//!
//! This module implements the four-proof Proof of State system for
//! TrustChain certificate operations and CT log validation.
//! Each proof is binary pass/fail. No voting, quorum, or leader election.

use anyhow::{anyhow, Result};
use pqcrypto_falcon::falcon1024;
use pqcrypto_traits::sign::{DetachedSignature, PublicKey, SecretKey};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

pub mod asset_integration;
pub mod hypermesh_client;
pub mod proof;
pub mod validation;
pub mod validator;

pub use asset_integration::*;
pub use proof::*;
pub use validation::*;
// Re-export validator types explicitly to avoid ambiguous glob with
// hypermesh_client::ValidationMetrics (different struct, same name).
pub use validator::{
    FourProofValidator, MisbehaviorDetector, MisbehaviorThresholds, MaliciousNodeInfo,
    ProofOfSpaceValidator, ProofOfStakeValidator, ProofOfWorkValidator, ProofOfTimeValidator,
    SecurityConfig, SecurityViolationType, StateAuthenticator, StorageNodeInfo,
    SuspiciousActivity, ValidationMetrics,
};
// Re-export hypermesh_client types explicitly, excluding ValidationMetrics
// (which collides with validator::ValidationMetrics — different struct).
pub use hypermesh_client::{
    ByzantineFaultToleranceStatus, CertificateType, FourProofValidationRequest,
    HyperMeshClientConfig, HyperMeshStateProofClient, PerformanceStatistics,
    ProofValidationResults, StateProofClientMetrics,
    StateProofValidationRequest, StateProofValidationResult, StateProofValidationService,
    StateProofValidationStatus, ValidationContext, ValidationDetails,
};

// The canonical four-proof composite and its validation bounds live in
// `hypermesh_lib`. TrustChain re-exports them so
// `trustchain::proof_of_state::StateProof` keeps resolving for every existing
// call site, and attaches the generation / crypto LOGIC via `StateProofOps`
// below. There is exactly ONE `StateProof` type in the workspace.
pub use hypermesh_lib::proof::{StateProof, StateRequirements};

/// TrustChain's generation + deep-validation logic for the canonical
/// [`StateProof`].
///
/// The TYPE lives in `hypermesh_lib` (single source of truth); the LOGIC that
/// needs hardware assessment, NTP and FALCON-1024 lives here. Implemented as an
/// extension trait because `StateProof` is a foreign type to this crate.
///
/// Pure-data operations (`new`, `validate`, `validate_with_requirements`,
/// `to_bytes`, `from_bytes`, `hash`, `new_for_testing`) remain inherent methods
/// on the lib type and need no import.
#[async_trait::async_trait]
pub trait StateProofOps: Sized {
    /// Generate a real state proof from live node/network state.
    async fn generate_from_network(node_id: &str) -> Result<Self>;

    /// Validate all four proofs, returning a detailed per-proof report.
    fn verify_all(&self) -> Result<validation::ProofValidation>;

    /// Comprehensive validation with detailed error reporting.
    async fn validate_comprehensive(&self) -> Result<bool>;

    /// Validate against a specific asset's proof requirements.
    fn validate_for_asset(
        &self,
        context: &asset_integration::AssetValidationContext,
    ) -> Result<validation::ProofValidation>;

    /// Validate against a WHEN-freshness bound, returning a detailed report.
    fn verify_with_requirements(
        &self,
        max_time_offset: Duration,
    ) -> Result<validation::ProofValidation>;
}

#[async_trait::async_trait]
impl StateProofOps for StateProof {
    async fn generate_from_network(node_id: &str) -> Result<Self> {
        Ok(Self {
            stake_proof: generate_stake_from_network(node_id).await?,
            time_proof: generate_time_with_ntp_sync().await?,
            space_proof: generate_space_from_system(node_id).await?,
            work_proof: generate_work_from_computation(node_id).await?,
        })
    }

    fn verify_all(&self) -> Result<validation::ProofValidation> {
        validation::verify_all_proofs(self)
    }

    async fn validate_comprehensive(&self) -> Result<bool> {
        let validation = self.verify_all()?;

        if !validation.all_valid {
            return Err(anyhow!(
                "State proof validation failed: {}",
                validation.error_summary()
            ));
        }

        Ok(true)
    }

    fn validate_for_asset(
        &self,
        context: &asset_integration::AssetValidationContext,
    ) -> Result<validation::ProofValidation> {
        asset_integration::validate_proof_for_asset(self, context)
    }

    fn verify_with_requirements(
        &self,
        max_time_offset: Duration,
    ) -> Result<validation::ProofValidation> {
        validation::verify_proof_with_requirements(self, max_time_offset)
    }
}

/// State proof validation result (binary pass/fail)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StateProofResult {
    Valid {
        valid: bool,
        validation_timestamp: SystemTime,
        validation_duration: Duration,
    },
    Invalid {
        reason: String,
        failed_proofs: Vec<String>,
        validation_timestamp: SystemTime,
    },
    Pending {
        validation_id: String,
        estimated_completion: SystemTime,
    },
}

impl StateProofResult {
    /// Check if the result is valid
    pub fn is_valid(&self) -> bool {
        matches!(self, StateProofResult::Valid { .. })
    }
}

/// State proof validation context
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateProofContext {
    pub validator_id: String,
    pub network_id: String,
    pub requirements: StateRequirements,
}

impl StateProofContext {
    pub fn new(validator_id: String, network_id: String) -> Self {
        Self {
            validator_id,
            network_id,
            requirements: StateRequirements::default(),
        }
    }

    pub fn localhost_testing(validator_id: String) -> Self {
        Self {
            validator_id,
            network_id: "localhost".to_string(),
            requirements: StateRequirements::localhost_testing(),
        }
    }

    pub fn production(validator_id: String, network_id: String) -> Self {
        Self {
            validator_id,
            network_id,
            requirements: StateRequirements::production(),
        }
    }
}

// Wire format for FALCON-signed state proofs is canonical in hypermesh_lib.
// TrustChain re-exports it; `TrustChainProofProvider` produces/consumes it and
// `SignedStateProof` converts to/from it. The bilateral handshake binds
// `signer_pubkey` to the peer's authenticated identity key (F2 Sybil defense).
pub use hypermesh_lib::proof::WireSignedProof;

/// [`StateProofProvider`] implementation backed by TrustChain's `SignedStateProof`.
///
/// Wraps TrustChain's `StateProof` generation and validation so that
/// STOQ's bilateral handshake can use it via the trait from lib.
///
/// Holds an `Arc<dyn NodeSigner>` so that generated proofs are signed
/// with the node's FALCON-1024 identity key and received proofs have
/// their signatures cryptographically verified.
pub struct TrustChainProofProvider {
    node_id: String,
    signer: Arc<dyn hypermesh_lib::NodeSigner + Send + Sync>,
}

impl TrustChainProofProvider {
    pub fn new(node_id: String, signer: Arc<dyn hypermesh_lib::NodeSigner + Send + Sync>) -> Self {
        Self { node_id, signer }
    }
}

#[async_trait::async_trait]
impl hypermesh_lib::StateProofProvider for TrustChainProofProvider {
    async fn generate_proof(&self) -> anyhow::Result<Vec<u8>> {
        let proof = StateProof::generate_from_network(&self.node_id)
            .await
            .map_err(|e| anyhow!("PoS proof generation failed: {e}"))?;

        // Serialize the inner StateProof as JSON
        let proof_bytes = serde_json::to_vec(&proof)
            .map_err(|e| anyhow!("Failed to serialize state proof: {e}"))?;

        // Generate random nonce for replay prevention
        let mut nonce = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce);

        // Compute BLAKE3(proof_bytes || nonce) and sign with FALCON-1024
        let mut hasher = blake3::Hasher::new();
        hasher.update(&proof_bytes);
        hasher.update(&nonce);
        let digest = hasher.finalize();

        let signature = self.signer.sign(digest.as_bytes())
            .map_err(|e| anyhow!("FALCON signing failed: {e}"))?;

        let wire = WireSignedProof {
            proof_bytes,
            signature,
            signer_pubkey: self.signer.public_key_bytes().to_vec(),
            nonce,
        };

        serde_json::to_vec(&wire)
            .map_err(|e| anyhow!("Failed to serialize WireSignedProof: {e}"))
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
                let pk = falcon1024::PublicKey::from_bytes(&wire.signer_pubkey)
                    .map_err(|e| anyhow!("Invalid FALCON public key in proof: {e}"))?;
                let sig = falcon1024::DetachedSignature::from_bytes(&wire.signature)
                    .map_err(|e| anyhow!("Invalid FALCON signature in proof: {e}"))?;

                if falcon1024::verify_detached_signature(&sig, digest.as_bytes(), &pk).is_err() {
                    tracing::warn!("FALCON signature verification failed on WireSignedProof");
                    return Ok(false);
                }

                // Signature valid — now validate the inner StateProof
                let proof: StateProof = serde_json::from_slice(&wire.proof_bytes)
                    .map_err(|e| anyhow!("Failed to deserialize inner StateProof: {e}"))?;
                Ok(proof.validate())
            }
            Err(e) => {
                // F2 (zero-trust directive): the unsigned-proof fallback is
                // removed. Every proof MUST be a FALCON-signed WireSignedProof.
                // Accepting a raw bincode StateProof with no signature was a
                // downgrade path that let an attacker bypass authentication
                // entirely. On decode failure we REJECT — never fall back to a
                // structural-only check.
                Err(anyhow!(
                    "Rejecting proof: not a valid FALCON-signed WireSignedProof ({e})"
                ))
            }
        }
    }
}

/// A state proof signed with FALCON-1024 for bilateral authentication.
///
/// Wraps a `StateProof` with a post-quantum FALCON-1024 detached signature,
/// the signer's public key, and a replay-prevention nonce.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedStateProof {
    /// The underlying four-proof state proof
    pub proof: StateProof,
    /// FALCON-1024 detached signature over BLAKE3(proof_bytes || nonce)
    pub signature: Vec<u8>,
    /// Signer's FALCON-1024 public key
    pub signer_pubkey: Vec<u8>,
    /// Random nonce to prevent replay attacks
    pub nonce: [u8; 32],
}

impl SignedStateProof {
    /// Create a signed state proof using FALCON-1024 keys.
    pub fn sign(proof: StateProof, secret_key: &[u8], public_key: &[u8]) -> Result<Self> {
        use rand::RngCore;
        let mut nonce = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut nonce);

        let proof_bytes = proof.to_bytes()?;
        let mut message_input = Vec::with_capacity(proof_bytes.len() + 32);
        message_input.extend_from_slice(&proof_bytes);
        message_input.extend_from_slice(&nonce);
        let message_hash = blake3::hash(&message_input);

        let sk = falcon1024::SecretKey::from_bytes(secret_key)
            .map_err(|e| anyhow!("Invalid FALCON-1024 secret key: {e}"))?;
        let sig = falcon1024::detached_sign(message_hash.as_bytes(), &sk);

        Ok(Self {
            proof,
            signature: sig.as_bytes().to_vec(),
            signer_pubkey: public_key.to_vec(),
            nonce,
        })
    }

    /// Verify the FALCON-1024 signature on this proof.
    pub fn verify(&self) -> Result<bool> {
        let proof_bytes = self.proof.to_bytes()?;
        let mut message_input = Vec::with_capacity(proof_bytes.len() + 32);
        message_input.extend_from_slice(&proof_bytes);
        message_input.extend_from_slice(&self.nonce);
        let message_hash = blake3::hash(&message_input);

        let pk = falcon1024::PublicKey::from_bytes(&self.signer_pubkey)
            .map_err(|e| anyhow!("Invalid FALCON-1024 public key: {e}"))?;
        let sig = falcon1024::DetachedSignature::from_bytes(&self.signature)
            .map_err(|e| anyhow!("Invalid FALCON-1024 signature: {e}"))?;

        Ok(falcon1024::verify_detached_signature(&sig, message_hash.as_bytes(), &pk).is_ok())
    }

    /// Derive the signer's node ID (BLAKE3 hex of public key).
    pub fn signer_node_id(&self) -> String {
        blake3::hash(&self.signer_pubkey).to_hex().to_string()
    }

    /// Serialize for network transmission (JSON).
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| anyhow!("Failed to serialize SignedStateProof: {e}"))
    }

    /// Deserialize from network transmission (JSON).
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        serde_json::from_slice(data)
            .map_err(|e| anyhow!("Failed to deserialize SignedStateProof: {e}"))
    }
}

impl From<SignedStateProof> for WireSignedProof {
    /// Convert an in-memory `SignedStateProof` to wire format.
    fn from(signed: SignedStateProof) -> Self {
        let proof_bytes = signed.proof.to_bytes().unwrap_or_default();
        Self {
            proof_bytes,
            signature: signed.signature,
            signer_pubkey: signed.signer_pubkey,
            nonce: signed.nonce,
        }
    }
}

impl TryFrom<WireSignedProof> for SignedStateProof {
    type Error = anyhow::Error;

    /// Convert a wire-format proof to the in-memory deserialized form.
    fn try_from(wire: WireSignedProof) -> Result<Self> {
        let proof = StateProof::from_bytes(&wire.proof_bytes)
            .or_else(|_| {
                serde_json::from_slice::<StateProof>(&wire.proof_bytes)
                    .map_err(|e| anyhow!("Failed to deserialize proof_bytes: {e}"))
            })?;
        Ok(Self {
            proof,
            signature: wire.signature,
            signer_pubkey: wire.signer_pubkey,
            nonce: wire.nonce,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_state_proof_creation() -> anyhow::Result<()> {
        let node_id = "test-node-01";
        let proof = StateProof::generate_from_network(node_id).await?;
        assert!(proof.validate());
        Ok(())
    }

    #[tokio::test]
    async fn test_state_proof_serialization() -> anyhow::Result<()> {
        let node_id = "test-node-01";
        let proof = StateProof::generate_from_network(node_id).await?;
        let bytes = proof.to_bytes()?;
        let deserialized = StateProof::from_bytes(&bytes)?;

        assert_eq!(
            proof.stake_proof.stake_holder_id,
            deserialized.stake_proof.stake_holder_id
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_state_requirements_validation() -> anyhow::Result<()> {
        let node_id = "test-node-01";
        let proof = StateProof::generate_from_network(node_id).await?;
        let requirements = StateRequirements::localhost_testing();

        assert!(proof.validate_with_requirements(&requirements));
        Ok(())
    }

    #[test]
    fn test_new_for_testing_creates_valid_proof() {
        let proof = StateProof::new_for_testing();

        // PoSpace is WHERE (location), never how-much: assert the proof is
        // bound to a location. Capacity is descriptive and must never gate.
        assert!(
            !proof.space_proof.node_id.is_empty(),
            "Space proof (location) must bind a node"
        );
        assert!(
            !proof.space_proof.storage_path.is_empty(),
            "Space proof (location) must bind a storage path"
        );
        assert!(
            !proof.stake_proof.stake_holder_id.is_empty(),
            "Stake proof (authorization) must bind an identity"
        );
        assert!(
            proof.work_proof.work_hash != [0u8; 32],
            "Work proof must carry a real (non-zero) work hash"
        );
        assert!(
            proof.time_proof.nonce > 0,
            "Time proof should have non-zero nonce"
        );

        assert!(proof.validate(), "Test proof should pass validation");
    }

    #[tokio::test]
    async fn test_state_proof_hash() -> anyhow::Result<()> {
        let node_id = "test-node-01";
        let proof = StateProof::generate_from_network(node_id).await?;
        let hash1 = proof.hash()?;
        let hash2 = proof.hash()?;

        assert_eq!(hash1, hash2);
        Ok(())
    }

    #[test]
    fn test_signed_state_proof_roundtrip() {
        let proof = StateProof::new_for_testing();
        let (pk, sk) = falcon1024::keypair();

        let signed = SignedStateProof::sign(
            proof,
            sk.as_bytes(),
            pk.as_bytes(),
        )
        .expect("test: signing should succeed");

        // Verify signature
        assert!(
            signed.verify().expect("test: verify should not error"),
            "FALCON-1024 signature should verify"
        );

        // Verify signer node ID is deterministic
        let node_id_1 = signed.signer_node_id();
        let node_id_2 = signed.signer_node_id();
        assert_eq!(node_id_1, node_id_2);
        assert!(!node_id_1.is_empty());

        // Serialize and deserialize
        let bytes = signed.to_bytes().expect("test: serialization");
        let deserialized =
            SignedStateProof::from_bytes(&bytes).expect("test: deserialization");
        assert!(
            deserialized.verify().expect("test: verify after deser"),
            "Signature should verify after deserialization"
        );
    }

    #[test]
    fn test_signed_state_proof_wrong_key_fails() {
        let proof = StateProof::new_for_testing();
        let (_pk1, sk1) = falcon1024::keypair();
        let (pk2, _sk2) = falcon1024::keypair();

        // Sign with sk1 but claim pk2
        let mut signed = SignedStateProof::sign(
            proof,
            sk1.as_bytes(),
            pk2.as_bytes(),
        )
        .expect("test: signing should succeed");

        // Verification should fail (wrong key)
        assert!(
            !signed.verify().expect("test: verify should not error"),
            "Signature should fail with wrong public key"
        );

        // Also test tampered nonce
        let (pk3, sk3) = falcon1024::keypair();
        signed = SignedStateProof::sign(
            StateProof::new_for_testing(),
            sk3.as_bytes(),
            pk3.as_bytes(),
        )
        .expect("test: signing");
        signed.nonce[0] ^= 0xFF;
        assert!(
            !signed.verify().expect("test: verify should not error"),
            "Signature should fail with tampered nonce"
        );
    }
}

/// Minimal signer for testing that uses real FALCON-1024 keys.
#[cfg(test)]
struct TestNodeSigner {
    public_key: Vec<u8>,
    secret_key: Vec<u8>,
    node_id: String,
}

#[cfg(test)]
impl TestNodeSigner {
    fn new() -> Self {
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

#[cfg(test)]
impl hypermesh_lib::NodeSigner for TestNodeSigner {
    fn node_id(&self) -> &str {
        &self.node_id
    }

    fn public_key_bytes(&self) -> &[u8] {
        &self.public_key
    }

    fn sign(&self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        let sk = falcon1024::SecretKey::from_bytes(&self.secret_key)
            .map_err(|e| anyhow!("Invalid FALCON secret key: {e}"))?;
        let sig = falcon1024::detached_sign(data, &sk);
        Ok(sig.as_bytes().to_vec())
    }

    fn verify_signature(pubkey: &[u8], data: &[u8], signature: &[u8]) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        let pk = falcon1024::PublicKey::from_bytes(pubkey)
            .map_err(|e| anyhow!("Invalid FALCON public key: {e}"))?;
        let sig = falcon1024::DetachedSignature::from_bytes(signature)
            .map_err(|e| anyhow!("Invalid FALCON signature: {e}"))?;
        Ok(falcon1024::verify_detached_signature(&sig, data, &pk).is_ok())
    }
}

#[cfg(test)]
mod proof_provider_tests {
    use super::*;
    use hypermesh_lib::{NodeSigner, StateProofProvider};

    #[tokio::test]
    async fn test_proof_provider_sign_verify_roundtrip() {
        let signer = Arc::new(TestNodeSigner::new());
        let provider = TrustChainProofProvider::new(
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
    async fn test_proof_provider_tampered_signature_fails() {
        let signer = Arc::new(TestNodeSigner::new());
        let provider = TrustChainProofProvider::new(
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
    async fn test_proof_provider_rejects_legacy_unsigned_proof() {
        // F2: the unsigned-proof downgrade path is removed. A raw bincode
        // StateProof (no signature) must be REJECTED, not accepted after a
        // structural-only check.
        let signer = Arc::new(TestNodeSigner::new());
        let provider = TrustChainProofProvider::new(
            signer.node_id().to_string(),
            signer.clone(),
        );

        // Create a raw StateProof in bincode (legacy format)
        let proof = StateProof::new_for_testing();
        let raw_bytes = proof.to_bytes()
            .expect("test: serialization should succeed");

        // validate_proof must now ERROR (reject) — no silent structural pass.
        let result = provider.validate_proof(&raw_bytes).await;
        assert!(
            result.is_err(),
            "Legacy unsigned proof must be rejected (F2), got: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_wire_signed_proof_signer_binding_helpers() {
        // F2: the WireSignedProof exposes its signer key so the handshake can
        // enforce that the proof signer == the authenticated peer identity.
        let signer = Arc::new(TestNodeSigner::new());
        let provider = TrustChainProofProvider::new(
            signer.node_id().to_string(),
            signer.clone(),
        );

        let proof_bytes = provider.generate_proof().await
            .expect("test: proof generation should succeed");
        let wire: WireSignedProof = serde_json::from_slice(&proof_bytes)
            .expect("test: should deserialize WireSignedProof");

        // The signer key equals the node's own FALCON pubkey.
        assert_eq!(
            wire.signer_pubkey_bytes(),
            signer.public_key_bytes(),
            "signer_pubkey_bytes must return the signing key"
        );
        assert!(
            wire.signer_matches(signer.public_key_bytes()),
            "signer_matches must be true for the actual signer"
        );

        // A different key must NOT match.
        let other = TestNodeSigner::new();
        assert!(
            !wire.signer_matches(other.public_key_bytes()),
            "signer_matches must be false for a different key (Sybil vector)"
        );
    }
}
