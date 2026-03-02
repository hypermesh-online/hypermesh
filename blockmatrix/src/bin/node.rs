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

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, Level};

use blockmatrix::assets::pipeline::{
    Asset, AssetPipeline, DecryptionKey, PipelineInputMetadata, ProcessedAsset, Shard,
    ShardMetadata,
};
use blockmatrix::assets::pipeline::distribution::{DistributedAsset, DistributionMetadata};
use blockmatrix::assets::pipeline::{PipelineStats};
use blockmatrix::bootstrap::{NodeBootstrap, PrivacyMode};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::shard_store::ShardStore;
use blockmatrix::network::shard_transport::StoqShardTransport;
use blockmatrix::network::NetworkManager;
use hypermesh_lib::ContentHash;

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

    /// Run as a reflector (public peer that accepts and relays)
    #[clap(long)]
    reflector: bool,

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

    /// Store a file as a distributed asset
    Store {
        /// Path to the file to store
        path: std::path::PathBuf,
    },

    /// Fetch a distributed asset
    Fetch {
        /// Asset ID to fetch
        asset_id: String,
        /// Output path (default: stdout)
        #[clap(short, long)]
        output: Option<std::path::PathBuf>,
    },
}

/// Shard map persisted to disk after Store, used by Fetch to reconstruct.
#[derive(Serialize, Deserialize)]
struct ShardMap {
    asset_id: String,
    shard_hashes: Vec<String>,
    decryption_key: DecryptionKey,
    shard_count: usize,
    original_size: usize,
    /// Per-shard metadata needed for reconstruction.
    shard_metadata: Vec<ShardMetadata>,
}

/// Return the directory used for shard map files (`~/.hypermesh/shard_maps/`).
fn shard_maps_dir() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".hypermesh").join("shard_maps"))
}

/// Run the Store subcommand: ingest a file through the asset pipeline and
/// persist the resulting shards + shard map locally.
async fn run_store(path: std::path::PathBuf) -> Result<()> {
    // 1. Read file
    let file_data =
        std::fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unnamed".to_string());
    let file_size = file_data.len();

    // 2. Derive asset ID from content hash
    let asset_id = hex::encode(blake3::hash(&file_data).as_bytes());

    info!(
        "Storing file {} ({} bytes) as asset {}",
        file_name, file_size, asset_id,
    );

    // 3. Build pipeline input
    let asset = Asset {
        id: asset_id.clone(),
        data: file_data,
        metadata: PipelineInputMetadata {
            name: file_name,
            content_type: "application/octet-stream".to_string(),
            size: file_size,
            created_at: chrono::Utc::now().timestamp(),
            custom: std::collections::HashMap::new(),
        },
    };

    // 4. Process through pipeline
    let pipeline = AssetPipeline::default().context("failed to create asset pipeline")?;
    let processed = pipeline
        .process_asset(asset)
        .await
        .context("pipeline processing failed")?;

    // 5. Store each shard in a local ShardStore and collect hashes
    let shard_store = ShardStore::new();
    let mut shard_hashes: Vec<String> = Vec::with_capacity(processed.shards.len());
    let mut shard_metadata: Vec<ShardMetadata> = Vec::with_capacity(processed.shards.len());

    for shard in &processed.shards {
        let hash_bytes = *blake3::hash(&shard.data).as_bytes();
        let content_hash = ContentHash(hash_bytes);
        shard_store.store(content_hash, shard.data.clone()).await;
        shard_hashes.push(hex::encode(hash_bytes));
        shard_metadata.push(shard.metadata.clone());
    }

    info!(
        "Stored {} shards in local shard store",
        processed.shards.len()
    );

    // 6. Persist shard map to disk
    let map = ShardMap {
        asset_id: asset_id.clone(),
        shard_hashes: shard_hashes.clone(),
        decryption_key: processed.decryption_key,
        shard_count: processed.shards.len(),
        original_size: processed.stats.original_size,
        shard_metadata,
    };

    let maps_dir = shard_maps_dir()?;
    std::fs::create_dir_all(&maps_dir)
        .with_context(|| format!("failed to create {}", maps_dir.display()))?;

    let map_path = maps_dir.join(format!("{}.json", asset_id));
    let map_json =
        serde_json::to_string_pretty(&map).context("failed to serialize shard map")?;
    std::fs::write(&map_path, &map_json)
        .with_context(|| format!("failed to write {}", map_path.display()))?;

    // 7. Also persist raw shard data alongside the map so Fetch can read them
    let shards_dir = maps_dir.join(&asset_id);
    std::fs::create_dir_all(&shards_dir)
        .with_context(|| format!("failed to create {}", shards_dir.display()))?;

    for (hash_hex, shard) in shard_hashes.iter().zip(processed.shards.iter()) {
        let shard_path = shards_dir.join(hash_hex);
        std::fs::write(&shard_path, &shard.data)
            .with_context(|| format!("failed to write shard {}", shard_path.display()))?;
    }

    // 8. Print shard map JSON to stdout
    println!("{map_json}");
    info!("Asset ID: {}", asset_id);

    Ok(())
}

