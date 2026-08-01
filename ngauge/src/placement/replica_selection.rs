// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Replica selection — advisory intelligence for choosing WHICH replica of a
//! shard to fetch from, and in what order.
//!
//! This is a pure *advisory* layer: it ranks candidate replica locations by a
//! proximity/health/priority score and tracks per-replica success/failure to
//! recommend a fetch strategy. It carries NO authorization, PoS, or content
//! validation — the caller (blockmatrix) owns those. NGauge only answers "given
//! these candidates and what I've observed, which should you try, in what
//! order, and how hard should you fall back?"
//!
//! Salvaged from the removed instruction-based-retrieval island
//! (`RetrievalPlan::optimize_for_position` + `fallback::{SelectionCriteria,
//! ReplicaSelector}`) and re-homed here as advisory placement intelligence.
//! Candidates are keyed by a network `id` (the string node id) and positioned
//! in the shared [`MatrixPosition`] space, so the layer stays free of any
//! blockmatrix type.

use std::collections::HashSet;

use hypermesh_lib::MatrixPosition;

/// Fallback strategy for handling missing / failed shard fetches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackStrategy {
    /// Try candidate replicas sequentially.
    Sequential,
    /// Try candidate replicas in parallel.
    Parallel,
    /// Reconstruct from other shards via Reed-Solomon.
    ReedSolomon,
    /// Adapt to the observed failure rate.
    Adaptive,
}

impl FallbackStrategy {
    /// Whether this strategy reconstructs via Reed-Solomon.
    pub fn needs_reed_solomon(&self) -> bool {
        matches!(self, FallbackStrategy::ReedSolomon)
    }

    /// Whether this strategy fetches candidates in parallel.
    pub fn uses_parallel(&self) -> bool {
        matches!(self, FallbackStrategy::Parallel | FallbackStrategy::Adaptive)
    }
}

/// A candidate replica location for a shard: a network `id`, its placement
/// [`MatrixPosition`], and the health/latency signals used to rank it.
#[derive(Debug, Clone)]
pub struct ReplicaCandidate {
    /// Network id of the node holding this replica (the string node id).
    pub id: String,
    /// Placement coordinate of the replica.
    pub position: MatrixPosition,
    /// Distance to the requester (set by [`order_by_proximity`] / scoring).
    pub distance: f64,
    /// Replica priority (higher = preferred).
    pub priority: u32,
    /// Node health score (0.0 – 1.0).
    pub health_score: f64,
    /// Estimated latency to this replica (milliseconds).
    pub estimated_latency_ms: u64,
}

impl ReplicaCandidate {
    /// Create a candidate with default distance/priority/latency.
    pub fn new(id: impl Into<String>, position: MatrixPosition, health_score: f64) -> Self {
        Self {
            id: id.into(),
            position,
            distance: 0.0,
            priority: 100,
            health_score,
            estimated_latency_ms: 0,
        }
    }

