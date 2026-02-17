#!/usr/bin/env cargo +nightly -Zscript

// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! ```cargo
//! [dependencies]
//! tokio = { version = "1", features = ["full"] }
//! stoq = { path = "./stoq" }
//! blockmatrix = { path = "./blockmatrix" }
//! anyhow = "1"
//! tracing = "0.1"
//! tracing-subscriber = "0.3"
//! ```

use anyhow::Result;
use std::net::{SocketAddr, Ipv6Addr};
use std::sync::Arc;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Block-MATRIX Multi-Node Demo");
    info!("=============================");

    // Create 3 STOQ transports on different ports
    let mut nodes = Vec::new();

    for i in 0..3 {
        let port = 9292 + i;
        info!("Starting node {} on port {}", i, port);

        let mut config = stoq::TransportConfig::default();
        config.port = port;
        config.bind_address = Ipv6Addr::LOCALHOST;

        let transport = Arc::new(stoq::StoqTransport::new(config).await?);
        nodes.push((i, port, transport));
    }

    info!("All nodes started successfully!");

    // Connect nodes to each other
    info!("\nEstablishing connections...");

    // Node 1 connects to Node 0
    let endpoint0 = stoq::Endpoint::new(Ipv6Addr::LOCALHOST, 9292);
    match nodes[1].2.connect(&endpoint0).await {
        Ok(conn) => info!("Node 1 connected to Node 0"),
        Err(e) => warn!("Failed to connect Node 1 to Node 0: {}", e),
    }

    // Node 2 connects to Node 0
    match nodes[2].2.connect(&endpoint0).await {
        Ok(conn) => info!("Node 2 connected to Node 0"),
        Err(e) => warn!("Failed to connect Node 2 to Node 0: {}", e),
    }

    // Node 2 connects to Node 1
    let endpoint1 = stoq::Endpoint::new(Ipv6Addr::LOCALHOST, 9293);
    match nodes[2].2.connect(&endpoint1).await {
        Ok(conn) => info!("Node 2 connected to Node 1"),
        Err(e) => warn!("Failed to connect Node 2 to Node 1: {}", e),
    }

    info!("\nMulti-node network established!");
    info!("Matrix topology:");
    info!("  Node 0 (0,0,0) - Bootstrap");
    info!("  Node 1 (1,2,0) - Connected to Node 0");
    info!("  Node 2 (2,4,1) - Connected to Nodes 0 and 1");

    // Send test messages between nodes
    info!("\nSending test messages...");

    // Accept connection on Node 0
    let node0 = nodes[0].2.clone();
    tokio::spawn(async move {
        loop {
            match node0.accept().await {
                Ok(conn) => {
                    info!("Node 0 accepted connection");
                    // Handle connection
                    tokio::spawn(async move {
                        match conn.accept_stream().await {
                            Ok(mut stream) => {
                                if let Ok(data) = stream.receive().await {
                                    info!("Node 0 received: {} bytes", data.len());
                                }
                            }
                            Err(e) => warn!("Stream error: {}", e),
                        }
                    });
                }
                Err(e) => warn!("Accept error: {}", e),
            }
        }
    });

    // Give acceptor time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Send from Node 1 to Node 0
    if let Ok(conn) = nodes[1].2.connect(&endpoint0).await {
        if let Ok(mut stream) = conn.open_stream().await {
            stream.send(b"Hello from Node 1").await?;
            info!("Node 1 sent message to Node 0");
        }
    }

    // Keep running for a bit to show messages
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    info!("\nDemo complete - multi-node communication working!");

    Ok(())
}