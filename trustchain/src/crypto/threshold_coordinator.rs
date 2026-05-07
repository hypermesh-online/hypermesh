// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Threshold-signing coordinator for distributed CAs.
//!
//! Drives [`ThresholdSigner::reconstruct_and_sign`] across a federation of
//! peer CAs that hold shares of the same FALCON-1024 root key.  The flow:
//!
//! 1. Caller provides the message to sign and the threshold `t` of `n`
//!    shares required.
//! 2. The coordinator selects `t` (or more) federated peers whose trust
//!    band is at least `Conditional` and broadcasts a sign request over
//!    [`crate::ca::wire_protocol::TAG_CA_SIGN_REQUEST`] (0x31) via the
//!    pluggable [`FederationTransport`].
//! 3. Responses are awaited until either `t` valid shares have arrived
//!    or the deadline elapses.
//! 4. When `t` shares are collected the coordinator hands them to
//!    [`ThresholdSigner::reconstruct_and_sign`] and returns the signature.
//!
//! Production transports wrap STOQ; tests use the in-process
//! [`MockFederationTransport`] below.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use tracing::{debug, warn};
use uuid::Uuid;

use super::threshold::{KeyShare, ThresholdConfig, ThresholdSigner};
use crate::ca::federation::FederationManager;
use crate::ca::trust_provider::PeerCertFingerprint;

/// Errors raised by the threshold-signing coordinator.
#[derive(thiserror::Error, Debug)]
pub enum ThresholdError {
    #[error("threshold-sign: federation has only {available} eligible peers, need at least {needed}")]
    NotEnoughPeers { available: usize, needed: usize },
    #[error("threshold-sign: only {received} valid shares before deadline (needed {needed})")]
    Timeout { received: usize, needed: usize },
    #[error("threshold-sign: transport error: {message}")]
    Transport { message: String },
    #[error("threshold-sign: signing failed: {message}")]
    Signing { message: String },
}

/// A request to peers for a threshold signing share contribution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignRequest {
    /// Random per-request UUID, echoed in [`SignResponse::request_id`].
    pub request_id: Uuid,
    /// Fingerprint of the CA whose key is being reconstructed.
    pub ca_fingerprint: [u8; 32],
    /// Message to sign — the caller is responsible for any pre-hashing.
    pub message: Vec<u8>,
    /// Required threshold `t` of `n`.
    pub threshold: u8,
}

/// A peer's response to a [`SignRequest`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignResponse {
    /// Echoed `request_id` from the originating [`SignRequest`].
    pub request_id: Uuid,
    /// The peer's contribution.  `None` indicates the peer holds no
    /// matching share or refused to contribute.
    pub share: Option<KeyShare>,
}

/// Pluggable transport for federation-level threshold signing.
///
/// Production wraps STOQ; tests use [`MockFederationTransport`].
#[async_trait]
pub trait FederationTransport: Send + Sync {
    /// Broadcast a sign request to the given peer fingerprints.
    async fn broadcast_sign_request(
        &self,
        peers: &[PeerCertFingerprint],
        req: SignRequest,
    ) -> Result<(), ThresholdError>;

    /// Await up to `deadline` for as many responses as possible, but
    /// return as soon as `threshold` valid responses have arrived.
    async fn await_responses(
        &self,
        request_id: Uuid,
        threshold: usize,
        deadline: Duration,
    ) -> Vec<SignResponse>;
}

/// Coordinator that drives `ThresholdSigner` over a federation.
pub struct ThresholdSignCoordinator {
    federation: Arc<FederationManager>,
    transport: Arc<dyn FederationTransport>,
}

impl ThresholdSignCoordinator {
    /// Construct a coordinator backed by the given federation manager
    /// and transport.
    pub fn new(
        federation: Arc<FederationManager>,
        transport: Arc<dyn FederationTransport>,
    ) -> Self {
        Self {
            federation,
            transport,
        }
    }

