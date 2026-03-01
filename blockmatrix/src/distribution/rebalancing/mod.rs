// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Dynamic Shard Rebalancing
//!
//! Monitors node topology changes (joins, leaves, failures) and triggers
//! shard redistribution when balance thresholds are exceeded. Uses matrix
//! distance calculations and octant diversity to place replicas optimally.
//!
//! # Design
//!
//! - **Minimal movement**: prefer small moves over full redistribution
//! - **Octant diversity**: spread replicas across different matrix octants
//! - **Cooldown**: prevent thrashing with configurable cooldown periods
//! - **Priority**: node failures trigger higher-priority emergency rebalancing

mod helpers;
mod types;

pub use types::*;

use crate::matrix::coordinate::MatrixCoordinate;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Manages dynamic shard rebalancing across the matrix topology.
///
/// Tracks shard placement, monitors node events, and produces rebalance
/// plans that minimize data movement while maintaining replica diversity.
pub struct RebalanceManager {
    config: RebalanceConfig,
    /// shard_id -> list of (node_id, position)
    shard_map: HashMap<ShardId, Vec<(String, MatrixCoordinate)>>,
    /// node_id -> position
    nodes: HashMap<String, MatrixCoordinate>,
    /// Timestamp of last rebalance completion.
    last_rebalance: Option<Instant>,
}

impl RebalanceManager {
    /// Create a new manager with the given configuration.
    pub fn new(config: RebalanceConfig) -> Self {
        Self {
            config,
            shard_map: HashMap::new(),
            nodes: HashMap::new(),
            last_rebalance: None,
        }
    }

    /// Handle a node joining the topology.
    ///
    /// Returns actions to replicate under-replicated shards to the new node
    /// if it improves octant diversity, and to move shards from overloaded nodes.
    pub fn on_node_joined(
        &mut self,
        node_id: &str,
        position: &MatrixCoordinate,
    ) -> Vec<RebalanceAction> {
        self.nodes.insert(node_id.to_string(), *position);

        let mut actions = Vec::new();
        helpers::under_replicated_actions(
            &self.shard_map,
            &self.nodes,
            node_id,
            position,
            self.config.min_replicas,
            &mut actions,
        );
        helpers::overload_relief_actions(
            &self.shard_map,
            &self.nodes,
            node_id,
            self.config.rebalance_threshold,
            self.config.min_replicas,
            &mut actions,
        );
        actions
    }

    /// Handle a node gracefully leaving the topology.
    ///
    /// Re-replicates every shard that was on the departing node.
    pub fn on_node_left(&mut self, node_id: &str) -> Vec<RebalanceAction> {
        self.nodes.remove(node_id);
        helpers::generate_departure_actions(
            &mut self.shard_map,
            &self.nodes,
            node_id,
            self.config.min_replicas,
        )
    }

    /// Handle a node failure (same as leave but treated as emergency).
    ///
    /// Generates the same actions as `on_node_left` but callers may
    /// prioritize these actions more aggressively.
    pub fn on_node_failed(&mut self, node_id: &str) -> Vec<RebalanceAction> {
        self.nodes.remove(node_id);
        helpers::generate_departure_actions(
            &mut self.shard_map,
            &self.nodes,
            node_id,
            self.config.min_replicas,
        )
    }

    /// Compute the current imbalance across all nodes.
    pub fn check_balance(&self) -> RebalanceReport {
        if self.nodes.is_empty() || self.shard_map.is_empty() {
            return RebalanceReport {
                imbalance_score: 0.0,
                overloaded_nodes: Vec::new(),
                underloaded_nodes: Vec::new(),
                orphaned_shards: helpers::find_orphaned_shards(
                    &self.shard_map,
                    &self.nodes,
                    self.config.min_replicas,
                ),
            };
        }

        let node_counts = helpers::shard_counts_per_node(&self.shard_map, &self.nodes);
        let total: usize = node_counts.values().sum();
        let avg = total as f64 / self.nodes.len().max(1) as f64;

        let mut overloaded = Vec::new();
        let mut underloaded = Vec::new();

        for (nid, &count) in &node_counts {
            let deviation = (count as f64 - avg).abs() / avg.max(1.0);
            if count as f64 > avg * (1.0 + self.config.rebalance_threshold) {
                overloaded.push(nid.clone());
            } else if (count as f64) < avg * (1.0 - self.config.rebalance_threshold)
                && deviation > 0.01
            {
                underloaded.push(nid.clone());
            }
        }

        for nid in self.nodes.keys() {
            if !node_counts.contains_key(nid) {
                underloaded.push(nid.clone());
            }
        }

        RebalanceReport {
            imbalance_score: helpers::compute_imbalance_score(&self.nodes, &node_counts),
            overloaded_nodes: overloaded,
            underloaded_nodes: underloaded,
            orphaned_shards: helpers::find_orphaned_shards(
                &self.shard_map,
                &self.nodes,
                self.config.min_replicas,
            ),
        }
    }

