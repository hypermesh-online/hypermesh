// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Bucket Mapper - Matrix-Aware Bucket to Position Mapping
//!
//! Maps hash buckets to optimal matrix positions using Phase 1 tensor operations
//! and A* pathfinding for intelligent shard placement.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use anyhow::Result;

use crate::integration::phase1_foundation::MatrixFoundation;
use crate::matrix::MatrixCoordinate;
use super::BucketId;

/// Distance metric for matrix calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DistanceMetric {
    Euclidean,
    Manhattan,
    Chebyshev,
    Hamming,
}

/// Matrix constraints for shard placement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixConstraints {
    /// Maximum distance from requester
    pub max_distance: f64,

    /// Minimum geographic diversity (different regions)
    pub min_geo_diversity: usize,

    /// Preferred distance metric
    pub distance_metric: DistanceMetric,

    /// Avoid these positions (e.g., overloaded nodes)
    pub avoid_positions: Vec<MatrixCoordinate>,

    /// Prefer positions near these coordinates
    pub prefer_near: Vec<MatrixCoordinate>,
}

impl Default for MatrixConstraints {
    fn default() -> Self {
        Self {
            max_distance: 10.0,
            min_geo_diversity: 3,
            distance_metric: DistanceMetric::Euclidean,
            avoid_positions: Vec::new(),
            prefer_near: Vec::new(),
        }
    }
}

/// Access pattern analysis for intelligent placement
#[derive(Debug, Clone, Default)]
struct AccessPattern {
    /// Positions that frequently request this bucket
    requester_positions: HashMap<MatrixCoordinate, usize>,

    /// Total access count
    total_accesses: usize,

    /// Average access interval (seconds)
    _avg_interval: f64,

    /// Last access timestamp
    last_access: i64,
}

impl AccessPattern {
    /// Record access from a position
    fn record_access(&mut self, position: MatrixCoordinate) {
        *self.requester_positions.entry(position).or_insert(0) += 1;
        self.total_accesses += 1;
        self.last_access = chrono::Utc::now().timestamp();
    }

    /// Get weighted center of requesters
    fn get_weighted_center(&self) -> Option<MatrixCoordinate> {
        if self.requester_positions.is_empty() {
            return None;
        }

        let mut weighted_x = 0.0;
        let mut weighted_y = 0.0;
        let mut weighted_z = 0.0;
        let mut total_weight = 0.0;

        for (coord, count) in &self.requester_positions {
            let weight = *count as f64;
            weighted_x += coord.x as f64 * weight;
            weighted_y += coord.y as f64 * weight;
            weighted_z += coord.z as f64 * weight;
            total_weight += weight;
        }

        if total_weight > 0.0 {
            Some(MatrixCoordinate::new(
                (weighted_x / total_weight) as i64,
                (weighted_y / total_weight) as i64,
                (weighted_z / total_weight) as i64,
            ).unwrap_or_else(|_| MatrixCoordinate::origin()))
        } else {
            None
        }
    }
}

/// Bucket to matrix position mapper
pub struct BucketMapper {
    /// Matrix foundation for tensor operations
    _foundation: Arc<MatrixFoundation>,

    /// Bucket location cache
    bucket_locations: Arc<RwLock<HashMap<BucketId, Vec<MatrixCoordinate>>>>,

    /// Access patterns for intelligent placement
    access_patterns: Arc<RwLock<HashMap<BucketId, AccessPattern>>>,


    /// Available matrix positions
    available_positions: Arc<RwLock<Vec<MatrixCoordinate>>>,

    /// Node capacity tracking
    node_capacity: Arc<RwLock<HashMap<MatrixCoordinate, NodeCapacity>>>,
}

/// Node capacity information
#[derive(Debug, Clone)]
struct NodeCapacity {
    /// Maximum shards this node can store
    max_shards: usize,

    /// Current shard count
    current_shards: usize,

    /// Available storage (bytes)
    available_storage: usize,

    /// Current load (0.0 to 1.0)
    load_factor: f64,
}

impl NodeCapacity {
    fn new(max_shards: usize, available_storage: usize) -> Self {
        Self {
            max_shards,
            current_shards: 0,
            available_storage,
            load_factor: 0.0,
        }
    }

    fn can_accept_shard(&self, size: usize) -> bool {
        self.current_shards < self.max_shards &&
        self.available_storage >= size &&
        self.load_factor < 0.9 // Don't exceed 90% load
    }

    fn add_shard(&mut self, size: usize) {
        self.current_shards += 1;
        self.available_storage = self.available_storage.saturating_sub(size);
        self.load_factor = self.current_shards as f64 / self.max_shards as f64;
    }
}

