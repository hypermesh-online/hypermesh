// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Per-network isolation — the real primitive, tested directly.
//!
//! These two tests assert genuine per-network isolation against the kept
//! enforcer. Because the enforcer is unmounted (no live isolation is wired),
//! they are `DefaultIsolationManager`-level unit tests over `validate_packet` —
//! the real primitive — plus the per-`NetworkId`-keyed `ShardLocationIndex`
//! (also real and kept):
//!
//! 1. `validate_packet` rejects a cross-network packet and accepts a
//!    same-network one, recording exactly one audited boundary violation.
//! 2. A shard registered only in network B is invisible to a network-A lookup,
//!    and the enforcer independently rejects the cross-network packet.

use blockmatrix::network::isolation::{
    zero_hash, DefaultIsolationManager, IsolationManager, Packet,
};
use blockmatrix::network::swarm_provider::ShardLocationIndex;
use blockmatrix::network::trust::NetworkType;
use hypermesh_lib::{ContentHash, NetworkId};

fn shard(seed: u8) -> ContentHash {
    ContentHash([seed; 32])
}

const NETWORK_A: NetworkId = NetworkId([0xAA; 16]);
const NETWORK_B: NetworkId = NetworkId([0xBB; 16]);

/// The kept enforcer rejects a fetch/packet that crosses a network boundary and
/// accepts one that stays within the node's own network — proving the boundary
/// primitive is real, not decorative.
#[tokio::test]
async fn cross_network_packet_is_rejected_same_network_accepted() {
    let mgr = DefaultIsolationManager::new();
    mgr.configure_network(NETWORK_A, NetworkType::P2P)
        .await
        .expect("test: configure network A");

    // Same-network A -> A: accepted.
    let same = Packet::new(NETWORK_A, NETWORK_A, zero_hash());
    assert!(
        mgr.validate_packet(&same).await.is_ok(),
        "a same-network packet must be accepted"
    );

    // Cross-network A -> B: rejected.
    let cross = Packet::new(NETWORK_A, NETWORK_B, zero_hash());
    assert!(
        mgr.validate_packet(&cross).await.is_err(),
        "a network-A node must be rejected sending to network B"
    );

    // Exactly one boundary violation was logged, for the cross-network attempt.
    let violations = mgr.check_violations().await;
    assert_eq!(violations.len(), 1, "exactly one cross-network rejection");
    assert_eq!(violations[0].source_network, NETWORK_A);
    assert_eq!(violations[0].destination_network, NETWORK_B);

    let stats = mgr.get_stats().await;
    assert_eq!(stats.packets_rejected, 1);
}

/// Isolation grounded in the real, network-keyed provider index: a shard
/// registered *only* in network B is invisible to a network-A lookup, and the
/// enforcer independently rejects the cross-network packet. Two independent
/// barriers — the keyed lookup and the boundary enforcer.
#[tokio::test]
async fn network_b_shard_is_isolated_from_network_a() {
    let index = ShardLocationIndex::new();
    let mgr = DefaultIsolationManager::new();
    mgr.configure_network(NETWORK_A, NetworkType::P2P)
        .await
        .expect("test: configure network A");

    let s = shard(0xBC);
    // Register a provider for the shard ONLY in network B.
    index
        .register_provider_in_network(NETWORK_B, "peer-in-network-b", &[s])
        .await;

    // Barrier 1: the network-A lookup does not see the network-B provider.
    assert!(
        index.get_providers_in_network(NETWORK_A, &s).await.is_empty(),
        "network-A lookup must not surface a network-B provider"
    );
    // The provider IS present under network B (the registration was real).
    assert_eq!(
        index.get_providers_in_network(NETWORK_B, &s).await,
        vec!["peer-in-network-b".to_string()],
    );

    // Barrier 2: the enforcer rejects a cross-network packet A -> B.
    let cross = Packet::new(NETWORK_A, NETWORK_B, zero_hash());
    assert!(
        mgr.validate_packet(&cross).await.is_err(),
        "the enforcer must reject a network-A -> network-B packet"
    );
}