    /// Generate an optimal redistribution plan.
    ///
    /// Returns an empty vec if cooldown has not elapsed.
    pub fn plan_rebalance(&self) -> Vec<RebalanceAction> {
        if !self.cooldown_elapsed() {
            return Vec::new();
        }

        let mut actions = Vec::new();
        self.plan_under_replicated(&mut actions);
        self.plan_excess_replicas(&mut actions);

        let report = self.check_balance();
        if report.imbalance_score >= self.config.rebalance_threshold {
            self.add_balance_moves(&report, &mut actions);
        }

        actions
    }

    /// Execute a set of actions (updates internal state tracking).
    ///
    /// Real shard data transfer is out of scope; this updates the placement
    /// map and records the result.
    pub fn execute_actions(&mut self, actions: &[RebalanceAction]) -> RebalanceResult {
        let start = Instant::now();
        let mut executed = 0usize;
        let mut failed = 0usize;

        for action in actions {
            if self.apply_action(action) {
                executed += 1;
            } else {
                failed += 1;
            }
        }

        self.last_rebalance = Some(Instant::now());

        RebalanceResult {
            actions_executed: executed,
            actions_failed: failed,
            duration: start.elapsed(),
        }
    }

    /// Register a shard placement.
    pub fn register_shard(
        &mut self,
        shard_id: ShardId,
        node_id: &str,
        position: &MatrixCoordinate,
    ) {
        self.shard_map
            .entry(shard_id)
            .or_default()
            .push((node_id.to_string(), *position));
        self.nodes.entry(node_id.to_string()).or_insert(*position);
    }

    /// Get current shard distribution: node -> list of shard ids.
    pub fn get_shard_distribution(&self) -> HashMap<String, Vec<ShardId>> {
        let mut dist: HashMap<String, Vec<ShardId>> = HashMap::new();
        for (shard_id, replicas) in &self.shard_map {
            for (nid, _) in replicas {
                dist.entry(nid.clone()).or_default().push(shard_id.clone());
            }
        }
        dist
    }

    // ------------------------------------------------------------------
    // Private plan/apply helpers
    // ------------------------------------------------------------------

    fn plan_under_replicated(&self, actions: &mut Vec<RebalanceAction>) {
        for (shard_id, replicas) in &self.shard_map {
            let live = helpers::live_replica_count(replicas, &self.nodes);
            if live < self.config.min_replicas {
                let needed = self.config.min_replicas - live;
                for target in helpers::pick_replica_targets(replicas, &self.nodes, needed) {
                    actions.push(RebalanceAction::ReplicateShard {
                        shard_id: shard_id.clone(),
                        to_node: target,
                    });
                }
            }
        }
    }

    fn plan_excess_replicas(&self, actions: &mut Vec<RebalanceAction>) {
        for (shard_id, replicas) in &self.shard_map {
            let live = helpers::live_replica_count(replicas, &self.nodes);
            if live > self.config.max_replicas {
                let excess = live - self.config.max_replicas;
                for node in helpers::pick_replica_removals(replicas, &self.nodes, excess) {
                    actions.push(RebalanceAction::RemoveReplica {
                        shard_id: shard_id.clone(),
                        from_node: node,
                    });
                }
            }
        }
    }

    fn add_balance_moves(&self, report: &RebalanceReport, actions: &mut Vec<RebalanceAction>) {
        for overloaded in &report.overloaded_nodes {
            for underloaded in &report.underloaded_nodes {
                if let Some(shard_id) =
                    helpers::find_movable_shard(&self.shard_map, overloaded, underloaded)
                {
                    actions.push(RebalanceAction::MoveShard {
                        shard_id,
                        from_node: overloaded.clone(),
                        to_node: underloaded.clone(),
                    });
                    break;
                }
            }
        }
    }

