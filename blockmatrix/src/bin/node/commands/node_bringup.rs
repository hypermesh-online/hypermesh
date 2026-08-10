// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Node network bring-up: initialize STOQ transport + identity, wire the
//! block-sync / gossip / shard infrastructure and the peer message loops,
//! then hand the intelligence-driven replication/placement loops off to
//! [`crate::network::replication_service`]. Extracted verbatim from the
//! `connect` command (P8) so the connect handler stays thin wiring.

use anyhow::Result;
use tracing::{debug, info, warn};

use blockmatrix::blockchain::propagation::{BlockPropagator, PropagationStrategy};
use blockmatrix::blockchain::stoq_transport::StoqBlockTransportAdapter;
use blockmatrix::blockchain::sync_manager::{SyncConfig, SyncManager};
use blockmatrix::bootstrap::{NodeBootstrap, PrivacyMode};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::reflector_pool::{ReflectorConfig, ReflectorPool};
use blockmatrix::network::shard_store::ShardStore;
use blockmatrix::network::shard_transport::StoqShardTransport;
use blockmatrix::network::NetworkManager;
use stoq::transport::NetworkType;

use crate::cli::Cli;

use super::background::{spawn_block_sync_loop, spawn_gossip_loop, spawn_peer_sync_loop};
use super::services::{count_dns_assets_in_block, propagate_block, register_reflector_peers};


/// Result bundle returned from `start_network`.
pub(crate) struct NetworkStartResult {
    pub(crate) network: std::sync::Arc<NetworkManager>,
    pub(crate) shard_store: std::sync::Arc<ShardStore>,
    pub(crate) shard_transport: std::sync::Arc<StoqShardTransport>,
    /// Shared shard location index — same instance used by PeerContext and
    /// the IPC daemon, so TAG_SHARD_ANNOUNCE updates and local provider
    /// registrations converge.
    pub(crate) shard_location_index: std::sync::Arc<
        blockmatrix::network::swarm_provider::ShardLocationIndex,
    >,
    /// Consumer-becomes-provider manager (R12) wired to the shared
    /// `shard_location_index`. Used by IPC fetch handlers to register the
    /// local node as a provider after a network fetch and to broadcast
    /// TAG_SHARD_ANNOUNCE to peers.
    pub(crate) consumer_provider_manager: std::sync::Arc<
        blockmatrix::network::consumer_provider::ConsumerProviderManager,
    >,
    #[cfg(feature = "intelligence")]
    pub(crate) swarm_demand_tracker: std::sync::Arc<blockmatrix::network::SwarmDemandTracker>,
    #[cfg(feature = "intelligence")]
    pub(crate) ngauge_analytics: std::sync::Arc<std::sync::Mutex<ngauge::SwarmAnalytics>>,
}

