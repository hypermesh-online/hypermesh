// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use dashmap::DashMap;
use tracing::debug;

/// A domain-to-backend mapping.
#[derive(Debug, Clone)]
pub struct DomainRoute {
    pub domain: String,
    pub backend_addr: SocketAddr,
    pub backend_name: String,
}

/// Routes requests by domain (SNI or Host header).
pub struct DomainRouter {
    /// Exact domain matches.
    exact_routes: Arc<DashMap<String, DomainRoute>>,
    /// Wildcard domain matches (e.g., "*.hypermesh.online").
    wildcard_routes: Arc<Vec<(String, DomainRoute)>>,
    stats: Arc<DomainRouterStats>,
}

struct DomainRouterStats {
    exact_hits: AtomicU64,
    wildcard_hits: AtomicU64,
    misses: AtomicU64,
}

impl Default for DomainRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl DomainRouter {
    /// Create a new empty domain router.
    pub fn new() -> Self {
        Self {
            exact_routes: Arc::new(DashMap::new()),
            wildcard_routes: Arc::new(Vec::new()),
            stats: Arc::new(DomainRouterStats {
                exact_hits: AtomicU64::new(0),
                wildcard_hits: AtomicU64::new(0),
                misses: AtomicU64::new(0),
            }),
        }
    }

    /// Create a domain router with pre-populated wildcard routes.
    pub fn with_wildcards(wildcards: Vec<(String, DomainRoute)>) -> Self {
        Self {
            exact_routes: Arc::new(DashMap::new()),
            wildcard_routes: Arc::new(wildcards),
            stats: Arc::new(DomainRouterStats {
                exact_hits: AtomicU64::new(0),
                wildcard_hits: AtomicU64::new(0),
                misses: AtomicU64::new(0),
            }),
        }
    }

    /// Register an exact domain route.
    pub fn add_route(&self, route: DomainRoute) {
        debug!(domain = %route.domain, backend = %route.backend_name, "registered domain route");
        self.exact_routes.insert(route.domain.clone(), route);
    }

    /// Register routes from a list of backend configurations.
    ///
    /// Each backend may declare multiple domains; an exact route is added for each.
    pub fn register_backends(&self, backends: &[crate::gateway_mode::BackendConfig]) {
        for backend in backends {
            for domain in &backend.domains {
                let route = DomainRoute {
                    domain: domain.clone(),
                    backend_addr: backend.addr,
                    backend_name: backend.name.clone(),
                };
                debug!(
                    domain = %domain,
                    backend = %backend.name,
                    "registered backend domain route"
                );
                self.exact_routes.insert(domain.clone(), route);
            }
        }
    }

    /// Route by SNI server name.
    ///
    /// Tries an exact match first, then falls back to wildcard patterns.
    pub fn route_by_sni(&self, sni: &str) -> Option<DomainRoute> {
        // Try exact match first.
        if let Some(route) = self.exact_routes.get(sni) {
            self.stats.exact_hits.fetch_add(1, Ordering::Relaxed);
            return Some(route.clone());
        }

        // Try wildcard match (e.g., "foo.hypermesh.online" matches "*.hypermesh.online").
        for (pattern, route) in self.wildcard_routes.iter() {
            if matches_wildcard(pattern, sni) {
                self.stats.wildcard_hits.fetch_add(1, Ordering::Relaxed);
                return Some(route.clone());
            }
        }

        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Route by Host header value (strips port if present, then delegates to SNI lookup).
    pub fn route_by_host(&self, host: &str) -> Option<DomainRoute> {
        let domain = host.split(':').next().unwrap_or(host);
        self.route_by_sni(domain)
    }

    /// Snapshot of routing statistics.
    pub fn stats(&self) -> DomainRouterStatsSnapshot {
        DomainRouterStatsSnapshot {
            exact_hits: self.stats.exact_hits.load(Ordering::Relaxed),
            wildcard_hits: self.stats.wildcard_hits.load(Ordering::Relaxed),
            misses: self.stats.misses.load(Ordering::Relaxed),
            registered_routes: self.exact_routes.len(),
        }
    }

    /// Number of exact routes currently registered.
    pub fn route_count(&self) -> usize {
        self.exact_routes.len()
    }

    /// Number of wildcard patterns registered.
    pub fn wildcard_count(&self) -> usize {
        self.wildcard_routes.len()
    }

    /// Resolve a hostname via blockchain DNS records and return a route decision.
    ///
    /// Extracts the subdomain from the Host header (e.g., `"persist"` from
    /// `"persist.hypermesh.online"`) and looks it up in the provided DNS records.
    /// Returns `None` if the host has no subdomain or no matching record exists.
    pub fn resolve_from_dns(
        &self,
        host: &str,
        dns_records: &HashMap<String, SocketAddr>,
    ) -> Option<DomainRoute> {
        let subdomain = extract_subdomain(host)?;
        let addr = dns_records.get(&subdomain)?;
        Some(DomainRoute {
            domain: subdomain.clone(),
            backend_addr: *addr,
            backend_name: subdomain,
        })
    }
}

/// Extract the first subdomain label from a full hostname.
///
/// Strips any trailing port (`host:port`) before splitting. Returns the first
/// label when the hostname has three or more dot-separated parts, otherwise
/// returns `None` (bare domain like `"hypermesh.online"` has no subdomain).
///
/// # Examples
///
/// - `"persist.hypermesh.online"` -> `Some("persist")`
/// - `"persist.hypermesh.online:8443"` -> `Some("persist")`
/// - `"a.b.hypermesh.online"` -> `Some("a")`
/// - `"hypermesh.online"` -> `None`
/// - `"localhost"` -> `None`
pub fn extract_subdomain(host: &str) -> Option<String> {
    let hostname = host.split(':').next().unwrap_or(host);
    let parts: Vec<&str> = hostname.split('.').collect();
    if parts.len() >= 3 {
        Some(parts[0].to_string())
    } else {
        None
    }
}

/// Check if a domain matches a wildcard pattern (e.g., `*.hypermesh.online`).
///
/// Rules:
/// - `*.example.com` matches `foo.example.com` but NOT `example.com` and NOT
///   `bar.foo.example.com` (single label substitution).
/// - A non-wildcard pattern is treated as an exact match.
fn matches_wildcard(pattern: &str, domain: &str) -> bool {
    if let Some(suffix) = pattern.strip_prefix("*.") {
        // Domain must end with ".suffix" and have exactly one additional label.
        domain.ends_with(suffix)
            && domain.len() > suffix.len() + 1
            && domain.as_bytes()[domain.len() - suffix.len() - 1] == b'.'
            && !domain[..domain.len() - suffix.len() - 1].contains('.')
    } else {
        pattern == domain
    }
}

/// A point-in-time snapshot of routing statistics.
#[derive(Debug, Clone)]
pub struct DomainRouterStatsSnapshot {
    pub exact_hits: u64,
    pub wildcard_hits: u64,
    pub misses: u64,
    pub registered_routes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway_mode::{BackendConfig, BackendProtocol};

