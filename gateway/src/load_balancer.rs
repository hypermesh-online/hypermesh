// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use dashmap::DashMap;
use tracing::{debug, warn};

/// Load balancing strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LoadBalanceStrategy {
    /// Cycle through healthy backends sequentially.
    RoundRobin,
    /// Select the backend with the fewest active connections.
    LeastConnections,
    /// Round-robin weighted by each backend's configured weight.
    WeightedRoundRobin,
    /// Combine health signals with least-connections.
    HealthAware,
}

impl Default for LoadBalanceStrategy {
    fn default() -> Self {
        Self::RoundRobin
    }
}

/// Public view of a backend tracked by the load balancer.
#[derive(Debug, Clone)]
pub struct LoadBalancedBackend {
    pub addr: SocketAddr,
    pub name: String,
    pub weight: u32,
    pub healthy: bool,
    pub active_connections: usize,
    pub total_requests: u64,
    pub total_failures: u64,
}

/// Internal per-backend state.
struct BackendState {
    name: String,
    weight: u32,
    healthy: bool,
    active_connections: AtomicUsize,
    total_requests: AtomicU64,
    total_failures: AtomicU64,
}

/// Load balancer that selects backends based on strategy.
pub struct LoadBalancer {
    backends: DashMap<SocketAddr, BackendState>,
    strategy: LoadBalanceStrategy,
    round_robin_counter: AtomicUsize,
}

impl LoadBalancer {
    pub fn new(strategy: LoadBalanceStrategy) -> Self {
        debug!(strategy = ?strategy, "Load balancer created");
        Self {
            backends: DashMap::new(),
            strategy,
            round_robin_counter: AtomicUsize::new(0),
        }
    }

    /// Register a backend with the given address, name, and weight.
    ///
    /// If a backend with the same address already exists, it is replaced.
    pub fn register(&self, addr: SocketAddr, name: String, weight: u32) {
        debug!(addr = %addr, name = %name, weight = weight, "Registering backend");
        self.backends.insert(
            addr,
            BackendState {
                name,
                weight: weight.max(1), // minimum weight of 1
                healthy: true,
                active_connections: AtomicUsize::new(0),
                total_requests: AtomicU64::new(0),
                total_failures: AtomicU64::new(0),
            },
        );
    }

    /// Remove a backend. Returns true if it existed.
    pub fn remove(&self, addr: &SocketAddr) -> bool {
        let removed = self.backends.remove(addr).is_some();
        if removed {
            debug!(addr = %addr, "Backend removed");
        }
        removed
    }

    /// Select a backend based on the configured strategy.
    ///
    /// Returns `None` if no healthy backends are available.
    pub fn select_backend(&self) -> Option<SocketAddr> {
        // Collect healthy backends as (addr, weight, active_connections).
        let healthy: Vec<(SocketAddr, u32, usize)> = self
            .backends
            .iter()
            .filter(|entry| entry.value().healthy)
            .map(|entry| {
                let addr = *entry.key();
                let weight = entry.value().weight;
                let conns = entry.value().active_connections.load(Ordering::Relaxed);
                (addr, weight, conns)
            })
            .collect();

        if healthy.is_empty() {
            warn!("No healthy backends available");
            return None;
        }

        let selected = match self.strategy {
            LoadBalanceStrategy::RoundRobin => {
                let idx = self.round_robin_counter.fetch_add(1, Ordering::Relaxed) % healthy.len();
                healthy[idx].0
            }
            LoadBalanceStrategy::LeastConnections => {
                // Pick the backend with fewest active connections.
                // Ties broken by order of iteration (stable).
                healthy
                    .iter()
                    .min_by_key(|(_addr, _weight, conns)| *conns)
                    .map(|(addr, _, _)| *addr)
                    .expect("healthy is non-empty (checked above)")
            }
            LoadBalanceStrategy::WeightedRoundRobin => self.select_weighted_round_robin(&healthy),
            LoadBalanceStrategy::HealthAware => {
                // Combine: prefer fewer connections, then higher weight.
                // Score = (active_connections + 1) * 1000 / (weight + 1). Lower is better.
                // Adding 1 prevents division by zero and avoids underflow.
                healthy
                    .iter()
                    .min_by_key(|(_addr, weight, conns)| {
                        ((*conns as u64) + 1) * 1000 / ((*weight as u64) + 1)
                    })
                    .map(|(addr, _, _)| *addr)
                    .expect("healthy is non-empty (checked above)")
            }
        };

        // Increment active connections and total requests for the selected backend.
        if let Some(state) = self.backends.get(&selected) {
            state.active_connections.fetch_add(1, Ordering::Relaxed);
            state.total_requests.fetch_add(1, Ordering::Relaxed);
        }

        Some(selected)
    }