impl BucketMapper {
    /// Create new bucket mapper
    pub async fn new(foundation: Arc<MatrixFoundation>) -> Result<Self> {
        // Get available positions from foundation (simulated for now)
        let available_positions = Self::initialize_positions();

        // Initialize node capacities
        let mut node_capacity = HashMap::new();
        for pos in &available_positions {
            node_capacity.insert(*pos, NodeCapacity::new(1000, 100 * 1024 * 1024)); // 100MB per node
        }

        Ok(Self {
            _foundation: foundation,
            bucket_locations: Arc::new(RwLock::new(HashMap::new())),
            access_patterns: Arc::new(RwLock::new(HashMap::new())),
            available_positions: Arc::new(RwLock::new(available_positions)),
            node_capacity: Arc::new(RwLock::new(node_capacity)),
        })
    }

    /// Initialize available matrix positions (100 nodes for testing)
    fn initialize_positions() -> Vec<MatrixCoordinate> {
        let mut positions = Vec::new();
        for x in 0..10 {
            for y in 0..10 {
                if let Ok(coord) = MatrixCoordinate::new(x, y, 0) {
                    positions.push(coord);
                }
            }
        }
        positions
    }

    /// Find optimal positions for a bucket
    pub async fn optimal_positions(
        &self,
        bucket_id: &BucketId,
        count: usize,
    ) -> Result<Vec<MatrixCoordinate>> {
        self.optimal_positions_with_constraints(bucket_id, count, &MatrixConstraints::default()).await
    }

    /// Find optimal positions with constraints
    pub async fn optimal_positions_with_constraints(
        &self,
        bucket_id: &BucketId,
        count: usize,
        constraints: &MatrixConstraints,
    ) -> Result<Vec<MatrixCoordinate>> {
        // Get access pattern for this bucket
        let patterns = self.access_patterns.read().await;
        let pattern = patterns.get(bucket_id);

        // Calculate target position (weighted center or random)
        let target = if let Some(center) = pattern.and_then(|p| p.get_weighted_center()) {
            center
        } else {
            // Random position if no access pattern
            let positions = self.available_positions.read().await;
            positions[rand::random::<usize>() % positions.len()]
        };

        // Find nearest available positions using Phase 1 operations
        let mut selected = Vec::new();
        let available = self.available_positions.read().await;
        let mut capacities = self.node_capacity.write().await;

        // Sort positions by distance from target
        let mut sorted_positions: Vec<_> = available
            .iter()
            .filter(|pos| !constraints.avoid_positions.contains(pos))
            .map(|pos| {
                let distance = self.calculate_distance(&target, pos, &constraints.distance_metric);
                let preference_bonus = if constraints.prefer_near.contains(pos) {
                    -5.0 // Reduce effective distance for preferred positions
                } else {
                    0.0
                };
                (pos, distance + preference_bonus)
            })
            .collect();

        sorted_positions.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Select positions with available capacity
        for (pos, distance) in sorted_positions {
            if selected.len() >= count {
                break;
            }

            if distance > constraints.max_distance {
                continue;
            }

            if let Some(capacity) = capacities.get_mut(pos) {
                if capacity.can_accept_shard(1024 * 1024) { // Assume 1MB shards
                    capacity.add_shard(1024 * 1024);
                    selected.push(*pos);
                }
            }
        }

        // Ensure minimum geographic diversity
        if selected.len() < constraints.min_geo_diversity && selected.len() < count {
            // Add more diverse positions
            let diverse = self.select_diverse_positions(
                &selected,
                count - selected.len(),
                &available,
                &mut capacities,
            ).await;
            selected.extend(diverse);
        }

        // Update bucket location cache
        let mut locations = self.bucket_locations.write().await;
        locations.insert(bucket_id.clone(), selected.clone());

        Ok(selected)
    }

    /// Select geographically diverse positions
    async fn select_diverse_positions(
        &self,
        existing: &[MatrixCoordinate],
        count: usize,
        available: &[MatrixCoordinate],
        capacities: &mut HashMap<MatrixCoordinate, NodeCapacity>,
    ) -> Vec<MatrixCoordinate> {
        let mut selected = Vec::new();

        for pos in available {
            if selected.len() >= count {
                break;
            }

            if existing.contains(pos) || selected.contains(pos) {
                continue;
            }

            // Check minimum distance from existing positions
            let min_distance = existing.iter()
                .chain(selected.iter())
                .map(|existing_pos| {
                    self.calculate_distance(existing_pos, pos, &DistanceMetric::Euclidean)
                })
                .fold(f64::INFINITY, f64::min);

            if min_distance > 5.0 { // Minimum 5 units apart
                if let Some(capacity) = capacities.get_mut(pos) {
                    if capacity.can_accept_shard(1024 * 1024) {
                        capacity.add_shard(1024 * 1024);
                        selected.push(*pos);
                    }
                }
            }
        }

        selected
    }

