// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! PoS-gated peer authentication tracking.
//!
//! After a successful bilateral handshake, peers are recorded in
//! [`AuthenticatedPeers`]. All block/shard/sync message handlers
//! check this map before processing incoming data.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Maximum time a peer remains authenticated before re-handshake is required.
const AUTH_TTL: std::time::Duration = std::time::Duration::from_secs(3600);

/// A peer that has completed bilateral PoS handshake verification.
#[derive(Clone, Debug)]
pub struct AuthenticatedPeer {
    /// Node ID (hex hash of genesis block).
    pub node_id: String,
    /// FALCON-1024 public key bytes.
    pub pubkey: Vec<u8>,
    /// Matrix coordinate of the peer.
    pub coordinate: (i32, i32, i32),
    /// Network ID the peer belongs to.
    pub network_id: String,
    /// When the peer was authenticated.
    pub authenticated_at: std::time::Instant,
    /// Last verified PoS proof bytes (from handshake).
    pub proof_bytes: Vec<u8>,
}

/// Thread-safe map of authenticated peers, keyed by node_id.
///
/// This is the single source of truth for "who is allowed to send
/// us blocks and shards." Populated after successful bilateral
/// handshake, consulted on every incoming message.
pub type AuthenticatedPeers = Arc<RwLock<HashMap<String, AuthenticatedPeer>>>;

/// Create a new empty authenticated peers map.
pub fn new_authenticated_peers() -> AuthenticatedPeers {
    Arc::new(RwLock::new(HashMap::new()))
}

/// Register a peer as authenticated after successful bilateral handshake.
///
/// Validates that the peer's proof bytes and public key are non-empty
/// before registration. Empty proofs indicate a handshake that did not
/// complete bilateral PoS verification (R11).
pub async fn register_authenticated_peer(
    peers: &AuthenticatedPeers,
    peer: AuthenticatedPeer,
) -> bool {
    let short_id = &peer.node_id[..8.min(peer.node_id.len())];

    // Reject peers with empty proof bytes — bilateral PoS must have completed
    if peer.proof_bytes.is_empty() {
        warn!(
            "Rejecting peer {} — empty proof_bytes (bilateral PoS handshake incomplete)",
            short_id,
        );
        return false;
    }

    // Reject peers with empty public key — identity not established
    if peer.pubkey.is_empty() {
        warn!(
            "Rejecting peer {} — empty pubkey (FALCON-1024 identity not established)",
            short_id,
        );
        return false;
    }

    // Reject peers with empty node_id
    if peer.node_id.is_empty() {
        warn!("Rejecting peer — empty node_id");
        return false;
    }

    debug!(
        "Registering authenticated peer {} (network={}, proof={} bytes, pubkey={} bytes)",
        short_id, peer.network_id, peer.proof_bytes.len(), peer.pubkey.len(),
    );
    peers.write().await.insert(peer.node_id.clone(), peer);
    true
}

/// Remove a peer from the authenticated set.
pub async fn remove_authenticated_peer(
    peers: &AuthenticatedPeers,
    node_id: &str,
) {
    let short_id = &node_id[..8.min(node_id.len())];
    if peers.write().await.remove(node_id).is_some() {
        warn!("Removed authenticated peer {}", short_id);
    }
}

/// Check whether a node_id is in the authenticated peers map.
pub async fn is_authenticated(
    peers: &AuthenticatedPeers,
    node_id: &str,
) -> bool {
    peers.read().await.contains_key(node_id)
}

/// Check whether a node_id belongs to the given network_id.
pub async fn is_same_network(
    peers: &AuthenticatedPeers,
    node_id: &str,
    our_network_id: &str,
) -> bool {
    match peers.read().await.get(node_id) {
        Some(peer) => peer.network_id == our_network_id,
        None => false,
    }
}

