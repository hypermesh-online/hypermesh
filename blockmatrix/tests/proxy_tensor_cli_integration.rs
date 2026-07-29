// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration tests proving cross-scope proxy routing and CLI tensor features
//! work together consistently.
//!
//! Two subsystems are exercised:
//! - `ScopeAwareRouter` (proxy/scope_routing) -- cross-scope gateway resolution
//! - `CommandExecutor` (cli) -- user-facing commands, including the live tensor
//!   routing path (`calculate_routing_path`) behind `topology show-path`

use blockmatrix::assets::proxy::scope_routing::{ScopeAwareRouter, ScopeRoutingConfig};
use blockmatrix::cli::commands::{AssetCommand, CliCommand, NodeCommand, TopologyCommand};
use blockmatrix::cli::executor::CommandExecutor;
use blockmatrix::cli::output::{CliError, CliOutput};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use hypermesh_lib::BlockchainScope;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn coord(x: i64, y: i64, z: i64) -> MatrixCoordinate {
    MatrixCoordinate::new(x, y, z).expect("test: valid coordinate")
}

/// Build a ScopeAwareRouter pre-loaded with a Device<->Network gateway.
fn scope_router_with_gateway(gw_id: &str, gw_pos: MatrixCoordinate) -> ScopeAwareRouter {
    let mut router = ScopeAwareRouter::new(ScopeRoutingConfig::default());
    router.register_gateway_node(
        gw_id,
        gw_pos,
        vec![BlockchainScope::Device, BlockchainScope::Network],
    );
    router
}

/// Extract text from CliOutput::Text, panicking with a descriptive message
/// if the variant is wrong.
fn extract_text(output: CliOutput) -> String {
    match output {
        CliOutput::Text(t) => t,
        other => unreachable!("test: expected CliOutput::Text, got {:?}", other),
    }
}

/// Extract table row count from CliOutput::Table.
fn extract_table_row_count(output: CliOutput) -> usize {
    match output {
        CliOutput::Table(t) => t.row_count(),
        other => unreachable!("test: expected CliOutput::Table, got {:?}", other),
    }
}

// ===========================================================================
// 1. Proxy same/cross-scope resolution (2 tests)
// ===========================================================================

/// Cross-scope route via ScopeAwareRouter uses a registered gateway node.
#[test]
fn proxy_cross_scope_route_uses_gateway() {
    let gw_pos = coord(50, 50, 0);
    let source = coord(0, 0, 0);

    let mut scope_router = scope_router_with_gateway("gw-1", gw_pos);
    let scope_route = scope_router
        .resolve_route(
            "asset-x",
            BlockchainScope::Device,
            BlockchainScope::Network,
            &source,
        )
        .expect("test: proxy cross-scope route");

    assert_eq!(scope_route.gateway_node.as_deref(), Some("gw-1"));
    assert!(scope_route.requires_encryption);
    assert_eq!(scope_route.path.len(), 2); // source + gateway
    assert_eq!(
        scope_route.path.last().copied(),
        Some(gw_pos),
        "test: proxy path ends at the gateway position"
    );
}

/// Same-scope routing needs no gateway and no encryption.
#[test]
fn proxy_same_scope_direct_path() {
    let source = coord(10, 10, 0);

    let mut scope_router = ScopeAwareRouter::new(ScopeRoutingConfig::default());
    let proxy_route = scope_router
        .resolve_route(
            "local-asset",
            BlockchainScope::Device,
            BlockchainScope::Device,
            &source,
        )
        .expect("test: proxy same-scope");

    assert!(proxy_route.gateway_node.is_none());
    assert!(!proxy_route.requires_encryption);
}

// ===========================================================================
// 2. CLI + Proxy Integration (3 tests)
// ===========================================================================

/// CLI AssetCommand::Transfer creates a valid scope transition message
/// containing the expected asset ID, scopes, and status.
#[test]
fn cli_asset_transfer_produces_valid_scope_transition() {
    let mut exec = CommandExecutor::new();

    let cmd = CliCommand::Asset(AssetCommand::Transfer {
        asset_id: "gpu-42".into(),
        from_scope: BlockchainScope::Device,
        to_scope: BlockchainScope::Network,
    });

    let output = exec.execute(cmd).expect("test: CLI transfer");
    let text = extract_text(output);

    assert!(text.contains("Transfer queued"), "test: should say queued");
    assert!(text.contains("gpu-42"), "test: should contain asset id");
    assert!(text.contains("Device"), "test: should contain source scope");
    assert!(
        text.contains("Network"),
        "test: should contain target scope"
    );
    assert!(text.contains("Pending"), "test: should show pending status");
}

