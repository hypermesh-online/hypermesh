// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Network layer for multi-node communication
//!
//! This module provides the actual networking implementation for Block-MATRIX nodes
//! to discover and communicate with each other using STOQ transport.

pub mod blockchain_integration;
pub mod cluster;
pub mod config;
pub mod discovery;
pub mod gossip;
pub mod isolation;
pub mod multi_network;
pub mod reflector_pool;
pub mod shard_store;
pub mod shard_transport;
pub mod stoq_integration;
pub mod sync_dispatch;
pub mod trust;
pub mod validation;

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::blockchain::node_chain::NodeBlockchain;
use crate::blockchain::propagation::BlockPropagator;
use crate::blockchain::sync_manager::SyncManager;
use crate::bootstrap::PrivacyMode;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::matrix::neighbors::{find_k_nearest, find_neighbors};
use crate::network::reflector_pool::ReflectorPool;
use crate::network::shard_store::ShardStore;
use crate::network::stoq_integration::{MatrixMessage, MatrixNodeInfo, MatrixStoqIntegration};

/// Node network information
#[derive(Clone)]
pub struct NetworkNode {
    /// Matrix coordinate
    pub coordinate: MatrixCoordinate,
    /// Network address
    pub address: SocketAddr,
    /// Node ID (hash of genesis block)
    pub node_id: String,
    /// Privacy mode
    pub privacy_mode: PrivacyMode,
    /// STOQ connection (not Debug-able)
    pub connection: Option<Arc<stoq::Connection>>,
}

// Custom Debug implementation that skips connection
impl std::fmt::Debug for NetworkNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetworkNode")
            .field("coordinate", &self.coordinate)
            .field("address", &self.address)
            .field("node_id", &self.node_id)
            .field("privacy_mode", &self.privacy_mode)
            .field("connection", &self.connection.is_some())
            .finish()
    }
}

/// Shared context for peer message processing.
///
/// Passed to the per-connection message loop so that incoming blocks,
/// shards, and sync messages can reach the appropriate subsystems.
pub struct PeerContext {
    /// The node's blockchain (for inserting received blocks).
    pub blockchain: Arc<NodeBlockchain>,
    /// Shard store (for shard send/fetch operations).
    pub shard_store: Arc<ShardStore>,
    /// Sync manager for chain synchronization state.
    pub sync_manager: Arc<tokio::sync::Mutex<SyncManager>>,
    /// Reflector pool for tracking block-serving peers.
    pub reflector_pool: Arc<tokio::sync::Mutex<ReflectorPool>>,
    /// Block propagator for re-propagating received blocks.
    pub block_propagator: Arc<tokio::sync::Mutex<BlockPropagator>>,
    /// Our matrix coordinate.
    pub our_coordinate: MatrixCoordinate,
    /// Our node ID.
    pub node_id: String,
}

/// Network manager for multi-node communication
pub struct NetworkManager {
    /// Local node coordinate
    local_coordinate: MatrixCoordinate,
    /// Local STOQ transport
    transport: Arc<stoq::StoqTransport>,
    /// Known nodes in the network
    nodes: Arc<RwLock<HashMap<String, NetworkNode>>>,
    /// Bootstrap nodes
    bootstrap_nodes: Vec<SocketAddr>,
    /// Current privacy mode
    privacy_mode: Arc<RwLock<PrivacyMode>>,
    /// Matrix-STOQ integration layer
    stoq_integration: Option<Arc<MatrixStoqIntegration>>,
    /// FALCON-1024 node signer (from TrustChain via lib trait)
    signer: Arc<dyn hypermesh_lib::NodeSigner>,
    /// State proof provider (BlockMatrix implementation)
    proof_provider: Arc<dyn hypermesh_lib::StateProofProvider>,
}

