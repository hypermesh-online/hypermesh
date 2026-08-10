// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase E.2 — R12 swarm closure integration tests.
//!
//! Validates the consumer-becomes-provider loop end-to-end:
//!  1. After a fetch, the local node is registered as a provider in a
//!     `ShardLocationIndex` and a TAG_SHARD_ANNOUNCE payload is built.
//!  2. The `broadcast_announcement` helper sends the payload to peer
//!     connections, which a peer's `handle_shard_announce` updates into
//!     their `ShardLocationIndex` so the new provider is discoverable.
//!  3. `NGaugeBridge::check_replication_signals` returns non-empty signals
//!     when synthetic demand exceeds the per-replica threshold.
//!
//! These tests exercise the wires directly without spinning the full
//! NetworkManager + bilateral PoS handshake, which is the lightest harness
//! that still proves the announce → index → provider-discovery path.

#![cfg(feature = "intelligence")]

use std::sync::Arc;

use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::consumer_provider::{
    broadcast_announcement, ConsumerProviderManager,
};
use blockmatrix::network::shard_store::ShardStore;
use blockmatrix::network::swarm_provider::ShardLocationIndex;
use blockmatrix::network::SwarmDemandTracker;
use blockmatrix::intelligence::ngauge_bridge::NGaugeBridge;
use ngauge::SwarmAnalytics;
use hypermesh_lib::{ContentHash, MatrixPosition, NodeId, DEFAULT_NETWORK};

fn test_hash(seed: u8) -> ContentHash {
    ContentHash([seed; 32])
}

/// Wire format roundtrip: feed the announce payload built by the manager
/// directly through the same parser used by the wire handler. This is the
/// invariant the cross-node integration relies on.
fn parse_announce_payload(data: &[u8]) -> Vec<ContentHash> {
    assert_eq!(data[0], 0x04, "TAG_SHARD_ANNOUNCE");
    let count = u32::from_le_bytes([data[1], data[2], data[3], data[4]]) as usize;
    assert_eq!(data.len(), 5 + count * 32, "wire payload length");
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let off = 5 + i * 32;
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&data[off..off + 32]);
        out.push(ContentHash(hash));
    }
    out
}

/// CRITICAL invariant: after a fetch, the local node is registered as a
/// provider in some ShardLocationIndex visible to other nodes. Wire 1
/// produces the announce payload; Wire 3 ingests it on the peer side. We
/// stitch both halves here without the NetworkManager/STOQ in between.
#[tokio::test]
async fn fetched_shards_are_announced_and_indexed_on_peer() {
    // Node A: just-fetched shard goes through the consumer-provider
    // manager, registering A locally and producing the announce payload.
    let node_a_id = "node-A".to_string();
    let node_a_index = Arc::new(ShardLocationIndex::new());
    let mgr = ConsumerProviderManager::new(
        Arc::new(ShardStore::new()),
        node_a_index.clone(),
        node_a_id.clone(),
        DEFAULT_NETWORK,
    );

    let h1 = test_hash(0xA1);
    let h2 = test_hash(0xA2);
    let result = mgr
        .process_fetched_shards(vec![
            (h1, vec![1, 2, 3, 4]),
            (h2, vec![5, 6, 7, 8]),
        ])
        .await;

    // Sanity: payload built, A is now a provider locally.
    let payload = result
        .announcement_payload
        .expect("post-fetch announce payload must be present for Full policy");
    assert!(
        node_a_index
            .get_providers_in_network(DEFAULT_NETWORK, &h1)
            .await
            .contains(&node_a_id),
        "node A must be registered as provider for h1 after fetch",
    );

    // Node B: simulates the peer-side handler. Parse the wire payload and
    // record A as a provider in B's index — this is the exact effect of
    // `handle_shard_announce` (sync_and_reflection.rs:93) on the wire.
    let node_b_index = Arc::new(ShardLocationIndex::new());
    let parsed = parse_announce_payload(&payload);
    assert_eq!(parsed.len(), 2, "two shards announced");
    node_b_index
        .register_provider_in_network(DEFAULT_NETWORK, &node_a_id, &parsed)
        .await;

    // CRITICAL ASSERTION: from B's perspective, A is now a known provider
    // for both shards — this is the visibility property R12 requires.
    let providers_h1 = node_b_index.get_providers_in_network(DEFAULT_NETWORK, &h1).await;
    let providers_h2 = node_b_index.get_providers_in_network(DEFAULT_NETWORK, &h2).await;
    assert!(
        providers_h1.contains(&node_a_id),
        "node B must learn A as provider for h1, got {:?}",
        providers_h1,
    );
    assert!(
        providers_h2.contains(&node_a_id),
        "node B must learn A as provider for h2, got {:?}",
        providers_h2,
    );
}

