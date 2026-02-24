// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Scope-aware STOQ traffic router.
//!
//! Routes requests based on [`BlockchainScope`] (Device vs Network) and
//! [`PrivacyMode`] (Anonymous/Private/Public). Same-scope traffic is routed
//! directly; cross-scope traffic goes through a gateway node that supports
//! both scopes. If no gateway is available, federation routing is attempted.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use dashmap::DashMap;
use tracing::{debug, warn};

use hypermesh_lib::{BlockchainScope, MatrixPosition, PrivacyMode};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A registered gateway node that can bridge between blockchain scopes.
#[derive(Debug, Clone)]
pub struct GatewayNode {
    pub node_id: String,
    pub addr: SocketAddr,
    pub position: MatrixPosition,
    pub supported_scopes: Vec<BlockchainScope>,
    pub trust_level: GatewayTrustLevel,
}

/// Trust classification for a gateway peer. Maps conceptually to
/// `trustchain::ca::federation::FederationTrustLevel` but is purposefully
/// decoupled so the gateway crate does not expose trustchain internals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayTrustLevel {
    /// Accept traffic without additional verification.
    Full,
    /// Accept traffic after extra validation (e.g. CT proof).
    Conditional,
    /// Reject all traffic.
    Untrusted,
}

/// Decision produced by [`ScopeRouter::route`].
#[derive(Debug, Clone)]
pub enum RouteDecision {
    /// Route directly to a target address (same scope).
    Direct { target: SocketAddr },
    /// Route via a gateway node that bridges the two scopes.
    ViaGateway {
        gateway: GatewayNode,
        target_scope: BlockchainScope,
    },
    /// Route via a federation bridge (cross-network).
    ViaFederation { federation_id: String },
    /// Routing denied with a human-readable reason.
    Denied { reason: String },
}

/// Snapshot of routing statistics returned by [`ScopeRouter::stats`].
#[derive(Debug, Clone)]
pub struct ScopeRouterStats {
    pub direct_routes: u64,
    pub gateway_routes: u64,
    pub federation_routes: u64,
    pub denied_routes: u64,
    pub registered_gateways: usize,
    pub registered_federations: usize,
}

// ---------------------------------------------------------------------------
// Internal stats
// ---------------------------------------------------------------------------

struct RouterStats {
    direct_routes: AtomicU64,
    gateway_routes: AtomicU64,
    federation_routes: AtomicU64,
    denied_routes: AtomicU64,
}

impl RouterStats {
    fn new() -> Self {
        Self {
            direct_routes: AtomicU64::new(0),
            gateway_routes: AtomicU64::new(0),
            federation_routes: AtomicU64::new(0),
            denied_routes: AtomicU64::new(0),
        }
    }
}

// ---------------------------------------------------------------------------
// ScopeRouter
// ---------------------------------------------------------------------------

/// Routes STOQ traffic based on [`BlockchainScope`] and [`PrivacyMode`].
///
/// Gateway nodes are registered dynamically and looked up by scope support.
/// When no direct gateway can satisfy a cross-scope request, the router falls
/// back to federation-level forwarding.
pub struct ScopeRouter {
    /// Registered gateway nodes keyed by `node_id`.
    registered_gateways: Arc<DashMap<String, GatewayNode>>,
    /// Federation trust levels keyed by `federation_id`.
    federation_trust: Arc<DashMap<String, GatewayTrustLevel>>,
    /// The scope of the local node (used for context, e.g. logging).
    _local_scope: BlockchainScope,
    /// Atomic counters.
    stats: Arc<RouterStats>,
}

impl ScopeRouter {
    /// Create a new router for the given local blockchain scope.
    pub fn new(local_scope: BlockchainScope) -> Self {
        Self {
            registered_gateways: Arc::new(DashMap::new()),
            federation_trust: Arc::new(DashMap::new()),
            _local_scope: local_scope,
            stats: Arc::new(RouterStats::new()),
        }
    }

