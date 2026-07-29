// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! World-membership reconciliation — the atomic two-set primitive.
//!
//! World formation touches **two independent membership sets** that must never
//! disagree about which world a held shard lives in:
//!
//! - `ngauge::WorldManager` — the authority on *which worlds this node is in*
//!   and *which world each shard is in* (`member_worlds` + `shard_worlds`).
//! - [`WorldIsolationGate`](crate::network::isolation::WorldIsolationGate) — the
//!   fetch gate's view of *which worlds it will accept* (`admitted`).
//!
//! Reconciled only by convention (a caller mutating one then the other in the
//! right order), a `check_fetch` landing in the window between the two
//! mutations can observe a held shard mapped to a world the gate has not yet
//! admitted (on FORM) or has already revoked (on MERGE) — a **stranded fetch**
//! of a shard the node legitimately holds. It is inert only because live
//! formation is not fired; this primitive closes the window *before* it ever is
//! (VISION.md §5.5).
//!
//! ## Crate layering
//! `blockmatrix` depends on `ngauge`, never the reverse — so `WorldManager`
//! cannot reach the gate, but blockmatrix can hold both. The
//! [`WorldCoordinator`] therefore lives here, owns both sets, and is the single
//! path that mutates them, enforcing one ordering rule per direction:
//!
//! - **FORM / split — admit-before-migrate.** `admit_world(child)` on the gate
//!   happens *before* the shards migrate into the child in the `WorldManager`,
//!   so `world_of(shard) == child` is never observable while the gate has not
//!   admitted `child`.
//! - **MERGE — remap-before-revoke.** The child's shards are re-mapped back to
//!   the parent in the `WorldManager` *before* `revoke_world(child)` on the
//!   gate, so a shard never maps to a world the gate has already dropped.
//!
//! ## Why the reader sees no window
//! Ordered publication (admit-before-migrate / remap-before-revoke) makes each
//! *single* set's transition safe, but it is **not** sufficient on its own,
//! because `check_fetch` reads **both** sets — first the `WorldManager`
//! (`world_of`), then the gate (`check_fetch`) — as two separate lock
//! acquisitions. A `merge` landing *between* those two reads can remap the shard
//! to the parent in the `WorldManager` **and** revoke the child on the gate,
//! leaving the reader holding a `world` value (`child`) it captured *before* the
//! remap and testing it against the gate *after* the revoke → a false rejection
//! of a shard the node legitimately holds in the parent. The two-set read must
//! therefore be a **consistent snapshot** with respect to `form`/`merge`.
//!
//! The sequencing lock is an `RwLock`, and that is exactly what closes this
//! cross-set window:
//! - `form`/`merge`/`merge_gap` take `seq.write()` (exclusive — they already
//!   serialize against each other and now against readers too).
//! - `check_fetch` takes `seq.read()` (shared) for the **whole duration** of
//!   both reads, so `world_of` and `gate.check_fetch` see the same generation of
//!   both sets: no `form`/`merge` can interleave between them.
//!
//! The earlier "lock-free readers" design — where `check_fetch` took no
//! sequencing lock and relied on ordered publication alone — was the *cause* of
//! this desync (a reader straddling a merge sees a stale `world` against a
//! post-revoke gate). We deliberately trade that lock-free-reader property for
//! correctness: `check_fetch` is a 30s replication-loop reader (not hot), and
//! `form`/`merge` are rare, so a shared read-lock costs nothing observable while
//! making the two-set read provably atomic. Standalone single-set reads
//! (`world_of`, `admits`, `world_count`) take no sequencing lock — a momentarily
//! stale single value is harmless; only the *combined* gate check needed
//! atomicity, and callers that must gate a fetch go through `check_fetch`.
//! The ordering rules remain load-bearing and testable (reverse either and the
//! per-set window reappears — see `tests/p6_world_coordinator.rs`).
//!
//! ## Scope
//! This is the primitive only. It does **not** fire world formation on the live
//! node: nothing here calls `form`/`merge` on its own, and the E.2 replication
//! poll routes its gate/manager construction through the coordinator purely so
//! there is a single owner of both sets — with no formation ever driven, the
//! coordinator resolves every shard to [`GLOBAL_WORLD`](hypermesh_lib::GLOBAL_WORLD)
//! and `check_fetch` is the same P5 no-op.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use hypermesh_lib::{ContentHash, NetworkId};
use ngauge::collective_intel::{CoverageGap, HotspotAlert};
use ngauge::{WorldFormation, WorldManager, WorldMerge};

use crate::network::isolation::WorldIsolationGate;
use crate::network::trust::NetworkType;