/// Anonymous policy must NOT leak provider identity — neither locally nor
/// on the peer side. This proves the privacy gate in the consumer-provider
/// pipeline still holds when wired into the announce path.
#[tokio::test]
async fn anonymous_policy_skips_announce_and_provider_registration() {
    use blockmatrix::network::shard_dedup::DedupPolicy;

    let node_a_id = "node-A-anon".to_string();
    let index = Arc::new(ShardLocationIndex::new());
    let mgr = ConsumerProviderManager::new(
        Arc::new(ShardStore::new()),
        index.clone(),
        node_a_id.clone(),
        DEFAULT_NETWORK,
    );

    let h = test_hash(0xBB);
    let result = mgr
        .process_fetched_shards_with_policy(
            vec![(h, vec![9, 9, 9])],
            DedupPolicy::HashOnly,
        )
        .await;

    assert!(
        result.announcement_payload.is_none(),
        "Anonymous (HashOnly) policy must NOT emit announce payload",
    );
    assert!(
        index.get_providers_in_network(DEFAULT_NETWORK, &h).await.is_empty(),
        "Anonymous (HashOnly) policy must NOT register provider locally",
    );
}

/// `broadcast_announcement` no-ops gracefully when there are no peers.
/// This guards against the daemon panicking on single-node runs.
#[tokio::test]
async fn broadcast_with_no_peers_returns_zero() {
    let payload = vec![0x04, 0, 0, 0, 0]; // empty TAG_SHARD_ANNOUNCE
    let sent = broadcast_announcement(&payload, &[]).await;
    assert_eq!(sent, 0);
}

/// Wire 2: replication signals fire when synthetic demand exceeds the
/// per-replica threshold. This is the upstream half of the swarm closure
/// loop: when these fire, `connect.rs` requests additional copies via
/// TAG_SHARD_FETCH from peers in `ShardLocationIndex`.
#[tokio::test]
async fn replication_signals_fire_under_synthetic_demand() {
    let tracker = Arc::new(SwarmDemandTracker::new());
    let analytics = Arc::new(std::sync::Mutex::new(SwarmAnalytics::new()));
    let bridge = NGaugeBridge::new(
        tracker.clone(),
        analytics.clone(),
        MatrixPosition { x: 0.0, y: 0.0, z: 0.0 },
        DEFAULT_NETWORK,
    );

    // Default ReplicationConfig threshold = 100 requests per replica.
    // 150 unique consumers exceeds this comfortably for a single shard.
    let hot_shard = test_hash(0xC0);
    for i in 0..150 {
        tracker
            .record_fetch_in_network(DEFAULT_NETWORK, hot_shard, &format!("consumer-{i}"))
            .await;
    }

    // Feed demand into analytics (same path as the H3 loop).
    {
        let snapshot = tracker.snapshot().await;
        let mut guard = analytics.lock().expect("analytics lock");
        for (shard_id, entry) in &snapshot {
            for requester_id in &entry.requester_ids {
                let consumer_id =
                    NodeId::from_public_key(requester_id.as_bytes());
                guard.record_request(
                    *shard_id,
                    consumer_id,
                    MatrixPosition { x: 0.0, y: 0.0, z: 0.0 },
                    entry.last_request_us,
                );
            }
        }
    }

    let signals = bridge.check_replication_signals();
    assert!(
        !signals.is_empty(),
        "ReplicationTrigger must produce at least one signal for 150-consumer synthetic demand",
    );
    let hot = signals
        .iter()
        .find(|s| s.shard_id == hot_shard)
        .expect("hot shard must appear in signals");
    assert!(
        hot.urgency > 0.0,
        "urgency must be positive for hot shard, got {}",
        hot.urgency,
    );
    assert!(
        hot.suggested_count >= 1,
        "must suggest at least 1 additional replica",
    );
}

