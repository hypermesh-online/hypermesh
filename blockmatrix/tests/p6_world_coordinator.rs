// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Worlds hardening — the atomic two-set reconciliation primitive.
//!
//! World formation touches two independent membership sets that must never
//! disagree about a held shard's world:
//!   - `ngauge::WorldManager` — shard→world + node→worlds (the TRUE mapping).
//!   - `WorldIsolationGate` — the fetch gate's admitted-worlds view.
//!
//! [`WorldCoordinator`] owns both and mutates them as one ordered unit
//! (admit-before-migrate on form, remap-before-revoke on merge). These tests
//! prove there is **no desync window**: at every observable point a held
//! shard's true-world fetch is accepted — and, non-vacuously, that reversing the
//! order re-opens exactly that window.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use blockmatrix::network::isolation::WorldIsolationGate;
use blockmatrix::network::trust::NetworkType;
use blockmatrix::network::world_coordinator::WorldCoordinator;
use hypermesh_lib::{ContentHash, MatrixPosition, GLOBAL_WORLD};
use ngauge::collective_intel::HotspotAlert;
use ngauge::WorldManager;

fn shard(seed: u8) -> ContentHash {
    ContentHash([seed; 32])
}

fn hotspot(congestion: f64) -> HotspotAlert {
    HotspotAlert {
        center: MatrixPosition {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        congestion_ratio: congestion,
        affected_nodes: 4,
        severity: "high".to_string(),
    }
}

// ── The invariant, under concurrency ─────────────────────────────────────────

/// Drive FORM then MERGE repeatedly while a pool of concurrent readers hammers
/// `check_fetch` on the held shards. The invariant — *a held shard's true-world
/// fetch is never rejected* — must hold at EVERY observable point, so no reader
/// ever sees a stranded fetch, no matter where its read lands relative to the
/// two-set mutation.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn no_desync_window_under_concurrent_form_merge() {
    let coord = Arc::new(
        WorldCoordinator::mount(GLOBAL_WORLD, NetworkType::P2P)
            .await
            .expect("test: mount coordinator"),
    );
    // A wide held set: FORM migrates every one of these into the child, so each
    // reader pass walks a long run of shards that are momentarily child-mapped.
    // That turns the tiny per-shard vulnerable interval (read `world = child`
    // from the manager, then consult the gate) into dozens of interleave targets
    // per pass — so a reintroduced two-independent-reads `check_fetch` is caught
    // on essentially every run, not 1-in-40.
    let held: Vec<ContentHash> = (1u8..=48).map(shard).collect();

    let stop = Arc::new(AtomicBool::new(false));
    let strandings = Arc::new(AtomicU64::new(0));

    // Reader pool: for every held shard, `check_fetch` resolves the shard's TRUE
    // world and consults the gate as one observation. A rejection of a held
    // shard is a desync window — record it (never panic inside the racing task,
    // so the count is a clean assertion at the end).
    //
    // The pool is kept a little above the worker-thread count so several readers
    // are always churning the gate's `admitted` lock; when a merge's `revoke`
    // takes that lock's write side, any reader that already read `world = child`
    // and is now queued for the (write-preferring) read side wakes to the
    // post-revoke set and strands — the exact cross-set window. Readers stay well
    // below the thread count of a starved driver, so the driver keeps firing
    // merges (the scarce event), which is what actually drives the collision.
    let mut readers = Vec::new();
    for _ in 0..6 {
        let c = coord.clone();
        let s = stop.clone();
        let strand = strandings.clone();
        let shards = held.clone();
        readers.push(tokio::spawn(async move {
            let mut observations: u64 = 0;
            while !s.load(Ordering::Relaxed) {
                for sh in &shards {
                    if c.check_fetch(sh).await.is_err() {
                        strand.fetch_add(1, Ordering::Relaxed);
                    }
                    observations += 1;
                }
                tokio::task::yield_now().await;
            }
            observations
        }));
    }

    // Driver: many form→merge cycles, yielding between the two so readers are
    // scheduled to observe the boundary transition mid-flight. More cycles =
    // more merges = more chances for a reader's two-set read to straddle the
    // remap+revoke and pile onto the gate's admitted lock exactly as it is held
    // for the revoke.
    for _ in 0..120_000 {
        if let Some(formation) = coord
            .form(GLOBAL_WORLD, &hotspot(0.95), &held)
            .await
            .expect("test: form ok")
        {
            tokio::task::yield_now().await;
            coord
                .merge(formation.child)
                .await
                .expect("test: merge ok");
        }
        tokio::task::yield_now().await;
    }

    stop.store(true, Ordering::Relaxed);
    let mut total_observations: u64 = 0;
    for r in readers {
        total_observations += r.await.expect("test: reader joins");
    }

    assert!(
        total_observations > 0,
        "readers must have actually observed the two sets during the race"
    );
    assert_eq!(
        strandings.load(Ordering::Relaxed),
        0,
        "a held shard's true-world fetch was rejected — the two membership sets desynced"
    );
}