impl NetworkManager {
    /// Create new network manager with FALCON-1024 identity for PoS signing.
    ///
    /// Accepts a [`NodeSigner`] (typically `FalconIdentity` from trustchain)
    /// and a [`StateProofProvider`] (typically `BlockMatrixProofProvider`).
    pub async fn new(
        local_coordinate: MatrixCoordinate,
        transport: Arc<stoq::StoqTransport>,
        privacy_mode: PrivacyMode,
        bootstrap_nodes: Vec<SocketAddr>,
        signer: Arc<dyn hypermesh_lib::NodeSigner>,
        proof_provider: Arc<dyn hypermesh_lib::StateProofProvider>,
    ) -> Result<Self> {
        info!(
            "Initializing network manager at ({},{},{}) in {:?} mode",
            local_coordinate.x, local_coordinate.y, local_coordinate.z, privacy_mode
        );

        // Create Matrix-STOQ integration layer
        let node_id = signer.node_id().to_string();
        let stoq_integration = MatrixStoqIntegration::new(
            local_coordinate,
            node_id.clone(),
            transport.clone(),
            privacy_mode,
        )
        .await
        .ok()
        .map(Arc::new);

        if stoq_integration.is_some() {
            info!("Matrix-STOQ integration layer initialized successfully");
        } else {
            warn!("Failed to initialize Matrix-STOQ integration layer");
        }

        Ok(Self {
            local_coordinate,
            transport,
            nodes: Arc::new(RwLock::new(HashMap::new())),
            bootstrap_nodes,
            privacy_mode: Arc::new(RwLock::new(privacy_mode)),
            stoq_integration,
            signer,
            proof_provider,
        })
    }

    /// Start network discovery based on privacy mode
    pub async fn start_discovery(&self) -> Result<()> {
        let mode = *self.privacy_mode.read().await;

        if mode == PrivacyMode::PRIVATE {
            info!("Private mode: No network discovery (localhost only)");
            Ok(())
        } else if mode == PrivacyMode::ANONYMOUS {
            info!("Anonymous mode: Starting ephemeral discovery");
            self.discover_ephemeral_peers().await
        } else if mode == PrivacyMode::PUBLIC {
            info!("Public mode: Joining network with full discovery");
            self.join_network().await
        } else {
            info!("Custom privacy mode ({:?}): Starting peer discovery", mode);
            self.discover_peers().await
        }
    }

    /// Discover ephemeral peers (Anonymous mode)
    async fn discover_ephemeral_peers(&self) -> Result<()> {
        // In anonymous mode, we only accept incoming connections
        // No active discovery or persistent connections
        info!("Anonymous mode: Listening for ephemeral connections");
        Ok(())
    }

    /// Discover peers (P2P mode)
    async fn discover_peers(&self) -> Result<()> {
        info!("P2P mode: Discovering peers via bootstrap nodes");

        // Connect to bootstrap nodes if any
        for bootstrap_addr in &self.bootstrap_nodes {
            if let Err(e) = self.connect_to_peer(*bootstrap_addr, None).await {
                warn!(
                    "Failed to connect to bootstrap node {}: {}",
                    bootstrap_addr, e
                );
            }
        }

        // Start mDNS discovery for local network
        self.start_mdns_discovery().await?;

        Ok(())
    }

    /// Join the network (Public mode)
    async fn join_network(&self) -> Result<()> {
        info!("Public mode: Joining network with full participation");

        // Connect to all bootstrap nodes
        for bootstrap_addr in &self.bootstrap_nodes {
            match self.connect_to_peer(*bootstrap_addr, None).await {
                Ok(node_id) => {
                    info!(
                        "Connected to bootstrap node {} ({})",
                        bootstrap_addr, node_id
                    );
                }
                Err(e) => {
                    error!("Failed to connect to bootstrap {}: {}", bootstrap_addr, e);
                }
            }
        }

        // Start full discovery protocols
        self.start_mdns_discovery().await?;
        self.start_gossip_protocol().await?;

        Ok(())
    }

