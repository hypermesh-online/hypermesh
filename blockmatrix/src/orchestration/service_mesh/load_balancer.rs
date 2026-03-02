// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Health-aware load balancer for the service mesh.
//!
//! Provides [`MeshLoadBalancer`] with RoundRobin and LeastConnections
//! strategies. Endpoint selection is driven by real health data from
//! [`ServiceHealth`] checks -- unhealthy endpoints are skipped.

use std::sync::atomic::{AtomicUsize, Ordering};

use super::{ServiceEndpoint, ServiceHealth};

/// Strategy used by the load balancer to pick endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// Cycle through endpoints in order.
    RoundRobin,
    /// Pick the endpoint with the fewest active connections.
    LeastConnections,
}

/// Health-aware load balancer that selects service endpoints.
pub struct MeshLoadBalancer {
    strategy: Strategy,
    /// Monotonic counter for round-robin rotation.
    counter: AtomicUsize,
}

impl MeshLoadBalancer {
    /// Create a new load balancer with the given strategy.
    pub fn new(strategy: Strategy) -> Self {
        Self {
            strategy,
            counter: AtomicUsize::new(0),
        }
    }

    /// Select the best endpoint from `candidates`.
    ///
    /// Only healthy endpoints (health == `Healthy` or `Warning`) are
    /// eligible. Returns `None` if no endpoint is eligible.
    pub fn select<'a>(&self, candidates: &'a [ServiceEndpoint]) -> Option<&'a ServiceEndpoint> {
        let eligible: Vec<&ServiceEndpoint> = candidates
            .iter()
            .filter(|ep| matches!(ep.health, ServiceHealth::Healthy | ServiceHealth::Warning))
            .collect();

        if eligible.is_empty() {
            return None;
        }

        match self.strategy {
            Strategy::RoundRobin => {
                let idx = self.counter.fetch_add(1, Ordering::Relaxed) % eligible.len();
                Some(eligible[idx])
            }
            Strategy::LeastConnections => eligible
                .into_iter()
                .min_by_key(|ep| ep.connections),
        }
    }

    /// Return the configured strategy.
    pub fn strategy(&self) -> Strategy {
        self.strategy
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::service_mesh::EndpointMetrics;
    use std::collections::HashMap;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::time::SystemTime;

    fn make_endpoint(id: &str, connections: u32, health: ServiceHealth) -> ServiceEndpoint {
        ServiceEndpoint {
            id: id.to_string(),
            service_id: "svc-test".to_string(),
            address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080),
            weight: 1.0,
            health,
            connections,
            metrics: EndpointMetrics {
                avg_response_time_ms: 10.0,
                request_rate: 50.0,
                error_rate: 0.0,
                cpu_utilization: 0.3,
                memory_utilization: 0.4,
                last_updated: SystemTime::now(),
            },
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_round_robin_cycles() {
        let lb = MeshLoadBalancer::new(Strategy::RoundRobin);
        let eps = vec![
            make_endpoint("ep1", 0, ServiceHealth::Healthy),
            make_endpoint("ep2", 0, ServiceHealth::Healthy),
            make_endpoint("ep3", 0, ServiceHealth::Healthy),
        ];

        let a = lb.select(&eps).expect("test: should select").id.clone();
        let b = lb.select(&eps).expect("test: should select").id.clone();
        let c = lb.select(&eps).expect("test: should select").id.clone();
        let d = lb.select(&eps).expect("test: should select").id.clone();

        // Should cycle: ep1 -> ep2 -> ep3 -> ep1.
        assert_eq!(a, "ep1");
        assert_eq!(b, "ep2");
        assert_eq!(c, "ep3");
        assert_eq!(d, "ep1");
    }

    #[test]
    fn test_least_connections_picks_lowest() {
        let lb = MeshLoadBalancer::new(Strategy::LeastConnections);
        let eps = vec![
            make_endpoint("busy", 50, ServiceHealth::Healthy),
            make_endpoint("idle", 2, ServiceHealth::Healthy),
            make_endpoint("medium", 20, ServiceHealth::Healthy),
        ];

        let selected = lb.select(&eps).expect("test: should select");
        assert_eq!(selected.id, "idle");
    }

    #[test]
    fn test_unhealthy_endpoints_skipped() {
        let lb = MeshLoadBalancer::new(Strategy::LeastConnections);
        let eps = vec![
            make_endpoint("dead", 0, ServiceHealth::Degraded),
            make_endpoint("alive", 10, ServiceHealth::Healthy),
        ];

        let selected = lb.select(&eps).expect("test: should select");
        assert_eq!(selected.id, "alive");
    }

    #[test]
    fn test_no_eligible_returns_none() {
        let lb = MeshLoadBalancer::new(Strategy::RoundRobin);
        let eps = vec![make_endpoint("dead", 0, ServiceHealth::Degraded)];

        assert!(lb.select(&eps).is_none());
    }
}
