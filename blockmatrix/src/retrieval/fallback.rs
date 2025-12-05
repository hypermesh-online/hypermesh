//! Fallback Management
//!
//! Handles missing shards and intelligent replica selection.

use anyhow::Result;
use std::collections::HashSet;

use crate::matrix::MatrixCoordinate;
use crate::assets::storage::Hash;

use super::{ShardLocation, ShardMapEntry};

/// Fallback strategy for handling missing shards
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackStrategy {
    /// Try all replicas sequentially
    Sequential,

    /// Try replicas in parallel
    Parallel,

    /// Use Reed-Solomon to reconstruct from other shards
    ReedSolomon,

    /// Adaptive based on failure rate
    Adaptive,
}

impl FallbackStrategy {
    /// Determine if this strategy requires Reed-Solomon
    pub fn needs_reed_solomon(&self) -> bool {
        matches!(self, FallbackStrategy::ReedSolomon)
    }

    /// Determine if this strategy uses parallel fetching
    pub fn uses_parallel(&self) -> bool {
        matches!(self, FallbackStrategy::Parallel | FallbackStrategy::Adaptive)
    }
}

/// Replica selection criteria
#[derive(Debug, Clone)]
pub struct SelectionCriteria {
    /// Preferred distance range (min, max)
    pub distance_range: Option<(f64, f64)>,

    /// Minimum health score
    pub min_health: f64,

    /// Maximum latency (milliseconds)
    pub max_latency_ms: Option<u64>,

    /// Exclude specific positions
    pub exclude_positions: HashSet<MatrixCoordinate>,

    /// Prioritize specific positions
    pub prioritize_positions: HashSet<MatrixCoordinate>,
}

impl Default for SelectionCriteria {
    fn default() -> Self {
        Self {
            distance_range: None,
            min_health: 0.5, // At least 50% health
            max_latency_ms: None,
            exclude_positions: HashSet::new(),
            prioritize_positions: HashSet::new(),
        }
    }
}

impl SelectionCriteria {
    /// Check if a location meets criteria
    pub fn meets_criteria(&self, location: &ShardLocation) -> bool {
        // Check health score
        if location.health_score < self.min_health {
            return false;
        }

        // Check latency
        if let Some(max_latency) = self.max_latency_ms {
            if location.estimated_latency_ms > max_latency {
                return false;
            }
        }

        // Check distance range
        if let Some((min_dist, max_dist)) = self.distance_range {
            if location.distance < min_dist || location.distance > max_dist {
                return false;
            }
        }

        // Check exclusions
        if self.exclude_positions.contains(&location.position) {
            return false;
        }

        true
    }
}

/// Replica selector for intelligent fallback
pub struct ReplicaSelector {
    /// Selection criteria
    criteria: SelectionCriteria,

    /// Fallback strategy
    strategy: FallbackStrategy,

    /// Track failed positions
    failed_positions: HashSet<MatrixCoordinate>,

    /// Track successful positions
    successful_positions: HashSet<MatrixCoordinate>,
}

impl ReplicaSelector {
    /// Create a new replica selector
    pub fn new(criteria: SelectionCriteria, strategy: FallbackStrategy) -> Self {
        Self {
            criteria,
            strategy,
            failed_positions: HashSet::new(),
            successful_positions: HashSet::new(),
        }
    }

    /// Select best replicas from available locations
    pub fn select_replicas(
        &self,
        locations: &[ShardLocation],
        max_replicas: usize,
    ) -> Vec<ShardLocation> {
        // Filter by criteria
        let mut suitable: Vec<ShardLocation> = locations.iter()
            .filter(|loc| self.criteria.meets_criteria(loc))
            .filter(|loc| !self.failed_positions.contains(&loc.position))
            .cloned()
            .collect();

        // Prioritize successful positions
        suitable.sort_by(|a, b| {
            let a_success = self.successful_positions.contains(&a.position);
            let b_success = self.successful_positions.contains(&b.position);

            match (a_success, b_success) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => {
                    // Sort by suitability score
                    b.suitability_score()
                        .partial_cmp(&a.suitability_score())
                        .unwrap_or(std::cmp::Ordering::Equal)
                }
            }
        });

