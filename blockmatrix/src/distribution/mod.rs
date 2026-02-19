// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Matrix-Aware Shard Distribution with PoS-driven Architecture
//!
//! CRITICAL ARCHITECTURAL PRINCIPLE:
//! "Let Proof of State decide, not hardcoded logic"
//!
//! All permission rules live in blockchain Asset records. Distribution queries
//! consensus validation for eligibility, then applies matrix optimization within
//! approved node pool.
//!
//! # Architecture
//!
//! 1. **PoS Validation** - Query consensus for node eligibility
//! 2. **Matrix Optimization** - Apply 8-octant distribution within eligible pool
//! 3. **Audit Trail** - Record placement on blockchain
//! 4. **Redistribution** - Handle PoS grant/revoke events
//!
//! # Example
//!
//! ```no_run
//! use blockmatrix::distribution::{distribute_shards_pos_aware, NodeInfo};
//! use blockmatrix::assets::core::{Asset, ConsensusProof};
//! use blockmatrix::assets::pipeline::sharding::Shard;
//! use blockmatrix::matrix::coordinate::MatrixCoordinate;
//! use blockmatrix::consensus::validation::DefaultConsensusValidator;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create test data
//! let asset = create_test_asset();
//! let shards = vec![/* shards */];
//! let nodes = vec![/* node info */];
//! let matrix = MatrixFoundation::new();
//! let consensus = DefaultConsensusValidator::new();
//!
//! // Distribute with PoS validation
//! let placements = distribute_shards_pos_aware(
//!     shards,
//!     &asset,
//!     &nodes,
//!     &matrix,
//!     &consensus
//! ).await?;
//! # Ok(())
//! # }
//! ```

use crate::assets::core::{AssetError, AssetResult};
use crate::assets::pipeline::sharding::Shard;
use crate::matrix::coordinate::MatrixCoordinate;
use serde::{Deserialize, Serialize};

pub mod pos_validator;
pub mod matrix_optimizer;
pub mod audit_trail;
pub mod redistribution;

// Re-exports
pub use pos_validator::{
    StorageAccessValidation, get_eligible_nodes, validate_node_eligibility
};
pub use matrix_optimizer::{
    distribute_across_octants, calculate_octant_placements, OctantDistribution
};
pub use audit_trail::{
    record_shard_placement_on_chain, AuditRecord, PlacementEvent
};
pub use redistribution::{
    handle_pos_revocation, redistribute_shards, RedistributionStrategy
};

/// Node information for distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Node identifier
    pub node_id: String,
    /// Matrix position
    pub position: MatrixCoordinate,
    /// Node privacy level
    pub privacy_level: String,
    /// Available storage capacity (bytes)
    pub available_storage: u64,
    /// Network identifier
    pub network_id: String,
}

impl NodeInfo {
    pub fn new(
        node_id: String,
        position: MatrixCoordinate,
        privacy_level: String,
        available_storage: u64,
        network_id: String,
    ) -> Self {
        Self {
            node_id,
            position,
            privacy_level,
            available_storage,
            network_id,
        }
    }
}

/// Shard placement with matrix position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardPlacement {
    /// Shard being placed
    pub shard_index: usize,
    /// Matrix coordinate for placement
    pub position: MatrixCoordinate,
    /// Node hosting this shard
    pub node_id: String,
    /// Octant assignment (0-7)
    pub octant: u8,
    /// Distance from origin
    pub distance_from_origin: f64,
}

/// Distribution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionResult {
    /// Asset identifier
    pub asset_id: String,
    /// Shard placements
    pub placements: Vec<ShardPlacement>,
    /// Distribution quality score (0-100)
    pub quality_score: f64,
    /// Number of octants used
    pub octants_used: usize,
    /// Average inter-shard distance
    pub avg_distance: f64,
}

/// Core distribution function with PoS-aware architecture
///
/// # Architecture
///
/// 1. **PoS Validation** - Query consensus for eligible nodes
/// 2. **Matrix Optimization** - Distribute across 8 octants using golden ratio
/// 3. **Blockchain Recording** - Audit trail of placement decisions
///
/// # Arguments
///
/// * `shards` - Shards to distribute
/// * `asset` - Asset being distributed (contains privacy rules)
/// * `all_nodes` - All available nodes in the network
/// * `consensus` - Consensus validator for PoS queries
///
/// # Returns
///
/// List of shard placements with matrix coordinates
pub async fn distribute_shards_pos_aware<C>(
    shards: Vec<Shard>,
    asset_id: &str,
    asset_privacy_level: &str,
    all_nodes: &[NodeInfo],
    consensus: &C,
) -> AssetResult<DistributionResult>
where
    C: pos_validator::ConsensusValidator,
{
    // Step 1: Query PoS validation for eligible nodes
    let eligible_nodes = get_eligible_nodes(
        asset_id,
        asset_privacy_level,
        &shards,
        all_nodes,
        consensus
    ).await?;

    if eligible_nodes.is_empty() {
        return Err(AssetError::ValidationError {
            message: "No eligible nodes found for shard distribution".to_string(),
        });
    }

    // Step 2: Apply matrix-aware optimization WITHIN eligible pool
    let octant_distribution = distribute_across_octants(
        shards,
        &eligible_nodes,
    )?;

    // Step 3: Blockchain records placement (audit trail)
    record_shard_placement_on_chain(
        asset_id,
        &octant_distribution.placements,
    ).await?;

    Ok(octant_distribution)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::validation::DefaultConsensusValidator;
    use hypermesh_lib::PrivacyMode;

    #[tokio::test]
    async fn test_distribution_with_eligible_nodes() {
        // Create test nodes
        let nodes = vec![
            NodeInfo::new(
                "node1".to_string(),
                MatrixCoordinate::new(10, 10, 10).unwrap(),
                "PrivateNetwork".to_string(),
                1_000_000_000,
                "network1".to_string(),
            ),
            NodeInfo::new(
                "node2".to_string(),
                MatrixCoordinate::new(20, 20, 20).unwrap(),
                "PrivateNetwork".to_string(),
                1_000_000_000,
                "network1".to_string(),
            ),
        ];

        // Create test shards
        let shards = vec![
            create_test_shard(0),
            create_test_shard(1),
        ];

        let consensus = DefaultConsensusValidator::for_testing();

        let result = distribute_shards_pos_aware(
            shards,
            "test-asset",
            "PrivateNetwork",
            &nodes,
            &consensus,
        )
        .await;

        assert!(result.is_ok());
        let distribution = result.unwrap();
        assert_eq!(distribution.placements.len(), 2);
    }

    fn create_test_shard(index: usize) -> Shard {
        use crate::assets::pipeline::sharding::{Shard, ShardMetadata};
        Shard {
            data: vec![0u8; 1024],
            metadata: ShardMetadata {
                index,
                is_parity: false,
                size: 1024,
                original_size: 1024,
                hash: "test-hash".to_string(),
            },
        }
    }
}
