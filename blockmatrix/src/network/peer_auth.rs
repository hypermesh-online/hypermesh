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
pub async fn register_authenticated_peer(
    peers: &AuthenticatedPeers,
    peer: AuthenticatedPeer,
) {
    let short_id = &peer.node_id[..8.min(peer.node_id.len())];
    debug!(
        "Registering authenticated peer {} (network={})",
        short_id, peer.network_id,
    );
    peers.write().await.insert(peer.node_id.clone(), peer);
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
        register_authenticated_peer(&peers, peer).await;
        assert!(is_authenticated(&peers, "abc123def456").await);
    }

    #[tokio::test]
    async fn test_remove_peer() {
        let peers = new_authenticated_peers();
        register_authenticated_peer(&peers, make_peer("node1234", "net-1")).await;
        assert!(is_authenticated(&peers, "node1234").await);

        remove_authenticated_peer(&peers, "node1234").await;
        assert!(!is_authenticated(&peers, "node1234").await);
    }

    #[tokio::test]
    async fn test_network_scoping() {
        let peers = new_authenticated_peers();
        register_authenticated_peer(&peers, make_peer("nodeAAAA", "net-alpha")).await;

        assert!(is_same_network(&peers, "nodeAAAA", "net-alpha").await);
        assert!(!is_same_network(&peers, "nodeAAAA", "net-beta").await);
    }

    #[tokio::test]
    async fn test_verify_peer_access_authenticated_same_network() {
        let peers = new_authenticated_peers();
        register_authenticated_peer(&peers, make_peer("nodeX123", "main")).await;

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
        register_authenticated_peer(&peers, make_peer("nodeY456", "other-net")).await;

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
}
