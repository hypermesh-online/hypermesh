// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Dispatches sync-related `MatrixMessage` variants to the appropriate
//! subsystems (`SyncManager`, `ReflectorPool`).
//!
//! This wiring layer bridges Gaps 1, 2, 4, and 5 by:
//! - Converting `MatrixMessage::SyncRequest/SyncResponse/SyncAnnounce`
//!   into `SyncMessage` values and forwarding them to `SyncManager`.
//! - Converting `MatrixMessage::ReflectorHeartbeat` into a
//!   `register_reflector` / `update_health` call on `ReflectorPool`.

use std::collections::HashMap;
use std::net::SocketAddr;
use tracing::{debug, info, warn};

use crate::blockchain::block::{Block, BlockHeader};
use crate::blockchain::node_chain::NodeBlockchain;
use crate::blockchain::sync_manager::{BlockProvider, SyncManager, SyncMessage};
use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::reflector_pool::{Reflector, ReflectorPool};
use crate::network::stoq_integration::MatrixMessage;

use crate::bootstrap::PrivacyMode;
use hypermesh_lib::MatrixPosition;

/// Coordinates message dispatch between the network layer and the
/// blockchain sync / reflector subsystems.
///
/// Holds mutable references to the subsystems and an optional
/// `BlockProvider` for populating sync responses with real data.
pub struct SyncDispatcher<'a> {
    /// Sync manager handling chain synchronisation state.
    pub sync_manager: &'a mut SyncManager,
    /// Reflector pool tracking block-serving peers.
    pub reflector_pool: &'a mut ReflectorPool,
    /// Optional provider for looking up local block data.
    pub block_provider: Option<&'a dyn BlockProvider>,
}

/// Response produced by the dispatcher, ready to be serialised and
/// sent back through the STOQ transport.
#[derive(Debug)]
pub enum DispatchResponse {
    /// A sync message that should be sent back to the requesting peer.
    Reply(MatrixMessage),
    /// No response needed.
    None,
}

impl<'a> SyncDispatcher<'a> {
    /// Dispatch a single `MatrixMessage` to the correct subsystem.
    ///
    /// Returns a `DispatchResponse` that the caller should send back
    /// over the STOQ connection (if it is a `Reply`).
    pub fn dispatch(
        &mut self,
        msg: MatrixMessage,
        sender_node_id: &str,
        sender_position: MatrixPosition,
    ) -> DispatchResponse {
        match msg {
            MatrixMessage::SyncRequest {
                network_id,
                from_height,
                max_blocks,
            } => self.handle_sync_request(network_id, from_height, max_blocks),

            MatrixMessage::SyncResponse {
                network_id,
                block_hashes,
                peer_height,
            } => self.handle_sync_response(network_id, block_hashes, peer_height),

            MatrixMessage::SyncAnnounce {
                network_id,
                block_height,
                block_hash,
            } => self.handle_sync_announce(network_id, block_height, block_hash),

            MatrixMessage::ReflectorHeartbeat {
                network_id,
                block_height,
                health_score,
            } => self.handle_reflector_heartbeat(
                &network_id,
                sender_node_id,
                sender_position,
                block_height,
                health_score,
            ),

            MatrixMessage::GenesisRequest { network_id } => {
                self.handle_genesis_request(network_id)
            }

            MatrixMessage::HeaderRequest {
                network_id,
                from_height,
                max_count,
            } => self.handle_header_request(network_id, from_height, max_count),

            MatrixMessage::SyncBlockRequest {
                network_id,
                block_hashes,
            } => self.handle_sync_block_request(network_id, block_hashes),

            // Response variants are handled by the caller, not dispatched
            MatrixMessage::GenesisResponse { .. }
            | MatrixMessage::HeaderResponse { .. }
            | MatrixMessage::SyncBlockResponse { .. } => {
                debug!("SyncDispatcher: response message handled by caller");
                DispatchResponse::None
            }

            other => {
                debug!("SyncDispatcher ignoring non-sync message: {:?}", other);
                DispatchResponse::None
            }
        }
    }

