//! Cross-Network CA Federation -- enables TrustChain nodes to form a trust
//! network where each node's CA trusts certificates from federated peer CAs.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use pqcrypto_falcon::falcon1024;
use pqcrypto_traits::sign::{DetachedSignature, PublicKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::errors::{Result as TrustChainResult, TrustChainError};
use crate::proof_of_state::{FourProofValidator, StateProof};

/// Trust level assigned to a federated peer CA.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FederationTrustLevel {
    /// Fully trusted -- accept certificates without additional checks.
    Full,
    /// Conditionally trusted -- verify against CT log before accepting.
    Conditional,
    /// Untrusted -- reject all certificates from this CA.
    Untrusted,
}

/// A peer CA in the federation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FederatedCA {
    pub ca_id: String,
    pub name: String,
    /// FALCON-1024 public key bytes for certificate verification.
    pub public_key: Vec<u8>,
    /// Root certificate in DER format.
    pub root_certificate: Vec<u8>,
    pub trust_level: FederationTrustLevel,
    pub joined_at: SystemTime,
    pub last_sync: Option<SystemTime>,
    /// IPv6 endpoint for CT log sync.
    pub endpoint: String,
}

/// Policy controlling federation behaviour.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FederationPolicy {
    pub max_peers: usize,
    pub require_ct_proof: bool,
    pub auto_demote_on_failure: bool,
    pub max_sync_age: Duration,
}

impl Default for FederationPolicy {
    fn default() -> Self {
        Self {
            max_peers: 64,
            require_ct_proof: true,
            auto_demote_on_failure: true,
            max_sync_age: Duration::from_secs(86_400),
        }
    }
}

/// Result of validating a certificate issued by a federated peer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FederatedValidationResult {
    pub valid: bool,
    pub issuer_ca_id: String,
    pub trust_level: FederationTrustLevel,
    pub validation_time: SystemTime,
    pub details: String,
}

/// Summary of federation health.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FederationStatus {
    pub local_ca_id: String,
    pub total_peers: usize,
    pub trusted_peers: usize,
    pub conditional_peers: usize,
    pub untrusted_peers: usize,
    pub last_sync: Option<SystemTime>,
}

/// A recorded federation event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FederationEvent {
    pub event_type: FederationEventType,
    pub ca_id: String,
    pub timestamp: SystemTime,
    pub details: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FederationEventType {
    PeerAdded,
    PeerRemoved,
    TrustLevelChanged,
    ValidationSuccess,
    ValidationFailure,
    SyncCompleted,
}

/// Manages federated peer CAs and cross-CA certificate validation.
pub struct FederationManager {
    local_ca_id: String,
    peers: Arc<RwLock<HashMap<String, FederatedCA>>>,
    policy: FederationPolicy,
    events: Arc<RwLock<Vec<FederationEvent>>>,
    /// Four-proof validator for bilateral PoS authentication of peers.
    state_proof_validator: Arc<tokio::sync::Mutex<FourProofValidator>>,
}

impl FederationManager {
    pub fn new(local_ca_id: String, policy: FederationPolicy) -> Self {
        Self {
            local_ca_id,
            peers: Arc::new(RwLock::new(HashMap::new())),
            policy,
            events: Arc::new(RwLock::new(Vec::new())),
            state_proof_validator: Arc::new(tokio::sync::Mutex::new(FourProofValidator::new())),
        }
    }

    /// Add a peer CA with bilateral Proof of State validation.
    ///
    /// The peer's state proof is validated through the four-proof system.
    /// If the proof is invalid, the peer is still added but forced to
    /// `Untrusted` trust level. Only peers with valid PoS may receive
    /// `Full` or `Conditional` trust.
    ///
    /// Fails if duplicate or at capacity.
    pub async fn add_peer(&self, peer: FederatedCA) -> TrustChainResult<()> {
        self.add_peer_with_proof(peer, None).await
    }

