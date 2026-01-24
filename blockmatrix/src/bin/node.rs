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
use tracing_subscriber;

use blockmatrix::bootstrap::{NodeBootstrap, PrivacyMode};
use blockmatrix::matrix::coordinate::MatrixCoordinate;

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
            PrivacyModeArg::Private => PrivacyMode::Private,
            PrivacyModeArg::Anonymous => PrivacyMode::Anonymous,
            PrivacyModeArg::P2P => PrivacyMode::P2P,
            PrivacyModeArg::Public => PrivacyMode::Public,
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
    info!("Initializing BlockMatrix node at ({}, {}, {})", coord.x, coord.y, coord.z);
    let bootstrap = NodeBootstrap::initialize(coord).await?;

    // Verify self-sufficiency
    bootstrap.verify_self_sufficient().await?;

    info!("=== Node Bootstrap Complete ===");
    info!("Genesis Block: {}", bootstrap.genesis_block().hash);
    info!("Certificate: {} (self-signed)", bootstrap.localhost_certificate().subject);
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

            info!("Node running in {:?} mode", bootstrap.privacy_mode().await);
            info!("Press Ctrl+C to stop");

            // Keep running
            tokio::signal::ctrl_c().await?;
            info!("Shutting down...");
        }
        Some(Commands::Status) => {
            info!("Node Status:");
            info!("  Genesis: {}", bootstrap.genesis_block().hash);
            info!("  Blockchain height: {}", bootstrap.blockchain().get_height().await);
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
