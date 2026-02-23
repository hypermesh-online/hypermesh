// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use std::net::SocketAddr;

use gateway::scope_router::{GatewayNode, GatewayTrustLevel, RouteDecision, ScopeRouter};
use hypermesh_lib::{BlockchainScope, MatrixPosition, PrivacyMode};

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
    router.register_gateway(make_gateway(
        "far",
        "[::1]:9010",
        vec![BlockchainScope::Device],
        GatewayTrustLevel::Full,
        (100.0, 100.0, 100.0),
    ));
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