    /// Add a peer CA with an explicit state proof for bilateral PoS validation.
    ///
    /// When `state_proof` is `Some`, the proof is validated and the peer's
    /// trust level is only honoured when PoS passes. Otherwise the peer is
    /// demoted to `Untrusted`.
    ///
    /// When `state_proof` is `None`, the peer is accepted at `Untrusted`
    /// trust level regardless of the requested trust level.
    pub async fn add_peer_with_proof(
        &self,
        mut peer: FederatedCA,
        state_proof: Option<&StateProof>,
    ) -> TrustChainResult<()> {
        let mut peers = self.peers.write().await;
        if peers.contains_key(&peer.ca_id) {
            return Err(TrustChainError::InvalidRequest {
                reason: format!("Peer CA '{}' already exists", peer.ca_id),
            });
        }
        if peers.len() >= self.policy.max_peers {
            return Err(TrustChainError::InvalidRequest {
                reason: format!(
                    "Federation at capacity ({}/{})",
                    peers.len(),
                    self.policy.max_peers
                ),
            });
        }

        // Bilateral PoS gate: validate state proof before granting trust
        let pos_valid = match state_proof {
            Some(proof) => {
                let mut validator = self.state_proof_validator.lock().await;
                match validator.validate_state_proof(proof).await {
                    Ok(result) => result.is_valid(),
                    Err(e) => {
                        warn!(
                            "PoS validation error for peer '{}': {}, demoting to Untrusted",
                            peer.ca_id, e
                        );
                        false
                    }
                }
            }
            None => {
                debug!(
                    "No state proof provided for peer '{}', assigning Untrusted",
                    peer.ca_id
                );
                false
            }
        };

        // Enforce trust level based on PoS result
        if !pos_valid && peer.trust_level != FederationTrustLevel::Untrusted {
            warn!(
                "Peer '{}' requested {:?} trust but PoS validation failed, demoting to Untrusted",
                peer.ca_id, peer.trust_level
            );
            peer.trust_level = FederationTrustLevel::Untrusted;
        }

        info!(
            "Adding federated peer CA '{}' ({}) with trust level {:?} (PoS valid: {})",
            peer.ca_id, peer.name, peer.trust_level, pos_valid
        );
        let (ca_id, name, trust) = (
            peer.ca_id.clone(),
            peer.name.clone(),
            format!("{:?}", peer.trust_level),
        );
        peers.insert(ca_id.clone(), peer);
        drop(peers);
        self.record_event(
            FederationEventType::PeerAdded,
            &ca_id,
            format!("Peer '{name}' added with trust {trust} (PoS valid: {pos_valid})"),
        )
        .await;
        Ok(())
    }

    /// Remove a peer CA by ID.
    pub async fn remove_peer(&self, ca_id: &str) -> TrustChainResult<()> {
        let mut peers = self.peers.write().await;
        if peers.remove(ca_id).is_none() {
            return Err(TrustChainError::InvalidRequest {
                reason: format!("Peer CA '{ca_id}' not found"),
            });
        }
        drop(peers);
        info!("Removed federated peer CA '{}'", ca_id);
        self.record_event(
            FederationEventType::PeerRemoved,
            ca_id,
            format!("Peer '{ca_id}' removed"),
        )
        .await;
        Ok(())
    }

    pub async fn get_peer(&self, ca_id: &str) -> Option<FederatedCA> {
        self.peers.read().await.get(ca_id).cloned()
    }

    pub async fn list_peers(&self) -> Vec<FederatedCA> {
        self.peers.read().await.values().cloned().collect()
    }

    /// Update the trust level of an existing peer.
    pub async fn update_trust_level(
        &self,
        ca_id: &str,
        level: FederationTrustLevel,
    ) -> TrustChainResult<()> {
        let mut peers = self.peers.write().await;
        let peer = peers
            .get_mut(ca_id)
            .ok_or_else(|| TrustChainError::InvalidRequest {
                reason: format!("Peer CA '{ca_id}' not found"),
            })?;
        let old = peer.trust_level.clone();
        peer.trust_level = level.clone();
        drop(peers);
        info!("Updated trust for '{}': {:?} -> {:?}", ca_id, old, level);
        self.record_event(
            FederationEventType::TrustLevelChanged,
            ca_id,
            format!("{old:?} -> {level:?}"),
        )
        .await;
        Ok(())
    }

