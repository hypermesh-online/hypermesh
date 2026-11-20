// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! PoS-gated peer authentication tracking.
//!
//! After a successful bilateral handshake, peers are recorded in
//! [`AuthenticatedPeers`]. All block/shard/sync message handlers
//! check this map before processing incoming data.

use hypermesh_lib::NetworkId;
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
    /// Local STOQ session identity, never supplied by peer metadata.
    pub connection_id: String,
    /// FALCON-1024 public key bytes.
    pub pubkey: Vec<u8>,
    /// Matrix coordinate of the peer.
    pub coordinate: (i32, i32, i32),
    /// Canonical network the peer belongs to.
    ///
    /// Derived at the handshake ingress boundary from the peer's advertised
    /// wire string via [`NetworkId::from_wire_str`]. Comparisons are against our
    /// own network id put through the SAME converter, so two nodes that agree on
    /// a wire string agree here.
    pub network_id: NetworkId,
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
    if peer.node_id.is_empty()
        || peer.connection_id.is_empty()
        || peer.proof_bytes.is_empty()
        || peer.pubkey.is_empty()
    {
        warn!("Rejecting peer — missing identity, session, proof or public key");
        return false;
    }
    debug!(
        "Registering authenticated peer {} (network={})",
        peer.node_id, peer.network_id
    );
    peers.write().await.insert(peer.node_id.clone(), peer);
    true
}

/// Remove only the registration owned by this local STOQ connection.
pub async fn remove_authenticated_peer(
    peers: &AuthenticatedPeers,
    node_id: &str,
    connection_id: &str,
) {
    let mut map = peers.write().await;
    if !connection_id.is_empty()
        && map
            .get(node_id)
            .is_some_and(|peer| peer.connection_id == connection_id)
    {
        map.remove(node_id);
        warn!("Removed authenticated peer {}", node_id);
    }
}

/// Check whether a node_id is in the authenticated peers map.
pub async fn is_authenticated(peers: &AuthenticatedPeers, node_id: &str) -> bool {
    peers.read().await.contains_key(node_id)
}

/// The canonical [`NetworkId`] an authenticated peer belongs to, if known.
///
/// Returns `None` when the peer has not completed a bilateral handshake (not in
/// the map). Used to attribute a peer's shard demand/announcements to ITS
/// network rather than flattening to the default.
pub async fn peer_network_id(peers: &AuthenticatedPeers, node_id: &str) -> Option<NetworkId> {
    peers.read().await.get(node_id).map(|p| p.network_id)
}

/// Check whether a node_id belongs to the given network.
///
/// `our_network_id` is the caller's wire-form network string; it is mapped to
/// its canonical [`NetworkId`] through the single ingress converter before the
/// comparison, so this stays byte-for-byte equivalent to the old
/// string-equality check under a single network.
pub async fn is_same_network(
    peers: &AuthenticatedPeers,
    node_id: &str,
    our_network_id: &str,
) -> bool {
    let ours = NetworkId::from_wire_str(our_network_id);
    match peers.read().await.get(node_id) {
        Some(peer) => peer.network_id == ours,
        None => false,
    }
}

/// Authorize this local session against a single current registry row.
/// Status/membership helpers above do not authorize a connection.
pub async fn verify_peer_access(
    peers: &AuthenticatedPeers,
    node_id: &str,
    connection_id: &str,
    our_network_id: &str,
) -> bool {
    authorize_session(peers, node_id, connection_id, our_network_id, false).await
}

/// Require current-session authorization and a bound proof from the SAME row.
/// The caller separately checks the shard's chain anchor; this is not a
/// cryptographic or asset-specific proof validation implementation.
pub async fn verify_shard_proof_binding(
    peers: &AuthenticatedPeers,
    node_id: &str,
    connection_id: &str,
    our_network_id: &str,
) -> bool {
    authorize_session(peers, node_id, connection_id, our_network_id, true).await
}

async fn authorize_session(
    peers: &AuthenticatedPeers,
    node_id: &str,
    connection_id: &str,
    our_network_id: &str,
    require_proof: bool,
) -> bool {
    let mut map = peers.write().await;
    let Some(peer) = map.get(node_id) else {
        warn!("Rejected message from unauthenticated peer {}", node_id);
        return false;
    };
    if connection_id.is_empty()
        || peer.connection_id != connection_id
        || peer.network_id != NetworkId::from_wire_str(our_network_id)
    {
        warn!("Rejected peer {} — session or network mismatch", node_id);
        return false;
    }
    if peer.authenticated_at.elapsed() > AUTH_TTL {
        map.remove(node_id);
        warn!("Rejected peer {} — authentication expired", node_id);
        return false;
    }
    !require_proof || !peer.proof_bytes.is_empty()
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
mod tests;

pub(super) mod demand;

/// Synchronous commit-time revalidation; caller holds this exact auth row.
fn shard_demand_network(
    peer: &AuthenticatedPeer,
    connection_id: &str,
    our_network_id: &str,
) -> Option<NetworkId> {
    let valid = !connection_id.is_empty()
        && peer.connection_id == connection_id
        && peer.network_id == NetworkId::from_wire_str(our_network_id)
        && peer.authenticated_at.elapsed() <= AUTH_TTL
        && !peer.proof_bytes.is_empty();
    valid.then_some(peer.network_id)
}