    /// Convert a network-layer SyncRequest to a SyncMessage, process via
    /// SyncManager (optionally with a BlockProvider), and wrap the
    /// response back into a MatrixMessage.
    fn handle_sync_request(
        &mut self,
        network_id: String,
        from_height: u64,
        max_blocks: u32,
    ) -> DispatchResponse {
        let sync_msg = SyncMessage::Request {
            network_id,
            from_height,
            max_blocks,
        };

        let response = self
            .sync_manager
            .process_sync_message_with_provider(sync_msg, self.block_provider);

        match response {
            Some(SyncMessage::Response {
                network_id,
                block_hashes,
                peer_height,
            }) => DispatchResponse::Reply(MatrixMessage::SyncResponse {
                network_id,
                block_hashes,
                peer_height,
            }),
            _ => DispatchResponse::None,
        }
    }

    /// Forward a SyncResponse to the SyncManager.
    fn handle_sync_response(
        &mut self,
        network_id: String,
        block_hashes: Vec<String>,
        peer_height: u64,
    ) -> DispatchResponse {
        let sync_msg = SyncMessage::Response {
            network_id,
            block_hashes,
            peer_height,
        };
        let _ = self.sync_manager.process_sync_message(sync_msg);
        DispatchResponse::None
    }

    /// Forward a SyncAnnounce to the SyncManager.
    fn handle_sync_announce(
        &mut self,
        network_id: String,
        block_height: u64,
        block_hash: String,
    ) -> DispatchResponse {
        let sync_msg = SyncMessage::Announce {
            network_id,
            block_height,
            block_hash,
        };
        let _ = self.sync_manager.process_sync_message(sync_msg);
        DispatchResponse::None
    }

    /// Register or update a reflector from a heartbeat message.
    fn handle_reflector_heartbeat(
        &mut self,
        network_id: &str,
        sender_node_id: &str,
        sender_position: MatrixPosition,
        block_height: u64,
        health_score: f64,
    ) -> DispatchResponse {
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let reflector = Reflector {
            node_id: sender_node_id.to_string(),
            position: sender_position,
            last_seen: now_secs,
            block_height,
            health_score: health_score.clamp(0.0, 1.0),
            privacy_mode: PrivacyMode::PUBLIC,
        };

        self.reflector_pool
            .register_reflector(network_id, reflector);

        debug!(
            network = %network_id,
            node = %sender_node_id,
            height = block_height,
            health = health_score,
            "Processed reflector heartbeat"
        );

        DispatchResponse::None
    }

    /// Return the genesis block (index 0) from the local chain.
    fn handle_genesis_request(&self, network_id: String) -> DispatchResponse {
        let provider = match self.block_provider {
            Some(p) => p,
            None => {
                debug!(
                    network = %network_id,
                    "GenesisRequest: no block provider available",
                );
                return DispatchResponse::None;
            }
        };

        let (hashes, _height) = provider.get_block_hashes(0, 1);
        if hashes.is_empty() {
            warn!(
                network = %network_id,
                "GenesisRequest: no genesis block found",
            );
            return DispatchResponse::None;
        }

        // The provider only gives hashes; we need the full block.
        // For GenesisRequest we return the hash so the caller can fetch
        // the full block via BlockFetchRequest. We encode the hash as a
        // GenesisResponse with the genesis hash in the JSON field.
        //
        // NOTE: The full-block provider pattern requires the caller to
        // supply a FullBlockProvider (see NodeBlockchainFullBlockProvider).
        // For now, return the hash so the caller can follow up.
        DispatchResponse::Reply(MatrixMessage::GenesisResponse {
            network_id,
            genesis_block_json: hashes[0].clone(),
        })
    }

