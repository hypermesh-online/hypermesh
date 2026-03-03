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
use stoq::transport::NetworkType;
use blockmatrix::assets::pipeline::distribution::{DistributedAsset, DistributionMetadata};
use blockmatrix::assets::pipeline::PipelineStats;
use blockmatrix::blockchain::node_chain::NodeBlockchain;
use blockmatrix::bootstrap::{node_id, LocalhostCertificate, NodeBootstrap, PrivacyMode};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::shard_store::ShardStore;
use blockmatrix::network::shard_transport::StoqShardTransport;
use blockmatrix::network::NetworkManager;
use blockmatrix::persistence::{BlockQuery, PersistenceConfig, PersistenceManager};
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

    /// Data directory for blockchain persistence
    #[clap(long, default_value = "~/.blockmatrix")]
    data_dir: String,

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

    /// DNS operations — register, resolve, or list names
    Dns {
        #[clap(subcommand)]
        action: DnsAction,
    },
}

#[derive(Subcommand, Debug)]
enum DnsAction {
    /// Register a DNS name for this node (writes to local blockchain)
    Register {
        /// Domain name to register (e.g., "persist")
        name: String,
        /// IPv6 address to point to (default: this node's address)
        #[clap(short, long)]
        addr: Option<String>,
    },
    /// Resolve a DNS name
    Resolve {
        /// Domain name to resolve
        name: String,
    },
    /// List all registered DNS names
    List,
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

/// Run the Dns subcommand: register, resolve, or list DNS names.
///
/// Registration writes the name to both the in-memory resolver (for
/// immediate local use) and the node's blockchain (DNS-as-asset, R10).
/// Path to the persisted DNS records file for a given node data directory.
fn dns_records_path(data_dir: &std::path::Path, node_id: &str) -> std::path::PathBuf {
    data_dir.join(node_id).join("dns_records.json")
}

/// Load persisted DNS records from disk and register them into the resolver.
async fn load_persisted_dns(
    dns: &blockmatrix::bootstrap::DnsResolver,
    data_dir: &std::path::Path,
    node_id: &str,
) {
    let path = dns_records_path(data_dir, node_id);
    if !path.exists() {
        return;
    }
    match std::fs::read_to_string(&path) {
        Ok(json) => {
            if let Ok(records) =
                serde_json::from_str::<std::collections::HashMap<String, String>>(&json)
            {
                let mut count = 0u64;
                for (name, addr_str) in &records {
                    if let Ok(addr) = addr_str.parse::<std::net::IpAddr>() {
                        dns.register(name.clone(), addr).await;
                        count += 1;
                    }
                }
                if count > 0 {
                    info!("Loaded {count} persisted DNS record(s) from disk");
                }
            }
        }
        Err(e) => {
            warn!("Failed to read DNS records from {}: {e}", path.display());
        }
    }
}

/// Persist a single DNS record by updating the on-disk JSON file.
fn persist_dns_record(
    data_dir: &std::path::Path,
    node_id: &str,
    name: &str,
    addr: std::net::IpAddr,
) -> Result<()> {
    let path = dns_records_path(data_dir, node_id);
    let mut records: std::collections::HashMap<String, String> = if path.exists() {
        let json = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        serde_json::from_str(&json).unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    };
    records.insert(name.to_string(), addr.to_string());
    let json = serde_json::to_string_pretty(&records)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, json)?;
    Ok(())
}

