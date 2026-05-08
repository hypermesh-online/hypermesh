// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Connect/disconnect subcommand handler -- starts the node daemon.

use anyhow::{Context, Result};
use tracing::{debug, info, warn};

use blockmatrix::assets::core::{AssetCategory, AssetData, AssetRegistration, BaseSystemType, NetworkScope};
use blockmatrix::blockchain::propagation::{BlockPropagator, PropagationStrategy};
use blockmatrix::blockchain::stoq_transport::StoqBlockTransportAdapter;
use blockmatrix::blockchain::sync_manager::{SyncConfig, SyncManager};
use blockmatrix::bootstrap::{NodeBootstrap, PrivacyMode};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::reflector_pool::{ReflectorConfig, ReflectorPool};
use blockmatrix::network::shard_store::ShardStore;
use blockmatrix::network::shard_transport::StoqShardTransport;
use blockmatrix::network::NetworkManager;
use blockmatrix::persistence::PersistenceManager;
use blockmatrix::ipc;
use stoq::transport::NetworkType;

use crate::cli::Cli;

use super::background::{spawn_block_sync_loop, spawn_gossip_loop, spawn_peer_sync_loop};
use super::services::{
    count_dns_assets_in_block, propagate_block, register_default_dashboard,
    register_reflector_peers,
};

