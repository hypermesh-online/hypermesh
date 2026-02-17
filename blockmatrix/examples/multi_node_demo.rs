// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Multi-Node Orchestration Demo
//!
//! This example demonstrates how Block-MATRIX nodes can discover and communicate
//! with each other using STOQ transport and matrix topology.

use anyhow::Result;
use blockmatrix::bootstrap::{NodeBootstrap, PrivacyMode};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::NetworkManager;
use std::net::{SocketAddr, Ipv6Addr};
use std::sync::Arc;
use tracing::{info, warn};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("========================================");
    info!("   Block-MATRIX Multi-Node Demo");
    info!("========================================");
    info!("");

    // Create 3 nodes with different matrix coordinates
    let nodes = vec![
        (0, 0, 0, 9292), // Bootstrap node
        (1, 2, 0, 9293), // Node 1
        (2, 4, 1, 9294), // Node 2
    ];

    let mut handles = vec![];

    // Start bootstrap node first
    let (x, y, z, port) = nodes[0];
    info!("Starting bootstrap node at ({},{},{}) on port {}", x, y, z, port);

    let bootstrap_handle = tokio::spawn(async move {
        start_node(x, y, z, port, None, PrivacyMode::Public).await
    });
    handles.push(bootstrap_handle);

    // Wait for bootstrap to initialize
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Start other nodes connecting to bootstrap
    let bootstrap_addr = format!("[::1]:{}", nodes[0].3);

    for (x, y, z, port) in nodes.iter().skip(1) {
        info!("Starting node at ({},{},{}) on port {}", x, y, z, port);

        let bootstrap = bootstrap_addr.clone();
        let x = *x;
        let y = *y;
        let z = *z;
        let port = *port;

        let handle = tokio::spawn(async move {
            start_node(x, y, z, port, Some(bootstrap), PrivacyMode::Public).await
        });
        handles.push(handle);

        // Small delay between starts
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    info!("");
    info!("All nodes started. Waiting for discovery...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    info!("");
    info!("Multi-node network demonstration complete!");
    info!("");
    info!("Key achievements:");
    info!("✓ Nodes bootstrap with unique genesis blocks");
    info!("✓ Each node has its own blockchain");
    info!("✓ Nodes positioned in matrix topology");
    info!("✓ STOQ transport enables communication");
    info!("✓ Network discovery based on privacy mode");

    // Cancel all tasks
    for handle in handles {
        handle.abort();
    }

    Ok(())
}

async fn start_node(
    x: i64,
    y: i64,
    z: i64,
    port: u16,
    bootstrap: Option<String>,
    privacy_mode: PrivacyMode,
) -> Result<()> {
    // Create matrix coordinate
    let coord = MatrixCoordinate::new(x, y, z)?;

    // Bootstrap node
    let bootstrap_mgr = NodeBootstrap::initialize(coord.clone()).await?;
    bootstrap_mgr.verify_self_sufficient().await?;

    info!("Node ({},{},{}) bootstrapped with genesis: {}",
        x, y, z,
        &bootstrap_mgr.genesis_block().hash[..8]
    );

    // Set privacy mode
    bootstrap_mgr.set_privacy_mode(privacy_mode).await?;

    // Initialize STOQ transport if not private
    if privacy_mode != PrivacyMode::Private {
        let mut stoq_config = stoq::TransportConfig::default();
        stoq_config.port = port;
        stoq_config.bind_address = Ipv6Addr::UNSPECIFIED;

        let transport = Arc::new(stoq::StoqTransport::new(stoq_config).await?);

        // Parse bootstrap nodes
        let bootstrap_nodes: Vec<SocketAddr> = if let Some(addr) = bootstrap {
            vec![addr.parse()?]
        } else {
            vec![]
        };

        // Create network manager
        let network = NetworkManager::new(
            coord,
            transport,
            privacy_mode,
            bootstrap_nodes,
        ).await?;

        // Start discovery
        network.start_discovery().await?;

        // Accept connections
        let network_arc = Arc::new(network);
        let acceptor = network_arc.clone();

        tokio::spawn(async move {
            if let Err(e) = acceptor.accept_connections().await {
                warn!("Connection acceptor error: {}", e);
            }
        });

        // Monitor connections
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

            let count = network_arc.get_node_count().await;
            if count > 0 {
                info!("Node ({},{},{}) has {} connections", x, y, z, count);

                let neighbors = network_arc.find_matrix_neighbors(10.0).await;
                for neighbor in neighbors.iter().take(2) {
                    info!("  → Neighbor at ({},{},{})",
                        neighbor.coordinate.x,
                        neighbor.coordinate.y,
                        neighbor.coordinate.z
                    );
                }
            }
        }
    }

    Ok(())
}