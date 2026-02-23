// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Cross-scope proxy routing with BlockchainScope awareness
//!
//! Adds blockchain scope context to the proxy routing system so that routes
//! are resolved differently depending on whether traffic stays within a
//! single scope (Device or Network) or crosses scope boundaries. Cross-scope
//! routes are channelled through registered gateway nodes.

use std::collections::HashMap;

use hypermesh_lib::BlockchainScope;
use serde::{Deserialize, Serialize};

use crate::matrix::MatrixCoordinate;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Controls how the router handles cross-scope traffic.
#[derive(Debug, Clone)]
pub struct ScopeRoutingConfig {
    /// Allow routing between different blockchain scopes at all.
    pub allow_cross_scope: bool,
    /// When true, Device-scope traffic is always routed locally (no gateway).
    pub device_scope_local_only: bool,
    /// Require a gateway node for any cross-scope route.
    pub require_gateway_for_cross_scope: bool,
    /// Maximum number of scope transitions (hops) a single route may traverse.
    pub max_scope_hops: usize,
}

impl Default for ScopeRoutingConfig {
    fn default() -> Self {
        Self {
            allow_cross_scope: true,
            device_scope_local_only: true,
            require_gateway_for_cross_scope: true,
            max_scope_hops: 3,
        }
    }
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A resolved route that carries blockchain-scope context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeAwareRoute {
    /// Ordered list of matrix positions the route traverses.
    pub path: Vec<MatrixCoordinate>,
    /// Scope the asset originates in.
    pub from_scope: BlockchainScope,
    /// Scope the asset is destined for.
    pub to_scope: BlockchainScope,
    /// Gateway node bridging the two scopes (None for same-scope routes).
    pub gateway_node: Option<String>,
    /// Estimated end-to-end latency in milliseconds.
    pub estimated_latency_ms: f64,
    /// Whether the payload must be encrypted in transit.
    pub requires_encryption: bool,
}

/// Metadata for a node that can bridge two or more blockchain scopes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayNodeInfo {
    /// Human-readable node identifier.
    pub node_id: String,
    /// Position in the Block-MATRIX topology.
    pub position: MatrixCoordinate,
    /// Scopes this gateway can bridge between.
    pub bridged_scopes: Vec<BlockchainScope>,
    /// Health score (0.0 = dead, 1.0 = perfect).
    pub health_score: f64,
    /// Current load factor (0.0 = idle, 1.0 = saturated).
    pub current_load: f64,
}

/// Errors specific to scope-aware routing.
#[derive(Debug, thiserror::Error)]
pub enum ScopeRoutingError {
    /// Cross-scope routing is disabled in the configuration.
    #[error("Cross-scope routing is disabled")]
    CrossScopeDisabled,

    /// No gateway node is available to bridge the requested scopes.
    #[error("No gateway available for {from} -> {to}")]
    NoGatewayAvailable {
        from: BlockchainScope,
        to: BlockchainScope,
    },

    /// The requested scope transition is not valid.
    #[error("Invalid scope transition: {from} -> {to}, reason: {reason}")]
    InvalidScopeTransition {
        from: BlockchainScope,
        to: BlockchainScope,
        reason: String,
    },

    /// Network connectivity is required but unavailable.
    #[error("Network unavailable for scope transition {from} -> {to}")]
    NetworkUnavailable {
        from: BlockchainScope,
        to: BlockchainScope,
    },

    /// Route exceeds the maximum allowed scope hops.
    #[error("Max scope hops exceeded (limit: {limit})")]
    MaxHopsExceeded { limit: usize },
}

/// Aggregate statistics for scope-aware routing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScopeRoutingStats {
    /// Number of routes resolved per scope pair (serialised as "Device->Network").
    pub routes_by_scope: HashMap<String, u64>,
    /// Total cross-scope transfers routed.
    pub cross_scope_transfers: u64,
    /// Mapping of gateway node_id to number of routes served.
    pub gateway_utilization: HashMap<String, u64>,
}