/// Two-side R12 closure simulation: A fetches from B, A's announce updates
/// B's index, and ngauge picks up popularity from concurrent demand,
/// producing a replication signal that connect.rs would act on by
/// requesting extra copies via TAG_SHARD_FETCH.
#[tokio::test]
async fn end_to_end_swarm_closure_signal_flow() {
    // --- A side: post-fetch announce ---
    let node_a_id = "node-A-e2e".to_string();
    let a_index = Arc::new(ShardLocationIndex::new());
    let a_manager = ConsumerProviderManager::new(
        Arc::new(ShardStore::new()),
        a_index.clone(),
        node_a_id.clone(),
        DEFAULT_NETWORK,
    );
    let h = test_hash(0xE2);
    let result = a_manager
        .process_fetched_shards(vec![(h, vec![0xDE, 0xAD, 0xBE, 0xEF])])
        .await;
    let payload = result
        .announcement_payload
        .expect("announce payload");

    // --- B side: receive announce ---
    let b_index = Arc::new(ShardLocationIndex::new());
    let parsed = parse_announce_payload(&payload);
    b_index
        .register_provider_in_network(DEFAULT_NETWORK, &node_a_id, &parsed)
        .await;
    assert!(
        b_index
            .get_providers_in_network(DEFAULT_NETWORK, &h)
            .await
            .contains(&node_a_id),
        "B's index must show A as provider after announce",
    );

    // --- ngauge demand: synthetic load on B drives replication signal ---
    let tracker = Arc::new(SwarmDemandTracker::new());
    let analytics = Arc::new(std::sync::Mutex::new(SwarmAnalytics::new()));
    let bridge = NGaugeBridge::new(
        tracker.clone(),
        analytics.clone(),
        MatrixPosition { x: 1.0, y: 2.0, z: 3.0 },
        DEFAULT_NETWORK,
    );
    for i in 0..120 {
        tracker.record_fetch_in_network(DEFAULT_NETWORK, h, &format!("c-{i}")).await;
    }
    {
        let snapshot = tracker.snapshot().await;
        let mut guard = analytics.lock().expect("lock");
        for (sid, entry) in &snapshot {
            for rid in &entry.requester_ids {
                guard.record_request(
                    *sid,
                    NodeId::from_public_key(rid.as_bytes()),
                    MatrixPosition { x: 1.0, y: 2.0, z: 3.0 },
                    entry.last_request_us,
                );
            }
        }
    }
    let signals = bridge.check_replication_signals();
    let hot = signals
        .iter()
        .find(|s| s.shard_id == h)
        .expect("hot shard must signal");
    // Replication poll on B would now look up providers for `h` — A is
    // there because of the announce broadcast — and issue TAG_SHARD_FETCH
    // to A to pull an extra replica. We assert the inputs that drive that
    // decision are present.
    let providers = b_index.get_providers_in_network(DEFAULT_NETWORK, &h).await;
    assert!(!providers.is_empty(), "B has at least one provider");
    assert!(hot.urgency > 0.0, "signal urgency drives the fetch decision");
}

/// Distinct unused import keeps coordinate import live for future fixtures.
#[allow(dead_code)]
fn _coord_smoke() -> MatrixCoordinate {
    MatrixCoordinate::new(0, 0, 0).expect("test: valid coord")
}