    /// Threshold-sign `message` using shares of the CA identified by
    /// `ca_fingerprint`.
    ///
    /// Returns the raw FALCON-1024 detached signature bytes, or an
    /// error if not enough valid shares are collected before `deadline`.
    pub async fn sign(
        &self,
        ca_fingerprint: [u8; 32],
        message: &[u8],
        threshold: u8,
        deadline: Duration,
    ) -> Result<Vec<u8>, ThresholdError> {
        // 1. Pick eligible peers.  We include any non-Untrusted peer
        //    that the federation knows about — coarse for alpha, refined
        //    later via the trust provider.
        let peers = self.federation.list_peers().await;
        let eligible: Vec<PeerCertFingerprint> = peers
            .iter()
            .filter(|p| {
                !matches!(
                    p.trust_level,
                    crate::ca::federation::FederationTrustLevel::Untrusted
                )
            })
            .filter_map(|p| derive_peer_fingerprint(&p.public_key))
            .collect();

        if eligible.len() + 1 < threshold as usize {
            // The local node also contributes its own share, hence +1.
            return Err(ThresholdError::NotEnoughPeers {
                available: eligible.len() + 1,
                needed: threshold as usize,
            });
        }

        // 2. Build request.
        let request_id = Uuid::new_v4();
        let req = SignRequest {
            request_id,
            ca_fingerprint,
            message: message.to_vec(),
            threshold,
        };

        // 3. Broadcast.  Errors here are fatal — peers may not have
        //    received the request at all.
        self.transport
            .broadcast_sign_request(&eligible, req.clone())
            .await?;

        // 4. Collect responses.  Always include the locally held share
        //    if present, so a single-CA-with-local-share configuration
        //    still works for tests and bootstrap scenarios.
        let mut shares: Vec<KeyShare> = Vec::new();
        if let Some(local) = self.federation.get_key_share(&ca_fingerprint).await {
            shares.push(local);
        }

        // Hard-cap the wait at `deadline`.
        let needed = threshold as usize;
        let responses = match timeout(
            deadline,
            self.transport
                .await_responses(request_id, needed.saturating_sub(shares.len()), deadline),
        )
        .await
        {
            Ok(rs) => rs,
            Err(_) => Vec::new(),
        };
        for resp in responses {
            if resp.request_id != request_id {
                debug!("threshold-sign: ignoring response with mismatched request_id");
                continue;
            }
            if let Some(share) = resp.share {
                if share.key_fingerprint == ca_fingerprint {
                    shares.push(share);
                    if shares.len() >= needed {
                        break;
                    }
                } else {
                    warn!("threshold-sign: peer returned share with mismatched fingerprint");
                }
            }
        }

        if shares.len() < needed {
            return Err(ThresholdError::Timeout {
                received: shares.len(),
                needed,
            });
        }

        // 5. Reconstruct and sign.  The signer reconstructs the secret
        //    in memory, signs, and immediately drops the secret.
        let signer = ThresholdSigner::new(ThresholdConfig {
            threshold,
            total_shares: shares.len() as u8,
        })
        .map_err(|e| ThresholdError::Signing {
            message: e.to_string(),
        })?;

        signer
            .reconstruct_and_sign(&shares[..needed], message)
            .map_err(|e| ThresholdError::Signing {
                message: e.to_string(),
            })
    }
}

/// SHA-256 fingerprint of a CA public key.
fn derive_peer_fingerprint(public_key: &[u8]) -> Option<PeerCertFingerprint> {
    if public_key.is_empty() {
        return None;
    }
    use sha2::{Digest, Sha256};
    let digest: [u8; 32] = Sha256::digest(public_key).into();
    Some(digest)
}

// ---------------------------------------------------------------------------
// Test transport
// ---------------------------------------------------------------------------