    /// Euclidean distance from this replica's position to a target.
    pub fn distance_to(&self, target: &MatrixPosition) -> f64 {
        let dx = self.position.x - target.x;
        let dy = self.position.y - target.y;
        let dz = self.position.z - target.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Combined suitability: 40% proximity, 30% priority, 30% health.
    pub fn suitability_score(&self) -> f64 {
        let distance_factor = 1.0 / (1.0 + self.distance);
        let priority_factor = self.priority as f64 / 100.0;
        0.4 * distance_factor + 0.3 * priority_factor + 0.3 * self.health_score
    }

    /// Whether this replica clears the minimum-health bar.
    pub fn is_suitable(&self) -> bool {
        self.health_score >= 0.5
    }
}

/// Criteria a candidate must meet to be considered for a fetch.
#[derive(Debug, Clone)]
pub struct SelectionCriteria {
    /// Preferred distance range `(min, max)`.
    pub distance_range: Option<(f64, f64)>,
    /// Minimum acceptable health score.
    pub min_health: f64,
    /// Maximum acceptable latency (milliseconds).
    pub max_latency_ms: Option<u64>,
    /// Candidate ids to exclude outright.
    pub exclude_ids: HashSet<String>,
    /// Candidate ids to prioritize when otherwise equal.
    pub prioritize_ids: HashSet<String>,
}

impl Default for SelectionCriteria {
    fn default() -> Self {
        Self {
            distance_range: None,
            min_health: 0.5,
            max_latency_ms: None,
            exclude_ids: HashSet::new(),
            prioritize_ids: HashSet::new(),
        }
    }
}

impl SelectionCriteria {
    /// Whether a candidate meets every active criterion.
    pub fn meets_criteria(&self, candidate: &ReplicaCandidate) -> bool {
        if candidate.health_score < self.min_health {
            return false;
        }
        if let Some(max_latency) = self.max_latency_ms {
            if candidate.estimated_latency_ms > max_latency {
                return false;
            }
        }
        if let Some((min_dist, max_dist)) = self.distance_range {
            if candidate.distance < min_dist || candidate.distance > max_dist {
                return false;
            }
        }
        !self.exclude_ids.contains(&candidate.id)
    }
}

/// Order candidate replicas nearest-first for a requester at `client`.
///
/// Returns candidate indices sorted by ascending distance to the requester —
/// the advisory analog of the old `RetrievalPlan::optimize_for_position`. Does
/// not mutate the candidates; the caller fetches in the returned order.
pub fn order_by_proximity(candidates: &[ReplicaCandidate], client: &MatrixPosition) -> Vec<usize> {
    let mut indexed: Vec<(usize, f64)> = candidates
        .iter()
        .enumerate()
        .map(|(idx, c)| (idx, c.distance_to(client)))
        .collect();
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    indexed.into_iter().map(|(idx, _)| idx).collect()
}

/// Advisory replica selector: ranks candidates and tracks observed
/// success/failure to recommend a fetch strategy.
pub struct ReplicaSelector {
    criteria: SelectionCriteria,
    strategy: FallbackStrategy,
    failed_ids: HashSet<String>,
    successful_ids: HashSet<String>,
}

impl ReplicaSelector {
    /// Create a selector with the given criteria and base strategy.
    pub fn new(criteria: SelectionCriteria, strategy: FallbackStrategy) -> Self {
        Self {
            criteria,
            strategy,
            failed_ids: HashSet::new(),
            successful_ids: HashSet::new(),
        }
    }

    /// Select up to `max_replicas` candidates, best-first.
    ///
    /// Filters by criteria and known-failed ids, then orders known-successful
    /// candidates ahead of the rest and by suitability within each group.
    pub fn select_replicas(
        &self,
        candidates: &[ReplicaCandidate],
        max_replicas: usize,
    ) -> Vec<ReplicaCandidate> {
        let mut suitable: Vec<ReplicaCandidate> = candidates
            .iter()
            .filter(|c| self.criteria.meets_criteria(c))
            .filter(|c| !self.failed_ids.contains(&c.id))
            .cloned()
            .collect();

        suitable.sort_by(|a, b| self.rank(a, b));
        suitable.truncate(max_replicas);
        suitable
    }

    /// Ordering: known-successful first, then by descending suitability.
    fn rank(&self, a: &ReplicaCandidate, b: &ReplicaCandidate) -> std::cmp::Ordering {
        let a_ok = self.successful_ids.contains(&a.id);
        let b_ok = self.successful_ids.contains(&b.id);
        match (a_ok, b_ok) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b
                .suitability_score()
                .partial_cmp(&a.suitability_score())
                .unwrap_or(std::cmp::Ordering::Equal),
        }
    }

    /// Record that a fetch from `id` failed.
    pub fn mark_failed(&mut self, id: impl Into<String>) {
        let id = id.into();
        self.successful_ids.remove(&id);
        self.failed_ids.insert(id);
    }

    /// Record that a fetch from `id` succeeded.
    pub fn mark_successful(&mut self, id: impl Into<String>) {
        let id = id.into();
        self.failed_ids.remove(&id);
        self.successful_ids.insert(id);
    }

    /// Clear failure tracking (retry everything).
    pub fn reset_failures(&mut self) {
        self.failed_ids.clear();
    }

    /// Observed failure rate over all recorded fetches (`0.0` when none).
    pub fn failure_rate(&self) -> f64 {
        let total = self.failed_ids.len() + self.successful_ids.len();
        if total == 0 {
            return 0.0;
        }
        self.failed_ids.len() as f64 / total as f64
    }

    /// Whether observed failures exceed the 30% fallback threshold.
    pub fn needs_fallback(&self) -> bool {
        self.failure_rate() > 0.3
    }