/// Run the Fetch subcommand: load a shard map from disk, reconstruct the
/// original file through the reverse pipeline, and write the output.
async fn run_fetch(asset_id: String, output: Option<std::path::PathBuf>) -> Result<()> {
    // 1. Locate shard map
    let maps_dir = shard_maps_dir()?;
    let map_path = maps_dir.join(format!("{}.json", asset_id));

    if !map_path.exists() {
        anyhow::bail!(
            "shard map not found at {}. Was the asset stored on this node?",
            map_path.display()
        );
    }

    let map_json = std::fs::read_to_string(&map_path)
        .with_context(|| format!("failed to read {}", map_path.display()))?;
    let map: ShardMap =
        serde_json::from_str(&map_json).context("failed to deserialize shard map")?;

    info!(
        "Loaded shard map for asset {} ({} shards, {} bytes original)",
        map.asset_id, map.shard_count, map.original_size,
    );

    // 2. Load shard data from disk
    let shards_dir = maps_dir.join(&asset_id);
    let mut shards: Vec<Shard> = Vec::with_capacity(map.shard_count);

    for (i, hash_hex) in map.shard_hashes.iter().enumerate() {
        let shard_path = shards_dir.join(hash_hex);
        let shard_data = std::fs::read(&shard_path)
            .with_context(|| format!("failed to read shard {}", shard_path.display()))?;

        let metadata = if i < map.shard_metadata.len() {
            map.shard_metadata[i].clone()
        } else {
            ShardMetadata {
                index: i,
                is_parity: false,
                size: shard_data.len(),
                original_size: shard_data.len(),
                hash: hash_hex.clone(),
            }
        };

        shards.push(Shard {
            data: shard_data,
            metadata,
        });
    }

    info!("Loaded {} shards from disk", shards.len());

    // 3. Reconstruct ProcessedAsset
    let processed = ProcessedAsset {
        asset_id: map.asset_id.clone(),
        shards,
        decryption_key: map.decryption_key,
        distributed: DistributedAsset {
            asset_id: map.asset_id.clone(),
            placements: vec![],
            metadata: DistributionMetadata {
                total_shards: map.shard_count,
                networks_used: 0,
                avg_shard_distance: 0.0,
                quality_score: 0.0,
                distributed_at: 0,
            },
        },
        stats: PipelineStats::default(),
    };

    // 4. Run reverse pipeline
    let pipeline = AssetPipeline::default().context("failed to create asset pipeline")?;
    let reconstructed = pipeline
        .reconstruct_asset(&processed)
        .await
        .context("asset reconstruction failed")?;

    // 5. Write output
    match output {
        Some(ref out_path) => {
            std::fs::write(out_path, &reconstructed)
                .with_context(|| format!("failed to write {}", out_path.display()))?;
            info!(
                "Reconstructed {} bytes -> {}",
                reconstructed.len(),
                out_path.display()
            );
        }
        None => {
            info!("Reconstructed {} bytes", reconstructed.len());
        }
    }

    Ok(())
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

                // Create shard infrastructure
                let shard_store = std::sync::Arc::new(ShardStore::new());
                let shard_transport = std::sync::Arc::new(
                    StoqShardTransport::new(transport.clone()),
                );
                info!(
                    "Shard store and transport initialized (store={} shards)",
                    shard_store.count().await
                );

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

                // If reflector, broadcast our position so peers can find us
                if cli.reflector {
                    info!("Reflector mode: broadcasting matrix position");
                    network_clone.broadcast_matrix_position().await?;
                }

                info!(
                    "Network initialized, accepting connections on port {}",
                    cli.stoq_port
                );

                // Periodically sync peer addresses to shard transport
                // and show connected node status
                let network_status = network_clone.clone();
                let shard_transport_sync = shard_transport.clone();
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(
                        tokio::time::Duration::from_secs(10),
                    );
                    loop {
                        interval.tick().await;

                        // Sync peer addresses into shard transport for auto-dial
                        let nodes = network_status.get_connected_nodes().await;
                        for node in &nodes {
                            let node_id = hypermesh_lib::NodeId::from_bytes(
                                *blake3::hash(node.node_id.as_bytes()).as_bytes(),
                            );
                            shard_transport_sync
                                .register_node_address(&node_id, node.address)
                                .await;
                        }

                        // Log status every 3rd tick (roughly 30s)
                        let node_count = nodes.len();
                        if node_count > 0 {
                            info!("Connected nodes: {}", node_count);
                            let neighbors =
                                network_status.find_matrix_neighbors(10.0).await;
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

                // Keep shard_store and shard_transport alive for the node's lifetime
                let _shard_store = shard_store;
                let _shard_transport = shard_transport;
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
        Some(Commands::Store { path }) => {
            run_store(path).await?;
        }
        Some(Commands::Fetch { asset_id, output }) => {
            run_fetch(asset_id, output).await?;
        }
        None => {
            // No command - just show bootstrap info
            info!("Node initialized successfully. Use 'start' to run or 'status' to check.");
        }
    }

    Ok(())
}
