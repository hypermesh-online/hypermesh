// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Worlds — emergent partitioning of the matrix (VISION.md §5.5).
//!
//! A **world** is a sub-mesh: a nestable network occupying an elastic slice of
//! the matrix. Worlds are *emergent, not drawn* — they form where load and node
//! distribution concentrate, and dissolve where they thin out. This is the
//! first live consumer of [`CollectiveIntelligence`](crate::collective_intel):
//! a [`HotspotAlert`] over a congested region forms/splits a child world; a
//! [`CoverageGap`] over an under-utilised region merges one back.
//!
//! Two kinds of world share ONE primitive (the nestable network):
//! - **pinned** — an operator *declared* it (a private network stood up by
//!   hand). The manager NEVER reabsorbs or reparents a pinned boundary; it only
//!   partitions *within* it.
//! - **emergent** — the manager *formed* it from load. These are the ones that
//!   split and merge as activation shifts.
//!
//! Nesting is what lets the two coexist: an emergent inner world carries a
//! `parent` pointer to its (possibly pinned) outer world, resolved by walking
//! the parent chain — the same model DNS domains use
//! (`blockmatrix/src/dns/domain.rs`: BLAKE3-derived id + `Option<parent>`
//! pointer + parent-chain walk), reused here over [`NetworkId`] rather than
//! reinvented.
//!
//! ## Safety invariant (load-bearing)
//! Forming a world must never strand its legitimate holders. A world only ever
//! *nests downward*: [`form_from_hotspot`](WorldManager::form_from_hotspot)
//! migrates only shards the node holds into a child the node simultaneously
//! *joins* ([`member_worlds`](WorldManager::member_worlds) gains the child). So
//! a shard's true world is always a world its holder is a member of — the gate
//! keeps accepting it. A shard never assigned to any child stays in its parent
//! world, untouched.

use std::collections::{HashMap, HashSet};

use hypermesh_lib::{ContentHash, NetworkId, GLOBAL_WORLD};
use tracing::{info, warn};

use crate::collective_intel::{CoverageGap, HotspotAlert, NetworkInsight};

/// Thresholds governing when the matrix repartitions. These are **placement /
/// load signals**, not authorization: a congestion ratio is a scheduler input
/// about where demand concentrates, never a PoS magnitude and never an
/// admission gate (VISION.md §6 — PoS is identity, never magnitude).
#[derive(Debug, Clone)]
pub struct WorldFormationConfig {
    /// Congestion ratio (0.0–1.0) a hotspot must exceed before a child world
    /// splits off. A load signal, not a stake.
    pub congestion_threshold: f64,
    /// A hotspot must carry at least this many hot shards to justify a split
    /// (a one-shard "hotspot" is noise, not a world).
    pub min_hot_shards: usize,
    /// Node count at or below which an under-served region's emergent children
    /// are reabsorbed into their parent.
    pub merge_utilization_floor: usize,
}

impl Default for WorldFormationConfig {
    fn default() -> Self {
        Self {
            congestion_threshold: 0.8,
            min_hot_shards: 1,
            merge_utilization_floor: 1,
        }
    }
}

/// Record of a world that split off from its parent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldFormation {
    /// The freshly-formed child world.
    pub child: NetworkId,
    /// The world it split off from.
    pub parent: NetworkId,
    /// Shards migrated down into the child.
    pub migrated: Vec<ContentHash>,
}

/// Record of a child world reabsorbed into its parent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMerge {
    /// The child world that was dissolved.
    pub child: NetworkId,
    /// The parent it was reabsorbed into.
    pub parent: NetworkId,
    /// Shards handed back up to the parent world.
    pub reabsorbed: Vec<ContentHash>,
}

/// Owns the node's view of world boundaries: which worlds it belongs to, which
/// world each shard lives in, the nesting graph, and the pinned boundaries it
/// must never auto-manage.
pub struct WorldManager {
    /// The root world this node always belongs to (typically [`GLOBAL_WORLD`]).
    home_world: NetworkId,
    /// Every world this node participates in (always contains `home_world`).
    member_worlds: HashSet<NetworkId>,
    /// A shard's assigned world. Absent ⇒ the shard lives in `home_world` and
    /// has not been claimed by any child (VISION.md §5.5: unassigned shards stay
    /// in the parent, untouched by formation).
    shard_worlds: HashMap<ContentHash, NetworkId>,
    /// child → parent nesting pointer (the DNS-domain model over `NetworkId`).
    parents: HashMap<NetworkId, NetworkId>,
    /// Operator-declared boundaries. Never merged, never reparented.
    pinned: HashSet<NetworkId>,
    /// Formation / merge thresholds.
    config: WorldFormationConfig,
}