    /// Weighted round-robin: distribute according to weight ratios.
    ///
    /// Uses modular arithmetic over the total weight to pick the right backend.
    fn select_weighted_round_robin(&self, healthy: &[(SocketAddr, u32, usize)]) -> SocketAddr {
        let total_weight: u64 = healthy.iter().map(|(_, w, _)| *w as u64).sum();
        if total_weight == 0 {
            // Fallback to simple round-robin if all weights are somehow 0.
            let idx = self.round_robin_counter.fetch_add(1, Ordering::Relaxed) % healthy.len();
            return healthy[idx].0;
        }

        let counter = self.round_robin_counter.fetch_add(1, Ordering::Relaxed) as u64;
        let target = counter % total_weight;

        let mut cumulative: u64 = 0;
        for (addr, weight, _) in healthy {
            cumulative += *weight as u64;
            if target < cumulative {
                return *addr;
            }
        }

        // Should never reach here, but return the last backend as fallback.
        healthy
            .last()
            .map(|(addr, _, _)| *addr)
            .expect("healthy is non-empty")
    }

    /// Report a successful request completion to a backend.
    ///
    /// Decrements active connections.
    pub fn report_success(&self, addr: &SocketAddr) {
        if let Some(state) = self.backends.get(addr) {
            let prev = state.active_connections.load(Ordering::Relaxed);
            if prev > 0 {
                state.active_connections.fetch_sub(1, Ordering::Relaxed);
            }
        }
    }