/// Owns both world-membership sets and is the single path that mutates them, so
/// the gate's `admitted` set and the `WorldManager`'s shard→world map cannot
/// desync. See the module docs for the two ordering rules.
pub struct WorldCoordinator {
    /// The shard→world / node→worlds authority (ngauge). Behind an async lock so
    /// a concurrent reader can resolve `world_of` while `form`/`merge` mutate.
    manager: Arc<RwLock<WorldManager>>,
    /// The fetch gate's admitted-worlds view (blockmatrix). Holds its own lock.
    gate: Arc<WorldIsolationGate>,
    /// Trust posture applied when admitting a formed child world.
    network_type: NetworkType,
    /// Sequences the two-set mutations against each other **and** against the
    /// two-set reader. `form`/`merge`/`merge_gap` take the write side (exclusive)
    /// so two boundary mutations never interleave; `check_fetch` takes the read
    /// side (shared) across both of its reads so its `world_of`+gate pair is a
    /// consistent snapshot no merge can straddle. Single-set reads
    /// (`world_of`/`admits`/`world_count`) do not take it.
    seq: RwLock<()>,
}

impl WorldCoordinator {
    /// Mount a coordinator for a node whose home world is `local_world`.
    ///
    /// Mounts the isolation gate and roots the `WorldManager` at the same home
    /// world, so both sets start admitting exactly the home world — identical to
    /// the P5 single-world node.
    pub async fn mount(local_world: NetworkId, network_type: NetworkType) -> Result<Self> {
        let gate = WorldIsolationGate::mount(local_world, network_type.clone()).await?;
        Ok(Self {
            manager: Arc::new(RwLock::new(WorldManager::new(local_world))),
            gate: Arc::new(gate),
            network_type,
            seq: RwLock::new(()),
        })
    }

    /// The node's home world.
    pub fn home_world(&self) -> NetworkId {
        self.gate.local_world()
    }

    /// The gate, for observability (violations/stats) in callers and tests.
    pub fn gate(&self) -> &WorldIsolationGate {
        &self.gate
    }

    /// The TRUE world a shard belongs to, per the `WorldManager`.
    pub async fn world_of(&self, shard: &ContentHash) -> NetworkId {
        self.manager.read().await.world_of(shard)
    }

    /// Whether this node participates in `world` (per the `WorldManager`).
    pub async fn admits(&self, world: NetworkId) -> bool {
        self.manager.read().await.admits(world)
    }

    /// Number of worlds this node participates in.
    pub async fn world_count(&self) -> usize {
        self.manager.read().await.world_count()
    }

    /// Consult the fetch gate for a shard, fed the shard's TRUE world from the
    /// `WorldManager` — the single call the replication path makes.
    ///
    /// Resolves the world and checks the gate under a **shared** hold of the
    /// sequencing lock, so the `world_of` read and the `gate.check_fetch` read
    /// observe the same generation of both sets: no `form`/`merge` (which take
    /// the write side) can interleave between them, closing the cross-set desync
    /// window. On a single-world node nothing ever takes the write side, so this
    /// is exactly the P5 no-op.
    pub async fn check_fetch(&self, shard: &ContentHash) -> Result<()> {
        let _seq = self.seq.read().await;
        let world = self.manager.read().await.world_of(shard);
        self.gate.check_fetch(world, shard).await
    }

    /// **FORM / split (admit-before-migrate).** A congested `parent` above
    /// threshold splits off a child world, migrating the hot chunk down into it.
    ///
    /// Ordering: the child id is decided from a read-only plan, the gate admits
    /// the child FIRST, and only then is the migration committed — so a
    /// concurrent `world_of(shard)` never resolves to a child the gate has not
    /// admitted. Returns `None` in exactly the cases
    /// `WorldManager::form_from_hotspot` would (not a member of `parent`,
    /// congestion below threshold, hot chunk too small).
    pub async fn form(
        &self,
        parent: NetworkId,
        alert: &HotspotAlert,
        hot_shards: &[ContentHash],
    ) -> Result<Option<WorldFormation>> {
        let _seq = self.seq.write().await;

        // Decide without mutating — the child id is known from the plan.
        let plan = {
            let wm = self.manager.read().await;
            wm.plan_formation(parent, alert, hot_shards)
        };
        let Some(plan) = plan else {
            return Ok(None);
        };
        let child = plan.child();

        // (1) Admit the child on the gate BEFORE the shard→child mapping is
        //     observable. If this fails, the plan is dropped uncommitted — the
        //     two sets stay consistent (neither changed).
        self.gate
            .admit_world(child, self.network_type.clone())
            .await?;

        // (2) THEN migrate the shards into the now-admitted child.
        let formation = {
            let mut wm = self.manager.write().await;
            wm.commit_formation(plan)
        };
        Ok(Some(formation))
    }