impl WorldManager {
    /// Create a manager rooted at `home_world`.
    pub fn new(home_world: NetworkId) -> Self {
        Self::with_config(home_world, WorldFormationConfig::default())
    }

    /// Create a manager rooted at [`GLOBAL_WORLD`] (the single implicit world a
    /// node starts in before any partitioning).
    pub fn global() -> Self {
        Self::new(GLOBAL_WORLD)
    }

    /// Create a manager with explicit thresholds.
    pub fn with_config(home_world: NetworkId, config: WorldFormationConfig) -> Self {
        let mut member_worlds = HashSet::new();
        member_worlds.insert(home_world);
        Self {
            home_world,
            member_worlds,
            shard_worlds: HashMap::new(),
            parents: HashMap::new(),
            pinned: HashSet::new(),
            config,
        }
    }

    /// The node's root world.
    pub fn home_world(&self) -> NetworkId {
        self.home_world
    }

    /// Every world this node participates in (deterministically ordered).
    pub fn member_worlds(&self) -> Vec<NetworkId> {
        let mut worlds: Vec<NetworkId> = self.member_worlds.iter().copied().collect();
        worlds.sort_by(|a, b| a.0.cmp(&b.0));
        worlds
    }

    /// Whether this node participates in `world`.
    pub fn admits(&self, world: NetworkId) -> bool {
        self.member_worlds.contains(&world)
    }

    /// The TRUE world a shard belongs to — the source Part C feeds the isolation
    /// gate. Unassigned shards live in the home world.
    pub fn world_of(&self, shard: &ContentHash) -> NetworkId {
        self.shard_worlds
            .get(shard)
            .copied()
            .unwrap_or(self.home_world)
    }

    /// The parent of `world`, if it is a nested child.
    pub fn parent_of(&self, world: NetworkId) -> Option<NetworkId> {
        self.parents.get(&world).copied()
    }

    /// Walk the nesting chain from `world` up to its root, inclusive — the DNS
    /// parent-chain walk over `NetworkId`. The first element is `world`; the
    /// last is a root (home or a pinned outer boundary with no parent).
    pub fn parent_chain(&self, world: NetworkId) -> Vec<NetworkId> {
        let mut chain = vec![world];
        let mut cursor = world;
        // Bounded by the number of registered parents; guards against a cycle.
        while let Some(parent) = self.parents.get(&cursor).copied() {
            if chain.contains(&parent) {
                warn!("world nesting cycle detected at {parent}; stopping walk");
                break;
            }
            chain.push(parent);
            cursor = parent;
        }
        chain
    }

    /// Declare a **pinned** (operator-intentional) world — a private network the
    /// operator stood up. It joins the node's membership and is protected from
    /// auto-merge and reparenting. `parent` nests it under an outer world (its
    /// own boundary is still never split above).
    pub fn pin_world(&mut self, world: NetworkId, parent: Option<NetworkId>) {
        self.pinned.insert(world);
        self.member_worlds.insert(world);
        if let Some(p) = parent {
            self.parents.insert(world, p);
        }
        info!(
            "world: pinned operator-declared boundary {} (parent {:?})",
            world, parent
        );
    }

    /// Whether `world` is a pinned operator boundary.
    pub fn is_pinned(&self, world: NetworkId) -> bool {
        self.pinned.contains(&world)
    }

