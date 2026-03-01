// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Cluster membership management: add, remove, shutdown, heartbeat recording.

use tracing::{debug, info};

use crate::matrix::coordinate::MatrixCoordinate;
use hypermesh_lib::BlockchainScope;

use super::{now_secs, ClusterError, ClusterEvent, ClusterManager, ClusterNode, NodeStatus};

impl ClusterManager {
    /// Add a node to the cluster in `Joining` status.
    ///
    /// The node's initial heartbeat is set to the current time.
    pub fn add_node(
        &mut self,
        node_id: &str,
        position: MatrixCoordinate,
        scope: BlockchainScope,
    ) -> Result<(), ClusterError> {
        if self.nodes.contains_key(node_id) {
            return Err(ClusterError::DuplicateNode(node_id.to_string()));
        }
        if self.nodes.len() >= self.config.max_nodes {
            return Err(ClusterError::ClusterFull {
                max: self.config.max_nodes,
            });
        }

        let now = now_secs();
        let node = ClusterNode {
            node_id: node_id.to_string(),
            position,
            scope,
            status: NodeStatus::Joining,
            last_heartbeat: now,
            health_score: 1.0,
            consecutive_failures: 0,
            failed_at: None,
        };

        info!(node_id = %node_id, position = %position, "Node added to cluster");
        self.nodes.insert(node_id.to_string(), node);
        self.pending_events.push(ClusterEvent::NodeJoined {
            node_id: node_id.to_string(),
            position,
        });
        Ok(())
    }

    /// Remove a node from the cluster, emitting a `NodeLeft` event.
    pub fn remove_node(&mut self, node_id: &str) -> Result<ClusterEvent, ClusterError> {
        if self.nodes.remove(node_id).is_none() {
            return Err(ClusterError::NodeNotFound(node_id.to_string()));
        }
        let event = ClusterEvent::NodeLeft {
            node_id: node_id.to_string(),
        };
        info!(node_id = %node_id, "Node removed from cluster");
        self.pending_events.push(event.clone());
        Ok(event)
    }

    /// Record a heartbeat for a node, resetting its failure counter.
    ///
    /// If the node was in `Joining` status, it transitions to `Healthy`.
    pub fn record_heartbeat(&mut self, node_id: &str) -> Result<(), ClusterError> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| ClusterError::NodeNotFound(node_id.to_string()))?;

        node.last_heartbeat = now_secs();
        node.consecutive_failures = 0;
        node.health_score = 1.0;

        if node.status == NodeStatus::Joining {
            node.status = NodeStatus::Healthy;
            debug!(node_id = %node_id, "Node transitioned Joining -> Healthy");
        } else if node.status == NodeStatus::Degraded {
            node.status = NodeStatus::Healthy;
            debug!(node_id = %node_id, "Node recovered from Degraded -> Healthy");
        }

        Ok(())
    }

    /// Initiate a graceful shutdown for a node.
    ///
    /// Transitions the node from Healthy/Degraded/Joining to Leaving,
    /// then removes it from the cluster.
    pub fn graceful_shutdown(&mut self, node_id: &str) -> Result<(), ClusterError> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| ClusterError::NodeNotFound(node_id.to_string()))?;

        let allowed = matches!(
            node.status,
            NodeStatus::Healthy | NodeStatus::Degraded | NodeStatus::Joining
        );
        if !allowed {
            return Err(ClusterError::InvalidTransition {
                node_id: node_id.to_string(),
                reason: format!(
                    "graceful shutdown requires Healthy/Degraded/Joining, got {}",
                    node.status
                ),
            });
        }

        node.status = NodeStatus::Leaving;
        debug!(node_id = %node_id, "Node transitioning to Leaving");

        self.nodes.remove(node_id);
        info!(node_id = %node_id, "Node gracefully shut down and removed");
        self.pending_events.push(ClusterEvent::NodeLeft {
            node_id: node_id.to_string(),
        });

        Ok(())
    }
}
