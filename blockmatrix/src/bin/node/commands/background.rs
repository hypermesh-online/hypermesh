// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Background loops -- peer sync, block sync, gossip, reflector heartbeat.

use tracing::{debug, info};

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
            for node in &nodes {
                let node_id = hypermesh_lib::NodeId::from_bytes(
                    *blake3::hash(node.node_id.as_bytes()).as_bytes(),
                );
                shard_transport
                    .register_node_address(&node_id, node.address)
                    .await;
                if let Some(ref conn) = node.connection {
                    shard_transport
                        .register_connection(&node_id, conn.clone())
                        .await;
                }
            }

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
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_block_sync_loop(
    sync_manager: std::sync::Arc<tokio::sync::Mutex<SyncManager>>,
    reflector_pool: std::sync::Arc<tokio::sync::Mutex<ReflectorPool>>,
    blockchain: std::sync::Arc<blockmatrix::blockchain::node_chain::NodeBlockchain>,
    network: std::sync::Arc<NetworkManager>,
    node_map: std::sync::Arc<
        tokio::sync::RwLock<
            std::collections::HashMap<String, (String, std::net::SocketAddr)>,
        >,
    >,
    transport: std::sync::Arc<stoq::StoqTransport>,
    block_propagator: std::sync::Arc<tokio::sync::Mutex<BlockPropagator>>,
    peer_coords: std::sync::Arc<tokio::sync::RwLock<Vec<MatrixCoordinate>>>,
    coord: MatrixCoordinate,
    metrics_node_id: String,
    network_id: String,
    is_reflector: bool,
    shard_store: std::sync::Arc<ShardStore>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        let mut metrics_reporter =
            blockmatrix::network::MetricsReporter::new(metrics_node_id);
        let os_abs = create_os_abstraction().ok();
        let mut cycle_count: u64 = 0;
        let mut last_propagated_height: u64 = blockchain.get_height().await;

        loop {
            interval.tick().await;
            cycle_count += 1;

            let addr_map = network.get_node_address_map().await;
            *node_map.write().await = addr_map;

            let live_coords = network.get_connected_coordinates().await;
            *peer_coords.write().await = live_coords;

            let addr_map_snapshot = node_map.read().await.clone();

            let fetched_blocks = {
                let mut sm = sync_manager.lock().await;
                let mut rp = reflector_pool.lock().await;
                TransportSyncDriver::run_sync_round(
                    &mut sm,
                    &mut rp,
                    &blockchain,
                    &transport,
                    &addr_map_snapshot,
                    &coord,
                )
                .await
            };

            for block in &fetched_blocks {
                match blockchain.insert_received_block(block.clone()).await {
                    Ok(()) => info!("Inserted synced block #{}", block.index),
                    Err(e) => debug!("Synced block #{} insertion failed: {}", block.index, e),
                }
            }
            if !fetched_blocks.is_empty() {
                info!("Sync round: fetched {} block(s)", fetched_blocks.len());
            }

            // Propagate locally-created blocks
            let current_height = blockchain.get_height().await;
            if current_height > last_propagated_height {
                let chain = blockchain.get_chain().await;
                for block in chain.iter().filter(|b| b.index > last_propagated_height) {
                    let coords = network.get_connected_coordinates().await;
                    if !coords.is_empty() {
                        let result = block_propagator
                            .lock()
                            .await
                            .propagate_block(block, &coords)
                            .await;
                        info!(
                            "Propagated block #{} to {} peer(s)",
                            block.index,
                            result.reached_nodes.len(),
                        );
                    }
                }
                last_propagated_height = current_height;
            }

            {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let pruned = reflector_pool.lock().await.prune_stale(now_ms);
                if pruned > 0 {
                    debug!("Pruned {} stale reflector(s)", pruned);
                }
            }

            if is_reflector {
                send_reflector_heartbeat(&blockchain, &network, &network_id).await;
            }

            // Emit node metrics every 30s (6 sync cycles)
            if cycle_count % 6 == 0 {
                let connected_nodes = network.get_connected_nodes().await;
                let chain_h = blockchain.get_height().await;
                let peers = connected_nodes.len();
                let shards = shard_store.count().await;
                let (cpu, mem) = os_abs
                    .as_ref()
                    .and_then(|os| os.get_resource_usage().ok())
                    .map(|u| (u.cpu_usage_percent, u.memory_usage_percent))
                    .unwrap_or((0.0, 0.0));
                let frame_bytes =
                    metrics_reporter.build_capacity_frame(chain_h, peers, shards, cpu, mem);
                metrics_reporter
                    .push_to_peers(&frame_bytes, &connected_nodes)
                    .await;
            }
        }
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

    let heartbeat_msg =
        blockmatrix::network::stoq_integration::MatrixMessage::ReflectorHeartbeat {
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
            if let Some(ref conn) = node.connection {
                match conn.open_stream().await {
                    Ok(mut stream) => {
                        if let Err(e) = stream.send(&tagged).await {
                            debug!(
                                "Heartbeat send to {} failed: {}",
                                &node.node_id[..8.min(node.node_id.len())],
                                e
                            );
                        }
                    }
                    Err(e) => {
                        debug!(
                            "Heartbeat stream to {} failed: {}",
                            &node.node_id[..8.min(node.node_id.len())],
                            e
                        );
                    }
                }
            }
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

            let shard_count = shard_store.count().await;
            let asset_ids: Vec<String> = (0..shard_count.min(100))
                .map(|i| format!("shard-{i}"))
                .collect();
            gossip_ref.update_local(coord, asset_ids).await;
            gossip_ref.prune_stale(300).await;

            let nodes = network.get_connected_nodes().await;
            if nodes.is_empty() {
                continue;
            }

            let msg = gossip_ref.build_outgoing_message().await;
            let msg_bytes = match serde_json::to_vec(&msg) {
                Ok(b) => b,
                Err(e) => {
                    debug!("Failed to serialize gossip message: {}", e);
                    continue;
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
    });
    info!("Gossip protocol started (interval=15s, fanout=3)");
}

/// Send a gossip message to a single peer over a STOQ stream.
async fn send_gossip_to_peer(
    node: &blockmatrix::network::NetworkNode,
    msg_bytes: &[u8],
) {
    let conn = match node.connection.as_ref() {
        Some(c) => c,
        None => return,
    };

    let short_id = &node.node_id[..8.min(node.node_id.len())];

    match conn.open_bi().await {
        Ok((mut send, _recv)) => {
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
        Err(e) => {
            debug!("Gossip stream open to {} failed: {}", short_id, e);
        }
    }
}
