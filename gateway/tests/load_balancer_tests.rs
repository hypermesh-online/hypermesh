// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use std::net::SocketAddr;

use gateway::load_balancer::{LoadBalanceStrategy, LoadBalancer};

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