    /// Derive a deterministic child `NetworkId` from a parent and a
    /// distinguishing tag (the hotspot's coordinates). Reuses the DNS derivation
    /// model — BLAKE3 over the parent identity plus a discriminator — so a child
    /// world's id is reproducible and content-bound, not random.
    pub fn derive_child_world(parent: NetworkId, tag: &[u8]) -> NetworkId {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"hypermesh-world-child");
        hasher.update(&parent.0);
        hasher.update(tag);
        let digest = hasher.finalize();
        let mut id = [0u8; 16];
        id.copy_from_slice(&digest.as_bytes()[..16]);
        // GLOBAL_WORLD is all-zero; a derived child must never collide with it.
        if id == [0u8; 16] {
            id[0] = 1;
        }
        NetworkId(id)
    }

    /// Emergent **form / split**: a congested region above threshold splits off
    /// a child world under `parent`, migrating the hot chunk's shards down into
    /// it. The node holds those shards, so it *joins* the child in the same step
    /// — a shard's world is always a world its holder belongs to.
    ///
    /// Returns `None` (no split) when: the node is not a member of `parent`
    /// (can't partition a world it isn't in), congestion is below threshold, or
    /// the hot chunk is too small to be a world. A pinned `parent` is fully
    /// valid — nesting *inside* it never touches its own boundary.
    pub fn form_from_hotspot(
        &mut self,
        parent: NetworkId,
        alert: &HotspotAlert,
        hot_shards: &[ContentHash],
    ) -> Option<WorldFormation> {
        if !self.member_worlds.contains(&parent) {
            return None;
        }
        if alert.congestion_ratio < self.config.congestion_threshold {
            return None;
        }
        if hot_shards.len() < self.config.min_hot_shards {
            return None;
        }

        // Tag the child by the hotspot's location so distinct hotspots under the
        // same parent derive distinct children.
        let mut tag = Vec::with_capacity(24);
        tag.extend_from_slice(&alert.center.x.to_le_bytes());
        tag.extend_from_slice(&alert.center.y.to_le_bytes());
        tag.extend_from_slice(&alert.center.z.to_le_bytes());
        let child = Self::derive_child_world(parent, &tag);
        if child == parent {
            return None;
        }

        self.parents.insert(child, parent);
        self.member_worlds.insert(child);

        let mut migrated = Vec::with_capacity(hot_shards.len());
        for shard in hot_shards {
            self.shard_worlds.insert(*shard, child);
            migrated.push(*shard);
        }

        info!(
            "world: formed child {} under {} from hotspot (congestion {:.2}, {} shards migrated)",
            child,
            parent,
            alert.congestion_ratio,
            migrated.len(),
        );
        Some(WorldFormation {
            child,
            parent,
            migrated,
        })
    }

    /// Emergent **merge**: reabsorb a child world back into its parent, handing
    /// its shards back up. Refuses to touch the home world or a **pinned**
    /// boundary — that is the concrete "never split/collapse an operator
    /// boundary" rule. Returns `None` if `child` is not a mergeable emergent
    /// world.
    pub fn merge_child(&mut self, child: NetworkId) -> Option<WorldMerge> {
        if child == self.home_world {
            return None;
        }
        if self.pinned.contains(&child) {
            warn!("world: refusing to merge pinned operator boundary {child}");
            return None;
        }
        if !self.member_worlds.contains(&child) {
            return None;
        }

        let parent = self.parents.remove(&child).unwrap_or(self.home_world);
        let mut reabsorbed = Vec::new();
        for (shard, world) in self.shard_worlds.iter_mut() {
            if *world == child {
                *world = parent;
                reabsorbed.push(*shard);
            }
        }
        self.member_worlds.remove(&child);

        info!(
            "world: merged child {} back into {} ({} shards reabsorbed)",
            child,
            parent,
            reabsorbed.len(),
        );
        Some(WorldMerge {
            child,
            parent,
            reabsorbed,
        })
    }

    /// Consume a [`HotspotAlert`] under `parent` — the CI-driven form path.
    pub fn consume_hotspot(
        &mut self,
        parent: NetworkId,
        alert: &HotspotAlert,
        hot_shards: &[ContentHash],
    ) -> Option<WorldFormation> {
        self.form_from_hotspot(parent, alert, hot_shards)
    }

    /// Consume a [`CoverageGap`] under `parent` — the CI-driven merge path. When
    /// the region has thinned to the utilisation floor, its emergent (non-pinned)
    /// children are reabsorbed. Pinned children are left intact.
    pub fn consume_coverage_gap(
        &mut self,
        parent: NetworkId,
        gap: &CoverageGap,
    ) -> Vec<WorldMerge> {
        if gap.node_count > self.config.merge_utilization_floor {
            return Vec::new();
        }
        let children: Vec<NetworkId> = self
            .parents
            .iter()
            .filter(|(child, p)| **p == parent && !self.pinned.contains(*child))
            .map(|(child, _)| *child)
            .collect();

        let mut merges = Vec::new();
        for child in children {
            if let Some(merge) = self.merge_child(child) {
                merges.push(merge);
            }
        }
        merges
    }

    /// Route a [`NetworkInsight`] to the right handler — the single entry point
    /// making [`CollectiveIntelligence`](crate::collective_intel) a live
    /// consumer. `hot_shards` names the shards the emitting node holds in the
    /// congested region (empty for a gap); `parent` is the world being
    /// partitioned within.
    pub fn apply_insight(
        &mut self,
        parent: NetworkId,
        insight: &NetworkInsight,
        hot_shards: &[ContentHash],
    ) -> WorldAction {
        match insight {
            NetworkInsight::HotspotAlert(alert) => self
                .consume_hotspot(parent, alert, hot_shards)
                .map(WorldAction::Formed)
                .unwrap_or(WorldAction::None),
            NetworkInsight::CoverageGap(gap) => {
                let merges = self.consume_coverage_gap(parent, gap);
                if merges.is_empty() {
                    WorldAction::None
                } else {
                    WorldAction::Merged(merges)
                }
            }
            // Capacity/economic insights do not move boundaries.
            _ => WorldAction::None,
        }
    }

    /// Number of worlds this node participates in.
    pub fn world_count(&self) -> usize {
        self.member_worlds.len()
    }
}

