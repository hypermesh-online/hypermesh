// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Multi-path scheduling strategies.
//!
//! Selects which path(s) to use for each send operation based on
//! pluggable scheduling strategies: round-robin, bandwidth-weighted,
//! lowest-latency, or redundant (send on all paths).

use std::sync::atomic::{AtomicU32, Ordering};

/// Scheduling strategy for path selection.
#[derive(Debug, Clone, PartialEq)]
pub enum PathScheduler {
    /// Cycle through paths in order.
    RoundRobin,
    /// Weight selection by EWMA bandwidth estimates.
    BandwidthWeighted,
    /// Always pick the path with the lowest RTT.
    LowestLatency,
    /// Send on all paths simultaneously (critical data).
    Redundant,
}

/// A path candidate with metrics used for selection decisions.
#[derive(Debug, Clone)]
pub struct PathCandidate {
    /// Unique identifier for this path.
    pub path_id: u32,
    /// Estimated bandwidth in bits per second.
    pub bandwidth_estimate_bps: f64,
    /// Round-trip time in milliseconds.
    pub rtt_ms: f64,
    /// Health score from 0.0 (dead) to 1.0 (perfect).
    pub health_score: f64,
    /// Total bytes sent on this path.
    pub bytes_sent: u64,
}

/// Selects paths based on the configured scheduling strategy.
pub struct PathSelector {
    strategy: PathScheduler,
    round_robin_index: AtomicU32,
}

impl PathSelector {
    /// Create a new selector with the given strategy.
    pub fn new(strategy: PathScheduler) -> Self {
        Self {
            strategy,
            round_robin_index: AtomicU32::new(0),
        }
    }

