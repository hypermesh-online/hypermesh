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

use blockmatrix::bootstrap::{NodeBootstrap, PrivacyMode};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::shard_store::ShardStore;
use blockmatrix::network::shard_transport::StoqShardTransport;
use blockmatrix::network::NetworkManager;

use super::node_transport::{
    bootstrap_nodes, create_transport, discover_network, initialize_block_sync,
    node_kernel_acceleration_allowed, Acceleration, BlockSyncSetup, PeerCore, ShardBase,
    ShardServices,
};
#[cfg(test)]
use super::node_transport::{stoq_listener_config, uses_physical_interface};
use crate::cli::Cli;

#[cfg(feature = "intelligence")]
use super::background::spawn_ca_discovery_loop;
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
    pub(crate) shard_location_index:
        std::sync::Arc<blockmatrix::network::swarm_provider::ShardLocationIndex>,
    /// Consumer-becomes-provider manager (R12) wired to the shared
    /// `shard_location_index`. Used by IPC fetch handlers to register the
    /// local node as a provider after a network fetch and to broadcast
    /// TAG_SHARD_ANNOUNCE to peers.
    pub(crate) consumer_provider_manager:
        std::sync::Arc<blockmatrix::network::consumer_provider::ConsumerProviderManager>,
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
    // Federation CA handle for the intelligence-gated CA-discovery loop. The
    // daemon passes `None` in the alpha default (self-signing only); it becomes
    // `Some(..)` once the node opts into a federation, at which point the
    // discovery loop begins surfacing connected peers to `FederationManager`.
    #[cfg(feature = "intelligence")] federation_manager: Option<
        std::sync::Arc<trustchain::ca::FederationManager>,
    >,
) -> Result<NetworkStartResult> {
    run_network(NetworkStartInput {
        cli,
        coord,
        nid,
        data_dir,
        bootstrap,
        privacy_mode,
        has_bootstrap_peers,
        share_inbox_store,
        #[cfg(feature = "intelligence")]
        federation_manager,
    })
    .await
}

struct NetworkStartInput<'a> {
    cli: &'a Cli,
    coord: MatrixCoordinate,
    nid: &'a str,
    data_dir: &'a std::path::Path,
    bootstrap: &'a NodeBootstrap,
    privacy_mode: PrivacyMode,
    has_bootstrap_peers: bool,
    share_inbox_store: std::sync::Arc<blockmatrix::sharing::inbox::InboxStore>,
    #[cfg(feature = "intelligence")]
    federation_manager: Option<std::sync::Arc<trustchain::ca::FederationManager>>,
}

struct BaseNetwork {
    transport: std::sync::Arc<stoq::StoqTransport>,
    shard_base: ShardBase,
    manager: NetworkManager,
    blocks: BlockSyncSetup,
}

async fn prepare_base(input: &NetworkStartInput<'_>) -> Result<BaseNetwork> {
    let transport = create_transport(input.cli, input.privacy_mode).await?;
    let bootstrap_nodes = bootstrap_nodes(input.cli);
    let shard_base = ShardBase::new(input.data_dir, input.nid, &transport).await;
    let manager = discover_network(
        input.cli,
        input.coord,
        input.data_dir,
        input.privacy_mode,
        transport.clone(),
        bootstrap_nodes,
    )
    .await?;
    let blocks = initialize_block_sync(
        input.coord,
        input.bootstrap,
        &input.cli.network_id,
        input.privacy_mode,
        input.has_bootstrap_peers || input.cli.reflector,
    )
    .await;
    Ok(BaseNetwork {
        transport,
        shard_base,
        manager,
        blocks,
    })
}

async fn run_network(input: NetworkStartInput<'_>) -> Result<NetworkStartResult> {
    let BaseNetwork {
        transport,
        shard_base,
        manager,
        blocks,
    } = prepare_base(&input).await?;
    let core = PeerCore::new(
        input.cli,
        input.coord,
        input.nid,
        input.privacy_mode,
        manager.connected_routes(),
    );
    let shards = shard_base.with_provider(input.nid, &input.cli.network_id);
    let acceleration = Acceleration::new(node_kernel_acceleration_allowed(input.cli));
    let ctx = make_peer_context(&input, &manager, &blocks, &shards, &core, &acceleration);
    let network = std::sync::Arc::new(manager);
    start_post_discovery(
        &input,
        &transport,
        &network,
        &ctx,
        &blocks,
        &shards,
        &core,
        &acceleration,
    )
    .await?;
    Ok(network_result(network, shards, core, acceleration))
}

