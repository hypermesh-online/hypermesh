// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Distributed Revocation Propagation
//!
//! Propagates revocation decisions across federated CA peers using a
//! message-based protocol. This module defines the protocol messages and
//! tracks propagation status without performing actual network I/O.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::errors::{Result as TrustChainResult, TrustChainError};

/// Status of revocation propagation to a specific peer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropagationStatus {
    /// Message queued but not yet sent.
    Pending,
    /// Message sent to the peer.
    Propagated,
    /// Peer acknowledged receipt.
    Confirmed,
    /// Propagation failed (with reason).
    Failed(String),
}

/// A revocation notice to be propagated.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RevocationNotice {
    /// Unique notice identifier.
    pub notice_id: String,
    /// Serial number of the revoked certificate.
    pub serial_number: String,
    /// Reason for revocation.
    pub reason: String,
    /// CA that initiated the revocation.
    pub issuing_ca_id: String,
    /// BLAKE3 hash of the notice for integrity verification.
    pub notice_hash: [u8; 32],
    /// When the revocation was issued.
    pub issued_at: SystemTime,
}

impl RevocationNotice {
    /// Create a new revocation notice with computed hash.
    pub fn new(
        serial_number: String,
        reason: String,
        issuing_ca_id: String,
    ) -> Self {
        let notice_id = uuid::Uuid::new_v4().to_string();
        let hash = Self::compute_hash(&notice_id, &serial_number, &reason);
        Self {
            notice_id,
            serial_number,
            reason,
            issuing_ca_id,
            notice_hash: hash,
            issued_at: SystemTime::now(),
        }
    }

    fn compute_hash(notice_id: &str, serial: &str, reason: &str) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(notice_id.as_bytes());
        hasher.update(serial.as_bytes());
        hasher.update(reason.as_bytes());
        *hasher.finalize().as_bytes()
    }
}

/// Propagation state for a single notice across all peers.
#[derive(Clone, Debug)]
struct NoticePropagation {
    /// The revocation notice being propagated (retained for audit/resend).
    _notice: RevocationNotice,
    /// Peer CA ID -> propagation status.
    peer_status: HashMap<String, PropagationStatus>,
}

/// Outbound message from the propagator (consumed by the transport layer).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PropagationMessage {
    /// Broadcast a revocation to a peer.
    BroadcastRevocation {
        peer_ca_id: String,
        notice: RevocationNotice,
    },
    /// Acknowledge receipt of a revocation from a peer.
    AcknowledgeRevocation {
        peer_ca_id: String,
        notice_id: String,
    },
}

/// Manages propagation of revocation notices to federation peers.
///
/// This is the protocol layer: it produces `PropagationMessage`s that the
/// transport layer must deliver. It does not perform network I/O directly.
pub struct RevocationPropagator {
    /// Local CA identifier.
    local_ca_id: String,
    /// Known federation peer CA IDs.
    known_peers: Arc<RwLock<Vec<String>>>,
    /// Active propagation records.
    propagations: Arc<RwLock<HashMap<String, NoticePropagation>>>,
    /// Notices received from remote peers (serial -> notice).
    received_notices: Arc<RwLock<HashMap<String, RevocationNotice>>>,
    /// Outbound message queue.
    outbox: Arc<RwLock<Vec<PropagationMessage>>>,
}