    /// Select a single path from the candidates.
    ///
    /// Returns `None` for `Redundant` strategy (caller should use
    /// `select_all` instead) or when no suitable candidates exist.
    pub fn select(&self, candidates: &[PathCandidate]) -> Option<u32> {
        if candidates.is_empty() {
            return None;
        }

        match &self.strategy {
            PathScheduler::RoundRobin => {
                let idx = self.round_robin_index.fetch_add(1, Ordering::Relaxed) as usize;
                let chosen = idx % candidates.len();
                Some(candidates[chosen].path_id)
            }

            PathScheduler::BandwidthWeighted => {
                // Weight each candidate by bandwidth * health_score.
                // We quantize the total weight into integer "slots"
                // and use a round-robin index modulo the total number
                // of slots to distribute selections proportionally.
                let weights: Vec<u64> = candidates
                    .iter()
                    .map(|c| {
                        // Scale to integer slots (1 slot per Mbps of weighted bw)
                        let w = (c.bandwidth_estimate_bps * c.health_score) / 1_000_000.0;
                        (w as u64).max(1)
                    })
                    .collect();

                let total_slots: u64 = weights.iter().sum();
                if total_slots == 0 {
                    return Some(candidates[0].path_id);
                }

                let idx = self.round_robin_index.fetch_add(1, Ordering::Relaxed) as u64;
                let position = idx % total_slots;

                let mut cumulative: u64 = 0;
                for (i, &w) in weights.iter().enumerate() {
                    cumulative += w;
                    if position < cumulative {
                        return Some(candidates[i].path_id);
                    }
                }

                // Fallback to last candidate (should not reach here)
                candidates.last().map(|c| c.path_id)
            }

            PathScheduler::LowestLatency => {
                // Select the candidate with the lowest effective latency.
                // Effective latency = rtt_ms / health_score (penalizes unhealthy paths).
                candidates
                    .iter()
                    .filter(|c| c.health_score > 0.0)
                    .min_by(|a, b| {
                        let eff_a = a.rtt_ms / a.health_score;
                        let eff_b = b.rtt_ms / b.health_score;
                        eff_a
                            .partial_cmp(&eff_b)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|c| c.path_id)
            }

            PathScheduler::Redundant => {
                // Redundant mode: caller should use select_all().
                None
            }
        }
    }

    /// Select all paths for the current strategy.
    ///
    /// For `Redundant`, returns all path IDs. For other strategies,
    /// returns a single-element vector with the selected path.
    pub fn select_all(&self, candidates: &[PathCandidate]) -> Vec<u32> {
        if candidates.is_empty() {
            return Vec::new();
        }

        match &self.strategy {
            PathScheduler::Redundant => candidates.iter().map(|c| c.path_id).collect(),
            _ => self.select(candidates).into_iter().collect(),
        }
    }

    /// Get a reference to the current scheduling strategy.
    pub fn strategy(&self) -> &PathScheduler {
        &self.strategy
    }

    /// Change the scheduling strategy.
    pub fn set_strategy(&mut self, strategy: PathScheduler) {
        self.strategy = strategy;
    }

    /// Apply an engauge routing recommendation by mapping lowercase
    /// strategy names to `PathScheduler` variants.
    ///
    /// Accepted names: `"bandwidth"` → `BandwidthWeighted`,
    /// `"latency"` → `LowestLatency`, `"redundant"` → `Redundant`,
    /// anything else → `RoundRobin`.
    ///
    /// Returns `true` if the strategy actually changed.
    pub fn apply_engauge_recommendation(&mut self, strategy_name: &str) -> bool {
        let new_strategy = match strategy_name {
            "bandwidth" => PathScheduler::BandwidthWeighted,
            "latency" => PathScheduler::LowestLatency,
            "redundant" => PathScheduler::Redundant,
            _ => PathScheduler::RoundRobin,
        };

        if self.strategy == new_strategy {
            return false;
        }

        tracing::info!(
            "PathSelector engauge recommendation: {:?} -> {:?} (input={:?})",
            self.strategy, new_strategy, strategy_name,
        );
        self.strategy = new_strategy;
        true
    }

    /// Record observed path metrics for a specific path.
    ///
    /// This updates the `PathCandidate` fields in the provided slice
    /// so that future selection decisions reflect real measurements.
    /// If no candidate with the given `path_id` exists, the call is
    /// silently ignored.
    pub fn record_path_metrics(
        candidates: &mut [PathCandidate],
        path_id: u32,
        bandwidth_mbps: f64,
        latency_ms: f64,
    ) {
        if let Some(candidate) = candidates.iter_mut().find(|c| c.path_id == path_id) {
            candidate.bandwidth_estimate_bps = bandwidth_mbps * 1_000_000.0;
            candidate.rtt_ms = latency_ms;
            tracing::debug!(
                path_id,
                bandwidth_mbps,
                latency_ms,
                "Recorded path metrics from engauge",
            );
        }
    }

    /// Apply an external scheduling recommendation (e.g., from engauge
    /// routing intelligence). Maps the recommended strategy to the
    /// internal `PathScheduler` enum. The `enable_redundant` flag forces
    /// redundant mode regardless of the recommended strategy when `true`.
    ///
    /// Returns `true` if the strategy actually changed.
    pub fn apply_recommendation(
        &mut self,
        enable_redundant: bool,
        recommended: &str,
    ) -> bool {
        let new_strategy = if enable_redundant {
            PathScheduler::Redundant
        } else {
            match recommended {
                "BandwidthWeighted" => PathScheduler::BandwidthWeighted,
                "LowestLatency" => PathScheduler::LowestLatency,
                "Redundant" => PathScheduler::Redundant,
                _ => PathScheduler::RoundRobin,
            }
        };

        if self.strategy == new_strategy {
            return false;
        }

        tracing::info!(
            "PathSelector strategy changed: {:?} -> {:?} (redundant={})",
            self.strategy, new_strategy, enable_redundant,
        );
        self.strategy = new_strategy;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_candidates(count: usize) -> Vec<PathCandidate> {
        (0..count)
            .map(|i| PathCandidate {
                path_id: i as u32,
                bandwidth_estimate_bps: 1_000_000_000.0, // 1 Gbps
                rtt_ms: 10.0,
                health_score: 1.0,
                bytes_sent: 0,
            })
            .collect()
    }

    #[test]
    fn test_round_robin_distribution() {
        let selector = PathSelector::new(PathScheduler::RoundRobin);
        let candidates = make_candidates(3);

        let mut counts = [0u32; 3];
        for _ in 0..6 {
            if let Some(id) = selector.select(&candidates) {
                counts[id as usize] += 1;
            }
        }

        // Each candidate should be selected exactly twice
        for (i, count) in counts.iter().enumerate() {
            assert_eq!(
                *count, 2,
                "Candidate {i} selected {count} times, expected 2"
            );
        }
    }

    #[test]
    fn test_bandwidth_weighted() {
        let selector = PathSelector::new(PathScheduler::BandwidthWeighted);

        let candidates = vec![
            PathCandidate {
                path_id: 0,
                bandwidth_estimate_bps: 100_000_000.0, // 100 Mbps
                rtt_ms: 10.0,
                health_score: 1.0,
                bytes_sent: 0,
            },
            PathCandidate {
                path_id: 1,
                bandwidth_estimate_bps: 1_000_000_000.0, // 1 Gbps (10x more)
                rtt_ms: 10.0,
                health_score: 1.0,
                bytes_sent: 0,
            },
        ];

        let mut counts = [0u32; 2];
        let iterations = 1000;
        for _ in 0..iterations {
            if let Some(id) = selector.select(&candidates) {
                counts[id as usize] += 1;
            }
        }

        // The higher-bandwidth path should be selected significantly more often.
        // With 10:1 bandwidth ratio, path 1 should get roughly 10x more selections.
        assert!(
            counts[1] > counts[0],
            "Higher bandwidth path should be selected more: path0={}, path1={}",
            counts[0],
            counts[1]
        );
    }

    #[test]
    fn test_lowest_latency() {
        let selector = PathSelector::new(PathScheduler::LowestLatency);

        let candidates = vec![
            PathCandidate {
                path_id: 0,
                bandwidth_estimate_bps: 1_000_000_000.0,
                rtt_ms: 50.0, // higher latency
                health_score: 1.0,
                bytes_sent: 0,
            },
            PathCandidate {
                path_id: 1,
                bandwidth_estimate_bps: 1_000_000_000.0,
                rtt_ms: 5.0, // lowest latency
                health_score: 1.0,
                bytes_sent: 0,
            },
            PathCandidate {
                path_id: 2,
                bandwidth_estimate_bps: 1_000_000_000.0,
                rtt_ms: 20.0,
                health_score: 1.0,
                bytes_sent: 0,
            },
        ];

        let selected = selector.select(&candidates);
        assert_eq!(
            selected,
            Some(1),
            "Should select path with lowest RTT (path 1, 5ms)"
        );
    }

    #[test]
    fn test_redundant_selects_all() {
        let selector = PathSelector::new(PathScheduler::Redundant);
        let candidates = make_candidates(4);

        // select() returns None for Redundant
        assert_eq!(selector.select(&candidates), None);

        // select_all() returns all path IDs
        let all = selector.select_all(&candidates);
        assert_eq!(all.len(), 4);
        assert_eq!(all, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_empty_candidates() {
        let selector = PathSelector::new(PathScheduler::RoundRobin);
        assert_eq!(selector.select(&[]), None);
        assert!(selector.select_all(&[]).is_empty());
    }

    #[test]
    fn test_apply_recommendation_changes_strategy() {
        let mut selector = PathSelector::new(PathScheduler::RoundRobin);
        assert_eq!(*selector.strategy(), PathScheduler::RoundRobin);

        let changed = selector.apply_recommendation(false, "LowestLatency");
        assert!(changed);
        assert_eq!(*selector.strategy(), PathScheduler::LowestLatency);

        // Same recommendation should not report a change.
        let not_changed = selector.apply_recommendation(false, "LowestLatency");
        assert!(!not_changed);
    }

    #[test]
    fn test_apply_recommendation_redundant_overrides() {
        let mut selector = PathSelector::new(PathScheduler::RoundRobin);

        let changed = selector.apply_recommendation(true, "BandwidthWeighted");
        assert!(changed);
        // enable_redundant=true overrides any strategy to Redundant.
        assert_eq!(*selector.strategy(), PathScheduler::Redundant);
    }

    #[test]
    fn test_apply_recommendation_unknown_falls_back() {
        let mut selector = PathSelector::new(PathScheduler::LowestLatency);

        let changed = selector.apply_recommendation(false, "UnknownStrategy");
        assert!(changed);
        // Unknown strategies fall back to RoundRobin.
        assert_eq!(*selector.strategy(), PathScheduler::RoundRobin);
    }

    #[test]
    fn test_apply_engauge_recommendation() {
        let mut selector = PathSelector::new(PathScheduler::RoundRobin);

        let changed = selector.apply_engauge_recommendation("bandwidth");
        assert!(changed);
        assert_eq!(*selector.strategy(), PathScheduler::BandwidthWeighted);

        let changed = selector.apply_engauge_recommendation("latency");
        assert!(changed);
        assert_eq!(*selector.strategy(), PathScheduler::LowestLatency);

        let changed = selector.apply_engauge_recommendation("redundant");
        assert!(changed);
        assert_eq!(*selector.strategy(), PathScheduler::Redundant);

        let changed = selector.apply_engauge_recommendation("unknown");
        assert!(changed);
        assert_eq!(*selector.strategy(), PathScheduler::RoundRobin);

        // Same value should not change
        let changed = selector.apply_engauge_recommendation("unknown");
        assert!(!changed);
    }

    #[test]
    fn test_record_path_metrics() {
        let mut candidates = make_candidates(3);

        PathSelector::record_path_metrics(&mut candidates, 1, 500.0, 25.0);

        assert!((candidates[1].bandwidth_estimate_bps - 500_000_000.0).abs() < 1.0);
        assert!((candidates[1].rtt_ms - 25.0).abs() < f64::EPSILON);

        // Path 0 should be unchanged
        assert!((candidates[0].bandwidth_estimate_bps - 1_000_000_000.0).abs() < 1.0);
    }

    #[test]
    fn test_record_path_metrics_unknown_path() {
        let mut candidates = make_candidates(2);

        // Recording metrics for non-existent path_id should be a no-op
        PathSelector::record_path_metrics(&mut candidates, 99, 100.0, 5.0);

        // All candidates unchanged
        for c in &candidates {
            assert!((c.bandwidth_estimate_bps - 1_000_000_000.0).abs() < 1.0);
        }
    }
}
