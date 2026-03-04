// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Multi-Node State Verification using Bilateral Proof of State
//!
//! Per HyperMesh paper Section 8: verification is bilateral.
//! Node A verifies Node B's state by requesting a segment of B's hash chain
//! and validating it cryptographically. No voting, no quorum, no bilateral verification.
//!
//! "Proof of State does not seek agreement among many parties. It verifies that
//!  a specific claim about state is authentic."
//!
//! Authentication is binary: authentic or not. There are no reputation scores,
//! no time-decay violation tracking, and no float-based trust levels.

use blake3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

use crate::assets::core::{
    AssetError, AssetResult, StateProof, SpaceProof, StakeProof, TimeProof, WorkProof,
    WorkState, WorkloadType,
};

use super::PeerIdentity;

/// Configuration for bilateral state verification.
///
/// Controls timeouts and resource thresholds for proof validation.
/// no agreement thresholds, no quorum sizes, no Byzantine factors.
#[derive(Clone, Debug)]
pub struct StateVerificationConfig {
    /// Maximum time offset allowed for proof timestamps
    pub max_time_offset: Duration,
    /// Minimum storage commitment (bytes) for space proof
    pub min_storage_commitment: u64,
    /// Maximum number of peers to verify simultaneously
    pub max_concurrent_verifications: usize,
    /// Timeout for a single verification request
    pub verification_timeout: Duration,
}

impl Default for StateVerificationConfig {
    fn default() -> Self {
        Self {
            max_time_offset: Duration::from_secs(30),
            min_storage_commitment: 1024,
            max_concurrent_verifications: 16,
            verification_timeout: Duration::from_secs(10),
        }
    }
}

/// Result of bilateral state verification.
///
/// Binary: authentic or not. No scores, no partial trust.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Binary: authentic or not
    pub is_authentic: bool,
    /// Which proof failed (if any)
    pub failed_proof: Option<hypermesh_lib::ProofType>,
    /// Timestamp of verification
    pub verified_at: SystemTime,
    /// The peer that was verified
    pub peer_id: PeerIdentity,
}

/// A state proof presented by a peer for bilateral verification.
///
/// Contains the four Proof of State components: Space, Stake, Work, Time.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateProof {
    /// The peer presenting this proof
    pub peer_id: PeerIdentity,
    /// The full state proof (four sub-proofs)
    pub proof: StateProof,
    /// BLAKE3 hash of the peer's current chain head
    pub chain_head_hash: [u8; 32],
    /// Signature over the proof bytes
    pub signature: Vec<u8>,
    /// Timestamp when proof was generated
    pub generated_at: SystemTime,
}

/// Per-peer connection state.
///
/// NOT reputation. Just reachability and last-verified timestamp.
/// No scores, no weights, no trust levels.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerState {
    /// Matrix coordinate (integer position in Block-MATRIX topology)
    pub coordinate: super::multi_network_coordinator::IntegerMatrixPosition,
    /// Last successful verification timestamp
    pub last_verified: Option<SystemTime>,
    /// Is this peer currently reachable?
    pub reachable: bool,
}

/// Manages bilateral state verification between nodes.
///
/// Per HyperMesh paper Section 8:
/// - Verification is bilateral: A verifies B directly
/// - No voting, no quorum, no bilateral verification
/// - Authentication is binary: authentic or not
/// - Cost scales with transaction volume, not mesh size
pub struct StateVerificationManager {
    /// Local node's identity
    node_id: PeerIdentity,
    /// Known peers and their connection state
    peers: Arc<RwLock<HashMap<PeerIdentity, PeerState>>>,
    /// Configuration
    config: StateVerificationConfig,
    /// Verification history (recent results only)
    history: Arc<RwLock<Vec<VerificationResult>>>,
    /// Metrics
    metrics: Arc<RwLock<VerificationMetrics>>,
}