    /// Determine how to route traffic from `from_scope` to `to_scope` under
    /// the given `privacy_mode`.
    ///
    /// # Routing rules
    ///
    /// 1. **Same scope** -- pick the nearest registered gateway that supports
    ///    the scope and route directly. Deny if none found.
    /// 2. **Cross-scope + Anonymous** -- always denied (anonymous mode cannot
    ///    carry identity needed for scope bridging).
    /// 3. **Cross-scope** -- find a trusted gateway that supports the target
    ///    scope. Fall back to federation routing. Deny if neither available.
    pub fn route(
        &self,
        from_scope: BlockchainScope,
        to_scope: BlockchainScope,
        privacy_mode: PrivacyMode,
    ) -> RouteDecision {
        // Same scope -> direct route
        if from_scope == to_scope {
            if let Some(nearest) = self.find_nearest_gateway(&[from_scope]) {
                self.stats.direct_routes.fetch_add(1, Ordering::Relaxed);
                debug!(
                    "direct route to {} for scope {:?}",
                    nearest.addr, from_scope
                );
                return RouteDecision::Direct {
                    target: nearest.addr,
                };
            }
            self.stats.denied_routes.fetch_add(1, Ordering::Relaxed);
            return RouteDecision::Denied {
                reason: "no target for same-scope route".into(),
            };
        }

        // Cross-scope: anonymous denied -- no identity to validate
        if privacy_mode == PrivacyMode::ANONYMOUS {
            self.stats.denied_routes.fetch_add(1, Ordering::Relaxed);
            warn!("cross-scope routing denied for anonymous mode");
            return RouteDecision::Denied {
                reason: "cross-scope routing denied for anonymous mode".into(),
            };
        }

        // Cross-scope: find a trusted gateway that supports the target scope
        if let Some(gw) = self.find_gateway_for_scope(to_scope) {
            if gw.trust_level == GatewayTrustLevel::Untrusted {
                self.stats.denied_routes.fetch_add(1, Ordering::Relaxed);
                return RouteDecision::Denied {
                    reason: "gateway untrusted".into(),
                };
            }
            self.stats.gateway_routes.fetch_add(1, Ordering::Relaxed);
            debug!(
                "cross-scope via gateway {} -> {:?}",
                gw.node_id, to_scope
            );
            return RouteDecision::ViaGateway {
                gateway: gw,
                target_scope: to_scope,
            };
        }

        // Fallback: federation routing
        if let Some(fed_id) = self.find_federation_for_scope(to_scope) {
            self.stats.federation_routes.fetch_add(1, Ordering::Relaxed);
            debug!("cross-scope via federation {}", fed_id);
            return RouteDecision::ViaFederation {
                federation_id: fed_id,
            };
        }

        self.stats.denied_routes.fetch_add(1, Ordering::Relaxed);
        RouteDecision::Denied {
            reason: format!("no route from {:?} to {:?}", from_scope, to_scope),
        }
    }

    /// Register a gateway node. Overwrites any previous entry with the same
    /// `node_id`.
    pub fn register_gateway(&self, node: GatewayNode) {
        debug!(
            "registered gateway {} at {} (scopes: {:?})",
            node.node_id, node.addr, node.supported_scopes
        );
        self.registered_gateways
            .insert(node.node_id.clone(), node);
    }

    /// Remove a gateway node by ID. Returns `true` if it existed.
    pub fn remove_gateway(&self, node_id: &str) -> bool {
        let removed = self.registered_gateways.remove(node_id).is_some();
        if removed {
            debug!("removed gateway {}", node_id);
        }
        removed
    }

    /// Register (or update) a federation's trust level for routing decisions.
    pub fn register_federation(
        &self,
        federation_id: String,
        trust_level: GatewayTrustLevel,
    ) {
        debug!(
            "registered federation {} with trust {:?}",
            federation_id, trust_level
        );
        self.federation_trust.insert(federation_id, trust_level);
    }

    /// Remove a federation. Returns `true` if it existed.
    pub fn remove_federation(&self, federation_id: &str) -> bool {
        self.federation_trust.remove(federation_id).is_some()
    }

