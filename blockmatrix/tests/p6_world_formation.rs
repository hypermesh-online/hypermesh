// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! P6 — worlds actually FORM, and the isolation gate is fed the shard's TRUE
//! world (not the home constant P5 used).
//!
//! This is the cross-crate proof that Part A (NGauge [`WorldManager`]) and
//! Part C (the blockmatrix [`WorldIsolationGate`]) compose into the safety
//! property from VISION.md §5.5:
//!
//! **Forming a world must not strand its legitimate holders.** After a world
//! forms, a member node fetching a shard in that world is ACCEPTED; a node
//! fetching a shard in a world it is NOT a member of is REJECTED; and a shard
//! never assigned to any child stays in the parent and is unaffected.

use blockmatrix::network::isolation::WorldIsolationGate;
use blockmatrix::network::trust::NetworkType;
use hypermesh_lib::{ContentHash, MatrixPosition, NetworkId, GLOBAL_WORLD};
use ngauge::collective_intel::{CoverageGap, HotspotAlert};
use ngauge::{WorldAction, WorldManager};
use ngauge::collective_intel::NetworkInsight;

fn shard(seed: u8) -> ContentHash {
    ContentHash([seed; 32])
}

fn hotspot(congestion: f64) -> HotspotAlert {
    HotspotAlert {
        center: MatrixPosition { x: 1.0, y: 2.0, z: 3.0 },
        congestion_ratio: congestion,
        affected_nodes: 4,
        severity: "high".to_string(),
    }
}

const FOREIGN_WORLD: NetworkId = NetworkId([0xF0; 16]);

// ── Behaviour 1 (form) + the SAFETY property, wired through the real gate ────

/// A hotspot forms a child world; the WorldManager is the shard→world source;
/// the gate — fed that TRUE world — accepts a member node's own migrated shard
/// and rejects a foreign-world shard. This is the property a wrong mapping
/// would break.
#[tokio::test]
async fn formed_world_does_not_strand_same_world_traffic() {
    // A single-world node begins rooted at GLOBAL_WORLD.
    let mut wm = WorldManager::new(GLOBAL_WORLD);
    let gate = WorldIsolationGate::mount(GLOBAL_WORLD, NetworkType::P2P)
        .await
        .expect("test: mount home-world gate");

    let hot = vec![shard(1), shard(2)];
    let cold = shard(9);

    // Before any world forms, every shard is in GLOBAL and the gate is a no-op.
    assert_eq!(wm.world_of(&hot[0]), GLOBAL_WORLD);
    assert!(gate
        .check_fetch(wm.world_of(&hot[0]), &hot[0])
        .await
        .is_ok());

    // A congested hotspot forms a child world; the node holds the hot chunk, so
    // the WorldManager migrates those shards down AND the node joins the child.
    let formation = wm
        .form_from_hotspot(GLOBAL_WORLD, &hotspot(0.95), &hot)
        .expect("test: hotspot forms a child world");
    let child = formation.child;

    // The node joins the child on the gate too (Part C: connect.rs mirrors the
    // WorldManager membership into the gate as worlds form).
    gate.admit_world(child, NetworkType::P2P)
        .await
        .expect("test: admit formed child world");

    // SAFETY: the shard's TRUE world is now the child — fed from the real
    // WorldManager map, not the home constant. A member node's own shard is
    // still ACCEPTED (not stranded).
    assert_eq!(wm.world_of(&hot[0]), child);
    assert!(
        gate.check_fetch(wm.world_of(&hot[0]), &hot[0]).await.is_ok(),
        "a legitimate holder's same-world fetch must be ACCEPTED after formation"
    );

    // An unassigned shard stays in the parent world and is unaffected.
    assert_eq!(wm.world_of(&cold), GLOBAL_WORLD);
    assert!(gate.check_fetch(wm.world_of(&cold), &cold).await.is_ok());

    // ISOLATION: a shard in a world the node never joined is REJECTED.
    assert!(
        gate.check_fetch(FOREIGN_WORLD, &shard(0xF1)).await.is_err(),
        "a shard in a foreign world must be REJECTED"
    );

    // Exactly one boundary violation — the foreign-world attempt.
    let violations = gate.violations().await;
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].source_network, GLOBAL_WORLD);
    assert_eq!(violations[0].destination_network, FOREIGN_WORLD);
}

