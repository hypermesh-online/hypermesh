// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Cluster E, phase 3 (STEP 6) — "shards belong to their network".
//!
//! A peer may serve/fetch a shard only if it is authenticated for that shard's
//! (asset's) network. On today's model the asset is registered on the serving
//! node's chain, so the asset's network IS the serving node's network, and the
//! operative gate on the shard-serve path (`authorize_shard_fetch`
//! → `verify_shard_proof_binding` → `verify_peer_access`) is:
//!
//!   requester's authenticated NetworkId == serving node's NetworkId
//!
//! This test drives that gate directly with two networks:
//!   * a peer authenticated for network **A** is REJECTED serving/fetching a
//!     shard when the serving node (and thus the asset) is in network **B**, and
//!   * the SAME peer is ALLOWED when the serving node is in network **A**.
//!
//! Under a single joined network the two ids coincide and the gate always
//! admits — the enforcement is a no-op until a second network exists (which is
//! exactly what the STEP 5 re-key preserves). The finer per-asset-SCOPE gate
//! (PUBLIC/Global vs BOUNDED, for a node holding assets from multiple networks)
//! is deferred to the asset_id.rs `NetworkScope` reshape — see the seam note in
//! `network/message_handlers/peer_connection.rs::authorize_shard_fetch`.

use blockmatrix::network::peer_auth::{
    new_authenticated_peers, register_authenticated_peer, verify_shard_proof_binding,
    AuthenticatedPeer,
};

/// Build an authenticated peer belonging to `network` with a non-empty bound
/// PoS proof (so `verify_shard_proof_binding` sees a real handshake).
fn peer_in_network(node_id: &str, network: &str) -> AuthenticatedPeer {
    AuthenticatedPeer {
        node_id: node_id.to_string(),
        pubkey: vec![0xAB; 32],
        coordinate: (0, 0, 0),
        // Ingress boundary: the peer's wire network label → canonical NetworkId,
        // the SAME converter the gate applies to the serving node's id.
        network_id: hypermesh_lib::NetworkId::from_wire_str(network),
        authenticated_at: std::time::Instant::now(),
        proof_bytes: vec![0xCD; 64],
    }
}

#[tokio::test]
async fn network_a_peer_rejected_for_network_b_shard_allowed_for_network_a() {
    let peers = new_authenticated_peers();

    // A peer that completed the bilateral handshake as a member of network A.
    assert!(
        register_authenticated_peer(&peers, peer_in_network("peer-a", "network-A")).await,
        "peer with a bound proof + pubkey must register",
    );

    // Serving node is in network B: the shard's asset lives on B's chain, so the
    // network-A peer is NOT authenticated for the asset's network → REJECTED.
    assert!(
        !verify_shard_proof_binding(&peers, "peer-a", "network-B").await,
        "a network-A peer must be REFUSED a shard served by a network-B node",
    );

    // Serving node is in network A: same network as the peer → ALLOWED.
    assert!(
        verify_shard_proof_binding(&peers, "peer-a", "network-A").await,
        "a same-network (A) peer must be ALLOWED to fetch the shard",
    );
}

#[tokio::test]
async fn single_network_gate_is_a_no_op() {
    // Under one joined network every id resolves to the same canonical
    // NetworkId, so the gate always admits an authenticated peer — the STEP 6
    // enforcement is provably inert until a second network exists.
    let peers = new_authenticated_peers();
    assert!(
        register_authenticated_peer(&peers, peer_in_network("peer-x", "trustnet-test")).await,
    );
    assert!(
        verify_shard_proof_binding(&peers, "peer-x", "trustnet-test").await,
        "the only joined network must always admit its own authenticated peer",
    );
}

#[tokio::test]
async fn unregistered_peer_is_refused_regardless_of_network() {
    // Defense-in-depth: a peer that never handshook is refused for any network.
    let peers = new_authenticated_peers();
    assert!(
        !verify_shard_proof_binding(&peers, "ghost", "network-A").await,
        "an unauthenticated peer must never pass the shard-serve gate",
    );
}
