// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Shard Redistribution on PoS Grant/Revoke Events
//!
//! Handles dynamic redistribution when blockchain Asset records change
//! (PoS grants or revocations).

use crate::assets::core::{AssetError, AssetResult};
use crate::assets::pipeline::sharding::Shard;
use crate::distribution::{
    audit_trail::record_redistribution, matrix_optimizer::distribute_across_octants,
    pos_validator::StateAuthenticator, NodeInfo, ShardPlacement,
};
use serde::{Deserialize, Serialize};

/// Redistribution strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RedistributionStrategy {
    /// Move only affected shards
    Minimal,
    /// Rebalance all shards for optimal distribution
    FullRebalance,
    /// Move affected shards to nearest available nodes
    NearestAvailable,
}

/// Redistribution trigger event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RedistributionTrigger {
    /// PoS access revoked for node
    PosRevocation { node_id: String, reason: String },
    /// PoS access granted to new node
    PosGrant { node_id: String },
    /// Node capacity exceeded
    CapacityExceeded { node_id: String },
    /// Node failure detected
    NodeFailure { node_id: String },
}

/// Redistribution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedistributionResult {
    /// Asset identifier
    pub asset_id: String,
    /// Trigger event
    pub trigger: RedistributionTrigger,
    /// New shard placements
    pub new_placements: Vec<ShardPlacement>,
    /// Moved shards (old_node_id, new_node_id, shard_index)
    pub moved_shards: Vec<(String, String, usize)>,
    /// Redistribution strategy used
    pub strategy: RedistributionStrategy,
}

/// Handle PoS revocation event
///
/// When blockchain Asset record revokes storage access for a node,
/// redistribute affected shards to other eligible nodes.
///
/// # Arguments
///
/// * `asset_id` - Asset identifier
/// * `asset_privacy_level` - Asset privacy level
/// * `revoked_node_id` - Node that lost storage access
/// * `current_placements` - Current shard placements
/// * `shards` - Shards to potentially redistribute
/// * `all_nodes` - All network nodes
/// * `state_proof` - State proof validator
/// * `strategy` - Redistribution strategy
///
/// # Returns
///
/// Redistribution result with new placements
pub async fn handle_pos_revocation<C>(
    asset_id: &str,
    _asset_privacy_level: &str,
    revoked_node_id: &str,
    current_placements: &[ShardPlacement],
    shards: Vec<Shard>,
    all_nodes: &[NodeInfo],
    _state_proof: &C,
    strategy: RedistributionStrategy,
) -> AssetResult<RedistributionResult>
where
    C: StateAuthenticator,
{
    // Find affected shards
    let affected_shards: Vec<_> = current_placements
        .iter()
        .filter(|p| p.node_id == revoked_node_id)
        .collect();

    if affected_shards.is_empty() {
        // No shards on revoked node
        return Ok(RedistributionResult {
            asset_id: asset_id.to_string(),
            trigger: RedistributionTrigger::PosRevocation {
                node_id: revoked_node_id.to_string(),
                reason: "PoS access revoked".to_string(),
            },
            new_placements: current_placements.to_vec(),
            moved_shards: Vec::new(),
            strategy,
        });
    }

    // Get eligible nodes (excluding revoked node)
    let eligible_nodes: Vec<_> = all_nodes
        .iter()
        .filter(|n| n.node_id != revoked_node_id)
        .cloned()
        .collect();

    // Redistribute based on strategy
    let (new_placements, moved_shards) = match strategy {
        RedistributionStrategy::Minimal => redistribute_minimal(
            &affected_shards,
            current_placements,
            shards,
            &eligible_nodes,
        )?,
        RedistributionStrategy::FullRebalance => {
            redistribute_full_rebalance(shards, &eligible_nodes)?
        }
        RedistributionStrategy::NearestAvailable => redistribute_nearest(
            &affected_shards,
            current_placements,
            shards,
            &eligible_nodes,
        )?,
    };

    // Record redistribution in the bounded in-memory placement-audit trail
    record_redistribution(
        asset_id,
        &new_placements,
        &format!("PoS revocation for node {revoked_node_id}"),
    )
    .await?;

    Ok(RedistributionResult {
        asset_id: asset_id.to_string(),
        trigger: RedistributionTrigger::PosRevocation {
            node_id: revoked_node_id.to_string(),
            reason: "PoS access revoked".to_string(),
        },
        new_placements,
        moved_shards,
        strategy,
    })
}

