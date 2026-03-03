// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tracing::debug;

/// A dashboard route mapping a path prefix to a HyperMesh backend.
#[derive(Debug, Clone)]
pub struct DashboardRoute {
    /// URL path prefix (e.g. `/dashboard`).
    pub path_prefix: String,
    /// Human-readable name shown in discovery endpoints.
    pub display_name: String,
    /// Short description of the route.
    pub description: String,
    /// Logical backend name that serves this route.
    pub backend_name: String,
}

/// Manages inbound clearnet-to-HyperMesh routing through dashboard paths.
pub struct InboundProxy {
    routes: Vec<DashboardRoute>,
    stats: Arc<InboundStats>,
}

struct InboundStats {
    requests_routed: AtomicU64,
    requests_unmatched: AtomicU64,
}

impl InboundProxy {
    /// Create an inbound proxy with a custom set of dashboard routes.
    pub fn new(routes: Vec<DashboardRoute>) -> Self {
        debug!(route_count = routes.len(), "inbound proxy initialized");
        Self {
            routes,
            stats: Arc::new(InboundStats {
                requests_routed: AtomicU64::new(0),
                requests_unmatched: AtomicU64::new(0),
            }),
        }
    }

    /// Create an inbound proxy with the default HyperMesh dashboard routes.
    pub fn with_defaults() -> Self {
        Self::new(vec![
            DashboardRoute {
                path_prefix: "/dashboard".into(),
                display_name: "Resource Dashboard".into(),
                description: "HyperMesh resource management panel".into(),
                backend_name: "blockmatrix".into(),
            },
            DashboardRoute {
                path_prefix: "/engauge".into(),
                display_name: "engauge Panel".into(),
                description: "Engagement metrics and analytics".into(),
                backend_name: "engauge".into(),
            },
            DashboardRoute {
                path_prefix: "/caesar".into(),
                display_name: "Caesar Wallet".into(),
                description: "Caesar EVP wallet interface".into(),
                backend_name: "caesar".into(),
            },
            DashboardRoute {
                path_prefix: "/catalog".into(),
                display_name: "Catalog Browser".into(),
                description: "Asset package registry browser".into(),
                backend_name: "catalog".into(),
            },
        ])
    }

    /// Find the dashboard route whose prefix matches the given request path.
    ///
    /// Routes are checked in registration order; the first match wins.
    pub fn match_route(&self, path: &str) -> Option<&DashboardRoute> {
        for route in &self.routes {
            if path.starts_with(&route.path_prefix) {
                self.stats.requests_routed.fetch_add(1, Ordering::Relaxed);
                debug!(path, backend = %route.backend_name, "inbound route matched");
                return Some(route);
            }
        }
        self.stats
            .requests_unmatched
            .fetch_add(1, Ordering::Relaxed);
        None
    }

    /// List all registered dashboard routes (useful for discovery/health endpoints).
    pub fn list_routes(&self) -> Vec<DashboardRoute> {
        self.routes.clone()
    }

    /// Number of registered dashboard routes.
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    /// Get a snapshot of inbound proxy statistics.
    pub fn inbound_stats(&self) -> InboundStatsSnapshot {
        InboundStatsSnapshot {
            requests_routed: self.stats.requests_routed.load(Ordering::Relaxed),
            requests_unmatched: self.stats.requests_unmatched.load(Ordering::Relaxed),
            registered_routes: self.routes.len(),
        }
    }
}

/// A point-in-time snapshot of inbound proxy statistics.
#[derive(Debug, Clone)]
pub struct InboundStatsSnapshot {
    pub requests_routed: u64,
    pub requests_unmatched: u64,
    pub registered_routes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_routes_populated() {
        let proxy = InboundProxy::with_defaults();
        assert_eq!(proxy.route_count(), 4);

        let names: Vec<&str> = proxy
            .routes
            .iter()
            .map(|r| r.display_name.as_str())
            .collect();
        assert!(names.contains(&"Resource Dashboard"));
        assert!(names.contains(&"engauge Panel"));
        assert!(names.contains(&"Caesar Wallet"));
        assert!(names.contains(&"Catalog Browser"));
    }

    #[test]
    fn match_route_dashboard() {
        let proxy = InboundProxy::with_defaults();
        let route = proxy.match_route("/dashboard/overview");
        assert!(route.is_some());
        let route = route.expect("test: checked above");
        assert_eq!(route.backend_name, "blockmatrix");
        assert_eq!(route.display_name, "Resource Dashboard");
    }

    #[test]
    fn match_route_engauge() {
        let proxy = InboundProxy::with_defaults();
        let route = proxy.match_route("/engauge/metrics");
        assert!(route.is_some());
        assert_eq!(
            route.expect("test: checked above").display_name,
            "engauge Panel"
        );
    }

    #[test]
    fn match_route_caesar() {
        let proxy = InboundProxy::with_defaults();
        let route = proxy.match_route("/caesar/wallet");
        assert!(route.is_some());
        let route = route.expect("test: checked above");
        assert_eq!(route.display_name, "Caesar Wallet");
        assert_eq!(route.backend_name, "caesar");
    }

    #[test]
    fn match_route_catalog() {
        let proxy = InboundProxy::with_defaults();
        let route = proxy.match_route("/catalog/browse");
        assert!(route.is_some());
        let route = route.expect("test: checked above");
        assert_eq!(route.display_name, "Catalog Browser");
        assert_eq!(route.backend_name, "catalog");
    }

    #[test]
    fn no_match_returns_none() {
        let proxy = InboundProxy::with_defaults();
        assert!(proxy.match_route("/unknown/path").is_none());
        assert!(proxy.match_route("/api/v1/something").is_none());
    }

    #[test]
    fn list_routes_returns_all() {
        let proxy = InboundProxy::with_defaults();
        let listed = proxy.list_routes();
        assert_eq!(listed.len(), 4);
    }

    #[test]
    fn stats_tracking() {
        let proxy = InboundProxy::with_defaults();

        // Two matches, one miss.
        let _ = proxy.match_route("/dashboard");
        let _ = proxy.match_route("/caesar");
        let _ = proxy.match_route("/nope");

        let stats = proxy.inbound_stats();
        assert_eq!(stats.requests_routed, 2);
        assert_eq!(stats.requests_unmatched, 1);
        assert_eq!(stats.registered_routes, 4);
    }

    #[test]
    fn custom_routes() {
        let proxy = InboundProxy::new(vec![DashboardRoute {
            path_prefix: "/custom".into(),
            display_name: "Custom Panel".into(),
            description: "A custom panel".into(),
            backend_name: "custom-backend".into(),
        }]);

        assert_eq!(proxy.route_count(), 1);
        let route = proxy.match_route("/custom/page");
        assert!(route.is_some());
        assert_eq!(
            route.expect("test: checked above").backend_name,
            "custom-backend"
        );
    }

    #[test]
    fn initial_stats_are_zero() {
        let proxy = InboundProxy::with_defaults();
        let stats = proxy.inbound_stats();
        assert_eq!(stats.requests_routed, 0);
        assert_eq!(stats.requests_unmatched, 0);
    }
}