/// Call a service method via IPC and print the result.
pub async fn service_ipc_call(
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

/// Run the connect/start flow: initialize STOQ, network, sync loops, IPC server,
/// then wait for Ctrl+C or IPC shutdown.
pub async fn run_connect(
    cli: &Cli,
    coord: MatrixCoordinate,
    nid: &str,
    data_dir: &std::path::Path,
    bootstrap: &NodeBootstrap,
    persistence: std::sync::Arc<PersistenceManager>,
) -> Result<()> {
    info!("Starting node services...");

    let target_mode = cli.privacy.into();
    if bootstrap.privacy_mode().await != target_mode {
        bootstrap.set_privacy_mode(target_mode).await?;
    }

    let mut network_ref: Option<std::sync::Arc<NetworkManager>> = None;
    let mut shard_store_ref: Option<std::sync::Arc<ShardStore>> = None;
    let mut shard_transport_ref: Option<std::sync::Arc<StoqShardTransport>> = None;
    let mut shard_location_index_ref: Option<
        std::sync::Arc<blockmatrix::network::swarm_provider::ShardLocationIndex>,
    > = None;
    let mut consumer_provider_manager_ref: Option<
        std::sync::Arc<blockmatrix::network::consumer_provider::ConsumerProviderManager>,
    > = None;
    #[cfg(feature = "intelligence")]
    let mut engauge_bits: Option<EngaugeBits> = None;

    let privacy_mode = bootstrap.privacy_mode().await;
    let has_bootstrap_peers = !cli.bootstrap.is_empty();

    if privacy_mode != PrivacyMode::PRIVATE || has_bootstrap_peers {
        let result = start_network(
            cli, coord, nid, data_dir, bootstrap, privacy_mode, has_bootstrap_peers,
        )
        .await?;
        network_ref = Some(result.network);
        shard_store_ref = Some(result.shard_store);
        shard_transport_ref = Some(result.shard_transport);
        shard_location_index_ref = Some(result.shard_location_index);
        consumer_provider_manager_ref = Some(result.consumer_provider_manager);
        #[cfg(feature = "intelligence")]
        {
            engauge_bits = Some(EngaugeBits {
                demand_tracker: result.swarm_demand_tracker,
                analytics: result.engauge_analytics,
            });
        }
    }

    // --- IPC Server Setup ---
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);

    let daemon_shard_store = shard_store_ref.unwrap_or_else(|| {
        std::sync::Arc::new(ShardStore::new_with_dir(
            data_dir.join(nid).join("shards"),
        ))
    });

    #[cfg(feature = "caesar")]
    let caesar_instance = {
        match caesar::CaesarProtocol::new(caesar::CaesarConfig::default()).await {
            Ok(protocol) => {
                info!("Caesar EVP protocol initialized");
                Some(std::sync::Arc::new(tokio::sync::RwLock::new(protocol)))
            }
            Err(e) => {
                warn!("Caesar init failed, caesar.* IPC will report unavailable: {e}");
                None
            }
        }
    };

    let daemon_state = std::sync::Arc::new(ipc::DaemonState {
        blockchain: bootstrap.blockchain().clone(),
        persistence: persistence.clone(),
        network: network_ref,
        shard_store: daemon_shard_store,
        shard_transport: shard_transport_ref,
        coordinate: coord,
        node_id: nid.to_string(),
        data_dir: data_dir.to_path_buf(),
        privacy_mode: format!("{:?}", bootstrap.privacy_mode().await),
        started_at: std::time::Instant::now(),
        shutdown_tx: shutdown_tx.clone(),
        dns_resolver: bootstrap.dns().clone(),
        dns_popularity_tracker: Some(std::sync::Arc::new(
            blockmatrix::dns::DnsPopularityTracker::new(),
        )),
        shard_location_index: shard_location_index_ref,
        consumer_provider_manager: consumer_provider_manager_ref,
        #[cfg(feature = "caesar")]
        caesar: caesar_instance,
        #[cfg(feature = "intelligence")]
        engauge_bridge: engauge_bits.map(|eb| {
            std::sync::Arc::new(blockmatrix::intelligence::engauge_bridge::EngaugeBridge::new(
                eb.demand_tracker,
                eb.analytics,
                hypermesh_lib::MatrixPosition {
                    x: coord.x as f64,
                    y: coord.y as f64,
                    z: coord.z as f64,
                },
            ))
        }),
        // Phase F.1: not yet wired — these become Some(...) when the
        // node actually joins a federation.  Until then,
        // `trustchain.request_cert` falls through to local self-signing.
        #[cfg(feature = "intelligence")]
        federation_manager: None,
        #[cfg(feature = "intelligence")]
        threshold_coordinator: None,

        transfer_coordinator: None,
            foundation_signing_key: None,
            dns_registrar: None,
        receipt_validator: std::sync::Arc::new(
            blockmatrix::assets::cross_chain::CrossChainReceiptValidator::new(),
        ),
    });

    // Phase I.1: rebuild the cross-chain receipt index from any
    // receipts already on the local chain. This makes restart-recovery
    // deterministic — receipts written before a daemon restart remain
    // queryable via `chain.lookup_cross_receipt`.
    {
        let count = daemon_state
            .receipt_validator
            .rebuild_from_chain(&daemon_state.blockchain)
            .await;
        if count > 0 {
            tracing::info!(
                "CrossChainReceiptValidator: indexed {} receipt(s) from persisted chain",
                count
            );
        }
    }

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

    info!("API access available via IPC (Unix socket). STOQ API bridge deferred until protocol discriminator is implemented.");

    register_default_dashboard(bootstrap, data_dir, nid, coord).await;

    // --- DNS self-registration from --name flag ---
    if let Some(ref name) = cli.name {
        if let Err(e) = register_node_dns_name(name, bootstrap, data_dir, nid, coord).await {
            warn!("DNS self-registration failed for '{}': {}", name, e);
        }
    }

    info!("Node running in {:?} mode", bootstrap.privacy_mode().await);
    info!("Press Ctrl+C to stop");

    #[cfg(unix)]
    let mut sigterm = match tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::terminate(),
    ) {
        Ok(sig) => Some(sig),
        Err(e) => {
            warn!("Failed to install SIGTERM handler: {}", e);
            None
        }
    };

    #[cfg(unix)]
    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            if let Err(e) = result {
                warn!("Failed to listen for Ctrl+C: {}", e);
            }
            info!("SIGINT received, shutting down...");
        }
        _ = async {
            match sigterm.as_mut() {
                Some(sig) => { sig.recv().await; }
                None => std::future::pending::<()>().await,
            }
        } => {
            info!("SIGTERM received, shutting down...");
        }
        _ = shutdown_rx.changed() => {
            info!("Shutdown requested via IPC");
        }
    }

    #[cfg(not(unix))]
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

