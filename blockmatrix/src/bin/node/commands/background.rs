// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Background loops -- peer sync, block sync, gossip, reflector heartbeat.

use tracing::{debug, info, warn};

use blockmatrix::blockchain::propagation::BlockPropagator;
use blockmatrix::blockchain::sync_manager::SyncManager;
use blockmatrix::create_os_abstraction;
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::reflector_pool::ReflectorPool;
use blockmatrix::network::shard_store::ShardStore;
use blockmatrix::network::shard_transport::StoqShardTransport;
use blockmatrix::network::sync_dispatch::TransportSyncDriver;
use blockmatrix::network::NetworkManager;

pub(super) fn spawn_peer_sync_loop(
    network: std::sync::Arc<NetworkManager>,
    shard_transport: std::sync::Arc<StoqShardTransport>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        loop {
            interval.tick().await;

            let nodes = network.get_connected_nodes().await;
            update_shard_connections(&nodes, &shard_transport).await;
            report_connected_nodes(&nodes, &network).await;
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_block_sync_loop(
    sync_manager: std::sync::Arc<tokio::sync::Mutex<SyncManager>>,
    reflector_pool: std::sync::Arc<tokio::sync::Mutex<ReflectorPool>>,
    blockchain: std::sync::Arc<blockmatrix::blockchain::node_chain::NodeBlockchain>,
    network: std::sync::Arc<NetworkManager>,
    node_map: std::sync::Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, (String, std::net::SocketAddr)>>,
    >,
    transport: std::sync::Arc<stoq::StoqTransport>,
    block_propagator: std::sync::Arc<tokio::sync::Mutex<BlockPropagator>>,
    peer_coords: blockmatrix::network::connected_peer_routes::ConnectedPeerRoutes,
    coord: MatrixCoordinate,
    metrics_node_id: String,
    network_id: String,
    is_reflector: bool,
    shard_store: std::sync::Arc<ShardStore>,
) {
    let service = block_sync::BlockSyncLoop {
        sync_manager,
        reflector_pool,
        blockchain,
        network,
        node_map,
        transport,
        block_propagator,
        peer_coords,
        coord,
        network_id,
        is_reflector,
        shard_store,
    };
    tokio::spawn(async move {
        service.run(metrics_node_id).await;
    });

    info!(
        "Block sync loop started (interval=5s, reflector={}, metrics_interval=30s)",
        is_reflector
    );
}

async fn send_reflector_heartbeat(
    blockchain: &std::sync::Arc<blockmatrix::blockchain::node_chain::NodeBlockchain>,
    network: &std::sync::Arc<NetworkManager>,
    network_id: &str,
) {
    let local_height = blockchain.get_height().await;
    debug!("Reflector heartbeat: height={}", local_height);

    let heartbeat_msg = blockmatrix::network::stoq_integration::MatrixMessage::ReflectorHeartbeat {
        network_id: network_id.to_string(),
        block_height: local_height,
        health_score: 1.0,
    };

    if let Ok(heartbeat_json) = serde_json::to_vec(&heartbeat_msg) {
        let mut tagged = Vec::with_capacity(1 + heartbeat_json.len());
        tagged.push(0x10u8); // TAG_SYNC_MESSAGE
        tagged.extend_from_slice(&heartbeat_json);

        let nodes = network.get_connected_nodes().await;
        for node in &nodes {
            send_heartbeat_to_peer(node, &tagged).await;
        }
    }
}

pub(super) async fn spawn_gossip_loop(
    gossip_proto: std::sync::Arc<blockmatrix::network::gossip::GossipProtocol>,
    network: std::sync::Arc<NetworkManager>,
    shard_store: std::sync::Arc<ShardStore>,
    coord: MatrixCoordinate,
) {
    gossip_proto.start().await;

    let gossip_ref = gossip_proto.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(gossip_ref.gossip_interval());
        loop {
            interval.tick().await;

            gossip_round(&gossip_ref, &network, &shard_store, coord).await;
        }
    });
    info!("Gossip protocol started (interval=15s, fanout=3)");
}

/// Send a gossip message to a single peer over a STOQ stream.
async fn send_gossip_to_peer(node: &blockmatrix::network::NetworkNode, msg_bytes: &[u8]) {
    let conn = match node.connection.as_ref() {
        Some(c) => c,
        None => return,
    };

    let short_id = &node.node_id[..8.min(node.node_id.len())];

    let (mut send, _recv) = match conn.open_bi().await {
        Ok(streams) => streams,
        Err(e) => {
            debug!("Gossip stream open to {} failed: {}", short_id, e);
            return;
        }
    };

    if let Err(e) = send
        .write_all(&[blockmatrix::network::CONN_TYPE_GOSSIP])
        .await
    {
        debug!("Gossip discriminator write to {} failed: {}", short_id, e);
        return;
    }
    if let Err(e) = send.write_all(msg_bytes).await {
        debug!("Gossip write to {} failed: {}", short_id, e);
        return;
    }
    if let Err(e) = send.finish() {
        debug!("Gossip stream finish to {} failed: {}", short_id, e);
        return;
    }
    debug!("Sent gossip to peer {}", short_id);
}

