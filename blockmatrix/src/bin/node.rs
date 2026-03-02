// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! BlockMatrix Node Binary
//!
//! CORRECT ARCHITECTURE: TrustChain + BlockMatrix unified bootstrap
//!
//! Every node starts with:
//! 1. Unique genesis block (own blockchain)
//! 2. Self-signed localhost certificate
//! 3. DNS initialized with localhost → ::1
//! 4. Privacy mode: Private (localhost only) by default
//!
//! Network participation is OPTIONAL based on privacy mode transition.

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, warn, Level};

use blockmatrix::bootstrap::{NodeBootstrap, PrivacyMode};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::NetworkManager;

#[derive(Parser, Debug)]
#[clap(name = "blockmatrix-node")]
#[clap(about = "BlockMatrix node with unified TrustChain bootstrap")]
#[clap(version)]
struct Cli {
    /// Enable debug logging
    #[clap(short, long)]
    debug: bool,

    /// Node X coordinate in matrix
    #[clap(short = 'x', long, default_value = "0")]
    coord_x: i64,

    /// Node Y coordinate in matrix
    #[clap(short = 'y', long, default_value = "0")]
    coord_y: i64,

    /// Node Z coordinate in matrix
    #[clap(short = 'z', long, default_value = "0")]
    coord_z: i64,

    /// Initial privacy mode
    #[clap(short, long, default_value = "private")]
    privacy: PrivacyModeArg,

    /// Bootstrap nodes (IPv6 addresses)
    #[clap(short = 'b', long)]
    bootstrap: Vec<String>,

    /// STOQ port
    #[clap(short = 's', long, default_value = "9292")]
    stoq_port: u16,

    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum PrivacyModeArg {
    Private,
    Anonymous,
    P2P,
    Public,
}

impl From<PrivacyModeArg> for PrivacyMode {
    fn from(arg: PrivacyModeArg) -> Self {
        match arg {
            PrivacyModeArg::Private => PrivacyMode::PRIVATE,
            PrivacyModeArg::Anonymous => PrivacyMode::ANONYMOUS,
            PrivacyModeArg::P2P => PrivacyMode::PRIVATE, // P2P collapses into PRIVATE
            PrivacyModeArg::Public => PrivacyMode::PUBLIC,
        }
    }
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the node
    Start,

    /// Show node status
    Status,

    /// Transition to different privacy mode
    SetPrivacy {
        #[clap(value_enum)]
        mode: PrivacyModeArg,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let level = if cli.debug { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .init();

    // Create matrix coordinate
    let coord = MatrixCoordinate::new(cli.coord_x, cli.coord_y, cli.coord_z)?;

    // Initialize node with unified bootstrap
    info!(
        "Initializing BlockMatrix node at ({}, {}, {})",
        coord.x, coord.y, coord.z
    );
    let bootstrap = NodeBootstrap::initialize(coord).await?;

    // Verify self-sufficiency
    bootstrap.verify_self_sufficient().await?;

    info!("=== Node Bootstrap Complete ===");
    info!("Genesis Block: {}", bootstrap.genesis_block().hash);
    info!(
        "Certificate: {} (self-signed)",
        bootstrap.localhost_certificate().subject
    );
    info!("Privacy Mode: {:?}", bootstrap.privacy_mode().await);

    // Get DNS records
    let dns_records = bootstrap.dns().all_records().await;
    for (name, addr) in dns_records {
        info!("DNS: {} → {}", name, addr);
    }

    // Execute command
    match cli.command {
        Some(Commands::Start) => {
            info!("Starting node services...");
            // Set initial privacy mode if different from default
            let target_mode = cli.privacy.into();
            if bootstrap.privacy_mode().await != target_mode {
                bootstrap.set_privacy_mode(target_mode).await?;
            }

            // Initialize STOQ transport if not in Private mode
            let privacy_mode = bootstrap.privacy_mode().await;
            if privacy_mode != PrivacyMode::PRIVATE {
                info!("Initializing STOQ transport on port {}", cli.stoq_port);

                // Configure STOQ transport based on privacy mode.
                // Anonymous mode: use ephemeral self-signed certs (no CA dependency).
                // Public mode: standard config (TrustChain CA certs will be used).
                let mut stoq_config = stoq::TransportConfig {
                    port: cli.stoq_port,
                    bind_address: std::net::Ipv6Addr::UNSPECIFIED,
                    ..stoq::TransportConfig::default()
                };

                if privacy_mode == PrivacyMode::ANONYMOUS {
                    // Anonymous mode: disable FALCON requirement (ephemeral certs
                    // don't need quantum-resistant CA signatures), reduce connection
                    // tracking for privacy.
                    stoq_config.enable_falcon_crypto = false;
                    info!("Anonymous mode: using ephemeral certificates, no CA dependency");
                }

                // Initialize STOQ
                let transport = std::sync::Arc::new(stoq::StoqTransport::new(stoq_config).await?);

                // Parse bootstrap nodes
                let bootstrap_nodes: Vec<std::net::SocketAddr> = cli
                    .bootstrap
                    .iter()
                    .filter_map(|addr| addr.parse().ok())
                    .collect();

                if !bootstrap_nodes.is_empty() {
                    info!("Bootstrap nodes: {:?}", bootstrap_nodes);
                }

                // Create network manager
                let network_manager =
                    NetworkManager::new(coord, transport.clone(), privacy_mode, bootstrap_nodes)
                        .await?;

                // Start discovery based on privacy mode
                network_manager.start_discovery().await?;

                // Start accepting connections in background
                let network_clone = std::sync::Arc::new(network_manager);
                let network_accept = network_clone.clone();
                tokio::spawn(async move {
                    if let Err(e) = network_accept.accept_connections().await {
                        warn!("Connection acceptor error: {}", e);
                    }
                });

                info!(
                    "Network initialized, accepting connections on port {}",
                    cli.stoq_port
                );

                // Periodically show connected nodes
                let network_status = network_clone.clone();
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
                    loop {
                        interval.tick().await;
                        let node_count = network_status.get_node_count().await;
                        if node_count > 0 {
                            info!("Connected nodes: {}", node_count);
                            let neighbors = network_status.find_matrix_neighbors(10.0).await;
                            for neighbor in neighbors.iter().take(3) {
                                info!(
                                    "  - Node {} at ({},{},{})",
                                    &neighbor.node_id[..8],
                                    neighbor.coordinate.x,
                                    neighbor.coordinate.y,
                                    neighbor.coordinate.z
                                );
                            }
                        }
                    }
                });
            }

            info!("Node running in {:?} mode", bootstrap.privacy_mode().await);
            info!("Press Ctrl+C to stop");

            // Keep running
            tokio::signal::ctrl_c().await?;
            info!("Shutting down...");
        }
        Some(Commands::Status) => {
            info!("Node Status:");
            info!("  Genesis: {}", bootstrap.genesis_block().hash);
            info!(
                "  Blockchain height: {}",
                bootstrap.blockchain().get_height().await
            );
            info!("  Privacy mode: {:?}", bootstrap.privacy_mode().await);
            info!("  Self-sufficient: ✓");
        }
        Some(Commands::SetPrivacy { mode }) => {
            let new_mode = mode.into();
            info!("Transitioning to {:?} mode...", new_mode);
            bootstrap.set_privacy_mode(new_mode).await?;
            info!("Privacy mode updated successfully");
        }
        None => {
            // No command - just show bootstrap info
            info!("Node initialized successfully. Use 'start' to run or 'status' to check.");
        }
    }

    Ok(())
}