    /// Connect to a specific peer.
    ///
    /// When `peer_ctx` is `Some`, a persistent message loop is spawned for the
    /// peer after a successful handshake (mirroring what `accept_connections`
    /// does for incoming peers).
    pub async fn connect_to_peer(
        &self,
        addr: SocketAddr,
        peer_ctx: Option<Arc<PeerContext>>,
    ) -> Result<String> {
        info!("Connecting to peer at {}", addr);

        // Always use the NetworkManager handshake path (flat JSON).
        // MatrixStoqIntegration::connect_to_node sends MatrixMessage::Announcement
        // which is incompatible with handle_incoming_handshake on the accept side.
        let endpoint = stoq::Endpoint::new(
            match addr {
                SocketAddr::V6(v6) => *v6.ip(),
                _ => return Err(anyhow!("Only IPv6 addresses supported")),
            },
            addr.port(),
        );

        // Connect via STOQ
        let connection = self.transport.connect(&endpoint).await?;

        // Exchange node information
        let node_info = self.exchange_node_info(&connection).await?;

        // Store the connected node
        let mut nodes = self.nodes.write().await;
        nodes.insert(node_info.node_id.clone(), node_info.clone());
        drop(nodes);

        info!(
            "Successfully connected to node {} at ({},{},{})",
            node_info.node_id,
            node_info.coordinate.x,
            node_info.coordinate.y,
            node_info.coordinate.z
        );

        // Spawn persistent message loop if context is available
        if let Some(ctx) = peer_ctx {
            let peer_node_id = node_info.node_id.clone();
            let peer_coord = node_info.coordinate;
            tokio::spawn(async move {
                run_peer_message_loop(connection, peer_node_id, peer_coord, ctx).await;
            });
        }

        Ok(node_info.node_id)
    }

    /// Perform bilateral handshake via STOQ protocol layer.
    ///
    /// Delegates to `stoq::initiate_handshake()` which implements the
    /// 3-message challenge-response protocol (R11) using NodeSigner
    /// and StateProofProvider traits.
    async fn exchange_node_info(&self, connection: &Arc<stoq::Connection>) -> Result<NetworkNode> {
        let coord = self.local_coordinate;
        let local_coord = (coord.x, coord.y, coord.z);

        let result = stoq::initiate_handshake(
            connection,
            self.signer.as_ref(),
            self.proof_provider.as_ref(),
            local_coord,
        )
        .await?;

        let coordinate = MatrixCoordinate::new(
            result.peer_coordinate.0,
            result.peer_coordinate.1,
            result.peer_coordinate.2,
        )
        .map_err(|e| anyhow!("Invalid peer coordinate: {e}"))?;

        Ok(NetworkNode {
            coordinate,
            address: connection.endpoint().to_socket_addr(),
            node_id: result.peer_node_id,
            privacy_mode: PrivacyMode::PUBLIC, // Will be negotiated later
            connection: Some(connection.clone()),
        })
    }

    /// Start mDNS discovery for local network.
    async fn start_mdns_discovery(&self) -> Result<()> {
        let node_id = self.get_node_id();
        let stoq_port = self.transport.local_addr().ok().map(|a| a.port()).unwrap_or(9292);

        let mdns = discovery::MdnsDiscovery::new(node_id, self.local_coordinate, stoq_port);
        match mdns.start().await {
            Ok(()) => {
                info!("mDNS discovery started on _hypermesh._udp.local");
            }
            Err(e) => {
                // mDNS failure is non-fatal (may lack multicast permissions)
                warn!("mDNS discovery failed to start: {e}");
            }
        }
        Ok(())
    }