/// Redistribute shards (public interface)
pub async fn redistribute_shards<C>(
    asset_id: &str,
    asset_privacy_level: &str,
    trigger: RedistributionTrigger,
    current_placements: &[ShardPlacement],
    shards: Vec<Shard>,
    all_nodes: &[NodeInfo],
    state_proof: &C,
    strategy: RedistributionStrategy,
) -> AssetResult<RedistributionResult>
where
    C: StateAuthenticator,
{
    match &trigger {
        RedistributionTrigger::PosRevocation { node_id, .. } => {
            handle_pos_revocation(
                asset_id,
                asset_privacy_level,
                node_id,
                current_placements,
                shards,
                all_nodes,
                state_proof,
                strategy,
            )
            .await
        }
        RedistributionTrigger::NodeFailure { node_id } => {
            // Similar to revocation
            handle_pos_revocation(
                asset_id,
                asset_privacy_level,
                node_id,
                current_placements,
                shards,
                all_nodes,
                state_proof,
                strategy,
            )
            .await
        }
        _ => Err(AssetError::ValidationError {
            message: format!("Unsupported trigger: {trigger:?}"),
        }),
    }
}

/// Minimal redistribution - move only affected shards
#[allow(clippy::type_complexity)]
fn redistribute_minimal(
    affected_shards: &[&ShardPlacement],
    current_placements: &[ShardPlacement],
    shards: Vec<Shard>,
    eligible_nodes: &[NodeInfo],
) -> AssetResult<(Vec<ShardPlacement>, Vec<(String, String, usize)>)> {
    let mut new_placements = current_placements.to_vec();
    let mut moved = Vec::new();

    // Get affected shard indices
    let affected_indices: Vec<usize> = affected_shards.iter().map(|p| p.shard_index).collect();

    // Select affected shards
    let affected_shard_data: Vec<_> = shards
        .into_iter()
        .enumerate()
        .filter(|(i, _)| affected_indices.contains(i))
        .map(|(_, s)| s)
        .collect();

    // Redistribute only affected shards
    let distribution = distribute_across_octants(&affected_shard_data, eligible_nodes)?;

    // Replace affected placements
    for affected in affected_shards {
        // Find new placement for this shard
        if let Some(new_placement) = distribution
            .placements
            .iter()
            .find(|p| p.shard_index == affected.shard_index)
        {
            // Record move
            moved.push((
                affected.node_id.clone(),
                new_placement.node_id.clone(),
                affected.shard_index,
            ));

            // Replace in placements
            if let Some(idx) = new_placements
                .iter()
                .position(|p| p.shard_index == affected.shard_index)
            {
                new_placements[idx] = new_placement.clone();
            }
        }
    }

    Ok((new_placements, moved))
}

/// Full rebalance - redistribute all shards
#[allow(clippy::type_complexity)]
fn redistribute_full_rebalance(
    shards: Vec<Shard>,
    eligible_nodes: &[NodeInfo],
) -> AssetResult<(Vec<ShardPlacement>, Vec<(String, String, usize)>)> {
    let distribution = distribute_across_octants(&shards, eligible_nodes)?;

    // All shards are potentially moved
    let moved = Vec::new(); // Would compare with previous placements

    Ok((distribution.placements, moved))
}

