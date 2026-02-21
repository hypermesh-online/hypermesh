// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Types for the dynamic shard rebalancing system.

use crate::matrix::coordinate::MatrixCoordinate;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Unique shard identifier (content hash string from ShardMetadata).
pub type ShardId = String;

/// Configuration for rebalance behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceConfig {
    /// Minimum replica count per shard (rebalance if below).
    pub min_replicas: usize,
    /// Maximum replica count per shard (remove excess).
    pub max_replicas: usize,
    /// Imbalance score threshold that triggers rebalancing (0.0 - 1.0).
    pub rebalance_threshold: f64,
    /// Cooldown between rebalance operations in seconds.
    pub cooldown_secs: u64,
}

impl Default for RebalanceConfig {
    fn default() -> Self {
        Self {
            min_replicas: 2,
            max_replicas: 5,
            rebalance_threshold: 0.3,
            cooldown_secs: 60,
        }
    }
}

/// Action to take during rebalancing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RebalanceAction {
    /// Move a shard from one node to another.
    MoveShard {
        shard_id: ShardId,
        from_node: String,
        to_node: String,
    },
    /// Create a new replica of a shard on a node.
    ReplicateShard {
        shard_id: ShardId,
        to_node: String,
    },
    /// Remove an excess replica from a node.
    RemoveReplica {
        shard_id: ShardId,
        from_node: String,
    },
}

/// Report produced by `check_balance`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceReport {
    /// Overall imbalance score (0.0 = perfect, 1.0 = worst).
    pub imbalance_score: f64,
    /// Nodes with more shards than average + threshold.
    pub overloaded_nodes: Vec<String>,
    /// Nodes with fewer shards than average - threshold.
    pub underloaded_nodes: Vec<String>,
    /// Shards with fewer replicas than `min_replicas`.
    pub orphaned_shards: Vec<ShardId>,
}

/// Result of executing rebalance actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceResult {
    /// Number of actions successfully executed.
    pub actions_executed: usize,
    /// Number of actions that failed.
    pub actions_failed: usize,
    /// Duration of the rebalance operation.
    pub duration: Duration,
}

/// Tracked placement of a single shard on a single node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardRecord {
    /// Shard identifier.
    pub shard_id: ShardId,
    /// Node hosting this replica.
    pub node_id: String,
    /// Matrix position of the hosting node.
    pub position: MatrixCoordinate,
}