/// Engauge handles carried back from `start_network` when the `intelligence`
/// feature is enabled.
#[cfg(feature = "intelligence")]
struct EngaugeBits {
    demand_tracker: std::sync::Arc<blockmatrix::network::SwarmDemandTracker>,
    analytics: std::sync::Arc<std::sync::Mutex<engauge::SwarmAnalytics>>,
}

/// Result bundle returned from `start_network`.
struct NetworkStartResult {
    network: std::sync::Arc<NetworkManager>,
    shard_store: std::sync::Arc<ShardStore>,
    shard_transport: std::sync::Arc<StoqShardTransport>,
    /// Shared shard location index — same instance used by PeerContext and
    /// the IPC daemon, so TAG_SHARD_ANNOUNCE updates and local provider
    /// registrations converge.
    shard_location_index: std::sync::Arc<
        blockmatrix::network::swarm_provider::ShardLocationIndex,
    >,
    /// Consumer-becomes-provider manager (R12) wired to the shared
    /// `shard_location_index`. Used by IPC fetch handlers to register the
    /// local node as a provider after a network fetch and to broadcast
    /// TAG_SHARD_ANNOUNCE to peers.
    consumer_provider_manager: std::sync::Arc<
        blockmatrix::network::consumer_provider::ConsumerProviderManager,
    >,
    #[cfg(feature = "intelligence")]
    swarm_demand_tracker: std::sync::Arc<blockmatrix::network::SwarmDemandTracker>,
    #[cfg(feature = "intelligence")]
    engauge_analytics: std::sync::Arc<std::sync::Mutex<engauge::SwarmAnalytics>>,
}

