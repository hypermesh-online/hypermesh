// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Scope-aware transaction routing using tensor mathematics
//!
//! Combines the existing tensor routing library (Vector3D, routing vectors,
//! similarity scoring) with [`BlockchainScope`] awareness so that transactions
//! can be routed within a single scope or bridged across scope boundaries.
//!
//! # Cross-scope routing
//!
//! When source and destination belong to different scopes the router locates
//! *boundary nodes* — nodes registered in the target scope — and builds a
//! bridged path through the best-aligned relay.  Each scope transition adds
//! a configurable cost penalty so that same-scope paths are naturally
//! preferred when available.

use std::collections::HashMap;

use hypermesh_lib::BlockchainScope;

use crate::matrix::coordinate::MatrixCoordinate;
use crate::matrix::tensor::routing::{calculate_routing_vector, routing_similarity};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Tuning knobs for the transaction router.
#[derive(Debug, Clone)]
pub struct TransactionRoutingConfig {
    /// Base cost multiplier per hop (multiplied by Euclidean distance).
    pub base_hop_cost: f64,
    /// Flat penalty added for every scope transition.
    pub cross_scope_penalty: f64,
    /// Hard limit on the number of hops in a route.
    pub max_path_length: usize,
    /// When `true`, same-scope routes receive a cost discount.
    pub prefer_same_scope: bool,
}

impl Default for TransactionRoutingConfig {
    fn default() -> Self {
        Self {
            base_hop_cost: 1.0,
            cross_scope_penalty: 10.0,
            max_path_length: 32,
            prefer_same_scope: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Route types
// ---------------------------------------------------------------------------

/// A single hop in a transaction route.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteHop {
    pub node_id: String,
    pub position: MatrixCoordinate,
    pub scope: BlockchainScope,
    pub hop_cost: f64,
}

/// Records a scope boundary crossing inside a route.
#[derive(Debug, Clone, PartialEq)]
pub struct ScopeTransition {
    pub at_hop_index: usize,
    pub from_scope: BlockchainScope,
    pub to_scope: BlockchainScope,
    pub transition_cost: f64,
}

/// A fully resolved transaction route.
#[derive(Debug, Clone)]
pub struct TransactionRoute {
    pub hops: Vec<RouteHop>,
    pub total_cost: f64,
    pub scope_transitions: Vec<ScopeTransition>,
    pub estimated_latency_ms: f64,
}

/// Errors produced by the transaction router.
#[derive(Debug, Clone, PartialEq)]
pub enum RoutingError {
    /// No path could be found between the two coordinates.
    NoPathFound,
    /// The target scope has no registered nodes.
    ScopeUnreachable(BlockchainScope),
    /// The route would exceed the configured maximum path length.
    MaxPathLengthExceeded { limit: usize },
    /// There are no nodes registered in the requested scope.
    NoNodesInScope(BlockchainScope),
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoPathFound => write!(f, "no path found"),
            Self::ScopeUnreachable(s) => write!(f, "scope unreachable: {s}"),
            Self::MaxPathLengthExceeded { limit } => {
                write!(f, "max path length exceeded ({limit})")
            }
            Self::NoNodesInScope(s) => write!(f, "no nodes in scope: {s}"),
        }
    }
}

impl std::error::Error for RoutingError {}

/// Aggregate routing statistics.
#[derive(Debug, Clone, Default)]
pub struct RoutingStatistics {
    pub total_routes: u64,
    pub cross_scope_routes: u64,
    pub total_hops: u64,
    pub total_cost: f64,
}

impl RoutingStatistics {
    /// Average hops per route (0.0 when no routes recorded).
    pub fn avg_hops(&self) -> f64 {
        if self.total_routes == 0 {
            return 0.0;
        }
        self.total_hops as f64 / self.total_routes as f64
    }

