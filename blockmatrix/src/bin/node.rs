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

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn, Level};

use blockmatrix::assets::core::{
    AssetCategory, AssetData, AssetRegistration, BaseSystemType, NetworkScope,
};
use blockmatrix::dns::domain::DomainRegistration;
use blockmatrix::dns::invitation;
use blockmatrix::assets::pipeline::{
    Asset, AssetPipeline, DecryptionKey, PipelineInputMetadata, ProcessedAsset, Shard,
    ShardMetadata,
};
use blockmatrix::create_os_abstraction;
use blockmatrix::proof_of_state::genesis_proof::{generate_genesis_proof, HardwareAssessment};
use blockmatrix::StateProof;
use stoq::transport::NetworkType;
use blockmatrix::assets::pipeline::distribution::{DistributedAsset, DistributionMetadata};
use blockmatrix::assets::pipeline::PipelineStats;
use blockmatrix::blockchain::node_chain::NodeBlockchain;
use blockmatrix::blockchain::propagation::{BlockPropagator, PropagationStrategy};
use blockmatrix::blockchain::stoq_transport::StoqBlockTransportAdapter;
use blockmatrix::blockchain::sync_manager::{NodeBlockchainBlockProvider, SyncConfig, SyncManager};
use blockmatrix::bootstrap::{node_id, LocalhostCertificate, NodeBootstrap, PrivacyMode};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::reflector_pool::{ReflectorConfig, ReflectorPool};
use blockmatrix::network::shard_store::ShardStore;
use blockmatrix::network::shard_transport::{ShardTransport, StoqShardTransport};
use blockmatrix::network::sync_dispatch::TransportSyncDriver;
use blockmatrix::network::NetworkManager;
use blockmatrix::ipc;
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

    /// Output in JSON format
    #[clap(long, global = true)]
    json: bool,

    /// Path to config file
    #[clap(long, global = true)]
    config: Option<std::path::PathBuf>,

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
    /// Connect to the mesh (starts daemon if not running)
    Connect {
        /// Privacy mode for network participation
        #[clap(value_enum, default_value = "public")]
        privacy: PrivacyModeArg,
        /// Run in foreground (don't background)
        #[clap(long)]
        foreground: bool,
    },

    /// Disconnect from the mesh (stop daemon)
    Disconnect,

    /// [DEPRECATED] Use 'connect' instead
    #[clap(hide = true)]
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

    /// Domain operations — register, create sub-domains, invite peers
    Domain {
        #[clap(subcommand)]
        action: DomainAction2,
    },

    /// Join a domain's network
    Join {
        /// Domain name (e.g., "home.persist.hypermesh")
        network: String,
        /// Invitation token (required for private domains)
        #[clap(long)]
        invite: Option<String>,
    },

    /// Manage configuration
    Config {
        #[clap(subcommand)]
        action: ConfigCommand,
    },

    /// Dashboard operations
    Dashboard {
        #[clap(subcommand)]
        action: DashboardAction,
    },

    /// Caesar EVP operations
    Caesar {
        #[clap(subcommand)]
        action: CaesarAction,
    },

    /// TrustChain CA operations
    Trustchain {
        #[clap(subcommand)]
        action: TrustchainAction,
    },

    /// Engauge analytics operations
    Engauge {
        #[clap(subcommand)]
        action: EngaugeAction,
    },

    /// Catalog registry operations
    Catalog {
        #[clap(subcommand)]
        action: CatalogAction,
    },

    /// Destroy all node data (blockchain, identity, shards, config)
    Destroy {
        /// Skip confirmation prompt
        #[clap(long)]
        chaotic: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCommand {
    /// Show current configuration
    Show,
    /// Get a config value by dotted key path
    Get {
        /// Dotted key path (e.g. "network.stoq_port")
        key: String,
    },
    /// Set a config value
    Set {
        /// Dotted key path (e.g. "network.stoq_port")
        key: String,
        /// Value to set (parsed as JSON if possible, otherwise string)
        value: String,
    },
    /// Initialize default config file
    Init,
}

#[derive(Subcommand, Debug)]
enum DashboardAction {
    /// Deploy a dashboard from a directory containing dashboard.toml
    Deploy {
        /// Path to directory containing dashboard.toml
        path: std::path::PathBuf,
    },
    /// List registered dashboards
    List,
    /// Show dashboard info
    Info {
        /// Dashboard name
        name: String,
    },
    /// Initialize a new dashboard project (scaffold)
    Init {
        /// Project name (default: my-dashboard)
        name: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum DomainAction2 {
    /// Register a new top-level domain on this node's blockchain
    Register {
        /// Domain name (e.g., "hypermesh")
        name: String,
        /// Privacy mode for the domain network
        #[clap(long, value_enum, default_value = "private")]
        privacy: PrivacyModeArg,
    },
    /// Create a sub-domain under an existing domain
    Create {
        /// Sub-domain name (e.g., "home.hypermesh")
        name: String,
        /// Privacy mode for the sub-domain network
        #[clap(long, value_enum, default_value = "private")]
        privacy: PrivacyModeArg,
    },
    /// List all registered domains
    List,
    /// Show nodes in a domain's network
    Nodes {
        /// Domain name to query
        domain: String,
    },
    /// Create an invitation token for a domain
    Invite {
        /// Domain name to invite to
        domain: String,
        /// Target peer node ID (or "open" for any node)
        #[clap(long)]
        peer: String,
        /// Invitation validity in seconds
        #[clap(long, default_value = "3600")]
        ttl: u64,
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

#[derive(Subcommand, Debug)]
enum CaesarAction {
    /// Show wallet info
    Wallet,
    /// Show balance
    Balance,
    /// List recent transactions
    Transactions {
        /// Maximum number of transactions
        #[clap(long, default_value = "10")]
        limit: u32,
    },
    /// Show reward earnings
    Rewards,
    /// Route a payment packet
    Route {
        /// Destination node
        destination: String,
        /// Amount in gold grams
        amount: f64,
    },
    /// Show governor parameters
    Governor,
}

#[derive(Subcommand, Debug)]
enum TrustchainAction {
    /// List certificates
    Certs,
    /// Issue a new certificate
    Issue {
        /// Certificate subject
        subject: String,
        /// Scope (anonymous, private, public)
        #[clap(long, default_value = "private")]
        scope: String,
    },
    /// Validate a certificate
    Validate {
        /// Path to PEM certificate file
        cert_path: String,
    },
    /// Revoke a certificate
    Revoke {
        /// Certificate ID
        cert_id: String,
    },
    /// List DNS zones
    Zones,
}

#[derive(Subcommand, Debug)]
enum EngaugeAction {
    /// Show capacity metrics
    Capacity,
    /// Show traffic analysis
    Traffic,
    /// List marketplace offerings
    Marketplace,
    /// Show node metrics
    Metrics,
    /// List active leases
    Leases,
}

#[derive(Subcommand, Debug)]
enum CatalogAction {
    /// Browse packages
    Browse {
        /// Search query
        #[clap(long)]
        query: Option<String>,
        /// Page number
        #[clap(long, default_value = "1")]
        page: u32,
    },
    /// Search packages
    Search {
        /// Search query
        query: String,
    },
    /// Get package info
    Info {
        /// Package name
        name: String,
    },
    /// Show registry statistics
    Stats,
}

/// Call a service method via IPC and print the result.
///
/// Routes the call through the IPC daemon. If the daemon is not running,
/// prints an offline message suggesting `hypermesh connect`.
async fn service_ipc_call(
    method: &str,
    params: serde_json::Value,
    json_output: bool,
) -> Result<()> {
    let client = ipc::IpcClient::new();
    if !client.is_daemon_running().await {
        if json_output {
            let err = serde_json::json!({
                "error": "daemon_offline",
                "message": format!("Service method '{}' requires a running daemon", method),
                "hint": "Start with: hypermesh connect public",
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&err).unwrap_or_default()
            );
        } else {
            eprintln!(
                "Service '{}' requires a running daemon.\nStart with: hypermesh connect public",
                method.split('.').next().unwrap_or(method),
            );
        }
        return Ok(());
    }
    match client.call_ok(method, params).await {
        Ok(resp) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&resp).unwrap_or_default()
            );
        }
        Err(e) => {
            if json_output {
                let err = serde_json::json!({
                    "error": "service_error",
                    "method": method,
                    "message": format!("{e}"),
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&err).unwrap_or_default()
                );
            } else {
                eprintln!("Error calling {method}: {e}");
            }
        }
    }
    Ok(())
}

/// Propagate a newly-created block to connected peers via the BlockPropagator.
///
/// This is a best-effort operation: propagation failures are logged but do
/// not fail the caller. Only propagates when there are connected peers.
async fn propagate_block(
    block: &blockmatrix::blockchain::block::Block,
    propagator: &tokio::sync::Mutex<BlockPropagator>,
    network: &NetworkManager,
) {
    let coords = network.get_connected_coordinates().await;
    if coords.is_empty() {
        debug!("No connected peers, skipping block propagation");
        return;
    }

    let result = propagator.lock().await.propagate_block(block, &coords).await;
    info!(
        "Block #{} propagated to {} peer(s) ({} failed)",
        block.index,
        result.reached_nodes.len(),
        result.failed_nodes.len(),
    );
}

/// Extract DNS registration data from a block's asset records.
///
/// DNS registrations are identified by their `AssetCategory::BaseSystem(BaseSystemType::Dns)`
/// category. The original name/address data is encoded in the `AssetData.config` field as
/// `DNS:REGISTER:{name}:{addr}` and hashed into `content_hash` during `from_asset_data`.
///
/// Since `AssetRegistration` only stores the content hash (not the original data),
/// this function cannot recover name/addr from the block alone. It returns the
/// count of DNS-typed assets found, which can trigger a full chain re-scan via
/// the persistence layer for nodes that have the original DNS records file.
fn count_dns_assets_in_block(block: &blockmatrix::blockchain::block::Block) -> usize {
    block
        .get_assets()
        .iter()
        .filter(|asset| {
            matches!(
                asset.category,
                AssetCategory::BaseSystem(BaseSystemType::Dns)
            )
        })
        .count()
}

/// Optional network context for shard distribution during asset storage.
///
/// When running inside `Commands::Connect`, the node has a live network with
/// connected peers. Standalone `Store` invocations have no network.
struct ShardDistributionCtx {
    network: std::sync::Arc<NetworkManager>,
    shard_transport: std::sync::Arc<StoqShardTransport>,
}

/// Distribute shards to connected network peers (best-effort).
///
/// Sends each shard to up to 6 nearest peers. Failures are logged but
/// do not abort the operation.
async fn distribute_shards_to_network(
    ctx: &ShardDistributionCtx,
    shards: &[(ContentHash, Vec<u8>)],
) {
    let connected_nodes = ctx.network.get_connected_nodes().await;
    if connected_nodes.is_empty() {
        info!("No connected peers for shard distribution (local-only storage)");
        return;
    }

    let mut distributed = 0usize;
    let mut failed = 0usize;
    for (shard_hash, shard_data) in shards {
        // Send to up to 6 nearest peers
        for node in connected_nodes.iter().take(6) {
            let node_id = hypermesh_lib::NodeId::from_bytes(
                *blake3::hash(node.node_id.as_bytes()).as_bytes(),
            );
            match ctx
                .shard_transport
                .send_shard(&node_id, shard_hash, shard_data)
                .await
            {
                Ok(()) => distributed += 1,
                Err(e) => {
                    warn!(
                        "Failed to distribute shard to {}: {}",
                        &node.node_id[..8.min(node.node_id.len())],
                        e
                    );
                    failed += 1;
                }
            }
        }
    }
    info!("Shard distribution: {} sent, {} failed", distributed, failed);
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

/// Assess node hardware and build `AssetRegistration` entries for each
/// detected resource (R1 compliance: hardware assessed, not self-reported,
/// registered as IPv6-addressed assets with Proof of State).
fn assess_hardware_assets() -> Result<Vec<AssetRegistration>> {
    let os = create_os_abstraction().context("failed to create OS abstraction")?;
    let platform = os.platform().to_string();
    let mut assets: Vec<AssetRegistration> = Vec::new();

    // CPU asset
    match os.detect_cpu() {
        Ok(cpu) => {
            let freq_str = cpu
                .frequency_mhz
                .map(|f| format!("{f} MHz"))
                .unwrap_or_else(|| "unknown".to_string());
            let asset_data = AssetData {
                config: format!(
                    "platform={platform},cores={},arch={}",
                    cpu.cores, cpu.architecture,
                )
                .into_bytes(),
                definition: format!(
                    "cpu:{}:{}:{}",
                    cpu.model, cpu.cores, freq_str,
                )
                .into_bytes(),
                metadata: format!(
                    "vendor={},freq={}",
                    cpu.vendor.as_deref().unwrap_or("unknown"),
                    freq_str,
                )
                .into_bytes(),
            };
            assets.push(AssetRegistration::from_asset_data(
                &asset_data,
                NetworkScope::Global,
                AssetCategory::BaseSystem(BaseSystemType::Cpu),
            ));
            info!(
                "Hardware: CPU {} ({} cores, {})",
                cpu.model, cpu.cores, freq_str,
            );
        }
        Err(e) => warn!("CPU detection failed: {e}"),
    }

    // Memory asset
    match os.detect_memory() {
        Ok(mem) => {
            let total_mb = mem.total_bytes / (1024 * 1024);
            let avail_mb = mem.available_bytes / (1024 * 1024);
            let asset_data = AssetData {
                config: format!("platform={platform}").into_bytes(),
                definition: format!(
                    "memory:total={},available={}",
                    mem.total_bytes, mem.available_bytes,
                )
                .into_bytes(),
                metadata: format!("usage={:.1}%", mem.usage_percent).into_bytes(),
            };
            assets.push(AssetRegistration::from_asset_data(
                &asset_data,
                NetworkScope::Global,
                AssetCategory::BaseSystem(BaseSystemType::Memory),
            ));
            info!("Hardware: Memory {total_mb} MB total, {avail_mb} MB available");
        }
        Err(e) => warn!("Memory detection failed: {e}"),
    }

    // Storage assets (one per mount point)
    match os.detect_storage() {
        Ok(devices) => {
            for dev in &devices {
                let total_gb = dev.total_bytes / (1024 * 1024 * 1024);
                let avail_gb = dev.available_bytes / (1024 * 1024 * 1024);
                let asset_data = AssetData {
                    config: format!(
                        "platform={platform},fs={},type={:?}",
                        dev.filesystem, dev.storage_type,
                    )
                    .into_bytes(),
                    definition: format!(
                        "storage:{}:total={},available={}",
                        dev.mount_point, dev.total_bytes, dev.available_bytes,
                    )
                    .into_bytes(),
                    metadata: format!(
                        "device={},usage={:.1}%",
                        dev.device, dev.usage_percent,
                    )
                    .into_bytes(),
                };
                assets.push(AssetRegistration::from_asset_data(
                    &asset_data,
                    NetworkScope::Global,
                    AssetCategory::BaseSystem(BaseSystemType::Storage),
                ));
                info!(
                    "Hardware: Storage {} ({} GB total, {} GB free, {:?})",
                    dev.mount_point, total_gb, avail_gb, dev.storage_type,
                );
            }
        }
        Err(e) => warn!("Storage detection failed: {e}"),
    }

    // Network asset (platform-level; individual interfaces are not enumerated
    // by OsAbstraction, so we register a single network presence asset)
    let asset_data = AssetData {
        config: format!("platform={platform}").into_bytes(),
        definition: b"network:ipv6:loopback=::1".to_vec(),
        metadata: b"interface=lo".to_vec(),
    };
    assets.push(AssetRegistration::from_asset_data(
        &asset_data,
        NetworkScope::Global,
        AssetCategory::BaseSystem(BaseSystemType::Network),
    ));
    info!("Hardware: Network interface registered");

    // GPU assets (optional, many nodes have none)
    match os.detect_gpu() {
        Ok(gpus) => {
            for gpu in &gpus {
                let mem_str = gpu
                    .memory_bytes
                    .map(|m| format!("{} MB", m / (1024 * 1024)))
                    .unwrap_or_else(|| "unknown".to_string());
                let asset_data = AssetData {
                    config: format!(
                        "platform={platform},vendor={},type={:?}",
                        gpu.vendor, gpu.gpu_type,
                    )
                    .into_bytes(),
                    definition: format!("gpu:{}:{}", gpu.model, mem_str).into_bytes(),
                    metadata: format!(
                        "capabilities={}",
                        gpu.capabilities.join(","),
                    )
                    .into_bytes(),
                };
                assets.push(AssetRegistration::from_asset_data(
                    &asset_data,
                    NetworkScope::Global,
                    AssetCategory::BaseSystem(BaseSystemType::Gpu),
                ));
                info!("Hardware: GPU {} ({}, {})", gpu.model, gpu.vendor, mem_str);
            }
        }
        Err(e) => warn!("GPU detection skipped: {e}"),
    }

    if assets.is_empty() {
        anyhow::bail!("Hardware assessment found zero assets — cannot satisfy R1");
    }

    info!("Hardware assessment complete: {} asset(s) detected", assets.len());
    Ok(assets)
}

/// Build a StateProof for hardware asset registration using real OS data.
///
/// Per R1: hardware assessed, not self-reported.
/// Per R2: four proofs from actual hardware measurements.
fn build_hardware_state_proof(node_id: &str, coordinate: MatrixCoordinate) -> StateProof {
    match create_os_abstraction() {
        Ok(os) => {
            let hw = HardwareAssessment::from_os(os.as_ref(), node_id, coordinate);
            generate_genesis_proof(&hw)
        }
        Err(e) => {
            warn!("OS abstraction unavailable ({e}), using fallback hardware values");
            let hw = HardwareAssessment {
                cpu_cores: num_cpus::get() as u32,
                cpu_mhz: 1000,
                memory_bytes: 4 * 1024 * 1024 * 1024,
                storage_bytes: 50 * 1024 * 1024 * 1024,
                storage_available_bytes: 25 * 1024 * 1024 * 1024,
                node_id: node_id.to_string(),
                coordinate,
            };
            generate_genesis_proof(&hw)
        }
    }
}

/// Build an `AssetRegistration` for the node's FALCON-1024 + Kyber-1024 identity.
///
/// Per R1 (sovereign genesis with asset instantiation) and R10 (universal asset
/// model), the identity keypair is registered as a blockchain asset. The content
/// hash is `BLAKE3(Identity_type_id || falcon_pubkey || kyber_pubkey)` matching
/// the `IdentityAssetRecord` specification in `hypermesh_lib::asset`.
fn build_identity_asset_registration(
    identity: &blockmatrix::identity::FalconIdentity,
) -> AssetRegistration {
    let asset_data = AssetData {
        config: Vec::new(),
        definition: identity.public_key.clone(),
        metadata: identity.kyber_public_key.clone(),
    };
    AssetRegistration::from_asset_data(
        &asset_data,
        NetworkScope::Global,
        AssetCategory::BaseSystem(BaseSystemType::Identity),
    )
}

/// Run the Store subcommand: ingest a file through the asset pipeline and
/// persist the resulting shards + shard map locally.
///
/// When `dist_ctx` is provided (node running with active network), shards are
/// also distributed to connected peers after local storage.
async fn run_store(path: std::path::PathBuf, dist_ctx: Option<&ShardDistributionCtx>) -> Result<()> {
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

    // 8. Distribute shards to network peers (if running inside a live node)
    if let Some(ctx) = dist_ctx {
        let shard_pairs: Vec<(ContentHash, Vec<u8>)> = processed
            .shards
            .iter()
            .map(|s| {
                let hash_bytes = *blake3::hash(&s.data).as_bytes();
                (ContentHash(hash_bytes), s.data.clone())
            })
            .collect();
        distribute_shards_to_network(ctx, &shard_pairs).await;
    } else {
        debug!("Standalone store: no network available for shard distribution");
    }

    // 9. Print shard map JSON to stdout
    println!("{map_json}");
    info!("Asset ID: {}", asset_id);

    Ok(())
}

/// Run the Fetch subcommand: load a shard map from disk, reconstruct the
/// original file through the reverse pipeline, and write the output.
///
/// For each shard in the map:
///   1. Try local ShardStore (if dist_ctx available)
///   2. Try reading from disk file
///   3. Try network fetch from connected peers (if dist_ctx available)
///   4. BLAKE3 verify every fetched shard
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

    // 2. Load shard data — try disk first, network fallback later
    let shards_dir = maps_dir.join(&asset_id);
    let mut shards: Vec<Shard> = Vec::with_capacity(map.shard_count);
    let mut network_fetched = 0usize;

    for (i, hash_hex) in map.shard_hashes.iter().enumerate() {
        let expected_hash = hex::decode(hash_hex)
            .with_context(|| format!("invalid shard hash hex at index {i}"))?;

        // Strategy 1: Read from local disk
        let shard_path = shards_dir.join(hash_hex);
        let shard_data = match std::fs::read(&shard_path) {
            Ok(data) => {
                // BLAKE3 verify
                let computed = blake3::hash(&data);
                if computed.as_bytes() != expected_hash.as_slice() {
                    warn!(
                        "Shard {i} BLAKE3 mismatch on disk — expected {}, got {}",
                        hash_hex,
                        hex::encode(computed.as_bytes()),
                    );
                    None // fall through to network fetch
                } else {
                    Some(data)
                }
            }
            Err(_) => None, // not on disk, try network
        };

        let shard_data = match shard_data {
            Some(data) => data,
            None => {
                // Strategy 2: Try network fetch from connected peers
                warn!("Shard {i} not available locally, attempting network fetch");
                fetch_shard_from_network(hash_hex, &expected_hash)
                    .await
                    .with_context(|| {
                        format!("shard {i} ({hash_hex}) not available locally or on network")
                    })?
            }
        };

        if shard_data.is_empty() {
            anyhow::bail!("shard {i} ({hash_hex}) returned empty data");
        }

        // Track network-fetched count
        if !shard_path.exists() {
            network_fetched += 1;
        }

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

    if network_fetched > 0 {
        info!("Fetched {} shard(s) from network peers", network_fetched);
    }
    info!("Loaded {} shards total", shards.len());

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
        content_hash: [0u8; 32],
        proof_hash: [0u8; 32],
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

/// Attempt to fetch a shard from connected network peers via IPC daemon.
///
/// Tries to connect to the local IPC daemon and request the shard. If the
/// daemon is not running or no peer has the shard, returns an error.
async fn fetch_shard_from_network(hash_hex: &str, expected_hash: &[u8]) -> Result<Vec<u8>> {
    // Connect to local IPC daemon to request network shard fetch
    let client = ipc::IpcClient::new();
    if !client.is_daemon_running().await {
        anyhow::bail!("no daemon running for network shard fetch");
    }

    // Request shard via IPC — the daemon has access to connected peers
    let resp = client
        .call_ok(
            "shard.fetch",
            serde_json::json!({ "shard_id": hash_hex }),
        )
        .await
        .map_err(|e| anyhow!("IPC shard fetch failed: {e}"))?;

    let shard_hex = resp["data"]
        .as_str()
        .ok_or_else(|| anyhow!("shard.fetch response missing 'data' field"))?;
    let shard_data =
        hex::decode(shard_hex).map_err(|e| anyhow!("invalid shard data hex: {e}"))?;

    // BLAKE3 verify
    let computed = blake3::hash(&shard_data);
    if computed.as_bytes() != expected_hash {
        anyhow::bail!(
            "network shard BLAKE3 mismatch: expected {}, got {}",
            hash_hex,
            hex::encode(computed.as_bytes()),
        );
    }

    info!("Fetched shard {} from network via IPC", hash_hex);
    Ok(shard_data)
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

/// Scan the blockchain for DNS-typed block entries and register them
/// in the local resolver. Called on startup after blockchain is loaded
/// from persistence, so that DNS names propagated from peers are available.
async fn extract_dns_from_blockchain(
    dns: &blockmatrix::bootstrap::DnsResolver,
    bootstrap: &NodeBootstrap,
) {
    let chain = bootstrap.blockchain().get_chain().await;
    let mut count = 0u64;

    for block in &chain {
        for entry in &block.entries {
            let is_dns = matches!(
                entry.registration.category,
                AssetCategory::BaseSystem(BaseSystemType::Dns)
            );
            if !is_dns {
                continue;
            }

            let dns_json = match &entry.storage_pointer {
                blockmatrix::blockchain::block::StoragePointer::Local { path } => path.as_str(),
                _ => continue,
            };

            let dns_entry: blockmatrix::dns::DnsBlockEntry = match serde_json::from_str(dns_json) {
                Ok(e) => e,
                Err(_) => continue,
            };

            let ip_addr = match &dns_entry.record_data {
                blockmatrix::dns::DnsRecordData::AAAA(addr) => std::net::IpAddr::V6(*addr),
                _ => continue,
            };

            dns.register(dns_entry.domain_name, ip_addr).await;
            count += 1;
        }
    }

    if count > 0 {
        info!("Extracted {count} DNS record(s) from blockchain");
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

/// Path to persisted domain registrations for a given node.
fn domain_registrations_path(data_dir: &std::path::Path, node_id: &str) -> std::path::PathBuf {
    data_dir.join(node_id).join("domain_registrations.json")
}

/// Load domain registrations from disk.
fn load_domain_registrations(
    data_dir: &std::path::Path,
    node_id: &str,
) -> Vec<DomainRegistration> {
    let path = domain_registrations_path(data_dir, node_id);
    if !path.exists() {
        return Vec::new();
    }
    match blockmatrix::dns::domain::load_domains(&path) {
        Ok(domains) => domains,
        Err(e) => {
            warn!("Failed to load domain registrations: {e}");
            Vec::new()
        }
    }
}

/// Save domain registrations to disk.
fn save_domain_registrations(
    data_dir: &std::path::Path,
    node_id: &str,
    domains: &[DomainRegistration],
) -> Result<()> {
    let path = domain_registrations_path(data_dir, node_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    blockmatrix::dns::domain::save_domains(domains, &path)
        .context("failed to save domain registrations")
}

/// Run the Domain subcommand: register, create, list, nodes, or invite.
async fn run_domain(
    action: DomainAction2,
    bootstrap: &NodeBootstrap,
    data_dir: &std::path::Path,
    node_id: &str,
) -> Result<()> {
    match action {
        DomainAction2::Register { name, privacy } => {
            let privacy_mode: PrivacyMode = privacy.into();
            let mut domains = load_domain_registrations(data_dir, node_id);

            // Check for duplicate
            if domains.iter().any(|d| d.domain_name == name) {
                anyhow::bail!("Domain '{}' is already registered on this node", name);
            }

            let reg = DomainRegistration::new(&name, privacy_mode, node_id.to_string());

            // Register as blockchain asset (DNS-as-asset, R10)
            let dns_data_str = format!("DOMAIN:REGISTER:{name}");
            let asset_data = AssetData {
                config: dns_data_str.as_bytes().to_vec(),
                definition: format!("domain-registration:{name}").into_bytes(),
                metadata: format!("network_id={},privacy={privacy_mode:?}", reg.network_id)
                    .into_bytes(),
            };
            let registration = AssetRegistration::from_asset_data(
                &asset_data,
                NetworkScope::Global,
                AssetCategory::BaseSystem(BaseSystemType::Dns),
            );
            let state_proof = StateProof::generate_from_network(node_id)
                .await
                .context("PoS proof generation failed for domain registration")?;
            let block = bootstrap
                .blockchain()
                .register_asset_record(registration, &state_proof)
                .await
                .map_err(|e| anyhow::anyhow!("blockchain write failed: {e}"))?;

            domains.push(reg.clone());
            save_domain_registrations(data_dir, node_id, &domains)?;

            println!();
            println!("  Domain Registered");
            println!("  -----------------");
            println!("  domain:     {name}");
            println!("  network_id: {}", reg.network_id);
            println!("  privacy:    {privacy_mode:?}");
            println!("  block:      #{}", block.index);
            println!();
        }
        DomainAction2::Create { name, privacy } => {
            let privacy_mode: PrivacyMode = privacy.into();
            let mut domains = load_domain_registrations(data_dir, node_id);

            if domains.iter().any(|d| d.domain_name == name) {
                anyhow::bail!("Domain '{}' is already registered on this node", name);
            }

            // Check parent domain exists
            if let Some(dot_pos) = name.find('.') {
                let parent = &name[dot_pos + 1..];
                if !parent.is_empty() && !domains.iter().any(|d| d.domain_name == parent) {
                    warn!(
                        "Parent domain '{}' not registered on this node (proceeding anyway)",
                        parent
                    );
                }
            }

            let reg = DomainRegistration::new(&name, privacy_mode, node_id.to_string());

            let dns_data_str = format!("DOMAIN:CREATE:{name}");
            let asset_data = AssetData {
                config: dns_data_str.as_bytes().to_vec(),
                definition: format!("domain-subdomain:{name}").into_bytes(),
                metadata: format!("network_id={},privacy={privacy_mode:?}", reg.network_id)
                    .into_bytes(),
            };
            let registration = AssetRegistration::from_asset_data(
                &asset_data,
                NetworkScope::Global,
                AssetCategory::BaseSystem(BaseSystemType::Dns),
            );
            let state_proof = StateProof::generate_from_network(node_id)
                .await
                .context("PoS proof generation failed for subdomain creation")?;
            let block = bootstrap
                .blockchain()
                .register_asset_record(registration, &state_proof)
                .await
                .map_err(|e| anyhow::anyhow!("blockchain write failed: {e}"))?;

            domains.push(reg.clone());
            save_domain_registrations(data_dir, node_id, &domains)?;

            println!();
            println!("  Sub-Domain Created");
            println!("  ------------------");
            println!("  domain:     {name}");
            println!("  network_id: {}", reg.network_id);
            println!(
                "  parent:     {}",
                reg.parent_network_id.as_deref().unwrap_or("(none)")
            );
            println!("  privacy:    {privacy_mode:?}");
            println!("  block:      #{}", block.index);
            println!();
        }
        DomainAction2::List => {
            let domains = load_domain_registrations(data_dir, node_id);
            if domains.is_empty() {
                println!("No domains registered.");
            } else {
                println!();
                println!("  Registered Domains");
                println!("  ------------------");
                for d in &domains {
                    println!(
                        "  {:<30} net={} privacy={:?}",
                        d.domain_name,
                        &d.network_id[..16],
                        d.privacy_mode,
                    );
                }
                println!();
            }
        }
        DomainAction2::Nodes { domain } => {
            // In offline mode, we can only show local info
            let domains = load_domain_registrations(data_dir, node_id);
            let found = domains.iter().find(|d| d.domain_name == domain);
            match found {
                Some(d) => {
                    println!();
                    println!("  Domain: {}", d.domain_name);
                    println!("  Network ID: {}", d.network_id);
                    println!("  Owner: {}", d.owner_node_id);
                    println!(
                        "  Members: (local node only — connect for network view)"
                    );
                    println!();
                }
                None => {
                    println!("Domain '{}' not found in local registrations.", domain);
                }
            }
        }
        DomainAction2::Invite { domain, peer, ttl } => {
            let domains = load_domain_registrations(data_dir, node_id);
            let found = domains.iter().find(|d| d.domain_name == domain);
            let reg = match found {
                Some(d) => d,
                None => {
                    anyhow::bail!(
                        "Domain '{}' not registered on this node. Register it first.",
                        domain
                    );
                }
            };

            // Use state_proof_bytes or node_id as keying material
            let proof_bytes = reg
                .state_proof_bytes
                .as_deref()
                .unwrap_or(node_id.as_bytes());

            let invitee = if peer == "open" { None } else { Some(peer.as_str()) };
            let inv = invitation::create_invitation(&domain, proof_bytes, invitee, ttl);
            let token = invitation::encode_invitation(&inv)
                .map_err(|e| anyhow::anyhow!("failed to encode invitation: {e}"))?;

            println!();
            println!("  Domain Invitation");
            println!("  -----------------");
            println!("  domain:  {domain}");
            println!("  peer:    {}", if peer == "open" { "(open)" } else { &peer });
            println!("  expires: {} seconds", ttl);
            println!("  token:");
            println!("  {token}");
            println!();
        }
    }
    Ok(())
}

/// Run the Join subcommand: join a domain network (optionally with invitation).
async fn run_join(
    network: &str,
    invite_token: Option<&str>,
    node_id: &str,
    data_dir: &std::path::Path,
) -> Result<()> {
    // Validate invitation if provided
    if let Some(token_str) = invite_token {
        let inv = invitation::decode_invitation(token_str)
            .map_err(|e| anyhow::anyhow!("Invalid invitation: {e}"))?;

        if inv.domain_name != network {
            anyhow::bail!(
                "Invitation is for domain '{}', not '{}'",
                inv.domain_name,
                network
            );
        }

        if !inv.invitee_node_id.is_empty() && inv.invitee_node_id != node_id {
            anyhow::bail!(
                "Invitation is for node '{}', not this node ('{}')",
                inv.invitee_node_id,
                node_id
            );
        }

        info!("Invitation validated for domain '{}'", network);
    }

    // Record membership intent locally
    let domains = load_domain_registrations(data_dir, node_id);
    let network_id = blockmatrix::dns::domain::derive_network_id(network);

    println!();
    println!("  Join Domain Network");
    println!("  -------------------");
    println!("  domain:     {network}");
    println!("  network_id: {network_id}");
    if domains.iter().any(|d| d.domain_name == network) {
        println!("  status:     already registered (owner)");
    } else {
        println!("  status:     membership recorded (connect daemon to sync)");
    }
    println!();

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

            // 3. Register on local blockchain (DNS-as-asset, R10)
            let bc = bootstrap.blockchain();

            // Build DnsBlockEntry so peers can extract the record from the block
            let ipv6_addr = match target_addr {
                std::net::IpAddr::V6(v6) => v6,
                std::net::IpAddr::V4(v4) => v4.to_ipv6_mapped(),
            };
            let dns_entry = blockmatrix::dns::DnsBlockEntry {
                domain_name: name.clone(),
                record_type: blockmatrix::dns::DnsRecordType::AAAA,
                record_data: blockmatrix::dns::DnsRecordData::AAAA(ipv6_addr),
                ttl: 300,
                owner: node_id.to_string(),
            };
            let dns_bytes = serde_json::to_vec(&dns_entry)
                .context("failed to serialize DNS entry")?;

            let asset_data = AssetData {
                config: name.as_bytes().to_vec(),
                definition: dns_bytes.clone(),
                metadata: Vec::new(),
            };
            let registration = AssetRegistration::from_asset_data(
                &asset_data,
                NetworkScope::Global,
                AssetCategory::BaseSystem(BaseSystemType::Dns),
            );
            let state_proof = StateProof::generate_from_network(node_id)
                .await
                .context("PoS proof generation failed for DNS registration")?;
            let block = bc
                .register_dns_asset(registration, &state_proof, dns_bytes)
                .await
                .map_err(|e| anyhow::anyhow!("blockchain write failed: {e}"))?;

            // DNS block propagation requires a running node with active
            // network connections (Commands::Connect). Standalone `dns register`
            // writes locally; the block will be propagated on next node start
            // via the bootstrap block propagation loop.
            info!(
                "DNS block #{} stored locally (propagation deferred to next node start)",
                block.index,
            );

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

/// Run the connect/start flow: initialize STOQ, network, sync loops, IPC server,
/// then wait for Ctrl+C or IPC shutdown.
async fn run_connect(
    cli: &Cli,
    coord: MatrixCoordinate,
    nid: &str,
    data_dir: &std::path::Path,
    bootstrap: &NodeBootstrap,
    persistence: std::sync::Arc<PersistenceManager>,
) -> Result<()> {
    info!("Starting node services...");

    // Set initial privacy mode if different from default
    let target_mode = cli.privacy.into();
    if bootstrap.privacy_mode().await != target_mode {
        bootstrap.set_privacy_mode(target_mode).await?;
    }

    // Network manager reference (populated if STOQ starts)
    let mut network_ref: Option<std::sync::Arc<NetworkManager>> = None;
    // Shard store reference (populated if network starts, otherwise standalone)
    let mut shard_store_ref: Option<std::sync::Arc<ShardStore>> = None;
    // STOQ transport reference (populated if network starts, used for API bridge)
    let mut transport_ref: Option<std::sync::Arc<stoq::StoqTransport>> = None;

    // Initialize STOQ transport for any mode that needs networking.
    // Private mode starts STOQ when bootstrap peers are provided (bounded network).
    // Only skip STOQ for purely local device-scope operation (private with no peers).
    let privacy_mode = bootstrap.privacy_mode().await;
    let has_bootstrap_peers = !cli.bootstrap.is_empty();
    if privacy_mode != PrivacyMode::PRIVATE || has_bootstrap_peers {
        info!("Initializing STOQ transport on port {}", cli.stoq_port);

        // Configure STOQ transport based on privacy mode.
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
            info!("Public mode: self-issuing certificate via local TrustChain");
            NetworkType::P2P
        } else {
            info!("Private mode: self-issuing certificate via local TrustChain");
            NetworkType::P2P
        };

        // Initialize STOQ with network-aware certificate strategy
        let transport = std::sync::Arc::new(
            stoq::StoqTransport::new_for_network(stoq_config, network_type).await?,
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
        let shard_transport =
            std::sync::Arc::new(StoqShardTransport::new(transport.clone()));
        info!(
            "Shard store and transport initialized (store={} shards)",
            shard_store.count().await
        );

        // Load or create FALCON-1024 node identity for PoS signing
        let identity_dir = data_dir.join(nid).join("identity");
        let falcon_identity = blockmatrix::identity::FalconIdentity::load_or_create(&identity_dir)?;
        info!(
            "Node identity: {}... (FALCON-1024)",
            &falcon_identity.node_id[..16]
        );

        // Create network manager with trait-based identity and proof provider
        let signer: std::sync::Arc<dyn hypermesh_lib::NodeSigner> =
            std::sync::Arc::new(falcon_identity);
        let proof_provider: std::sync::Arc<dyn hypermesh_lib::StateProofProvider> =
            std::sync::Arc::new(
                blockmatrix::proof_of_state::BlockMatrixProofProvider::new(
                    signer.node_id().to_string(),
                ),
            );
        let network_manager = NetworkManager::new(
            coord,
            transport.clone(),
            privacy_mode,
            bootstrap_nodes,
            signer,
            proof_provider,
        )
        .await?;

        // Start discovery based on privacy mode
        network_manager.start_discovery().await?;

        // --- Block Sync Infrastructure ---
        let node_map: std::sync::Arc<
            tokio::sync::RwLock<
                std::collections::HashMap<String, (String, std::net::SocketAddr)>,
            >,
        > = std::sync::Arc::new(tokio::sync::RwLock::new(
            std::collections::HashMap::new(),
        ));

        let block_transport = std::sync::Arc::new(StoqBlockTransportAdapter::new(
            transport.clone(),
            node_map.clone(),
        ));

        let block_propagator = std::sync::Arc::new(tokio::sync::Mutex::new(
            BlockPropagator::with_transport(
                coord,
                PropagationStrategy::NearestN(6),
                block_transport.clone(),
            ),
        ));

        let genesis_hash = bootstrap.genesis_block().hash.clone();
        let sync_manager = std::sync::Arc::new(tokio::sync::Mutex::new(
            SyncManager::new(genesis_hash.clone(), SyncConfig::default()),
        ));

        let reflector_pool = std::sync::Arc::new(tokio::sync::Mutex::new(
            ReflectorPool::new(ReflectorConfig::default()),
        ));

        // Join a Network scope chain if we have peers or are a reflector
        if has_bootstrap_peers || cli.reflector {
            let network_id = format!(
                "public-{}",
                &genesis_hash[..16.min(genesis_hash.len())]
            );
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            match sync_manager
                .lock()
                .await
                .join_network(network_id.clone(), privacy_mode, now_secs)
            {
                Ok(()) => info!("Joined Network scope chain: {}", network_id),
                Err(e) => warn!("Failed to join network scope: {}", e),
            }
        }

        info!("Block sync infrastructure initialized (propagation=NearestN(6))");

        // Create peer context for receive-side message handling
        let connected_peer_coords = std::sync::Arc::new(
            tokio::sync::RwLock::new(Vec::<blockmatrix::matrix::coordinate::MatrixCoordinate>::new()),
        );
        let peer_ctx = std::sync::Arc::new(blockmatrix::network::PeerContext {
            blockchain: bootstrap.blockchain().clone(),
            shard_store: shard_store.clone(),
            sync_manager: sync_manager.clone(),
            reflector_pool: reflector_pool.clone(),
            block_propagator: block_propagator.clone(),
            our_coordinate: coord,
            node_id: nid.to_string(),
            blockchain_scope: if has_bootstrap_peers || cli.reflector {
                hypermesh_lib::BlockchainScope::Network
            } else {
                hypermesh_lib::BlockchainScope::Device
            },
            spatial_bucket_assigner: None,
            connected_peer_coords: connected_peer_coords.clone(),
            dns_resolver: Some(bootstrap.dns().clone()),
        });

        // Start message loops for peers connected during discovery (before PeerContext existed)
        let network_clone = std::sync::Arc::new(network_manager);
        network_clone.start_peer_message_loops(peer_ctx.clone()).await;

        // Register discovered peers as reflectors so sync requests have targets
        if has_bootstrap_peers || cli.reflector {
            let discovered_peers = network_clone.get_connected_nodes().await;
            let network_id = format!(
                "public-{}",
                &genesis_hash[..16.min(genesis_hash.len())]
            );
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let mut rp = reflector_pool.lock().await;
            for peer in &discovered_peers {
                let reflector = blockmatrix::network::reflector_pool::Reflector {
                    node_id: peer.node_id.clone(),
                    position: hypermesh_lib::MatrixPosition {
                        x: peer.coordinate.x as f64,
                        y: peer.coordinate.y as f64,
                        z: peer.coordinate.z as f64,
                    },
                    last_seen: now_secs,
                    block_height: 0,
                    health_score: 1.0,
                    privacy_mode,
                };
                rp.register_reflector(&network_id, reflector);
                info!(
                    "Registered peer {} as reflector for {}",
                    &peer.node_id[..8.min(peer.node_id.len())],
                    &network_id,
                );
            }
            drop(rp);
        }

        // Immediately seed the block transport node_map so propagation works
        // (the 5s sync loop would otherwise leave it empty for the first cycle)
        let addr_map = network_clone.get_node_address_map().await;
        *node_map.write().await = addr_map;

        // Start accepting connections in background (with peer context for message loop)
        let network_accept = network_clone.clone();
        let ctx_accept = peer_ctx.clone();
        tokio::spawn(async move {
            if let Err(e) = network_accept.accept_connections(Some(ctx_accept)).await {
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
        let network_status = network_clone.clone();
        let shard_transport_sync = shard_transport.clone();
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(10));
            loop {
                interval.tick().await;

                let nodes = network_status.get_connected_nodes().await;
                for node in &nodes {
                    let node_id = hypermesh_lib::NodeId::from_bytes(
                        *blake3::hash(node.node_id.as_bytes()).as_bytes(),
                    );
                    shard_transport_sync
                        .register_node_address(&node_id, node.address)
                        .await;
                }

                let node_count = nodes.len();
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

        // Block sync + reflector heartbeat loop + metrics reporting
        let sync_mgr_loop = sync_manager.clone();
        let refl_pool_loop = reflector_pool.clone();
        let blockchain_sync = bootstrap.blockchain().clone();
        let network_sync = network_clone.clone();
        let node_map_sync = node_map.clone();
        let block_transport_sync = block_transport.clone();
        let is_reflector = cli.reflector;
        let sync_coord = coord;
        let shard_store_metrics = shard_store.clone();
        let peer_coords_sync = connected_peer_coords.clone();
        let metrics_node_id = nid.to_string();
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(5));
            let mut metrics_reporter =
                blockmatrix::network::MetricsReporter::new(metrics_node_id);
            let os_abs = create_os_abstraction().ok();
            let mut cycle_count: u64 = 0;
            loop {
                interval.tick().await;
                cycle_count += 1;

                let addr_map = network_sync.get_node_address_map().await;
                *node_map_sync.write().await = addr_map;

                // Keep PeerContext's connected coordinates in sync for re-propagation
                let live_coords = network_sync.get_connected_coordinates().await;
                *peer_coords_sync.write().await = live_coords;

                let chain = blockchain_sync.get_chain().await;
                let provider = NodeBlockchainBlockProvider::from_blocks(&chain);
                let local_height = blockchain_sync.get_height().await;

                {
                    let mut sm = sync_mgr_loop.lock().await;
                    let rp = refl_pool_loop.lock().await;
                    let synced = TransportSyncDriver::run_sync_round(
                        &mut sm,
                        &rp,
                        Some(&provider),
                        block_transport_sync.as_ref(),
                        local_height,
                        &sync_coord,
                    )
                    .await;
                    if synced > 0 {
                        info!("Sync round: {} network(s) synchronized", synced);
                    }
                }

                {
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    let pruned = refl_pool_loop.lock().await.prune_stale(now_ms);
                    if pruned > 0 {
                        debug!("Pruned {} stale reflector(s)", pruned);
                    }
                }

                if is_reflector {
                    debug!("Reflector heartbeat: height={}", local_height);

                    // Broadcast reflector heartbeat to connected peers
                    let genesis_hash = blockchain_sync.get_head().await
                        .map(|b| b.hash.clone())
                        .unwrap_or_default();
                    let network_id = format!(
                        "public-{}",
                        &genesis_hash[..16.min(genesis_hash.len())]
                    );

                    let heartbeat_msg = blockmatrix::network::stoq_integration::MatrixMessage::ReflectorHeartbeat {
                        network_id,
                        block_height: local_height,
                        health_score: 1.0,
                    };

                    if let Ok(heartbeat_json) = serde_json::to_vec(&heartbeat_msg) {
                        let mut tagged = Vec::with_capacity(1 + heartbeat_json.len());
                        tagged.push(0x10u8); // TAG_SYNC_MESSAGE
                        tagged.extend_from_slice(&heartbeat_json);

                        let nodes = network_sync.get_connected_nodes().await;
                        for node in &nodes {
                            if let Some(ref conn) = node.connection {
                                match conn.open_stream().await {
                                    Ok(mut stream) => {
                                        if let Err(e) = stream.send(&tagged).await {
                                            debug!("Heartbeat send to {} failed: {}", &node.node_id[..8.min(node.node_id.len())], e);
                                        }
                                    }
                                    Err(e) => {
                                        debug!("Heartbeat stream to {} failed: {}", &node.node_id[..8.min(node.node_id.len())], e);
                                    }
                                }
                            }
                        }
                    }
                }

                // Emit node metrics every 30s (6 sync cycles)
                if cycle_count % 6 == 0 {
                    let chain_h = blockchain_sync.get_height().await;
                    let peers = network_sync.get_connected_nodes().await.len();
                    let shards = shard_store_metrics.count().await;
                    let (cpu, mem) = os_abs
                        .as_ref()
                        .and_then(|os| os.get_resource_usage().ok())
                        .map(|u| (u.cpu_usage_percent, u.memory_usage_percent))
                        .unwrap_or((0.0, 0.0));
                    let _frame_bytes = metrics_reporter.build_capacity_frame(
                        chain_h, peers, shards, cpu, mem,
                    );
                    // TODO: Push frame_bytes to engauge STOQ API at [::1]:9296
                    // when engauge is running as a co-located service
                }
            }
        });
        info!(
            "Block sync loop started (interval=5s, reflector={}, metrics_interval=30s)",
            cli.reflector
        );

        // Propagate any blocks created during bootstrap
        {
            let chain = bootstrap.blockchain().get_chain().await;
            for block in chain.iter().filter(|b| b.index > 0) {
                propagate_block(block, &block_propagator, &network_clone).await;

                let dns_count = count_dns_assets_in_block(block);
                if dns_count > 0 {
                    debug!(
                        "Block #{} contains {} DNS asset(s)",
                        block.index, dns_count,
                    );
                }
            }
        }

        // Keep infrastructure alive for the node's lifetime
        let _shard_transport = shard_transport;
        let _block_propagator = block_propagator.clone();
        let _sync_manager = sync_manager.clone();
        let _reflector_pool = reflector_pool.clone();

        network_ref = Some(network_clone);
        shard_store_ref = Some(shard_store);
        transport_ref = Some(transport);
    }

    // --- IPC Server Setup ---
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);

    // Use network shard store if available, otherwise create a standalone one
    let daemon_shard_store = shard_store_ref.unwrap_or_else(|| std::sync::Arc::new(ShardStore::new()));

    let daemon_state = std::sync::Arc::new(ipc::DaemonState {
        blockchain: bootstrap.blockchain().clone(),
        persistence: persistence.clone(),
        network: network_ref,
        shard_store: daemon_shard_store,
        coordinate: coord,
        node_id: nid.to_string(),
        data_dir: data_dir.to_path_buf(),
        privacy_mode: format!("{:?}", bootstrap.privacy_mode().await),
        started_at: std::time::Instant::now(),
        shutdown_tx: shutdown_tx.clone(),
        dns_resolver: bootstrap.dns().clone(),
    });

    let mut handler = ipc::RequestHandler::new();
    ipc::register_all(&mut handler, daemon_state.clone());

    let handler = std::sync::Arc::new(handler);

    let ipc_server = match ipc::IpcServer::new(handler.clone()) {
        Ok(server) => {
            let server = std::sync::Arc::new(server);
            let server_run = server.clone();
            tokio::spawn(async move {
                if let Err(e) = server_run.run().await {
                    warn!("IPC server error: {}", e);
                }
            });
            info!("IPC server started");
            Some(server)
        }
        Err(e) => {
            warn!("Failed to start IPC server: {e}");
            None
        }
    };

    // STOQ API bridge: handlers registered but listen loop disabled.
    // The STOQ API server's listen() would compete with accept_connections()
    // for incoming QUIC connections on the same transport. Until we add protocol
    // discrimination (API vs peer handshake), API access uses IPC (Unix socket)
    // and Gateway proxies external requests to IPC.
    // TODO: Add stream-level protocol discriminator so API and peer connections
    // can share the same STOQ port, or use a dedicated API port.
    info!("API access available via IPC (Unix socket). STOQ API bridge deferred until protocol discriminator is implemented.");

    // Register default system dashboard as a blockchain asset if none exists
    {
        use blockmatrix::dashboard::deploy;
        let chain = bootstrap.blockchain().get_chain().await;
        if deploy::find_active_dashboard(&chain).is_none() {
            info!("Registering default system dashboard as blockchain asset...");

            let mut files = std::collections::BTreeMap::new();

            // Try to load the built React UI from ui/frontend/dist/
            // Check multiple possible locations relative to the binary
            let ui_dist_candidates = [
                std::path::PathBuf::from("ui/frontend/dist"),
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.join("../ui/frontend/dist")))
                    .unwrap_or_default(),
                data_dir.join("ui/dist"),
            ];

            let mut loaded_ui = false;
            for ui_dist in &ui_dist_candidates {
                if ui_dist.join("index.html").exists() {
                    match deploy::collect_dir_files(ui_dist) {
                        Ok(ui_files) if !ui_files.is_empty() => {
                            info!("Loading UI from {}", ui_dist.display());
                            for (path, content) in &ui_files {
                                files.insert(format!("private/{path}"), content.clone());
                            }
                            loaded_ui = true;
                            break;
                        }
                        _ => {}
                    }
                }
            }

            // Fallback: use embedded placeholder HTML
            if !loaded_ui {
                info!("Built UI not found, using embedded fallback dashboard");
                files.insert(
                    "private/index.html".to_string(),
                    blockmatrix::dashboard::default::DEFAULT_PRIVATE_HTML.as_bytes().to_vec(),
                );
            }

            // Public scope: always use the embedded onboarding page
            files.insert(
                "public/index.html".to_string(),
                blockmatrix::dashboard::default::DEFAULT_PUBLIC_HTML.as_bytes().to_vec(),
            );

            let bundle = deploy::bundle_files(&files);

            let manifest_toml = r#"[dashboard]
name = "default"
version = "1.0.0"
description = "Default HyperMesh node dashboard"
domain = "localhost.hypermesh"

[access]
public = "public"
private = "private"
"#;

            // Register as Dashboard asset on blockchain
            let asset_data = blockmatrix::assets::core::AssetData {
                config: b"DASHBOARD:DEPLOY:default".to_vec(),
                definition: bundle.clone(),
                metadata: manifest_toml.as_bytes().to_vec(),
            };
            let registration = blockmatrix::assets::core::AssetRegistration::from_asset_data(
                &asset_data,
                blockmatrix::assets::core::NetworkScope::Global,
                blockmatrix::assets::core::AssetCategory::BaseSystem(
                    blockmatrix::assets::core::BaseSystemType::Dashboard,
                ),
            );
            let content_hash = registration.content_hash;
            let state_proof = build_hardware_state_proof(&nid, coord);
            match bootstrap
                .blockchain()
                .register_asset_record(registration, &state_proof)
                .await
            {
                Ok(block) => {
                    // Store bundle in asset store (keyed by blockchain content hash)
                    if let Err(e) = deploy::store_dashboard_bundle(
                        &data_dir, &content_hash, manifest_toml, &bundle,
                    ) {
                        warn!("Failed to store dashboard bundle: {}", e);
                    }
                    info!(
                        "Default dashboard registered as asset (block #{}, hash {})",
                        block.index,
                        hex::encode(content_hash)
                    );
                }
                Err(e) => warn!("Failed to register default dashboard: {}", e),
            }
        }
    }

    info!("Node running in {:?} mode", bootstrap.privacy_mode().await);
    info!("Press Ctrl+C to stop");

    // Wait for Ctrl+C or IPC shutdown
    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            if let Err(e) = result {
                warn!("Failed to listen for Ctrl+C: {}", e);
            }
            info!("Ctrl+C received, shutting down...");
        }
        _ = shutdown_rx.changed() => {
            info!("Shutdown requested via IPC");
        }
    }

    // Shut down IPC server
    if let Some(server) = ipc_server {
        server.shutdown();
    }

    info!("Shutting down -- flushing persistence...");
    if let Err(e) = persistence.flush().await {
        warn!("Persistence flush error: {}", e);
    }
    if let Err(e) = persistence.shutdown().await {
        warn!("Persistence shutdown error: {}", e);
    }
    info!("Persistence flushed, shutdown complete.");

    Ok(())
}

/// Load config from `--config` path or the default location.
fn load_config(cli: &Cli) -> ipc::HypermeshConfig {
    match &cli.config {
        Some(path) => ipc::HypermeshConfig::load_from(path),
        None => ipc::HypermeshConfig::load(),
    }
}

/// Parse a string value as JSON; fall back to a JSON string if it fails.
fn parse_config_value(raw: &str) -> serde_json::Value {
    serde_json::from_str(raw).unwrap_or_else(|_| serde_json::Value::String(raw.to_string()))
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut cli = Cli::parse();

    // Initialize logging
    let level = if cli.debug { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .init();

    // --- Handle Config subcommand early (no bootstrap needed) ---
    if let Some(Commands::Config { ref action }) = cli.command {
        match action {
            ConfigCommand::Show => {
                let config = load_config(&cli);
                let output = serde_json::to_string_pretty(&config)
                    .context("failed to serialize config")?;
                println!("{output}");
                return Ok(());
            }
            ConfigCommand::Get { key } => {
                let config = load_config(&cli);
                let value = serde_json::to_value(&config)
                    .context("failed to serialize config")?;
                match ipc::config::get_dotpath(&value, key) {
                    Some(v) => {
                        let output = serde_json::to_string_pretty(v)
                            .context("failed to format value")?;
                        println!("{output}");
                    }
                    None => {
                        eprintln!("Key not found: {key}");
                        std::process::exit(1);
                    }
                }
                return Ok(());
            }
            ConfigCommand::Set { key, value } => {
                let mut config = load_config(&cli);
                let mut json_value = serde_json::to_value(&config)
                    .context("failed to serialize config")?;
                let parsed = parse_config_value(value);
                ipc::config::set_dotpath(&mut json_value, key, parsed)
                    .map_err(|e| anyhow::anyhow!("failed to set key: {e}"))?;
                config = serde_json::from_value(json_value)
                    .context("invalid config after update")?;
                match &cli.config {
                    Some(path) => config.save_to(path)
                        .map_err(|e| anyhow::anyhow!("{e}"))?,
                    None => config.save()
                        .map_err(|e| anyhow::anyhow!("{e}"))?,
                }
                println!("Set {key} = {value}");
                return Ok(());
            }
            ConfigCommand::Init => {
                let config = ipc::HypermeshConfig::default();
                let path = match &cli.config {
                    Some(p) => p.clone(),
                    None => ipc::HypermeshConfig::default_path(),
                };
                config.save_to(&path)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                println!("Created {}", path.display());
                return Ok(());
            }
        }
    }

    // --- Handle Destroy subcommand (manual-only, no bootstrap, no data-dir required) ---
    if let Some(Commands::Destroy { chaotic }) = &cli.command {
        // Resolve data dir from config or CLI defaults — never requires explicit --data-dir
        let config = load_config(&cli);
        let data_dir_str = if config.node.data_dir != "~/.blockmatrix" {
            config.node.data_dir.clone()
        } else {
            cli.data_dir.clone()
        };
        let data_dir = if data_dir_str.starts_with('~') {
            dirs::home_dir()
                .context("could not determine home directory")?
                .join(&data_dir_str[2..])
        } else {
            std::path::PathBuf::from(&data_dir_str)
        };

        if !data_dir.exists() {
            eprintln!("Nothing to destroy: {} does not exist", data_dir.display());
            return Ok(());
        }

        // Find all node_* dirs under data_dir
        let mut node_dirs: Vec<std::path::PathBuf> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&data_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if name.to_string_lossy().starts_with("node_") && entry.path().is_dir() {
                    node_dirs.push(entry.path());
                }
            }
        }

        if node_dirs.is_empty() {
            eprintln!("Nothing to destroy: no node data found in {}", data_dir.display());
            return Ok(());
        }

        eprintln!("Found {} node(s) to destroy:", node_dirs.len());
        for d in &node_dirs {
            eprintln!("  {}", d.display());
        }

        if !chaotic {
            eprintln!("\nType 'yes' to confirm:");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if input.trim() != "yes" {
                eprintln!("Aborted.");
                return Ok(());
            }
        }

        for d in &node_dirs {
            std::fs::remove_dir_all(d)
                .context(format!("failed to remove {}", d.display()))?;
            println!("Destroyed {}", d.display());
        }

        // Clean IPC sockets (3-tier fallback locations)
        if let Ok(sock) = std::env::var("HYPERMESH_SOCK") {
            if std::path::Path::new(&sock).exists() {
                std::fs::remove_file(&sock).ok();
                println!("Removed socket {sock}");
            }
        }
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let sock_dir = std::path::PathBuf::from(runtime_dir).join("hypermesh");
            if sock_dir.exists() {
                std::fs::remove_dir_all(&sock_dir).ok();
                println!("Removed {}", sock_dir.display());
            }
        }
        if let Some(home) = dirs::home_dir() {
            let sock = home.join(".hypermesh").join("ctl.sock");
            if sock.exists() {
                std::fs::remove_file(&sock).ok();
                println!("Removed {}", sock.display());
            }
            // Clean legacy identity location
            let old_identity = home.join(".hypermesh").join("identity");
            if old_identity.exists() {
                std::fs::remove_dir_all(&old_identity).ok();
                println!("Cleaned legacy identity at ~/.hypermesh/identity/");
            }
        }

        return Ok(());
    }

    // --- Merge config file with CLI flags (CLI wins) ---
    let config = load_config(&cli);
    if cli.coord_x == 0 && config.node.coord_x != 0 {
        cli.coord_x = config.node.coord_x;
    }
    if cli.coord_y == 0 && config.node.coord_y != 0 {
        cli.coord_y = config.node.coord_y;
    }
    if cli.coord_z == 0 && config.node.coord_z != 0 {
        cli.coord_z = config.node.coord_z;
    }
    if cli.stoq_port == 9292 && config.network.stoq_port != 9292 {
        cli.stoq_port = config.network.stoq_port;
    }
    if cli.data_dir == "~/.blockmatrix" && config.node.data_dir != "~/.blockmatrix" {
        cli.data_dir = config.node.data_dir.clone();
    }
    if cli.bootstrap.is_empty() && !config.network.bootstrap_nodes.is_empty() {
        cli.bootstrap = config.network.bootstrap_nodes.clone();
    }
    if !cli.reflector && config.network.reflector {
        cli.reflector = true;
    }

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

        // === R1/R10: Load identity and assess hardware for genesis asset registration ===
        let identity_dir = data_dir.join(&nid).join("identity");
        let falcon_identity = blockmatrix::identity::FalconIdentity::load_or_create(&identity_dir)?;
        info!(
            "Genesis identity: {}... (FALCON-1024 + Kyber-1024)",
            &falcon_identity.node_id[..16]
        );

        info!("Assessing node hardware for asset registration (R1)...");
        match assess_hardware_assets() {
            Ok(mut hw_assets) => {
                // Register FALCON+Kyber identity as a blockchain asset (R1/R10)
                hw_assets.push(build_identity_asset_registration(&falcon_identity));

                // Use genesis proof from real hardware assessment (not generate_from_network
                // which may reject the node_id or produce insufficient stake).
                let state_proof = build_hardware_state_proof(&nid, coord);
                match bootstrap
                    .blockchain()
                    .register_asset_records(hw_assets, &state_proof)
                    .await
                {
                    Ok(block) => {
                        info!(
                            "Registered hardware + identity assets in block #{} (hash: {})",
                            block.index,
                            &block.hash[..16],
                        );
                        info!(
                            "Identity registered as blockchain asset (node_id: {})",
                            &falcon_identity.node_id[..16],
                        );
                        // Persist the hardware asset block
                        if let Err(e) = persistence.save_block(&block).await {
                            warn!("Failed to persist hardware asset block: {e}");
                        }
                    }
                    Err(e) => warn!("Failed to register hardware assets: {e}"),
                }
            }
            Err(e) => warn!("Hardware assessment failed: {e}"),
        }

        (bootstrap, persistence)
    };

    let persistence = std::sync::Arc::new(persistence);

    // Load persisted DNS records (user-registered names survive restarts)
    load_persisted_dns(bootstrap.dns(), &data_dir, &nid).await;

    // Extract DNS entries from blockchain (propagated from peers)
    extract_dns_from_blockchain(bootstrap.dns(), &bootstrap).await;

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
            eprintln!("'start' is deprecated. Use 'hypermesh connect public --foreground' instead.");
            eprintln!("Running as 'connect public --foreground'...");
            // Fall through to Connect logic by recursing into same path
            run_connect(
                &cli, coord, &nid, &data_dir, &bootstrap, persistence.clone(),
            ).await?;
        }
        Some(Commands::Connect { privacy, .. }) => {
            // Override global privacy with Connect subcommand's value
            cli.privacy = privacy;

            // Check if daemon is already running
            let client = ipc::IpcClient::new();
            if client.is_daemon_running().await {
                println!("Daemon already running.");
                return Ok(());
            }
            run_connect(
                &cli, coord, &nid, &data_dir, &bootstrap, persistence.clone(),
            ).await?;
        }
        Some(Commands::Disconnect) => {
            let client = ipc::IpcClient::new();
            if !client.is_daemon_running().await {
                eprintln!("No daemon running.");
                std::process::exit(1);
            }
            match client.call("shutdown", serde_json::json!({})).await {
                Ok(_) => println!("Daemon shutting down."),
                Err(e) => eprintln!("Failed to send shutdown: {e}"),
            }
        }
        Some(Commands::Status) => {
            let client = ipc::IpcClient::new();
            if client.is_daemon_running().await {
                match client.call_ok("status", serde_json::json!({})).await {
                    Ok(resp) => println!(
                        "{}",
                        serde_json::to_string_pretty(&resp).unwrap_or_default()
                    ),
                    Err(e) => eprintln!("Error: {e}"),
                }
            } else if cli.json {
                // JSON offline status
                let height = bootstrap.blockchain().get_height().await;
                let privacy = format!("{:?}", bootstrap.privacy_mode().await);
                let status = serde_json::json!({
                    "online": false,
                    "genesis": bootstrap.genesis_block().hash,
                    "chain_height": height,
                    "privacy_mode": privacy,
                    "self_sufficient": true,
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&status).unwrap_or_default()
                );
            } else {
                // Offline status: show bootstrap info
                info!("Node Status (offline):");
                info!("  Genesis: {}", bootstrap.genesis_block().hash);
                info!(
                    "  Blockchain height: {}",
                    bootstrap.blockchain().get_height().await
                );
                info!("  Privacy mode: {:?}", bootstrap.privacy_mode().await);
                info!("  Self-sufficient: yes");
                eprintln!("No daemon running. Start with: hypermesh connect public");
            }
        }
        Some(Commands::SetPrivacy { mode }) => {
            let client = ipc::IpcClient::new();
            if client.is_daemon_running().await {
                let mode_str = format!("{mode:?}");
                match client
                    .call_ok("set_privacy", serde_json::json!({"mode": mode_str}))
                    .await
                {
                    Ok(resp) => println!(
                        "{}",
                        serde_json::to_string_pretty(&resp).unwrap_or_default()
                    ),
                    Err(e) => eprintln!("Error: {e}"),
                }
            } else {
                let new_mode = mode.into();
                info!("Transitioning to {:?} mode...", new_mode);
                bootstrap.set_privacy_mode(new_mode).await?;
                info!("Privacy mode updated successfully");
            }
        }
        Some(Commands::Store { path }) => {
            let client = ipc::IpcClient::new();
            if client.is_daemon_running().await {
                let path_str = path.display().to_string();
                match client
                    .call_ok("store", serde_json::json!({"path": path_str}))
                    .await
                {
                    Ok(resp) => println!(
                        "{}",
                        serde_json::to_string_pretty(&resp).unwrap_or_default()
                    ),
                    Err(e) => {
                        warn!("IPC store failed ({e}), falling back to standalone");
                        run_store(path, None).await?;
                    }
                }
            } else {
                run_store(path, None).await?;
            }
        }
        Some(Commands::Fetch { asset_id, output }) => {
            let client = ipc::IpcClient::new();
            if client.is_daemon_running().await {
                match client
                    .call_ok(
                        "fetch",
                        serde_json::json!({
                            "asset_id": asset_id,
                            "output": output.as_ref().map(|p| p.display().to_string()),
                        }),
                    )
                    .await
                {
                    Ok(resp) => println!(
                        "{}",
                        serde_json::to_string_pretty(&resp).unwrap_or_default()
                    ),
                    Err(e) => {
                        warn!("IPC fetch failed ({e}), falling back to standalone");
                        run_fetch(asset_id, output).await?;
                    }
                }
            } else {
                run_fetch(asset_id, output).await?;
            }
        }
        Some(Commands::Dns { action }) => {
            let client = ipc::IpcClient::new();
            if client.is_daemon_running().await {
                let result = match &action {
                    DnsAction::Register { name, addr } => {
                        client
                            .call_ok(
                                "dns.register",
                                serde_json::json!({"name": name, "addr": addr}),
                            )
                            .await
                    }
                    DnsAction::Resolve { name } => {
                        client
                            .call_ok("dns.resolve", serde_json::json!({"name": name}))
                            .await
                    }
                    DnsAction::List => {
                        client
                            .call_ok("dns.list", serde_json::json!({}))
                            .await
                    }
                };
                match result {
                    Ok(resp) => println!(
                        "{}",
                        serde_json::to_string_pretty(&resp).unwrap_or_default()
                    ),
                    Err(e) => eprintln!("Error: {e}"),
                }
            } else {
                // Offline fallback: use local bootstrap DNS
                run_dns(action, &bootstrap, &data_dir, &nid).await?;
            }
        }
        Some(Commands::Domain { action }) => {
            let client = ipc::IpcClient::new();
            if client.is_daemon_running().await {
                let result = match &action {
                    DomainAction2::Register { name, privacy } => {
                        client
                            .call_ok(
                                "domain.register",
                                serde_json::json!({
                                    "name": name,
                                    "privacy": format!("{privacy:?}"),
                                }),
                            )
                            .await
                    }
                    DomainAction2::List => {
                        client
                            .call_ok("domain.list", serde_json::json!({}))
                            .await
                    }
                    _ => {
                        // Create/Nodes/Invite — fall through to offline handler
                        // (these don't need a running daemon)
                        run_domain(action, &bootstrap, &data_dir, &nid).await?;
                        return Ok(());
                    }
                };
                match result {
                    Ok(resp) => println!(
                        "{}",
                        serde_json::to_string_pretty(&resp).unwrap_or_default()
                    ),
                    Err(e) => {
                        warn!("IPC domain call failed ({e}), falling back to offline");
                        run_domain(action, &bootstrap, &data_dir, &nid).await?;
                    }
                }
            } else {
                run_domain(action, &bootstrap, &data_dir, &nid).await?;
            }
        }
        Some(Commands::Join { network, invite }) => {
            let client = ipc::IpcClient::new();
            if client.is_daemon_running().await {
                match client
                    .call_ok(
                        "domain.join",
                        serde_json::json!({
                            "domain": network,
                            "invite": invite,
                        }),
                    )
                    .await
                {
                    Ok(resp) => println!(
                        "{}",
                        serde_json::to_string_pretty(&resp).unwrap_or_default()
                    ),
                    Err(e) => eprintln!("Error: {e}"),
                }
            } else {
                run_join(&network, invite.as_deref(), &nid, &data_dir).await?;
            }
        }
        Some(Commands::Config { .. }) => {
            // Config commands are handled early in main() before bootstrap.
            // This arm is unreachable but satisfies exhaustiveness.
            unreachable!("config commands handled before bootstrap");
        }
        Some(Commands::Dashboard { action }) => {
            match action {
                DashboardAction::Deploy { path } => {
                    let manifest_path = path.join("dashboard.toml");
                    if !manifest_path.exists() {
                        eprintln!("No dashboard.toml found in {}", path.display());
                        std::process::exit(1);
                    }
                    let toml_str = std::fs::read_to_string(&manifest_path)
                        .with_context(|| format!("Failed to read {}", manifest_path.display()))?;
                    let manifest = blockmatrix::dashboard::parse_manifest(&toml_str)
                        .map_err(|e| anyhow::anyhow!(e))?;
                    if let Err(errors) =
                        blockmatrix::dashboard::validate_manifest(&manifest, &path)
                    {
                        for e in &errors {
                            eprintln!("Validation error: {e}");
                        }
                        std::process::exit(1);
                    }
                    info!(
                        "Dashboard '{}' v{} validated",
                        manifest.dashboard.name, manifest.dashboard.version
                    );
                    info!("Domain: {}", manifest.dashboard.domain);

                    // Collect all files from the access scope directories
                    let files = blockmatrix::dashboard::deploy::collect_dashboard_files(
                        &path,
                        &manifest.access,
                    )
                    .with_context(|| "failed to collect dashboard files")?;

                    if files.is_empty() {
                        eprintln!("No files found in dashboard scope directories");
                        std::process::exit(1);
                    }

                    // Bundle files
                    let bundle = blockmatrix::dashboard::deploy::bundle_files(&files);

                    // Check if daemon is running -- prefer IPC for deploy
                    let client = ipc::IpcClient::new();
                    if client.is_daemon_running().await {
                        // Build files payload as base64 for IPC
                        use base64::Engine as _;
                        let files_json: serde_json::Value = files
                            .iter()
                            .map(|(k, v)| {
                                (k.clone(), serde_json::Value::String(
                                    base64::engine::general_purpose::STANDARD.encode(v),
                                ))
                            })
                            .collect::<serde_json::Map<String, serde_json::Value>>()
                            .into();

                        match client
                            .call_ok(
                                "dashboard.deploy",
                                serde_json::json!({
                                    "name": manifest.dashboard.name,
                                    "manifest_toml": toml_str,
                                    "files": files_json,
                                }),
                            )
                            .await
                        {
                            Ok(resp) => println!(
                                "{}",
                                serde_json::to_string_pretty(&resp).unwrap_or_default()
                            ),
                            Err(e) => {
                                eprintln!("Deploy via daemon failed: {e}");
                                std::process::exit(1);
                            }
                        }
                    } else {
                        // Direct deploy (no daemon running)
                        let asset_data = AssetData {
                            config: format!(
                                "DASHBOARD:DEPLOY:{}",
                                manifest.dashboard.name
                            )
                            .into_bytes(),
                            definition: bundle.clone(),
                            metadata: toml_str.as_bytes().to_vec(),
                        };
                        let registration = AssetRegistration::from_asset_data(
                            &asset_data,
                            NetworkScope::Global,
                            AssetCategory::BaseSystem(BaseSystemType::Dashboard),
                        );
                        let content_hash = registration.content_hash;
                        let state_proof = StateProof::generate_from_network(&nid)
                            .await
                            .context("PoS proof generation failed for dashboard deploy")?;
                        let block = bootstrap
                            .blockchain()
                            .register_asset_record(registration, &state_proof)
                            .await
                            .map_err(|e| anyhow::anyhow!("blockchain write failed: {e}"))?;

                        // Store bundle in asset store (keyed by blockchain content hash)
                        blockmatrix::dashboard::deploy::store_dashboard_bundle(
                            &data_dir, &content_hash, &toml_str, &bundle,
                        )
                        .with_context(|| "failed to store dashboard bundle")?;

                        println!();
                        println!("  Dashboard Deployed");
                        println!("  ------------------");
                        println!("  name:    {}", manifest.dashboard.name);
                        println!("  version: {}", manifest.dashboard.version);
                        println!("  domain:  {}", manifest.dashboard.domain);
                        println!("  hash:    {}", hex::encode(content_hash));
                        println!("  block:   #{}", block.index);
                        println!("  files:   {}", files.len());
                        println!();
                    }
                }
                DashboardAction::List => {
                    let client = ipc::IpcClient::new();
                    if client.is_daemon_running().await {
                        match client.call_ok("dashboard.list", serde_json::json!({})).await {
                            Ok(resp) => println!(
                                "{}",
                                serde_json::to_string_pretty(&resp).unwrap_or_default()
                            ),
                            Err(e) => eprintln!("Error: {e}"),
                        }
                    } else {
                        println!(
                            "No dashboards registered yet. \
                             Deploy with: hypermesh dashboard deploy <path>"
                        );
                    }
                }
                DashboardAction::Info { name } => {
                    let client = ipc::IpcClient::new();
                    if client.is_daemon_running().await {
                        match client
                            .call_ok(
                                "dashboard.info",
                                serde_json::json!({"name": name}),
                            )
                            .await
                        {
                            Ok(resp) => println!(
                                "{}",
                                serde_json::to_string_pretty(&resp).unwrap_or_default()
                            ),
                            Err(e) => eprintln!("Error: {e}"),
                        }
                    } else {
                        println!("Dashboard '{}': not found", name);
                    }
                }
                DashboardAction::Init { name } => {
                    let project_name =
                        name.unwrap_or_else(|| "my-dashboard".to_string());
                    info!("Scaffolding dashboard project: {}", project_name);

                    let dir = std::path::PathBuf::from(&project_name);
                    std::fs::create_dir_all(dir.join("dist/public"))?;
                    std::fs::create_dir_all(dir.join("dist/private"))?;

                    let manifest_toml =
                        blockmatrix::dashboard::scaffold_manifest(&project_name);
                    std::fs::write(dir.join("dashboard.toml"), &manifest_toml)?;

                    std::fs::write(
                        dir.join("dist/public/index.html"),
                        blockmatrix::dashboard::scaffold_html(&project_name, "public"),
                    )?;
                    std::fs::write(
                        dir.join("dist/private/index.html"),
                        blockmatrix::dashboard::scaffold_html(&project_name, "private"),
                    )?;

                    println!("Created dashboard project at ./{project_name}/");
                    println!("  dashboard.toml");
                    println!("  dist/public/index.html");
                    println!("  dist/private/index.html");
                    println!(
                        "\nDeploy with: hypermesh dashboard deploy ./{project_name}/"
                    );
                }
            }
        }
        Some(Commands::Caesar { action }) => {
            let json = cli.json;
            match action {
                CaesarAction::Wallet => {
                    service_ipc_call("caesar.wallet", serde_json::json!({}), json).await?;
                }
                CaesarAction::Balance => {
                    service_ipc_call("caesar.balance", serde_json::json!({}), json).await?;
                }
                CaesarAction::Transactions { limit } => {
                    service_ipc_call(
                        "caesar.transactions",
                        serde_json::json!({"limit": limit}),
                        json,
                    ).await?;
                }
                CaesarAction::Rewards => {
                    service_ipc_call("caesar.rewards", serde_json::json!({}), json).await?;
                }
                CaesarAction::Route { destination, amount } => {
                    service_ipc_call(
                        "caesar.route_packet",
                        serde_json::json!({
                            "destination": destination,
                            "amount_grams": amount,
                        }),
                        json,
                    ).await?;
                }
                CaesarAction::Governor => {
                    service_ipc_call("caesar.governor_params", serde_json::json!({}), json).await?;
                }
            }
        }
        Some(Commands::Trustchain { action }) => {
            let json = cli.json;
            match action {
                TrustchainAction::Certs => {
                    service_ipc_call("trustchain.certificates", serde_json::json!({}), json).await?;
                }
                TrustchainAction::Issue { subject, scope } => {
                    service_ipc_call(
                        "trustchain.issue",
                        serde_json::json!({
                            "subject": subject,
                            "scope": scope,
                        }),
                        json,
                    ).await?;
                }
                TrustchainAction::Validate { cert_path } => {
                    service_ipc_call(
                        "trustchain.validate",
                        serde_json::json!({"cert_pem": cert_path}),
                        json,
                    ).await?;
                }
                TrustchainAction::Revoke { cert_id } => {
                    service_ipc_call(
                        "trustchain.revoke",
                        serde_json::json!({"cert_id": cert_id}),
                        json,
                    ).await?;
                }
                TrustchainAction::Zones => {
                    service_ipc_call("trustchain.dns_zones", serde_json::json!({}), json).await?;
                }
            }
        }
        Some(Commands::Engauge { action }) => {
            let json = cli.json;
            match action {
                EngaugeAction::Capacity => {
                    service_ipc_call("engauge.capacity", serde_json::json!({}), json).await?;
                }
                EngaugeAction::Traffic => {
                    service_ipc_call("engauge.traffic", serde_json::json!({}), json).await?;
                }
                EngaugeAction::Marketplace => {
                    service_ipc_call("engauge.marketplace", serde_json::json!({}), json).await?;
                }
                EngaugeAction::Metrics => {
                    service_ipc_call("engauge.node_metrics", serde_json::json!({}), json).await?;
                }
                EngaugeAction::Leases => {
                    service_ipc_call("engauge.leases", serde_json::json!({}), json).await?;
                }
            }
        }
        Some(Commands::Catalog { action }) => {
            let json = cli.json;
            match action {
                CatalogAction::Browse { query, page } => {
                    service_ipc_call(
                        "catalog.browse",
                        serde_json::json!({"query": query, "page": page}),
                        json,
                    ).await?;
                }
                CatalogAction::Search { query } => {
                    service_ipc_call(
                        "catalog.search",
                        serde_json::json!({"query": query}),
                        json,
                    ).await?;
                }
                CatalogAction::Info { name } => {
                    service_ipc_call(
                        "catalog.package_info",
                        serde_json::json!({"name": name}),
                        json,
                    ).await?;
                }
                CatalogAction::Stats => {
                    service_ipc_call("catalog.registry_stats", serde_json::json!({}), json).await?;
                }
            }
        }
        Some(Commands::Destroy { .. }) => {
            // Handled early (before bootstrap), should not reach here
            unreachable!("destroy handled before bootstrap");
        }
        None => {
            // No command - just show bootstrap info
            info!("Node initialized successfully. Use 'connect' to run or 'status' to check.");
        }
    }

    Ok(())
}