/// Initialize STOQ transport, network manager, and all background loops.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn start_network(
    cli: &Cli,
    coord: MatrixCoordinate,
    nid: &str,
    data_dir: &std::path::Path,
    bootstrap: &NodeBootstrap,
    privacy_mode: PrivacyMode,
    has_bootstrap_peers: bool,
    share_inbox_store: std::sync::Arc<blockmatrix::sharing::inbox::InboxStore>,
) -> Result<NetworkStartResult> {
    info!("Initializing STOQ transport on port {}", cli.stoq_port);

    let mut stoq_config = stoq::TransportConfig {
        port: cli.stoq_port,
        bind_address: std::net::Ipv6Addr::UNSPECIFIED,
        ..stoq::TransportConfig::default()
    };

    // Substrate selects the real, carrier-aware outbound interface (R16) and
    // injects it into STOQ (STOQ does not depend on base — injection only).
    // Falls back to STOQ's own auto-detection if selection fails.
    use base::Substrate as _;
    let substrate = base::DefaultSubstrate::new();
    match substrate.active_interface().await {
        Ok(iface) => {
            info!("Substrate selected outbound interface: {}", iface.name);
            stoq_config.interface = Some(iface.name);
        }
        Err(e) => {
            warn!(
                "Substrate interface selection failed ({e}); \
                 STOQ will auto-detect the interface"
            );
        }
    }

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

    let transport = std::sync::Arc::new(
        stoq::StoqTransport::new_for_network(stoq_config, network_type).await?,
    );

    let bootstrap_nodes: Vec<std::net::SocketAddr> = cli
        .bootstrap
        .iter()
        .filter_map(|addr| addr.parse().ok())
        .collect();

    if !bootstrap_nodes.is_empty() {
        info!("Bootstrap nodes: {:?}", bootstrap_nodes);
    }

    let shard_store = std::sync::Arc::new(ShardStore::new_with_dir(
        data_dir.join(nid).join("shards"),
    ));
    let shard_transport =
        std::sync::Arc::new(StoqShardTransport::new(transport.clone()));
    info!(
        "Shard store and transport initialized (store={} shards)",
        shard_store.count().await
    );

    // D5: identity lives at the coordinate-independent `data_dir/identity`.
    let identity_dir = blockmatrix::bootstrap::identity_dir(data_dir);
    let falcon_identity =
        blockmatrix::identity::FalconIdentity::load_or_create(&identity_dir)?;
    info!(
        "Node identity: {}... (FALCON-1024)",
        &falcon_identity.node_id[..16]
    );

    let signer: std::sync::Arc<dyn hypermesh_lib::NodeSigner> =
        std::sync::Arc::new(falcon_identity);
    let proof_provider: std::sync::Arc<dyn hypermesh_lib::StateProofProvider> =
        std::sync::Arc::new(
            blockmatrix::proof_of_state::BlockMatrixProofProvider::new(
                signer.node_id().to_string(),
                signer.clone(),
            ),
        );
    let network_manager = NetworkManager::new(
        coord,
        transport.clone(),
        privacy_mode,
        bootstrap_nodes,
        signer,
        proof_provider,
        cli.network_id.clone(),
    )
    .await?;

    network_manager.start_discovery().await?;

    // --- Block Sync Infrastructure ---
    let node_map: std::sync::Arc<
        tokio::sync::RwLock<
            std::collections::HashMap<String, (String, std::net::SocketAddr)>,
        >,
    > = std::sync::Arc::new(tokio::sync::RwLock::new(
        std::collections::HashMap::new(),
    ));

    let block_transport = std::sync::Arc::new(StoqBlockTransportAdapter::new());

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

    let network_id = cli.network_id.clone();
    if has_bootstrap_peers || cli.reflector {
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

    let gossip_proto = std::sync::Arc::new(
        blockmatrix::network::gossip::GossipProtocol::new(
            nid.to_string(),
            coord,
            cli.stoq_port,
            format!("{:?}", privacy_mode),
        ),
    );

    let connected_peer_coords = std::sync::Arc::new(
        tokio::sync::RwLock::new(Vec::<MatrixCoordinate>::new()),
    );
    let swarm_demand_tracker = std::sync::Arc::new(
        blockmatrix::network::SwarmDemandTracker::new(),
    );

    // R12 consumer-becomes-provider: shared shard location index. The same
    // Arc is passed to PeerContext (so TAG_SHARD_ANNOUNCE handlers update it)
    // and returned to the daemon (so IPC fetch handlers can register the
    // local node and discover providers for replication requests).
    let shard_location_index = std::sync::Arc::new(
        blockmatrix::network::swarm_provider::ShardLocationIndex::new(),
    );
    let consumer_provider_manager = std::sync::Arc::new(
        blockmatrix::network::consumer_provider::ConsumerProviderManager::new(
            shard_store.clone(),
            shard_location_index.clone(),
            nid.to_string(),
            hypermesh_lib::NetworkId::from_wire_str(&network_id),
        ),
    );

    // --- ngauge intelligence wiring (H2-H5) ---
    #[cfg(feature = "intelligence")]
    let ngauge_analytics = std::sync::Arc::new(
        std::sync::Mutex::new(ngauge::SwarmAnalytics::new()),
    );
    #[cfg(feature = "intelligence")]
    let ngauge_ingestion = std::sync::Arc::new(
        std::sync::Mutex::new(ngauge::MetricsIngestionPipeline::with_defaults()),
    );

    // P5: construct the shared eBPF orchestrator once. It is used both by
    // the PeerContext (to mirror peer authentication into the kernel
    // fast-path maps) AND by the ngauge routing-intelligence feedback loop
    // below. When the orchestrator cannot be created the node runs in the
    // userspace-only tier (graceful degradation) — every kernel-map write
    // downstream is a no-op unless an XDP program is actually attached.
    let ebpf_orchestrator: Option<std::sync::Arc<hypermesh_ebpf::HyperMeshEbpf>> =
        match hypermesh_ebpf::HyperMeshEbpf::new(hypermesh_ebpf::EbpfConfig::default()) {
            Ok(e) => Some(std::sync::Arc::new(e)),
            Err(err) => {
                warn!("eBPF orchestrator unavailable, kernel gate + feedback disabled: {err}");
                None
            }
        };

    let peer_ctx = std::sync::Arc::new(blockmatrix::network::PeerContext {
        blockchain: bootstrap.blockchain().clone(),
        shard_store: shard_store.clone(),
        sync_manager: sync_manager.clone(),
        reflector_pool: reflector_pool.clone(),
        block_propagator: block_propagator.clone(),
        our_coordinate: coord,
        node_id: nid.to_string(),
        network_id: network_id.clone(),
        blockchain_scope: if has_bootstrap_peers || cli.reflector {
            hypermesh_lib::BlockchainScope::Network
        } else {
            hypermesh_lib::BlockchainScope::Device
        },
        spatial_bucket_assigner: None,
        connected_peer_coords: connected_peer_coords.clone(),
        dns_resolver: Some(bootstrap.dns().clone()),
        authenticated_peers: network_manager.authenticated_peers(),
        gossip_protocol: Some(gossip_proto.clone()),
        swarm_demand_tracker: swarm_demand_tracker.clone(),
        shard_location_index: Some(shard_location_index.clone()),
        inbox_store: Some(share_inbox_store.clone()),
        message_store: Some(std::sync::Arc::new(
            blockmatrix::messaging::store::MessageStore::new(
                Some(data_dir.join(nid).join("messages")),
            ),
        )),
        rotation_chains: Some(std::sync::Arc::new(
            tokio::sync::RwLock::new(std::collections::HashMap::new()),
        )),
        #[cfg(feature = "intelligence")]
        ngauge_analytics: Some(ngauge_analytics.clone()),
        #[cfg(feature = "intelligence")]
        ngauge_ingestion: Some(ngauge_ingestion.clone()),
        // Phase G.2 — alpha-default inert. Daemon opt-in plumbs a real
        // TransferCoordinator here once a STOQ-backed TransferTransport
        // is configured (Phase G.2 deliverable on the daemon side).
        transfer_coordinator: None,
        // P5: share the eBPF orchestrator so peer authentication mirrors
        // into the kernel maps (no-op unless an XDP program is attached).
        ebpf: ebpf_orchestrator.clone(),
    });

    let network_clone = std::sync::Arc::new(network_manager);
    network_clone
        .start_peer_message_loops(peer_ctx.clone())
        .await;

    register_reflector_peers(
        &network_clone,
        &reflector_pool,
        &network_id,
        has_bootstrap_peers,
        cli.reflector,
        privacy_mode,
    )
    .await;

    let addr_map = network_clone.get_node_address_map().await;
    *node_map.write().await = addr_map;

    // Inject authenticated connections into the block transport adapter so
    // that block propagation reuses the handshake connection instead of
    // opening new (unauthenticated) STOQ connections.
    for node in network_clone.get_connected_nodes().await {
        if let Some(ref conn) = node.connection {
            block_transport.inject_connection(&node.coordinate, conn.clone()).await;
        }
    }

    // Accept connections in background
    let network_accept = network_clone.clone();
    let ctx_accept = peer_ctx.clone();
    tokio::spawn(async move {
        if let Err(e) = network_accept.accept_connections(Some(ctx_accept)).await {
            warn!("Connection acceptor error: {}", e);
        }
    });

    if cli.reflector {
        info!("Reflector mode: broadcasting matrix position");
        network_clone.broadcast_matrix_position().await?;
    }

    info!(
        "Network initialized, accepting connections on port {}",
        cli.stoq_port
    );

    spawn_peer_sync_loop(network_clone.clone(), shard_transport.clone());

    spawn_block_sync_loop(
        sync_manager.clone(),
        reflector_pool.clone(),
        bootstrap.blockchain().clone(),
        network_clone.clone(),
        node_map.clone(),
        transport.clone(),
        block_propagator.clone(),
        connected_peer_coords.clone(),
        coord,
        nid.to_string(),
        network_id.clone(),
        cli.reflector,
        shard_store.clone(),
    );

    spawn_gossip_loop(
        gossip_proto.clone(),
        network_clone.clone(),
        shard_store.clone(),
        coord,
    )
    .await;

    // P8: hand the intelligence-driven replication / placement loops (H3
    // demand feed 10s, H4/H5 propagation-weight + replication-signal 15s, E.2
    // replication-poll 30s) off to the scoped service. Same handles (the SAME
    // Arcs the transport bring-up holds), same intervals, same DEFAULT_NETWORK
    // defaults, same flat per-network replication — a pure move (P8), not a
    // behaviour change.
    #[cfg(feature = "intelligence")]
    blockmatrix::network::replication_service::ReplicationService {
        coord,
        node_id: nid.to_string(),
        privacy_mode,
        swarm_demand_tracker: swarm_demand_tracker.clone(),
        ngauge_analytics: ngauge_analytics.clone(),
        block_propagator: block_propagator.clone(),
        network: network_clone.clone(),
        sync_manager: sync_manager.clone(),
        shard_location_index: shard_location_index.clone(),
        shard_transport: shard_transport.clone(),
        ebpf: ebpf_orchestrator.clone(),
    }
    .spawn()
    .await?;

    // Propagate bootstrap blocks
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

    // Keep infrastructure alive
    let _block_propagator = block_propagator.clone();
    let _sync_manager = sync_manager.clone();
    let _reflector_pool = reflector_pool.clone();

    Ok(NetworkStartResult {
        network: network_clone,
        shard_store,
        shard_transport,
        shard_location_index,
        consumer_provider_manager,
        #[cfg(feature = "intelligence")]
        swarm_demand_tracker,
        #[cfg(feature = "intelligence")]
        ngauge_analytics,
    })
}