    /// Average cost per route (0.0 when no routes recorded).
    pub fn avg_cost(&self) -> f64 {
        if self.total_routes == 0 {
            return 0.0;
        }
        self.total_cost / self.total_routes as f64
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Registered node entry.
#[derive(Debug, Clone)]
struct NodeEntry {
    node_id: String,
    position: MatrixCoordinate,
    scope: BlockchainScope,
}

/// Scope-aware transaction router built on top of tensor math primitives.
#[derive(Debug)]
pub struct TransactionRouter {
    config: TransactionRoutingConfig,
    nodes: HashMap<String, NodeEntry>,
    stats: RoutingStatistics,
    /// Optional per-node weight adjustments from engauge routing intelligence.
    /// Maps node_id to a multiplicative weight factor (e.g., 0.5 penalizes,
    /// 1.5 boosts). Applied in relay scoring so congested routes are penalized.
    weight_adjustments: Option<HashMap<String, f64>>,
}

impl TransactionRouter {
    /// Create a new router with the given configuration.
    pub fn new(config: TransactionRoutingConfig) -> Self {
        Self {
            config,
            nodes: HashMap::new(),
            stats: RoutingStatistics::default(),
            weight_adjustments: None,
        }
    }

    /// Set per-node weight adjustments from engauge routing intelligence.
    ///
    /// Each entry maps a `node_id` to a multiplicative weight factor that is
    /// applied during relay scoring. Values below 1.0 penalize congested nodes;
    /// values above 1.0 boost preferred nodes.
    pub fn set_weight_adjustments(&mut self, adjustments: Vec<(String, f64)>) {
        if adjustments.is_empty() {
            self.weight_adjustments = None;
        } else {
            self.weight_adjustments = Some(adjustments.into_iter().collect());
        }
    }

    /// Register a node with its matrix position and blockchain scope.
    pub fn register_node(
        &mut self,
        node_id: &str,
        position: MatrixCoordinate,
        scope: BlockchainScope,
    ) {
        self.nodes.insert(
            node_id.to_string(),
            NodeEntry {
                node_id: node_id.to_string(),
                position,
                scope,
            },
        );
    }

    /// Return all nodes that participate in `scope`.
    pub fn find_scope_boundary_nodes(
        &self,
        scope: BlockchainScope,
    ) -> Vec<(String, MatrixCoordinate)> {
        self.nodes
            .values()
            .filter(|e| e.scope == scope)
            .map(|e| (e.node_id.clone(), e.position))
            .collect()
    }

    /// Route a transaction between two coordinates / scopes.
    ///
    /// * Same scope: direct tensor-based path.
    /// * Cross scope: find a relay node in the target scope that best aligns
    ///   with the source-to-destination direction, then build a two-leg path.
    pub fn route_transaction(
        &mut self,
        from: &MatrixCoordinate,
        to: &MatrixCoordinate,
        from_scope: BlockchainScope,
        to_scope: BlockchainScope,
    ) -> Result<TransactionRoute, RoutingError> {
        if from_scope == to_scope {
            self.route_same_scope(from, to, from_scope)
        } else {
            self.route_cross_scope(from, to, from_scope, to_scope)
        }
    }

    /// Calculate the total cost of a route using the router configuration.
    pub fn calculate_route_cost(&self, route: &TransactionRoute) -> f64 {
        let hop_cost: f64 = route.hops.iter().map(|h| h.hop_cost).sum();
        let transition_cost: f64 = route
            .scope_transitions
            .iter()
            .map(|t| t.transition_cost)
            .sum();
        let same_scope_discount =
            if self.config.prefer_same_scope && route.scope_transitions.is_empty() {
                0.9 // 10 % discount
            } else {
                1.0
            };

        (hop_cost + transition_cost) * same_scope_discount
    }

    /// Select the best relay from `candidates` using direction alignment
    /// scoring (dot product via `routing_similarity`).
    ///
    /// When [`set_weight_adjustments`] has been called, each candidate's
    /// alignment score is multiplied by its weight factor so that congested
    /// nodes (weight < 1.0) are penalised and preferred nodes (weight > 1.0)
    /// are boosted.
    pub fn find_optimal_relay(
        &self,
        from: &MatrixCoordinate,
        to: &MatrixCoordinate,
        candidates: &[(String, MatrixCoordinate)],
    ) -> Option<(String, MatrixCoordinate)> {
        if candidates.is_empty() {
            return None;
        }

        let ideal_dir = calculate_routing_vector(from, to);

        candidates
            .iter()
            .map(|(id, pos)| {
                let dir_to_candidate = calculate_routing_vector(from, pos);
                let dir_from_candidate = calculate_routing_vector(pos, to);
                // Prefer candidates aligned with the overall direction AND
                // positioned so that the second leg also aligns well.
                let mut score = routing_similarity(&ideal_dir, &dir_to_candidate)
                    + routing_similarity(&ideal_dir, &dir_from_candidate);

                // Apply engauge weight adjustment if available for this node
                if let Some(ref adjustments) = self.weight_adjustments {
                    if let Some(&factor) = adjustments.get(id) {
                        score *= factor;
                    }
                }

                (id.clone(), *pos, score)
            })
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, pos, _)| (id, pos))
    }

