// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Distribution stage data types (placement result shapes).
//!
//! P4 note: this file no longer computes placements. The synthetic
//! golden-ratio geometry engine (`MatrixDistributor`) was deleted — it fed
//! the live store path with matrix positions unrelated to any real node
//! (`register_node` was never called in production, so it always fell through
//! to fabricated sphere-packing coordinates). Placement now belongs to the
//! store path, which has the live PoS-eligible peer set and their proximity
//! coordinates: see [`crate::network::placement`] →
//! [`crate::distribution::distribute_shards_pos_aware`] (the single placement
//! authority). What remains here are the pure data types that describe a
//! placement result and the pipeline's (now always-empty) `DistributedAsset`
//! carrier on [`crate::assets::pipeline::orchestrator::ProcessedAsset`].

use crate::matrix::coordinate::MatrixCoordinate;
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
