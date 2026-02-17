// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Matrix-Aware Shard Distribution Optimizer
//!
//! Applies 8-octant distribution with golden ratio distance optimization
//! WITHIN the eligible node pool approved by PoS validation.

use crate::assets::core::{AssetError, AssetResult};
use crate::assets::pipeline::sharding::Shard;
use crate::distribution::{NodeInfo, ShardPlacement, DistributionResult};
use crate::matrix::coordinate::MatrixCoordinate;
use serde::{Deserialize, Serialize};

/// Golden ratio constant for optimal spacing
const GOLDEN_RATIO: f64 = 1.618033988749895;

/// Octant identifier (0-7)
///
/// 3D space divided into 8 octants based on sign of coordinates:
/// - Octant 0: (+x, +y, +z)
/// - Octant 1: (-x, +y, +z)
/// - Octant 2: (+x, -y, +z)
/// - Octant 3: (-x, -y, +z)
/// - Octant 4: (+x, +y, -z)
/// - Octant 5: (-x, +y, -z)
/// - Octant 6: (+x, -y, -z)
/// - Octant 7: (-x, -y, -z)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Octant(pub u8);

impl Octant {
    /// Determine octant from matrix coordinate
    pub fn from_coordinate(coord: &MatrixCoordinate) -> Self {
        let octant =
            (if coord.x < 0 { 1 } else { 0 }) |
            (if coord.y < 0 { 2 } else { 0 }) |
            (if coord.z < 0 { 4 } else { 0 });
        Octant(octant)
    }

    /// Get octant value (0-7)
    pub fn value(&self) -> u8 {
        self.0
    }
}

/// Octant distribution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OctantDistribution {
    /// Shard placements with octant assignments
    pub placements: Vec<ShardPlacement>,
    /// Distribution quality score (0-100)
    pub quality_score: f64,
    /// Number of octants used
    pub octants_used: usize,
    /// Average inter-shard distance
    pub avg_distance: f64,
}

/// Distribute shards across 8 octants using eligible nodes
///
/// Applies matrix-aware optimization WITHIN eligible node pool:
/// 1. Group eligible nodes by octant
/// 2. Assign shards to octants using golden ratio
/// 3. Select optimal node within each octant
///
/// # Arguments
///
/// * `shards` - Shards to distribute
/// * `eligible_nodes` - Nodes that passed PoS validation
///
/// # Returns
///
/// Octant distribution with placement coordinates
pub fn distribute_across_octants(
    shards: Vec<Shard>,
    eligible_nodes: &[NodeInfo],
) -> AssetResult<DistributionResult> {
    if eligible_nodes.is_empty() {
        return Err(AssetError::ValidationError {
            message: "No eligible nodes for distribution".to_string(),
        });
    }

    if shards.is_empty() {
        return Err(AssetError::ValidationError {
            message: "No shards to distribute".to_string(),
        });
    }

    // Group nodes by octant
    let octant_nodes = group_nodes_by_octant(eligible_nodes);

    // Calculate octant assignments for shards
    let shard_octants = calculate_octant_placements(shards.len(), &octant_nodes)?;

    // Place shards in assigned octants
    let placements = place_shards_in_octants(
        &shards,
        &shard_octants,
        &octant_nodes,
        eligible_nodes,
    )?;

    // Calculate statistics
    let stats = calculate_distribution_stats(&placements);

    Ok(DistributionResult {
        asset_id: "unknown".to_string(), // Set by caller
        placements,
        quality_score: stats.quality_score,
        octants_used: stats.octants_used,
        avg_distance: stats.avg_distance,
    })
}

/// Calculate optimal octant placements for shards
pub fn calculate_octant_placements(
    num_shards: usize,
    octant_nodes: &[Vec<&NodeInfo>; 8],
) -> AssetResult<Vec<u8>> {
    let mut placements = Vec::with_capacity(num_shards);

    // Count available octants
    let available_octants: Vec<u8> = octant_nodes
        .iter()
        .enumerate()
        .filter(|(_, nodes)| !nodes.is_empty())
        .map(|(i, _)| i as u8)
        .collect();

    if available_octants.is_empty() {
        return Err(AssetError::ValidationError {
            message: "No octants have eligible nodes".to_string(),
        });
    }

    // Distribute shards across available octants using golden ratio
    let golden_increment = GOLDEN_RATIO * available_octants.len() as f64;

    for i in 0..num_shards {
        let octant_idx = ((i as f64 * golden_increment) as usize) % available_octants.len();
        placements.push(available_octants[octant_idx]);
    }

    Ok(placements)
}

