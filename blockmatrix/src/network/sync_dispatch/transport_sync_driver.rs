// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! `TransportSyncDriver` — runs sync rounds over real STOQ connections,
//! plus peer query helpers.

use std::collections::HashMap;
use std::net::SocketAddr;
use tracing::{debug, info, warn};

use crate::blockchain::block::Block;
use crate::blockchain::node_chain::NodeBlockchain;
use crate::blockchain::sync_manager::{SyncManager, SyncMessage};
use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::reflector_pool::{Reflector, ReflectorPool};
use crate::network::stoq_integration::MatrixMessage;

/// Maximum number of reflectors to try before giving up on a network.
const MAX_REFLECTOR_RETRIES: usize = 3;

/// Drives sync operations over real STOQ connections.
///
/// For each network needing sync:
/// 1. Selects the best reflectors from the pool.
/// 2. Tries them in order (up to `MAX_REFLECTOR_RETRIES`).
/// 3. On success, updates reflector health positively.
/// 4. On failure, updates reflector health negatively and tries next.
/// 5. Returns fetched blocks for the caller to insert.
pub struct TransportSyncDriver;

impl TransportSyncDriver {
    /// Run one sync round for all networks that need syncing.
    ///
    /// Returns blocks fetched from peers. The caller is responsible for
    /// inserting them into the blockchain and extracting DNS entries.
    ///
    /// `reflector_pool` is taken as `&mut` so that health scores can be
    /// updated based on sync outcomes.
    pub async fn run_sync_round(
        sync_manager: &mut SyncManager,
        reflector_pool: &mut ReflectorPool,
        blockchain: &NodeBlockchain,
        transport: &stoq::StoqTransport,
        node_map: &HashMap<String, (String, SocketAddr)>,
        local_coordinate: &MatrixCoordinate,
    ) -> Vec<Block> {
        let networks: Vec<String> = sync_manager
            .networks_needing_sync()
            .iter()
            .map(|s| s.to_string())
            .collect();

        let mut fetched_blocks = Vec::new();

        for network_id in &networks {
            let local_height = blockchain.get_height().await;

            let request = match sync_manager.generate_sync_request(network_id, local_height) {
                Some(r) => r,
                None => continue,
            };

            // Get multiple reflectors for retry
            let reflector_ids: Vec<(String, String)> = reflector_pool
                .get_best_reflectors(network_id, MAX_REFLECTOR_RETRIES)
                .iter()
                .map(|r| (r.node_id.clone(), network_id.clone()))
                .collect();

            if reflector_ids.is_empty() {
                debug!(network = %network_id, "No reflectors available for sync");
                continue;
            }

            let mut synced = false;
            for (ref_node_id, ref_net_id) in &reflector_ids {
                // Re-fetch the reflector from the pool each iteration
                // since health may have changed.
                let reflector = {
                    let candidates = reflector_pool.get_best_reflectors(ref_net_id, MAX_REFLECTOR_RETRIES);
                    match candidates.iter().find(|r| r.node_id == *ref_node_id).cloned() {
                        Some(r) => r,
                        None => continue,
                    }
                };

                let sync_result = Self::sync_from_reflector(
                    &request,
                    &reflector,
                    blockchain,
                    transport,
                    node_map,
                    local_coordinate,
                )
                .await;

                let reflector_label = reflector.node_id
                    [..8.min(reflector.node_id.len())]
                    .to_string();

                match sync_result {
                    Ok((blocks, peer_height)) => {
                        // Health feedback: success
                        reflector_pool.update_reflector_health(
                            ref_node_id,
                            ref_net_id,
                            true,
                        );

                        let response = SyncMessage::Response {
                            network_id: network_id.clone(),
                            block_hashes: Vec::new(),
                            peer_height,
                        };
                        sync_manager.process_sync_message(response);

                        if !blocks.is_empty() {
                            info!(
                                network = %network_id,
                                count = blocks.len(),
                                "Fetched blocks from reflector {reflector_label}",
                            );
                        }
                        fetched_blocks.extend(blocks);
                        synced = true;

                        // Feed sync results into engauge intelligence (when enabled)
                        Self::feed_intelligence_after_sync(network_id, peer_height);

                        break;
                    }
                    Err(e) => {
                        // Health feedback: failure
                        reflector_pool.update_reflector_health(
                            ref_node_id,
                            ref_net_id,
                            false,
                        );

                        debug!(
                            network = %network_id,
                            reflector = %ref_node_id,
                            error = %e,
                            "Sync failed, trying next reflector",
                        );
                    }
                }
            }

            if !synced {
                warn!(
                    network = %network_id,
                    tried = reflector_ids.len(),
                    "All reflectors failed for sync round",
                );
            }
        }

        fetched_blocks
    }