    /// Return a snapshot of the routing statistics collected so far.
    pub fn get_routing_statistics(&self) -> RoutingStatistics {
        self.stats.clone()
    }

    // -- private helpers ----------------------------------------------------

    fn route_same_scope(
        &mut self,
        from: &MatrixCoordinate,
        to: &MatrixCoordinate,
        scope: BlockchainScope,
    ) -> Result<TransactionRoute, RoutingError> {
        let distance = from.euclidean_distance(to);
        let hop_cost = distance * self.config.base_hop_cost;

        let hops = vec![
            RouteHop {
                node_id: format!("src@{from}"),
                position: *from,
                scope,
                hop_cost: 0.0,
            },
            RouteHop {
                node_id: format!("dst@{to}"),
                position: *to,
                scope,
                hop_cost,
            },
        ];

        if hops.len() > self.config.max_path_length {
            return Err(RoutingError::MaxPathLengthExceeded {
                limit: self.config.max_path_length,
            });
        }

        let mut route = TransactionRoute {
            hops,
            total_cost: 0.0,
            scope_transitions: Vec::new(),
            estimated_latency_ms: distance * 0.1, // simple model
        };
        route.total_cost = self.calculate_route_cost(&route);

        self.record_stats(&route);
        Ok(route)
    }

    fn route_cross_scope(
        &mut self,
        from: &MatrixCoordinate,
        to: &MatrixCoordinate,
        from_scope: BlockchainScope,
        to_scope: BlockchainScope,
    ) -> Result<TransactionRoute, RoutingError> {
        let candidates = self.find_scope_boundary_nodes(to_scope);
        if candidates.is_empty() {
            return Err(RoutingError::NoNodesInScope(to_scope));
        }

        let (relay_id, relay_pos) = self
            .find_optimal_relay(from, to, &candidates)
            .ok_or(RoutingError::ScopeUnreachable(to_scope))?;

        let leg1_dist = from.euclidean_distance(&relay_pos);
        let leg2_dist = relay_pos.euclidean_distance(to);

        let hops = vec![
            RouteHop {
                node_id: format!("src@{from}"),
                position: *from,
                scope: from_scope,
                hop_cost: 0.0,
            },
            RouteHop {
                node_id: relay_id,
                position: relay_pos,
                scope: to_scope,
                hop_cost: leg1_dist * self.config.base_hop_cost,
            },
            RouteHop {
                node_id: format!("dst@{to}"),
                position: *to,
                scope: to_scope,
                hop_cost: leg2_dist * self.config.base_hop_cost,
            },
        ];

        if hops.len() > self.config.max_path_length {
            return Err(RoutingError::MaxPathLengthExceeded {
                limit: self.config.max_path_length,
            });
        }

        let transition = ScopeTransition {
            at_hop_index: 1,
            from_scope,
            to_scope,
            transition_cost: self.config.cross_scope_penalty,
        };

        let mut route = TransactionRoute {
            hops,
            total_cost: 0.0,
            scope_transitions: vec![transition],
            estimated_latency_ms: (leg1_dist + leg2_dist) * 0.1,
        };
        route.total_cost = self.calculate_route_cost(&route);

        self.record_stats(&route);
        Ok(route)
    }