/// Group nodes by their octant position
fn group_nodes_by_octant(nodes: &[NodeInfo]) -> [Vec<&NodeInfo>; 8] {
    let mut octant_groups: [Vec<&NodeInfo>; 8] = Default::default();

    for node in nodes {
        let octant = Octant::from_coordinate(&node.position);
        octant_groups[octant.value() as usize].push(node);
    }

    octant_groups
}

/// Place shards in their assigned octants
fn place_shards_in_octants(
    shards: &[Shard],
    octant_assignments: &[u8],
    octant_nodes: &[Vec<&NodeInfo>; 8],
    all_eligible_nodes: &[NodeInfo],
) -> AssetResult<Vec<ShardPlacement>> {
    let origin = MatrixCoordinate::origin();
    let mut placements = Vec::new();

    for (shard_idx, &octant) in octant_assignments.iter().enumerate() {
        // Get nodes in this octant
        let nodes = &octant_nodes[octant as usize];

        if nodes.is_empty() {
            return Err(AssetError::ValidationError {
                message: format!("No nodes available in octant {}", octant),
            });
        }

        // Select node with optimal distance (closest to golden ratio distance)
        let target_distance = GOLDEN_RATIO * (shard_idx as f64 + 1.0) * 10.0;
        let node = select_optimal_node(nodes, &origin, target_distance);

        placements.push(ShardPlacement {
            shard_index: shard_idx,
            position: node.position.clone(),
            node_id: node.node_id.clone(),
            octant,
            distance_from_origin: origin.euclidean_distance(&node.position),
        });
    }

    Ok(placements)
}

