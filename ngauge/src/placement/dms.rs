// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! DMS decision seam — NGauge is the DMS *brain*.
//!
//! For a network, this folds the three dormant/existing placement signals into
//! a concrete [`DmsPlan`]:
//!
//! - [`ReplicationTrigger::check_in_network`] — *which* shards need more copies
//!   and *how urgent*,
//! - [`DispersionAdvisor::recommend_placement_in_network`] — *where* the swarm
//!   wants new replicas (k-means over consumer demand), and
//! - [`replica_selection`] ([`order_by_proximity`] / [`ReplicaCandidate`] /
//!   [`ReplicaSelector`]) — *which source* to pull from and *how hard to fall
//!   back*. This is the layer that was salvaged-but-dormant; the planner turns
//!   it LIVE.
//!
//! [`DmsDriver::plan`] is a **pure, synchronous** function: the caller holds the
//! `std::sync::Mutex<SwarmAnalytics>` guard, builds the plan, then DROPS the
//! guard BEFORE any `.await`. The guard is not `Send`, so the plan MUST be
//! fully materialised before I/O. blockmatrix owns the I/O behind the
//! [`MirrorExecutor`] / [`ReflectExecutor`] traits (defined here, lib types
//! only — ngauge never gains a blockmatrix dependency).

use std::collections::HashMap;

use async_trait::async_trait;
use hypermesh_lib::{ContentHash, MatrixPosition, NetworkId, NodeId};

use crate::swarm_analytics::{
    DispersionAdvisor, ReplicationConfig, ReplicationTrigger, SwarmAnalytics,
};

use super::replica_selection::{
    order_by_proximity, FallbackStrategy, ReplicaCandidate, ReplicaSelector, SelectionCriteria,
};

/// A DMS plan: what to **mirror** (pull extra replicas of a hot shard) and what
/// to **reflect** (announce/serve the shards this node already holds). Pure
/// data — no locks, no I/O.
#[derive(Debug, Clone, Default)]
pub struct DmsPlan {
    /// Shards to mirror (fetch an extra replica of) and from where.
    pub mirror: Vec<MirrorAction>,
    /// Shards to reflect (re-announce so consumers can find them).
    pub reflect: Vec<ReflectAction>,
}

/// One mirror decision: pull a replica of `shard_id` in `network` from `source`.
#[derive(Debug, Clone)]
pub struct MirrorAction {
    /// Network the shard is replicated within.
    pub network: NetworkId,
    /// Content hash of the shard to fetch.
    pub shard_id: ContentHash,
    /// The provider selected to fetch from.
    pub source: NodeId,
    /// Replication urgency carried from the trigger (higher = more urgent).
    pub urgency: f32,
    /// Advisory fallback strategy for handling a missed/failed fetch.
    pub fallback: FallbackStrategy,
}

/// One reflect decision: ensure the shards held for this asset are announced.
#[derive(Debug, Clone)]
pub struct ReflectAction {
    /// Network the shard is served within.
    pub network: NetworkId,
    /// Content hash of the shard to (re-)announce.
    pub shard_id: ContentHash,
}

/// Errors surfaced by the DMS executors back to the driver.
#[derive(Debug, thiserror::Error)]
pub enum DmsError {
    /// A mirror fetch (or the follow-on registration) failed.
    #[error("dms mirror fetch failed: {0}")]
    Fetch(String),
    /// A reflect announce failed.
    #[error("dms reflect announce failed: {0}")]
    Announce(String),
}

/// Executes a [`MirrorAction`]: fetch the shard, register this node as a new
/// provider, and report the resulting replica count. Implemented in blockmatrix
/// (the I/O owner); the trait lives here so ngauge owns the decision contract.
#[async_trait]
pub trait MirrorExecutor {
    /// Fetch + register a replica; returns the new replica count on success.
    async fn fetch_and_register(&self, action: &MirrorAction) -> Result<u32, DmsError>;
}

/// Executes a [`ReflectAction`]: announce a held shard so consumers can find it.
#[async_trait]
pub trait ReflectExecutor {
    /// Announce the shard to the network.
    async fn announce(&self, action: &ReflectAction) -> Result<(), DmsError>;
}

