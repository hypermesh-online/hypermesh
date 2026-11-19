// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Narrow transport and discovery setup, preserving bring-up await order.

use anyhow::Result;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

use blockmatrix::bootstrap::PrivacyMode;
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::NetworkManager;
use stoq::transport::NetworkType;

use crate::cli::Cli;

pub(super) fn stoq_listener_config(cli: &Cli) -> stoq::TransportConfig {
    stoq::TransportConfig {
        port: cli.stoq_port,
        bind_address: cli.stoq_bind_address,
        enable_kernel_acceleration: node_kernel_acceleration_allowed(cli),
        ..stoq::TransportConfig::default()
    }
}

pub(super) fn node_kernel_acceleration_allowed(cli: &Cli) -> bool {
    !cli.stoq_bind_address.is_loopback()
}

pub(super) fn uses_physical_interface(config: &stoq::TransportConfig) -> bool {
    !config.bind_address.is_loopback()
}

pub(super) async fn create_transport(
    cli: &Cli,
    privacy_mode: PrivacyMode,
) -> Result<Arc<stoq::StoqTransport>> {
    info!("Initializing STOQ transport on port {}", cli.stoq_port);
    let mut config = stoq_listener_config(cli);
    // Physical selection is irrelevant to an explicit local-only listener.
    if uses_physical_interface(&config) {
        use base::Substrate as _;
        let substrate = base::DefaultSubstrate::new();
        match substrate.active_interface().await {
            Ok(iface) => {
                info!("Substrate selected outbound interface: {}", iface.name);
                config.interface = Some(iface.name);
            }
            Err(e) => warn!(
                "Substrate interface selection failed ({e}); STOQ will auto-detect the interface"
            ),
        }
    }
    let network_type = if privacy_mode == PrivacyMode::ANONYMOUS {
        config.enable_falcon_crypto = false;
        info!("Anonymous mode: using ephemeral certificates, no CA dependency");
        NetworkType::Anonymous
    } else if privacy_mode == PrivacyMode::PUBLIC {
        info!("Public mode: self-issuing certificate via local TrustChain");
        NetworkType::P2P
    } else {
        info!("Private mode: self-issuing certificate via local TrustChain");
        NetworkType::P2P
    };
    Ok(Arc::new(
        stoq::StoqTransport::new_for_network(config, network_type).await?,
    ))
}

pub(super) fn bootstrap_nodes(cli: &Cli) -> Vec<SocketAddr> {
    let nodes: Vec<SocketAddr> = cli
        .bootstrap
        .iter()
        .filter_map(|addr| addr.parse().ok())
        .collect();
    if !nodes.is_empty() {
        info!("Bootstrap nodes: {:?}", nodes);
    }
    nodes
}

pub(super) async fn discover_network(
    cli: &Cli,
    coord: MatrixCoordinate,
    data_dir: &Path,
    privacy_mode: PrivacyMode,
    transport: Arc<stoq::StoqTransport>,
    nodes: Vec<SocketAddr>,
) -> Result<NetworkManager> {
    let identity_dir = blockmatrix::bootstrap::identity_dir(data_dir);
    let falcon_identity = blockmatrix::identity::FalconIdentity::load_or_create(&identity_dir)?;
    info!(
        "Node identity: {}... (FALCON-1024)",
        &falcon_identity.node_id[..16]
    );
    let signer: Arc<dyn hypermesh_lib::NodeSigner> = Arc::new(falcon_identity);
    let proof_provider: Arc<dyn hypermesh_lib::StateProofProvider> =
        Arc::new(blockmatrix::proof_of_state::BlockMatrixProofProvider::new(
            signer.node_id().to_string(),
            signer.clone(),
        ));
    let manager = NetworkManager::new(
        coord,
        transport,
        privacy_mode,
        nodes,
        signer,
        proof_provider,
        cli.network_id.clone(),
    )
    .await?;
    manager.start_discovery().await?;
    Ok(manager)
}

type NodeMap = Arc<tokio::sync::RwLock<std::collections::HashMap<String, (String, SocketAddr)>>>;
type Propagator = Arc<tokio::sync::Mutex<blockmatrix::blockchain::propagation::BlockPropagator>>;
type Sync = Arc<tokio::sync::Mutex<blockmatrix::blockchain::sync_manager::SyncManager>>;
type Reflectors = Arc<tokio::sync::Mutex<blockmatrix::network::reflector_pool::ReflectorPool>>;

pub(super) struct BlockSyncSetup {
    pub(super) node_map: NodeMap,
    pub(super) block_transport:
        Arc<blockmatrix::blockchain::stoq_transport::StoqBlockTransportAdapter>,
    pub(super) block_propagator: Propagator,
    pub(super) sync_manager: Sync,
    pub(super) reflector_pool: Reflectors,
}

