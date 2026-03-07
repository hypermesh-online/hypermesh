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
use crate::identity::FalconIdentity;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::matrix::neighbors::{find_k_nearest, find_neighbors};
use crate::network::reflector_pool::ReflectorPool;
use crate::network::shard_store::ShardStore;
use crate::network::stoq_integration::{MatrixMessage, MatrixNodeInfo, MatrixStoqIntegration};
use crate::proof_of_state::StateProof;

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
    /// FALCON-1024 public key for signing state proofs
    identity_pubkey: Vec<u8>,
    /// FALCON-1024 secret key for signing state proofs
    identity_secret: Vec<u8>,
}

impl NetworkManager {
    /// Create new network manager with FALCON-1024 identity for PoS signing.
    ///
    /// `identity_pubkey` and `identity_secret` are raw FALCON-1024 key bytes
    /// from `FalconIdentity`. State proofs sent during handshake are signed
    /// with these keys.
    pub async fn new(
        local_coordinate: MatrixCoordinate,
        transport: Arc<stoq::StoqTransport>,
        privacy_mode: PrivacyMode,
        bootstrap_nodes: Vec<SocketAddr>,
        identity_pubkey: Vec<u8>,
        identity_secret: Vec<u8>,
    ) -> Result<Self> {
        info!(
            "Initializing network manager at ({},{},{}) in {:?} mode",
            local_coordinate.x, local_coordinate.y, local_coordinate.z, privacy_mode
        );

        // Create Matrix-STOQ integration layer
        let node_id = blake3::hash(&identity_pubkey).to_hex().to_string();
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
            identity_pubkey,
            identity_secret,
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
            if let Err(e) = self.connect_to_peer(*bootstrap_addr).await {
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
            match self.connect_to_peer(*bootstrap_addr).await {
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

    /// Connect to a specific peer
    pub async fn connect_to_peer(&self, addr: SocketAddr) -> Result<String> {
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

        info!(
            "Successfully connected to node {} at ({},{},{})",
            node_info.node_id,
            node_info.coordinate.x,
            node_info.coordinate.y,
            node_info.coordinate.z
        );

        Ok(node_info.node_id)
    }

    /// Exchange node information with peer using 3-message bilateral
    /// challenge-response handshake (R11).
    ///
    /// Protocol:
    ///   Message 1 (A->B): coordinate, node_id, privacy_mode, falcon_pubkey, nonce_a
    ///   Message 2 (B->A): coordinate, node_id, privacy_mode, falcon_pubkey, nonce_b,
    ///                      proof_bytes, signature (covers BLAKE3(nonce_a || proof_bytes))
    ///   Message 3 (A->B): proof_bytes, signature (covers BLAKE3(nonce_b || proof_bytes))
    ///
    /// Both sides verify:
    ///   1. BLAKE3(falcon_pubkey) == declared node_id (identity binding)
    ///   2. FALCON signature covers OUR nonce (prevents replay)
    ///   3. StateProof.validate() passes (PoS thresholds)
    async fn exchange_node_info(&self, connection: &Arc<stoq::Connection>) -> Result<NetworkNode> {
        let mut stream = connection.open_stream().await?;

        let node_id_string = self.get_node_id();

        // Generate 32-byte challenge nonce
        let mut nonce_a = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce_a);

        // --- Message 1 (A->B): send our info + challenge nonce ---
        let msg1 = serde_json::json!({
            "coordinate": {
                "x": self.local_coordinate.x,
                "y": self.local_coordinate.y,
                "z": self.local_coordinate.z,
            },
            "node_id": node_id_string,
            "privacy_mode": format!("{:?}", *self.privacy_mode.read().await),
            "falcon_pubkey": hex::encode(&self.identity_pubkey),
            "nonce": hex::encode(nonce_a),
        });
        stream.send(msg1.to_string().as_bytes()).await?;

        // --- Receive Message 2 from B ---
        let msg2_data = stream.receive().await?;
        let msg2: serde_json::Value = serde_json::from_slice(&msg2_data)?;

        // Verify B's identity binding: BLAKE3(pubkey) == node_id
        let peer_pubkey = Self::extract_and_verify_identity(&msg2)?;

        // Verify B's challenge-response signature
        let peer_proof_bytes = Self::extract_hex_field(&msg2, "proof_bytes")?;
        let peer_signature = Self::extract_hex_field(&msg2, "signature")?;
        Self::verify_challenge_response(&peer_pubkey, &nonce_a, &peer_proof_bytes, &peer_signature)?;

        // Verify B's state proof
        let peer_proof = StateProof::from_bytes(&peer_proof_bytes)
            .map_err(|e| anyhow!("Failed to deserialize peer state proof: {e}"))?;
        if !peer_proof.validate() {
            return Err(anyhow!("Peer state proof validation failed"));
        }

        // Extract B's challenge nonce
        let nonce_b = Self::extract_hex_field(&msg2, "nonce")?;
        if nonce_b.len() != 32 {
            return Err(anyhow!("Invalid peer nonce length: {}", nonce_b.len()));
        }

        // --- Message 3 (A->B): respond to B's challenge ---
        let our_proof = StateProof::generate_from_network(&node_id_string)
            .await
            .unwrap_or_else(|e| {
                warn!("PoS proof generation failed (using test fallback): {e}");
                StateProof::new_for_testing()
            });
        let our_proof_bytes = our_proof
            .to_bytes()
            .map_err(|e| anyhow!("Failed to serialize state proof: {e}"))?;

        // Sign BLAKE3(nonce_b || our_proof_bytes) with our FALCON key
        let our_signature = Self::sign_challenge(&self.identity_secret, &nonce_b, &our_proof_bytes)?;

        let msg3 = serde_json::json!({
            "proof_bytes": hex::encode(&our_proof_bytes),
            "signature": hex::encode(&our_signature),
        });
        stream.send(msg3.to_string().as_bytes()).await?;

        // Parse peer coordinates
        let coordinate = Self::parse_coordinate(&msg2)?;
        let peer_node_id = msg2["node_id"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing node_id"))?
            .to_string();
        let privacy_mode = Self::parse_privacy_mode(&msg2);

        info!(
            "Bilateral verification complete with peer {}",
            &peer_node_id[..8.min(peer_node_id.len())]
        );

        Ok(NetworkNode {
            coordinate,
            address: connection.endpoint().to_socket_addr(),
            node_id: peer_node_id,
            privacy_mode,
            connection: Some(connection.clone()),
        })
    }

    // ── Bilateral verification helpers ─────────────────────────────────

    /// Extract a hex-encoded byte field from JSON.
    fn extract_hex_field(msg: &serde_json::Value, field: &str) -> Result<Vec<u8>> {
        let hex_str = msg
            .get(field)
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing field '{field}' in handshake message"))?;
        hex::decode(hex_str)
            .map_err(|e| anyhow!("Invalid hex in field '{field}': {e}"))
    }

    /// Extract peer's FALCON public key and verify identity binding.
    ///
    /// Checks that `BLAKE3(falcon_pubkey) == declared node_id`.
    fn extract_and_verify_identity(msg: &serde_json::Value) -> Result<Vec<u8>> {
        let pubkey = Self::extract_hex_field(msg, "falcon_pubkey")?;
        let declared_id = msg["node_id"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing node_id in handshake"))?;

        let computed_id = blake3::hash(&pubkey).to_hex().to_string();
        if computed_id != declared_id {
            return Err(anyhow!(
                "Identity binding failed: BLAKE3(pubkey)={} != declared node_id={}",
                &computed_id[..16],
                &declared_id[..16.min(declared_id.len())],
            ));
        }
        Ok(pubkey)
    }

    /// Sign a challenge: FALCON-1024(BLAKE3(nonce || proof_bytes)).
    fn sign_challenge(
        secret_key: &[u8],
        challenge_nonce: &[u8],
        proof_bytes: &[u8],
    ) -> Result<Vec<u8>> {
        let mut hasher = blake3::Hasher::new();
        hasher.update(challenge_nonce);
        hasher.update(proof_bytes);
        let digest = hasher.finalize();

        use pqcrypto_traits::sign::{DetachedSignature, SecretKey as _};
        let sk = pqcrypto_falcon::falcon1024::SecretKey::from_bytes(secret_key)
            .map_err(|e| anyhow!("Invalid FALCON secret key: {e}"))?;
        let sig = pqcrypto_falcon::falcon1024::detached_sign(digest.as_bytes(), &sk);
        Ok(sig.as_bytes().to_vec())
    }

    /// Verify a challenge-response: check FALCON signature over BLAKE3(nonce || proof_bytes).
    fn verify_challenge_response(
        pubkey: &[u8],
        our_nonce: &[u8],
        proof_bytes: &[u8],
        signature: &[u8],
    ) -> Result<()> {
        let mut hasher = blake3::Hasher::new();
        hasher.update(our_nonce);
        hasher.update(proof_bytes);
        let digest = hasher.finalize();

        let valid = FalconIdentity::verify(pubkey, digest.as_bytes(), signature)
            .map_err(|e| anyhow!("FALCON verification error: {e}"))?;
        if !valid {
            return Err(anyhow!("FALCON challenge-response verification failed"));
        }
        debug!("FALCON challenge-response verified successfully");
        Ok(())
    }

    /// Parse coordinate from handshake JSON message.
    fn parse_coordinate(msg: &serde_json::Value) -> Result<MatrixCoordinate> {
        let x = msg["coordinate"]["x"]
            .as_i64()
            .ok_or_else(|| anyhow!("Missing coordinate x in handshake"))?;
        let y = msg["coordinate"]["y"]
            .as_i64()
            .ok_or_else(|| anyhow!("Missing coordinate y in handshake"))?;
        let z = msg["coordinate"]["z"]
            .as_i64()
            .ok_or_else(|| anyhow!("Missing coordinate z in handshake"))?;
        MatrixCoordinate::new(x, y, z).map_err(|e| anyhow!("Invalid coordinate: {e}"))
    }

    /// Parse privacy mode from peer handshake JSON.
    fn parse_privacy_mode(peer_info: &serde_json::Value) -> PrivacyMode {
        match peer_info["privacy_mode"].as_str().unwrap_or("Private") {
            "Anonymous" => PrivacyMode::ANONYMOUS,
            "Public" => PrivacyMode::PUBLIC,
            _ => PrivacyMode::PRIVATE, // Private, P2P, and unknown all collapse to PRIVATE
        }
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

                let data = match serde_json::to_vec(&message) {
                    Ok(d) => d,
                    Err(_) => continue,
                };

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
        blake3::hash(&self.identity_pubkey).to_hex().to_string()
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
            match self.connect_to_peer(addr).await {
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
                    let privacy_mode = self.privacy_mode.clone();
                    let ctx = peer_ctx.clone();
                    let id_pubkey = self.identity_pubkey.clone();
                    let id_secret = self.identity_secret.clone();

                    // Handle connection in background
                    tokio::spawn(async move {
                        match Self::handle_incoming_handshake(
                            connection.clone(),
                            nodes,
                            local_coord,
                            privacy_mode,
                            id_pubkey,
                            id_secret,
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

    /// Handle the 3-message bilateral handshake for an incoming connection (R11).
    ///
    /// Protocol (acceptor/B side):
    ///   Receive Message 1 from A (coordinate, node_id, falcon_pubkey, nonce_a)
    ///   Send Message 2 to A (coordinate, node_id, falcon_pubkey, nonce_b,
    ///                        proof_bytes, signature over BLAKE3(nonce_a || proof))
    ///   Receive Message 3 from A (proof_bytes, signature over BLAKE3(nonce_b || proof))
    ///
    /// Returns `(peer_node_id, peer_coordinate)` on success.
    async fn handle_incoming_handshake(
        connection: Arc<stoq::Connection>,
        nodes: Arc<RwLock<HashMap<String, NetworkNode>>>,
        local_coord: MatrixCoordinate,
        privacy_mode: Arc<RwLock<PrivacyMode>>,
        identity_pubkey: Vec<u8>,
        identity_secret: Vec<u8>,
    ) -> Result<(String, MatrixCoordinate)> {
        debug!("Accepted incoming connection — starting bilateral handshake");

        let mut stream = connection.accept_stream().await?;

        // --- Receive Message 1 from A ---
        let msg1_data = stream.receive().await?;
        let msg1: serde_json::Value = serde_json::from_slice(&msg1_data)?;

        // Verify A's identity binding: BLAKE3(pubkey) == node_id
        let peer_pubkey = Self::extract_and_verify_identity(&msg1)?;

        // Extract A's challenge nonce
        let nonce_a = Self::extract_hex_field(&msg1, "nonce")?;
        if nonce_a.len() != 32 {
            return Err(anyhow!("Invalid peer nonce length: {}", nonce_a.len()));
        }

        // --- Generate our challenge nonce and state proof ---
        let mut nonce_b = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce_b);

        let node_id = blake3::hash(&identity_pubkey).to_hex().to_string();

        let our_proof = StateProof::generate_from_network(&node_id)
            .await
            .unwrap_or_else(|e| {
                warn!("PoS proof generation failed (using test fallback): {e}");
                StateProof::new_for_testing()
            });
        let our_proof_bytes = our_proof
            .to_bytes()
            .map_err(|e| anyhow!("Failed to serialize state proof: {e}"))?;

        // Sign BLAKE3(nonce_a || our_proof_bytes) — proves we responded to A's challenge
        let our_signature = Self::sign_challenge(&identity_secret, &nonce_a, &our_proof_bytes)?;

        let privacy_str = format!("{:?}", *privacy_mode.read().await);

        // --- Send Message 2 to A ---
        let msg2 = serde_json::json!({
            "coordinate": {
                "x": local_coord.x,
                "y": local_coord.y,
                "z": local_coord.z,
            },
            "node_id": node_id,
            "privacy_mode": privacy_str,
            "falcon_pubkey": hex::encode(&identity_pubkey),
            "nonce": hex::encode(nonce_b),
            "proof_bytes": hex::encode(&our_proof_bytes),
            "signature": hex::encode(&our_signature),
        });
        stream.send(msg2.to_string().as_bytes()).await?;

        // --- Receive Message 3 from A ---
        let msg3_data = stream.receive().await?;
        let msg3: serde_json::Value = serde_json::from_slice(&msg3_data)?;

        // Verify A's challenge-response signature
        let peer_proof_bytes = Self::extract_hex_field(&msg3, "proof_bytes")?;
        let peer_signature = Self::extract_hex_field(&msg3, "signature")?;
        Self::verify_challenge_response(&peer_pubkey, &nonce_b, &peer_proof_bytes, &peer_signature)?;

        // Verify A's state proof
        let peer_proof = StateProof::from_bytes(&peer_proof_bytes)
            .map_err(|e| anyhow!("Failed to deserialize peer state proof: {e}"))?;
        if !peer_proof.validate() {
            return Err(anyhow!("Peer state proof validation failed"));
        }

        // Parse peer info from Message 1
        let coordinate = Self::parse_coordinate(&msg1)?;
        let peer_node_id = msg1["node_id"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing node_id from incoming peer"))?
            .to_string();
        let peer_privacy_mode = Self::parse_privacy_mode(&msg1);

        let node = NetworkNode {
            coordinate,
            address: connection.endpoint().to_socket_addr(),
            node_id: peer_node_id.clone(),
            privacy_mode: peer_privacy_mode,
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
        let manager = NetworkManager::new(
            coord,
            transport,
            PrivacyMode::PRIVATE,
            vec![],
            test_identity.public_key.clone(),
            test_identity.secret_key_bytes().to_vec(),
        )
        .await
        .expect("test: manager creation");

        assert_eq!(manager.get_node_count().await, 0);
    }
}