    /// **MERGE (remap-before-revoke).** Reabsorb a single emergent `child` back
    /// into its parent: the `WorldManager` re-maps the child's shards up to the
    /// parent FIRST, then the gate revokes the child — so a shard never maps to
    /// a revoked world. Returns `None` if `child` is not a mergeable emergent
    /// world (home world or pinned boundary or not a member).
    pub async fn merge(&self, child: NetworkId) -> Result<Option<WorldMerge>> {
        let _seq = self.seq.write().await;

        // (1) Re-map the child's shards back to the parent BEFORE revoking.
        let merge = {
            let mut wm = self.manager.write().await;
            wm.merge_child(child)
        };
        let Some(merge) = merge else {
            return Ok(None);
        };

        // (2) THEN revoke the child on the gate.
        self.gate.revoke_world(child).await;
        Ok(Some(merge))
    }

    /// **MERGE via coverage gap (remap-before-revoke).** The CI-driven merge
    /// path: an under-served region's emergent children are reabsorbed. All
    /// shard re-mappings are committed in the `WorldManager` FIRST (one write),
    /// then each dissolved child is revoked on the gate — so the remap-before-
    /// revoke ordering holds across every child at once.
    pub async fn merge_gap(
        &self,
        parent: NetworkId,
        gap: &CoverageGap,
    ) -> Result<Vec<WorldMerge>> {
        let _seq = self.seq.write().await;

        // (1) Re-map every reabsorbed child's shards up to the parent.
        let merges = {
            let mut wm = self.manager.write().await;
            wm.consume_coverage_gap(parent, gap)
        };

        // (2) THEN revoke each dissolved child on the gate.
        for m in &merges {
            self.gate.revoke_world(m.child).await;
        }
        Ok(merges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::{MatrixPosition, GLOBAL_WORLD};

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

    /// A freshly mounted coordinator is a single-world node: home is admitted,
    /// every shard resolves to it, and the gate is a no-op.
    #[tokio::test]
    async fn mount_is_single_world_noop() {
        let coord = WorldCoordinator::mount(GLOBAL_WORLD, NetworkType::P2P)
            .await
            .expect("test: mount");
        assert_eq!(coord.home_world(), GLOBAL_WORLD);
        assert_eq!(coord.world_count().await, 1);
        assert_eq!(coord.world_of(&shard(1)).await, GLOBAL_WORLD);
        assert!(coord.check_fetch(&shard(1)).await.is_ok());
    }

    /// FORM then MERGE reconciles both sets: after form the migrated shard maps
    /// to the child and the gate accepts it; after merge it maps back to the
    /// parent and the gate rejects the dissolved child.
    #[tokio::test]
    async fn form_then_merge_keeps_both_sets_consistent() {
        let coord = WorldCoordinator::mount(GLOBAL_WORLD, NetworkType::P2P)
            .await
            .expect("test: mount");
        let hot = vec![shard(1), shard(2)];

        let formation = coord
            .form(GLOBAL_WORLD, &hotspot(0.95), &hot)
            .await
            .expect("test: form ok")
            .expect("test: congested hotspot forms a child");
        let child = formation.child;

        // Both sets agree: shard maps to child AND the gate admits it.
        assert_eq!(coord.world_of(&shard(1)).await, child);
        assert!(coord.check_fetch(&shard(1)).await.is_ok());
        // An unassigned shard stays in the parent, still accepted.
        assert_eq!(coord.world_of(&shard(9)).await, GLOBAL_WORLD);
        assert!(coord.check_fetch(&shard(9)).await.is_ok());

        let merge = coord
            .merge(child)
            .await
            .expect("test: merge ok")
            .expect("test: child is mergeable");
        assert_eq!(merge.child, child);

        // Shard is back in the parent and still accepted; the dissolved child is
        // now foreign to the gate.
        assert_eq!(coord.world_of(&shard(1)).await, GLOBAL_WORLD);
        assert!(coord.check_fetch(&shard(1)).await.is_ok());
        assert!(coord.gate().check_fetch(child, &shard(1)).await.is_err());
    }

    /// Below-threshold congestion forms nothing and leaves both sets untouched.
    #[tokio::test]
    async fn low_congestion_forms_nothing() {
        let coord = WorldCoordinator::mount(GLOBAL_WORLD, NetworkType::P2P)
            .await
            .expect("test: mount");
        assert!(coord
            .form(GLOBAL_WORLD, &hotspot(0.5), &[shard(1)])
            .await
            .expect("test: form ok")
            .is_none());
        assert_eq!(coord.world_count().await, 1);
        // Gate admitted nothing new either.
        assert!(!coord.gate().admits(hotspot_child()).await);
    }

    // A child id derivable from the below-threshold hotspot, to assert the gate
    // did NOT admit it when no formation occurred.
    fn hotspot_child() -> NetworkId {
        let mut tag = Vec::with_capacity(24);
        tag.extend_from_slice(&1.0f64.to_le_bytes());
        tag.extend_from_slice(&2.0f64.to_le_bytes());
        tag.extend_from_slice(&3.0f64.to_le_bytes());
        WorldManager::derive_child_world(GLOBAL_WORLD, &tag)
    }
}