// ── Behaviour 2 (merge) ──────────────────────────────────────────────────────

/// A coverage gap reabsorbs the emergent child; its shards return to the parent
/// and — after the node leaves the dissolved world — the gate rejects a fetch
/// for the (now non-existent) child world.
#[tokio::test]
async fn coverage_gap_merges_child_and_gate_follows() {
    let mut wm = WorldManager::new(GLOBAL_WORLD);
    let gate = WorldIsolationGate::mount(GLOBAL_WORLD, NetworkType::P2P)
        .await
        .expect("test: mount gate");

    let formation = wm
        .form_from_hotspot(GLOBAL_WORLD, &hotspot(0.9), &[shard(1)])
        .expect("test: form child");
    let child = formation.child;
    gate.admit_world(child, NetworkType::P2P)
        .await
        .expect("test: admit child");
    assert!(gate.check_fetch(child, &shard(1)).await.is_ok());

    // The region thins out → the child is reabsorbed.
    let gap = CoverageGap {
        center: MatrixPosition { x: 1.0, y: 2.0, z: 3.0 },
        radius: 10.0,
        node_count: 0,
    };
    let merges = wm.consume_coverage_gap(GLOBAL_WORLD, &gap);
    assert_eq!(merges.len(), 1);
    assert_eq!(merges[0].child, child);

    // Shard is back in the parent world; the node leaves the child on the gate.
    assert_eq!(wm.world_of(&shard(1)), GLOBAL_WORLD);
    gate.revoke_world(child).await;

    // The reabsorbed shard is fetched via its parent world (accepted); the
    // dissolved child world is now foreign and rejected.
    assert!(gate.check_fetch(wm.world_of(&shard(1)), &shard(1)).await.is_ok());
    assert!(gate.check_fetch(child, &shard(1)).await.is_err());
}

// ── Behaviour 3 (pin) + Behaviour 4 (nesting) ───────────────────────────────

/// A pinned operator boundary is never split above (never auto-merged), while an
/// emergent inner world nests WITHIN it and resolves up the parent chain.
#[tokio::test]
async fn pinned_outer_and_emergent_inner_coexist_via_nesting() {
    let mut wm = WorldManager::new(GLOBAL_WORLD);
    let op_world = NetworkId([0x0A; 16]);
    wm.pin_world(op_world, Some(GLOBAL_WORLD));

    // Behaviour 3: a coverage gap must never collapse the pinned boundary.
    let gap = CoverageGap {
        center: MatrixPosition { x: 0.0, y: 0.0, z: 0.0 },
        radius: 10.0,
        node_count: 0,
    };
    assert!(wm
        .consume_coverage_gap(GLOBAL_WORLD, &gap)
        .iter()
        .all(|m| m.child != op_world));
    assert!(wm.is_pinned(op_world));
    assert!(wm.admits(op_world));

    // Behaviour 4: an emergent inner world forms WITHIN the pinned outer world
    // (via the insight router — CollectiveIntelligence's live consumer) and its
    // parent chain resolves inner → pinned-outer → GLOBAL.
    let action = wm.apply_insight(
        op_world,
        &NetworkInsight::HotspotAlert(hotspot(0.9)),
        &[shard(5)],
    );
    let inner = match action {
        WorldAction::Formed(f) => {
            assert_eq!(f.parent, op_world);
            f.child
        }
        other => panic!("test: expected Formed, got {other:?}"),
    };
    assert!(!wm.is_pinned(inner), "emergent inner world is not pinned");
    assert_eq!(
        wm.parent_chain(inner),
        vec![inner, op_world, GLOBAL_WORLD],
        "nesting resolves emergent-inner through pinned-outer to the root"
    );
}