/// The per-shard provider candidates the caller gathered from its location
/// index (blockmatrix-owned, async). Passed into [`DmsDriver::plan`] because
/// provider *identity* is not part of `SwarmAnalytics` — the planner decides
/// *which* of these to pull from.
#[derive(Debug, Clone)]
pub struct ShardCandidates {
    /// The shard these candidates provide.
    pub shard_id: ContentHash,
    /// Candidate providers that carry a live matrix coordinate (participate in
    /// dispersion + centrality selection).
    pub positioned: Vec<ReplicaCandidate>,
    /// Every candidate provider id (positioned or not), for the deterministic
    /// last-resort pick.
    pub all_ids: Vec<String>,
}

/// The DMS brain: folds trigger + dispersion + selection into a [`DmsPlan`].
pub struct DmsDriver;

impl DmsDriver {
    /// Build the DMS plan for `network` from live `analytics` and the per-shard
    /// provider `candidates` the caller gathered.
    ///
    /// PURE + synchronous — no `.await`, no lock acquisition. The caller holds
    /// the `SwarmAnalytics` guard for the duration of this call and DROPS it
    /// before executing the plan (the guard is `!Send`).
    ///
    /// Only shards whose replication urgency strictly exceeds `urgency_floor`
    /// (and that have at least one candidate provider) become mirror actions —
    /// mirroring the old `poll.rs` `urgency > 0.5` filter 1:1.
    pub fn plan(
        analytics: &SwarmAnalytics,
        network: NetworkId,
        candidates: &[ShardCandidates],
        urgency_floor: f32,
    ) -> DmsPlan {
        // (1) Which shards need copies + how urgent — ngauge decides.
        let signals = ReplicationTrigger::new(ReplicationConfig::default())
            .check_in_network(analytics, network);
        let urgency_by_shard: HashMap<ContentHash, f64> =
            signals.iter().map(|s| (s.shard_id, s.urgency)).collect();

        let advisor = DispersionAdvisor::new();
        let mut mirror = Vec::new();

        for bundle in candidates {
            // Gate on the trigger's urgency for this shard (old > 0.5 filter).
            let urgency = match urgency_by_shard.get(&bundle.shard_id) {
                Some(u) if *u > urgency_floor as f64 => *u,
                _ => continue,
            };
            if bundle.all_ids.is_empty() {
                continue;
            }

            // (2) Where does the swarm want new replicas? (k-means over demand.)
            let recommendations = advisor.recommend_placement_in_network(
                network,
                &bundle.shard_id,
                analytics,
                bundle.all_ids.len().max(1),
            );

            // (3) Which source to pull from + how hard to fall back.
            let source_id = select_source(&bundle.positioned, &bundle.all_ids, &recommendations);
            mirror.push(MirrorAction {
                network,
                shard_id: bundle.shard_id,
                source: NodeId::from_public_key(source_id.as_bytes()),
                urgency: urgency as f32,
                fallback: recommend_fallback(&bundle.positioned),
            });
        }

        // Reflect is driven by the head-observer (Phase 4); the replication
        // poll loop emits none, so behaviour is preserved exactly.
        DmsPlan {
            mirror,
            reflect: Vec::new(),
        }
    }
}

/// Port of the old `poll.rs::select_dispersion_source` (the W1 duplicate), now
/// expressed on the live `replica_selection` primitives.
///
/// 1. If the dispersion advisor recommended placements, pick the positioned
///    candidate nearest to any recommendation (pull toward under-served demand).
/// 2. Otherwise pick the geometrically central positioned candidate
///    ([`order_by_proximity`] to the candidate centroid).
/// 3. Otherwise (no coordinates at all) the deterministic smallest node id.
fn select_source(
    positioned: &[ReplicaCandidate],
    all_ids: &[String],
    recommendations: &[MatrixPosition],
) -> String {
    // (1) Dispersion-aware: nearest positioned candidate to a recommendation.
    if !recommendations.is_empty() {
        let mut best: Option<(&str, f64)> = None;
        for cand in positioned {
            let nearest = recommendations
                .iter()
                .map(|r| cand.distance_to(r))
                .fold(f64::INFINITY, f64::min);
            match best {
                Some((_, d)) if d <= nearest => {}
                _ => best = Some((cand.id.as_str(), nearest)),
            }
        }
        if let Some((id, _)) = best {
            return id.to_string();
        }
    }

    // (2) Geometric centrality of the positioned candidates.
    if !positioned.is_empty() {
        let centroid = centroid_of(positioned);
        if let Some(&idx) = order_by_proximity(positioned, &centroid).first() {
            return positioned[idx].id.clone();
        }
    }

    // (3) Deterministic last resort: smallest node id.
    all_ids.iter().min().cloned().unwrap_or_default()
}