pub(super) async fn initialize_block_sync(
    coord: MatrixCoordinate,
    bootstrap: &blockmatrix::bootstrap::NodeBootstrap,
    network_id: &str,
    privacy_mode: PrivacyMode,
    should_join: bool,
) -> BlockSyncSetup {
    use blockmatrix::blockchain::propagation::{BlockPropagator, PropagationStrategy};
    use blockmatrix::blockchain::stoq_transport::StoqBlockTransportAdapter;
    use blockmatrix::blockchain::sync_manager::{SyncConfig, SyncManager};
    use blockmatrix::network::reflector_pool::{ReflectorConfig, ReflectorPool};

    let node_map = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let block_transport = Arc::new(StoqBlockTransportAdapter::new());
    let block_propagator = Arc::new(tokio::sync::Mutex::new(BlockPropagator::with_transport(
        coord,
        PropagationStrategy::NearestN(6),
        block_transport.clone(),
    )));
    let genesis_hash = bootstrap.genesis_block().hash.clone();
    let sync_manager = Arc::new(tokio::sync::Mutex::new(SyncManager::new(
        genesis_hash.clone(),
        SyncConfig::default(),
    )));
    let reflector_pool = Arc::new(tokio::sync::Mutex::new(ReflectorPool::new(
        ReflectorConfig::default(),
    )));
    if should_join {
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        match sync_manager
            .lock()
            .await
            .join_network(network_id.to_string(), privacy_mode, now_secs)
        {
            Ok(()) => info!("Joined Network scope chain: {}", network_id),
            Err(e) => warn!("Failed to join network scope: {}", e),
        }
    }
    info!("Block sync infrastructure initialized (propagation=NearestN(6))");
    BlockSyncSetup {
        node_map,
        block_transport,
        block_propagator,
        sync_manager,
        reflector_pool,
    }
}

pub(super) struct ShardBase {
    pub(super) store: Arc<blockmatrix::network::shard_store::ShardStore>,
    pub(super) transport: Arc<blockmatrix::network::shard_transport::StoqShardTransport>,
}

impl ShardBase {
    pub(super) async fn new(
        data_dir: &Path,
        nid: &str,
        transport: &Arc<stoq::StoqTransport>,
    ) -> Self {
        use blockmatrix::network::shard_store::ShardStore;
        use blockmatrix::network::shard_transport::StoqShardTransport;
        let store = Arc::new(ShardStore::new_with_dir(data_dir.join(nid).join("shards")));
        let shard_transport = Arc::new(StoqShardTransport::new(transport.clone()));
        info!(
            "Shard store and transport initialized (store={} shards)",
            store.count().await
        );
        Self {
            store,
            transport: shard_transport,
        }
    }

    pub(super) fn with_provider(self, nid: &str, network_id: &str) -> ShardServices {
        use blockmatrix::network::consumer_provider::ConsumerProviderManager;
        use blockmatrix::network::swarm_provider::ShardLocationIndex;
        let location_index = Arc::new(ShardLocationIndex::new());
        let consumer_provider = Arc::new(ConsumerProviderManager::new(
            self.store.clone(),
            location_index.clone(),
            nid.to_string(),
            hypermesh_lib::NetworkId::from_wire_str(network_id),
        ));
        ShardServices {
            store: self.store,
            transport: self.transport,
            location_index,
            consumer_provider,
        }
    }
}

pub(super) struct ShardServices {
    pub(super) store: Arc<blockmatrix::network::shard_store::ShardStore>,
    pub(super) transport: Arc<blockmatrix::network::shard_transport::StoqShardTransport>,
    pub(super) location_index: Arc<blockmatrix::network::swarm_provider::ShardLocationIndex>,
    pub(super) consumer_provider:
        Arc<blockmatrix::network::consumer_provider::ConsumerProviderManager>,
}

pub(super) struct PeerCore {
    pub(super) gossip: Arc<blockmatrix::network::gossip::GossipProtocol>,
    pub(super) connected_coords: blockmatrix::network::connected_peer_routes::ConnectedPeerRoutes,
    pub(super) demand_tracker: Arc<blockmatrix::network::SwarmDemandTracker>,
}

impl PeerCore {
    pub(super) fn new(
        cli: &Cli,
        coord: MatrixCoordinate,
        nid: &str,
        privacy: PrivacyMode,
        connected_coords: blockmatrix::network::connected_peer_routes::ConnectedPeerRoutes,
    ) -> Self {
        let gossip = Arc::new(blockmatrix::network::gossip::GossipProtocol::new(
            nid.to_string(),
            coord,
            cli.stoq_port,
            format!("{:?}", privacy),
        ));
        let demand_tracker = Arc::new(blockmatrix::network::SwarmDemandTracker::new());
        Self {
            gossip,
            connected_coords,
            demand_tracker,
        }
    }
}

pub(super) struct Acceleration {
    #[cfg(feature = "intelligence")]
    pub(super) analytics: Arc<std::sync::Mutex<ngauge::SwarmAnalytics>>,
    #[cfg(feature = "intelligence")]
    pub(super) ingestion: Arc<std::sync::Mutex<ngauge::MetricsIngestionPipeline>>,
    pub(super) ebpf: Option<Arc<hypermesh_ebpf::HyperMeshEbpf>>,
}

impl Acceleration {
    pub(super) fn new(enable_kernel_acceleration: bool) -> Self {
        #[cfg(feature = "intelligence")]
        let analytics = Arc::new(std::sync::Mutex::new(ngauge::SwarmAnalytics::new()));
        #[cfg(feature = "intelligence")]
        let ingestion = Arc::new(std::sync::Mutex::new(
            ngauge::MetricsIngestionPipeline::with_defaults(),
        ));
        let ebpf = if enable_kernel_acceleration {
            Self::try_initialize_kernel()
        } else {
            None
        };
        Self {
            #[cfg(feature = "intelligence")]
            analytics,
            #[cfg(feature = "intelligence")]
            ingestion,
            ebpf,
        }
    }

    fn try_initialize_kernel() -> Option<Arc<hypermesh_ebpf::HyperMeshEbpf>> {
        match hypermesh_ebpf::HyperMeshEbpf::new(hypermesh_ebpf::EbpfConfig::default()) {
            Ok(ebpf) => Some(Arc::new(ebpf)),
            Err(err) => {
                warn!("eBPF orchestrator unavailable, kernel gate + feedback disabled: {err}");
                None
            }
        }
    }
}