fn make_peer_context(
    input: &NetworkStartInput<'_>,
    manager: &NetworkManager,
    blocks: &BlockSyncSetup,
    shards: &ShardServices,
    core: &PeerCore,
    acceleration: &Acceleration,
) -> std::sync::Arc<blockmatrix::network::PeerContext> {
    PeerContextBuild {
        cli: input.cli,
        coord: input.coord,
        nid: input.nid,
        data_dir: input.data_dir,
        bootstrap: input.bootstrap,
        has_bootstrap_peers: input.has_bootstrap_peers,
        inbox: input.share_inbox_store.clone(),
        manager,
        blocks,
        shards,
        core,
        acceleration,
    }
    .build()
}

async fn start_post_discovery(
    input: &NetworkStartInput<'_>,
    transport: &std::sync::Arc<stoq::StoqTransport>,
    network: &std::sync::Arc<NetworkManager>,
    ctx: &std::sync::Arc<blockmatrix::network::PeerContext>,
    blocks: &BlockSyncSetup,
    shards: &ShardServices,
    core: &PeerCore,
    acceleration: &Acceleration,
) -> Result<()> {
    activate_peer_links(
        input.cli,
        network,
        ctx,
        blocks,
        input.has_bootstrap_peers,
        input.privacy_mode,
    )
    .await?;
    spawn_base_loops(input.cli, network, blocks, shards, core, ctx, transport).await;
    #[cfg(feature = "intelligence")]
    if let Some(ref fed) = input.federation_manager {
        spawn_ca_discovery_loop(network.clone(), fed.clone());
    }
    #[cfg(feature = "intelligence")]
    spawn_replication(
        input.privacy_mode,
        network,
        blocks,
        shards,
        core,
        acceleration,
        ctx,
    )
    .await?;
    propagate_bootstrap_blocks(input.bootstrap, blocks, network).await;
    Ok(())
}

fn network_result(
    network: std::sync::Arc<NetworkManager>,
    shards: ShardServices,
    core: PeerCore,
    acceleration: Acceleration,
) -> NetworkStartResult {
    NetworkStartResult {
        network,
        shard_store: shards.store,
        shard_transport: shards.transport,
        shard_location_index: shards.location_index,
        consumer_provider_manager: shards.consumer_provider,
        #[cfg(feature = "intelligence")]
        swarm_demand_tracker: core.demand_tracker,
        #[cfg(feature = "intelligence")]
        ngauge_analytics: acceleration.analytics,
    }
}

struct PeerContextBuild<'a> {
    cli: &'a Cli,
    coord: MatrixCoordinate,
    nid: &'a str,
    data_dir: &'a std::path::Path,
    bootstrap: &'a NodeBootstrap,
    has_bootstrap_peers: bool,
    inbox: std::sync::Arc<blockmatrix::sharing::inbox::InboxStore>,
    manager: &'a NetworkManager,
    blocks: &'a BlockSyncSetup,
    shards: &'a ShardServices,
    core: &'a PeerCore,
    acceleration: &'a Acceleration,
}

impl PeerContextBuild<'_> {
    fn build(self) -> std::sync::Arc<blockmatrix::network::PeerContext> {
        std::sync::Arc::new(blockmatrix::network::PeerContext {
            blockchain: self.bootstrap.blockchain().clone(),
            shard_store: self.shards.store.clone(),
            sync_manager: self.blocks.sync_manager.clone(),
            reflector_pool: self.blocks.reflector_pool.clone(),
            block_propagator: self.blocks.block_propagator.clone(),
            our_coordinate: self.coord,
            node_id: self.nid.to_string(),
            network_id: self.cli.network_id.clone(),
            blockchain_scope: if self.has_bootstrap_peers || self.cli.reflector {
                hypermesh_lib::BlockchainScope::Network
            } else {
                hypermesh_lib::BlockchainScope::Device
            },
            spatial_bucket_assigner: None,
            connected_routes: self.core.connected_coords.clone(),
            connected_nodes: self.manager.connected_nodes(),
            dns_resolver: Some(self.bootstrap.dns().clone()),
            authenticated_peers: self.manager.authenticated_peers(),
            gossip_protocol: Some(self.core.gossip.clone()),
            swarm_demand_tracker: self.core.demand_tracker.clone(),
            shard_location_index: Some(self.shards.location_index.clone()),
            inbox_store: Some(self.inbox),
            message_store: Some(std::sync::Arc::new(
                blockmatrix::messaging::store::MessageStore::new(Some(
                    self.data_dir.join(self.nid).join("messages"),
                )),
            )),
            rotation_chains: Some(std::sync::Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            ))),
            #[cfg(feature = "intelligence")]
            ngauge_analytics: Some(self.acceleration.analytics.clone()),
            #[cfg(feature = "intelligence")]
            ngauge_ingestion: Some(self.acceleration.ingestion.clone()),
            transfer_coordinator: None,
            ebpf: self.acceleration.ebpf.clone(),
        })
    }
}