/// A tighter, deterministic interleave: between every step of a form→merge
/// cycle, synchronously assert the invariant for both a migrated shard and an
/// unassigned one. This pins the property at each transition point without
/// relying on the scheduler.
#[tokio::test]
async fn invariant_holds_at_every_step_of_a_cycle() {
    let coord = WorldCoordinator::mount(GLOBAL_WORLD, NetworkType::P2P)
        .await
        .expect("test: mount");
    let hot = vec![shard(1), shard(2)];
    let cold = shard(9);

    // Step 0 — single world: everything resolves to home, accepted.
    assert!(coord.check_fetch(&hot[0]).await.is_ok());
    assert!(coord.check_fetch(&cold).await.is_ok());

    // Step 1 — after FORM: migrated shard maps to child AND the gate admits it;
    // the unassigned shard is untouched.
    let formation = coord
        .form(GLOBAL_WORLD, &hotspot(0.95), &hot)
        .await
        .expect("test: form ok")
        .expect("test: hotspot forms child");
    let child = formation.child;
    assert_eq!(coord.world_of(&hot[0]).await, child);
    assert!(
        coord.check_fetch(&hot[0]).await.is_ok(),
        "post-form: a migrated shard's true-world fetch must be accepted"
    );
    assert_eq!(coord.world_of(&cold).await, GLOBAL_WORLD);
    assert!(coord.check_fetch(&cold).await.is_ok());

    // Step 2 — after MERGE: shard maps back to parent AND is still accepted; the
    // dissolved child is now foreign to the gate.
    coord
        .merge(child)
        .await
        .expect("test: merge ok")
        .expect("test: child mergeable");
    assert_eq!(coord.world_of(&hot[0]).await, GLOBAL_WORLD);
    assert!(
        coord.check_fetch(&hot[0]).await.is_ok(),
        "post-merge: the reabsorbed shard's true-world fetch must be accepted"
    );
    assert!(
        coord.gate().check_fetch(child, &shard(1)).await.is_err(),
        "post-merge: the dissolved child world is foreign and rejected"
    );
}

// ── Non-vacuity: reverse the order and the window re-opens ────────────────────
//
// The coordinator only implements the CORRECT order, so to prove the tests above
// are non-vacuous we assemble the WRONG order by hand from the exact same
// exposed seam the coordinator composes (`WorldManager::plan_formation` /
// `commit_formation` + `WorldIsolationGate::admit_world`/`revoke_world`) and show
// the invariant DOES break at the intermediate point. This is the precise window
// the coordinator's ordering closes.

/// WRONG order on FORM — **migrate-before-admit**: commit the shard→child
/// mapping before the gate admits the child. Between the two, a held shard's
/// true-world fetch is REJECTED — a stranded fetch the coordinator prevents by
/// admitting first.
#[tokio::test]
async fn wrong_order_migrate_before_admit_strands_a_fetch() {
    let mut wm = WorldManager::new(GLOBAL_WORLD);
    let gate = WorldIsolationGate::mount(GLOBAL_WORLD, NetworkType::P2P)
        .await
        .expect("test: mount gate");
    let hot = vec![shard(1), shard(2)];

    let plan = wm
        .plan_formation(GLOBAL_WORLD, &hotspot(0.95), &hot)
        .expect("test: plan a formation");
    let child = plan.child();

    // WRONG: migrate FIRST — the shard→child mapping is now observable...
    wm.commit_formation(plan);
    let observed_world = wm.world_of(&hot[0]);
    assert_eq!(observed_world, child);

    // ...but the gate has not admitted `child` yet → the held shard's true-world
    // fetch is stranded. This is the desync window the coordinator closes.
    assert!(
        gate.check_fetch(observed_world, &hot[0]).await.is_err(),
        "migrate-before-admit MUST strand a held shard (proves the test is non-vacuous)"
    );

    // For contrast: admitting first (the coordinator's order) would have made
    // this same observation safe.
    gate.admit_world(child, NetworkType::P2P)
        .await
        .expect("test: admit child");
    assert!(gate.check_fetch(observed_world, &hot[0]).await.is_ok());
}

/// WRONG order on MERGE — **revoke-before-remap**: revoke the child on the gate
/// before the shards are re-mapped back to the parent. Between the two, the
/// shard still maps to the now-revoked child → its fetch is REJECTED. The
/// coordinator prevents this by re-mapping first.
#[tokio::test]
async fn wrong_order_revoke_before_remap_strands_a_fetch() {
    let mut wm = WorldManager::new(GLOBAL_WORLD);
    let gate = WorldIsolationGate::mount(GLOBAL_WORLD, NetworkType::P2P)
        .await
        .expect("test: mount gate");
    let hot = vec![shard(1), shard(2)];

    // Form correctly so there is a child to merge.
    let plan = wm
        .plan_formation(GLOBAL_WORLD, &hotspot(0.95), &hot)
        .expect("test: plan");
    let child = plan.child();
    gate.admit_world(child, NetworkType::P2P)
        .await
        .expect("test: admit child");
    wm.commit_formation(plan);
    assert!(gate.check_fetch(wm.world_of(&hot[0]), &hot[0]).await.is_ok());

    // WRONG: revoke FIRST — the gate drops `child`...
    gate.revoke_world(child).await;
    // ...but the shard still maps to `child` (not yet re-mapped) → stranded.
    let observed_world = wm.world_of(&hot[0]);
    assert_eq!(observed_world, child);
    assert!(
        gate.check_fetch(observed_world, &hot[0]).await.is_err(),
        "revoke-before-remap MUST strand a held shard (proves the test is non-vacuous)"
    );

    // For contrast: re-mapping first (the coordinator's order) resolves the
    // shard to the parent, which the gate still admits.
    let merge = wm.merge_child(child).expect("test: merge child");
    assert_eq!(merge.child, child);
    assert_eq!(wm.world_of(&hot[0]), GLOBAL_WORLD);
    assert!(gate.check_fetch(wm.world_of(&hot[0]), &hot[0]).await.is_ok());
}

/// The coordinator is shared across reader tasks via `Arc` in the concurrency
/// test, so it must be `Send + Sync`. Pin that at compile time.
#[test]
fn coordinator_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<WorldCoordinator>();
}
