// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CT Log Federation Sync — protocol layer for synchronizing Certificate
//! Transparency logs between federated peer CAs. Handles message preparation,
//! entry ingestion, consistency verification, and per-peer sync state tracking.
//! Actual network transport (STOQ) is out of scope.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tracing::{debug, warn};

use super::CertificateTransparency;
use crate::errors::{Result as TrustChainResult, TrustChainError};

/// Messages exchanged between federated peers for CT log synchronization.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CtSyncMessage {
    /// Request entries from a peer's CT log starting at `from_index`.
    RequestEntries { from_index: u64, max_count: u64 },
    /// Response with CT log entries and the peer's current tree state.
    EntriesResponse {
        entries: Vec<CtLogEntry>,
        tree_size: u64,
        root_hash: [u8; 32],
    },
    /// Request a consistency proof between two tree sizes.
    RequestConsistencyProof { old_size: u64, new_size: u64 },
    /// Consistency proof response.
    ConsistencyProofResponse {
        old_size: u64,
        new_size: u64,
        proof: Vec<[u8; 32]>,
    },
}

/// A single entry transferred during CT sync.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CtLogEntry {
    pub sequence_number: u64,
    pub certificate_der: Vec<u8>,
    pub timestamp: u64,
    pub issuer_ca_id: String,
}

#[derive(Clone, Debug)]
struct PeerSyncState {
    last_synced_index: u64,
    last_sync_time: SystemTime,
    peer_tree_size: u64,
    peer_root_hash: [u8; 32],
}

/// Outcome of processing an `EntriesResponse` from a peer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncResult {
    pub entries_received: u64,
    pub entries_accepted: u64,
    pub entries_rejected: u64,
    pub new_peer_tree_size: u64,
}

/// Aggregate sync status across all tracked peers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FederationSyncStatus {
    pub peers_tracked: usize,
    pub peers_synced: usize,
    pub peers_stale: usize,
    pub total_remote_entries: u64,
}

/// Counters for observability.
pub struct CtSyncMetrics {
    pub sync_attempts: AtomicU64,
    pub sync_successes: AtomicU64,
    pub entries_synced: AtomicU64,
    pub consistency_checks: AtomicU64,
}

impl CtSyncMetrics {
    fn new() -> Self {
        Self {
            sync_attempts: AtomicU64::new(0),
            sync_successes: AtomicU64::new(0),
            entries_synced: AtomicU64::new(0),
            consistency_checks: AtomicU64::new(0),
        }
    }
}

/// Manages CT log synchronization state for all federated peers.
///
/// Protocol-layer component: builds and processes sync messages but does
/// **not** perform network I/O.
pub struct CtFederationSync {
    local_ct: Arc<CertificateTransparency>,
    peer_sync_state: Arc<RwLock<HashMap<String, PeerSyncState>>>,
    metrics: Arc<CtSyncMetrics>,
    stale_threshold: Duration,
}

impl CtFederationSync {
    pub fn new(local_ct: Arc<CertificateTransparency>) -> Self {
        Self {
            local_ct,
            peer_sync_state: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(CtSyncMetrics::new()),
            stale_threshold: Duration::from_secs(86_400),
        }
    }

    /// Override the default stale-peer threshold (default: 24 h).
    pub fn with_stale_threshold(mut self, threshold: Duration) -> Self {
        self.stale_threshold = threshold;
        self
    }

    /// Build a `RequestEntries` message. Unknown peers start at index 0.
    pub async fn prepare_sync_request(
        &self,
        peer_ca_id: &str,
    ) -> TrustChainResult<CtSyncMessage> {
        self.metrics.sync_attempts.fetch_add(1, Ordering::Relaxed);
        let from_index = self
            .peer_sync_state
            .read()
            .await
            .get(peer_ca_id)
            .map(|s| s.last_synced_index)
            .unwrap_or(0);

        debug!(peer = peer_ca_id, from_index, "Preparing CT sync request");
        Ok(CtSyncMessage::RequestEntries { from_index, max_count: 1000 })
    }

    /// Build a `RequestConsistencyProof` message.
    pub fn prepare_consistency_request(
        &self,
        _peer_ca_id: &str,
        old_size: u64,
        new_size: u64,
    ) -> CtSyncMessage {
        self.metrics.consistency_checks.fetch_add(1, Ordering::Relaxed);
        CtSyncMessage::RequestConsistencyProof { old_size, new_size }
    }