async fn activate_peer_links(
    cli: &Cli,
    network: &std::sync::Arc<NetworkManager>,
    ctx: &std::sync::Arc<blockmatrix::network::PeerContext>,
    blocks: &BlockSyncSetup,
    has_bootstrap_peers: bool,
    privacy_mode: PrivacyMode,
) -> Result<()> {
    network.start_peer_message_loops(ctx.clone()).await;
    register_reflector_peers(
        network,
        &blocks.reflector_pool,
        &cli.network_id,
        has_bootstrap_peers,
        cli.reflector,
        privacy_mode,
    )
    .await;
    let addr_map = network.get_node_address_map().await;
    *blocks.node_map.write().await = addr_map;
    inject_authenticated_connections(network, &blocks.block_transport).await;
    let network_accept = network.clone();
    let ctx_accept = ctx.clone();
    tokio::spawn(async move {
        if let Err(e) = network_accept.accept_connections(Some(ctx_accept)).await {
            warn!("Connection acceptor error: {}", e);
        }
    });
    if cli.reflector {
        info!("Reflector mode: broadcasting matrix position");
        network.broadcast_matrix_position().await?;
    }
    info!(
        "Network initialized, accepting connections on port {}",
        cli.stoq_port
    );
    Ok(())
}

async fn inject_authenticated_connections(
    network: &std::sync::Arc<NetworkManager>,
    transport: &std::sync::Arc<blockmatrix::blockchain::stoq_transport::StoqBlockTransportAdapter>,
) {
    for node in network.get_connected_nodes().await {
        if let Some(ref conn) = node.connection {
            transport
                .inject_connection(&node.coordinate, conn.clone())
                .await;
        }
    }
}

async fn spawn_base_loops(
    cli: &Cli,
    network: &std::sync::Arc<NetworkManager>,
    blocks: &BlockSyncSetup,
    shards: &ShardServices,
    core: &PeerCore,
    ctx: &std::sync::Arc<blockmatrix::network::PeerContext>,
    transport: &std::sync::Arc<stoq::StoqTransport>,
) {
    spawn_peer_sync_loop(network.clone(), shards.transport.clone());
    spawn_block_sync_loop(
        blocks.sync_manager.clone(),
        blocks.reflector_pool.clone(),
        ctx.blockchain.clone(),
        network.clone(),
        blocks.node_map.clone(),
        transport.clone(),
        blocks.block_propagator.clone(),
        core.connected_coords.clone(),
        ctx.our_coordinate,
        ctx.node_id.clone(),
        ctx.network_id.clone(),
        cli.reflector,
        shards.store.clone(),
    );
    spawn_gossip_loop(
        core.gossip.clone(),
        network.clone(),
        shards.store.clone(),
        ctx.our_coordinate,
    )
    .await;
}

#[cfg(feature = "intelligence")]
async fn spawn_replication(
    privacy_mode: PrivacyMode,
    network: &std::sync::Arc<NetworkManager>,
    blocks: &BlockSyncSetup,
    shards: &ShardServices,
    core: &PeerCore,
    acceleration: &Acceleration,
    ctx: &std::sync::Arc<blockmatrix::network::PeerContext>,
) -> Result<()> {
    blockmatrix::network::replication_service::ReplicationService {
        coord: ctx.our_coordinate,
        node_id: ctx.node_id.clone(),
        privacy_mode,
        swarm_demand_tracker: core.demand_tracker.clone(),
        ngauge_analytics: acceleration.analytics.clone(),
        block_propagator: blocks.block_propagator.clone(),
        network: network.clone(),
        sync_manager: blocks.sync_manager.clone(),
        shard_location_index: shards.location_index.clone(),
        shard_transport: shards.transport.clone(),
        ebpf: acceleration.ebpf.clone(),
        blockchain: ctx.blockchain.clone(),
        shard_store: shards.store.clone(),
        consumer_provider: shards.consumer_provider.clone(),
    }
    .spawn()
    .await?;
    Ok(())
}

async fn propagate_bootstrap_blocks(
    bootstrap: &NodeBootstrap,
    blocks: &BlockSyncSetup,
    network: &std::sync::Arc<NetworkManager>,
) {
    let chain = bootstrap.blockchain().get_chain().await;
    for block in chain.iter().filter(|b| b.index > 0) {
        propagate_block(block, &blocks.block_propagator, network).await;
        let dns_count = count_dns_assets_in_block(block);
        if dns_count > 0 {
            debug!("Block #{} contains {} DNS asset(s)", block.index, dns_count);
        }
    }
}

#[cfg(test)]
#[path = "../tests/transport_config.rs"]
mod listener_config_tests;