/// Advisory fallback strategy for a mirror action. The planner has no observed
/// fetch history yet, so an [`FallbackStrategy::Adaptive`] selector recommends
/// [`FallbackStrategy::Sequential`] — i.e. try the chosen source, which is
/// exactly what the executor does (behaviour-preserving; the field is advisory).
fn recommend_fallback(_positioned: &[ReplicaCandidate]) -> FallbackStrategy {
    ReplicaSelector::new(SelectionCriteria::default(), FallbackStrategy::Adaptive)
        .recommend_strategy()
}

/// Mean position of the positioned candidates.
fn centroid_of(candidates: &[ReplicaCandidate]) -> MatrixPosition {
    let n = candidates.len().max(1) as f64;
    let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
    for c in candidates {
        x += c.position.x;
        y += c.position.y;
        z += c.position.z;
    }
    MatrixPosition {
        x: x / n,
        y: y / n,
        z: z / n,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::DEFAULT_NETWORK;

    fn pos(x: f64, y: f64) -> MatrixPosition {
        MatrixPosition { x, y, z: 0.0 }
    }

    fn cand(id: &str, x: f64, y: f64) -> ReplicaCandidate {
        ReplicaCandidate::new(id, pos(x, y), 1.0)
    }

    #[test]
    fn dispersion_recommendation_picks_nearest_candidate() {
        // Two candidates; a recommendation sits on top of "b" -> pick "b".
        let positioned = vec![cand("a", 0.0, 0.0), cand("b", 10.0, 0.0)];
        let all_ids = vec!["a".to_string(), "b".to_string()];
        let recs = vec![pos(9.0, 0.0)];
        assert_eq!(select_source(&positioned, &all_ids, &recs), "b");
    }

    #[test]
    fn no_recommendation_picks_central_candidate() {
        // No recommendations -> geometric centrality. "mid" is the centroid.
        let positioned = vec![
            cand("lo", -10.0, 0.0),
            cand("mid", 0.0, 0.0),
            cand("hi", 10.0, 0.0),
        ];
        let all_ids = vec!["lo".to_string(), "mid".to_string(), "hi".to_string()];
        assert_eq!(select_source(&positioned, &all_ids, &[]), "mid");
    }

    #[test]
    fn no_coordinates_falls_back_to_smallest_id() {
        // No positioned candidates at all -> deterministic smallest id.
        let all_ids = vec!["zeta".to_string(), "alpha".to_string(), "mu".to_string()];
        assert_eq!(select_source(&[], &all_ids, &[]), "alpha");
    }

    #[test]
    fn plan_emits_no_action_below_urgency_floor() {
        // A fresh analytics with no demand -> no signals -> no mirror actions,
        // even though candidates are supplied.
        let analytics = SwarmAnalytics::new();
        let candidates = vec![ShardCandidates {
            shard_id: ContentHash([7u8; 32]),
            positioned: vec![cand("a", 0.0, 0.0)],
            all_ids: vec!["a".to_string()],
        }];
        let plan = DmsDriver::plan(&analytics, DEFAULT_NETWORK, &candidates, 0.5);
        assert!(plan.mirror.is_empty());
        assert!(plan.reflect.is_empty());
    }

    #[test]
    fn plan_mirrors_an_urgent_undersupplied_shard() {
        // Record heavy demand for a shard with zero replicas -> the trigger
        // fires with urgency 1.0 (below min_replicas) -> one mirror action to
        // the sole candidate provider.
        let shard = ContentHash([9u8; 32]);
        let mut analytics = SwarmAnalytics::new();
        for i in 0..500u64 {
            analytics.record_request_in_network(
                DEFAULT_NETWORK,
                shard,
                NodeId([i as u8; 32]),
                pos(0.0, 0.0),
                i,
            );
        }
        analytics.set_replica_count_in_network(DEFAULT_NETWORK, shard, 0);

        let candidates = vec![ShardCandidates {
            shard_id: shard,
            positioned: vec![cand("provider-1", 1.0, 1.0)],
            all_ids: vec!["provider-1".to_string()],
        }];
        let plan = DmsDriver::plan(&analytics, DEFAULT_NETWORK, &candidates, 0.5);
        assert_eq!(plan.mirror.len(), 1);
        let action = &plan.mirror[0];
        assert_eq!(action.shard_id, shard);
        assert_eq!(action.network, DEFAULT_NETWORK);
        assert!(action.urgency > 0.5);
        assert_eq!(
            action.source,
            NodeId::from_public_key("provider-1".as_bytes())
        );
    }
}