        // Limit to max replicas
        suitable.truncate(max_replicas);
        suitable
    }

    /// Mark a position as failed
    pub fn mark_failed(&mut self, position: MatrixCoordinate) {
        self.failed_positions.insert(position);
        self.successful_positions.remove(&position);
    }

    /// Mark a position as successful
    pub fn mark_successful(&mut self, position: MatrixCoordinate) {
        self.successful_positions.insert(position);
        self.failed_positions.remove(&position);
    }

    /// Reset failure tracking
    pub fn reset_failures(&mut self) {
        self.failed_positions.clear();
    }

    /// Get failure rate
    pub fn failure_rate(&self) -> f64 {
        let total = self.failed_positions.len() + self.successful_positions.len();
        if total == 0 {
            return 0.0;
        }
        self.failed_positions.len() as f64 / total as f64
    }

    /// Check if fallback is needed
    pub fn needs_fallback(&self) -> bool {
        self.failure_rate() > 0.3 // More than 30% failures
    }

    /// Get strategy recommendation based on current state
    pub fn recommend_strategy(&self) -> FallbackStrategy {
        match self.strategy {
            FallbackStrategy::Adaptive => {
                let failure_rate = self.failure_rate();
                if failure_rate > 0.5 {
                    // High failure rate: use Reed-Solomon
                    FallbackStrategy::ReedSolomon
                } else if failure_rate > 0.2 {
                    // Moderate failures: try parallel
                    FallbackStrategy::Parallel
                } else {
                    // Low failures: sequential is fine
                    FallbackStrategy::Sequential
                }
            }
            other => other,
        }
    }
}

/// Fallback manager for handling retrieval failures
pub struct FallbackManager {
    /// Replica selector
    selector: ReplicaSelector,

    /// Track missing shards
    missing_shards: HashSet<Hash>,

    /// Track retrieved shards
    retrieved_shards: HashSet<Hash>,
}

impl FallbackManager {
    /// Create a new fallback manager
    pub fn new(criteria: SelectionCriteria, strategy: FallbackStrategy) -> Self {
        Self {
            selector: ReplicaSelector::new(criteria, strategy),
            missing_shards: HashSet::new(),
            retrieved_shards: HashSet::new(),
        }
    }

    /// Create with default settings
    pub fn with_defaults() -> Self {
        Self::new(SelectionCriteria::default(), FallbackStrategy::Adaptive)
    }

    /// Handle a failed shard fetch
    pub fn handle_failure(
        &mut self,
        shard_hash: Hash,
        failed_position: MatrixCoordinate,
    ) {
        self.selector.mark_failed(failed_position);
        self.missing_shards.insert(shard_hash);
    }

    /// Handle a successful shard fetch
    pub fn handle_success(
        &mut self,
        shard_hash: Hash,
        successful_position: MatrixCoordinate,
    ) {
        self.selector.mark_successful(successful_position);
        self.retrieved_shards.insert(shard_hash);
        self.missing_shards.remove(&shard_hash);
    }

    /// Get alternative locations for a failed shard
    pub fn get_alternatives(
        &self,
        entry: &ShardMapEntry,
        max_alternatives: usize,
    ) -> Vec<ShardLocation> {
        self.selector.select_replicas(&entry.locations, max_alternatives)
    }

    /// Check if retrieval can succeed with current state
    pub fn can_succeed(&self, min_shards_required: usize, total_shards: usize) -> bool {
        let available = total_shards - self.missing_shards.len();
        available >= min_shards_required
    }

    /// Get retrieval status
    pub fn get_status(&self) -> FallbackStatus {
        FallbackStatus {
            missing_shards: self.missing_shards.len(),
            retrieved_shards: self.retrieved_shards.len(),
            failure_rate: self.selector.failure_rate(),
            needs_fallback: self.selector.needs_fallback(),
            recommended_strategy: self.selector.recommend_strategy(),
        }
    }

    /// Reset state for new retrieval
    pub fn reset(&mut self) {
        self.selector.reset_failures();
        self.missing_shards.clear();
        self.retrieved_shards.clear();
    }
}

/// Fallback status information
#[derive(Debug, Clone)]
pub struct FallbackStatus {
    /// Number of missing shards
    pub missing_shards: usize,

    /// Number of successfully retrieved shards
    pub retrieved_shards: usize,

    /// Current failure rate
    pub failure_rate: f64,

    /// Whether fallback strategy should be activated
    pub needs_fallback: bool,

    /// Recommended strategy for current conditions
    pub recommended_strategy: FallbackStrategy,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_location(
        x: i64,
        health: f64,
        latency: u64,
    ) -> ShardLocation {
        let mut location = ShardLocation::new(
            MatrixCoordinate::new(x, 0, 0).unwrap(),
            health,
        );
        location.estimated_latency_ms = latency;
        location
    }

    #[test]
    fn test_selection_criteria() {
        let criteria = SelectionCriteria {
            min_health: 0.7,
            max_latency_ms: Some(100),
            ..Default::default()
        };

        let good = create_test_location(0, 0.9, 50);
        let bad_health = create_test_location(1, 0.5, 50);
        let bad_latency = create_test_location(2, 0.9, 200);

        assert!(criteria.meets_criteria(&good));
        assert!(!criteria.meets_criteria(&bad_health));
        assert!(!criteria.meets_criteria(&bad_latency));
    }