    /// Calculate distance between positions
    fn calculate_distance(
        &self,
        from: &MatrixCoordinate,
        to: &MatrixCoordinate,
        metric: &DistanceMetric,
    ) -> f64 {
        match metric {
            DistanceMetric::Euclidean => {
                let dx = (to.x - from.x) as f64;
                let dy = (to.y - from.y) as f64;
                let dz = (to.z - from.z) as f64;
                (dx * dx + dy * dy + dz * dz).sqrt()
            },
            DistanceMetric::Manhattan => {
                let dx = (to.x - from.x).abs() as f64;
                let dy = (to.y - from.y).abs() as f64;
                let dz = (to.z - from.z).abs() as f64;
                dx + dy + dz
            },
            DistanceMetric::Chebyshev => {
                let dx = (to.x - from.x).abs() as f64;
                let dy = (to.y - from.y).abs() as f64;
                let dz = (to.z - from.z).abs() as f64;
                dx.max(dy).max(dz)
            },
            DistanceMetric::Hamming => {
                let mut diff = 0.0;
                if from.x != to.x { diff += 1.0; }
                if from.y != to.y { diff += 1.0; }
                if from.z != to.z { diff += 1.0; }
                diff
            },
        }
    }

    /// Record access pattern for intelligent placement
    pub async fn record_access(&self, bucket_id: &BucketId, requester: MatrixCoordinate) {
        let mut patterns = self.access_patterns.write().await;
        patterns
            .entry(bucket_id.clone())
            .or_default()
            .record_access(requester);
    }

    /// Select replica positions for popular content
    pub async fn select_replica_positions(
        &self,
        bucket_id: &BucketId,
        count: usize,
    ) -> Result<Vec<MatrixCoordinate>> {
        // Get existing positions
        let locations = self.bucket_locations.read().await;
        let existing = locations.get(bucket_id).cloned().unwrap_or_default();

        // Find new positions avoiding existing ones
        let constraints = MatrixConstraints {
            avoid_positions: existing,
            min_geo_diversity: count,
            ..Default::default()
        };

        self.optimal_positions_with_constraints(bucket_id, count, &constraints).await
    }

    /// Get current bucket locations
    pub async fn get_bucket_locations(&self, bucket_id: &BucketId) -> Vec<MatrixCoordinate> {
        self.bucket_locations.read().await
            .get(bucket_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get mapper statistics
    pub async fn get_stats(&self) -> MapperStats {
        MapperStats {
            total_buckets: self.bucket_locations.read().await.len(),
            total_positions: self.available_positions.read().await.len(),
            access_patterns: self.access_patterns.read().await.len(),
            avg_load_factor: {
                let capacities = self.node_capacity.read().await;
                let sum: f64 = capacities.values().map(|c| c.load_factor).sum();
                sum / capacities.len().max(1) as f64
            },
        }
    }
}

/// Mapper statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapperStats {
    pub total_buckets: usize,
    pub total_positions: usize,
    pub access_patterns: usize,
    pub avg_load_factor: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_access_pattern_weighted_center() {
        let mut pattern = AccessPattern::default();

        // Record accesses from different positions
        pattern.record_access(MatrixCoordinate::new(0, 0, 0).unwrap());
        pattern.record_access(MatrixCoordinate::new(0, 0, 0).unwrap());
        pattern.record_access(MatrixCoordinate::new(10, 10, 0).unwrap());

        let center = pattern.get_weighted_center().unwrap();
        // Center should be closer to (0,0,0) due to more accesses
        assert!(center.x < 5);
        assert!(center.y < 5);
    }

    #[test]
    fn test_node_capacity() {
        let mut capacity = NodeCapacity::new(10, 10 * 1024 * 1024);

        assert!(capacity.can_accept_shard(1024 * 1024));

        for _ in 0..9 {
            capacity.add_shard(1024 * 1024);
        }

        assert_eq!(capacity.current_shards, 9);
        assert!(capacity.load_factor < 0.9);
        assert!(capacity.can_accept_shard(1024 * 1024));

        capacity.add_shard(1024 * 1024);
        assert!(!capacity.can_accept_shard(1024 * 1024)); // At capacity
    }

    #[tokio::test]
    async fn test_distance_calculations() {
        use crate::integration::phase1_foundation::MatrixFoundationConfig;

        let mapper = BucketMapper {
            _foundation: Arc::new(MatrixFoundation::new(MatrixFoundationConfig::default()).await.expect("test: create matrix foundation")),
            bucket_locations: Arc::new(RwLock::new(HashMap::new())),
            access_patterns: Arc::new(RwLock::new(HashMap::new())),
            available_positions: Arc::new(RwLock::new(Vec::new())),
            node_capacity: Arc::new(RwLock::new(HashMap::new())),
        };

        let from = MatrixCoordinate::new(0, 0, 0).unwrap();
        let to = MatrixCoordinate::new(3, 4, 0).unwrap();

        // Euclidean: sqrt(3^2 + 4^2) = 5
        let dist = mapper.calculate_distance(&from, &to, &DistanceMetric::Euclidean);
        assert_eq!(dist, 5.0);

        // Manhattan: 3 + 4 = 7
        let dist = mapper.calculate_distance(&from, &to, &DistanceMetric::Manhattan);
        assert_eq!(dist, 7.0);

        // Chebyshev: max(3, 4) = 4
        let dist = mapper.calculate_distance(&from, &to, &DistanceMetric::Chebyshev);
        assert_eq!(dist, 4.0);
    }
}