/// Verify that a sender is both authenticated and in the same network.
///
/// Returns `true` if the peer passes both checks. Logs a warning on
/// rejection so operators can see unauthorized access attempts.
pub async fn verify_peer_access(
    peers: &AuthenticatedPeers,
    node_id: &str,
    our_network_id: &str,
) -> bool {
    let short_id = &node_id[..8.min(node_id.len())];

    let expired = {
        let map = peers.read().await;
        match map.get(node_id) {
            None => {
                warn!(
                    "Rejected message from unauthenticated peer {}",
                    short_id,
                );
                return false;
            }
            Some(peer) if peer.network_id != our_network_id => {
                warn!(
                    "Rejected message from peer {} (network '{}' != ours '{}')",
                    short_id, peer.network_id, our_network_id,
                );
                return false;
            }
            Some(peer) => peer.authenticated_at.elapsed() > AUTH_TTL,
        }
    };

    if expired {
        warn!(
            "Rejected message from peer {} (auth expired after {:?})",
            short_id, AUTH_TTL,
        );
        peers.write().await.remove(node_id);
        return false;
    }

    true
}

/// Validate that a peer's claimed public key is consistent with their
/// known rotation chain. Returns `true` if the key is either:
/// 1. The original key (no rotations recorded), or
/// 2. The latest `new_key_fingerprint` in the rotation chain.
pub async fn validate_key_continuity(
    rotation_chains: &tokio::sync::RwLock<HashMap<String, Vec<serde_json::Value>>>,
    node_id: &str,
    claimed_key_fingerprint: &str,
) -> bool {
    let chains = rotation_chains.read().await;
    match chains.get(node_id) {
        None => true, // No rotation history = first contact, accept any key
        Some(chain) if chain.is_empty() => true,
        Some(chain) => {
            // Latest rotation's new_key should match claimed key
            if let Some(latest) = chain.last() {
                let latest_new = latest
                    .get("new_key_fingerprint")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if latest_new == claimed_key_fingerprint {
                    return true;
                }
                warn!(
                    node = node_id,
                    expected = latest_new,
                    claimed = claimed_key_fingerprint,
                    "Key continuity check FAILED — claimed key doesn't match rotation chain",
                );
                false
            } else {
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_peer(node_id: &str, network_id: &str) -> AuthenticatedPeer {
        AuthenticatedPeer {
            node_id: node_id.to_string(),
            pubkey: vec![1, 2, 3],
            coordinate: (0, 0, 0),
            network_id: network_id.to_string(),
            authenticated_at: std::time::Instant::now(),
            proof_bytes: vec![4, 5, 6],
        }
    }

    #[tokio::test]
    async fn test_register_and_check() {
        let peers = new_authenticated_peers();
        let peer = make_peer("abc123def456", "net-1");

        assert!(!is_authenticated(&peers, "abc123def456").await);
        assert!(register_authenticated_peer(&peers, peer).await);
        assert!(is_authenticated(&peers, "abc123def456").await);
    }

    #[tokio::test]
    async fn test_register_rejects_empty_proof() {
        let peers = new_authenticated_peers();
        let mut peer = make_peer("emptyproof1", "net-1");
        peer.proof_bytes = Vec::new();

        assert!(!register_authenticated_peer(&peers, peer).await);
        assert!(!is_authenticated(&peers, "emptyproof1").await);
    }

    #[tokio::test]
    async fn test_register_rejects_empty_pubkey() {
        let peers = new_authenticated_peers();
        let mut peer = make_peer("emptypubk1", "net-1");
        peer.pubkey = Vec::new();

        assert!(!register_authenticated_peer(&peers, peer).await);
        assert!(!is_authenticated(&peers, "emptypubk1").await);
    }

    #[tokio::test]
    async fn test_register_rejects_empty_node_id() {
        let peers = new_authenticated_peers();
        let peer = AuthenticatedPeer {
            node_id: String::new(),
            pubkey: vec![1, 2, 3],
            coordinate: (0, 0, 0),
            network_id: "net-1".to_string(),
            authenticated_at: std::time::Instant::now(),
            proof_bytes: vec![4, 5, 6],
        };

        assert!(!register_authenticated_peer(&peers, peer).await);
    }

    #[tokio::test]
    async fn test_remove_peer() {
        let peers = new_authenticated_peers();
        assert!(register_authenticated_peer(&peers, make_peer("node1234", "net-1")).await);
        assert!(is_authenticated(&peers, "node1234").await);

        remove_authenticated_peer(&peers, "node1234").await;
        assert!(!is_authenticated(&peers, "node1234").await);
    }

    #[tokio::test]
    async fn test_network_scoping() {
        let peers = new_authenticated_peers();
        assert!(register_authenticated_peer(&peers, make_peer("nodeAAAA", "net-alpha")).await);

        assert!(is_same_network(&peers, "nodeAAAA", "net-alpha").await);
        assert!(!is_same_network(&peers, "nodeAAAA", "net-beta").await);
    }

    #[tokio::test]
    async fn test_verify_peer_access_authenticated_same_network() {
        let peers = new_authenticated_peers();
        assert!(register_authenticated_peer(&peers, make_peer("nodeX123", "main")).await);

        assert!(verify_peer_access(&peers, "nodeX123", "main").await);
    }

    #[tokio::test]
    async fn test_verify_peer_access_unauthenticated() {
        let peers = new_authenticated_peers();
        assert!(!verify_peer_access(&peers, "unknown1", "main").await);
    }

    #[tokio::test]
    async fn test_verify_peer_access_wrong_network() {
        let peers = new_authenticated_peers();
        assert!(register_authenticated_peer(&peers, make_peer("nodeY456", "other-net")).await);

        assert!(!verify_peer_access(&peers, "nodeY456", "main").await);
    }

    #[tokio::test]
    async fn expired_auth_is_rejected() {
        let peers = new_authenticated_peers();
        let mut peer = make_peer("expiredAA", "net-1");
        peer.authenticated_at =
            std::time::Instant::now() - std::time::Duration::from_secs(7200);

        peers.write().await.insert(peer.node_id.clone(), peer);

        assert!(!verify_peer_access(&peers, "expiredAA", "net-1").await);
        assert!(!peers.read().await.contains_key("expiredAA"));
    }

    // ── Key continuity tests ────────────────────────────────────────

    #[tokio::test]
    async fn test_validate_key_continuity_no_history() {
        let chains = tokio::sync::RwLock::new(HashMap::new());
        // No rotation chain at all → accept any key
        assert!(validate_key_continuity(&chains, "node-aaa", "fp-xyz").await);
    }

    #[tokio::test]
    async fn test_validate_key_continuity_empty_chain() {
        let chains = tokio::sync::RwLock::new(HashMap::new());
        chains
            .write()
            .await
            .insert("node-bbb".to_string(), Vec::new());
        // Empty chain → accept any key
        assert!(validate_key_continuity(&chains, "node-bbb", "fp-xyz").await);
    }

    #[tokio::test]
    async fn test_validate_key_continuity_matches_latest() {
        let chains = tokio::sync::RwLock::new(HashMap::new());
        let entry = serde_json::json!({
            "old_key_fingerprint": "fp-A",
            "new_key_fingerprint": "fp-B",
            "reason": "Scheduled",
        });
        chains
            .write()
            .await
            .insert("node-ccc".to_string(), vec![entry]);
        // Claimed key matches latest rotation's new_key → true
        assert!(validate_key_continuity(&chains, "node-ccc", "fp-B").await);
    }

    #[tokio::test]
    async fn test_validate_key_continuity_stale_key() {
        let chains = tokio::sync::RwLock::new(HashMap::new());
        let entry = serde_json::json!({
            "old_key_fingerprint": "fp-A",
            "new_key_fingerprint": "fp-B",
            "reason": "Scheduled",
        });
        chains
            .write()
            .await
            .insert("node-ddd".to_string(), vec![entry]);
        // Claimed key is the old key, not the new one → false
        assert!(!validate_key_continuity(&chains, "node-ddd", "fp-A").await);
    }

    #[tokio::test]
    async fn test_validate_key_continuity_multi_rotation() {
        let chains = tokio::sync::RwLock::new(HashMap::new());
        let entry1 = serde_json::json!({
            "old_key_fingerprint": "fp-A",
            "new_key_fingerprint": "fp-B",
        });
        let entry2 = serde_json::json!({
            "old_key_fingerprint": "fp-B",
            "new_key_fingerprint": "fp-C",
        });
        chains
            .write()
            .await
            .insert("node-eee".to_string(), vec![entry1, entry2]);
        // Only the latest new_key (fp-C) should pass
        assert!(validate_key_continuity(&chains, "node-eee", "fp-C").await);
        assert!(!validate_key_continuity(&chains, "node-eee", "fp-B").await);
        assert!(!validate_key_continuity(&chains, "node-eee", "fp-A").await);
    }
}