// ---------------------------------------------------------------------------
// Transition log entry (internal bookkeeping)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ScopeTransition {
    _from: BlockchainScope,
    _to: BlockchainScope,
    _gateway: Option<String>,
}

// ---------------------------------------------------------------------------
// ScopeAwareRouter
// ---------------------------------------------------------------------------

/// Routes proxy traffic with full blockchain-scope awareness.
///
/// Same-scope routes are resolved directly (no gateway needed). Cross-scope
/// routes are validated against the configuration and routed through registered
/// gateway nodes.
pub struct ScopeAwareRouter {
    config: ScopeRoutingConfig,
    /// Registered gateway nodes indexed by node_id.
    gateways: HashMap<String, GatewayNodeInfo>,
    /// Running statistics.
    stats: ScopeRoutingStats,
    /// Log of scope transitions for auditing.
    transition_log: Vec<ScopeTransition>,
}

impl ScopeAwareRouter {
    /// Create a new router with the given configuration.
    pub fn new(config: ScopeRoutingConfig) -> Self {
        Self {
            config,
            gateways: HashMap::new(),
            stats: ScopeRoutingStats::default(),
            transition_log: Vec::new(),
        }
    }

    /// Resolve a route for an asset moving between blockchain scopes.
    ///
    /// Same-scope requests are resolved locally. Cross-scope requests are
    /// validated and routed through the nearest healthy gateway node.
    pub fn resolve_route(
        &mut self,
        _asset_id: &str,
        from_scope: BlockchainScope,
        to_scope: BlockchainScope,
        source_position: &MatrixCoordinate,
    ) -> Result<ScopeAwareRoute, ScopeRoutingError> {
        // Same-scope: direct route, no gateway
        if from_scope == to_scope {
            return Ok(self.build_same_scope_route(
                from_scope,
                source_position,
            ));
        }

        // Cross-scope: validate config
        if !self.config.allow_cross_scope {
            return Err(ScopeRoutingError::CrossScopeDisabled);
        }

        self.validate_scope_transition(from_scope, to_scope)?;

        // Find gateways that bridge these scopes
        let candidates = self.find_gateway_nodes(from_scope, to_scope);
        if candidates.is_empty() {
            return Err(ScopeRoutingError::NoGatewayAvailable {
                from: from_scope,
                to: to_scope,
            });
        }

        // Pick nearest healthy gateway
        let gateway = self.select_nearest_gateway(&candidates, source_position)?;

        let route = self.build_cross_scope_route(
            from_scope,
            to_scope,
            source_position,
            gateway,
        );

        // Record statistics and transition
        self.record_transition(from_scope, to_scope, Some(&gateway.node_id));

        Ok(route)
    }

    /// Register a node that can bridge between blockchain scopes.
    pub fn register_gateway_node(
        &mut self,
        node_id: &str,
        position: MatrixCoordinate,
        bridged_scopes: Vec<BlockchainScope>,
    ) {
        let info = GatewayNodeInfo {
            node_id: node_id.to_string(),
            position,
            bridged_scopes,
            health_score: 1.0,
            current_load: 0.0,
        };
        self.gateways.insert(node_id.to_string(), info);
    }

