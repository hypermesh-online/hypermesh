// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! BlockMatrix Node Binary
//!
//! CORRECT ARCHITECTURE: TrustChain + BlockMatrix unified bootstrap
//!
//! Every node starts with:
//! 1. Unique genesis block (own blockchain)
//! 2. Self-signed localhost certificate
//! 3. DNS initialized with localhost -> ::1
//! 4. Privacy mode: Private (localhost only) by default
//!
//! Network participation is OPTIONAL based on privacy mode transition.

mod bootstrap;
mod cli;
mod commands;
mod config;
mod dispatch;
mod dispatch_dashboard;
mod hardware;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::{info, Level};

use blockmatrix::bootstrap::node_id;
use blockmatrix::matrix::coordinate::MatrixCoordinate;

use cli::{Cli, Commands};
use commands::dns::{extract_dns_from_blockchain, load_persisted_dns};
use config::{handle_config, handle_destroy, merge_config_into_cli};
use dispatch::dispatch_command;

#[tokio::main]
async fn main() -> Result<()> {
    let mut cli = Cli::parse();

    let level = if cli.debug { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .init();

    // --- Handle Config subcommand early (no bootstrap needed) ---
    if let Some(Commands::Config { ref action }) = cli.command {
        return handle_config(action, &cli);
    }

    // --- Handle Destroy subcommand (no bootstrap needed) ---
    if let Some(Commands::Destroy { chaotic }) = &cli.command {
        return handle_destroy(*chaotic, &cli);
    }

    // --- Merge config file with CLI flags (CLI wins) ---
    merge_config_into_cli(&mut cli);

    let coord = MatrixCoordinate::new(cli.coord_x, cli.coord_y, cli.coord_z)?;
    let nid = node_id(&coord);

    let data_dir = if cli.data_dir.starts_with('~') {
        let home = dirs::home_dir().context("could not determine home directory")?;
        home.join(&cli.data_dir[2..])
    } else {
        std::path::PathBuf::from(&cli.data_dir)
    };

    let metadata_path = data_dir
        .join(&nid)
        .join("blockchain")
        .join("metadata.json");
    let has_persisted_state = metadata_path.exists();

    let (boot, persistence) = if has_persisted_state {
        bootstrap::resume_node(&data_dir, &nid, coord).await?
    } else {
        bootstrap::fresh_boot(&data_dir, &nid, coord).await?
    };

    let persistence = std::sync::Arc::new(persistence);

    load_persisted_dns(boot.dns(), &data_dir, &nid).await;
    extract_dns_from_blockchain(boot.dns(), &boot).await;
    boot.verify_self_sufficient().await?;

    info!("=== Node Bootstrap Complete ===");
    info!("Genesis Block: {}", boot.genesis_block().hash);
    info!(
        "Certificate: {} (self-signed)",
        boot.localhost_certificate().subject
    );
    info!("Privacy Mode: {:?}", boot.privacy_mode().await);
    info!(
        "Persisted: {}",
        if has_persisted_state {
            "resumed from disk"
        } else {
            "fresh (saved)"
        }
    );

    let dns_records = boot.dns().all_records().await;
    for (name, addr) in dns_records {
        info!("DNS: {} -> {}", name, addr);
    }

    dispatch_command(cli, coord, &nid, &data_dir, &boot, persistence).await
}
