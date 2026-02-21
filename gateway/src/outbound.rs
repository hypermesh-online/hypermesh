// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tracing::{debug, warn};

use crate::error::GatewayError;

/// Allowlist rule governing which outbound requests the gateway will forward.
#[derive(Debug, Clone)]
pub struct OutboundRule {
    /// Domain pattern (exact or wildcard like `*.example.com`).
    pub domain_pattern: String,
    /// Allowed HTTP methods. An empty list means all methods are allowed.
    pub allowed_methods: Vec<String>,
    /// Maximum request payload size in bytes.
    pub max_payload_bytes: usize,
}

impl OutboundRule {
    /// Check if a request matches this rule.
    pub fn matches(&self, domain: &str, method: &str, payload_size: usize) -> bool {
        let domain_match = if self.domain_pattern.starts_with("*.") {
            let suffix = &self.domain_pattern[1..]; // ".example.com"
            domain.ends_with(suffix) || domain == &self.domain_pattern[2..]
        } else {
            domain == self.domain_pattern
        };

        domain_match
            && (self.allowed_methods.is_empty()
                || self.allowed_methods.iter().any(|m| m == method))
            && payload_size <= self.max_payload_bytes
    }
}

/// Manages outbound proxy from HyperMesh STOQ to clearnet HTTP/3.
pub struct OutboundProxy {
    allowlist: Arc<Vec<OutboundRule>>,
    stats: Arc<OutboundStats>,
}

struct OutboundStats {
    requests_forwarded: AtomicU64,
    requests_denied: AtomicU64,
    bytes_sent: AtomicU64,
}

impl OutboundProxy {
    /// Create a new outbound proxy with the given allowlist rules.
    pub fn new(rules: Vec<OutboundRule>) -> Self {
        debug!(rule_count = rules.len(), "outbound proxy initialized");
        Self {
            allowlist: Arc::new(rules),
            stats: Arc::new(OutboundStats {
                requests_forwarded: AtomicU64::new(0),
                requests_denied: AtomicU64::new(0),
                bytes_sent: AtomicU64::new(0),
            }),
        }
    }

    /// Check whether an outbound request is permitted by the allowlist.
    ///
    /// Returns `Ok(())` if at least one rule matches, or a `GatewayError::AuthFailed`
    /// describing why the request was denied.
    pub fn check_allowed(
        &self,
        domain: &str,
        method: &str,
        payload_size: usize,
    ) -> Result<(), GatewayError> {
        if self
            .allowlist
            .iter()
            .any(|r| r.matches(domain, method, payload_size))
        {
            debug!(domain, method, "outbound request allowed");
            Ok(())
        } else {
            self.stats.requests_denied.fetch_add(1, Ordering::Relaxed);
            warn!(domain, method, "outbound request denied — not in allowlist");
            Err(GatewayError::AuthFailed {
                reason: format!("outbound request to '{}' not in allowlist", domain),
            })
        }
    }

    /// Record a successfully forwarded outbound request and byte count.
    pub fn record_forward(&self, bytes: u64) {
        self.stats.requests_forwarded.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Get a snapshot of outbound proxy statistics.
    pub fn outbound_stats(&self) -> OutboundStatsSnapshot {
        OutboundStatsSnapshot {
            requests_forwarded: self.stats.requests_forwarded.load(Ordering::Relaxed),
            requests_denied: self.stats.requests_denied.load(Ordering::Relaxed),
            bytes_sent: self.stats.bytes_sent.load(Ordering::Relaxed),
        }
    }

    /// Number of allowlist rules currently configured.
    pub fn rule_count(&self) -> usize {
        self.allowlist.len()
    }
}

/// A point-in-time snapshot of outbound proxy statistics.
#[derive(Debug, Clone)]
pub struct OutboundStatsSnapshot {
    pub requests_forwarded: u64,
    pub requests_denied: u64,
    pub bytes_sent: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rule(domain: &str, methods: &[&str], max_bytes: usize) -> OutboundRule {
        OutboundRule {
            domain_pattern: domain.into(),
            allowed_methods: methods.iter().map(|m| m.to_string()).collect(),
            max_payload_bytes: max_bytes,
        }
    }

    #[test]
    fn exact_domain_rule_matches() {
        let rule = make_rule("api.example.com", &[], 1_000_000);
        assert!(rule.matches("api.example.com", "GET", 0));
        assert!(!rule.matches("other.example.com", "GET", 0));
    }

    #[test]
    fn wildcard_domain_rule_matches() {
        let rule = make_rule("*.example.com", &[], 1_000_000);
        assert!(rule.matches("api.example.com", "GET", 0));
        assert!(rule.matches("example.com", "GET", 0)); // bare domain also matches
        assert!(!rule.matches("totally-different.org", "GET", 0));
    }

    #[test]
    fn method_filter_enforced() {
        let rule = make_rule("api.example.com", &["GET", "POST"], 1_000_000);
        assert!(rule.matches("api.example.com", "GET", 0));
        assert!(rule.matches("api.example.com", "POST", 0));
        assert!(!rule.matches("api.example.com", "DELETE", 0));
    }

    #[test]
    fn empty_methods_allows_all() {
        let rule = make_rule("api.example.com", &[], 1_000_000);
        assert!(rule.matches("api.example.com", "DELETE", 0));
        assert!(rule.matches("api.example.com", "PATCH", 500));
    }

    #[test]
    fn payload_limit_enforced() {
        let rule = make_rule("api.example.com", &[], 1024);
        assert!(rule.matches("api.example.com", "POST", 1024));
        assert!(!rule.matches("api.example.com", "POST", 1025));
    }

    #[test]
    fn check_allowed_passes_for_matching_rule() {
        let proxy = OutboundProxy::new(vec![make_rule("api.example.com", &[], 1_000_000)]);
        assert!(proxy.check_allowed("api.example.com", "GET", 0).is_ok());
    }

    #[test]
    fn check_allowed_denied_for_no_match() {
        let proxy = OutboundProxy::new(vec![make_rule("api.example.com", &[], 1_000_000)]);
        let result = proxy.check_allowed("evil.org", "GET", 0);
        assert!(result.is_err());

        let stats = proxy.outbound_stats();
        assert_eq!(stats.requests_denied, 1);
    }

    #[test]
    fn record_forward_updates_stats() {
        let proxy = OutboundProxy::new(vec![]);
        proxy.record_forward(512);
        proxy.record_forward(256);

        let stats = proxy.outbound_stats();
        assert_eq!(stats.requests_forwarded, 2);
        assert_eq!(stats.bytes_sent, 768);
    }

    #[test]
    fn empty_allowlist_denies_all() {
        let proxy = OutboundProxy::new(vec![]);
        assert!(proxy.check_allowed("any.domain", "GET", 0).is_err());
    }

    #[test]
    fn rule_count_matches() {
        let proxy = OutboundProxy::new(vec![
            make_rule("a.com", &[], 100),
            make_rule("b.com", &[], 200),
        ]);
        assert_eq!(proxy.rule_count(), 2);
    }

    #[test]
    fn initial_stats_are_zero() {
        let proxy = OutboundProxy::new(vec![]);
        let stats = proxy.outbound_stats();
        assert_eq!(stats.requests_forwarded, 0);
        assert_eq!(stats.requests_denied, 0);
        assert_eq!(stats.bytes_sent, 0);
    }
}