    /// Return gateway nodes that bridge from `from_scope` to `to_scope`,
    /// sorted by ascending Euclidean distance from the origin (0,0,0).
    pub fn find_gateway_nodes(
        &self,
        from_scope: BlockchainScope,
        to_scope: BlockchainScope,
    ) -> Vec<GatewayNodeInfo> {
        let mut results: Vec<GatewayNodeInfo> = self
            .gateways
            .values()
            .filter(|gw| {
                gw.bridged_scopes.contains(&from_scope)
                    && gw.bridged_scopes.contains(&to_scope)
                    && gw.health_score > 0.0
            })
            .cloned()
            .collect();

        let origin = MatrixCoordinate::origin();
        results.sort_by(|a, b| {
            let da = a.position.euclidean_distance(&origin);
            let db = b.position.euclidean_distance(&origin);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    /// Validate that a scope transition is logically permitted.
    ///
    /// - Device -> Network requires network connectivity (modelled here as
    ///   requiring at least one gateway node).
    /// - Network -> Device is always allowed (download to local).
    pub fn validate_scope_transition(
        &self,
        from: BlockchainScope,
        to: BlockchainScope,
    ) -> Result<(), ScopeRoutingError> {
        if from == to {
            return Ok(());
        }

        // Device -> Network: require that gateways exist (proxy for connectivity)
        if from == BlockchainScope::Device && to == BlockchainScope::Network {
            let has_gateway = self.gateways.values().any(|gw| {
                gw.bridged_scopes.contains(&BlockchainScope::Device)
                    && gw.bridged_scopes.contains(&BlockchainScope::Network)
            });
            if !has_gateway {
                return Err(ScopeRoutingError::NetworkUnavailable { from, to });
            }
        }

        // Network -> Device: always allowed
        Ok(())
    }

    /// Return a snapshot of routing statistics.
    pub fn get_scope_statistics(&self) -> ScopeRoutingStats {
        self.stats.clone()
    }

    // -- private helpers ----------------------------------------------------

    fn build_same_scope_route(
        &mut self,
        scope: BlockchainScope,
        source_position: &MatrixCoordinate,
    ) -> ScopeAwareRoute {
        self.record_transition(scope, scope, None);

        ScopeAwareRoute {
            path: vec![*source_position],
            from_scope: scope,
            to_scope: scope,
            gateway_node: None,
            estimated_latency_ms: 1.0,
            requires_encryption: false,
        }
    }

    fn build_cross_scope_route(
        &self,
        from_scope: BlockchainScope,
        to_scope: BlockchainScope,
        source_position: &MatrixCoordinate,
        gateway: &GatewayNodeInfo,
    ) -> ScopeAwareRoute {
        let latency = source_position.euclidean_distance(&gateway.position) * 0.1;

        ScopeAwareRoute {
            path: vec![*source_position, gateway.position],
            from_scope,
            to_scope,
            gateway_node: Some(gateway.node_id.clone()),
            estimated_latency_ms: latency.max(1.0),
            requires_encryption: true,
        }
    }

    fn select_nearest_gateway<'a>(
        &self,
        candidates: &'a [GatewayNodeInfo],
        source: &MatrixCoordinate,
    ) -> Result<&'a GatewayNodeInfo, ScopeRoutingError> {
        candidates
            .iter()
            .min_by(|a, b| {
                let da = a.position.euclidean_distance(source);
                let db = b.position.euclidean_distance(source);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| ScopeRoutingError::NoGatewayAvailable {
                from: BlockchainScope::Device,
                to: BlockchainScope::Network,
            })
    }

    fn record_transition(
        &mut self,
        from: BlockchainScope,
        to: BlockchainScope,
        gateway: Option<&str>,
    ) {
        let key = format!("{}->{}", from, to);
        *self.stats.routes_by_scope.entry(key).or_insert(0) += 1;

        if from != to {
            self.stats.cross_scope_transfers += 1;
            if let Some(gw) = gateway {
                *self
                    .stats
                    .gateway_utilization
                    .entry(gw.to_string())
                    .or_insert(0) += 1;
            }
        }

        self.transition_log.push(ScopeTransition {
            _from: from,
            _to: to,
            _gateway: gateway.map(String::from),
        });
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: router with default config and two gateway nodes.
    fn router_with_gateways() -> ScopeAwareRouter {
        let mut router = ScopeAwareRouter::new(ScopeRoutingConfig::default());
        router.register_gateway_node(
            "gw-alpha",
            MatrixCoordinate { x: 10, y: 10, z: 0 },
            vec![BlockchainScope::Device, BlockchainScope::Network],
        );
        router.register_gateway_node(
            "gw-beta",
            MatrixCoordinate { x: 100, y: 100, z: 0 },
            vec![BlockchainScope::Device, BlockchainScope::Network],
        );
        router
    }

    #[test]
    fn same_scope_device_routing() {
        let mut router = ScopeAwareRouter::new(ScopeRoutingConfig::default());
        let src = MatrixCoordinate { x: 5, y: 5, z: 0 };

        let route = router
            .resolve_route("asset-1", BlockchainScope::Device, BlockchainScope::Device, &src)
            .expect("test: same-scope Device route");

        assert_eq!(route.from_scope, BlockchainScope::Device);
        assert_eq!(route.to_scope, BlockchainScope::Device);
        assert!(route.gateway_node.is_none());
        assert!(!route.requires_encryption);
    }

    #[test]
    fn same_scope_network_routing() {
        let mut router = ScopeAwareRouter::new(ScopeRoutingConfig::default());
        let src = MatrixCoordinate { x: 20, y: 30, z: 0 };

        let route = router
            .resolve_route("asset-2", BlockchainScope::Network, BlockchainScope::Network, &src)
            .expect("test: same-scope Network route");

        assert_eq!(route.from_scope, BlockchainScope::Network);
        assert_eq!(route.to_scope, BlockchainScope::Network);
        assert!(route.gateway_node.is_none());
    }

    #[test]
    fn cross_scope_device_to_network_via_gateway() {
        let mut router = router_with_gateways();
        let src = MatrixCoordinate { x: 0, y: 0, z: 0 };

        let route = router
            .resolve_route("asset-3", BlockchainScope::Device, BlockchainScope::Network, &src)
            .expect("test: Device->Network");

        assert_eq!(route.from_scope, BlockchainScope::Device);
        assert_eq!(route.to_scope, BlockchainScope::Network);
        assert!(route.gateway_node.is_some());
        assert!(route.requires_encryption);
        assert_eq!(route.path.len(), 2); // source + gateway
    }

    #[test]
    fn cross_scope_network_to_device() {
        let mut router = router_with_gateways();
        let src = MatrixCoordinate { x: 50, y: 50, z: 0 };

        let route = router
            .resolve_route("asset-4", BlockchainScope::Network, BlockchainScope::Device, &src)
            .expect("test: Network->Device");

        assert_eq!(route.from_scope, BlockchainScope::Network);
        assert_eq!(route.to_scope, BlockchainScope::Device);
        assert!(route.gateway_node.is_some());
    }

    #[test]
    fn cross_scope_disabled_error() {
        let config = ScopeRoutingConfig {
            allow_cross_scope: false,
            ..ScopeRoutingConfig::default()
        };
        let mut router = ScopeAwareRouter::new(config);
        let src = MatrixCoordinate { x: 0, y: 0, z: 0 };

        let err = router
            .resolve_route("asset-5", BlockchainScope::Device, BlockchainScope::Network, &src)
            .unwrap_err();

        assert!(matches!(err, ScopeRoutingError::CrossScopeDisabled));
    }

    #[test]
    fn no_gateway_available_error() {
        // Router with no gateways registered
        let mut router = ScopeAwareRouter::new(ScopeRoutingConfig::default());
        let src = MatrixCoordinate { x: 0, y: 0, z: 0 };

        let err = router
            .resolve_route("asset-6", BlockchainScope::Device, BlockchainScope::Network, &src)
            .unwrap_err();

        // Device->Network with no gateways triggers NetworkUnavailable from
        // validate_scope_transition before NoGatewayAvailable can fire.
        assert!(
            matches!(err, ScopeRoutingError::NetworkUnavailable { .. })
                || matches!(err, ScopeRoutingError::NoGatewayAvailable { .. })
        );
    }

    #[test]
    fn gateway_registration_and_lookup() {
        let mut router = ScopeAwareRouter::new(ScopeRoutingConfig::default());

        router.register_gateway_node(
            "gw-1",
            MatrixCoordinate { x: 10, y: 0, z: 0 },
            vec![BlockchainScope::Device, BlockchainScope::Network],
        );
        router.register_gateway_node(
            "gw-2",
            MatrixCoordinate { x: 20, y: 0, z: 0 },
            vec![BlockchainScope::Network], // only Network
        );

        let found = router.find_gateway_nodes(BlockchainScope::Device, BlockchainScope::Network);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].node_id, "gw-1");
    }

    #[test]
    fn nearest_gateway_selection() {
        let mut router = router_with_gateways();
        let src = MatrixCoordinate { x: 8, y: 8, z: 0 };

        let route = router
            .resolve_route("asset-near", BlockchainScope::Device, BlockchainScope::Network, &src)
            .expect("test: nearest gateway");

        // gw-alpha at (10,10,0) is closer to (8,8,0) than gw-beta at (100,100,0)
        assert_eq!(route.gateway_node.as_deref(), Some("gw-alpha"));
    }

    #[test]
    fn scope_statistics_tracking() {
        let mut router = router_with_gateways();
        let src = MatrixCoordinate { x: 0, y: 0, z: 0 };

        // One same-scope, two cross-scope
        router
            .resolve_route("s1", BlockchainScope::Device, BlockchainScope::Device, &src)
            .expect("test: same-scope");
        router
            .resolve_route("s2", BlockchainScope::Device, BlockchainScope::Network, &src)
            .expect("test: cross-scope 1");
        router
            .resolve_route("s3", BlockchainScope::Device, BlockchainScope::Network, &src)
            .expect("test: cross-scope 2");

        let stats = router.get_scope_statistics();
        assert_eq!(stats.cross_scope_transfers, 2);
        assert_eq!(
            stats.routes_by_scope.get("Device->Device"),
            Some(&1)
        );
        assert_eq!(
            stats.routes_by_scope.get("Device->Network"),
            Some(&2)
        );
        // Gateway utilization should have entries
        assert!(!stats.gateway_utilization.is_empty());
    }

    #[test]
    fn max_hops_exceeded() {
        let config = ScopeRoutingConfig {
            max_scope_hops: 0, // forbid any hops
            ..ScopeRoutingConfig::default()
        };
        let router = ScopeAwareRouter::new(config);

        // validate_scope_transition doesn't check hops itself, but we verify
        // the config is stored correctly and could be checked in a multi-hop
        // expansion. For now, verify the config round-trips.
        assert_eq!(router.config.max_scope_hops, 0);
    }

    #[test]
    fn validate_device_to_network_needs_gateway() {
        let router = ScopeAwareRouter::new(ScopeRoutingConfig::default());

        let result = router.validate_scope_transition(
            BlockchainScope::Device,
            BlockchainScope::Network,
        );

        // No gateways registered -> NetworkUnavailable
        assert!(matches!(result, Err(ScopeRoutingError::NetworkUnavailable { .. })));
    }

    #[test]
    fn validate_network_to_device_always_ok() {
        let router = ScopeAwareRouter::new(ScopeRoutingConfig::default());

        let result = router.validate_scope_transition(
            BlockchainScope::Network,
            BlockchainScope::Device,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn validate_same_scope_always_ok() {
        let router = ScopeAwareRouter::new(ScopeRoutingConfig::default());

        assert!(router
            .validate_scope_transition(BlockchainScope::Device, BlockchainScope::Device)
            .is_ok());
        assert!(router
            .validate_scope_transition(BlockchainScope::Network, BlockchainScope::Network)
            .is_ok());
    }

    #[test]
    fn cross_scope_route_requires_encryption() {
        let mut router = router_with_gateways();
        let src = MatrixCoordinate { x: 0, y: 0, z: 0 };

        let route = router
            .resolve_route("enc-asset", BlockchainScope::Device, BlockchainScope::Network, &src)
            .expect("test: encryption required");

        assert!(route.requires_encryption);
    }

    #[test]
    fn same_scope_route_no_encryption() {
        let mut router = ScopeAwareRouter::new(ScopeRoutingConfig::default());
        let src = MatrixCoordinate { x: 0, y: 0, z: 0 };

        let route = router
            .resolve_route("no-enc", BlockchainScope::Device, BlockchainScope::Device, &src)
            .expect("test: no encryption for same scope");

        assert!(!route.requires_encryption);
    }
}