    fn test_addr() -> SocketAddr {
        "[::1]:8080".parse().expect("test: valid addr")
    }

    #[test]
    fn exact_domain_match() {
        let router = DomainRouter::new();
        router.add_route(DomainRoute {
            domain: "trust.hypermesh.online".into(),
            backend_addr: test_addr(),
            backend_name: "trustchain".into(),
        });

        let result = router.route_by_sni("trust.hypermesh.online");
        assert!(result.is_some());
        let route = result.expect("test: checked above");
        assert_eq!(route.backend_name, "trustchain");
        assert_eq!(route.backend_addr, test_addr());
    }

    #[test]
    fn no_match_returns_none() {
        let router = DomainRouter::new();
        router.add_route(DomainRoute {
            domain: "trust.hypermesh.online".into(),
            backend_addr: test_addr(),
            backend_name: "trustchain".into(),
        });

        assert!(router.route_by_sni("unknown.example.com").is_none());
    }

    #[test]
    fn host_header_with_port_stripped() {
        let router = DomainRouter::new();
        router.add_route(DomainRoute {
            domain: "trust.hypermesh.online".into(),
            backend_addr: test_addr(),
            backend_name: "trustchain".into(),
        });

        let result = router.route_by_host("trust.hypermesh.online:8443");
        assert!(result.is_some());
        assert_eq!(
            result.expect("test: checked above").backend_name,
            "trustchain"
        );
    }

    #[test]
    fn wildcard_matching() {
        let wildcard_route = DomainRoute {
            domain: "*.hypermesh.online".into(),
            backend_addr: test_addr(),
            backend_name: "wildcard-backend".into(),
        };
        let router =
            DomainRouter::with_wildcards(vec![("*.hypermesh.online".into(), wildcard_route)]);

        // Single-label subdomain matches.
        let result = router.route_by_sni("foo.hypermesh.online");
        assert!(result.is_some());
        assert_eq!(
            result.expect("test: checked above").backend_name,
            "wildcard-backend"
        );

        // Bare domain does NOT match wildcard.
        assert!(router.route_by_sni("hypermesh.online").is_none());

        // Multi-label subdomain does NOT match (single label only).
        assert!(router.route_by_sni("a.b.hypermesh.online").is_none());
    }

    #[test]
    fn register_backends_from_config() {
        let router = DomainRouter::new();
        let backends = vec![
            BackendConfig {
                name: "trustchain".into(),
                addr: "[::1]:50053".parse().expect("test: valid addr"),
                server_name: "trustchain".into(),
                protocol: BackendProtocol::Http3,
                path_prefixes: vec!["/api/v1/trustchain".into()],
                domains: vec![
                    "trust.hypermesh.online".into(),
                    "ca.hypermesh.online".into(),
                ],
            },
            BackendConfig {
                name: "blockmatrix".into(),
                addr: "[::1]:8446".parse().expect("test: valid addr"),
                server_name: "blockmatrix".into(),
                protocol: BackendProtocol::Http3,
                path_prefixes: vec!["/api/v1/blockmatrix".into()],
                domains: vec!["hypermesh.online".into()],
            },
        ];

        router.register_backends(&backends);
        assert_eq!(router.route_count(), 3);

        let tc = router.route_by_sni("trust.hypermesh.online");
        assert!(tc.is_some());
        assert_eq!(tc.expect("test: checked above").backend_name, "trustchain");

        let ca = router.route_by_sni("ca.hypermesh.online");
        assert!(ca.is_some());
        assert_eq!(ca.expect("test: checked above").backend_name, "trustchain");

        let bm = router.route_by_sni("hypermesh.online");
        assert!(bm.is_some());
        assert_eq!(bm.expect("test: checked above").backend_name, "blockmatrix");
    }