    /// Return a snapshot of routing statistics.
    pub fn stats(&self) -> ScopeRouterStats {
        ScopeRouterStats {
            direct_routes: self.stats.direct_routes.load(Ordering::Relaxed),
            gateway_routes: self.stats.gateway_routes.load(Ordering::Relaxed),
            federation_routes: self.stats.federation_routes.load(Ordering::Relaxed),
            denied_routes: self.stats.denied_routes.load(Ordering::Relaxed),
            registered_gateways: self.registered_gateways.len(),
            registered_federations: self.federation_trust.len(),
        }
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Find the nearest gateway that supports at least one of the given scopes.
    /// "Nearest" is defined by Euclidean distance from the origin (0,0,0).
    fn find_nearest_gateway(&self, scopes: &[BlockchainScope]) -> Option<GatewayNode> {
        let mut best: Option<(f64, GatewayNode)> = None;
        for entry in self.registered_gateways.iter() {
            let gw = entry.value();
            let supports = gw
                .supported_scopes
                .iter()
                .any(|s| scopes.contains(s));
            if !supports {
                continue;
            }
            if gw.trust_level == GatewayTrustLevel::Untrusted {
                continue;
            }
            let dist = euclidean_distance_to_origin(&gw.position);
            if best.as_ref().map_or(true, |(d, _)| dist < *d) {
                best = Some((dist, gw.clone()));
            }
        }
        best.map(|(_, gw)| gw)
    }

    /// Find any trusted gateway that supports `scope`.
    fn find_gateway_for_scope(&self, scope: BlockchainScope) -> Option<GatewayNode> {
        self.find_nearest_gateway(&[scope])
    }

    /// Look for a trusted federation that could handle the target scope.
    fn find_federation_for_scope(
        &self,
        _scope: BlockchainScope,
    ) -> Option<String> {
        // Return the first non-untrusted federation.
        for entry in self.federation_trust.iter() {
            if *entry.value() != GatewayTrustLevel::Untrusted {
                return Some(entry.key().clone());
            }
        }
        None
    }
}

/// Euclidean distance from a [`MatrixPosition`] to the origin (0, 0, 0).
fn euclidean_distance_to_origin(pos: &MatrixPosition) -> f64 {
    (pos.x * pos.x + pos.y * pos.y + pos.z * pos.z).sqrt()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_gateway(
        id: &str,
        addr: &str,
        scopes: Vec<BlockchainScope>,
        trust: GatewayTrustLevel,
        pos: (f64, f64, f64),
    ) -> GatewayNode {
        GatewayNode {
            node_id: id.into(),
            addr: addr.parse().expect("test: valid addr"),
            position: MatrixPosition {
                x: pos.0,
                y: pos.1,
                z: pos.2,
            },
            supported_scopes: scopes,
            trust_level: trust,
        }
    }

    #[test]
    fn same_scope_direct_route() {
        let router = ScopeRouter::new(BlockchainScope::Device);
        router.register_gateway(make_gateway(
            "gw1",
            "[::1]:9000",
            vec![BlockchainScope::Device],
            GatewayTrustLevel::Full,
            (1.0, 0.0, 0.0),
        ));

        let decision = router.route(
            BlockchainScope::Device,
            BlockchainScope::Device,
            PrivacyMode::PRIVATE,
        );
        match decision {
            RouteDecision::Direct { target } => {
                assert_eq!(target, "[::1]:9000".parse::<SocketAddr>().expect("test: valid addr"));
            }
            other => unreachable!("expected Direct, got {:?}", other),
        }
        assert_eq!(router.stats().direct_routes, 1);
    }

    #[test]
    fn same_scope_denied_when_no_gateways() {
        let router = ScopeRouter::new(BlockchainScope::Device);
        let decision = router.route(
            BlockchainScope::Device,
            BlockchainScope::Device,
            PrivacyMode::PRIVATE,
        );
        match decision {
            RouteDecision::Denied { reason } => {
                assert!(reason.contains("no target"), "reason: {}", reason);
            }
            other => unreachable!("expected Denied, got {:?}", other),
        }
        assert_eq!(router.stats().denied_routes, 1);
    }

    #[test]
    fn cross_scope_anonymous_denied() {
        let router = ScopeRouter::new(BlockchainScope::Device);
        router.register_gateway(make_gateway(
            "gw1",
            "[::1]:9000",
            vec![BlockchainScope::Device, BlockchainScope::Network],
            GatewayTrustLevel::Full,
            (1.0, 0.0, 0.0),
        ));

        let decision = router.route(
            BlockchainScope::Device,
            BlockchainScope::Network,
            PrivacyMode::ANONYMOUS,
        );
        match decision {
            RouteDecision::Denied { reason } => {
                assert!(reason.contains("anonymous"), "reason: {}", reason);
            }
            other => unreachable!("expected Denied, got {:?}", other),
        }
        assert_eq!(router.stats().denied_routes, 1);
    }

    #[test]
    fn cross_scope_via_gateway() {
        let router = ScopeRouter::new(BlockchainScope::Device);
        router.register_gateway(make_gateway(
            "bridge",
            "[::1]:9001",
            vec![BlockchainScope::Network],
            GatewayTrustLevel::Full,
            (2.0, 0.0, 0.0),
        ));

        let decision = router.route(
            BlockchainScope::Device,
            BlockchainScope::Network,
            PrivacyMode::PRIVATE,
        );
        match decision {
            RouteDecision::ViaGateway {
                gateway,
                target_scope,
            } => {
                assert_eq!(gateway.node_id, "bridge");
                assert_eq!(target_scope, BlockchainScope::Network);
            }
            other => unreachable!("expected ViaGateway, got {:?}", other),
        }
        assert_eq!(router.stats().gateway_routes, 1);
    }

    #[test]
    fn cross_scope_untrusted_gateway_denied() {
        let router = ScopeRouter::new(BlockchainScope::Device);
        router.register_gateway(make_gateway(
            "bad-gw",
            "[::1]:9002",
            vec![BlockchainScope::Network],
            GatewayTrustLevel::Untrusted,
            (1.0, 0.0, 0.0),
        ));

        let decision = router.route(
            BlockchainScope::Device,
            BlockchainScope::Network,
            PrivacyMode::PUBLIC,
        );
        match decision {
            RouteDecision::Denied { reason } => {
                assert!(
                    reason.contains("no route"),
                    "expected no-route deny, got: {}",
                    reason
                );
            }
            other => unreachable!("expected Denied, got {:?}", other),
        }
    }

    #[test]
    fn cross_scope_fallback_to_federation() {
        let router = ScopeRouter::new(BlockchainScope::Device);
        // No gateways, but a federation is registered
        router.register_federation("fed-alpha".into(), GatewayTrustLevel::Full);

        let decision = router.route(
            BlockchainScope::Device,
            BlockchainScope::Network,
            PrivacyMode::PRIVATE,
        );
        match decision {
            RouteDecision::ViaFederation { federation_id } => {
                assert_eq!(federation_id, "fed-alpha");
            }
            other => unreachable!("expected ViaFederation, got {:?}", other),
        }
        assert_eq!(router.stats().federation_routes, 1);
    }

    #[test]
    fn untrusted_federation_skipped() {
        let router = ScopeRouter::new(BlockchainScope::Device);
        router.register_federation("bad-fed".into(), GatewayTrustLevel::Untrusted);

        let decision = router.route(
            BlockchainScope::Device,
            BlockchainScope::Network,
            PrivacyMode::PUBLIC,
        );
        match decision {
            RouteDecision::Denied { .. } => {}
            other => unreachable!("expected Denied, got {:?}", other),
        }
    }

    #[test]
    fn register_and_remove_gateway() {
        let router = ScopeRouter::new(BlockchainScope::Device);
        router.register_gateway(make_gateway(
            "gw1",
            "[::1]:9000",
            vec![BlockchainScope::Device],
            GatewayTrustLevel::Full,
            (0.0, 0.0, 0.0),
        ));
        assert_eq!(router.stats().registered_gateways, 1);

        assert!(router.remove_gateway("gw1"));
        assert_eq!(router.stats().registered_gateways, 0);
        assert!(!router.remove_gateway("gw1"));
    }

    #[test]
    fn register_and_remove_federation() {
        let router = ScopeRouter::new(BlockchainScope::Network);
        router.register_federation("fed-1".into(), GatewayTrustLevel::Conditional);
        assert_eq!(router.stats().registered_federations, 1);

        assert!(router.remove_federation("fed-1"));
        assert_eq!(router.stats().registered_federations, 0);
        assert!(!router.remove_federation("fed-1"));
    }

    #[test]
    fn nearest_gateway_selected() {
        let router = ScopeRouter::new(BlockchainScope::Device);
        // Far gateway
        router.register_gateway(make_gateway(
            "far",
            "[::1]:9010",
            vec![BlockchainScope::Device],
            GatewayTrustLevel::Full,
            (100.0, 100.0, 100.0),
        ));
        // Near gateway
        router.register_gateway(make_gateway(
            "near",
            "[::1]:9011",
            vec![BlockchainScope::Device],
            GatewayTrustLevel::Full,
            (1.0, 1.0, 1.0),
        ));

        let decision = router.route(
            BlockchainScope::Device,
            BlockchainScope::Device,
            PrivacyMode::PUBLIC,
        );
        match decision {
            RouteDecision::Direct { target } => {
                assert_eq!(target, "[::1]:9011".parse::<SocketAddr>().expect("test: valid addr"));
            }
            other => unreachable!("expected Direct to near gateway, got {:?}", other),
        }
    }

    #[test]
    fn stats_snapshot_accurate() {
        let router = ScopeRouter::new(BlockchainScope::Device);
        let s = router.stats();
        assert_eq!(s.direct_routes, 0);
        assert_eq!(s.gateway_routes, 0);
        assert_eq!(s.federation_routes, 0);
        assert_eq!(s.denied_routes, 0);
        assert_eq!(s.registered_gateways, 0);
        assert_eq!(s.registered_federations, 0);

        // One denied route (no gateways)
        let _ = router.route(
            BlockchainScope::Device,
            BlockchainScope::Device,
            PrivacyMode::PRIVATE,
        );
        let s = router.stats();
        assert_eq!(s.denied_routes, 1);
    }

    #[test]
    fn conditional_gateway_accepted_for_cross_scope() {
        let router = ScopeRouter::new(BlockchainScope::Device);
        router.register_gateway(make_gateway(
            "cond-gw",
            "[::1]:9020",
            vec![BlockchainScope::Network],
            GatewayTrustLevel::Conditional,
            (1.0, 0.0, 0.0),
        ));

        let decision = router.route(
            BlockchainScope::Device,
            BlockchainScope::Network,
            PrivacyMode::PRIVATE,
        );
        match decision {
            RouteDecision::ViaGateway { gateway, .. } => {
                assert_eq!(gateway.trust_level, GatewayTrustLevel::Conditional);
            }
            other => unreachable!("expected ViaGateway, got {:?}", other),
        }
    }

    #[test]
    fn gateway_overwrite_on_re_register() {
        let router = ScopeRouter::new(BlockchainScope::Device);
        router.register_gateway(make_gateway(
            "gw1",
            "[::1]:9000",
            vec![BlockchainScope::Device],
            GatewayTrustLevel::Full,
            (1.0, 0.0, 0.0),
        ));
        // Re-register with different address
        router.register_gateway(make_gateway(
            "gw1",
            "[::1]:9999",
            vec![BlockchainScope::Device],
            GatewayTrustLevel::Full,
            (1.0, 0.0, 0.0),
        ));
        assert_eq!(router.stats().registered_gateways, 1);

        let decision = router.route(
            BlockchainScope::Device,
            BlockchainScope::Device,
            PrivacyMode::PRIVATE,
        );
        match decision {
            RouteDecision::Direct { target } => {
                assert_eq!(target, "[::1]:9999".parse::<SocketAddr>().expect("test: valid addr"));
            }
            other => unreachable!("expected Direct with updated addr, got {:?}", other),
        }
    }

    #[test]
    fn euclidean_distance_calculation() {
        let pos = MatrixPosition {
            x: 3.0,
            y: 4.0,
            z: 0.0,
        };
        let dist = euclidean_distance_to_origin(&pos);
        assert!((dist - 5.0).abs() < 1e-10);
    }
}