    fn apply_action(&mut self, action: &RebalanceAction) -> bool {
        match action {
            RebalanceAction::MoveShard {
                shard_id,
                from_node,
                to_node,
            } => self.apply_move(shard_id, from_node, to_node),
            RebalanceAction::ReplicateShard { shard_id, to_node } => {
                self.apply_replicate(shard_id, to_node)
            }
            RebalanceAction::RemoveReplica {
                shard_id,
                from_node,
            } => self.apply_remove(shard_id, from_node),
        }
    }

    fn apply_move(&mut self, shard_id: &str, from_node: &str, to_node: &str) -> bool {
        let to_pos = match self.nodes.get(to_node) {
            Some(p) => *p,
            None => return false,
        };
        let replicas = match self.shard_map.get_mut(shard_id) {
            Some(r) => r,
            None => return false,
        };
        if replicas.iter().any(|(nid, _)| nid == to_node) {
            return false;
        }
        replicas.retain(|(nid, _)| nid != from_node);
        replicas.push((to_node.to_string(), to_pos));
        true
    }

    fn apply_replicate(&mut self, shard_id: &str, to_node: &str) -> bool {
        let to_pos = match self.nodes.get(to_node) {
            Some(p) => *p,
            None => return false,
        };
        let replicas = match self.shard_map.get_mut(shard_id) {
            Some(r) => r,
            None => return false,
        };
        if replicas.iter().any(|(nid, _)| nid == to_node) {
            return false;
        }
        replicas.push((to_node.to_string(), to_pos));
        true
    }

    fn apply_remove(&mut self, shard_id: &str, from_node: &str) -> bool {
        let replicas = match self.shard_map.get_mut(shard_id) {
            Some(r) => r,
            None => return false,
        };
        let before = replicas.len();
        replicas.retain(|(nid, _)| nid != from_node);
        replicas.len() < before
    }