    #[test]
    fn test_replica_selector() {
        let criteria = SelectionCriteria::default();
        let selector = ReplicaSelector::new(criteria, FallbackStrategy::Sequential);

        let locations = vec![
            create_test_location(0, 0.9, 10),
            create_test_location(1, 0.8, 20),
            create_test_location(2, 0.7, 30),
        ];

        let selected = selector.select_replicas(&locations, 2);
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn test_failure_tracking() {
        let criteria = SelectionCriteria::default();
        let mut selector = ReplicaSelector::new(criteria, FallbackStrategy::Adaptive);

        let pos1 = MatrixCoordinate::new(0, 0, 0).unwrap();
        let pos2 = MatrixCoordinate::new(1, 0, 0).unwrap();

        selector.mark_failed(pos1.clone());
        selector.mark_successful(pos2.clone());

        assert!(selector.failed_positions.contains(&pos1));
        assert!(selector.successful_positions.contains(&pos2));
        assert_eq!(selector.failure_rate(), 0.5);
    }

    #[test]
    fn test_strategy_recommendation() {
        let criteria = SelectionCriteria::default();
        let mut selector = ReplicaSelector::new(criteria, FallbackStrategy::Adaptive);

        // Low failure rate: Sequential
        selector.mark_successful(MatrixCoordinate::new(0, 0, 0).unwrap());
        selector.mark_successful(MatrixCoordinate::new(1, 0, 0).unwrap());
        selector.mark_failed(MatrixCoordinate::new(2, 0, 0).unwrap());
        assert_eq!(selector.recommend_strategy(), FallbackStrategy::Sequential);

        // High failure rate: Reed-Solomon
        selector.mark_failed(MatrixCoordinate::new(3, 0, 0).unwrap());
        selector.mark_failed(MatrixCoordinate::new(4, 0, 0).unwrap());
        assert_eq!(selector.recommend_strategy(), FallbackStrategy::ReedSolomon);
    }

    #[test]
    fn test_fallback_manager() {
        let manager = FallbackManager::with_defaults();
        let status = manager.get_status();

        assert_eq!(status.missing_shards, 0);
        assert_eq!(status.retrieved_shards, 0);
        assert!(!status.needs_fallback);
    }

    #[test]
    fn test_handle_failures() {
        let mut manager = FallbackManager::with_defaults();

        let shard_hash = [1u8; 32];
        let pos = MatrixCoordinate::new(0, 0, 0).unwrap();

        manager.handle_failure(shard_hash, pos.clone());

        let status = manager.get_status();
        assert_eq!(status.missing_shards, 1);
        assert!(manager.missing_shards.contains(&shard_hash));
    }

    #[test]
    fn test_handle_success() {
        let mut manager = FallbackManager::with_defaults();

        let shard_hash = [1u8; 32];
        let pos = MatrixCoordinate::new(0, 0, 0).unwrap();

        manager.handle_success(shard_hash, pos.clone());

        let status = manager.get_status();
        assert_eq!(status.retrieved_shards, 1);
        assert!(manager.retrieved_shards.contains(&shard_hash));
    }

    #[test]
    fn test_can_succeed() {
        let mut manager = FallbackManager::with_defaults();

        // Reed-Solomon 10+4: need 10 out of 14
        let min_required = 10;
        let total_shards = 14;

        // All shards available
        assert!(manager.can_succeed(min_required, total_shards));

        // Mark 3 as missing (11 available)
        for i in 0..3 {
            manager.handle_failure([i as u8; 32], MatrixCoordinate::new(i, 0, 0).unwrap());
        }
        assert!(manager.can_succeed(min_required, total_shards));

        // Mark 2 more as missing (9 available)
        for i in 3..5 {
            manager.handle_failure([i as u8; 32], MatrixCoordinate::new(i, 0, 0).unwrap());
        }
        assert!(!manager.can_succeed(min_required, total_shards));
    }

    #[test]
    fn test_get_alternatives() {
        let manager = FallbackManager::with_defaults();

        let shard_hash = [1u8; 32];
        let locations = vec![
            create_test_location(0, 0.9, 10),
            create_test_location(1, 0.8, 20),
            create_test_location(2, 0.7, 30),
        ];

        let entry = ShardMapEntry::new(shard_hash, locations);
        let alternatives = manager.get_alternatives(&entry, 2);

        assert_eq!(alternatives.len(), 2);
    }

    #[test]
    fn test_reset() {
        let mut manager = FallbackManager::with_defaults();

        let shard_hash = [1u8; 32];
        let pos = MatrixCoordinate::new(0, 0, 0).unwrap();

        manager.handle_failure(shard_hash, pos);
        assert_eq!(manager.missing_shards.len(), 1);

        manager.reset();
        assert_eq!(manager.missing_shards.len(), 0);
        assert_eq!(manager.retrieved_shards.len(), 0);
    }
}