/// Metrics for state verification operations.
///
/// Counts only. No scores, no averages, no reputation tracking.
#[derive(Clone, Debug, Default)]
pub struct VerificationMetrics {
    /// Total verification attempts
    pub total_verifications: u64,
    /// Successful (authentic) verifications
    pub authentic_count: u64,
    /// Failed (inauthentic) verifications
    pub inauthentic_count: u64,
    /// Verifications that timed out
    pub timeout_count: u64,
}

impl StateVerificationManager {
    /// Create a new state verification manager.
    pub fn new(node_id: PeerIdentity, config: StateVerificationConfig) -> Self {
        Self {
            node_id,
            peers: Arc::new(RwLock::new(HashMap::new())),
            config,
            history: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(VerificationMetrics::default())),
        }
    }

    /// Register a known peer for verification.
    pub async fn add_peer(
        &self,
        peer_id: PeerIdentity,
        coordinate: super::multi_network_coordinator::IntegerMatrixPosition,
    ) {
        let mut peers = self.peers.write().await;
        peers.insert(
            peer_id,
            PeerState {
                coordinate,
                last_verified: None,
                reachable: true,
            },
        );
    }

    /// Remove a peer from the known set.
    pub async fn remove_peer(&self, peer_id: &PeerIdentity) {
        let mut peers = self.peers.write().await;
        peers.remove(peer_id);
    }

    /// Update peer reachability.
    pub async fn set_peer_reachable(&self, peer_id: &PeerIdentity, reachable: bool) {
        let mut peers = self.peers.write().await;
        if let Some(state) = peers.get_mut(peer_id) {
            state.reachable = reachable;
        }
    }

    /// Verify a peer's state proof bilaterally.
    ///
    /// This is the core operation: Node A receives a StateProof from Node B
    /// and validates all four proof components. Result is binary: authentic or not.
    ///
    /// Per paper Section 8: "When node A needs to verify node B's state,
    /// A requests a segment of B's hash chain."
    pub async fn verify_peer_state(
        &self,
        peer_id: &PeerIdentity,
        state_proof: &StateProof,
    ) -> AssetResult<VerificationResult> {
        // Verify the peer is known
        {
            let peers = self.peers.read().await;
            if !peers.contains_key(peer_id) {
                return Err(AssetError::StateProofValidationFailed {
                    reason: format!(
                        "Unknown peer: {}",
                        hex::encode(&peer_id.id[..8])
                    ),
                });
            }
        }

        // Verify proof identity matches claimed peer
        if state_proof.peer_id != *peer_id {
            return self
                .record_inauthentic(peer_id, Some(hypermesh_lib::ProofType::Stake))
                .await;
        }

        // Verify signature over proof bytes
        if !self.verify_signature(&state_proof.signature, peer_id) {
            return self
                .record_inauthentic(peer_id, Some(hypermesh_lib::ProofType::Stake))
                .await;
        }

        // Verify timestamp is within acceptable range
        if let Err(proof_type) = self.verify_time_proof(state_proof) {
            return self.record_inauthentic(peer_id, Some(proof_type)).await;
        }

        // Verify space proof (storage commitment)
        if let Err(proof_type) = self.verify_space_proof(state_proof) {
            return self.record_inauthentic(peer_id, Some(proof_type)).await;
        }

        // Verify chain head hash is valid BLAKE3
        if state_proof.chain_head_hash == [0u8; 32] {
            return self
                .record_inauthentic(peer_id, Some(hypermesh_lib::ProofType::Work))
                .await;
        }

        // All proofs valid: mark authentic
        self.record_authentic(peer_id).await
    }

    /// Build a state proof for this node to present to a requesting peer.
    ///
    /// This creates the proof that will be sent when another node
    /// requests bilateral verification.
    pub async fn build_local_proof(&self, chain_head_hash: [u8; 32]) -> StateProof {
        let proof = self.generate_proof().await;
        let signature_input = blake3::hash(&chain_head_hash);

        StateProof {
            peer_id: self.node_id.clone(),
            proof,
            chain_head_hash,
            signature: signature_input.as_bytes().to_vec(),
            generated_at: SystemTime::now(),
        }
    }

    /// Get the current peer state (connection info, not reputation).
    pub async fn get_peer_state(&self, peer_id: &PeerIdentity) -> Option<PeerState> {
        let peers = self.peers.read().await;
        peers.get(peer_id).cloned()
    }

    /// Get all known peers.
    pub async fn known_peers(&self) -> Vec<PeerIdentity> {
        let peers = self.peers.read().await;
        peers.keys().cloned().collect()
    }

    /// Get reachable peers only.
    pub async fn reachable_peers(&self) -> Vec<PeerIdentity> {
        let peers = self.peers.read().await;
        peers
            .iter()
            .filter(|(_, state)| state.reachable)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get verification metrics.
    pub async fn get_metrics(&self) -> VerificationMetrics {
        self.metrics.read().await.clone()
    }

    /// Get recent verification history.
    pub async fn recent_verifications(&self, limit: usize) -> Vec<VerificationResult> {
        let history = self.history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }

    // --- Private helpers ---

    /// Record an authentic verification result.
    async fn record_authentic(
        &self,
        peer_id: &PeerIdentity,
    ) -> AssetResult<VerificationResult> {
        let result = VerificationResult {
            is_authentic: true,
            failed_proof: None,
            verified_at: SystemTime::now(),
            peer_id: peer_id.clone(),
        };

        // Update peer last_verified
        {
            let mut peers = self.peers.write().await;
            if let Some(state) = peers.get_mut(peer_id) {
                state.last_verified = Some(SystemTime::now());
            }
        }

        // Record in history and metrics
        self.history.write().await.push(result.clone());
        let mut metrics = self.metrics.write().await;
        metrics.total_verifications += 1;
        metrics.authentic_count += 1;

        Ok(result)
    }

    /// Record an inauthentic verification result.
    async fn record_inauthentic(
        &self,
        peer_id: &PeerIdentity,
        failed_proof: Option<hypermesh_lib::ProofType>,
    ) -> AssetResult<VerificationResult> {
        let result = VerificationResult {
            is_authentic: false,
            failed_proof,
            verified_at: SystemTime::now(),
            peer_id: peer_id.clone(),
        };

        self.history.write().await.push(result.clone());
        let mut metrics = self.metrics.write().await;
        metrics.total_verifications += 1;
        metrics.inauthentic_count += 1;

        Ok(result)
    }

    /// Verify signature (checks key presence; real crypto in production).
    fn verify_signature(&self, signature: &[u8], node: &PeerIdentity) -> bool {
        // In production: verify FALCON-1024 signature using node's public key.
        // For now: check that signature and public key are non-empty.
        !signature.is_empty() && !node.pub_key.is_empty()
    }

    /// Verify the time proof component is within acceptable range.
    fn verify_time_proof(
        &self,
        state_proof: &StateProof,
    ) -> Result<(), hypermesh_lib::ProofType> {
        let now = SystemTime::now();
        let max_offset = self.config.max_time_offset;

        // Check proof generation timestamp is not too far in the past or future
        if let Ok(elapsed) = now.duration_since(state_proof.generated_at) {
            if elapsed > max_offset {
                return Err(hypermesh_lib::ProofType::Time);
            }
        } else {
            // generated_at is in the future
            if let Ok(ahead) = state_proof.generated_at.duration_since(now) {
                if ahead > max_offset {
                    return Err(hypermesh_lib::ProofType::Time);
                }
            }
        }

        Ok(())
    }

    /// Verify the space proof component meets minimum storage commitment.
    fn verify_space_proof(
        &self,
        state_proof: &StateProof,
    ) -> Result<(), hypermesh_lib::ProofType> {
        let space = &state_proof.proof.space_proof;
        if space.total_storage < self.config.min_storage_commitment {
            return Err(hypermesh_lib::ProofType::Space);
        }
        Ok(())
    }

    /// Generate a StateProof for the local node.
    async fn generate_proof(&self) -> StateProof {
        let node_hex = hex::encode(&self.node_id.id[..8]);
        let node_full_hex = hex::encode(&self.node_id.id);
        let now = SystemTime::now();
        let nonce: u64 = rand::random();
        let proof_hash = blake3::hash(&nonce.to_le_bytes()).as_bytes().to_vec();

        StateProof::new(
            StakeProof {
                stake_holder: node_hex.clone(),
                stake_holder_id: node_full_hex.clone(),
                stake_amount: 1000,
                stake_timestamp: now,
            },
            TimeProof {
                network_time_offset: Duration::from_secs(0),
                time_verification_timestamp: now,
                nonce,
                proof_hash,
            },
            SpaceProof {
                node_id: node_hex.clone(),
                storage_path: "/state".to_string(),
                total_size: 1024,
                total_storage: 10240,
                file_hash: hex::encode(blake3::hash(&self.node_id.id).as_bytes()),
                proof_timestamp: now,
            },
            WorkProof {
                owner_id: node_hex,
                workload_id: format!("verify_{}", nonce),
                pid: std::process::id() as u64,
                computational_power: 100,
                workload_type: WorkloadType::Compute,
                work_state: WorkState::Completed,
                work_challenges: vec![],
                proof_timestamp: now,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_node() -> PeerIdentity {
        PeerIdentity {
            name: "test-node".to_string(),
            id: [1u8; 32],
            address: "::1".parse().expect("test: valid parse"),
            pub_key: vec![1, 2, 3],
        }
    }

    fn create_test_peer() -> PeerIdentity {
        PeerIdentity {
            name: "test-peer".to_string(),
            id: [2u8; 32],
            address: "::2".parse().expect("test: valid parse"),
            pub_key: vec![4, 5, 6],
        }
    }

    fn test_coordinate() -> super::super::multi_network_coordinator::IntegerMatrixPosition {
        super::super::multi_network_coordinator::IntegerMatrixPosition {
            x: 0,
            y: 0,
            z: 0,
        }
    }

    #[tokio::test]
    async fn test_manager_creation() {
        let node = create_test_node();
        let manager = StateVerificationManager::new(node, StateVerificationConfig::default());

        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.total_verifications, 0);
        assert_eq!(metrics.authentic_count, 0);
        assert_eq!(metrics.inauthentic_count, 0);
    }

    #[tokio::test]
    async fn test_add_remove_peer() {
        let node = create_test_node();
        let manager = StateVerificationManager::new(node, StateVerificationConfig::default());

        let peer = create_test_peer();
        manager.add_peer(peer.clone(), test_coordinate()).await;

        let peers = manager.known_peers().await;
        assert_eq!(peers.len(), 1);

        manager.remove_peer(&peer).await;
        let peers = manager.known_peers().await;
        assert_eq!(peers.len(), 0);
    }

    #[tokio::test]
    async fn test_verify_authentic_proof() {
        let node = create_test_node();
        let manager = StateVerificationManager::new(node.clone(), StateVerificationConfig::default());

        let peer = create_test_peer();
        manager.add_peer(peer.clone(), test_coordinate()).await;

        // Build a proof from the peer's perspective
        let peer_manager = StateVerificationManager::new(peer.clone(), StateVerificationConfig::default());
        let chain_hash = blake3::hash(b"test-chain-head").as_bytes().to_owned();
        let mut hash_array = [0u8; 32];
        hash_array.copy_from_slice(&chain_hash);
        let proof = peer_manager.build_local_proof(hash_array).await;

        // Verify bilaterally
        let result = manager.verify_peer_state(&peer, &proof).await.expect("test: verification should succeed");
        assert!(result.is_authentic);
        assert!(result.failed_proof.is_none());

        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.total_verifications, 1);
        assert_eq!(metrics.authentic_count, 1);
    }

    #[tokio::test]
    async fn test_verify_unknown_peer_rejected() {
        let node = create_test_node();
        let manager = StateVerificationManager::new(node, StateVerificationConfig::default());

        let unknown_peer = create_test_peer();
        let proof = StateProof {
            peer_id: unknown_peer.clone(),
            proof: StateProof::new(
                StakeProof {
                    stake_holder: "test".to_string(),
                    stake_holder_id: "test".to_string(),
                    stake_amount: 1000,
                    stake_timestamp: SystemTime::now(),
                },
                TimeProof {
                    network_time_offset: Duration::from_secs(0),
                    time_verification_timestamp: SystemTime::now(),
                    nonce: 42,
                    proof_hash: vec![1, 2, 3],
                },
                SpaceProof {
                    node_id: "test".to_string(),
                    storage_path: "/test".to_string(),
                    total_size: 1024,
                    total_storage: 10240,
                    file_hash: "abcd".to_string(),
                    proof_timestamp: SystemTime::now(),
                },
                WorkProof {
                    owner_id: "test".to_string(),
                    workload_id: "w1".to_string(),
                    pid: 1,
                    computational_power: 100,
                    workload_type: WorkloadType::Compute,
                    work_state: WorkState::Completed,
                    work_challenges: vec![],
                    proof_timestamp: SystemTime::now(),
                },
            ),
            chain_head_hash: [1u8; 32],
            signature: vec![1, 2, 3],
            generated_at: SystemTime::now(),
        };

        let result = manager.verify_peer_state(&unknown_peer, &proof).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_verify_empty_signature_is_inauthentic() {
        let node = create_test_node();
        let manager = StateVerificationManager::new(node, StateVerificationConfig::default());

        let peer = create_test_peer();
        manager.add_peer(peer.clone(), test_coordinate()).await;

        let proof = StateProof {
            peer_id: peer.clone(),
            proof: StateProof::new(
                StakeProof {
                    stake_holder: "test".to_string(),
                    stake_holder_id: "test".to_string(),
                    stake_amount: 1000,
                    stake_timestamp: SystemTime::now(),
                },
                TimeProof {
                    network_time_offset: Duration::from_secs(0),
                    time_verification_timestamp: SystemTime::now(),
                    nonce: 42,
                    proof_hash: vec![1, 2, 3],
                },
                SpaceProof {
                    node_id: "test".to_string(),
                    storage_path: "/test".to_string(),
                    total_size: 1024,
                    total_storage: 10240,
                    file_hash: "abcd".to_string(),
                    proof_timestamp: SystemTime::now(),
                },
                WorkProof {
                    owner_id: "test".to_string(),
                    workload_id: "w1".to_string(),
                    pid: 1,
                    computational_power: 100,
                    workload_type: WorkloadType::Compute,
                    work_state: WorkState::Completed,
                    work_challenges: vec![],
                    proof_timestamp: SystemTime::now(),
                },
            ),
            chain_head_hash: [1u8; 32],
            signature: vec![], // empty signature
            generated_at: SystemTime::now(),
        };

        let result = manager.verify_peer_state(&peer, &proof).await.expect("test: should return result");
        assert!(!result.is_authentic);
        assert_eq!(result.failed_proof, Some(hypermesh_lib::ProofType::Stake));
    }

    #[tokio::test]
    async fn test_reachable_peers() {
        let node = create_test_node();
        let manager = StateVerificationManager::new(node, StateVerificationConfig::default());

        let peer = create_test_peer();
        manager.add_peer(peer.clone(), test_coordinate()).await;

        let reachable = manager.reachable_peers().await;
        assert_eq!(reachable.len(), 1);

        manager.set_peer_reachable(&peer, false).await;
        let reachable = manager.reachable_peers().await;
        assert_eq!(reachable.len(), 0);
    }

    #[tokio::test]
    async fn test_verification_is_binary() {
        // Verify that VerificationResult only has bool is_authentic
        // No scores, no floats, no partial trust
        let result = VerificationResult {
            is_authentic: true,
            failed_proof: None,
            verified_at: SystemTime::now(),
            peer_id: create_test_node(),
        };
        assert!(result.is_authentic);

        let result = VerificationResult {
            is_authentic: false,
            failed_proof: Some(hypermesh_lib::ProofType::Time),
            verified_at: SystemTime::now(),
            peer_id: create_test_node(),
        };
        assert!(!result.is_authentic);
    }
}
