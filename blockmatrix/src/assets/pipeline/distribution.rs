//! Distribution Stage - Matrix-aware shard placement
//!
//! Uses Phase 1 tensor operations for optimal shard placement across the matrix.

use crate::assets::pipeline::{PipelineError, PipelineResult};
use crate::matrix::coordinate::MatrixCoordinate;
use crate::matrix::tensor::routing::calculate_routing_path;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Matrix constraints for distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixConstraints {
    /// Minimum distance between shards
    pub min_distance: f64,
    /// Maximum distance for retrieval efficiency
    pub max_distance: f64,
    /// Enable load balancing
    pub load_balance: bool,
    /// Maximum hops for routing
    pub max_hops: usize,
}

impl Default for MatrixConstraints {
    fn default() -> Self {
        Self {
            min_distance: 5.0,
            max_distance: 50.0,
            load_balance: true,
            max_hops: 10,
        }
    }
}

/// Distribution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionConfig {
    /// Matrix constraints
    pub constraints: MatrixConstraints,
    /// Network IDs for multi-network distribution
    pub network_ids: Vec<String>,
    /// Preferred matrix zones
    pub preferred_zones: Vec<MatrixZone>,
    /// Replication factor (1 = no replication)
    pub replication_factor: usize,
}

impl Default for DistributionConfig {
    fn default() -> Self {
        Self {
            constraints: MatrixConstraints::default(),
            network_ids: vec!["default".to_string()],
            preferred_zones: Vec::new(),
            replication_factor: 1,
        }
    }
}

/// Matrix zone for preferred placement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixZone {
    /// Zone center
    pub center: MatrixCoordinate,
    /// Zone radius
    pub radius: f64,
    /// Priority (higher = more preferred)
    pub priority: u32,
}

/// Shard placement information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardPlacement {
    /// Shard index
    pub shard_index: usize,
    /// Matrix position
    pub position: MatrixCoordinate,
    /// Network ID
    pub network_id: String,
    /// Node ID at this position (if known)
    pub node_id: Option<String>,
    /// Distance from origin
    pub distance_from_origin: f64,
    /// Routing path to this position
    pub routing_path: Vec<MatrixCoordinate>,
}

impl ShardPlacement {
    /// Calculate distance to another placement
    pub fn distance_to(&self, other: &ShardPlacement) -> f64 {
        self.position.euclidean_distance(&other.position)
    }
}

/// Distributed asset with shard placements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedAsset {
    /// Asset identifier
    pub asset_id: String,
    /// Shard placements
    pub placements: Vec<ShardPlacement>,
    /// Distribution metadata
    pub metadata: DistributionMetadata,
}

/// Distribution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionMetadata {
    /// Total number of shards
    pub total_shards: usize,
    /// Number of networks used
    pub networks_used: usize,
    /// Average distance between shards
    pub avg_shard_distance: f64,
    /// Distribution quality score (0-100)
    pub quality_score: f64,
    /// Distribution timestamp
    pub distributed_at: i64,
}

/// Distribution statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DistributionStats {
    /// Number of shards distributed
    pub shards_distributed: usize,
    /// Number of networks used
    pub networks_used: usize,
    /// Average distance between shards
    pub avg_shard_distance: f64,
    /// Minimum distance between shards
    pub min_shard_distance: f64,
    /// Maximum distance between shards
    pub max_shard_distance: f64,
    /// Distribution time in milliseconds
    pub duration_ms: u64,
    /// Quality score (0-100)
    pub quality_score: f64,
}

/// Matrix-aware distributor
pub struct MatrixDistributor {
    config: DistributionConfig,
    /// Available matrix positions (node_id -> position)
    available_nodes: HashMap<String, MatrixCoordinate>,
}

impl MatrixDistributor {
    /// Create new distributor with configuration
    pub fn new(config: DistributionConfig) -> Self {
        Self {
            config,
            available_nodes: HashMap::new(),
        }
    }

    /// Create distributor with default configuration
    pub fn default() -> Self {
        Self::new(DistributionConfig::default())
    }

    /// Register an available node at a matrix position
    pub fn register_node(&mut self, node_id: String, position: MatrixCoordinate) {
        self.available_nodes.insert(node_id, position);
    }

    /// Unregister a node
    pub fn unregister_node(&mut self, node_id: &str) {
        self.available_nodes.remove(node_id);
    }

    /// Find optimal positions for shard placement
    pub fn find_optimal_positions(
        &self,
        num_shards: usize,
    ) -> PipelineResult<Vec<MatrixCoordinate>> {
        if num_shards == 0 {
            return Err(PipelineError::DistributionFailed(
                "Number of shards must be > 0".to_string()
            ));
        }

        // If we have registered nodes, use them
        if !self.available_nodes.is_empty() {
            return self.select_from_available_nodes(num_shards);
        }

        // Otherwise, generate positions based on constraints
        self.generate_optimal_positions(num_shards)
    }