impl RevocationPropagator {
    /// Create a new propagator for the given local CA.
    pub fn new(local_ca_id: String) -> Self {
        Self {
            local_ca_id,
            known_peers: Arc::new(RwLock::new(Vec::new())),
            propagations: Arc::new(RwLock::new(HashMap::new())),
            received_notices: Arc::new(RwLock::new(HashMap::new())),
            outbox: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a federation peer for propagation.
    pub async fn add_peer(&self, peer_ca_id: String) {
        let mut peers = self.known_peers.write().await;
        if !peers.contains(&peer_ca_id) {
            peers.push(peer_ca_id.clone());
            debug!("Added revocation propagation peer: {}", peer_ca_id);
        }
    }

    /// Remove a federation peer.
    pub async fn remove_peer(&self, peer_ca_id: &str) {
        let mut peers = self.known_peers.write().await;
        peers.retain(|p| p != peer_ca_id);
    }

    /// Broadcast a revocation notice to all known peers.
    ///
    /// Creates outbound messages for each peer and returns the notice ID.
    pub async fn broadcast_revocation(
        &self,
        serial_number: String,
        reason: String,
    ) -> TrustChainResult<String> {
        let notice = RevocationNotice::new(
            serial_number.clone(),
            reason.clone(),
            self.local_ca_id.clone(),
        );
        let notice_id = notice.notice_id.clone();

        let peers = self.known_peers.read().await;
        if peers.is_empty() {
            warn!("No peers registered for revocation propagation");
        }

        let mut peer_status = HashMap::new();
        let mut outbox = self.outbox.write().await;

        for peer_id in peers.iter() {
            outbox.push(PropagationMessage::BroadcastRevocation {
                peer_ca_id: peer_id.clone(),
                notice: notice.clone(),
            });
            peer_status.insert(peer_id.clone(), PropagationStatus::Pending);
        }

        // Record the propagation
        let mut propagations = self.propagations.write().await;
        propagations.insert(
            notice_id.clone(),
            NoticePropagation {
                _notice: notice,
                peer_status,
            },
        );

        info!(
            "Queued revocation broadcast for '{}' to {} peers",
            serial_number,
            peers.len()
        );

        Ok(notice_id)
    }

    /// Accept an incoming revocation notice from a remote peer.
    ///
    /// Validates the notice hash and stores it. Generates an ack message.
    pub async fn receive_revocation(
        &self,
        notice: RevocationNotice,
    ) -> TrustChainResult<()> {
        // Verify hash integrity
        let expected = RevocationNotice::compute_hash(
            &notice.notice_id,
            &notice.serial_number,
            &notice.reason,
        );
        if expected != notice.notice_hash {
            return Err(TrustChainError::InvalidRequest {
                reason: format!(
                    "Notice hash mismatch for '{}'",
                    notice.notice_id
                ),
            });
        }

        let notice_id = notice.notice_id.clone();
        let peer_ca_id = notice.issuing_ca_id.clone();

        // Store the received notice
        let mut received = self.received_notices.write().await;
        received.insert(notice.serial_number.clone(), notice);

        // Generate acknowledgement
        let mut outbox = self.outbox.write().await;
        outbox.push(PropagationMessage::AcknowledgeRevocation {
            peer_ca_id,
            notice_id: notice_id.clone(),
        });

        info!(
            "Accepted revocation notice '{}', acknowledgement queued",
            notice_id
        );
        Ok(())
    }

    /// Mark a peer as having received (propagated) a notice.
    pub async fn mark_propagated(
        &self,
        notice_id: &str,
        peer_ca_id: &str,
    ) -> TrustChainResult<()> {
        let mut propagations = self.propagations.write().await;
        let prop = propagations.get_mut(notice_id).ok_or_else(|| {
            TrustChainError::InvalidRequest {
                reason: format!("Notice '{}' not found", notice_id),
            }
        })?;
        if let Some(status) = prop.peer_status.get_mut(peer_ca_id) {
            *status = PropagationStatus::Propagated;
        }
        Ok(())
    }

    /// Mark a peer as having confirmed receipt of a notice.
    pub async fn mark_confirmed(
        &self,
        notice_id: &str,
        peer_ca_id: &str,
    ) -> TrustChainResult<()> {
        let mut propagations = self.propagations.write().await;
        let prop = propagations.get_mut(notice_id).ok_or_else(|| {
            TrustChainError::InvalidRequest {
                reason: format!("Notice '{}' not found", notice_id),
            }
        })?;
        if let Some(status) = prop.peer_status.get_mut(peer_ca_id) {
            *status = PropagationStatus::Confirmed;
        }
        Ok(())
    }

    /// Get the propagation status for a specific notice.
    pub async fn get_propagation_status(
        &self,
        notice_id: &str,
    ) -> Option<HashMap<String, PropagationStatus>> {
        let propagations = self.propagations.read().await;
        propagations
            .get(notice_id)
            .map(|p| p.peer_status.clone())
    }

    /// Drain the outbound message queue.
    pub async fn drain_outbox(&self) -> Vec<PropagationMessage> {
        let mut outbox = self.outbox.write().await;
        std::mem::take(&mut *outbox)
    }

    /// Check if a serial number was revoked by a received notice.
    pub async fn is_remotely_revoked(&self, serial_number: &str) -> bool {
        self.received_notices
            .read()
            .await
            .contains_key(serial_number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_broadcast_to_peers() {
        let propagator = RevocationPropagator::new("local-ca".to_string());

        propagator.add_peer("peer-alpha".to_string()).await;
        propagator.add_peer("peer-beta".to_string()).await;

        let notice_id = propagator
            .broadcast_revocation("serial-001".to_string(), "compromised".to_string())
            .await
            .expect("test: broadcast");

        // Check outbox has messages for both peers
        let messages = propagator.drain_outbox().await;
        assert_eq!(messages.len(), 2);

        let peer_ids: Vec<String> = messages
            .iter()
            .filter_map(|m| match m {
                PropagationMessage::BroadcastRevocation { peer_ca_id, .. } => {
                    Some(peer_ca_id.clone())
                }
                _ => None,
            })
            .collect();
        assert!(peer_ids.contains(&"peer-alpha".to_string()));
        assert!(peer_ids.contains(&"peer-beta".to_string()));

        // Check propagation status
        let status = propagator
            .get_propagation_status(&notice_id)
            .await
            .expect("test: status exists");
        assert_eq!(
            status.get("peer-alpha"),
            Some(&PropagationStatus::Pending)
        );
        assert_eq!(
            status.get("peer-beta"),
            Some(&PropagationStatus::Pending)
        );
    }

    #[tokio::test]
    async fn test_receive_and_acknowledge() {
        let propagator = RevocationPropagator::new("local-ca".to_string());

        let notice = RevocationNotice::new(
            "serial-remote-1".to_string(),
            "superseded".to_string(),
            "remote-ca".to_string(),
        );

        propagator
            .receive_revocation(notice.clone())
            .await
            .expect("test: receive");

        // The serial should now be marked as remotely revoked
        assert!(propagator.is_remotely_revoked("serial-remote-1").await);

        // An ack should be in the outbox
        let messages = propagator.drain_outbox().await;
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            PropagationMessage::AcknowledgeRevocation {
                peer_ca_id,
                notice_id,
            } => {
                assert_eq!(peer_ca_id, "remote-ca");
                assert_eq!(notice_id, &notice.notice_id);
            }
            other => unreachable!("Expected AcknowledgeRevocation, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_tampered_notice_rejected() {
        let propagator = RevocationPropagator::new("local-ca".to_string());

        let mut notice = RevocationNotice::new(
            "serial-tamper".to_string(),
            "key compromise".to_string(),
            "remote-ca".to_string(),
        );

        // Tamper with the reason
        notice.reason = "different reason".to_string();

        let err = propagator.receive_revocation(notice).await;
        assert!(err.is_err(), "Tampered notice should be rejected");
    }

    #[tokio::test]
    async fn test_propagation_status_transitions() {
        let propagator = RevocationPropagator::new("local-ca".to_string());
        propagator.add_peer("peer-1".to_string()).await;

        let notice_id = propagator
            .broadcast_revocation("serial-status".to_string(), "test".to_string())
            .await
            .expect("test: broadcast");

        // Pending -> Propagated
        propagator
            .mark_propagated(&notice_id, "peer-1")
            .await
            .expect("test: mark propagated");

        let status = propagator
            .get_propagation_status(&notice_id)
            .await
            .expect("test: status");
        assert_eq!(
            status.get("peer-1"),
            Some(&PropagationStatus::Propagated)
        );

        // Propagated -> Confirmed
        propagator
            .mark_confirmed(&notice_id, "peer-1")
            .await
            .expect("test: mark confirmed");

        let status = propagator
            .get_propagation_status(&notice_id)
            .await
            .expect("test: status");
        assert_eq!(
            status.get("peer-1"),
            Some(&PropagationStatus::Confirmed)
        );
    }
}