/// The boundary change (if any) an insight produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldAction {
    /// A child world split off.
    Formed(WorldFormation),
    /// One or more child worlds were reabsorbed.
    Merged(Vec<WorldMerge>),
    /// The insight did not move any boundary.
    None,
}

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::MatrixPosition;

    fn shard(seed: u8) -> ContentHash {
        ContentHash([seed; 32])
    }

    fn hotspot(congestion: f64) -> HotspotAlert {
        HotspotAlert {
            center: MatrixPosition {
                x: 5.0,
                y: 6.0,
                z: 7.0,
            },
            congestion_ratio: congestion,
            affected_nodes: 4,
            severity: "high".to_string(),
        }
    }

    fn gap(node_count: usize) -> CoverageGap {
        CoverageGap {
            center: MatrixPosition {
                x: 5.0,
                y: 6.0,
                z: 7.0,
            },
            radius: 10.0,
            node_count,
        }
    }

    // ── Behaviour 1: form / split from a hotspot ────────────────────────────
    #[test]
    fn hotspot_forms_child_world_with_parent_set() {
        let mut wm = WorldManager::global();
        let hot = vec![shard(1), shard(2)];

        let formation = wm
            .form_from_hotspot(GLOBAL_WORLD, &hotspot(0.95), &hot)
            .expect("test: congested hotspot must form a child");

        assert_eq!(formation.parent, GLOBAL_WORLD);
        assert_ne!(formation.child, GLOBAL_WORLD);
        assert_eq!(formation.migrated, hot);
        // Nesting: child points at its parent.
        assert_eq!(wm.parent_of(formation.child), Some(GLOBAL_WORLD));
        // Node joined the child, so its own shards stay same-world.
        assert!(wm.admits(formation.child));
        // Shard→world now resolves to the child, not the constant.
        assert_eq!(wm.world_of(&shard(1)), formation.child);
        // An unassigned shard stays in the parent, untouched.
        assert_eq!(wm.world_of(&shard(9)), GLOBAL_WORLD);
    }

    #[test]
    fn low_congestion_does_not_form_a_world() {
        let mut wm = WorldManager::global();
        assert!(wm
            .form_from_hotspot(GLOBAL_WORLD, &hotspot(0.5), &[shard(1)])
            .is_none());
        assert_eq!(wm.world_count(), 1);
    }

    #[test]
    fn cannot_form_within_a_world_the_node_is_not_in() {
        let mut wm = WorldManager::global();
        let foreign = NetworkId([0x77; 16]);
        assert!(wm
            .form_from_hotspot(foreign, &hotspot(0.99), &[shard(1)])
            .is_none());
    }

    // ── Behaviour 2: merge reabsorbs an under-served child ──────────────────
    #[test]
    fn coverage_gap_reabsorbs_child() {
        let mut wm = WorldManager::global();
        let formation = wm
            .form_from_hotspot(GLOBAL_WORLD, &hotspot(0.9), &[shard(1), shard(2)])
            .expect("test: form child");
        let child = formation.child;
        assert_eq!(wm.world_count(), 2);

        let merges = wm.consume_coverage_gap(GLOBAL_WORLD, &gap(0));
        assert_eq!(merges.len(), 1);
        assert_eq!(merges[0].child, child);
        assert_eq!(merges[0].parent, GLOBAL_WORLD);
        // Shards handed back up; node no longer a member of the dissolved world.
        assert_eq!(wm.world_of(&shard(1)), GLOBAL_WORLD);
        assert!(!wm.admits(child));
        assert_eq!(wm.world_count(), 1);
    }

    #[test]
    fn coverage_gap_with_healthy_utilization_does_not_merge() {
        let mut wm = WorldManager::global();
        wm.form_from_hotspot(GLOBAL_WORLD, &hotspot(0.9), &[shard(1)])
            .expect("test: form child");
        // node_count above the floor → no reabsorption.
        assert!(wm.consume_coverage_gap(GLOBAL_WORLD, &gap(5)).is_empty());
        assert_eq!(wm.world_count(), 2);
    }

    // ── Behaviour 3: a pinned boundary is never split above / collapsed ──────
    #[test]
    fn pinned_boundary_is_never_merged() {
        let mut wm = WorldManager::global();
        let op_world = NetworkId([0x0A; 16]);
        wm.pin_world(op_world, Some(GLOBAL_WORLD));

        // Direct merge attempt refused.
        assert!(wm.merge_child(op_world).is_none());
        // A coverage gap under GLOBAL must not reabsorb the pinned operator world.
        let merges = wm.consume_coverage_gap(GLOBAL_WORLD, &gap(0));
        assert!(merges.iter().all(|m| m.child != op_world));
        assert!(wm.admits(op_world), "pinned world remains a member");
        assert!(wm.is_pinned(op_world));
    }

    #[test]
    fn emergent_inner_splits_within_pinned_outer_not_above() {
        let mut wm = WorldManager::global();
        let op_world = NetworkId([0x0A; 16]);
        wm.pin_world(op_world, Some(GLOBAL_WORLD));

        // Hotspot INSIDE the pinned outer world → an emergent child nests under
        // it. The pinned boundary itself is untouched.
        let formation = wm
            .form_from_hotspot(op_world, &hotspot(0.9), &[shard(3)])
            .expect("test: form inner world within pinned outer");
        assert_eq!(formation.parent, op_world);
        assert!(wm.is_pinned(op_world));
        assert!(!wm.is_pinned(formation.child));
    }

    // ── Behaviour 4: nesting resolves emergent-inner via the pinned-outer chain
    #[test]
    fn nested_chain_resolves_emergent_inner_through_pinned_outer() {
        let mut wm = WorldManager::global();
        let op_world = NetworkId([0x0A; 16]);
        wm.pin_world(op_world, Some(GLOBAL_WORLD));
        let formation = wm
            .form_from_hotspot(op_world, &hotspot(0.9), &[shard(3)])
            .expect("test: inner world");

        // inner → pinned-outer → GLOBAL root.
        let chain = wm.parent_chain(formation.child);
        assert_eq!(chain, vec![formation.child, op_world, GLOBAL_WORLD]);
    }

    #[test]
    fn apply_insight_routes_hotspot_and_gap() {
        let mut wm = WorldManager::global();
        let formed = wm.apply_insight(
            GLOBAL_WORLD,
            &NetworkInsight::HotspotAlert(hotspot(0.9)),
            &[shard(1)],
        );
        let child = match formed {
            WorldAction::Formed(f) => f.child,
            other => unreachable!("test: expected Formed, got {other:?}"),
        };
        assert!(wm.admits(child));

        let merged = wm.apply_insight(
            GLOBAL_WORLD,
            &NetworkInsight::CoverageGap(gap(0)),
            &[],
        );
        match merged {
            WorldAction::Merged(m) => assert_eq!(m[0].child, child),
            other => unreachable!("test: expected Merged, got {other:?}"),
        }
    }

    #[test]
    fn derived_child_is_deterministic_and_never_global() {
        let a = WorldManager::derive_child_world(GLOBAL_WORLD, b"tag");
        let b = WorldManager::derive_child_world(GLOBAL_WORLD, b"tag");
        assert_eq!(a, b, "derivation is deterministic");
        assert_ne!(a, GLOBAL_WORLD, "a child never collides with GLOBAL_WORLD");
        let c = WorldManager::derive_child_world(GLOBAL_WORLD, b"other");
        assert_ne!(a, c, "distinct tags derive distinct children");
    }
}