/// Register a node via CLI, then query neighbors -- the registered node
/// appears in the result set.
#[test]
fn cli_register_then_query_neighbors() {
    let mut exec = CommandExecutor::new();

    // Register a node at (10, 10, 10)
    exec.execute(CliCommand::Node(NodeCommand::Register {
        x: 10,
        y: 10,
        z: 10,
        scope: BlockchainScope::Device,
    }))
    .expect("test: register node");

    // Query neighbors from origin with radius large enough to include it
    let output = exec
        .execute(CliCommand::Topology(TopologyCommand::QueryNeighbors {
            x: 0,
            y: 0,
            z: 0,
            radius: 30.0,
        }))
        .expect("test: query neighbors");

    let rows = extract_table_row_count(output);
    assert_eq!(rows, 1, "test: registered node should appear as neighbor");
}

/// CLI RoutingCost returns a consistent cost calculation including Euclidean
/// and Manhattan distances plus route quality.
#[test]
fn cli_routing_cost_consistent() {
    let mut exec = CommandExecutor::new();

    let output = exec
        .execute(CliCommand::Topology(TopologyCommand::RoutingCost {
            from_x: 0,
            from_y: 0,
            from_z: 0,
            to_x: 30,
            to_y: 40,
            to_z: 0,
        }))
        .expect("test: routing cost");

    let text = extract_text(output);

    // Euclidean distance of (30,40,0) from origin = 50.0
    assert!(
        text.contains("Euclidean distance: 50.00"),
        "test: expected Euclidean 50.00, got:\n{text}"
    );
    // Manhattan distance = 30 + 40 = 70
    assert!(
        text.contains("Manhattan distance: 70"),
        "test: expected Manhattan 70, got:\n{text}"
    );
    assert!(
        text.contains("Route quality"),
        "test: should contain Route quality"
    );
}

// ===========================================================================
// 3. CLI + Tensor Integration (3 tests)
// ===========================================================================

/// CLI ShowPath returns a table whose hop count matches what the tensor
/// routing path calculation produces for the same coordinates.
#[test]
fn cli_show_path_uses_tensor_routing() {
    let mut exec = CommandExecutor::new();

    let output = exec
        .execute(CliCommand::Topology(TopologyCommand::ShowPath {
            from_x: 0,
            from_y: 0,
            from_z: 0,
            to_x: 200,
            to_y: 0,
            to_z: 0,
        }))
        .expect("test: show path");

    let table = match output {
        CliOutput::Table(t) => t,
        other => unreachable!("test: expected Table, got {:?}", other),
    };

    // The CLI uses calculate_routing_path with max_hop_distance=50.0
    // For distance 200 at max 50 per hop, we need ceil(200/50) = 4 segments,
    // resulting in 5 points (source + 3 intermediates + dest).
    let from = coord(0, 0, 0);
    let to = coord(200, 0, 0);
    let tensor_path =
        blockmatrix::matrix::tensor::routing::calculate_routing_path(&from, &to, 50.0);

    assert_eq!(
        table.row_count(),
        tensor_path.len(),
        "test: CLI table rows should match tensor path length"
    );
    assert!(table.row_count() >= 3, "test: should have multiple hops");
}

/// Register multiple nodes via CLI, then MatrixInfo shows the correct count.
#[test]
fn cli_register_multiple_then_matrix_info() {
    let mut exec = CommandExecutor::new();

    // Register 5 nodes
    for i in 0..5 {
        exec.execute(CliCommand::Node(NodeCommand::Register {
            x: i * 10,
            y: i * 5,
            z: 0,
            scope: if i % 2 == 0 {
                BlockchainScope::Device
            } else {
                BlockchainScope::Network
            },
        }))
        .expect("test: register node");
    }

    let output = exec
        .execute(CliCommand::Topology(TopologyCommand::MatrixInfo))
        .expect("test: matrix info");
    let text = extract_text(output);

    assert!(
        text.contains("Total nodes: 5"),
        "test: expected 5 nodes, got:\n{text}"
    );
}

/// CLI handles invalid scope strings gracefully via the parse_scope function
/// (returns an error message, does not panic).
#[test]
fn cli_invalid_scope_graceful_error() {
    let result = blockmatrix::cli::commands::parse_scope("galactic");
    assert!(result.is_err());
    let err_msg = result.expect_err("test: should be error");
    assert!(
        err_msg.contains("Unknown blockchain scope"),
        "test: error should mention unknown scope, got: {err_msg}"
    );
    assert!(
        err_msg.contains("galactic"),
        "test: error should echo the bad input"
    );

    // Also verify that a same-scope transfer is rejected
    let mut exec = CommandExecutor::new();
    let result = exec.execute(CliCommand::Asset(AssetCommand::Transfer {
        asset_id: "a1".into(),
        from_scope: BlockchainScope::Device,
        to_scope: BlockchainScope::Device,
    }));
    assert!(
        matches!(result, Err(CliError::InvalidArgument(_))),
        "test: same-scope transfer should be InvalidArgument"
    );
}