/// Initialize STOQ transport, network manager, and all background loops.
#[allow(clippy::too_many_arguments)]
async fn start_network(
    cli: &Cli,
    coord: MatrixCoordinate,
    nid: &str,
    data_dir: &std::path::Path,
    bootstrap: &NodeBootstrap,
    privacy_mode: PrivacyMode,
    has_bootstrap_peers: bool,
) -> Result<NetworkStartResult> {
    info!("Initializing STOQ transport on port {}", cli.stoq_port);

    let mut stoq_config = stoq::TransportConfig {
        port: cli.stoq_port,
        bind_address: std::net::Ipv6Addr::UNSPECIFIED,
        ..stoq::TransportConfig::default()
    };

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

    let identity_dir = data_dir.join(nid).join("identity");
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
        ),
    );

    // --- engauge intelligence wiring (H2-H5) ---
    #[cfg(feature = "intelligence")]
    let engauge_analytics = std::sync::Arc::new(
        std::sync::Mutex::new(engauge::SwarmAnalytics::new()),
    );
    #[cfg(feature = "intelligence")]
    let engauge_ingestion = std::sync::Arc::new(
        std::sync::Mutex::new(engauge::MetricsIngestionPipeline::with_defaults()),
    );

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
        dns_popularity_tracker: Some(std::sync::Arc::new(
            blockmatrix::dns::DnsPopularityTracker::new(),
        )),
        shard_location_index: Some(shard_location_index.clone()),
        inbox_store: Some(std::sync::Arc::new(
            blockmatrix::sharing::inbox::InboxStore::new(
                Some(data_dir.join(nid).join("inbox")),
            ),
        )),
        message_store: Some(std::sync::Arc::new(
            blockmatrix::messaging::store::MessageStore::new(
                Some(data_dir.join(nid).join("messages")),
            ),
        )),
        rotation_chains: Some(std::sync::Arc::new(
            tokio::sync::RwLock::new(std::collections::HashMap::new()),
        )),
        #[cfg(feature = "intelligence")]
        engauge_analytics: Some(engauge_analytics.clone()),
        #[cfg(feature = "intelligence")]
        engauge_ingestion: Some(engauge_ingestion.clone()),
        // Phase G.2 — alpha-default inert. Daemon opt-in plumbs a real
        // TransferCoordinator here once a STOQ-backed TransferTransport
        // is configured (Phase G.2 deliverable on the daemon side).
        transfer_coordinator: None,
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

    // --- H3: Spawn EngaugeBridge periodic feed loop ---
    #[cfg(feature = "intelligence")]
    {
        let bridge_position = hypermesh_lib::MatrixPosition {
            x: coord.x as f64,
            y: coord.y as f64,
            z: coord.z as f64,
        };
        // Spawn periodic feed: every 10 seconds, feed demand data into SwarmAnalytics.
        // We implement the loop here instead of calling run_periodic_feed() because
        // the std::sync::MutexGuard held by that method is not Send-safe across await.
        let feed_tracker = swarm_demand_tracker.clone();
        let feed_analytics = engauge_analytics.clone();
        let feed_position = bridge_position;
        tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(10);
            loop {
                tokio::time::sleep(interval).await;
                // Snapshot demand data (async lock).
                let snapshot = feed_tracker.snapshot().await;
                // Feed into analytics (sync lock, no await while held).
                match feed_analytics.lock() {
                    Ok(mut analytics) => {
                        for (shard_id, entry) in &snapshot {
                            for requester_id in &entry.requester_ids {
                                let consumer_id = hypermesh_lib::NodeId::from_public_key(
                                    requester_id.as_bytes(),
                                );
                                analytics.record_request(
                                    *shard_id,
                                    consumer_id,
                                    feed_position,
                                    entry.last_request_us,
                                );
                            }
                        }
                        if !snapshot.is_empty() {
                            debug!(
                                "Fed {} shard demand entries into SwarmAnalytics",
                                snapshot.len(),
                            );
                        }
                    }
                    Err(e) => {
                        debug!("Failed to lock analytics for feed: {e}");
                    }
                }
            }
        });
        info!("engauge intelligence bridge started (periodic_feed=10s)");

        // --- Phase E.1: Construct eBPF feedback adapter for the routing
        // intelligence feed. The adapter pushes congestion-derived privacy
        // actions and routing rules from engauge into HyperMeshEbpf.
        let ebpf_for_feedback: Option<std::sync::Arc<hypermesh_ebpf::HyperMeshEbpf>> =
            match hypermesh_ebpf::HyperMeshEbpf::new(hypermesh_ebpf::EbpfConfig::default()) {
                Ok(e) => Some(std::sync::Arc::new(e)),
                Err(err) => {
                    warn!("eBPF orchestrator unavailable, feedback loop disabled: {err}");
                    None
                }
            };
        if ebpf_for_feedback.is_some() {
            info!("eBPF feedback adapter ready for engauge routing intelligence");
        }

        // --- H4: Spawn propagation weight feed loop ---
        let h4_analytics = engauge_analytics.clone();
        let h4_propagator = block_propagator.clone();
        let h4_network = network_clone.clone();
        let h4_coord = coord;
        let h4_ebpf = ebpf_for_feedback.clone();
        tokio::spawn(async move {
            // Construct a RoutingIntelFeed once and reuse it across iterations
            // so the eBPF feedback adapter remains attached.
            let mut feed = engauge::routing_intel::RoutingIntelFeed::new(30);
            if let Some(ebpf) = h4_ebpf.clone() {
                let adapter: Box<dyn engauge::routing_intel::EbpfPolicyFeedback> =
                    Box::new(blockmatrix::intelligence::EbpfFeedbackAdapter::new(ebpf));
                feed.set_ebpf_feedback(adapter);
            }

            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(15));
            loop {
                interval.tick().await;
                // Build a node_id -> coordinate map from connected peers.
                let nodes = h4_network.get_connected_nodes().await;
                if nodes.is_empty() {
                    continue;
                }
                let mut node_coords: std::collections::HashMap<
                    hypermesh_lib::NodeId,
                    blockmatrix::matrix::coordinate::MatrixCoordinate,
                > = std::collections::HashMap::new();
                let mut candidate_ids: Vec<hypermesh_lib::NodeId> = Vec::new();
                for node in &nodes {
                    let nid = hypermesh_lib::NodeId::from_public_key(node.node_id.as_bytes());
                    node_coords.insert(nid, node.coordinate);
                    candidate_ids.push(nid);
                }
                // Use RoutingIntelligence to compute weight adjustments.
                // In alpha, we create a fresh instance per cycle (no subscriber data
                // accumulated yet, so weights will be neutral=1.0). When the
                // MetricsIngestionPipeline starts feeding RoutingIntelligence
                // in a later sprint, these weights become meaningful.
                let ri = engauge::RoutingIntelligence::new(30);
                let source_pos = hypermesh_lib::MatrixPosition {
                    x: h4_coord.x as f64,
                    y: h4_coord.y as f64,
                    z: h4_coord.z as f64,
                };
                // Drive the feed's eBPF feedback hook by publishing an update.
                // When the adapter is attached, this propagates congestion-
                // derived privacy actions to HyperMeshEbpf.
                let _ = feed.publish_update(&source_pos, &source_pos, &candidate_ids);

                let modifiers = engauge::RoutingAdvisor::compute_weight_adjustments(
                    &ri, &source_pos, &source_pos, &candidate_ids,
                );
                if !modifiers.is_empty() {
                    let weights = blockmatrix::intelligence::engauge_bridge::compute_propagation_weights(
                        &modifiers, &node_coords,
                    );
                    if !weights.is_empty() {
                        h4_propagator.lock().await.set_propagation_weights(weights).await;
                        debug!("Updated propagation weights from engauge ({} modifiers)", modifiers.len());
                    }
                }

                // --- H5: Check replication triggers ---
                match h4_analytics.lock() {
                    Ok(analytics) => {
                        let trigger = engauge::ReplicationTrigger::new(
                            engauge::ReplicationConfig::default(),
                        );
                        let signals = trigger.check(&analytics);
                        for signal in &signals {
                            if signal.urgency > 0.5 {
                                info!(
                                    "Replication signal: shard {} needs {} more replicas (urgency: {:.2}, rate: {})",
                                    hex::encode(&signal.shard_id.0[..4]),
                                    signal.suggested_count,
                                    signal.urgency,
                                    signal.current_request_rate,
                                );
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Failed to lock analytics for replication check: {e}");
                    }
                }
            }
        });
        info!("engauge propagation weight + replication loop started (interval=15s)");

        // --- Phase E.2: Replication-poll task. Every 30s, ask engauge which
        // shards need more replicas and proactively fetch additional copies
        // from known providers via TAG_SHARD_FETCH. Closes the consumer-
        // becomes-provider loop: hot shards get pulled by additional nodes,
        // and each successful fetch announces the new node as a provider,
        // so future requests fan out across the swarm.
        {
            let rp_analytics = engauge_analytics.clone();
            let rp_index = shard_location_index.clone();
            let rp_transport = shard_transport.clone();
            let rp_local_node_id = nid.to_string();
            tokio::spawn(async move {
                let mut interval =
                    tokio::time::interval(tokio::time::Duration::from_secs(30));
                // Skip the immediate tick so the first fetch happens after
                // the network has had time to come up.
                interval.tick().await;
                loop {
                    interval.tick().await;
                    let signals = match rp_analytics.lock() {
                        Ok(guard) => engauge::ReplicationTrigger::new(
                            engauge::ReplicationConfig::default(),
                        )
                        .check(&guard),
                        Err(e) => {
                            debug!(
                                "replication-poll: analytics lock poisoned: {e}"
                            );
                            continue;
                        }
                    };
                    if signals.is_empty() {
                        continue;
                    }
                    for signal in signals.iter().filter(|s| s.urgency > 0.5) {
                        // Find peers known to provide this shard.
                        let providers = rp_index.get_providers(&signal.shard_id).await;
                        // Skip if we are the only known provider (cannot
                        // self-replicate) or no providers at all.
                        let candidates: Vec<&String> = providers
                            .iter()
                            .filter(|id| id.as_str() != rp_local_node_id.as_str())
                            .collect();
                        if candidates.is_empty() {
                            debug!(
                                "replication-poll: no remote providers for shard {} yet",
                                hex::encode(&signal.shard_id.0[..4])
                            );
                            continue;
                        }
                        // Pick the first candidate (alpha policy — refine
                        // with engauge dispersion in a later sprint).
                        let target_node_id = candidates[0].clone();
                        let target_id = hypermesh_lib::NodeId::from_public_key(
                            target_node_id.as_bytes(),
                        );
                        use blockmatrix::network::shard_transport::ShardTransport;
                        match rp_transport
                            .fetch_shard(&target_id, &signal.shard_id)
                            .await
                        {
                            Ok(_data) => {
                                info!(
                                    "replication-poll: fetched extra replica of {} from {} (urgency {:.2})",
                                    hex::encode(&signal.shard_id.0[..4]),
                                    &target_node_id[..8.min(target_node_id.len())],
                                    signal.urgency,
                                );
                            }
                            Err(e) => {
                                debug!(
                                    "replication-poll: fetch from {} failed: {}",
                                    &target_node_id[..8.min(target_node_id.len())],
                                    e,
                                );
                            }
                        }
                    }
                }
            });
            info!("Phase E.2 replication-poll loop started (interval=30s)");
        }
    }

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
        engauge_analytics,
    })
}