async fn run_dns(
    action: DnsAction,
    bootstrap: &NodeBootstrap,
    data_dir: &std::path::Path,
    node_id: &str,
) -> Result<()> {
    match action {
        DnsAction::Register { name, addr } => {
            // Resolve target address
            let target_addr: std::net::IpAddr = if let Some(ref a) = addr {
                a.parse().with_context(|| format!("invalid IPv6 address: {a}"))?
            } else {
                // Default: this node's loopback (::1)
                std::net::IpAddr::from(std::net::Ipv6Addr::LOCALHOST)
            };

            // 1. Register in local resolver (immediate effect)
            bootstrap.dns().register(name.clone(), target_addr).await;

            // 2. Persist to disk so it survives restarts
            persist_dns_record(data_dir, node_id, &name, target_addr)?;

            // 3. Register on local blockchain (DNS-as-asset)
            let bc = bootstrap.blockchain();
            let tx_data = format!("DNS:REGISTER:{name}:{target_addr}");
            let block = bc
                .add_block_with_data(tx_data.into_bytes())
                .await
                .map_err(|e| anyhow::anyhow!("blockchain write failed: {e}"))?;

            println!();
            println!("  DNS Registered");
            println!("  --------------");
            println!("  name:  {name}");
            println!("  addr:  {target_addr}");
            println!("  block: {}", block.hash);
            println!("  chain: height {}", bc.get_height().await);
            println!();
        }
        DnsAction::Resolve { name } => {
            match bootstrap.dns().resolve(&name).await {
                Some(addr) => {
                    println!("{name} → {addr}");
                }
                None => {
                    println!("{name}: not found");
                }
            }
        }
        DnsAction::List => {
            let records = bootstrap.dns().all_records().await;
            if records.is_empty() {
                println!("No DNS records registered.");
            } else {
                println!();
                println!("  DNS Records");
                println!("  -----------");
                let mut sorted: Vec<_> = records.into_iter().collect();
                sorted.sort_by(|a, b| a.0.cmp(&b.0));
                for (name, addr) in sorted {
                    println!("  {name:<20} → {addr}");
                }
                println!();
            }
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
    let nid = node_id(&coord);

    // Resolve data directory (expand ~ to home)
    let data_dir = if cli.data_dir.starts_with('~') {
        let home = dirs::home_dir().context("could not determine home directory")?;
        home.join(&cli.data_dir[2..]) // strip "~/"
    } else {
        std::path::PathBuf::from(&cli.data_dir)
    };

    // Check for existing persisted state
    let metadata_path = data_dir.join(&nid).join("blockchain").join("metadata.json");
    let has_persisted_state = metadata_path.exists();

    let (bootstrap, persistence) = if has_persisted_state {
        // === RESUME: load persisted state ===
        info!("Found persisted state at {}, resuming node", data_dir.display());

        let persistence_config = PersistenceConfig {
            storage_dir: data_dir.clone(),
            enable_background: true,
            ..PersistenceConfig::default()
        };
        let persistence = PersistenceManager::new(persistence_config, nid.clone())
            .await
            .context("failed to initialize persistence manager")?;

        // Run recovery (WAL replay, integrity check)
        let report = persistence.recover().await
            .context("recovery failed")?;
        info!(
            "Recovery complete: status={:?}, blocks_recovered={}, wal_replayed={}",
            report.status, report.stats.blocks_recovered, report.stats.wal_entries_replayed,
        );

        // Load all blocks from storage
        let chain_metadata = persistence.load_block(BlockQuery::ByIndex(0)).await
            .context("failed to load genesis block")?;
        let genesis_block = chain_metadata
            .ok_or_else(|| anyhow::anyhow!("persisted state exists but genesis block missing"))?;

        // Get chain height from storage stats to load all blocks
        let stats = persistence.get_stats().await;
        let chain_height = stats.block_count.saturating_sub(1);

        let blocks = if chain_height > 0 {
            let mut all_blocks = vec![genesis_block.clone()];
            for idx in 1..=chain_height {
                if let Some(block) = persistence.load_block(BlockQuery::ByIndex(idx)).await
                    .context("failed to load block")? {
                    all_blocks.push(block);
                }
            }
            all_blocks
        } else {
            vec![genesis_block.clone()]
        };

        info!("Loaded {} blocks from disk", blocks.len());

        // Reconstruct in-memory blockchain
        let blockchain = std::sync::Arc::new(
            NodeBlockchain::from_blocks(coord, blocks)
                .map_err(|e| anyhow::anyhow!("failed to reconstruct blockchain: {}", e))?,
        );

        // Load certificate
        let cert_path = data_dir.join(&nid).join("certificate.json");
        let localhost_cert = if cert_path.exists() {
            let cert_json = std::fs::read_to_string(&cert_path)
                .with_context(|| format!("failed to read {}", cert_path.display()))?;
            serde_json::from_str::<LocalhostCertificate>(&cert_json)
                .context("failed to deserialize certificate")?
        } else {
            warn!("Certificate not found on disk, generating fresh one");
            // Will be re-saved below
            NodeBootstrap::generate_fresh_certificate()?
        };

        let bootstrap = NodeBootstrap::resume(coord, blockchain, genesis_block, localhost_cert).await?;
        (bootstrap, persistence)
    } else {
        // === FIRST BOOT: create fresh node ===
        info!(
            "No persisted state found, initializing fresh node at ({}, {}, {})",
            coord.x, coord.y, coord.z
        );

        let bootstrap = NodeBootstrap::initialize(coord).await?;

        // Initialize persistence and save initial state
        let persistence_config = PersistenceConfig {
            storage_dir: data_dir.clone(),
            enable_background: true,
            ..PersistenceConfig::default()
        };
        let persistence = PersistenceManager::new(persistence_config, nid.clone())
            .await
            .context("failed to initialize persistence manager")?;

        // Persist genesis block
        persistence.save_block(bootstrap.genesis_block()).await
            .context("failed to persist genesis block")?;

        // Persist certificate
        let cert_path = data_dir.join(&nid).join("certificate.json");
        let cert_json = serde_json::to_string_pretty(bootstrap.localhost_certificate())
            .context("failed to serialize certificate")?;
        std::fs::write(&cert_path, &cert_json)
            .with_context(|| format!("failed to write {}", cert_path.display()))?;

        info!("Persisted genesis block and certificate to {}", data_dir.display());
        (bootstrap, persistence)
    };

    let persistence = std::sync::Arc::new(persistence);

    // Load persisted DNS records (user-registered names survive restarts)
    load_persisted_dns(bootstrap.dns(), &data_dir, &nid).await;

    // Verify self-sufficiency
    bootstrap.verify_self_sufficient().await?;

    info!("=== Node Bootstrap Complete ===");
    info!("Genesis Block: {}", bootstrap.genesis_block().hash);
    info!(
        "Certificate: {} (self-signed)",
        bootstrap.localhost_certificate().subject
    );
    info!("Privacy Mode: {:?}", bootstrap.privacy_mode().await);
    info!("Persisted: {}", if has_persisted_state { "resumed from disk" } else { "fresh (saved)" });

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
                // Certificate strategy follows PrivacyMode (transport layer):
                //   Anonymous → ephemeral self-signed certs (no CA dependency)
                //   Private   → local TrustChain CA (local://trustchain)
                //   Public    → global TrustChain CA (quic://trust.hypermesh.online)
                //     - Exception: reflector nodes ARE trust anchors, use local CA
                let mut stoq_config = stoq::TransportConfig {
                    port: cli.stoq_port,
                    bind_address: std::net::Ipv6Addr::UNSPECIFIED,
                    ..stoq::TransportConfig::default()
                };

                // Determine certificate strategy from privacy mode
                let network_type = if privacy_mode == PrivacyMode::ANONYMOUS {
                    stoq_config.enable_falcon_crypto = false;
                    info!("Anonymous mode: using ephemeral certificates, no CA dependency");
                    NetworkType::Anonymous
                } else if privacy_mode == PrivacyMode::PUBLIC {
                    if cli.reflector {
                        // Reflector nodes ARE trust anchors — self-issue certs locally.
                        // trust.hypermesh.online can't request certs from itself over QUIC.
                        info!("Public reflector: self-issuing certificate via local TrustChain");
                        NetworkType::P2P
                    } else {
                        // Joining nodes request certs from the global TrustChain CA
                        info!("Public mode: requesting certificate from trust.hypermesh.online");
                        NetworkType::Public
                    }
                } else {
                    // Private/P2P: node is its own CA via local TrustChain
                    info!("Private mode: self-issuing certificate via local TrustChain");
                    NetworkType::P2P
                };

                // Initialize STOQ with network-aware certificate strategy
                let transport = std::sync::Arc::new(
                    stoq::StoqTransport::new_for_network(stoq_config, network_type).await?
                );

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
            info!("Shutting down — flushing persistence...");
            if let Err(e) = persistence.flush().await {
                warn!("Persistence flush error: {}", e);
            }
            if let Err(e) = persistence.shutdown().await {
                warn!("Persistence shutdown error: {}", e);
            }
            info!("Persistence flushed, shutdown complete.");
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
        Some(Commands::Dns { action }) => {
            run_dns(action, &bootstrap, &data_dir, &nid).await?;
        }
        None => {
            // No command - just show bootstrap info
            info!("Node initialized successfully. Use 'start' to run or 'status' to check.");
        }
    }

    Ok(())
}