    /// Validate a certificate issued by a federated peer CA.
    ///
    /// Looks up the peer, rejects if `Untrusted`, then verifies the FALCON-1024
    /// signature using the peer's public key.
    pub async fn validate_federated_certificate(
        &self,
        cert_der: &[u8],
        issuer_ca_id: &str,
    ) -> TrustChainResult<FederatedValidationResult> {
        let peer = {
            let peers = self.peers.read().await;
            peers
                .get(issuer_ca_id)
                .cloned()
                .ok_or_else(|| TrustChainError::InvalidRequest {
                    reason: format!("Issuer CA '{issuer_ca_id}' is not a known federation peer"),
                })?
        };

        if peer.trust_level == FederationTrustLevel::Untrusted {
            warn!(
                "Rejecting certificate from untrusted peer '{}'",
                issuer_ca_id
            );
            self.record_event(
                FederationEventType::ValidationFailure,
                issuer_ca_id,
                "Rejected: untrusted".into(),
            )
            .await;
            return Ok(FederatedValidationResult {
                valid: false,
                issuer_ca_id: issuer_ca_id.to_string(),
                trust_level: FederationTrustLevel::Untrusted,
                validation_time: SystemTime::now(),
                details: "Certificate rejected: issuing CA is untrusted".into(),
            });
        }

        let valid = Self::verify_falcon_signature(cert_der, &peer.public_key);
        if valid {
            debug!(
                "Certificate from '{}' passed FALCON-1024 verification",
                issuer_ca_id
            );
            self.record_event(
                FederationEventType::ValidationSuccess,
                issuer_ca_id,
                "Signature verified".into(),
            )
            .await;
        } else {
            warn!(
                "Certificate from '{}' FAILED FALCON-1024 verification",
                issuer_ca_id
            );
            self.record_event(
                FederationEventType::ValidationFailure,
                issuer_ca_id,
                "Signature verification failed".into(),
            )
            .await;
            if self.policy.auto_demote_on_failure {
                if let Some(p) = self.peers.write().await.get_mut(issuer_ca_id) {
                    p.trust_level = FederationTrustLevel::Untrusted;
                }
                self.record_event(
                    FederationEventType::TrustLevelChanged,
                    issuer_ca_id,
                    "Auto-demoted to Untrusted".into(),
                )
                .await;
            }
        }

        Ok(FederatedValidationResult {
            valid,
            issuer_ca_id: issuer_ca_id.to_string(),
            trust_level: peer.trust_level.clone(),
            validation_time: SystemTime::now(),
            details: if valid {
                "FALCON-1024 verification succeeded".into()
            } else {
                "FALCON-1024 verification failed".into()
            },
        })
    }

    /// Return a summary of the federation's current health.
    pub async fn get_federation_status(&self) -> FederationStatus {
        let peers = self.peers.read().await;
        let (mut trusted, mut conditional, mut untrusted) = (0, 0, 0);
        let mut latest_sync: Option<SystemTime> = None;
        for peer in peers.values() {
            match peer.trust_level {
                FederationTrustLevel::Full => trusted += 1,
                FederationTrustLevel::Conditional => conditional += 1,
                FederationTrustLevel::Untrusted => untrusted += 1,
            }
            if let Some(sync) = peer.last_sync {
                latest_sync = Some(latest_sync.map_or(sync, |prev: SystemTime| prev.max(sync)));
            }
        }
        FederationStatus {
            local_ca_id: self.local_ca_id.clone(),
            total_peers: peers.len(),
            trusted_peers: trusted,
            conditional_peers: conditional,
            untrusted_peers: untrusted,
            last_sync: latest_sync,
        }
    }

    pub async fn get_events(&self) -> Vec<FederationEvent> {
        self.events.read().await.clone()
    }

    // -- Private helpers -----------------------------------------------------

    /// Verify FALCON-1024 signature. Wire format: `[4B sig_len LE][sig][cert body]`
    fn verify_falcon_signature(signed_blob: &[u8], pub_key_bytes: &[u8]) -> bool {
        if signed_blob.len() < 4 {
            debug!("Signed blob too short for length prefix");
            return false;
        }
        let sig_len = u32::from_le_bytes([
            signed_blob[0],
            signed_blob[1],
            signed_blob[2],
            signed_blob[3],
        ]) as usize;
        let header_end = 4 + sig_len;
        if signed_blob.len() < header_end {
            debug!(
                "Signed blob too short: need {} bytes, have {}",
                header_end,
                signed_blob.len()
            );
            return false;
        }
        let (sig_bytes, cert_body) = (&signed_blob[4..header_end], &signed_blob[header_end..]);

        let public_key = match falcon1024::PublicKey::from_bytes(pub_key_bytes) {
            Ok(pk) => pk,
            Err(e) => {
                warn!("Failed to reconstruct FALCON-1024 public key: {}", e);
                return false;
            }
        };
        let signature = match falcon1024::DetachedSignature::from_bytes(sig_bytes) {
            Ok(sig) => sig,
            Err(e) => {
                debug!("Failed to reconstruct FALCON-1024 signature: {}", e);
                return false;
            }
        };
        let hash: [u8; 32] = Sha256::digest(cert_body).into();
        falcon1024::verify_detached_signature(&signature, &hash, &public_key).is_ok()
    }