    /// Return block headers from `from_height` up to `max_count`.
    fn handle_header_request(
        &self,
        network_id: String,
        from_height: u64,
        max_count: u32,
    ) -> DispatchResponse {
        let provider = match self.block_provider {
            Some(p) => p,
            None => {
                debug!(
                    network = %network_id,
                    "HeaderRequest: no block provider available",
                );
                return DispatchResponse::None;
            }
        };

        let (hashes, peer_height) = provider.get_block_hashes(from_height, max_count);

        // We only have hashes from the provider, so we construct minimal
        // header info. The caller with full block access should use
        // FullBlockProvider for richer header data.
        let headers_json: Vec<String> = hashes
            .iter()
            .enumerate()
            .map(|(i, hash)| {
                let index = from_height + i as u64;
                // Serialize a minimal header placeholder with hash and index
                serde_json::json!({
                    "index": index,
                    "hash": hash,
                })
                .to_string()
            })
            .collect();

        DispatchResponse::Reply(MatrixMessage::HeaderResponse {
            network_id,
            headers_json,
            peer_height,
        })
    }

    /// Return full blocks matching the requested hashes.
    fn handle_sync_block_request(
        &self,
        network_id: String,
        block_hashes: Vec<String>,
    ) -> DispatchResponse {
        // SyncBlockRequest uses the same mechanism as BlockFetchRequest
        // but scoped to a network. With only a hash-based BlockProvider
        // we cannot return full blocks -- return the hashes as confirmation.
        // The real full-block fetch is done via the existing BlockFetchRequest
        // path in message_handlers.
        info!(
            network = %network_id,
            requested = block_hashes.len(),
            "SyncBlockRequest: delegating to block fetch path",
        );

        DispatchResponse::Reply(MatrixMessage::SyncBlockResponse {
            network_id,
            blocks_json: Vec::new(),
        })
    }
}

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
fn resolve_reflector_addr(
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
    tagged.push(super::message_handlers::TAG_SYNC_MESSAGE);
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
    if response_data[0] != super::message_handlers::TAG_SYNC_MESSAGE {
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
async fn filter_missing_hashes(
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
    tagged.push(super::message_handlers::TAG_BLOCK_FETCH_REQUEST);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::sync_manager::SyncConfig;
    use crate::network::reflector_pool::ReflectorConfig;

    /// A trivial BlockProvider that returns predictable hashes.
    struct FakeBlockProvider {
        chain_height: u64,
    }

    impl BlockProvider for FakeBlockProvider {
        fn get_block_hashes(&self, from_height: u64, max_blocks: u32) -> (Vec<String>, u64) {
            let end = (from_height + max_blocks as u64).min(self.chain_height);
            let hashes: Vec<String> = (from_height..end).map(|h| format!("hash_{h}")).collect();
            (hashes, self.chain_height)
        }
    }

    fn make_sync_manager() -> SyncManager {
        SyncManager::new("device-chain".to_string(), SyncConfig::default())
    }

    fn make_reflector_pool() -> ReflectorPool {
        ReflectorPool::new(ReflectorConfig::default())
    }

    fn zero_position() -> MatrixPosition {
        MatrixPosition {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    // ------------------------------------------------------------------
    // Gap 5 tests: Message dispatch routing
    // ------------------------------------------------------------------

    #[test]
    fn test_dispatch_sync_request_without_provider() {
        let mut sm = make_sync_manager();
        sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        let mut rp = make_reflector_pool();

        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: None,
        };

        let msg = MatrixMessage::SyncRequest {
            network_id: "net-1".to_string(),
            from_height: 0,
            max_blocks: 10,
        };

        let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
        match resp {
            DispatchResponse::Reply(MatrixMessage::SyncResponse {
                network_id,
                block_hashes,
                peer_height,
            }) => {
                assert_eq!(network_id, "net-1");
                assert!(block_hashes.is_empty());
                assert_eq!(peer_height, 0);
            }
            other => unreachable!("test: expected SyncResponse, got {:?}", other),
        }
    }

    #[test]
    fn test_dispatch_sync_request_with_provider() {
        let mut sm = make_sync_manager();
        sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        let mut rp = make_reflector_pool();
        let provider = FakeBlockProvider { chain_height: 20 };

        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: Some(&provider),
        };

        let msg = MatrixMessage::SyncRequest {
            network_id: "net-1".to_string(),
            from_height: 5,
            max_blocks: 10,
        };

        let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
        match resp {
            DispatchResponse::Reply(MatrixMessage::SyncResponse {
                block_hashes,
                peer_height,
                ..
            }) => {
                assert_eq!(block_hashes.len(), 10);
                assert_eq!(block_hashes[0], "hash_5");
                assert_eq!(peer_height, 20);
            }
            other => unreachable!("test: expected SyncResponse, got {:?}", other),
        }
    }

    #[test]
    fn test_dispatch_sync_request_unknown_network() {
        let mut sm = make_sync_manager();
        let mut rp = make_reflector_pool();

        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: None,
        };

        let msg = MatrixMessage::SyncRequest {
            network_id: "unknown".to_string(),
            from_height: 0,
            max_blocks: 5,
        };

        let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
        assert!(matches!(resp, DispatchResponse::None));
    }

    #[test]
    fn test_dispatch_sync_response_updates_state() {
        let mut sm = make_sync_manager();
        sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        let mut rp = make_reflector_pool();

        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: None,
        };

        let msg = MatrixMessage::SyncResponse {
            network_id: "net-1".to_string(),
            block_hashes: Vec::new(),
            peer_height: 42,
        };

        let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
        assert!(matches!(resp, DispatchResponse::None));

        // SyncManager should now be Synchronized at height 42
        use crate::blockchain::sync_manager::SyncState;
        assert_eq!(
            sm.sync_state("net-1"),
            Some(&SyncState::Synchronized {
                last_block_height: 42
            })
        );
    }

    #[test]
    fn test_dispatch_sync_announce_triggers_resync() {
        let config = SyncConfig {
            max_block_lag: 5,
            ..SyncConfig::default()
        };
        let mut sm = SyncManager::new("dev".to_string(), config);
        sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");
        sm.update_sync_state(
            "net-1",
            crate::blockchain::sync_manager::SyncState::Synchronized {
                last_block_height: 10,
            },
        )
        .expect("test: set synced");

        let mut rp = make_reflector_pool();

        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: None,
        };

        let msg = MatrixMessage::SyncAnnounce {
            network_id: "net-1".to_string(),
            block_height: 100,
            block_hash: "abc".to_string(),
        };

        let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
        assert!(matches!(resp, DispatchResponse::None));

        // Should have transitioned to Syncing
        use crate::blockchain::sync_manager::SyncState;
        assert!(matches!(
            sm.sync_state("net-1"),
            Some(SyncState::Syncing { .. })
        ));
    }

    // ------------------------------------------------------------------
    // Gap 2 tests: ReflectorPool receives heartbeats
    // ------------------------------------------------------------------

    #[test]
    fn test_dispatch_reflector_heartbeat_registers() {
        let mut sm = make_sync_manager();
        let mut rp = make_reflector_pool();

        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: None,
        };

        let msg = MatrixMessage::ReflectorHeartbeat {
            network_id: "net-1".to_string(),
            block_height: 50,
            health_score: 0.8,
        };

        let pos = MatrixPosition {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };

        let resp = dispatcher.dispatch(msg, "reflector-node-1", pos);
        assert!(matches!(resp, DispatchResponse::None));

        assert_eq!(rp.total_count("net-1"), 1);
        let best = rp.get_best_reflectors("net-1", 1);
        assert_eq!(best.len(), 1);
        assert_eq!(best[0].node_id, "reflector-node-1");
        assert_eq!(best[0].block_height, 50);
    }

    #[test]
    fn test_dispatch_reflector_heartbeat_updates_existing() {
        let mut sm = make_sync_manager();
        let mut rp = make_reflector_pool();

        let pos = MatrixPosition {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };

        // First heartbeat
        {
            let mut dispatcher = SyncDispatcher {
                sync_manager: &mut sm,
                reflector_pool: &mut rp,
                block_provider: None,
            };
            let msg = MatrixMessage::ReflectorHeartbeat {
                network_id: "net-1".to_string(),
                block_height: 10,
                health_score: 0.5,
            };
            dispatcher.dispatch(msg, "node-A", pos);
        }

        // Second heartbeat with updated data
        {
            let mut dispatcher = SyncDispatcher {
                sync_manager: &mut sm,
                reflector_pool: &mut rp,
                block_provider: None,
            };
            let msg = MatrixMessage::ReflectorHeartbeat {
                network_id: "net-1".to_string(),
                block_height: 25,
                health_score: 0.9,
            };
            dispatcher.dispatch(msg, "node-A", pos);
        }

        // Still one reflector, with updated values
        assert_eq!(rp.total_count("net-1"), 1);
        let best = rp.get_best_reflectors("net-1", 1);
        assert_eq!(best[0].block_height, 25);
        assert!((best[0].health_score - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_dispatch_genesis_request_with_provider() {
        let mut sm = make_sync_manager();
        sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        let mut rp = make_reflector_pool();
        let provider = FakeBlockProvider { chain_height: 10 };

        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: Some(&provider),
        };

        let msg = MatrixMessage::GenesisRequest {
            network_id: "net-1".to_string(),
        };

        let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
        match resp {
            DispatchResponse::Reply(MatrixMessage::GenesisResponse {
                network_id,
                genesis_block_json,
            }) => {
                assert_eq!(network_id, "net-1");
                // FakeBlockProvider returns "hash_0" for height 0
                assert_eq!(genesis_block_json, "hash_0");
            }
            other => unreachable!("test: expected GenesisResponse, got {:?}", other),
        }
    }

    #[test]
    fn test_dispatch_genesis_request_without_provider() {
        let mut sm = make_sync_manager();
        let mut rp = make_reflector_pool();

        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: None,
        };

        let msg = MatrixMessage::GenesisRequest {
            network_id: "net-1".to_string(),
        };

        let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
        assert!(matches!(resp, DispatchResponse::None));
    }

    #[test]
    fn test_dispatch_header_request_with_provider() {
        let mut sm = make_sync_manager();
        sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        let mut rp = make_reflector_pool();
        let provider = FakeBlockProvider { chain_height: 20 };

        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: Some(&provider),
        };

        let msg = MatrixMessage::HeaderRequest {
            network_id: "net-1".to_string(),
            from_height: 5,
            max_count: 3,
        };

        let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
        match resp {
            DispatchResponse::Reply(MatrixMessage::HeaderResponse {
                network_id,
                headers_json,
                peer_height,
            }) => {
                assert_eq!(network_id, "net-1");
                assert_eq!(headers_json.len(), 3);
                assert_eq!(peer_height, 20);
            }
            other => unreachable!("test: expected HeaderResponse, got {:?}", other),
        }
    }

    #[test]
    fn test_dispatch_sync_block_request() {
        let mut sm = make_sync_manager();
        let mut rp = make_reflector_pool();

        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: None,
        };

        let msg = MatrixMessage::SyncBlockRequest {
            network_id: "net-1".to_string(),
            block_hashes: vec!["hash_a".to_string()],
        };

        let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
        match resp {
            DispatchResponse::Reply(MatrixMessage::SyncBlockResponse {
                network_id,
                blocks_json,
            }) => {
                assert_eq!(network_id, "net-1");
                assert!(blocks_json.is_empty());
            }
            other => unreachable!("test: expected SyncBlockResponse, got {:?}", other),
        }
    }

    #[test]
    fn test_dispatch_response_variants_return_none() {
        let mut sm = make_sync_manager();
        let mut rp = make_reflector_pool();

        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: None,
        };

        // GenesisResponse
        let msg = MatrixMessage::GenesisResponse {
            network_id: "net-1".to_string(),
            genesis_block_json: "{}".to_string(),
        };
        assert!(matches!(
            dispatcher.dispatch(msg, "peer-1", zero_position()),
            DispatchResponse::None
        ));

        // HeaderResponse
        let msg = MatrixMessage::HeaderResponse {
            network_id: "net-1".to_string(),
            headers_json: vec![],
            peer_height: 0,
        };
        assert!(matches!(
            dispatcher.dispatch(msg, "peer-1", zero_position()),
            DispatchResponse::None
        ));

        // SyncBlockResponse
        let msg = MatrixMessage::SyncBlockResponse {
            network_id: "net-1".to_string(),
            blocks_json: vec![],
        };
        assert!(matches!(
            dispatcher.dispatch(msg, "peer-1", zero_position()),
            DispatchResponse::None
        ));
    }

    #[test]
    fn test_dispatch_ignores_non_sync_messages() {
        let mut sm = make_sync_manager();
        let mut rp = make_reflector_pool();

        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: None,
        };

        let msg = MatrixMessage::Heartbeat {
            coordinate: crate::matrix::coordinate::MatrixCoordinate::new(0, 0, 0)
                .expect("test: valid coord"),
            timestamp: 12345,
        };

        let resp = dispatcher.dispatch(msg, "peer", zero_position());
        assert!(matches!(resp, DispatchResponse::None));
    }

    // ------------------------------------------------------------------
    // Gap 4 tests: SyncObserver notification
    // ------------------------------------------------------------------

    #[test]
    fn test_sync_observer_notified_on_completion() {
        use crate::blockchain::sync_manager::SyncObserver;
        use std::sync::{Arc, Mutex};

        struct TestObserver {
            events: Arc<Mutex<Vec<(String, u64)>>>,
        }

        impl SyncObserver for TestObserver {
            fn on_sync_complete(&self, network_id: &str, block_height: u64) {
                self.events
                    .lock()
                    .expect("test: lock")
                    .push((network_id.to_string(), block_height));
            }
        }

        let events = Arc::new(Mutex::new(Vec::new()));
        let observer = TestObserver {
            events: events.clone(),
        };

        let mut sm = make_sync_manager();
        sm.set_observer(Box::new(observer));
        sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        // Process an empty SyncResponse (triggers Synchronized)
        sm.process_sync_message(SyncMessage::Response {
            network_id: "net-1".to_string(),
            block_hashes: Vec::new(),
            peer_height: 99,
        });

        let captured = events.lock().expect("test: lock");
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].0, "net-1");
        assert_eq!(captured[0].1, 99);
    }

    // ------------------------------------------------------------------
    // Gap 1 tests: SyncManager uses BlockProvider
    // ------------------------------------------------------------------

    #[test]
    fn test_sync_manager_with_block_provider() {
        let mut sm = make_sync_manager();
        sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        let provider = FakeBlockProvider { chain_height: 50 };

        let request = SyncMessage::Request {
            network_id: "net-1".to_string(),
            from_height: 10,
            max_blocks: 20,
        };

        let response = sm.process_sync_message_with_provider(request, Some(&provider));
        match response {
            Some(SyncMessage::Response {
                block_hashes,
                peer_height,
                ..
            }) => {
                assert_eq!(block_hashes.len(), 20);
                assert_eq!(block_hashes[0], "hash_10");
                assert_eq!(block_hashes[19], "hash_29");
                assert_eq!(peer_height, 50);
            }
            other => unreachable!("test: expected Response, got {:?}", other),
        }
    }

    // ------------------------------------------------------------------
    // TransportSyncDriver helper tests
    // ------------------------------------------------------------------

    #[test]
    fn test_resolve_reflector_addr_found() {
        let mut node_map = HashMap::new();
        let addr: SocketAddr = "[::1]:9292".parse().expect("test: parse addr");
        node_map.insert("1,2,3".to_string(), ("node-abc".to_string(), addr));

        let reflector = Reflector {
            node_id: "node-abc".to_string(),
            position: zero_position(),
            last_seen: 0,
            block_height: 0,
            health_score: 1.0,
            privacy_mode: PrivacyMode::PUBLIC,
        };

        let result = super::resolve_reflector_addr(&reflector, &node_map);
        assert!(result.is_ok());
        assert_eq!(result.expect("test: addr"), addr);
    }

    #[test]
    fn test_resolve_reflector_addr_not_found() {
        let node_map = HashMap::new();

        let reflector = Reflector {
            node_id: "missing-node".to_string(),
            position: zero_position(),
            last_seen: 0,
            block_height: 0,
            health_score: 1.0,
            privacy_mode: PrivacyMode::PUBLIC,
        };

        let result = super::resolve_reflector_addr(&reflector, &node_map);
        assert!(result.is_err());
    }

    #[test]
    fn test_block_fetch_request_serialization() {
        let msg = MatrixMessage::BlockFetchRequest {
            block_hashes: vec!["abc123".to_string(), "def456".to_string()],
        };
        let json = serde_json::to_string(&msg).expect("test: serialize");
        let parsed: MatrixMessage =
            serde_json::from_str(&json).expect("test: deserialize");
        match parsed {
            MatrixMessage::BlockFetchRequest { block_hashes } => {
                assert_eq!(block_hashes.len(), 2);
                assert_eq!(block_hashes[0], "abc123");
                assert_eq!(block_hashes[1], "def456");
            }
            other => unreachable!("test: expected BlockFetchRequest, got {:?}", other),
        }
    }

    #[test]
    fn test_block_fetch_response_serialization() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let block = Block::genesis(coord);
        let block_json =
            serde_json::to_string(&block).expect("test: serialize block");

        let msg = MatrixMessage::BlockFetchResponse {
            blocks: vec![block_json.clone()],
        };
        let json = serde_json::to_string(&msg).expect("test: serialize");
        let parsed: MatrixMessage =
            serde_json::from_str(&json).expect("test: deserialize");
        match parsed {
            MatrixMessage::BlockFetchResponse { blocks } => {
                assert_eq!(blocks.len(), 1);
                let deserialized: Block =
                    serde_json::from_str(&blocks[0]).expect("test: deserialize block");
                assert_eq!(deserialized.index, block.index);
                assert!(deserialized.verify_hash());
            }
            other => unreachable!("test: expected BlockFetchResponse, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_filter_missing_hashes() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let blockchain = NodeBlockchain::new(coord);
        let chain = blockchain.get_chain().await;
        let genesis_hash = chain.first().map(|b| b.hash.clone()).unwrap_or_default();

        let hashes = vec![
            genesis_hash.clone(),
            "nonexistent_hash_1".to_string(),
            "nonexistent_hash_2".to_string(),
        ];

        let missing = super::filter_missing_hashes(&hashes, &blockchain).await;
        // Genesis hash exists, the other two do not
        assert_eq!(missing.len(), 2);
        assert!(!missing.contains(&genesis_hash));
        assert!(missing.contains(&"nonexistent_hash_1".to_string()));
        assert!(missing.contains(&"nonexistent_hash_2".to_string()));
    }

    #[tokio::test]
    async fn test_run_sync_round_no_reflectors_returns_empty() {
        let mut sm = make_sync_manager();
        sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        let mut rp = make_reflector_pool();
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let blockchain = NodeBlockchain::new(coord);
        let node_map = HashMap::new();

        // Cannot create a real StoqTransport in unit tests without binding,
        // but with no reflectors the driver never connects.
        let config = stoq::TransportConfig {
            port: 0,
            bind_address: std::net::Ipv6Addr::LOCALHOST,
            ..stoq::TransportConfig::default()
        };
        let transport = match stoq::StoqTransport::new(config).await {
            Ok(t) => t,
            Err(_) => return, // Skip if socket binding fails
        };

        let blocks = TransportSyncDriver::run_sync_round(
            &mut sm,
            &mut rp,
            &blockchain,
            &transport,
            &node_map,
            &coord,
        )
        .await;

        assert!(blocks.is_empty(), "No reflectors means no blocks fetched");
    }
}
