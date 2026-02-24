// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Cluster health monitoring: health checks, degraded marking, recovery.

use tracing::{info, warn};

use super::{
    ClusterManager, ClusterError, ClusterEvent, ClusterHealth,
    NodeStatus, now_secs,
};

impl ClusterManager {
    /// Run a health check across all nodes, updating statuses and scores.
    ///
    /// Returns a [`ClusterHealth`] summary. Nodes whose heartbeat age exceeds
    /// the configured interval are degraded; those exceeding the failure
    /// threshold are marked failed.
    pub fn check_health(&mut self) -> ClusterHealth {
        let now = now_secs();
        let interval = self.config.health_check_interval_secs;
        let threshold = self.config.failure_threshold;

        let mut healthy = 0usize;
        let mut degraded = 0usize;
        let mut failed = 0usize;
        let mut new_failures: Vec<(String, String)> = Vec::new();

        for node in self.nodes.values_mut() {
            if node.status == NodeStatus::Leaving || node.status == NodeStatus::Failed {
                if node.status == NodeStatus::Failed {
                    failed += 1;
                }
                continue;
            }

            let age = now.saturating_sub(node.last_heartbeat);

            if age <= interval {
                node.status = NodeStatus::Healthy;
                node.health_score = 1.0;
                node.consecutive_failures = 0;
                healthy += 1;
            } else if age <= interval.saturating_mul(2) {
                node.status = NodeStatus::Degraded;
                node.health_score = 0.5;
                node.consecutive_failures += 1;
                degraded += 1;
            } else {
                node.consecutive_failures += 1;
                if node.consecutive_failures >= threshold {
                    node.status = NodeStatus::Failed;
                    node.health_score = 0.0;
                    node.failed_at = Some(now);
                    new_failures.push((
                        node.node_id.clone(),
                        format!(
                            "exceeded failure threshold ({} consecutive misses)",
                            node.consecutive_failures
                        ),
                    ));
                    failed += 1;
                } else {
                    node.status = NodeStatus::Unreachable;
                    node.health_score = 0.2;
                    degraded += 1;
                }
            }
        }

        for (node_id, reason) in new_failures {
            warn!(node_id = %node_id, reason = %reason, "Node failed");
            self.pending_events.push(ClusterEvent::NodeFailed {
                node_id,
                reason,
            });
        }

        let total = self.nodes.len();
        let score = if total > 0 {
            healthy as f64 / total as f64
        } else {
            0.0
        };

        let event = ClusterEvent::HealthCheckCompleted {
            healthy,
            degraded,
            failed,
        };
        self.pending_events.push(event);

        ClusterHealth {
            total_nodes: total,
            healthy_nodes: healthy,
            degraded_nodes: degraded,
            failed_nodes: failed,
            cluster_health_score: score,
        }
    }

    /// Manually mark a node as degraded with a reason
    pub fn mark_node_degraded(
        &mut self,
        node_id: &str,
        reason: &str,
    ) -> Result<(), ClusterError> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| ClusterError::NodeNotFound(node_id.to_string()))?;

        node.status = NodeStatus::Degraded;
        node.health_score = 0.5;
        warn!(node_id = %node_id, reason = %reason, "Node manually marked degraded");
        Ok(())
    }

    /// Attempt to recover a Failed node.
    ///
    /// Recovery is only possible after [`ClusterConfig::recovery_timeout_secs`]
    /// have elapsed since the node entered Failed state. On success the node
    /// transitions to `Joining` and its heartbeat/failure counters are reset.
    pub fn attempt_recovery(
        &mut self,
        node_id: &str,
    ) -> Result<NodeStatus, ClusterError> {
        let now = now_secs();
        let timeout = self.config.recovery_timeout_secs;

        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| ClusterError::NodeNotFound(node_id.to_string()))?;

        if node.status != NodeStatus::Failed {
            return Err(ClusterError::InvalidTransition {
                node_id: node_id.to_string(),
                reason: format!("recovery requires Failed status, got {}", node.status),
            });
        }

        let failed_at = node.failed_at.unwrap_or(0);
        if now.saturating_sub(failed_at) < timeout {
            return Err(ClusterError::InvalidTransition {
                node_id: node_id.to_string(),
                reason: format!(
                    "recovery timeout not elapsed ({} of {} secs)",
                    now.saturating_sub(failed_at),
                    timeout
                ),
            });
        }

        node.status = NodeStatus::Joining;
        node.consecutive_failures = 0;
        node.health_score = 1.0;
        node.last_heartbeat = now;
        node.failed_at = None;

        info!(node_id = %node_id, "Node recovered -> Joining");
        self.pending_events.push(ClusterEvent::NodeRecovered {
            node_id: node_id.to_string(),
        });

        Ok(NodeStatus::Joining)
    }
}