    fn cooldown_elapsed(&self) -> bool {
        match self.last_rebalance {
            None => true,
            Some(ts) => ts.elapsed() >= Duration::from_secs(self.config.cooldown_secs),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coord(x: i64, y: i64, z: i64) -> MatrixCoordinate {
        MatrixCoordinate::new(x, y, z).expect("test: valid coordinate")
    }

    fn make_manager() -> RebalanceManager {
        RebalanceManager::new(RebalanceConfig {
            min_replicas: 2,
            max_replicas: 4,
            rebalance_threshold: 0.3,
            cooldown_secs: 0,
        })
    }

    fn register_shard_on_nodes(
        mgr: &mut RebalanceManager,
        shard_id: &str,
        nodes: &[(&str, MatrixCoordinate)],
    ) {
        for (nid, pos) in nodes {
            mgr.register_shard(shard_id.to_string(), nid, pos);
        }
    }

    #[test]
    fn test_node_join_triggers_replication() {
        let mut mgr = make_manager();
        let pos_a = coord(10, 10, 10);
        mgr.register_shard("shard-1".to_string(), "node-a", &pos_a);

        let actions = mgr.on_node_joined("node-b", &coord(-10, -10, -10));
        assert!(
            actions.iter().any(|a| matches!(
                a,
                RebalanceAction::ReplicateShard { shard_id, to_node }
                if shard_id == "shard-1" && to_node == "node-b"
            )),
            "Expected ReplicateShard action, got: {actions:?}"
        );
    }

    #[test]
    fn test_node_leave_triggers_replication() {
        let mut mgr = make_manager();
        register_shard_on_nodes(
            &mut mgr,
            "shard-1",
            &[
                ("node-a", coord(10, 10, 10)),
                ("node-b", coord(-10, 10, 10)),
            ],
        );
        mgr.nodes.insert("node-c".to_string(), coord(10, -10, 10));

        let actions = mgr.on_node_left("node-a");
        assert!(
            actions.iter().any(|a| matches!(
                a, RebalanceAction::ReplicateShard { shard_id, .. } if shard_id == "shard-1"
            )),
            "Expected ReplicateShard for shard-1, got: {actions:?}"
        );
    }

    #[test]
    fn test_node_failure_triggers_emergency_replication() {
        let mut mgr = make_manager();
        register_shard_on_nodes(
            &mut mgr,
            "shard-1",
            &[
                ("node-a", coord(10, 10, 10)),
                ("node-b", coord(-10, 10, 10)),
            ],
        );
        mgr.nodes.insert("node-c".to_string(), coord(10, -10, -10));

        let actions = mgr.on_node_failed("node-a");
        assert!(
            actions.iter().any(|a| matches!(
                a, RebalanceAction::ReplicateShard { shard_id, .. } if shard_id == "shard-1"
            )),
            "Expected emergency replication, got: {actions:?}"
        );
        assert!(
            !mgr.nodes.contains_key("node-a"),
            "Failed node should be removed"
        );
    }

    #[test]
    fn test_check_balance_detects_imbalance() {
        let mut mgr = make_manager();
        let pos_a = coord(10, 10, 10);
        mgr.nodes.insert("node-a".to_string(), pos_a);
        mgr.nodes.insert("node-b".to_string(), coord(-10, 10, 10));

        for i in 0..10 {
            mgr.register_shard(format!("shard-{i}"), "node-a", &pos_a);
        }

        let report = mgr.check_balance();
        assert!(
            report.imbalance_score > 0.3,
            "Expected high imbalance: {}",
            report.imbalance_score
        );
        assert!(report.overloaded_nodes.contains(&"node-a".to_string()));
        assert!(report.underloaded_nodes.contains(&"node-b".to_string()));
    }

    #[test]
    fn test_cooldown_prevents_thrashing() {
        let mut mgr = RebalanceManager::new(RebalanceConfig {
            min_replicas: 2,
            max_replicas: 4,
            rebalance_threshold: 0.3,
            cooldown_secs: 3600,
        });
        let pos_a = coord(10, 10, 10);
        mgr.nodes.insert("node-a".to_string(), pos_a);
        mgr.nodes.insert("node-b".to_string(), coord(-10, 10, 10));
        for i in 0..10 {
            mgr.register_shard(format!("shard-{i}"), "node-a", &pos_a);
        }

        let actions = mgr.plan_rebalance();
        assert!(!actions.is_empty(), "First plan should produce actions");
        mgr.execute_actions(&actions);

        assert!(
            mgr.plan_rebalance().is_empty(),
            "Cooldown should block second plan"
        );
    }

    #[test]
    fn test_min_replica_enforcement() {
        let mut mgr = make_manager();
        mgr.nodes.insert("node-a".to_string(), coord(10, 10, 10));
        mgr.nodes.insert("node-b".to_string(), coord(-10, 10, 10));
        mgr.nodes.insert("node-c".to_string(), coord(10, -10, 10));
        mgr.register_shard("shard-1".to_string(), "node-a", &coord(10, 10, 10));

        let actions = mgr.plan_rebalance();
        let count = actions.iter()
            .filter(|a| matches!(a, RebalanceAction::ReplicateShard { shard_id, .. } if shard_id == "shard-1"))
            .count();
        assert!(count >= 1, "Expected >= 1 ReplicateShard, got {count}");
    }

    #[test]
    fn test_octant_diversity_preference() {
        let mut mgr = make_manager();
        mgr.nodes.insert("node-a".to_string(), coord(10, 10, 10));
        mgr.nodes.insert("node-b".to_string(), coord(20, 20, 20));
        mgr.nodes.insert("node-c".to_string(), coord(-10, -10, -10));
        mgr.register_shard("shard-1".to_string(), "node-a", &coord(10, 10, 10));

        let actions = mgr.plan_rebalance();
        let target = actions.iter().find_map(|a| match a {
            RebalanceAction::ReplicateShard { shard_id, to_node } if shard_id == "shard-1" => {
                Some(to_node.clone())
            }
            _ => None,
        });
        assert_eq!(
            target,
            Some("node-c".to_string()),
            "Should prefer different octant"
        );
    }

    #[test]
    fn test_empty_topology() {
        let mgr = make_manager();
        let report = mgr.check_balance();
        assert_eq!(report.imbalance_score, 0.0);
        assert!(report.overloaded_nodes.is_empty());
        assert!(report.underloaded_nodes.is_empty());
        assert!(report.orphaned_shards.is_empty());
        assert!(mgr.plan_rebalance().is_empty());
    }

    #[test]
    fn test_large_topology_stress() {
        let mut mgr = make_manager();
        for i in 0..50 {
            let sx = if i % 2 == 0 { 1 } else { -1 };
            let sy = if (i / 2) % 2 == 0 { 1 } else { -1 };
            let sz = if (i / 4) % 2 == 0 { 1 } else { -1 };
            mgr.nodes.insert(
                format!("node-{i}"),
                coord(
                    sx * (i * 7 + 5) as i64,
                    sy * (i * 3 + 10) as i64,
                    sz * (i * 11 + 2) as i64,
                ),
            );
        }
        for i in 0..100 {
            let idx = i % 50;
            let pos = *mgr
                .nodes
                .get(&format!("node-{idx}"))
                .expect("test: node exists");
            mgr.register_shard(format!("shard-{i}"), &format!("node-{idx}"), &pos);
        }

        let report = mgr.check_balance();
        assert!(
            !report.orphaned_shards.is_empty(),
            "Shards under-replicated"
        );

        let actions = mgr.plan_rebalance();
        assert!(
            actions.len() >= 100,
            "Expected >= 100 actions, got {}",
            actions.len()
        );

        let result = mgr.execute_actions(&actions);
        assert!(result.actions_executed > 0);
    }

    #[test]
    fn test_concurrent_join_and_leave() {
        let mut mgr = make_manager();
        register_shard_on_nodes(
            &mut mgr,
            "shard-1",
            &[
                ("node-a", coord(10, 10, 10)),
                ("node-b", coord(-10, 10, 10)),
                ("node-c", coord(10, -10, 10)),
            ],
        );

        let leave_actions = mgr.on_node_left("node-a");
        let join_actions = mgr.on_node_joined("node-d", &coord(-10, -10, -10));

        let combined: Vec<_> = leave_actions.into_iter().chain(join_actions).collect();
        let result = mgr.execute_actions(&combined);
        assert_eq!(result.actions_failed, 0, "No actions should fail");

        let dist = mgr.get_shard_distribution();
        let count = dist
            .iter()
            .filter(|(_, shards)| shards.contains(&"shard-1".to_string()))
            .count();
        assert!(
            count >= mgr.config.min_replicas,
            "shard-1 needs >= {} replicas, has {}",
            mgr.config.min_replicas,
            count
        );
    }

    #[test]
    fn test_execute_actions_updates_state() {
        let mut mgr = make_manager();
        mgr.nodes.insert("node-a".to_string(), coord(10, 10, 10));
        mgr.nodes.insert("node-b".to_string(), coord(-10, 10, 10));
        mgr.register_shard("shard-1".to_string(), "node-a", &coord(10, 10, 10));

        let result = mgr.execute_actions(&[RebalanceAction::ReplicateShard {
            shard_id: "shard-1".to_string(),
            to_node: "node-b".to_string(),
        }]);
        assert_eq!(result.actions_executed, 1);

        let dist = mgr.get_shard_distribution();
        assert!(dist
            .get("node-b")
            .is_some_and(|s| s.contains(&"shard-1".to_string())));
    }

    #[test]
    fn test_remove_excess_replicas() {
        let mut mgr = RebalanceManager::new(RebalanceConfig {
            min_replicas: 2,
            max_replicas: 3,
            rebalance_threshold: 0.3,
            cooldown_secs: 0,
        });
        let positions = [
            coord(10, 10, 10),
            coord(-10, 10, 10),
            coord(10, -10, 10),
            coord(-10, -10, 10),
            coord(10, 10, -10),
        ];
        for (i, pos) in positions.iter().enumerate() {
            let nid = format!("node-{i}");
            mgr.nodes.insert(nid.clone(), *pos);
            mgr.register_shard("shard-1".to_string(), &nid, pos);
        }

        let removes = mgr
            .plan_rebalance()
            .iter()
            .filter(|a| matches!(a, RebalanceAction::RemoveReplica { .. }))
            .count();
        assert!(
            removes >= 2,
            "Should remove >= 2 excess (5 - max 3), got {removes}"
        );
    }

    #[test]
    fn test_get_shard_distribution() {
        let mut mgr = make_manager();
        mgr.register_shard("shard-1".to_string(), "node-a", &coord(10, 10, 10));
        mgr.register_shard("shard-1".to_string(), "node-b", &coord(-10, 10, 10));
        mgr.register_shard("shard-2".to_string(), "node-a", &coord(10, 10, 10));

        let dist = mgr.get_shard_distribution();
        assert_eq!(dist.get("node-a").map(|v| v.len()).unwrap_or(0), 2);
        assert_eq!(dist.get("node-b").map(|v| v.len()).unwrap_or(0), 1);
    }
}