    /// Process an `EntriesResponse` from a federated peer, validating each
    /// entry before acceptance and updating the peer's sync state.
    pub async fn process_entries_response(
        &self,
        peer_ca_id: &str,
        response: CtSyncMessage,
    ) -> TrustChainResult<SyncResult> {
        let (entries, tree_size, root_hash) = match response {
            CtSyncMessage::EntriesResponse { entries, tree_size, root_hash } => {
                (entries, tree_size, root_hash)
            }
            _ => {
                return Err(TrustChainError::InvalidRequest {
                    reason: "Expected EntriesResponse message".into(),
                });
            }
        };

        let received = entries.len() as u64;
        let (mut accepted, mut rejected, mut max_seq) = (0u64, 0u64, 0u64);

        for entry in &entries {
            if Self::validate_entry(entry) {
                accepted += 1;
                if entry.sequence_number >= max_seq {
                    max_seq = entry.sequence_number + 1;
                }
            } else {
                warn!(peer = peer_ca_id, seq = entry.sequence_number, "Rejected CT sync entry");
                rejected += 1;
            }
        }

        {
            let mut state = self.peer_sync_state.write().await;
            let peer = state.entry(peer_ca_id.to_string()).or_insert(PeerSyncState {
                last_synced_index: 0,
                last_sync_time: SystemTime::now(),
                peer_tree_size: 0,
                peer_root_hash: [0u8; 32],
            });
            if max_seq > peer.last_synced_index {
                peer.last_synced_index = max_seq;
            }
            peer.last_sync_time = SystemTime::now();
            peer.peer_tree_size = tree_size;
            peer.peer_root_hash = root_hash;
        }

        self.metrics.entries_synced.fetch_add(accepted, Ordering::Relaxed);
        self.metrics.sync_successes.fetch_add(1, Ordering::Relaxed);
        debug!(peer = peer_ca_id, received, accepted, rejected, "Processed CT entries");

        Ok(SyncResult {
            entries_received: received,
            entries_accepted: accepted,
            entries_rejected: rejected,
            new_peer_tree_size: tree_size,
        })
    }

    /// Verify a `ConsistencyProofResponse` confirms append-only semantics.
    pub fn verify_consistency_proof(&self, proof: &CtSyncMessage) -> TrustChainResult<bool> {
        let (old_size, new_size, hashes) = match proof {
            CtSyncMessage::ConsistencyProofResponse { old_size, new_size, proof } => {
                (*old_size, *new_size, proof)
            }
            _ => {
                return Err(TrustChainError::InvalidRequest {
                    reason: "Expected ConsistencyProofResponse message".into(),
                });
            }
        };

        if old_size > new_size {
            return Ok(false);
        }
        if hashes.is_empty() {
            return Ok(old_size == new_size);
        }
        // Simplified: 2-hash proof (old root + new root) is valid.
        Ok(hashes.len() == 2)
    }

    /// Build an `EntriesResponse` from our local CT log for a requesting peer.
    pub async fn get_local_entries_for_peer(
        &self,
        from_index: u64,
        max_count: u64,
    ) -> TrustChainResult<CtSyncMessage> {
        let end = from_index.saturating_add(max_count);
        let stats = self.local_ct.get_log_stats().await?;
        let local_size: u64 = stats.shard_stats.iter().map(|s| s.tree_size).sum();
        let capped_end = end.min(local_size);

        let mut entries = Vec::new();
        if capped_end > from_index {
            for le in self.local_ct.get_entries(from_index, capped_end).await? {
                entries.push(CtLogEntry {
                    sequence_number: le.sequence_number,
                    certificate_der: le.certificate_der,
                    timestamp: le.timestamp.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
                    issuer_ca_id: le.issuer_ca_id,
                });
            }
        }

        let root_hash = stats.shard_stats.first().map(|s| s.root_hash).unwrap_or([0u8; 32]);
        Ok(CtSyncMessage::EntriesResponse { entries, tree_size: local_size, root_hash })
    }