    /// Select positions from available nodes
    fn select_from_available_nodes(
        &self,
        num_shards: usize,
    ) -> PipelineResult<Vec<MatrixCoordinate>> {
        let mut selected = Vec::new();
        let mut available: Vec<_> = self.available_nodes.values().cloned().collect();

        // Sort by distance from origin for deterministic selection
        available.sort_by(|a, b| {
            let dist_a = a.euclidean_distance(&MatrixCoordinate::origin());
            let dist_b = b.euclidean_distance(&MatrixCoordinate::origin());
            dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Select nodes with maximum distance between them
        for _ in 0..num_shards.min(available.len()) {
            if selected.is_empty() {
                // First node - select closest to origin
                selected.push(available[0].clone());
            } else {
                // Find node with maximum minimum distance to already selected
                let best = available.iter()
                    .filter(|pos| !selected.contains(pos))
                    .max_by(|a, b| {
                        let min_dist_a = selected.iter()
                            .map(|s| s.euclidean_distance(a))
                            .min_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal))
                            .unwrap_or(0.0);
                        let min_dist_b = selected.iter()
                            .map(|s| s.euclidean_distance(b))
                            .min_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal))
                            .unwrap_or(0.0);
                        min_dist_a.partial_cmp(&min_dist_b).unwrap_or(std::cmp::Ordering::Equal)
                    });

                if let Some(node) = best {
                    selected.push(node.clone());
                }
            }
        }

        // If we don't have enough nodes, generate additional positions
        if selected.len() < num_shards {
            let additional = self.generate_optimal_positions(num_shards - selected.len())?;
            selected.extend(additional);
        }

        Ok(selected)
    }

    /// Generate optimal positions based on matrix topology
    fn generate_optimal_positions(
        &self,
        num_shards: usize,
    ) -> PipelineResult<Vec<MatrixCoordinate>> {
        let mut positions = Vec::new();

        // Use sphere packing algorithm for optimal distribution
        let golden_ratio = 1.618033988749895;
        let angle_increment = 2.0 * std::f64::consts::PI / golden_ratio;

        for i in 0..num_shards {
            let t = i as f64 / num_shards as f64;
            let inclination = (1.0 - 2.0 * t).acos();
            let azimuth = angle_increment * i as f64;

            // Map to matrix coordinates with configurable distance
            let radius = self.config.constraints.min_distance
                + (self.config.constraints.max_distance - self.config.constraints.min_distance) * t;

            let x = (radius * inclination.sin() * azimuth.cos()).round() as i64;
            let y = (radius * inclination.sin() * azimuth.sin()).round() as i64;
            let z = (radius * inclination.cos()).round() as i64;

            if let Ok(coord) = MatrixCoordinate::new(x, y, z) {
                positions.push(coord);
            }
        }

        // Ensure we have exactly num_shards positions
        if positions.len() < num_shards {
            return Err(PipelineError::DistributionFailed(
                format!("Could not generate {} positions, only created {}", num_shards, positions.len())
            ));
        }

        Ok(positions)
    }

    /// Distribute shards to optimal positions
    pub fn distribute(
        &self,
        asset_id: String,
        num_shards: usize,
    ) -> PipelineResult<(DistributedAsset, DistributionStats)> {
        let start = std::time::Instant::now();

        // Find optimal positions
        let positions = self.find_optimal_positions(num_shards)?;

        // Create placements with routing paths
        let origin = MatrixCoordinate::origin();
        let mut placements = Vec::new();

        for (i, position) in positions.iter().enumerate() {
            let network_id = self.select_network_for_shard(i);
            let distance = origin.euclidean_distance(position);

            // Calculate routing path using Phase 1 tensor operations
            let max_hop_distance = self.config.constraints.max_distance
                / self.config.constraints.max_hops as f64;
            let routing_path = calculate_routing_path(&origin, position, max_hop_distance);

            // Find node ID if available
            let node_id = self.available_nodes.iter()
                .find(|(_, pos)| *pos == position)
                .map(|(id, _)| id.clone());

            placements.push(ShardPlacement {
                shard_index: i,
                position: position.clone(),
                network_id,
                node_id,
                distance_from_origin: distance,
                routing_path,
            });
        }

        // Calculate statistics
        let stats = self.calculate_stats(&placements, start.elapsed().as_millis() as u64);

        let metadata = DistributionMetadata {
            total_shards: num_shards,
            networks_used: self.config.network_ids.len(),
            avg_shard_distance: stats.avg_shard_distance,
            quality_score: stats.quality_score,
            distributed_at: chrono::Utc::now().timestamp(),
        };

        let distributed = DistributedAsset {
            asset_id,
            placements,
            metadata,
        };

        Ok((distributed, stats))
    }

    /// Select network for a shard (round-robin or load-balanced)
    fn select_network_for_shard(&self, shard_index: usize) -> String {
        if self.config.network_ids.is_empty() {
            "default".to_string()
        } else {
            self.config.network_ids[shard_index % self.config.network_ids.len()].clone()
        }
    }

    /// Calculate distribution statistics
    fn calculate_stats(&self, placements: &[ShardPlacement], duration_ms: u64) -> DistributionStats {
        let mut distances = Vec::new();

        // Calculate all pairwise distances
        for i in 0..placements.len() {
            for j in (i + 1)..placements.len() {
                let dist = placements[i].distance_to(&placements[j]);
                distances.push(dist);
            }
        }

        let avg_distance = if !distances.is_empty() {
            distances.iter().sum::<f64>() / distances.len() as f64
        } else {
            0.0
        };

        let min_distance = distances.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_distance = distances.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        // Calculate quality score (0-100)
        let quality_score = self.calculate_quality_score(&placements, avg_distance, min_distance);

        let networks_used = placements.iter()
            .map(|p| &p.network_id)
            .collect::<std::collections::HashSet<_>>()
            .len();

        DistributionStats {
            shards_distributed: placements.len(),
            networks_used,
            avg_shard_distance: avg_distance,
            min_shard_distance: min_distance,
            max_shard_distance: max_distance,
            duration_ms,
            quality_score,
        }
    }

    /// Calculate distribution quality score
    fn calculate_quality_score(
        &self,
        placements: &[ShardPlacement],
        avg_distance: f64,
        min_distance: f64,
    ) -> f64 {
        // Quality factors:
        // 1. Distance uniformity (higher is better)
        // 2. Constraint compliance (meets min/max distance)
        // 3. Network distribution

        let mut score = 0.0;

        // Distance uniformity (40 points)
        if avg_distance > 0.0 {
            let uniformity = min_distance / avg_distance;
            score += uniformity * 40.0;
        }

        // Constraint compliance (40 points)
        let meets_min = min_distance >= self.config.constraints.min_distance;
        let meets_max = avg_distance <= self.config.constraints.max_distance;
        if meets_min && meets_max {
            score += 40.0;
        } else if meets_min || meets_max {
            score += 20.0;
        }

        // Network distribution (20 points)
        let networks_used = placements.iter()
            .map(|p| &p.network_id)
            .collect::<std::collections::HashSet<_>>()
            .len();
        let network_score = (networks_used as f64 / self.config.network_ids.len() as f64) * 20.0;
        score += network_score;

        score.min(100.0)
    }

    /// Get distribution configuration
    pub fn config(&self) -> &DistributionConfig {
        &self.config
    }

    /// Get number of registered nodes
    pub fn node_count(&self) -> usize {
        self.available_nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimal_position_generation() {
        let distributor = MatrixDistributor::default();
        let positions = distributor.find_optimal_positions(10).unwrap();

        assert_eq!(positions.len(), 10);

        // Check distances are within constraints
        for i in 0..positions.len() {
            for j in (i + 1)..positions.len() {
                let dist = positions[i].euclidean_distance(&positions[j]);
                assert!(dist >= distributor.config.constraints.min_distance * 0.8); // Allow some tolerance
            }
        }
    }

    #[test]
    fn test_distribution() {
        let distributor = MatrixDistributor::default();
        let (distributed, stats) = distributor.distribute("test-asset".to_string(), 14).unwrap();

        assert_eq!(distributed.placements.len(), 14);
        assert_eq!(stats.shards_distributed, 14);
        assert!(stats.avg_shard_distance > 0.0);
        assert!(stats.quality_score > 0.0);
    }

    #[test]
    fn test_node_registration() {
        let mut distributor = MatrixDistributor::default();

        // Register some nodes
        distributor.register_node("node1".to_string(), MatrixCoordinate::new(10, 20, 30).unwrap());
        distributor.register_node("node2".to_string(), MatrixCoordinate::new(50, 60, 70).unwrap());

        assert_eq!(distributor.node_count(), 2);

        // Distribution should prefer registered nodes
        let (distributed, _) = distributor.distribute("test".to_string(), 2).unwrap();
        assert_eq!(distributed.placements.len(), 2);

        // At least some placements should have node IDs
        let with_nodes = distributed.placements.iter().filter(|p| p.node_id.is_some()).count();
        assert!(with_nodes > 0);
    }

    #[test]
    fn test_multi_network_distribution() {
        let config = DistributionConfig {
            network_ids: vec!["net1".to_string(), "net2".to_string(), "net3".to_string()],
            ..Default::default()
        };

        let distributor = MatrixDistributor::new(config);
        let (distributed, stats) = distributor.distribute("test".to_string(), 6).unwrap();

        // Should use multiple networks
        let networks: std::collections::HashSet<_> = distributed.placements.iter()
            .map(|p| &p.network_id)
            .collect();

        assert!(networks.len() > 1);
        assert!(stats.networks_used > 1);
    }

    #[test]
    fn test_routing_paths() {
        let distributor = MatrixDistributor::default();
        let (distributed, _) = distributor.distribute("test".to_string(), 5).unwrap();

        // All placements should have routing paths
        for placement in &distributed.placements {
            assert!(!placement.routing_path.is_empty());
            assert_eq!(placement.routing_path.first().unwrap(), &MatrixCoordinate::origin());
            assert_eq!(placement.routing_path.last().unwrap(), &placement.position);
        }
    }

    #[test]
    fn test_quality_score() {
        let distributor = MatrixDistributor::default();
        let (_, stats) = distributor.distribute("test".to_string(), 10).unwrap();

        // Quality score should be in range [0, 100]
        assert!(stats.quality_score >= 0.0);
        assert!(stats.quality_score <= 100.0);
    }
}