/// In-process test transport that simulates federation peers responding
/// with shares from a pre-loaded map.
pub struct MockFederationTransport {
    /// Per-peer shares to return.  Keyed by peer fingerprint.
    pub responses: std::sync::Mutex<std::collections::HashMap<PeerCertFingerprint, KeyShare>>,
    /// Peers that should "go silent" and not respond.
    pub silent: std::sync::Mutex<std::collections::HashSet<PeerCertFingerprint>>,
    /// Captured requests for assertion.
    pub captured: std::sync::Mutex<Vec<SignRequest>>,
    /// Channel for delivering simulated responses to the
    /// [`FederationTransport::await_responses`] call.
    response_tx: tokio::sync::mpsc::UnboundedSender<SignResponse>,
    response_rx: tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<SignResponse>>,
}

impl MockFederationTransport {
    /// Build a fresh mock transport.
    pub fn new() -> Arc<Self> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Arc::new(Self {
            responses: std::sync::Mutex::new(std::collections::HashMap::new()),
            silent: std::sync::Mutex::new(std::collections::HashSet::new()),
            captured: std::sync::Mutex::new(Vec::new()),
            response_tx: tx,
            response_rx: tokio::sync::Mutex::new(rx),
        })
    }

    /// Pre-load a peer's response.
    pub fn set_share(&self, peer: PeerCertFingerprint, share: KeyShare) {
        self.responses
            .lock()
            .expect("test: mock transport lock")
            .insert(peer, share);
    }

    /// Mark a peer as silent.
    pub fn silence(&self, peer: PeerCertFingerprint) {
        self.silent
            .lock()
            .expect("test: mock transport lock")
            .insert(peer);
    }
}

#[async_trait]
impl FederationTransport for MockFederationTransport {
    async fn broadcast_sign_request(
        &self,
        peers: &[PeerCertFingerprint],
        req: SignRequest,
    ) -> Result<(), ThresholdError> {
        self.captured
            .lock()
            .expect("test: mock transport lock")
            .push(req.clone());

        // Each addressed peer immediately enqueues a response (or stays
        // silent).  For deterministic tests, we enqueue inline.
        let responses = self
            .responses
            .lock()
            .expect("test: mock transport lock")
            .clone();
        let silent = self
            .silent
            .lock()
            .expect("test: mock transport lock")
            .clone();

        for peer in peers {
            if silent.contains(peer) {
                continue;
            }
            let share = responses.get(peer).cloned();
            let resp = SignResponse {
                request_id: req.request_id,
                share,
            };
            // Send is best-effort; failure means rx already dropped.
            let _ = self.response_tx.send(resp);
        }
        Ok(())
    }

    async fn await_responses(
        &self,
        request_id: Uuid,
        threshold: usize,
        deadline: Duration,
    ) -> Vec<SignResponse> {
        let mut out = Vec::new();
        let mut rx = self.response_rx.lock().await;
        let deadline = tokio::time::Instant::now() + deadline;
        while out.len() < threshold {
            let now = tokio::time::Instant::now();
            if now >= deadline {
                break;
            }
            match tokio::time::timeout_at(deadline, rx.recv()).await {
                Ok(Some(resp)) => {
                    if resp.request_id == request_id {
                        out.push(resp);
                    }
                }
                Ok(None) => break, // channel closed
                Err(_) => break,   // deadline reached
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ca::federation::{FederationManager, FederationPolicy};

    fn fed() -> Arc<FederationManager> {
        Arc::new(FederationManager::new(
            "local-ca".into(),
            FederationPolicy::default(),
        ))
    }

    #[tokio::test]
    async fn coordinator_rejects_without_enough_peers() {
        let coordinator = ThresholdSignCoordinator::new(fed(), MockFederationTransport::new());
        let result = coordinator
            .sign([0xAB; 32], b"msg", 3, Duration::from_millis(50))
            .await;
        assert!(matches!(
            result,
            Err(ThresholdError::NotEnoughPeers { .. })
        ));
    }
}
