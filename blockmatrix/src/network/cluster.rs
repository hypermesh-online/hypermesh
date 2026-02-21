// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Cluster health monitoring and node lifecycle management
//!
//! Provides [`ClusterManager`] for tracking cluster membership, monitoring
//! node health via heartbeats, and managing node lifecycle transitions
//! (Joining -> Healthy -> Degraded -> Failed -> recovery).
//!
//! Health checks use heartbeat age relative to configurable intervals:
//! - Within 1x interval: Healthy
//! - Within 2x interval: Degraded
//! - Exceeding failure threshold consecutive misses: Failed
//!
//! Failed nodes may attempt recovery after a configurable timeout.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;
use tracing::{debug, info, warn};

use crate::matrix::coordinate::MatrixCoordinate;
use hypermesh_lib::BlockchainScope;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors returned by [`ClusterManager`] operations
#[derive(Debug, Error)]
pub enum ClusterError {
    /// The referenced node was not found in the cluster
    #[error("node not found: {0}")]
    NodeNotFound(String),

    /// The cluster has reached its maximum node capacity
    #[error("cluster full: max {max} nodes")]
    ClusterFull { max: usize },

    /// A node with this ID already exists
    #[error("duplicate node: {0}")]
    DuplicateNode(String),

    /// The operation is invalid for the node's current status
    #[error("invalid status transition for node {node_id}: {reason}")]
    InvalidTransition { node_id: String, reason: String },
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration parameters for cluster health monitoring
#[derive(Debug, Clone)]
pub struct ClusterConfig {
    /// Seconds between expected heartbeats
    pub health_check_interval_secs: u64,
    /// Consecutive missed checks before a node is marked Failed
    pub failure_threshold: u32,
    /// Seconds a Failed node must wait before attempting recovery
    pub recovery_timeout_secs: u64,
    /// Maximum number of nodes the cluster will accept
    pub max_nodes: usize,
    /// Minimum healthy nodes for the cluster to be considered operational
    pub min_healthy_nodes: usize,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            health_check_interval_secs: 10,
            failure_threshold: 3,
            recovery_timeout_secs: 60,
            max_nodes: 256,
            min_healthy_nodes: 1,
        }
    }
}

// ---------------------------------------------------------------------------
// Node types
// ---------------------------------------------------------------------------