    async fn record_event(&self, event_type: FederationEventType, ca_id: &str, details: String) {
        self.events.write().await.push(FederationEvent {
            event_type,
            ca_id: ca_id.to_string(),
            timestamp: SystemTime::now(),
            details,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_policy() -> FederationPolicy {
        FederationPolicy {
            max_peers: 5,
            require_ct_proof: false,
            auto_demote_on_failure: true,
            max_sync_age: Duration::from_secs(3600),
        }
    }

    fn make_peer(id: &str, trust: FederationTrustLevel) -> FederatedCA {
        FederatedCA {
            ca_id: id.to_string(),
            name: format!("Peer {id}"),
            public_key: vec![0u8; 32],
            root_certificate: vec![1u8; 64],
            trust_level: trust,
            joined_at: SystemTime::now(),
            last_sync: None,
            endpoint: "[::1]:8443".to_string(),
        }
    }

    /// Helper: create a test state proof that passes FourProofValidator.
    fn test_state_proof() -> StateProof {
        StateProof::default_for_testing()
    }

    #[tokio::test]
    async fn test_add_and_list_peers() {
        let mgr = FederationManager::new("local-ca".into(), test_policy());
        // Without PoS proof, all peers are demoted to Untrusted — still stored
        mgr.add_peer(make_peer("alpha", FederationTrustLevel::Full))
            .await
            .expect("test: add alpha");
        mgr.add_peer(make_peer("beta", FederationTrustLevel::Conditional))
            .await
            .expect("test: add beta");
        mgr.add_peer(make_peer("gamma", FederationTrustLevel::Untrusted))
            .await
            .expect("test: add gamma");
        assert_eq!(mgr.list_peers().await.len(), 3);
    }

    #[tokio::test]
    async fn test_add_peer_without_proof_demotes_to_untrusted() {
        let mgr = FederationManager::new("local-ca".into(), test_policy());
        mgr.add_peer(make_peer("alpha", FederationTrustLevel::Full))
            .await
            .expect("test: add");
        let peer = mgr.get_peer("alpha").await.expect("test: peer exists");
        assert_eq!(
            peer.trust_level,
            FederationTrustLevel::Untrusted,
            "Peer without PoS proof should be demoted to Untrusted"
        );
    }

    #[tokio::test]
    async fn test_add_peer_with_valid_proof_keeps_trust() {
        let mgr = FederationManager::new("local-ca".into(), test_policy());
        let proof = test_state_proof();
        mgr.add_peer_with_proof(
            make_peer("alpha", FederationTrustLevel::Full),
            Some(&proof),
        )
        .await
        .expect("test: add with proof");
        let peer = mgr.get_peer("alpha").await.expect("test: peer exists");
        assert_eq!(
            peer.trust_level,
            FederationTrustLevel::Full,
            "Peer with valid PoS proof should keep requested trust level"
        );
    }

    #[tokio::test]
    async fn test_remove_peer() {
        let mgr = FederationManager::new("local-ca".into(), test_policy());
        mgr.add_peer(make_peer("alpha", FederationTrustLevel::Untrusted))
            .await
            .expect("test: add");
        assert!(mgr.get_peer("alpha").await.is_some());
        mgr.remove_peer("alpha").await.expect("test: remove");
        assert!(mgr.get_peer("alpha").await.is_none());
    }

    #[tokio::test]
    async fn test_duplicate_peer_rejected() {
        let mgr = FederationManager::new("local-ca".into(), test_policy());
        mgr.add_peer(make_peer("alpha", FederationTrustLevel::Full))
            .await
            .expect("test: first add");
        assert!(mgr
            .add_peer(make_peer("alpha", FederationTrustLevel::Full))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_update_trust_level() {
        let mgr = FederationManager::new("local-ca".into(), test_policy());
        mgr.add_peer(make_peer("alpha", FederationTrustLevel::Untrusted))
            .await
            .expect("test: add");
        mgr.update_trust_level("alpha", FederationTrustLevel::Conditional)
            .await
            .expect("test: update");
        let peer = mgr.get_peer("alpha").await.expect("test: peer exists");
        assert_eq!(peer.trust_level, FederationTrustLevel::Conditional);
    }

    #[tokio::test]
    async fn test_validate_trusted_cert() {
        let (pk, sk) = falcon1024::keypair();
        let cert_body = b"test-certificate-body";
        let hash: [u8; 32] = Sha256::digest(cert_body).into();
        let sig = falcon1024::detached_sign(&hash, &sk);
        let sig_bytes = sig.as_bytes();
        // Wire format: [4-byte sig_len LE][signature][cert_body]
        let mut blob = (sig_bytes.len() as u32).to_le_bytes().to_vec();
        blob.extend_from_slice(sig_bytes);
        blob.extend_from_slice(cert_body);

        let mgr = FederationManager::new("local-ca".into(), test_policy());
        let mut peer = make_peer("signer-ca", FederationTrustLevel::Full);
        peer.public_key = pk.as_bytes().to_vec();
        // Use add_peer_with_proof to preserve Full trust level
        let proof = test_state_proof();
        mgr.add_peer_with_proof(peer, Some(&proof))
            .await
            .expect("test: add peer");

        let result = mgr
            .validate_federated_certificate(&blob, "signer-ca")
            .await
            .expect("test: validate");
        assert!(result.valid, "certificate should be valid");
        assert_eq!(result.trust_level, FederationTrustLevel::Full);
    }

    #[tokio::test]
    async fn test_validate_untrusted_cert_rejected() {
        let mgr = FederationManager::new("local-ca".into(), test_policy());
        mgr.add_peer(make_peer("bad-ca", FederationTrustLevel::Untrusted))
            .await
            .expect("test: add");
        let result = mgr
            .validate_federated_certificate(b"any-cert-data", "bad-ca")
            .await
            .expect("test: validate");
        assert!(!result.valid);
        assert_eq!(result.trust_level, FederationTrustLevel::Untrusted);
    }

    #[tokio::test]
    async fn test_max_peers_enforced() {
        let mgr = FederationManager::new("local-ca".into(), test_policy());
        for i in 0..5 {
            mgr.add_peer(make_peer(&format!("p{i}"), FederationTrustLevel::Untrusted))
                .await
                .expect("test: add");
        }
        assert!(mgr
            .add_peer(make_peer("overflow", FederationTrustLevel::Untrusted))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_federation_status() {
        let mgr = FederationManager::new("local-ca".into(), test_policy());
        let proof = test_state_proof();
        // Use add_peer_with_proof to preserve requested trust levels
        mgr.add_peer_with_proof(
            make_peer("a", FederationTrustLevel::Full),
            Some(&proof),
        )
        .await
        .expect("test: add a");
        mgr.add_peer_with_proof(
            make_peer("b", FederationTrustLevel::Full),
            Some(&proof),
        )
        .await
        .expect("test: add b");
        mgr.add_peer_with_proof(
            make_peer("c", FederationTrustLevel::Conditional),
            Some(&proof),
        )
        .await
        .expect("test: add c");
        mgr.add_peer(make_peer("d", FederationTrustLevel::Untrusted))
            .await
            .expect("test: add d");
        let status = mgr.get_federation_status().await;
        assert_eq!(status.local_ca_id, "local-ca");
        assert_eq!(status.total_peers, 4);
        assert_eq!(status.trusted_peers, 2);
        assert_eq!(status.conditional_peers, 1);
        assert_eq!(status.untrusted_peers, 1);
    }

    #[tokio::test]
    async fn test_federation_events_logged() {
        let mgr = FederationManager::new("local-ca".into(), test_policy());
        mgr.add_peer(make_peer("alpha", FederationTrustLevel::Untrusted))
            .await
            .expect("test: add");
        mgr.update_trust_level("alpha", FederationTrustLevel::Conditional)
            .await
            .expect("test: update");
        mgr.remove_peer("alpha").await.expect("test: remove");
        let events = mgr.get_events().await;
        assert!(
            events.len() >= 3,
            "expected >= 3 events, got {}",
            events.len()
        );
        assert!(matches!(
            events[0].event_type,
            FederationEventType::PeerAdded
        ));
        assert!(matches!(
            events[1].event_type,
            FederationEventType::TrustLevelChanged
        ));
        assert!(matches!(
            events[2].event_type,
            FederationEventType::PeerRemoved
        ));
    }
}