    #[test]
    fn stats_tracking() {
        let router = DomainRouter::new();
        router.add_route(DomainRoute {
            domain: "a.com".into(),
            backend_addr: test_addr(),
            backend_name: "a".into(),
        });

        // Generate some hits and misses.
        let _ = router.route_by_sni("a.com"); // exact hit
        let _ = router.route_by_sni("a.com"); // exact hit
        let _ = router.route_by_sni("b.com"); // miss

        let snap = router.stats();
        assert_eq!(snap.exact_hits, 2);
        assert_eq!(snap.misses, 1);
        assert_eq!(snap.registered_routes, 1);
    }

    #[test]
    fn empty_router_returns_none() {
        let router = DomainRouter::new();
        assert!(router.route_by_sni("anything.com").is_none());
        assert!(router.route_by_host("anything.com:443").is_none());
        assert_eq!(router.route_count(), 0);

        let snap = router.stats();
        assert_eq!(snap.misses, 2);
    }

    #[test]
    fn wildcard_stats_tracked() {
        let wildcard_route = DomainRoute {
            domain: "*.example.com".into(),
            backend_addr: test_addr(),
            backend_name: "wc".into(),
        };
        let router = DomainRouter::with_wildcards(vec![("*.example.com".into(), wildcard_route)]);

        let _ = router.route_by_sni("sub.example.com");
        let snap = router.stats();
        assert_eq!(snap.wildcard_hits, 1);
        assert_eq!(snap.exact_hits, 0);
    }

    #[test]
    fn matches_wildcard_edge_cases() {
        // Exact pattern (no wildcard prefix).
        assert!(matches_wildcard("example.com", "example.com"));
        assert!(!matches_wildcard("example.com", "other.com"));

        // Wildcard must not match bare domain.
        assert!(!matches_wildcard("*.example.com", "example.com"));

        // Wildcard single label only.
        assert!(matches_wildcard("*.example.com", "sub.example.com"));
        assert!(!matches_wildcard("*.example.com", "a.b.example.com"));
    }

    // ===== extract_subdomain (5 tests) ====================================

    #[test]
    fn extract_subdomain_from_three_part_host() {
        assert_eq!(
            extract_subdomain("persist.hypermesh.online"),
            Some("persist".to_string()),
        );
    }

    #[test]
    fn extract_subdomain_strips_port() {
        assert_eq!(
            extract_subdomain("persist.hypermesh.online:8443"),
            Some("persist".to_string()),
        );
    }

    #[test]
    fn extract_subdomain_multi_level_returns_first() {
        assert_eq!(
            extract_subdomain("a.b.hypermesh.online"),
            Some("a".to_string()),
        );
    }

    #[test]
    fn extract_subdomain_bare_domain_returns_none() {
        assert_eq!(extract_subdomain("hypermesh.online"), None);
    }

    #[test]
    fn extract_subdomain_single_label_returns_none() {
        assert_eq!(extract_subdomain("localhost"), None);
    }

    // ===== resolve_from_dns (3 tests) =====================================

    #[test]
    fn resolve_from_dns_known_subdomain() {
        let router = DomainRouter::new();
        let mut records = HashMap::new();
        records.insert(
            "persist".to_string(),
            "[::1]:8080".parse().expect("test: valid addr"),
        );

        let route = router.resolve_from_dns("persist.hypermesh.online", &records);
        assert!(route.is_some());
        let route = route.expect("test: checked above");
        assert_eq!(route.domain, "persist");
        assert_eq!(route.backend_name, "persist");
        assert_eq!(
            route.backend_addr,
            "[::1]:8080".parse::<SocketAddr>().expect("test: valid addr"),
        );
    }

    #[test]
    fn resolve_from_dns_unknown_subdomain_returns_none() {
        let router = DomainRouter::new();
        let records = HashMap::new();
        let route = router.resolve_from_dns("unknown.hypermesh.online", &records);
        assert!(route.is_none());
    }

    #[test]
    fn resolve_from_dns_bare_domain_returns_none() {
        let router = DomainRouter::new();
        let mut records = HashMap::new();
        records.insert(
            "persist".to_string(),
            "[::1]:8080".parse().expect("test: valid addr"),
        );

        let route = router.resolve_from_dns("hypermesh.online", &records);
        assert!(route.is_none());
    }
}