    /// Recommend a strategy for the current observed conditions. Only
    /// [`FallbackStrategy::Adaptive`] adapts; other strategies are returned
    /// unchanged.
    pub fn recommend_strategy(&self) -> FallbackStrategy {
        match self.strategy {
            FallbackStrategy::Adaptive => {
                let rate = self.failure_rate();
                if rate > 0.5 {
                    FallbackStrategy::ReedSolomon
                } else if rate > 0.2 {
                    FallbackStrategy::Parallel
                } else {
                    FallbackStrategy::Sequential
                }
            }
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: f64) -> MatrixPosition {
        MatrixPosition { x, y: 0.0, z: 0.0 }
    }

    fn candidate(id: &str, x: f64, health: f64, latency: u64) -> ReplicaCandidate {
        let mut c = ReplicaCandidate::new(id, pos(x), health);
        c.estimated_latency_ms = latency;
        c
    }

    #[test]
    fn test_fallback_strategy_flags() {
        assert!(FallbackStrategy::ReedSolomon.needs_reed_solomon());
        assert!(FallbackStrategy::Parallel.uses_parallel());
        assert!(FallbackStrategy::Adaptive.uses_parallel());
        assert!(!FallbackStrategy::Sequential.uses_parallel());
    }

    #[test]
    fn test_distance_and_proximity_order() {
        let candidates = vec![
            candidate("far", 10.0, 1.0, 0),
            candidate("near", 0.0, 1.0, 0),
            candidate("mid", 5.0, 1.0, 0),
        ];
        // 3-4-5 sanity on the position distance.
        assert_eq!(candidates[1].distance_to(&pos(3.0)), 3.0);

        let order = order_by_proximity(&candidates, &pos(0.0));
        assert_eq!(order[0], 1, "nearest replica must come first");
    }

    #[test]
    fn test_selection_criteria() {
        let criteria = SelectionCriteria {
            min_health: 0.7,
            max_latency_ms: Some(100),
            ..Default::default()
        };
        assert!(criteria.meets_criteria(&candidate("good", 0.0, 0.9, 50)));
        assert!(!criteria.meets_criteria(&candidate("bad-health", 1.0, 0.5, 50)));
        assert!(!criteria.meets_criteria(&candidate("bad-latency", 2.0, 0.9, 200)));
    }

    #[test]
    fn test_exclude_ids() {
        let mut criteria = SelectionCriteria::default();
        criteria.exclude_ids.insert("blocked".to_string());
        assert!(!criteria.meets_criteria(&candidate("blocked", 0.0, 1.0, 0)));
        assert!(criteria.meets_criteria(&candidate("allowed", 0.0, 1.0, 0)));
    }

    #[test]
    fn test_select_replicas_limits_and_ranks() {
        let selector = ReplicaSelector::new(SelectionCriteria::default(), FallbackStrategy::Sequential);
        let candidates = vec![
            candidate("a", 0.0, 0.9, 10),
            candidate("b", 1.0, 0.8, 20),
            candidate("c", 2.0, 0.7, 30),
        ];
        let selected = selector.select_replicas(&candidates, 2);
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn test_failure_tracking_and_rate() {
        let mut selector = ReplicaSelector::new(SelectionCriteria::default(), FallbackStrategy::Adaptive);
        selector.mark_failed("a");
        selector.mark_successful("b");
        assert_eq!(selector.failure_rate(), 0.5);

        // A previously-failed id excluded from selection.
        let candidates = vec![candidate("a", 0.0, 1.0, 0), candidate("b", 0.0, 1.0, 0)];
        let selected = selector.select_replicas(&candidates, 5);
        assert!(selected.iter().all(|c| c.id != "a"));
    }

    #[test]
    fn test_strategy_recommendation() {
        let mut selector = ReplicaSelector::new(SelectionCriteria::default(), FallbackStrategy::Adaptive);

        // 1/5 = 0.2 (not > 0.2): Sequential.
        for id in ["s0", "s1", "s2", "s3"] {
            selector.mark_successful(id);
        }
        selector.mark_failed("f0");
        assert_eq!(selector.recommend_strategy(), FallbackStrategy::Sequential);

        // 3/7 ≈ 0.43 (> 0.2): Parallel.
        selector.mark_failed("f1");
        selector.mark_failed("f2");
        assert_eq!(selector.recommend_strategy(), FallbackStrategy::Parallel);

        // 5/9 ≈ 0.56 (> 0.5): Reed-Solomon.
        selector.mark_failed("f3");
        selector.mark_failed("f4");
        assert_eq!(selector.recommend_strategy(), FallbackStrategy::ReedSolomon);
    }

    #[test]
    fn test_successful_candidates_ranked_first() {
        let mut selector = ReplicaSelector::new(SelectionCriteria::default(), FallbackStrategy::Sequential);
        selector.mark_successful("b");
        let candidates = vec![
            candidate("a", 0.0, 1.0, 0), // higher suitability (nearer)
            candidate("b", 9.0, 0.6, 0), // known-successful but worse score
        ];
        let selected = selector.select_replicas(&candidates, 2);
        assert_eq!(selected[0].id, "b", "known-successful replica ranks first");
    }
}