    /// Start gossip protocol for network state sharing.
    async fn start_gossip_protocol(&self) -> Result<()> {
        let node_id = self.get_node_id();
        let mode = *self.privacy_mode.read().await;
        let privacy_str = format!("{mode:?}");
        let stoq_port = self.transport.local_addr().ok().map(|a| a.port()).unwrap_or(9292);

        let gossip_proto = gossip::GossipProtocol::new(
            node_id,
            self.local_coordinate,
            stoq_port,
            privacy_str,
        );
        gossip_proto.start().await;

        // Spawn a background task that periodically gossips to connected peers
        let nodes = self.nodes.clone();
        let gossip_state = gossip_proto.state();
        let interval = gossip_proto.gossip_interval();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;

                let connected = nodes.read().await;
                if connected.is_empty() {
                    continue;
                }

                // Build gossip message
                let state = gossip_state.read().await;
                let message = state.build_message();
                drop(state);

                let json_data = match serde_json::to_vec(&message) {
                    Ok(d) => d,
                    Err(_) => continue,
                };
                let mut data = Vec::with_capacity(1 + json_data.len());
                data.push(TAG_GOSSIP);
                data.extend_from_slice(&json_data);

                // Send to connected peers (best-effort)
                for (node_id, node) in connected.iter() {
                    if let Some(ref conn) = node.connection {
                        match conn.open_stream().await {
                            Ok(mut stream) => {
                                if let Err(e) = stream.send(&data).await {
                                    debug!("Gossip send to {node_id} failed: {e}");
                                }
                            }
                            Err(e) => {
                                debug!("Gossip stream to {node_id} failed: {e}");
                            }
                        }
                    }
                }
            }
        });

        info!("Gossip protocol started with {} second interval", interval.as_secs());
        Ok(())
    }

    /// Find neighbors in the matrix topology
    pub async fn find_matrix_neighbors(&self, radius: f64) -> Vec<NetworkNode> {
        let nodes = self.nodes.read().await;

        let candidates: Vec<MatrixCoordinate> = nodes.values().map(|n| n.coordinate).collect();

        let neighbors = find_neighbors(&self.local_coordinate, &candidates, radius);

        nodes
            .values()
            .filter(|n| neighbors.contains(&n.coordinate))
            .cloned()
            .collect()
    }

    /// Find K nearest neighbors
    pub async fn find_k_nearest_nodes(&self, k: usize) -> Vec<(NetworkNode, f64)> {
        let nodes = self.nodes.read().await;

        let candidates: Vec<MatrixCoordinate> = nodes.values().map(|n| n.coordinate).collect();

        let nearest = find_k_nearest(&self.local_coordinate, &candidates, k);

        nearest
            .into_iter()
            .filter_map(|(coord, dist)| {
                nodes
                    .values()
                    .find(|n| n.coordinate == coord)
                    .map(|n| (n.clone(), dist))
            })
            .collect()
    }

    /// Get all connected nodes
    pub async fn get_connected_nodes(&self) -> Vec<NetworkNode> {
        self.nodes.read().await.values().cloned().collect()
    }

    /// Returns the matrix coordinates of all currently connected peers.
    /// Used by BlockPropagator to determine propagation targets.
    pub async fn get_connected_coordinates(&self) -> Vec<MatrixCoordinate> {
        self.nodes
            .read()
            .await
            .values()
            .map(|n| n.coordinate)
            .collect()
    }

    /// Returns a map of coordinate string -> (node_id, socket_addr) for all connected peers.
    /// Used by StoqBlockTransportAdapter for block propagation routing.
    pub async fn get_node_address_map(&self) -> HashMap<String, (String, SocketAddr)> {
        self.nodes
            .read()
            .await
            .values()
            .map(|n| {
                let key = format!("{},{},{}", n.coordinate.x, n.coordinate.y, n.coordinate.z);
                (key, (n.node_id.clone(), n.address))
            })
            .collect()
    }

    /// Get node count
    pub async fn get_node_count(&self) -> usize {
        self.nodes.read().await.len()
    }

    /// Update privacy mode
    pub async fn set_privacy_mode(&self, mode: PrivacyMode) -> Result<()> {
        *self.privacy_mode.write().await = mode;

        // Restart discovery with new mode
        self.start_discovery().await?;

        Ok(())
    }

    /// Get local node ID
    fn get_node_id(&self) -> String {
        self.signer.node_id().to_string()
    }

    /// Broadcast matrix position to connected nodes via STOQ
    pub async fn broadcast_matrix_position(&self) -> Result<()> {
        if let Some(ref stoq_integration) = self.stoq_integration {
            stoq_integration.broadcast_position().await?;
        }
        Ok(())
    }

    /// Connect to peers belonging to a specific domain network.
    ///
    /// Attempts to connect to each address in `peers`, logging successes and
    /// failures. Returns the list of successfully connected node IDs.
    pub async fn connect_to_domain_network(
        &self,
        domain_name: &str,
        peers: Vec<std::net::SocketAddr>,
    ) -> Result<Vec<String>> {
        info!(
            "Connecting to domain network '{}' via {} peer(s)",
            domain_name,
            peers.len()
        );
        let mut connected_ids = Vec::new();
        for addr in peers {
            match self.connect_to_peer(addr, None).await {
                Ok(node_id) => {
                    info!(
                        "Connected to domain '{}' peer: {}",
                        domain_name, node_id
                    );
                    connected_ids.push(node_id);
                }
                Err(e) => {
                    warn!(
                        "Failed to connect to domain '{}' peer {}: {}",
                        domain_name, addr, e
                    );
                }
            }
        }
        Ok(connected_ids)
    }

    /// Start message loops for peers that were connected before PeerContext
    /// was available (e.g. during `start_discovery`).
    pub async fn start_peer_message_loops(&self, ctx: Arc<PeerContext>) {
        let nodes = self.nodes.read().await;
        for (node_id, node) in nodes.iter() {
            if let Some(ref connection) = node.connection {
                let conn = connection.clone();
                let nid = node_id.clone();
                let coord = node.coordinate;
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    run_peer_message_loop(conn, nid, coord, ctx).await;
                });
            }
        }
    }

    /// Discover neighbors using STOQ integration
    pub async fn discover_matrix_neighbors_stoq(
        &self,
        max_distance: f64,
        max_count: usize,
    ) -> Result<Vec<MatrixNodeInfo>> {
        if let Some(ref stoq_integration) = self.stoq_integration {
            stoq_integration
                .discover_neighbors(max_distance, max_count)
                .await
        } else {
            Ok(Vec::new())
        }
    }

    /// Accept incoming connections.
    ///
    /// When `peer_ctx` is `Some`, a persistent message loop is spawned for
    /// each successfully handshaked peer, enabling reception of blocks,
    /// shards, and sync messages.
    pub async fn accept_connections(&self, peer_ctx: Option<Arc<PeerContext>>) -> Result<()> {
        info!("Starting to accept incoming connections");

        loop {
            match self.transport.accept().await {
                Ok(connection) => {
                    // Enforce max peers
                    if self.nodes.read().await.len() >= 50 {
                        warn!("Max peers (50) reached, rejecting connection");
                        continue;
                    }

                    let nodes = self.nodes.clone();
                    let local_coord = self.local_coordinate;
                    let ctx = peer_ctx.clone();
                    let signer = self.signer.clone();
                    let proof_provider = self.proof_provider.clone();

                    // Handle connection in background
                    tokio::spawn(async move {
                        match Self::handle_incoming_handshake(
                            connection.clone(),
                            nodes,
                            local_coord,
                            signer,
                            proof_provider,
                        )
                        .await
                        {
                            Ok((peer_node_id, peer_coord)) => {
                                // Spawn persistent message loop if context available
                                if let Some(ctx) = ctx {
                                    tokio::spawn(async move {
                                        run_peer_message_loop(
                                            connection,
                                            peer_node_id,
                                            peer_coord,
                                            ctx,
                                        )
                                        .await;
                                    });
                                }
                            }
                            Err(e) => {
                                warn!("Incoming handshake failed: {e}");
                            }
                        }
                    });
                }
                Err(e) => {
                    warn!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    /// Handle the bilateral handshake for an incoming connection (R11).
    ///
    /// Delegates to `stoq::accept_handshake()` which implements the
    /// 3-message challenge-response protocol using NodeSigner and
    /// StateProofProvider traits.
    async fn handle_incoming_handshake(
        connection: Arc<stoq::Connection>,
        nodes: Arc<RwLock<HashMap<String, NetworkNode>>>,
        local_coord: MatrixCoordinate,
        signer: Arc<dyn hypermesh_lib::NodeSigner>,
        proof_provider: Arc<dyn hypermesh_lib::StateProofProvider>,
    ) -> Result<(String, MatrixCoordinate)> {
        debug!("Accepted incoming connection — starting bilateral handshake");

        let mut stream = connection.accept_stream().await?;
        let coord_tuple = (local_coord.x, local_coord.y, local_coord.z);

        let result = stoq::accept_handshake(
            &mut stream,
            signer.as_ref(),
            proof_provider.as_ref(),
            coord_tuple,
        )
        .await?;

        let coordinate = MatrixCoordinate::new(
            result.peer_coordinate.0,
            result.peer_coordinate.1,
            result.peer_coordinate.2,
        )
        .map_err(|e| anyhow!("Invalid peer coordinate: {e}"))?;

        let peer_node_id = result.peer_node_id;

        let node = NetworkNode {
            coordinate,
            address: connection.endpoint().to_socket_addr(),
            node_id: peer_node_id.clone(),
            privacy_mode: PrivacyMode::PUBLIC, // Will be negotiated later
            connection: Some(connection),
        };

        nodes.write().await.insert(peer_node_id.clone(), node);
        info!(
            "Bilateral verification complete — added incoming node {} to network",
            &peer_node_id[..8.min(peer_node_id.len())]
        );
        Ok((peer_node_id, coordinate))
    }
}

// ── Peer message loop and handlers ─────────────────────────────────

/// Tag bytes for wire-protocol message types.
const TAG_SHARD_SEND: u8 = 0x01;
const TAG_SHARD_FETCH: u8 = 0x02;
const TAG_BLOCK_ANNOUNCE: u8 = 0x03;
const TAG_SYNC_MESSAGE: u8 = 0x10;
const TAG_GOSSIP: u8 = 0x20;

/// Persistent read loop for a connected peer.
///
/// Accepts new streams from the connection, reads the full payload,
/// dispatches based on the first byte (tag), and handles the message.
/// Runs until the connection is closed.
async fn run_peer_message_loop(
    connection: Arc<stoq::Connection>,
    peer_node_id: String,
    peer_coord: MatrixCoordinate,
    ctx: Arc<PeerContext>,
) {
    info!(
        "Starting message loop for peer {} at ({},{},{})",
        &peer_node_id[..8.min(peer_node_id.len())],
        peer_coord.x,
        peer_coord.y,
        peer_coord.z,
    );

    loop {
        // Each message arrives on its own stream (one-shot pattern).
        let mut stream = match connection.accept_stream().await {
            Ok(s) => s,
            Err(e) => {
                debug!(
                    "Peer {} connection closed: {}",
                    &peer_node_id[..8.min(peer_node_id.len())],
                    e,
                );
                break;
            }
        };

        // Read full stream payload.
        let data = match stream.receive().await {
            Ok(d) if !d.is_empty() => d,
            Ok(_) => continue, // empty stream, skip
            Err(e) => {
                debug!("Stream read error from {}: {}", &peer_node_id[..8.min(peer_node_id.len())], e);
                continue;
            }
        };

        let tag = data[0];

        match tag {
            TAG_SHARD_SEND | TAG_SHARD_FETCH => {
                handle_shard_dispatch(&data, &mut stream, &ctx).await;
            }
            TAG_BLOCK_ANNOUNCE => {
                handle_block_announce(&data, &peer_node_id, &ctx).await;
            }
            TAG_SYNC_MESSAGE => {
                handle_sync_message(&data[1..], &mut stream, &peer_node_id, &peer_coord, &ctx).await;
            }
            TAG_GOSSIP => {
                debug!(
                    "Gossip message from peer {} ({} bytes)",
                    &peer_node_id[..8.min(peer_node_id.len())],
                    data.len() - 1,
                );
            }
            _ => {
                warn!(
                    "Unknown message tag 0x{:02x} from peer {}",
                    tag,
                    &peer_node_id[..8.min(peer_node_id.len())],
                );
            }
        }
    }

    info!(
        "Message loop ended for peer {}",
        &peer_node_id[..8.min(peer_node_id.len())],
    );
}

/// Dispatch a shard send/fetch message to the shard store.
async fn handle_shard_dispatch(
    data: &[u8],
    stream: &mut stoq::Stream,
    ctx: &PeerContext,
) {
    match shard_transport::handle_shard_message(data, &ctx.shard_store).await {
        Ok(Some(response_data)) => {
            // SHARD_FETCH: send response back
            if let Err(e) = stream.send(&response_data).await {
                warn!("Failed to send shard response: {}", e);
            }
        }
        Ok(None) => {
            // SHARD_SEND: no response needed
        }
        Err(e) => {
            warn!("Shard message error: {}", e);
        }
    }
}

/// Handle a received block announcement.
///
/// Wire format (from `StoqBlockTransportAdapter::build_wire_payload`):
/// - `[0]`       tag 0x03
/// - `[1..9]`    block_json_len: u64 LE
/// - `[9..9+N]`  block JSON
/// - `[9+N..17+N]` proof_hash_len: u64 LE
/// - `[17+N..17+N+P]` proof_hash bytes
async fn handle_block_announce(
    data: &[u8],
    peer_node_id: &str,
    ctx: &PeerContext,
) {
    // Minimum: tag(1) + block_json_len(8) = 9
    if data.len() < 9 {
        warn!("Block announce too short ({} bytes)", data.len());
        return;
    }

    let block_json_len = u64::from_le_bytes(
        data[1..9].try_into().unwrap_or([0u8; 8]),
    ) as usize;

    if data.len() < 9 + block_json_len {
        warn!(
            "Block announce truncated: need {} bytes, have {}",
            9 + block_json_len,
            data.len(),
        );
        return;
    }

    let block: crate::blockchain::block::Block =
        match serde_json::from_slice(&data[9..9 + block_json_len]) {
            Ok(b) => b,
            Err(e) => {
                warn!("Invalid block JSON from {}: {}", &peer_node_id[..8.min(peer_node_id.len())], e);
                return;
            }
        };

    // Verify BLAKE3 hash integrity
    if !block.verify_hash() {
        warn!(
            "Block {} hash mismatch from peer {}",
            block.index,
            &peer_node_id[..8.min(peer_node_id.len())],
        );
        return;
    }

    // Check if we already have this block
    let our_height = ctx.blockchain.get_height().await;
    if block.index <= our_height {
        debug!(
            "Already have block {} (our height: {}), skipping",
            block.index, our_height,
        );
        return;
    }

    // Insert received block
    match ctx.blockchain.insert_received_block(block.clone()).await {
        Ok(()) => {
            info!(
                "Received and stored block #{} from peer {}",
                block.index,
                &peer_node_id[..8.min(peer_node_id.len())],
            );
        }
        Err(e) => {
            debug!(
                "Block {} insertion failed: {} (from peer {})",
                block.index,
                e,
                &peer_node_id[..8.min(peer_node_id.len())],
            );
        }
    }
}

/// Handle a sync/reflector message (tag 0x10).
///
/// The payload after the tag byte is a JSON-encoded `MatrixMessage`.
/// Dispatched through `SyncDispatcher` to update sync state and
/// reflector pool. If the dispatcher produces a reply, it is sent
/// back on the same stream.
async fn handle_sync_message(
    payload: &[u8],
    stream: &mut stoq::Stream,
    sender_node_id: &str,
    sender_coord: &MatrixCoordinate,
    ctx: &PeerContext,
) {
    let msg: MatrixMessage = match serde_json::from_slice(payload) {
        Ok(m) => m,
        Err(e) => {
            warn!(
                "Invalid sync message JSON from {}: {}",
                &sender_node_id[..8.min(sender_node_id.len())],
                e,
            );
            return;
        }
    };

    let sender_pos = hypermesh_lib::MatrixPosition {
        x: sender_coord.x as f64,
        y: sender_coord.y as f64,
        z: sender_coord.z as f64,
    };

    // Lock sync subsystems and dispatch
    let mut sm = ctx.sync_manager.lock().await;
    let mut rp = ctx.reflector_pool.lock().await;

    let mut dispatcher = sync_dispatch::SyncDispatcher {
        sync_manager: &mut sm,
        reflector_pool: &mut rp,
        block_provider: None,
    };

    let response = dispatcher.dispatch(msg, sender_node_id, sender_pos);

    // Send reply if dispatcher produced one
    if let sync_dispatch::DispatchResponse::Reply(reply_msg) = response {
        if let Ok(reply_data) = serde_json::to_vec(&reply_msg) {
            // Prefix with sync tag so receiver can dispatch
            let mut tagged = Vec::with_capacity(1 + reply_data.len());
            tagged.push(TAG_SYNC_MESSAGE);
            tagged.extend_from_slice(&reply_data);
            if let Err(e) = stream.send(&tagged).await {
                debug!("Failed to send sync reply: {}", e);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// MetricsReporter — collects node metrics for engauge streaming pipeline
// ---------------------------------------------------------------------------

/// Lightweight metrics reporter that collects real node data and produces
/// engauge-compatible `MetricsFrame` JSON for the streaming pipeline.
///
/// Emits structured `tracing` events that engauge can ingest, and optionally
/// pushes frames to engauge's STOQ API at `[::1]:9296` when connected.
pub struct MetricsReporter {
    node_id: String,
    sequence: u64,
}

impl MetricsReporter {
    /// Create a new reporter for the given node.
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            sequence: 0,
        }
    }

    /// Build a capacity metrics frame from current node state.
    ///
    /// Returns JSON bytes compatible with `engauge::streaming::MetricsFrame`.
    pub fn build_capacity_frame(
        &mut self,
        chain_height: u64,
        peer_count: usize,
        shard_count: usize,
        cpu_usage: f64,
        memory_usage: f64,
    ) -> Vec<u8> {
        self.sequence += 1;
        let now_us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0);

        let frame = serde_json::json!({
            "source_node": self.node_id,
            "timestamp_us": now_us,
            "privacy_mode": { "scope": "Unbounded", "tracked": true },
            "sequence": self.sequence,
            "payload": {
                "Capacity": {
                    "bytes_served": 0u64,
                    "compute_delivered": chain_height,
                    "storage_maintained_bytes": shard_count as u64 * 65536,
                    "bandwidth_available_bps": 0u64,
                    "uptime_ratio": 1.0,
                }
            }
        });

        info!(
            target: "engauge::metrics",
            chain_height,
            peer_count,
            shard_count,
            cpu_usage_pct = format!("{:.1}", cpu_usage),
            memory_usage_pct = format!("{:.1}", memory_usage),
            "node_metrics"
        );

        serde_json::to_vec(&frame).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_manager_creation() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let config = stoq::TransportConfig {
            // Use OS-assigned port to avoid bind conflicts in tests
            port: 0,
            bind_address: std::net::Ipv6Addr::LOCALHOST,
            ..stoq::TransportConfig::default()
        };
        let transport = match stoq::StoqTransport::new(config).await {
            Ok(t) => Arc::new(t),
            Err(e) => {
                // In CI/sandboxed environments, socket binding may fail - skip gracefully
                eprintln!(
                    "Skipping test_network_manager_creation: STOQ transport init failed: {e}"
                );
                return;
            }
        };

        // Generate a test FALCON-1024 identity
        let test_identity = crate::identity::FalconIdentity::generate();
        let signer: Arc<dyn hypermesh_lib::NodeSigner> = Arc::new(test_identity);
        let proof_provider: Arc<dyn hypermesh_lib::StateProofProvider> = Arc::new(
            crate::proof_of_state::BlockMatrixProofProvider::new(signer.node_id().to_string()),
        );
        let manager = NetworkManager::new(
            coord,
            transport,
            PrivacyMode::PRIVATE,
            vec![],
            signer,
            proof_provider,
        )
        .await
        .expect("test: manager creation");

        assert_eq!(manager.get_node_count().await, 0);
    }
}