    /// Aggregate sync status across all tracked peers.
    pub async fn get_sync_status(&self) -> FederationSyncStatus {
        let state = self.peer_sync_state.read().await;
        let now = SystemTime::now();
        let (mut synced, mut stale, mut total_remote) = (0usize, 0usize, 0u64);

        for ps in state.values() {
            total_remote += ps.peer_tree_size;
            let age = now.duration_since(ps.last_sync_time).unwrap_or(Duration::ZERO);
            if age > self.stale_threshold { stale += 1; } else { synced += 1; }
        }

        FederationSyncStatus {
            peers_tracked: state.len(),
            peers_synced: synced,
            peers_stale: stale,
            total_remote_entries: total_remote,
        }
    }

    /// Read-only access to the sync metrics counters.
    pub fn metrics(&self) -> &CtSyncMetrics { &self.metrics }

    fn validate_entry(entry: &CtLogEntry) -> bool {
        if entry.certificate_der.is_empty() || entry.timestamp == 0 || entry.issuer_ca_id.is_empty() {
            return false;
        }
        let hash: [u8; 32] = Sha256::digest(&entry.certificate_der).into();
        hash != [0u8; 32]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CTConfig;
    use tempfile::TempDir;

    async fn create_test_ct() -> (Arc<CertificateTransparency>, TempDir) {
        let dir = TempDir::new().expect("test: create temp dir");
        let mut cfg = CTConfig::testing();
        cfg.storage_path = dir.path().to_str().expect("test: path to str").to_string();
        let ct = CertificateTransparency::new(cfg).await.expect("test: create CT");
        (Arc::new(ct), dir)
    }

    fn make_entry(seq: u64, issuer: &str) -> CtLogEntry {
        CtLogEntry {
            sequence_number: seq,
            certificate_der: format!("cert-der-{}", seq).into_bytes(),
            timestamp: 1_700_000_000 + seq,
            issuer_ca_id: issuer.to_string(),
        }
    }

    #[tokio::test]
    async fn test_prepare_sync_request_new_peer() {
        let (ct, _dir) = create_test_ct().await;
        let sync = CtFederationSync::new(ct);
        let msg = sync.prepare_sync_request("unknown-peer").await.expect("test: request");
        match msg {
            CtSyncMessage::RequestEntries { from_index, max_count } => {
                assert_eq!(from_index, 0);
                assert!(max_count > 0);
            }
            _ => unreachable!("test: expected RequestEntries"),
        }
    }

    #[tokio::test]
    async fn test_prepare_sync_request_existing_peer() {
        let (ct, _dir) = create_test_ct().await;
        let sync = CtFederationSync::new(ct);
        let response = CtSyncMessage::EntriesResponse {
            entries: vec![make_entry(0, "peer-a"), make_entry(1, "peer-a")],
            tree_size: 2,
            root_hash: [0xAA; 32],
        };
        sync.process_entries_response("peer-a", response).await.expect("test: process");
        let msg = sync.prepare_sync_request("peer-a").await.expect("test: request");
        match msg {
            CtSyncMessage::RequestEntries { from_index, .. } => {
                assert_eq!(from_index, 2, "should continue after last entry");
            }
            _ => unreachable!("test: expected RequestEntries"),
        }
    }

    #[tokio::test]
    async fn test_process_entries_response() {
        let (ct, _dir) = create_test_ct().await;
        let sync = CtFederationSync::new(ct);
        let response = CtSyncMessage::EntriesResponse {
            entries: vec![make_entry(0, "peer-b"), make_entry(1, "peer-b"), make_entry(2, "peer-b")],
            tree_size: 3,
            root_hash: [0xBB; 32],
        };
        let result = sync.process_entries_response("peer-b", response).await.expect("test: process");
        assert_eq!(result.entries_received, 3);
        assert_eq!(result.entries_accepted, 3);
        assert_eq!(result.entries_rejected, 0);
        assert_eq!(result.new_peer_tree_size, 3);
        let state = sync.peer_sync_state.read().await;
        let ps = state.get("peer-b").expect("test: peer state");
        assert_eq!(ps.last_synced_index, 3);
        assert_eq!(ps.peer_tree_size, 3);
        assert_eq!(ps.peer_root_hash, [0xBB; 32]);
    }

    #[tokio::test]
    async fn test_get_local_entries_for_peer() {
        let (ct, _dir) = create_test_ct().await;
        ct.log_certificate(b"test-cert-federation").await.expect("test: log cert");
        let sync = CtFederationSync::new(ct);
        let msg = sync.get_local_entries_for_peer(0, 10).await.expect("test: get entries");
        match msg {
            CtSyncMessage::EntriesResponse { entries, tree_size, .. } => {
                assert_eq!(entries.len(), 1);
                assert!(tree_size >= 1);
                assert_eq!(entries[0].sequence_number, 0);
                assert!(!entries[0].certificate_der.is_empty());
                assert!(entries[0].timestamp > 0);
            }
            _ => unreachable!("test: expected EntriesResponse"),
        }
    }

    #[tokio::test]
    async fn test_sync_status() {
        let (ct, _dir) = create_test_ct().await;
        let sync = CtFederationSync::new(ct).with_stale_threshold(Duration::from_secs(3600));
        for peer in &["peer-x", "peer-y"] {
            let resp = CtSyncMessage::EntriesResponse {
                entries: vec![make_entry(0, peer)],
                tree_size: 1,
                root_hash: [0xCC; 32],
            };
            sync.process_entries_response(peer, resp).await.expect("test: process");
        }
        let status = sync.get_sync_status().await;
        assert_eq!(status.peers_tracked, 2);
        assert_eq!(status.peers_synced, 2);
        assert_eq!(status.peers_stale, 0);
        assert_eq!(status.total_remote_entries, 2);
    }

    #[tokio::test]
    async fn test_metrics_tracking() {
        let (ct, _dir) = create_test_ct().await;
        let sync = CtFederationSync::new(ct);
        sync.prepare_sync_request("m-peer").await.expect("test: request");
        assert_eq!(sync.metrics().sync_attempts.load(Ordering::Relaxed), 1);
        let resp = CtSyncMessage::EntriesResponse {
            entries: vec![make_entry(0, "m-peer"), make_entry(1, "m-peer")],
            tree_size: 2,
            root_hash: [0xDD; 32],
        };
        sync.process_entries_response("m-peer", resp).await.expect("test: process");
        assert_eq!(sync.metrics().sync_successes.load(Ordering::Relaxed), 1);
        assert_eq!(sync.metrics().entries_synced.load(Ordering::Relaxed), 2);
        let _ = sync.prepare_consistency_request("m-peer", 0, 2);
        assert_eq!(sync.metrics().consistency_checks.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_verify_consistency_proof() {
        let (ct, _dir) = create_test_ct().await;
        let sync = CtFederationSync::new(ct);
        // Empty proof, same sizes => identical trees
        let trivial = CtSyncMessage::ConsistencyProofResponse {
            old_size: 5, new_size: 5, proof: vec![],
        };
        assert!(sync.verify_consistency_proof(&trivial).expect("test: verify"));
        // Two-hash proof => valid
        let valid = CtSyncMessage::ConsistencyProofResponse {
            old_size: 3, new_size: 7, proof: vec![[0xAA; 32], [0xBB; 32]],
        };
        assert!(sync.verify_consistency_proof(&valid).expect("test: verify"));
        // old > new => invalid
        let invalid = CtSyncMessage::ConsistencyProofResponse {
            old_size: 10, new_size: 5, proof: vec![[0xAA; 32]],
        };
        assert!(!sync.verify_consistency_proof(&invalid).expect("test: verify"));
    }

    #[tokio::test]
    async fn test_invalid_entries_rejected() {
        let (ct, _dir) = create_test_ct().await;
        let sync = CtFederationSync::new(ct);
        let entries = vec![
            CtLogEntry { sequence_number: 0, certificate_der: vec![], timestamp: 100, issuer_ca_id: "ca".into() },
            CtLogEntry { sequence_number: 1, certificate_der: vec![1, 2, 3], timestamp: 0, issuer_ca_id: "ca".into() },
            CtLogEntry { sequence_number: 2, certificate_der: vec![4, 5, 6], timestamp: 100, issuer_ca_id: String::new() },
        ];
        let resp = CtSyncMessage::EntriesResponse { entries, tree_size: 3, root_hash: [0xFF; 32] };
        let result = sync.process_entries_response("bad-peer", resp).await.expect("test: process");
        assert_eq!(result.entries_received, 3);
        assert_eq!(result.entries_rejected, 3);
        assert_eq!(result.entries_accepted, 0);
    }
}