    fn record_stats(&mut self, route: &TransactionRoute) {
        self.stats.total_routes += 1;
        self.stats.total_hops += route.hops.len() as u64;
        self.stats.total_cost += route.total_cost;
        if !route.scope_transitions.is_empty() {
            self.stats.cross_scope_routes += 1;
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn default_router() -> TransactionRouter {
        TransactionRouter::new(TransactionRoutingConfig::default())
    }

    fn coord(x: i64, y: i64, z: i64) -> MatrixCoordinate {
        MatrixCoordinate::new(x, y, z).expect("test: valid coord")
    }

    // 1. Same-scope Device transaction routing
    #[test]
    fn same_scope_device_routing() {
        let mut router = default_router();
        let route = router
            .route_transaction(
                &coord(0, 0, 0),
                &coord(10, 0, 0),
                BlockchainScope::Device,
                BlockchainScope::Device,
            )
            .expect("test: same-scope device route");

        assert_eq!(route.hops.len(), 2);
        assert!(route.scope_transitions.is_empty());
        assert!(route.total_cost > 0.0);
        assert_eq!(route.hops[0].scope, BlockchainScope::Device);
        assert_eq!(route.hops[1].scope, BlockchainScope::Device);
    }

    // 2. Same-scope Network transaction routing
    #[test]
    fn same_scope_network_routing() {
        let mut router = default_router();
        let route = router
            .route_transaction(
                &coord(0, 0, 0),
                &coord(5, 5, 5),
                BlockchainScope::Network,
                BlockchainScope::Network,
            )
            .expect("test: same-scope network route");

        assert_eq!(route.hops.len(), 2);
        assert!(route.scope_transitions.is_empty());
        assert_eq!(route.hops[0].scope, BlockchainScope::Network);
    }

    // 3. Cross-scope Device -> Network routing
    #[test]
    fn cross_scope_device_to_network() {
        let mut router = default_router();
        router.register_node("relay-1", coord(5, 0, 0), BlockchainScope::Network);

        let route = router
            .route_transaction(
                &coord(0, 0, 0),
                &coord(10, 0, 0),
                BlockchainScope::Device,
                BlockchainScope::Network,
            )
            .expect("test: cross-scope device->network");

        assert_eq!(route.hops.len(), 3);
        assert_eq!(route.scope_transitions.len(), 1);
        assert_eq!(
            route.scope_transitions[0].from_scope,
            BlockchainScope::Device
        );
        assert_eq!(
            route.scope_transitions[0].to_scope,
            BlockchainScope::Network
        );
    }

    // 4. Cross-scope Network -> Device routing
    #[test]
    fn cross_scope_network_to_device() {
        let mut router = default_router();
        router.register_node("device-node", coord(8, 3, 0), BlockchainScope::Device);

        let route = router
            .route_transaction(
                &coord(0, 0, 0),
                &coord(10, 5, 0),
                BlockchainScope::Network,
                BlockchainScope::Device,
            )
            .expect("test: cross-scope network->device");

        assert_eq!(route.scope_transitions.len(), 1);
        assert_eq!(
            route.scope_transitions[0].from_scope,
            BlockchainScope::Network
        );
        assert_eq!(route.scope_transitions[0].to_scope, BlockchainScope::Device);
    }

    // 5. Route cost calculation with and without scope penalty
    #[test]
    fn route_cost_with_and_without_penalty() {
        let mut router = default_router();
        router.register_node("relay", coord(5, 0, 0), BlockchainScope::Network);

        let same = router
            .route_transaction(
                &coord(0, 0, 0),
                &coord(10, 0, 0),
                BlockchainScope::Device,
                BlockchainScope::Device,
            )
            .expect("test: same-scope cost");

        let cross = router
            .route_transaction(
                &coord(0, 0, 0),
                &coord(10, 0, 0),
                BlockchainScope::Device,
                BlockchainScope::Network,
            )
            .expect("test: cross-scope cost");

        // Cross-scope must be more expensive due to penalty
        assert!(
            cross.total_cost > same.total_cost,
            "cross {:.2} should exceed same {:.2}",
            cross.total_cost,
            same.total_cost
        );
    }

    // 6. No path found (empty topology for cross-scope)
    #[test]
    fn no_path_empty_topology() {
        let mut router = default_router();
        // No nodes registered for Network scope
        let result = router.route_transaction(
            &coord(0, 0, 0),
            &coord(10, 0, 0),
            BlockchainScope::Device,
            BlockchainScope::Network,
        );
        assert!(result.is_err());
        assert_eq!(
            result.expect_err("test: expected error"),
            RoutingError::NoNodesInScope(BlockchainScope::Network)
        );
    }

    // 7. No nodes in target scope
    #[test]
    fn no_nodes_in_target_scope() {
        let mut router = default_router();
        // Register only Device nodes
        router.register_node("dev-1", coord(1, 0, 0), BlockchainScope::Device);
        router.register_node("dev-2", coord(2, 0, 0), BlockchainScope::Device);

        let result = router.route_transaction(
            &coord(0, 0, 0),
            &coord(10, 0, 0),
            BlockchainScope::Device,
            BlockchainScope::Network,
        );

        assert_eq!(
            result.expect_err("test: expected NoNodesInScope"),
            RoutingError::NoNodesInScope(BlockchainScope::Network)
        );
    }

    // 8. Optimal relay selection (alignment scoring)
    #[test]
    fn optimal_relay_alignment() {
        let router = default_router();
        let from = coord(0, 0, 0);
        let to = coord(100, 0, 0);

        let candidates = vec![
            ("aligned".to_string(), coord(50, 0, 0)), // on the direct line
            ("off-axis".to_string(), coord(50, 80, 0)), // far off the direct line
            ("behind".to_string(), coord(-20, 0, 0)), // behind the source
        ];

        let best = router
            .find_optimal_relay(&from, &to, &candidates)
            .expect("test: should find relay");

        assert_eq!(best.0, "aligned");
    }

    // 9. Max path length exceeded
    #[test]
    fn max_path_length_exceeded() {
        let config = TransactionRoutingConfig {
            max_path_length: 1, // impossibly small
            ..Default::default()
        };
        let mut router = TransactionRouter::new(config);

        let result = router.route_transaction(
            &coord(0, 0, 0),
            &coord(10, 0, 0),
            BlockchainScope::Device,
            BlockchainScope::Device,
        );

        assert_eq!(
            result.expect_err("test: expected MaxPathLengthExceeded"),
            RoutingError::MaxPathLengthExceeded { limit: 1 }
        );
    }

    // 10. Multiple scope transitions (two cross-scope routes in sequence)
    #[test]
    fn multiple_scope_transitions_stats() {
        let mut router = default_router();
        router.register_node("net-relay", coord(5, 0, 0), BlockchainScope::Network);
        router.register_node("dev-relay", coord(8, 0, 0), BlockchainScope::Device);

        // First cross-scope route: Device -> Network
        let r1 = router
            .route_transaction(
                &coord(0, 0, 0),
                &coord(10, 0, 0),
                BlockchainScope::Device,
                BlockchainScope::Network,
            )
            .expect("test: first cross-scope");
        assert_eq!(r1.scope_transitions.len(), 1);

        // Second cross-scope route: Network -> Device
        let r2 = router
            .route_transaction(
                &coord(10, 0, 0),
                &coord(20, 0, 0),
                BlockchainScope::Network,
                BlockchainScope::Device,
            )
            .expect("test: second cross-scope");
        assert_eq!(r2.scope_transitions.len(), 1);

        let stats = router.get_routing_statistics();
        assert_eq!(stats.cross_scope_routes, 2);
    }

    // 11. Routing statistics tracking
    #[test]
    fn routing_statistics_tracking() {
        let mut router = default_router();
        router.register_node("net-1", coord(5, 0, 0), BlockchainScope::Network);

        // Same-scope
        let _ = router
            .route_transaction(
                &coord(0, 0, 0),
                &coord(3, 4, 0),
                BlockchainScope::Device,
                BlockchainScope::Device,
            )
            .expect("test: stats same-scope");

        // Cross-scope
        let _ = router
            .route_transaction(
                &coord(0, 0, 0),
                &coord(10, 0, 0),
                BlockchainScope::Device,
                BlockchainScope::Network,
            )
            .expect("test: stats cross-scope");

        let stats = router.get_routing_statistics();
        assert_eq!(stats.total_routes, 2);
        assert_eq!(stats.cross_scope_routes, 1);
        assert!(stats.avg_hops() > 0.0);
        assert!(stats.avg_cost() > 0.0);
    }

    // 12. Large topology (20+ nodes, mixed scopes)
    #[test]
    fn large_topology_mixed_scopes() {
        let mut router = default_router();

        // Register 12 Device nodes in a grid
        for i in 0..12 {
            let x = (i % 4) * 10;
            let y = (i / 4) * 10;
            router.register_node(&format!("dev-{i}"), coord(x, y, 0), BlockchainScope::Device);
        }

        // Register 12 Network nodes in a shifted grid
        for i in 0..12 {
            let x = (i % 4) * 10 + 5;
            let y = (i / 4) * 10 + 5;
            router.register_node(
                &format!("net-{i}"),
                coord(x, y, 0),
                BlockchainScope::Network,
            );
        }

        assert_eq!(router.nodes.len(), 24);

        // Same-scope route across the grid
        let same = router
            .route_transaction(
                &coord(0, 0, 0),
                &coord(30, 20, 0),
                BlockchainScope::Device,
                BlockchainScope::Device,
            )
            .expect("test: large same-scope");
        assert!(same.total_cost > 0.0);

        // Cross-scope route
        let cross = router
            .route_transaction(
                &coord(0, 0, 0),
                &coord(35, 25, 0),
                BlockchainScope::Device,
                BlockchainScope::Network,
            )
            .expect("test: large cross-scope");
        assert!(!cross.scope_transitions.is_empty());

        let stats = router.get_routing_statistics();
        assert_eq!(stats.total_routes, 2);
        assert_eq!(stats.cross_scope_routes, 1);
    }

    // 13. find_scope_boundary_nodes returns correct set
    #[test]
    fn scope_boundary_nodes_filtering() {
        let mut router = default_router();
        router.register_node("d1", coord(0, 0, 0), BlockchainScope::Device);
        router.register_node("d2", coord(1, 0, 0), BlockchainScope::Device);
        router.register_node("n1", coord(2, 0, 0), BlockchainScope::Network);

        let device_nodes = router.find_scope_boundary_nodes(BlockchainScope::Device);
        assert_eq!(device_nodes.len(), 2);

        let network_nodes = router.find_scope_boundary_nodes(BlockchainScope::Network);
        assert_eq!(network_nodes.len(), 1);
        assert_eq!(network_nodes[0].0, "n1");
    }

    // 14. Same-scope cost gets discount when prefer_same_scope is enabled
    #[test]
    fn same_scope_discount_applied() {
        let config = TransactionRoutingConfig {
            prefer_same_scope: true,
            ..Default::default()
        };
        let mut router = TransactionRouter::new(config);

        let route = router
            .route_transaction(
                &coord(0, 0, 0),
                &coord(10, 0, 0),
                BlockchainScope::Device,
                BlockchainScope::Device,
            )
            .expect("test: discount route");

        // Raw hop cost would be 10.0 * 1.0 = 10.0; with 0.9 discount = 9.0
        assert!(
            (route.total_cost - 9.0).abs() < 0.001,
            "expected ~9.0 got {:.3}",
            route.total_cost
        );
    }

    // 15. Estimated latency is proportional to distance
    #[test]
    fn estimated_latency_proportional() {
        let mut router = default_router();

        let short = router
            .route_transaction(
                &coord(0, 0, 0),
                &coord(5, 0, 0),
                BlockchainScope::Device,
                BlockchainScope::Device,
            )
            .expect("test: short latency");

        let long = router
            .route_transaction(
                &coord(0, 0, 0),
                &coord(50, 0, 0),
                BlockchainScope::Device,
                BlockchainScope::Device,
            )
            .expect("test: long latency");

        assert!(long.estimated_latency_ms > short.estimated_latency_ms);
    }
}