/// Select optimal node within octant based on distance target
fn select_optimal_node<'a>(
    nodes: &[&'a NodeInfo],
    origin: &MatrixCoordinate,
    target_distance: f64,
) -> &'a NodeInfo {
    nodes
        .iter()
        .min_by(|a, b| {
            let dist_a = (origin.euclidean_distance(&a.position) - target_distance).abs();
            let dist_b = (origin.euclidean_distance(&b.position) - target_distance).abs();
            dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap()
}

/// Distribution statistics
struct DistributionStats {
    quality_score: f64,
    octants_used: usize,
    avg_distance: f64,
}

/// Calculate distribution quality metrics
fn calculate_distribution_stats(placements: &[ShardPlacement]) -> DistributionStats {
    // Count unique octants
    let mut octants_used = std::collections::HashSet::new();
    for p in placements {
        octants_used.insert(p.octant);
    }

    // Calculate average distance
    let avg_distance = if !placements.is_empty() {
        placements.iter().map(|p| p.distance_from_origin).sum::<f64>()
            / placements.len() as f64
    } else {
        0.0
    };

    // Calculate pairwise distances
    let mut distances = Vec::new();
    for i in 0..placements.len() {
        for j in (i + 1)..placements.len() {
            let dist = placements[i]
                .position
                .euclidean_distance(&placements[j].position);
            distances.push(dist);
        }
    }

    // Quality score based on:
    // - Octant distribution (40 points)
    // - Distance uniformity (40 points)
    // - Coverage (20 points)
    let mut quality = 0.0;

    // Octant distribution score
    let octant_ratio = octants_used.len() as f64 / 8.0;
    quality += octant_ratio * 40.0;

    // Distance uniformity score
    if !distances.is_empty() {
        let mean = distances.iter().sum::<f64>() / distances.len() as f64;
        let variance = distances
            .iter()
            .map(|d| (d - mean).powi(2))
            .sum::<f64>()
            / distances.len() as f64;
        let std_dev = variance.sqrt();
        let uniformity = 1.0 - (std_dev / mean).min(1.0);
        quality += uniformity * 40.0;
    }

    // Coverage score (all shards placed)
    quality += 20.0;

    DistributionStats {
        quality_score: quality.min(100.0),
        octants_used: octants_used.len(),
        avg_distance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::pipeline::sharding::ShardMetadata;

    fn create_test_shard(index: usize) -> Shard {
        Shard {
            data: vec![0u8; 1024],
            metadata: ShardMetadata {
                index,
                is_parity: false,
                size: 1024,
                original_size: 1024,
                hash: format!("hash-{}", index),
            },
        }
    }

    fn create_test_nodes() -> Vec<NodeInfo> {
        vec![
            NodeInfo::new(
                "node1".to_string(),
                MatrixCoordinate::new(10, 10, 10).unwrap(),
                "PrivateNetwork".to_string(),
                1_000_000_000,
                "network1".to_string(),
            ),
            NodeInfo::new(
                "node2".to_string(),
                MatrixCoordinate::new(-10, 10, 10).unwrap(),
                "PrivateNetwork".to_string(),
                1_000_000_000,
                "network1".to_string(),
            ),
            NodeInfo::new(
                "node3".to_string(),
                MatrixCoordinate::new(10, -10, 10).unwrap(),
                "PrivateNetwork".to_string(),
                1_000_000_000,
                "network1".to_string(),
            ),
            NodeInfo::new(
                "node4".to_string(),
                MatrixCoordinate::new(-10, -10, 10).unwrap(),
                "PrivateNetwork".to_string(),
                1_000_000_000,
                "network1".to_string(),
            ),
        ]
    }

    #[test]
    fn test_octant_from_coordinate() {
        let coord1 = MatrixCoordinate::new(10, 10, 10).unwrap();
        assert_eq!(Octant::from_coordinate(&coord1).value(), 0);

        let coord2 = MatrixCoordinate::new(-10, 10, 10).unwrap();
        assert_eq!(Octant::from_coordinate(&coord2).value(), 1);

        let coord3 = MatrixCoordinate::new(10, -10, 10).unwrap();
        assert_eq!(Octant::from_coordinate(&coord3).value(), 2);
    }

    #[test]
    fn test_group_nodes_by_octant() {
        let nodes = create_test_nodes();
        let grouped = group_nodes_by_octant(&nodes);

        // Verify nodes are in correct octants
        assert_eq!(grouped[0].len(), 1); // (+,+,+)
        assert_eq!(grouped[1].len(), 1); // (-,+,+)
        assert_eq!(grouped[2].len(), 1); // (+,-,+)
        assert_eq!(grouped[3].len(), 1); // (-,-,+)
    }

    #[test]
    fn test_calculate_octant_placements() {
        let nodes = create_test_nodes();
        let grouped = group_nodes_by_octant(&nodes);

        let placements = calculate_octant_placements(8, &grouped).unwrap();
        assert_eq!(placements.len(), 8);

        // Verify octants are distributed
        let unique_octants: std::collections::HashSet<_> =
            placements.iter().copied().collect();
        assert!(unique_octants.len() > 1);
    }

    #[test]
    fn test_distribute_across_octants() {
        let nodes = create_test_nodes();
        let shards = vec![
            create_test_shard(0),
            create_test_shard(1),
            create_test_shard(2),
            create_test_shard(3),
        ];

        let result = distribute_across_octants(shards, &nodes).unwrap();

        assert_eq!(result.placements.len(), 4);
        assert!(result.quality_score > 0.0);
        assert!(result.octants_used > 0);
        assert!(result.avg_distance > 0.0);
    }

    #[test]
    fn test_golden_ratio_distribution() {
        let nodes = create_test_nodes();
        let shards: Vec<_> = (0..10).map(create_test_shard).collect();

        let result = distribute_across_octants(shards, &nodes).unwrap();

        // Verify golden ratio spacing in distances
        let distances: Vec<f64> = result
            .placements
            .iter()
            .map(|p| p.distance_from_origin)
            .collect();

        // Check that distances generally increase
        for i in 1..distances.len() {
            // Allow some variance due to node positions
            assert!(distances[i] >= distances[i - 1] * 0.8);
        }
    }

    #[test]
    fn test_empty_nodes_error() {
        let shards = vec![create_test_shard(0)];
        let result = distribute_across_octants(shards, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_shards_error() {
        let nodes = create_test_nodes();
        let result = distribute_across_octants(vec![], &nodes);
        assert!(result.is_err());
    }
}