    /// Feed post-sync intelligence data into the engauge bridge.
    ///
    /// After a successful sync round, notifies the intelligence layer so that
    /// swarm analytics can be updated and replication triggers checked. Actual
    /// shard creation for replication signals is future work — for now we log
    /// the signals.
    fn feed_intelligence_after_sync(network_id: &str, peer_height: u64) {
        // Log sync completion for intelligence pipeline observability
        debug!(
            network = %network_id,
            peer_height = peer_height,
            "Intelligence feedback: sync round completed"
        );

        // When the intelligence feature is enabled, the caller's EngaugeBridge
        // periodic feeder picks up demand data automatically. Here we log that
        // the sync round is a good time for the caller to invoke
        // `check_replication_needs()` on the IntelligenceLayer.
        //
        // Actual replication trigger checking requires access to the shared
        // SwarmAnalytics (owned by EngaugeBridge), which is wired at the node
        // level, not inside TransportSyncDriver. The node's run loop should
        // call `intelligence_layer.check_replication_needs()` after each
        // `run_sync_round()` returns.
    }

    /// Perform sync exchange with a single reflector.
    ///
    /// Returns `(fetched_blocks, peer_height)`.
    async fn sync_from_reflector(
        request: &SyncMessage,
        reflector: &Reflector,
        blockchain: &NodeBlockchain,
        transport: &stoq::StoqTransport,
        node_map: &HashMap<String, (String, SocketAddr)>,
        _local_coordinate: &MatrixCoordinate,
    ) -> anyhow::Result<(Vec<Block>, u64)> {
        let addr = resolve_reflector_addr(reflector, node_map)?;
        let connection = connect_to_peer(transport, addr).await?;

        // Step 1: Send SyncRequest, get SyncResponse
        let (block_hashes, peer_height) =
            send_sync_request(&connection, request).await?;

        // Step 2: Filter to hashes we don't have
        let missing: Vec<String> = filter_missing_hashes(&block_hashes, blockchain).await;

        if missing.is_empty() {
            return Ok((Vec::new(), peer_height));
        }

        // Step 3: Fetch missing blocks
        let blocks = fetch_blocks(&connection, missing).await?;

        Ok((blocks, peer_height))
    }
}

/// Resolve a reflector's node_id to a SocketAddr via the node_map.
pub(super) fn resolve_reflector_addr(
    reflector: &Reflector,
    node_map: &HashMap<String, (String, SocketAddr)>,
) -> anyhow::Result<SocketAddr> {
    for (_key, (node_id, addr)) in node_map {
        if *node_id == reflector.node_id {
            return Ok(*addr);
        }
    }
    Err(anyhow::anyhow!(
        "Reflector {} not found in node map",
        &reflector.node_id[..8.min(reflector.node_id.len())],
    ))
}

/// Establish a STOQ connection to a peer address.
async fn connect_to_peer(
    transport: &stoq::StoqTransport,
    addr: SocketAddr,
) -> anyhow::Result<std::sync::Arc<stoq::Connection>> {
    let ipv6 = match addr {
        SocketAddr::V6(v6) => *v6.ip(),
        SocketAddr::V4(v4) => v4.ip().to_ipv6_mapped(),
    };
    let endpoint = stoq::Endpoint::new(ipv6, addr.port());
    transport.connect(&endpoint).await
}

