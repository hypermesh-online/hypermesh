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
pub use hypermesh_client::*;
pub use proof::*;
pub use validation::*;
pub use validator::*;

/// Proof of State Four-Proof Authentication
///
/// Bilateral binary pass/fail authentication. Each proof answers one question:
/// - WHO owns/validates (PoStake)
/// - WHEN it occurred (PoTime)
/// - WHERE it's stored (PoSpace)
/// - WHAT computational work (PoWork)
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct StateProof {
    /// WHO owns/validates (economic security)
    pub stake_proof: StakeProof,
    /// WHEN it occurred (temporal ordering)
    pub time_proof: TimeProof,
    /// WHERE it's stored (storage commitment)
    pub space_proof: SpaceProof,
    /// WHAT computational work (resource proof)
    pub work_proof: WorkProof,
}

impl StateProof {
    /// Create a new state proof with all four proofs
    pub fn new(
        stake_proof: StakeProof,
        time_proof: TimeProof,
        space_proof: SpaceProof,
        work_proof: WorkProof,
    ) -> Self {
        Self {
            stake_proof,
            time_proof,
            space_proof,
            work_proof,
        }
    }

    /// Generate real state proof from network state
    pub async fn generate_from_network(node_id: &str) -> Result<Self> {
        let stake_proof = StakeProof::generate_from_network(node_id).await?;
        let time_proof = TimeProof::generate_with_ntp_sync().await?;
        let space_proof = SpaceProof::generate_from_system(node_id).await?;
        let work_proof = WorkProof::generate_from_computation(node_id).await?;

        Ok(Self {
            stake_proof,
            time_proof,
            space_proof,
            work_proof,
        })
    }

    /// TEST-ONLY: Create a valid test proof
    #[cfg(test)]
    pub fn default_for_testing() -> Self {
        Self::new_for_testing()
    }

    /// Create a testing proof -- only available in test builds or with localhost-testing feature
    #[cfg(any(test, feature = "localhost-testing"))]
    pub fn new_for_testing() -> Self {
        let mut space_proof = SpaceProof::new(
            "test_node_001".to_string(),
            "test_storage_path".to_string(),
            100 * 1024 * 1024 * 1024, // 100GB total_storage
        );
        space_proof.total_size = 50 * 1024 * 1024 * 1024;
        space_proof.file_hash =
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2".to_string();

        Self {
            stake_proof: StakeProof::new(
                "test_stake_holder".to_string(),
                "test_node_001".to_string(),
                10000,
            ),
            time_proof: TimeProof::new(Duration::from_secs(1)),
            space_proof,
            work_proof: WorkProof::new(
                "test_owner".to_string(),
                "test_workload_001".to_string(),
                1234,
                1000,
                WorkloadType::Compute,
                WorkState::Running,
            ),
        }
    }

    /// Validate all four proofs (binary pass/fail)
    pub fn validate(&self) -> bool {
        self.stake_proof.validate()
            && self.time_proof.validate()
            && self.space_proof.validate()
            && self.work_proof.validate()
    }

    /// Comprehensive validation with detailed error reporting
    pub async fn validate_comprehensive(&self) -> Result<bool> {
        let validation = self.verify_all()?;

        if !validation.all_valid {
            return Err(anyhow!(
                "State proof validation failed: {}",
                validation.error_summary()
            ));
        }

        Ok(true)
    }

    /// Validate with specific requirements
    pub fn validate_with_requirements(&self, requirements: &StateRequirements) -> bool {
        if self.stake_proof.stake_amount < requirements.minimum_stake {
            return false;
        }
        if self.time_proof.network_time_offset > requirements.max_time_offset {
            return false;
        }
        if self.space_proof.total_storage < requirements.minimum_storage {
            return false;
        }
        if self.work_proof.computational_power < requirements.minimum_compute {
            return false;
        }
        self.validate()
    }

    /// Serialize for network transmission
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow!("Failed to serialize StateProof: {e}"))
    }

    /// Deserialize from network transmission
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        bincode::deserialize(data).map_err(|e| anyhow!("Failed to deserialize StateProof: {e}"))
    }

    /// Generate cryptographic hash (BLAKE3)
    pub fn hash(&self) -> Result<[u8; 32]> {
        let bytes = self.to_bytes()?;
        Ok(*blake3::hash(&bytes).as_bytes())
    }
}

/// Requirements for state proof validation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateRequirements {
    /// Minimum stake amount for validation
    pub minimum_stake: u64,
    /// Maximum time offset for synchronization
    pub max_time_offset: Duration,
    /// Minimum storage commitment
    pub minimum_storage: u64,
    /// Minimum computational power
    pub minimum_compute: u64,
}

impl Default for StateRequirements {
    fn default() -> Self {
        Self {
            minimum_stake: 5000,
            max_time_offset: Duration::from_secs(60),
            minimum_storage: 1024 * 1024 * 1024, // 1GB
            minimum_compute: 1000,
        }
    }
}

impl StateRequirements {
    pub fn production() -> Self {
        Self {
            minimum_stake: 50000,
            max_time_offset: Duration::from_secs(30),
            minimum_storage: 10 * 1024 * 1024 * 1024, // 10GB
            minimum_compute: 10000,
        }
    }

    pub fn localhost_testing() -> Self {
        Self {
            minimum_stake: 100,
            max_time_offset: Duration::from_secs(300),
            minimum_storage: 1024 * 1024, // 1MB
            minimum_compute: 10,
        }
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

/// Wire format for FALCON-signed state proofs used by `TrustChainProofProvider`.
///
/// This envelope wraps a serialized `StateProof` with a FALCON-1024 detached
/// signature, the signer's public key, and a replay-prevention nonce. It is
/// the on-the-wire format so that every proof exchanged during bilateral
/// handshakes is cryptographically bound to the signing node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireSignedProof {
    /// JSON-serialized `StateProof`
    pub proof_bytes: Vec<u8>,
    /// FALCON-1024 detached signature over `BLAKE3(proof_bytes || nonce)`
    pub signature: Vec<u8>,
    /// Signer's full FALCON-1024 public key
    pub signer_pubkey: Vec<u8>,
    /// Random nonce to prevent replay attacks
    pub nonce: [u8; 32],
}

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
            Err(_) => {
                // Backward compatibility: try raw StateProof (bincode format)
                tracing::warn!(
                    "Received unsigned state proof (legacy format) — \
                     cryptographic verification skipped"
                );
                let proof = StateProof::from_bytes(incoming)
                    .map_err(|e| anyhow!("Failed to deserialize state proof: {e}"))?;
                Ok(proof.validate())
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
            proof.stake_proof.stake_amount,
            deserialized.stake_proof.stake_amount
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

        assert!(
            proof.space_proof.total_size > 0,
            "Space proof should have non-zero total_size"
        );
        assert!(
            proof.stake_proof.stake_amount >= 50,
            "Stake proof should have sufficient amount for CPU validation"
        );
        assert!(
            proof.work_proof.computational_power >= 16,
            "Work proof should have sufficient computational power for CPU"
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
    async fn test_proof_provider_backward_compat_raw_state_proof() {
        let signer = Arc::new(TestNodeSigner::new());
        let provider = TrustChainProofProvider::new(
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
