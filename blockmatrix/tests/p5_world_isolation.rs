// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! P5 — the two-sided world-isolation gate.
//!
//! Proves the mounted boundary enforcer ([`WorldIsolationGate`]) satisfies both
//! halves of the safety property from VISION.md §5.5:
//!
//! 1. **No-op (single world).** A node whose home world is `GLOBAL_WORLD`
//!    accepts every same-world fetch; the [`ShardLocationIndex`] lookup and the
//!    gate behave exactly as before worlds existed. Nothing observable changes.
//! 2. **Isolation (two worlds).** A world-A node is *rejected* when it tries to
//!    fetch a shard that belongs to world B, and *accepted* for its own world.
//!    The gate is real, not decorative.

use blockmatrix::network::isolation::WorldIsolationGate;
use blockmatrix::network::swarm_provider::ShardLocationIndex;
use blockmatrix::network::trust::NetworkType;
use hypermesh_lib::{ContentHash, NetworkId, GLOBAL_WORLD};

fn shard(seed: u8) -> ContentHash {
    ContentHash([seed; 32])
}

const WORLD_A: NetworkId = NetworkId([0xAA; 16]);
const WORLD_B: NetworkId = NetworkId([0xBB; 16]);

// ─── Side 1: no-op (single world) ────────────────────────────────────────────

/// A single-world node's home is `GLOBAL_WORLD`; every same-world fetch is
/// accepted and nothing about the index lookup changes.
#[tokio::test]
async fn single_world_gate_is_a_noop() {
    let gate = WorldIsolationGate::mount(GLOBAL_WORLD, NetworkType::P2P)
        .await
        .expect("test: mount global-world gate");

    let s = shard(1);

    // The only world a single-world node can be handed is its own → accepted.
    assert!(
        gate.check_fetch(GLOBAL_WORLD, &s).await.is_ok(),
        "same-world (GLOBAL_WORLD) fetch must be accepted"
    );

    // No violation was recorded — the gate did not reject legitimate traffic.
    assert!(
        gate.violations().await.is_empty(),
        "a single-world node must record zero isolation violations"
    );
    let stats = gate.stats().await;
    assert_eq!(stats.packets_rejected, 0, "no same-world fetch is rejected");
}

/// The world-keyed [`ShardLocationIndex`] plus the gate together behave, for a
/// single `GLOBAL_WORLD` node, exactly as the pre-world flat index did: a
/// provider registered in `GLOBAL_WORLD` is discoverable and the gate lets the
/// fetch through.
#[tokio::test]
async fn single_world_replication_lookup_unchanged() {
    let index = ShardLocationIndex::new();
    let gate = WorldIsolationGate::mount(GLOBAL_WORLD, NetworkType::P2P)
        .await
        .expect("test: mount global-world gate");

    let s = shard(7);
    index
        .register_provider_in_world(GLOBAL_WORLD, "peer-remote", &[s])
        .await;

    // The gate permits the same-world fetch...
    assert!(gate.check_fetch(GLOBAL_WORLD, &s).await.is_ok());
    // ...and the provider is found via the world-scoped lookup, identical to
    // the flat lookup.
    let providers = index.get_providers_in_world(GLOBAL_WORLD, &s).await;
    assert_eq!(providers, vec!["peer-remote".to_string()]);
    assert_eq!(
        index.get_providers(&s).await,
        providers,
        "GLOBAL_WORLD lookup must equal the flat convenience lookup"
    );
}

// ─── Side 2: isolation (two worlds) ──────────────────────────────────────────

/// A world-A node rejects a fetch for a shard belonging to world B, and accepts
/// a fetch for a shard belonging to its own world. This proves the gate
/// consults the boundary — it is not decorative.
#[tokio::test]
async fn cross_world_fetch_is_rejected_same_world_accepted() {
    let gate = WorldIsolationGate::mount(WORLD_A, NetworkType::P2P)
        .await
        .expect("test: mount world-A gate");

    let foreign = shard(0xB0);
    let own = shard(0xA0);

    // Cross-world: world-A node fetching a world-B shard → REJECTED.
    assert!(
        gate.check_fetch(WORLD_B, &foreign).await.is_err(),
        "a world-A node must be rejected fetching a world-B shard"
    );

    // Same-world: world-A node fetching its own shard → ACCEPTED.
    assert!(
        gate.check_fetch(WORLD_A, &own).await.is_ok(),
        "a world-A node must be accepted fetching its own shard"
    );

    // Exactly one boundary violation was logged, for the cross-world attempt.
    let violations = gate.violations().await;
    assert_eq!(violations.len(), 1, "exactly one cross-world rejection");
    assert_eq!(violations[0].source_network, WORLD_A);
    assert_eq!(violations[0].destination_network, WORLD_B);

    let stats = gate.stats().await;
    assert_eq!(stats.packets_rejected, 1);
}

/// The isolation is grounded in the real, world-keyed provider index: a shard
/// registered *only* in world B is invisible to a world-A lookup, and even if a
/// world-B provider hint were consulted, the gate rejects the fetch. Two
/// independent barriers — the keyed lookup and the mounted gate.
#[tokio::test]
async fn world_b_shard_is_isolated_from_world_a_node() {
    let index = ShardLocationIndex::new();
    let gate = WorldIsolationGate::mount(WORLD_A, NetworkType::P2P)
        .await
        .expect("test: mount world-A gate");

    let s = shard(0xBC);
    // Register a provider for the shard ONLY in world B.
    index
        .register_provider_in_world(WORLD_B, "peer-in-world-b", &[s])
        .await;

    // Barrier 1: the world-A lookup does not see the world-B provider.
    assert!(
        index.get_providers_in_world(WORLD_A, &s).await.is_empty(),
        "world-A lookup must not surface a world-B provider"
    );
    // The provider IS present under world B (the registration was real).
    assert_eq!(
        index.get_providers_in_world(WORLD_B, &s).await,
        vec!["peer-in-world-b".to_string()],
    );

    // Barrier 2: even if a world-B provider hint reached the world-A fetch
    // path, the mounted gate rejects the cross-world fetch outright.
    assert!(
        gate.check_fetch(WORLD_B, &s).await.is_err(),
        "gate must reject a world-A node fetching the world-B shard"
    );
}