/// Phase F.1: periodic peer-CA discovery.
///
/// Every 60 seconds, iterates the currently connected peers via
/// [`NetworkManager::get_connected_nodes`] and calls
/// [`trustchain::ca::FederationManager::add_peer`] for any peer not
/// already in the federation.  PoS gating, byzantine override, and
/// ngauge trust-band capping are all enforced inside `add_peer`, so
/// the discovery loop only needs to surface candidates.
///
/// This is a no-op when `federation` is `None`, which is the alpha
/// default until the daemon explicitly opts into federation.
#[cfg(feature = "intelligence")]
pub(super) fn spawn_ca_discovery_loop(
    network: std::sync::Arc<NetworkManager>,
    federation: std::sync::Arc<trustchain::ca::FederationManager>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        // Skip the immediate tick — let the network warm up.
        interval.tick().await;
        loop {
            interval.tick().await;
            ca_discovery_round(&network, &federation).await;
        }
    });
}

mod block_sync;

async fn gossip_round(
    gossip_ref: &blockmatrix::network::gossip::GossipProtocol,
    network: &NetworkManager,
    shard_store: &ShardStore,
    coord: MatrixCoordinate,
) {
    let shard_count = shard_store.count().await;
    let asset_ids: Vec<String> = (0..shard_count.min(100))
        .map(|i| format!("shard-{i}"))
        .collect();
    gossip_ref.update_local(coord, asset_ids).await;
    gossip_ref.prune_stale(300).await;

    let nodes = network.get_connected_nodes().await;
    if nodes.is_empty() {
        return;
    }

    let msg = gossip_ref.build_outgoing_message().await;
    let msg_bytes = match serde_json::to_vec(&msg) {
        Ok(b) => b,
        Err(e) => {
            debug!("Failed to serialize gossip message: {}", e);
            return;
        }
    };

    let targets = gossip_ref.select_gossip_targets().await;
    for node in &nodes {
        if !targets.contains(&node.node_id) {
            continue;
        }
        send_gossip_to_peer(node, &msg_bytes).await;
    }

    debug!(
        "Gossip round: sent to {} of {} peers",
        targets.len().min(nodes.len()),
        nodes.len(),
    );
}

#[cfg(feature = "intelligence")]
async fn ca_discovery_round(
    network: &NetworkManager,
    federation: &trustchain::ca::FederationManager,
) {
    let nodes = network.get_connected_nodes().await;
    if nodes.is_empty() {
        return;
    }
    // Snapshot existing peers to avoid duplicate add_peer calls.
    let known: std::collections::HashSet<String> = federation
        .list_peers()
        .await
        .into_iter()
        .map(|p| p.ca_id)
        .collect();

    for node in &nodes {
        if known.contains(&node.node_id) {
            continue;
        }
        register_ca_candidate(node, federation).await;
    }
}

#[cfg(feature = "intelligence")]
async fn register_ca_candidate(
    node: &blockmatrix::network::NetworkNode,
    federation: &trustchain::ca::FederationManager,
) {
    use std::time::SystemTime;
    use trustchain::ca::{FederatedCA, FederationTrustLevel};
    let peer = FederatedCA {
        ca_id: node.node_id.clone(),
        name: format!("peer-{}", &node.node_id[..8.min(node.node_id.len())]),
        public_key: Vec::new(), // populated when handshake exposes the peer's key
        root_certificate: Vec::new(),
        trust_level: FederationTrustLevel::Conditional,
        joined_at: SystemTime::now(),
        last_sync: None,
        endpoint: node.address.to_string(),
    };
    match federation.add_peer(peer).await {
        Ok(()) => {
            debug!(
                "CA discovery: added peer '{}' to federation",
                &node.node_id[..8.min(node.node_id.len())]
            );
        }
        Err(e) => {
            debug!(
                "CA discovery: add_peer for '{}' failed: {}",
                &node.node_id[..8.min(node.node_id.len())],
                e
            );
        }
    }
}

async fn send_heartbeat_to_peer(node: &blockmatrix::network::NetworkNode, tagged: &[u8]) {
    let Some(conn) = node.connection.as_ref() else {
        return;
    };
    match conn.open_stream().await {
        Ok(stream) => send_heartbeat_payload(stream, tagged, &node.node_id).await,
        Err(e) => {
            debug!(
                "Heartbeat stream to {} failed: {}",
                &node.node_id[..8.min(node.node_id.len())],
                e
            );
        }
    }
}

async fn send_heartbeat_payload(mut stream: stoq::Stream, tagged: &[u8], node_id: &str) {
    if let Err(e) = stream.send(tagged).await {
        debug!(
            "Heartbeat send to {} failed: {}",
            &node_id[..8.min(node_id.len())],
            e
        );
    }
}

async fn update_shard_connections(
    nodes: &[blockmatrix::network::NetworkNode],
    shard_transport: &StoqShardTransport,
) {
    for node in nodes {
        let node_id =
            hypermesh_lib::NodeId::from_bytes(*blake3::hash(node.node_id.as_bytes()).as_bytes());
        shard_transport
            .register_node_address(&node_id, node.address)
            .await;
        if let Some(ref conn) = node.connection {
            shard_transport
                .register_connection(&node_id, conn.clone())
                .await;
        }
    }
}

async fn report_connected_nodes(
    nodes: &[blockmatrix::network::NetworkNode],
    network: &NetworkManager,
) {
    let node_count = nodes.len();
    if node_count > 0 {
        info!("Connected nodes: {}", node_count);
        let neighbors = network.find_matrix_neighbors(10.0).await;
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