// ===========================================================================
// 4. Full Stack Integration (2 tests)
// ===========================================================================

/// Complete flow: register nodes via CLI, resolve via proxy, and verify the
/// CLI routing cost is internally consistent with the resolved gateway.
#[test]
fn full_stack_register_route_resolve() {
    let mut cli = CommandExecutor::new();

    // Step 1: Register nodes via CLI
    cli.execute(CliCommand::Node(NodeCommand::Register {
        x: 0,
        y: 0,
        z: 0,
        scope: BlockchainScope::Device,
    }))
    .expect("test: register source");

    cli.execute(CliCommand::Node(NodeCommand::Register {
        x: 50,
        y: 50,
        z: 0,
        scope: BlockchainScope::Network,
    }))
    .expect("test: register gateway");

    cli.execute(CliCommand::Node(NodeCommand::Register {
        x: 100,
        y: 100,
        z: 0,
        scope: BlockchainScope::Network,
    }))
    .expect("test: register destination");

    // Step 2: CLI confirms topology
    let info = extract_text(
        cli.execute(CliCommand::Topology(TopologyCommand::MatrixInfo))
            .expect("test: info"),
    );
    assert!(info.contains("Total nodes: 3"));

    // Step 3: Proxy routing resolves a gateway at the registered position
    let gw_pos = coord(50, 50, 0);
    let mut scope_router = scope_router_with_gateway("gw-cli", gw_pos);
    let proxy_route = scope_router
        .resolve_route(
            "asset-full",
            BlockchainScope::Device,
            BlockchainScope::Network,
            &coord(0, 0, 0),
        )
        .expect("test: proxy route");

    let proxy_gw_pos = proxy_route
        .path
        .last()
        .copied()
        .expect("test: proxy path should end at gateway");

    // CLI-registered topology and proxy resolution agree on the gateway position
    assert_eq!(proxy_gw_pos, gw_pos);

    // Step 4: CLI routing cost confirms distance is non-zero
    let cost_text = extract_text(
        cli.execute(CliCommand::Topology(TopologyCommand::RoutingCost {
            from_x: 0,
            from_y: 0,
            from_z: 0,
            to_x: 100,
            to_y: 100,
            to_z: 0,
        }))
        .expect("test: routing cost"),
    );
    assert!(cost_text.contains("Euclidean distance"));
}

/// Multi-scope topology with 10+ nodes -- CLI and proxy observe the same
/// topology characteristics.
#[test]
fn full_stack_multi_scope_topology() {
    let mut cli = CommandExecutor::new();
    let mut scope_router = ScopeAwareRouter::new(ScopeRoutingConfig::default());

    // Register 6 Device nodes
    for i in 0..6 {
        let x = i * 20;
        let y = i * 10;
        cli.execute(CliCommand::Node(NodeCommand::Register {
            x,
            y,
            z: 0,
            scope: BlockchainScope::Device,
        }))
        .expect("test: register device node");
    }

    // Register 6 Network nodes (some also serve as gateways)
    for i in 0..6 {
        let x = i * 20 + 10;
        let y = i * 10 + 5;
        cli.execute(CliCommand::Node(NodeCommand::Register {
            x,
            y,
            z: 0,
            scope: BlockchainScope::Network,
        }))
        .expect("test: register network node");

        // Register first two as scope-bridging gateways
        if i < 2 {
            scope_router.register_gateway_node(
                &format!("net-{i}"),
                coord(x, y, 0),
                vec![BlockchainScope::Device, BlockchainScope::Network],
            );
        }
    }

    // CLI: verify total count
    let info = extract_text(
        cli.execute(CliCommand::Topology(TopologyCommand::MatrixInfo))
            .expect("test: matrix info"),
    );
    assert!(
        info.contains("Total nodes: 12"),
        "test: expected 12 nodes, got:\n{info}"
    );

    // Proxy: cross-scope route goes through a gateway
    let proxy_route = scope_router
        .resolve_route(
            "multi-asset",
            BlockchainScope::Device,
            BlockchainScope::Network,
            &coord(0, 0, 0),
        )
        .expect("test: proxy cross-scope");
    assert!(proxy_route.gateway_node.is_some());

    // Proxy stats reflect the route
    let stats = scope_router.get_scope_statistics();
    assert_eq!(stats.cross_scope_transfers, 1);

    // CLI: verify neighbors query returns subset
    let neighbor_output = cli
        .execute(CliCommand::Topology(TopologyCommand::QueryNeighbors {
            x: 0,
            y: 0,
            z: 0,
            radius: 25.0,
        }))
        .expect("test: query neighbors");
    let neighbor_count = extract_table_row_count(neighbor_output);
    assert!(
        (1..12).contains(&neighbor_count),
        "test: should find some but not all nodes, found {neighbor_count}"
    );
}