/// Operational status of a cluster node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeStatus {
    /// Node is in the process of joining the cluster
    Joining,
    /// Node is healthy and responsive
    Healthy,
    /// Node has a stale heartbeat but has not exceeded the failure threshold
    Degraded,
    /// Node has not responded and cannot be reached
    Unreachable,
    /// Node is gracefully leaving the cluster
    Leaving,
    /// Node has exceeded the failure threshold
    Failed,
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Joining => write!(f, "Joining"),
            Self::Healthy => write!(f, "Healthy"),
            Self::Degraded => write!(f, "Degraded"),
            Self::Unreachable => write!(f, "Unreachable"),
            Self::Leaving => write!(f, "Leaving"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

/// A node tracked by the cluster manager
#[derive(Debug, Clone)]
pub struct ClusterNode {
    /// Unique identifier for this node
    pub node_id: String,
    /// Position in the Block-MATRIX topology
    pub position: MatrixCoordinate,
    /// Blockchain scope this node participates in
    pub scope: BlockchainScope,
    /// Current operational status
    pub status: NodeStatus,
    /// Unix timestamp (seconds) of the last heartbeat
    pub last_heartbeat: u64,
    /// Health score from 0.0 (dead) to 1.0 (perfect)
    pub health_score: f64,
    /// Number of consecutive missed health checks
    pub consecutive_failures: u32,
    /// Unix timestamp (seconds) when the node entered Failed state
    pub failed_at: Option<u64>,
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Events emitted by cluster state transitions
#[derive(Debug, Clone, PartialEq)]
pub enum ClusterEvent {
    /// A new node joined the cluster
    NodeJoined {
        node_id: String,
        position: MatrixCoordinate,
    },
    /// A node left the cluster (graceful or removal)
    NodeLeft { node_id: String },
    /// A node was marked as Failed
    NodeFailed { node_id: String, reason: String },
    /// A previously failed node re-entered Joining state
    NodeRecovered { node_id: String },
    /// A health check cycle completed
    HealthCheckCompleted {
        healthy: usize,
        degraded: usize,
        failed: usize,
    },
}

// ---------------------------------------------------------------------------
// Cluster health summary
// ---------------------------------------------------------------------------

/// Aggregate health snapshot of the cluster
#[derive(Debug, Clone)]
pub struct ClusterHealth {
    /// Total number of tracked nodes
    pub total_nodes: usize,
    /// Nodes in Healthy status
    pub healthy_nodes: usize,
    /// Nodes in Degraded status
    pub degraded_nodes: usize,
    /// Nodes in Failed status
    pub failed_nodes: usize,
    /// Ratio of healthy nodes to total (0.0 to 1.0)
    pub cluster_health_score: f64,
}

// ---------------------------------------------------------------------------
// ClusterManager
// ---------------------------------------------------------------------------

/// Manages cluster membership, health monitoring, and node lifecycle.
///
/// Nodes are identified by string IDs and tracked through status transitions
/// driven by heartbeat recording and periodic health checks.
pub struct ClusterManager {
    /// Tracked nodes keyed by node_id
    nodes: HashMap<String, ClusterNode>,
    /// Configuration
    config: ClusterConfig,
    /// Pending events waiting to be drained
    pending_events: Vec<ClusterEvent>,
}

impl ClusterManager {
    /// Create a new cluster manager with the given configuration
    pub fn new(config: ClusterConfig) -> Self {
        info!(
            max_nodes = config.max_nodes,
            interval_secs = config.health_check_interval_secs,
            failure_threshold = config.failure_threshold,
            "ClusterManager created"
        );
        Self {
            nodes: HashMap::new(),
            config,
            pending_events: Vec::new(),
        }
    }

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
            // Skip nodes that are leaving or already failed
            if node.status == NodeStatus::Leaving || node.status == NodeStatus::Failed {
                if node.status == NodeStatus::Failed {
                    failed += 1;
                }
                continue;
            }

            let age = now.saturating_sub(node.last_heartbeat);

            if age <= interval {
                // Fresh heartbeat
                node.status = NodeStatus::Healthy;
                node.health_score = 1.0;
                node.consecutive_failures = 0;
                healthy += 1;
            } else if age <= interval.saturating_mul(2) {
                // Stale but within tolerance
                node.status = NodeStatus::Degraded;
                node.health_score = 0.5;
                node.consecutive_failures += 1;
                degraded += 1;
            } else {
                // Missed heartbeat
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

        // Emit failure events
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

    /// Get a reference to a node by ID
    pub fn get_node_status(&self, node_id: &str) -> Option<&ClusterNode> {
        self.nodes.get(node_id)
    }

    /// List all tracked nodes
    pub fn list_nodes(&self) -> Vec<&ClusterNode> {
        self.nodes.values().collect()
    }

    /// List nodes matching a given status
    pub fn list_by_status(&self, status: NodeStatus) -> Vec<&ClusterNode> {
        self.nodes
            .values()
            .filter(|n| n.status == status)
            .collect()
    }

    /// Drain and return all pending events
    pub fn get_events(&mut self) -> Vec<ClusterEvent> {
        std::mem::take(&mut self.pending_events)
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

    /// Initiate a graceful shutdown for a node.
    ///
    /// Transitions the node from Healthy/Degraded/Joining to Leaving,
    /// then removes it from the cluster.
    pub fn graceful_shutdown(
        &mut self,
        node_id: &str,
    ) -> Result<(), ClusterError> {
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

        // Remove after marking Leaving
        self.nodes.remove(node_id);
        info!(node_id = %node_id, "Node gracefully shut down and removed");
        self.pending_events.push(ClusterEvent::NodeLeft {
            node_id: node_id.to_string(),
        });

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Current time as unix seconds. Returns 0 on clock error to avoid panics.
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ClusterConfig {
        ClusterConfig {
            health_check_interval_secs: 10,
            failure_threshold: 3,
            recovery_timeout_secs: 5,
            max_nodes: 4,
            min_healthy_nodes: 1,
        }
    }

    fn coord(x: i64, y: i64, z: i64) -> MatrixCoordinate {
        MatrixCoordinate::new(x, y, z).expect("test: valid coordinate")
    }

    // 1. Add node to cluster
    #[test]
    fn test_add_node() {
        let mut mgr = ClusterManager::new(test_config());
        mgr.add_node("n1", coord(0, 0, 0), BlockchainScope::Device)
            .expect("test: add node");

        assert_eq!(mgr.list_nodes().len(), 1);
        let node = mgr.get_node_status("n1").expect("test: node exists");
        assert_eq!(node.status, NodeStatus::Joining);
        assert!((node.health_score - 1.0).abs() < f64::EPSILON);
    }

    // 2. Remove node generates event
    #[test]
    fn test_remove_node_generates_event() {
        let mut mgr = ClusterManager::new(test_config());
        mgr.add_node("n1", coord(1, 2, 3), BlockchainScope::Device)
            .expect("test: add");

        // Drain add event
        let _ = mgr.get_events();

        let event = mgr.remove_node("n1").expect("test: remove");
        assert_eq!(
            event,
            ClusterEvent::NodeLeft {
                node_id: "n1".to_string()
            }
        );
        assert!(mgr.get_node_status("n1").is_none());
    }

    // 3. Record heartbeat updates timestamp
    #[test]
    fn test_record_heartbeat_updates_timestamp() {
        let mut mgr = ClusterManager::new(test_config());
        mgr.add_node("n1", coord(0, 0, 0), BlockchainScope::Device)
            .expect("test: add");

        let before = mgr
            .get_node_status("n1")
            .expect("test: exists")
            .last_heartbeat;

        // Recording a heartbeat should set it to now (>= before)
        mgr.record_heartbeat("n1").expect("test: heartbeat");

        let after = mgr
            .get_node_status("n1")
            .expect("test: exists")
            .last_heartbeat;
        assert!(after >= before);

        // Joining -> Healthy on first heartbeat
        assert_eq!(
            mgr.get_node_status("n1").expect("test: exists").status,
            NodeStatus::Healthy
        );
    }

    // 4. Health check marks healthy nodes
    #[test]
    fn test_health_check_marks_healthy() {
        let mut mgr = ClusterManager::new(test_config());
        mgr.add_node("n1", coord(0, 0, 0), BlockchainScope::Device)
            .expect("test: add");
        mgr.record_heartbeat("n1").expect("test: heartbeat");

        let health = mgr.check_health();
        assert_eq!(health.healthy_nodes, 1);
        assert_eq!(health.degraded_nodes, 0);
        assert_eq!(health.failed_nodes, 0);
        assert!((health.cluster_health_score - 1.0).abs() < f64::EPSILON);
    }

    // 5. Health check marks degraded nodes (stale heartbeat)
    #[test]
    fn test_health_check_marks_degraded() {
        let mut mgr = ClusterManager::new(test_config());
        mgr.add_node("n1", coord(0, 0, 0), BlockchainScope::Device)
            .expect("test: add");

        // Manually set heartbeat to 15 seconds ago (> interval=10, <= 2*interval=20)
        let node = mgr.nodes.get_mut("n1").expect("test: exists");
        node.status = NodeStatus::Healthy;
        node.last_heartbeat = now_secs().saturating_sub(15);

        let health = mgr.check_health();
        assert_eq!(health.degraded_nodes, 1);
        assert_eq!(health.healthy_nodes, 0);

        let node = mgr.get_node_status("n1").expect("test: exists");
        assert_eq!(node.status, NodeStatus::Degraded);
        assert!((node.health_score - 0.5).abs() < f64::EPSILON);
    }

    // 6. Health check marks failed nodes (threshold exceeded)
    #[test]
    fn test_health_check_marks_failed() {
        let mut mgr = ClusterManager::new(test_config());
        mgr.add_node("n1", coord(0, 0, 0), BlockchainScope::Device)
            .expect("test: add");

        // Set heartbeat far in the past and pre-load consecutive_failures
        let node = mgr.nodes.get_mut("n1").expect("test: exists");
        node.status = NodeStatus::Healthy;
        node.last_heartbeat = now_secs().saturating_sub(100);
        node.consecutive_failures = 2; // Next check will be #3, hitting threshold

        let health = mgr.check_health();
        assert_eq!(health.failed_nodes, 1);

        let node = mgr.get_node_status("n1").expect("test: exists");
        assert_eq!(node.status, NodeStatus::Failed);
        assert!((node.health_score).abs() < f64::EPSILON);
    }

    // 7. Cluster health score calculation
    #[test]
    fn test_cluster_health_score() {
        let mut mgr = ClusterManager::new(test_config());

        // Add 4 nodes, make 2 healthy and 2 stale
        for i in 0..4 {
            mgr.add_node(
                &format!("n{}", i),
                coord(i, 0, 0),
                BlockchainScope::Device,
            )
            .expect("test: add");
        }

        // Make n0 and n1 healthy
        mgr.record_heartbeat("n0").expect("test: hb");
        mgr.record_heartbeat("n1").expect("test: hb");

        // Make n2 and n3 degraded (stale heartbeat)
        let stale = now_secs().saturating_sub(15);
        for id in &["n2", "n3"] {
            let node = mgr.nodes.get_mut(*id).expect("test: exists");
            node.status = NodeStatus::Healthy;
            node.last_heartbeat = stale;
        }

        let health = mgr.check_health();
        assert_eq!(health.total_nodes, 4);
        assert_eq!(health.healthy_nodes, 2);
        // score = 2/4 = 0.5
        assert!((health.cluster_health_score - 0.5).abs() < f64::EPSILON);
    }

    // 8. Node recovery after timeout
    #[test]
    fn test_node_recovery_after_timeout() {
        let mut mgr = ClusterManager::new(test_config());
        mgr.add_node("n1", coord(0, 0, 0), BlockchainScope::Device)
            .expect("test: add");

        // Force into Failed state with failed_at in the past
        let node = mgr.nodes.get_mut("n1").expect("test: exists");
        node.status = NodeStatus::Failed;
        node.failed_at = Some(now_secs().saturating_sub(10)); // 10s ago, timeout is 5s

        let status = mgr.attempt_recovery("n1").expect("test: recovery");
        assert_eq!(status, NodeStatus::Joining);

        let node = mgr.get_node_status("n1").expect("test: exists");
        assert_eq!(node.consecutive_failures, 0);
        assert!((node.health_score - 1.0).abs() < f64::EPSILON);
    }

    // 8b. Recovery fails if timeout not elapsed
    #[test]
    fn test_recovery_fails_before_timeout() {
        let mut mgr = ClusterManager::new(test_config());
        mgr.add_node("n1", coord(0, 0, 0), BlockchainScope::Device)
            .expect("test: add");

        let node = mgr.nodes.get_mut("n1").expect("test: exists");
        node.status = NodeStatus::Failed;
        node.failed_at = Some(now_secs()); // just failed

        let result = mgr.attempt_recovery("n1");
        assert!(result.is_err());
    }

    // 9. Graceful shutdown lifecycle
    #[test]
    fn test_graceful_shutdown() {
        let mut mgr = ClusterManager::new(test_config());
        mgr.add_node("n1", coord(0, 0, 0), BlockchainScope::Device)
            .expect("test: add");
        mgr.record_heartbeat("n1").expect("test: hb");

        // Drain prior events
        let _ = mgr.get_events();

        mgr.graceful_shutdown("n1").expect("test: shutdown");

        // Node should be removed
        assert!(mgr.get_node_status("n1").is_none());
        assert_eq!(mgr.list_nodes().len(), 0);

        // Should have a NodeLeft event
        let events = mgr.get_events();
        assert!(events.iter().any(|e| matches!(
            e,
            ClusterEvent::NodeLeft { node_id } if node_id == "n1"
        )));
    }

    // 10. Max nodes enforcement
    #[test]
    fn test_max_nodes_enforcement() {
        let mut mgr = ClusterManager::new(test_config()); // max_nodes = 4

        for i in 0..4 {
            mgr.add_node(
                &format!("n{}", i),
                coord(i, 0, 0),
                BlockchainScope::Device,
            )
            .expect("test: add");
        }

        // 5th node should fail
        let result = mgr.add_node("n4", coord(4, 0, 0), BlockchainScope::Device);
        assert!(matches!(result, Err(ClusterError::ClusterFull { max: 4 })));
    }

    // 11. List by status filtering
    #[test]
    fn test_list_by_status() {
        let mut mgr = ClusterManager::new(test_config());
        mgr.add_node("n1", coord(0, 0, 0), BlockchainScope::Device)
            .expect("test: add");
        mgr.add_node("n2", coord(1, 0, 0), BlockchainScope::Network)
            .expect("test: add");
        mgr.add_node("n3", coord(2, 0, 0), BlockchainScope::Device)
            .expect("test: add");

        // n1 -> Healthy
        mgr.record_heartbeat("n1").expect("test: hb");
        // n2 stays Joining
        // n3 -> manually degraded
        mgr.mark_node_degraded("n3", "test reason")
            .expect("test: degrade");

        let joining = mgr.list_by_status(NodeStatus::Joining);
        assert_eq!(joining.len(), 1);
        assert_eq!(joining[0].node_id, "n2");

        let healthy = mgr.list_by_status(NodeStatus::Healthy);
        assert_eq!(healthy.len(), 1);
        assert_eq!(healthy[0].node_id, "n1");

        let degraded = mgr.list_by_status(NodeStatus::Degraded);
        assert_eq!(degraded.len(), 1);
        assert_eq!(degraded[0].node_id, "n3");
    }

    // 12. Event drain and accumulation
    #[test]
    fn test_event_drain_and_accumulation() {
        let mut mgr = ClusterManager::new(test_config());

        // Add two nodes -> 2 NodeJoined events
        mgr.add_node("n1", coord(0, 0, 0), BlockchainScope::Device)
            .expect("test: add");
        mgr.add_node("n2", coord(1, 0, 0), BlockchainScope::Device)
            .expect("test: add");

        let events = mgr.get_events();
        assert_eq!(events.len(), 2);
        assert!(events.iter().all(|e| matches!(e, ClusterEvent::NodeJoined { .. })));

        // Second drain should be empty
        let events2 = mgr.get_events();
        assert!(events2.is_empty());

        // More operations accumulate new events
        mgr.remove_node("n1").expect("test: remove");
        let events3 = mgr.get_events();
        assert_eq!(events3.len(), 1);
        assert!(matches!(&events3[0], ClusterEvent::NodeLeft { node_id } if node_id == "n1"));
    }

    // 13. Duplicate node rejection
    #[test]
    fn test_duplicate_node_rejection() {
        let mut mgr = ClusterManager::new(test_config());
        mgr.add_node("n1", coord(0, 0, 0), BlockchainScope::Device)
            .expect("test: add");

        let result = mgr.add_node("n1", coord(1, 1, 1), BlockchainScope::Network);
        assert!(matches!(result, Err(ClusterError::DuplicateNode(_))));
    }

    // 14. Graceful shutdown rejects Failed nodes
    #[test]
    fn test_graceful_shutdown_rejects_failed() {
        let mut mgr = ClusterManager::new(test_config());
        mgr.add_node("n1", coord(0, 0, 0), BlockchainScope::Device)
            .expect("test: add");

        let node = mgr.nodes.get_mut("n1").expect("test: exists");
        node.status = NodeStatus::Failed;

        let result = mgr.graceful_shutdown("n1");
        assert!(matches!(result, Err(ClusterError::InvalidTransition { .. })));
    }

    // 15. Recovery rejects non-Failed nodes
    #[test]
    fn test_recovery_rejects_non_failed() {
        let mut mgr = ClusterManager::new(test_config());
        mgr.add_node("n1", coord(0, 0, 0), BlockchainScope::Device)
            .expect("test: add");
        mgr.record_heartbeat("n1").expect("test: hb");

        let result = mgr.attempt_recovery("n1");
        assert!(matches!(result, Err(ClusterError::InvalidTransition { .. })));
    }
}