/// Register a DNS name for this node on the local blockchain.
///
/// This replicates the `dns register` command logic so that nodes can
/// self-register their name at boot via `--name`.
async fn register_node_dns_name(
    name: &str,
    bootstrap: &NodeBootstrap,
    data_dir: &std::path::Path,
    node_id: &str,
    coord: MatrixCoordinate,
) -> Result<()> {
    // TODO: Use the node's peer-facing address once NAT detection is implemented.
    // For now, register with ::1 -- peers discover the real address from the
    // STOQ connection's remote address, not from the DNS record's stored IP.
    let target_addr = std::net::IpAddr::from(std::net::Ipv6Addr::LOCALHOST);

    // Register in the in-memory resolver
    bootstrap.dns().register(name.to_string(), target_addr).await;

    // Persist to disk so the record survives restarts
    super::dns::persist_dns_record(data_dir, node_id, name, target_addr)?;

    // Write a blockchain DNS asset so it propagates to peers
    let bc = bootstrap.blockchain();
    let ipv6_addr = match target_addr {
        std::net::IpAddr::V6(v6) => v6,
        std::net::IpAddr::V4(v4) => v4.to_ipv6_mapped(),
    };
    let dns_entry = blockmatrix::dns::DnsBlockEntry {
        domain_name: name.to_string(),
        record_type: blockmatrix::dns::DnsRecordType::AAAA,
        record_data: blockmatrix::dns::DnsRecordData::AAAA(ipv6_addr),
        ttl: 300,
        owner: node_id.to_string(),
        grant_signature: None,
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
    let state_proof = crate::hardware::build_hardware_state_proof(node_id, coord);
    let block = bc
        .register_dns_asset(registration, &state_proof, dns_bytes)
        .await
        .map_err(|e| anyhow::anyhow!("blockchain write failed: {e}"))?;

    info!(
        "DNS self-registered '{}' -> {} (block #{})",
        name, target_addr, block.index,
    );

    Ok(())
}