/// Nearest available redistribution
#[allow(clippy::type_complexity)]
fn redistribute_nearest(
    affected_shards: &[&ShardPlacement],
    current_placements: &[ShardPlacement],
    _shards: Vec<Shard>,
    eligible_nodes: &[NodeInfo],
) -> AssetResult<(Vec<ShardPlacement>, Vec<(String, String, usize)>)> {
    let mut new_placements = current_placements.to_vec();
    let mut moved = Vec::new();

    for affected in affected_shards {
        // Find nearest eligible node
        let nearest_node = eligible_nodes
            .iter()
            .min_by(|a, b| {
                let dist_a = affected.position.euclidean_distance(&a.position);
                let dist_b = affected.position.euclidean_distance(&b.position);
                dist_a
                    .partial_cmp(&dist_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| AssetError::ValidationError {
                message: "No eligible nodes available".to_string(),
            })?;

        // Create new placement
        let new_placement = ShardPlacement {
            shard_index: affected.shard_index,
            position: nearest_node.position,
            node_id: nearest_node.node_id.clone(),
            octant: affected.octant,
            distance_from_origin: affected
                .position
                .euclidean_distance(&crate::matrix::coordinate::MatrixCoordinate::origin()),
        };

        // Record move
        moved.push((
            affected.node_id.clone(),
            new_placement.node_id.clone(),
            affected.shard_index,
        ));

        // Replace in placements
        if let Some(idx) = new_placements
            .iter()
            .position(|p| p.shard_index == affected.shard_index)
        {
            new_placements[idx] = new_placement;
        }
    }

    Ok((new_placements, moved))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::pipeline::sharding::{Shard, ShardMetadata};
    use crate::distribution::pos_validator::MockStateAuthenticator;
    use crate::matrix::coordinate::MatrixCoordinate;

    fn create_test_shard(index: usize) -> Shard {
        Shard {
            data: vec![0u8; 1024],
            metadata: ShardMetadata {
                index,
                is_parity: false,
                size: 1024,
                original_size: 1024,
                hash: format!("hash-{index}"),
            },
        }
    }

    fn create_test_nodes() -> Vec<NodeInfo> {
        vec![
            NodeInfo::new(
                "node1".to_string(),
                MatrixCoordinate::new(10, 10, 10).expect("test: valid coordinate"),
                "PrivateNetwork".to_string(),
                1_000_000_000,
                "network1".to_string(),
            ),
            NodeInfo::new(
                "node2".to_string(),
                MatrixCoordinate::new(20, 20, 20).expect("test: valid coordinate"),
                "PrivateNetwork".to_string(),
                1_000_000_000,
                "network1".to_string(),
            ),
            NodeInfo::new(
                "node3".to_string(),
                MatrixCoordinate::new(30, 30, 30).expect("test: valid coordinate"),
                "PrivateNetwork".to_string(),
                1_000_000_000,
                "network1".to_string(),
            ),
        ]
    }

    #[tokio::test]
    async fn test_handle_pos_revocation() {
        let nodes = create_test_nodes();
        let shards = vec![create_test_shard(0), create_test_shard(1)];

        let current_placements = vec![
            ShardPlacement {
                shard_index: 0,
                position: nodes[0].position,
                node_id: "node1".to_string(),
                octant: 0,
                distance_from_origin: 17.32,
            },
            ShardPlacement {
                shard_index: 1,
                position: nodes[1].position,
                node_id: "node2".to_string(),
                octant: 0,
                distance_from_origin: 34.64,
            },
        ];

        let state_proof = MockStateAuthenticator::new(true);

        let result = handle_pos_revocation(
            "test-asset",
            "PrivateNetwork",
            "node1",
            &current_placements,
            shards,
            &nodes,
            &state_proof,
            RedistributionStrategy::Minimal,
        )
        .await
        .expect("test: expected success");

        assert_eq!(result.moved_shards.len(), 1);
        assert_eq!(result.moved_shards[0].0, "node1");
    }

    #[tokio::test]
    async fn test_no_shards_affected() {
        let nodes = create_test_nodes();
        let shards = vec![create_test_shard(0)];

        let current_placements = vec![ShardPlacement {
            shard_index: 0,
            position: nodes[0].position,
            node_id: "node1".to_string(),
            octant: 0,
            distance_from_origin: 17.32,
        }];

        let state_proof = MockStateAuthenticator::new(true);

        let result = handle_pos_revocation(
            "test-asset",
            "PrivateNetwork",
            "node2", // Different node
            &current_placements,
            shards,
            &nodes,
            &state_proof,
            RedistributionStrategy::Minimal,
        )
        .await
        .expect("test: expected success");

        assert_eq!(result.moved_shards.len(), 0);
    }

    #[tokio::test]
    async fn test_redistribution_strategies() {
        let nodes = create_test_nodes();
        let shards = vec![create_test_shard(0), create_test_shard(1)];

        let current_placements = vec![
            ShardPlacement {
                shard_index: 0,
                position: nodes[0].position,
                node_id: "node1".to_string(),
                octant: 0,
                distance_from_origin: 17.32,
            },
            ShardPlacement {
                shard_index: 1,
                position: nodes[1].position,
                node_id: "node2".to_string(),
                octant: 0,
                distance_from_origin: 34.64,
            },
        ];

        let state_proof = MockStateAuthenticator::new(true);

        // Test each strategy
        for strategy in [
            RedistributionStrategy::Minimal,
            RedistributionStrategy::NearestAvailable,
        ] {
            let result = handle_pos_revocation(
                "test-asset",
                "PrivateNetwork",
                "node1",
                &current_placements,
                shards.clone(),
                &nodes,
                &state_proof,
                strategy,
            )
            .await;

            assert!(result.is_ok());
        }
    }
}