/// Open a stream, send a SyncRequest, and read the SyncResponse.
///
/// Returns `(block_hashes, peer_height)` from the response.
async fn send_sync_request(
    connection: &stoq::Connection,
    request: &SyncMessage,
) -> anyhow::Result<(Vec<String>, u64)> {
    let matrix_msg = match request {
        SyncMessage::Request {
            network_id,
            from_height,
            max_blocks,
        } => MatrixMessage::SyncRequest {
            network_id: network_id.clone(),
            from_height: *from_height,
            max_blocks: *max_blocks,
        },
        _ => return Err(anyhow::anyhow!("Expected SyncMessage::Request")),
    };

    let payload = serde_json::to_vec(&matrix_msg)?;

    let mut stream = connection.open_stream().await?;
    stream
        .write_discriminator(crate::network::CONN_TYPE_PEER_MESSAGE)
        .await?;

    // Write tag + payload as a single message
    let mut tagged = Vec::with_capacity(1 + payload.len());
    tagged.push(crate::network::message_handlers::TAG_SYNC_MESSAGE);
    tagged.extend_from_slice(&payload);
    stream.send(&tagged).await?;

    // send() closes our write half. The peer reads our message via
    // read_to_end (which completes once our write side is finished),
    // processes it, and writes a reply via send() (closing their write
    // half). We read their reply via receive() on our still-open recv half.
    let response_data = stream.receive().await?;

    // Response has TAG_SYNC_MESSAGE prefix
    if response_data.is_empty() {
        return Err(anyhow::anyhow!("Empty sync response"));
    }
    if response_data[0] != crate::network::message_handlers::TAG_SYNC_MESSAGE {
        return Err(anyhow::anyhow!(
            "Unexpected response tag: 0x{:02x}",
            response_data[0],
        ));
    }

    let response_msg: MatrixMessage = serde_json::from_slice(&response_data[1..])?;
    match response_msg {
        MatrixMessage::SyncResponse {
            block_hashes,
            peer_height,
            ..
        } => Ok((block_hashes, peer_height)),
        other => Err(anyhow::anyhow!(
            "Expected SyncResponse, got {:?}",
            std::mem::discriminant(&other),
        )),
    }
}

/// Filter block hashes to only those missing from the local blockchain.
pub(super) async fn filter_missing_hashes(
    hashes: &[String],
    blockchain: &NodeBlockchain,
) -> Vec<String> {
    let mut missing = Vec::new();
    for hash in hashes {
        if !blockchain.has_block(hash).await {
            missing.push(hash.clone());
        }
    }
    missing
}

/// Open a new stream and fetch blocks by hash from the peer.
async fn fetch_blocks(
    connection: &stoq::Connection,
    block_hashes: Vec<String>,
) -> anyhow::Result<Vec<Block>> {
    let request = MatrixMessage::BlockFetchRequest { block_hashes };
    let payload = serde_json::to_vec(&request)?;

    let mut stream = connection.open_stream().await?;
    stream
        .write_discriminator(crate::network::CONN_TYPE_PEER_MESSAGE)
        .await?;

    let mut tagged = Vec::with_capacity(1 + payload.len());
    tagged.push(crate::network::message_handlers::TAG_BLOCK_FETCH_REQUEST);
    tagged.extend_from_slice(&payload);
    stream.send(&tagged).await?;

    let response_data = stream.receive().await?;
    if response_data.is_empty() {
        return Ok(Vec::new());
    }

    let response_msg: MatrixMessage = serde_json::from_slice(&response_data)?;
    match response_msg {
        MatrixMessage::BlockFetchResponse { blocks } => {
            let mut result = Vec::with_capacity(blocks.len());
            for block_json in &blocks {
                match serde_json::from_str::<Block>(block_json) {
                    Ok(block) => {
                        if block.verify_hash() {
                            result.push(block);
                        } else {
                            warn!("Fetched block failed hash verification, discarding");
                        }
                    }
                    Err(e) => {
                        debug!("Failed to deserialize fetched block: {}", e);
                    }
                }
            }
            Ok(result)
        }
        other => Err(anyhow::anyhow!(
            "Expected BlockFetchResponse, got {:?}",
            std::mem::discriminant(&other),
        )),
    }
}