    /// Report a failed request to a backend.
    ///
    /// Decrements active connections and increments the failure counter.
    pub fn report_failure(&self, addr: &SocketAddr) {
        if let Some(state) = self.backends.get(addr) {
            let prev = state.active_connections.load(Ordering::Relaxed);
            if prev > 0 {
                state.active_connections.fetch_sub(1, Ordering::Relaxed);
            }
            state.total_failures.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Update health status for a backend.
    pub fn update_health(&self, addr: &SocketAddr, healthy: bool) {
        if let Some(mut state) = self.backends.get_mut(addr) {
            state.healthy = healthy;
            debug!(addr = %addr, healthy = healthy, "Backend health updated");
        }
    }

    /// Get all backends and their current status.
    pub fn list_backends(&self) -> Vec<LoadBalancedBackend> {
        self.backends
            .iter()
            .map(|entry| {
                let state = entry.value();
                LoadBalancedBackend {
                    addr: *entry.key(),
                    name: state.name.clone(),
                    weight: state.weight,
                    healthy: state.healthy,
                    active_connections: state.active_connections.load(Ordering::Relaxed),
                    total_requests: state.total_requests.load(Ordering::Relaxed),
                    total_failures: state.total_failures.load(Ordering::Relaxed),
                }
            })
            .collect()
    }

    /// Get the configured strategy.
    pub fn strategy(&self) -> LoadBalanceStrategy {
        self.strategy
    }

    /// Get the number of registered backends.
    pub fn backend_count(&self) -> usize {
        self.backends.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(port: u16) -> SocketAddr {
        format!("[::1]:{port}")
            .parse()
            .expect("test: valid socket addr")
    }

    fn make_lb(strategy: LoadBalanceStrategy) -> LoadBalancer {
        let lb = LoadBalancer::new(strategy);
        lb.register(addr(8001), "backend-a".into(), 1);
        lb.register(addr(8002), "backend-b".into(), 1);
        lb.register(addr(8003), "backend-c".into(), 1);
        lb
    }

    #[test]
    fn register_and_list_backends() {
        let lb = LoadBalancer::new(LoadBalanceStrategy::RoundRobin);
        assert_eq!(lb.backend_count(), 0);

        lb.register(addr(9000), "test".into(), 5);
        assert_eq!(lb.backend_count(), 1);

        let list = lb.list_backends();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "test");
        assert_eq!(list[0].weight, 5);
        assert!(list[0].healthy);
        assert_eq!(list[0].active_connections, 0);
    }

    #[test]
    fn remove_backend() {
        let lb = LoadBalancer::new(LoadBalanceStrategy::RoundRobin);
        lb.register(addr(9000), "a".into(), 1);
        assert_eq!(lb.backend_count(), 1);

        assert!(lb.remove(&addr(9000)));
        assert_eq!(lb.backend_count(), 0);

        // Removing again returns false
        assert!(!lb.remove(&addr(9000)));
    }

    #[test]
    fn round_robin_cycles_through_backends() {
        let lb = make_lb(LoadBalanceStrategy::RoundRobin);

        let first = lb.select_backend().expect("test: should have backend");
        let second = lb.select_backend().expect("test: should have backend");
        let third = lb.select_backend().expect("test: should have backend");
        let fourth = lb.select_backend().expect("test: should have backend");

        // After 3 selections we should cycle back to the first
        assert_eq!(first, fourth, "round-robin should cycle after 3 backends");
        // All three should be different
        assert_ne!(first, second);
        assert_ne!(second, third);
    }

    #[test]
    fn least_connections_selects_least_loaded() {
        let lb = make_lb(LoadBalanceStrategy::LeastConnections);

        // Select backend-a twice (adds 2 connections to it)
        let a1 = lb.select_backend().expect("test: backend");
        let _a2 = lb.select_backend().expect("test: backend");

        // Report success on a1 to reduce its connections
        lb.report_success(&a1);

        // The backends with 0 connections should be preferred. After two
        // selections, at least one backend has 0 connections.
        let next = lb.select_backend().expect("test: backend");

        // The selected backend should have had the fewest connections
        let backends = lb.list_backends();
        let min_conns = backends
            .iter()
            .filter(|b| b.healthy)
            .map(|b| b.active_connections)
            .min()
            .expect("test: non-empty");

        // next should have been selected because it had min connections
        // (before selection incremented it)
        let next_state = backends
            .iter()
            .find(|b| b.addr == next)
            .expect("test: found");
        // It was incremented by select, so it should be min_conns + 1 or equal
        assert!(
            next_state.active_connections <= min_conns + 1,
            "least-connections should pick least loaded backend"
        );
    }

    #[test]
    fn health_aware_avoids_unhealthy() {
        let lb = make_lb(LoadBalanceStrategy::HealthAware);

        // Mark two backends unhealthy
        lb.update_health(&addr(8001), false);
        lb.update_health(&addr(8002), false);

        // Only backend-c is healthy
        let selected = lb.select_backend().expect("test: backend");
        assert_eq!(selected, addr(8003));
    }

    #[test]
    fn no_healthy_backends_returns_none() {
        let lb = make_lb(LoadBalanceStrategy::RoundRobin);
        lb.update_health(&addr(8001), false);
        lb.update_health(&addr(8002), false);
        lb.update_health(&addr(8003), false);

        assert!(lb.select_backend().is_none());
    }

    #[test]
    fn empty_balancer_returns_none() {
        let lb = LoadBalancer::new(LoadBalanceStrategy::RoundRobin);
        assert!(lb.select_backend().is_none());
    }

    #[test]
    fn report_success_decrements_connections() {
        let lb = LoadBalancer::new(LoadBalanceStrategy::RoundRobin);
        lb.register(addr(7000), "single".into(), 1);

        let selected = lb.select_backend().expect("test: backend");
        let before = lb
            .list_backends()
            .iter()
            .find(|b| b.addr == selected)
            .expect("test: found")
            .active_connections;
        assert_eq!(before, 1);

        lb.report_success(&selected);
        let after = lb
            .list_backends()
            .iter()
            .find(|b| b.addr == selected)
            .expect("test: found")
            .active_connections;
        assert_eq!(after, 0);
    }

    #[test]
    fn report_failure_increments_failure_count() {
        let lb = LoadBalancer::new(LoadBalanceStrategy::RoundRobin);
        lb.register(addr(7001), "fail-test".into(), 1);

        let selected = lb.select_backend().expect("test: backend");
        lb.report_failure(&selected);
        lb.report_failure(&selected);

        let backend = lb
            .list_backends()
            .iter()
            .find(|b| b.addr == selected)
            .cloned()
            .expect("test: found");
        assert_eq!(backend.total_failures, 2);
        assert_eq!(backend.total_requests, 1); // Only select increments requests
    }

    #[test]
    fn update_health_toggles_state() {
        let lb = LoadBalancer::new(LoadBalanceStrategy::RoundRobin);
        lb.register(addr(7002), "health-test".into(), 1);

        let initial = lb.list_backends()[0].healthy;
        assert!(initial);

        lb.update_health(&addr(7002), false);
        assert!(!lb.list_backends()[0].healthy);

        lb.update_health(&addr(7002), true);
        assert!(lb.list_backends()[0].healthy);
    }

    #[test]
    fn weighted_round_robin_respects_weights() {
        let lb = LoadBalancer::new(LoadBalanceStrategy::WeightedRoundRobin);
        lb.register(addr(6001), "heavy".into(), 3);
        lb.register(addr(6002), "light".into(), 1);

        // Over a total weight of 4, heavy should be selected ~3x and light ~1x.
        let mut heavy_count = 0usize;
        let mut light_count = 0usize;

        // Run through multiple full cycles (4 * 10 = 40 selections)
        for _ in 0..40 {
            let selected = lb.select_backend().expect("test: backend");
            lb.report_success(&selected); // release connection
            if selected == addr(6001) {
                heavy_count += 1;
            } else {
                light_count += 1;
            }
        }

        // heavy should get roughly 3x the traffic of light
        assert!(
            heavy_count > light_count,
            "heavy ({heavy_count}) should exceed light ({light_count})"
        );
        // More precisely: heavy should be about 30, light about 10
        assert!(
            heavy_count >= 25,
            "heavy backend should get at least 25 of 40 requests, got {heavy_count}"
        );
    }

    #[test]
    fn strategy_accessor() {
        let lb = LoadBalancer::new(LoadBalanceStrategy::LeastConnections);
        assert_eq!(lb.strategy(), LoadBalanceStrategy::LeastConnections);
    }

    #[test]
    fn default_strategy_is_round_robin() {
        assert_eq!(
            LoadBalanceStrategy::default(),
            LoadBalanceStrategy::RoundRobin
        );
    }

    #[test]
    fn report_on_unknown_backend_is_noop() {
        let lb = LoadBalancer::new(LoadBalanceStrategy::RoundRobin);
        // Should not panic
        lb.report_success(&addr(9999));
        lb.report_failure(&addr(9999));
    }

    #[test]
    fn list_backends_includes_all_fields() {
        let lb = LoadBalancer::new(LoadBalanceStrategy::RoundRobin);
        lb.register(addr(5000), "full-test".into(), 7);

        // Select to increment counters
        let selected = lb.select_backend().expect("test: backend");
        lb.report_failure(&selected);

        let list = lb.list_backends();
        assert_eq!(list.len(), 1);
        let b = &list[0];
        assert_eq!(b.name, "full-test");
        assert_eq!(b.weight, 7);
        assert!(b.healthy);
        assert_eq!(b.total_requests, 1);
        assert_eq!(b.total_failures, 1);
        assert_eq!(b.active_connections, 0); // report_failure decremented
    }
}
