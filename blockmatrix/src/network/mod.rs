// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Network layer for multi-node communication
//!
//! This module provides the actual networking implementation for Block-MATRIX nodes
//! to discover and communicate with each other using STOQ transport.

pub mod blockchain_integration;
pub mod ca_enrollment;
pub mod cluster;
pub mod consumer_provider;
pub mod config;
pub mod discovery;
pub mod gossip;
pub mod hash_bucket;
pub mod isolation;
pub mod message_handlers;
pub mod metrics_reporter;
pub mod multi_network;
pub mod peer_auth;
mod peer_discovery;
pub mod shard_dedup;
pub mod shard_distribution;
pub mod reflector_pool;
pub mod shard_store;
pub mod shard_transport;
pub mod stoq_integration;
pub mod swarm_provider;
pub mod sync_dispatch;
pub mod trust;
pub mod validation;

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::net::{Ipv6Addr, SocketAddr};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::blockchain::node_chain::NodeBlockchain;
use crate::blockchain::propagation::BlockPropagator;
use crate::blockchain::sync_manager::SyncManager;
use crate::bootstrap::{DnsResolver, PrivacyMode};
use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::hash_bucket::SpatialBucketAssigner;
use crate::network::reflector_pool::ReflectorPool;
use crate::network::shard_store::ShardStore;
use crate::network::peer_auth::AuthenticatedPeers;
use crate::network::gossip::GossipProtocol;
use crate::network::stoq_integration::{MatrixNodeInfo, MatrixStoqIntegration};
use hypermesh_lib::BlockchainScope;

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

/// Tracks shard fetch demand from network peers.
///
/// Records which shards are being requested and by whom, providing
/// the data needed for engauge's `SwarmAnalytics` to make replication
/// and dispersion decisions.
pub struct SwarmDemandTracker {
    /// Per-shard request counts and last-access timestamps.
    entries: tokio::sync::Mutex<HashMap<hypermesh_lib::ContentHash, DemandEntry>>,
}

/// A single shard's demand record.
#[derive(Debug, Clone)]
pub struct DemandEntry {
    /// Total number of fetch requests for this shard.
    pub request_count: u64,
    /// Unix microsecond timestamp of last fetch request.
    pub last_request_us: u64,
    /// Node IDs of requesters (deduplicated for unique consumer count).
    pub requester_ids: std::collections::HashSet<String>,
}

impl SwarmDemandTracker {
    /// Create a new empty demand tracker.
    pub fn new() -> Self {
        Self {
            entries: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Record a fetch request for a shard from a specific peer.
    pub async fn record_fetch(
        &self,
        shard_id: hypermesh_lib::ContentHash,
        requester_node_id: &str,
    ) {
        let now_us = chrono::Utc::now().timestamp_micros() as u64;
        let mut entries = self.entries.lock().await;
        let entry = entries.entry(shard_id).or_insert_with(|| DemandEntry {
            request_count: 0,
            last_request_us: 0,
            requester_ids: std::collections::HashSet::new(),
        });
        entry.request_count += 1;
        entry.last_request_us = now_us;
        entry.requester_ids.insert(requester_node_id.to_string());
    }

    /// Get a snapshot of all demand entries for feeding into engauge.
    pub async fn snapshot(&self) -> HashMap<hypermesh_lib::ContentHash, DemandEntry> {
        self.entries.lock().await.clone()
    }

    /// Get demand entry for a specific shard.
    pub async fn get(&self, shard_id: &hypermesh_lib::ContentHash) -> Option<DemandEntry> {
        self.entries.lock().await.get(shard_id).cloned()
    }
}

impl Default for SwarmDemandTracker {
    fn default() -> Self {
        Self::new()
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
    /// Shared network ID for multi-node sync (nodes with the same ID sync blocks).
    pub network_id: String,
    /// Blockchain scope determining block handling behavior.
    pub blockchain_scope: BlockchainScope,
    /// Spatial bucket assigner for Public mode (Network scope + Public transport).
    /// When `Some`, blocks are filtered by shard-placement proximity.
    pub spatial_bucket_assigner: Option<Arc<RwLock<SpatialBucketAssigner>>>,
    /// Live list of connected peer coordinates for block re-propagation.
    pub connected_peer_coords: Arc<RwLock<Vec<MatrixCoordinate>>>,
    /// DNS resolver for populating DNS entries extracted from received blocks.
    pub dns_resolver: Option<DnsResolver>,
    /// Authenticated peers map — only peers in this map can send us data.
    pub authenticated_peers: AuthenticatedPeers,
    /// Gossip protocol instance for mesh coordination.
    pub gossip_protocol: Option<Arc<GossipProtocol>>,
    /// Swarm demand tracker for recording shard fetch requests.
    /// Fed into engauge SwarmAnalytics when the `intelligence` feature is enabled.
    pub swarm_demand_tracker: Arc<SwarmDemandTracker>,
    /// DNS popularity tracker for engauge-driven replication of popular names.
    /// When `Some`, DNS resolution requests are recorded for analytics.
    pub dns_popularity_tracker: Option<Arc<crate::dns::DnsPopularityTracker>>,
    /// Shard location index for consumer-becomes-provider (R12).
    /// Tracks which peers provide which shards, populated from TAG_SHARD_ANNOUNCE.
    pub shard_location_index: Option<Arc<swarm_provider::ShardLocationIndex>>,
    /// Inbox for received share invitations (P2P file sharing).
    pub inbox_store: Option<Arc<crate::sharing::inbox::InboxStore>>,
    /// Store for received direct messages (P2P encrypted messaging).
    pub message_store: Option<Arc<crate::messaging::store::MessageStore>>,
    /// Key rotation chains per node (node_id -> list of rotation entries).
    /// Used for key continuity verification and split-brain detection.
    pub rotation_chains: Option<Arc<RwLock<HashMap<String, Vec<serde_json::Value>>>>>,
    /// engauge SwarmAnalytics for processing received metrics and demand data.
    /// When `Some`, `handle_metrics_connection` feeds frames into this pipeline
    /// and the EngaugeBridge reads analytics for propagation weight computation.
    #[cfg(feature = "intelligence")]
    pub engauge_analytics: Option<Arc<std::sync::Mutex<engauge::SwarmAnalytics>>>,
    /// engauge MetricsIngestionPipeline for processing received MetricsFrame payloads.
    /// When `Some`, incoming metrics frames are routed through differential privacy
    /// filtering and stored for trending analysis.
    #[cfg(feature = "intelligence")]
    pub engauge_ingestion: Option<Arc<std::sync::Mutex<engauge::MetricsIngestionPipeline>>>,
}

/// Result of a bilateral handshake including both network node info
/// and the raw cryptographic proof data needed for peer authentication.
struct HandshakeData {
    node: NetworkNode,
    /// FALCON-1024 public key of the peer
    peer_pubkey: Vec<u8>,
    /// Validated state proof bytes from the peer
    peer_proof: Vec<u8>,
    /// Network ID received from the peer during post-handshake metadata exchange.
    /// Empty string if the peer is running an older version without this field.
    peer_network_id: String,
}

/// Post-handshake metadata exchanged at the blockmatrix layer.
///
/// Sent after the STOQ bilateral PoS handshake completes, carrying
/// application-level fields that don't belong in the crypto protocol.
/// Uses `#[serde(default)]` on all fields for backward compatibility
/// with older nodes that don't send this message.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct HandshakeMetadata {
    /// Network ID for multi-node sync (nodes with the same ID sync blocks).
    #[serde(default)]
    network_id: String,
}

/// Check whether an IPv6 address is valid for WAN (non-loopback) connections.
///
/// Returns `false` for loopback (`::1`). All other addresses are considered
/// valid WAN targets. Link-local addresses (`fe80::`) are valid but may not
/// route across subnets.
fn is_valid_wan_address(addr: &Ipv6Addr) -> bool {
    !addr.is_loopback()
}

/// Network manager for multi-node communication
pub struct NetworkManager {
    /// Local node coordinate
    pub(super) local_coordinate: MatrixCoordinate,
    /// Local STOQ transport
    pub(super) transport: Arc<stoq::StoqTransport>,
    /// Known nodes in the network
    pub(super) nodes: Arc<RwLock<HashMap<String, NetworkNode>>>,
    /// Bootstrap nodes
    pub(super) bootstrap_nodes: Vec<SocketAddr>,
    /// Current privacy mode
    pub(super) privacy_mode: Arc<RwLock<PrivacyMode>>,
    /// Matrix-STOQ integration layer
    pub(super) stoq_integration: Option<Arc<MatrixStoqIntegration>>,
    /// FALCON-1024 node signer (from TrustChain via lib trait)
    pub(super) signer: Arc<dyn hypermesh_lib::NodeSigner>,
    /// State proof provider (BlockMatrix implementation)
    pub(super) proof_provider: Arc<dyn hypermesh_lib::StateProofProvider>,
    /// Authenticated peers map shared with PeerContext.
    pub(super) authenticated_peers: AuthenticatedPeers,
    /// Network ID for multi-node sync (sent in handshake metadata).
    pub(super) network_id: String,
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
        network_id: String,
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
            authenticated_peers: peer_auth::new_authenticated_peers(),
            network_id,
        })
    }

    // Discovery methods (start_discovery, join_network, etc.) are in
    // peer_discovery.rs as a separate impl block.

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
        let ipv6 = match addr {
            SocketAddr::V6(v6) => *v6.ip(),
            SocketAddr::V4(v4) => v4.ip().to_ipv6_mapped(),
        };

        // WAN address validation: reject loopback when WAN mode is active
        let wan_enabled = self.transport.wan_enabled();
        if wan_enabled && !is_valid_wan_address(&ipv6) {
            return Err(anyhow!(
                "Cannot connect to loopback address {} in WAN mode",
                ipv6
            ));
        }

        // Warn about link-local addresses over WAN — they may not route
        if wan_enabled && (ipv6.segments()[0] & 0xffc0) == 0xfe80 {
            warn!(
                "Connecting to link-local address {} over WAN — may not route across subnets",
                ipv6
            );
        }
        let endpoint = stoq::Endpoint::new(ipv6, addr.port());

        // Connect via STOQ
        let connection = self.transport.connect(&endpoint).await?;

        // Exchange node information via bilateral PoS handshake (R11)
        let our_network_id = peer_ctx
            .as_ref()
            .map(|c| c.network_id.as_str())
            .unwrap_or(self.network_id.as_str());
        let handshake = self.exchange_node_info(&connection, our_network_id).await?;
        let node_info = handshake.node;

        // Store the connected node
        let mut nodes = self.nodes.write().await;
        nodes.insert(node_info.node_id.clone(), node_info.clone());
        drop(nodes);

        info!(
            "Successfully connected to node {} at ({},{},{}) — bilateral PoS verified (proof={} bytes, pubkey={} bytes)",
            node_info.node_id,
            node_info.coordinate.x,
            node_info.coordinate.y,
            node_info.coordinate.z,
            handshake.peer_proof.len(),
            handshake.peer_pubkey.len(),
        );

        // Register as authenticated peer — proof_bytes and pubkey come from
        // the bilateral handshake result. register_authenticated_peer will
        // reject if either is empty (enforcing R11 bilateral verification).
        // Use the network_id received from the peer during metadata exchange,
        // NOT our own network_id.
        let registered = peer_auth::register_authenticated_peer(
            &self.authenticated_peers,
            peer_auth::AuthenticatedPeer {
                node_id: node_info.node_id.clone(),
                pubkey: handshake.peer_pubkey,
                coordinate: (
                    node_info.coordinate.x as i32,
                    node_info.coordinate.y as i32,
                    node_info.coordinate.z as i32,
                ),
                network_id: handshake.peer_network_id,
                authenticated_at: std::time::Instant::now(),
                proof_bytes: handshake.peer_proof,
            },
        )
        .await;

        if !registered {
            // Peer failed authentication validation — disconnect
            warn!(
                "Peer {} failed authentication registration — disconnecting",
                &node_info.node_id[..8.min(node_info.node_id.len())]
            );
            // Remove from nodes map since auth failed
            self.nodes.write().await.remove(&node_info.node_id);
            return Err(anyhow!(
                "Peer {} bilateral PoS verification incomplete — proof or pubkey missing",
                node_info.node_id
            ));
        }

        // Request CA certificate in background (Phase 2 bootstrap)
        self.spawn_ca_enrollment_if_needed();

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
    /// Opens a stream, writes a `CONN_TYPE_HANDSHAKE` discriminator byte,
    /// then delegates to `stoq::initiate_handshake_on_stream()` which
    /// implements the 3-message challenge-response protocol (R11) using
    /// NodeSigner and StateProofProvider traits.
    ///
    /// After the STOQ crypto handshake, exchanges a blockmatrix-level
    /// metadata message carrying `network_id` (for multi-node sync).
    ///
    /// Returns both the `NetworkNode` and the raw proof/pubkey data so
    /// the caller can register a fully-verified `AuthenticatedPeer`.
    async fn exchange_node_info(
        &self,
        connection: &Arc<stoq::Connection>,
        our_network_id: &str,
    ) -> Result<HandshakeData> {
        let coord = self.local_coordinate;
        let local_coord = (coord.x, coord.y, coord.z);

        // Open stream and write connection-type discriminator before handshake
        let mut stream = connection.open_stream().await?;
        stream.write_discriminator(CONN_TYPE_HANDSHAKE).await?;

        let result = stoq::initiate_handshake_on_stream(
            &mut stream,
            self.signer.as_ref(),
            self.proof_provider.as_ref(),
            local_coord,
        )
        .await?;

        // Post-handshake metadata exchange (blockmatrix layer).
        // Initiator sends metadata first, then reads peer's metadata.
        let our_meta = HandshakeMetadata {
            network_id: our_network_id.to_string(),
        };
        let meta_bytes = serde_json::to_vec(&our_meta)
            .map_err(|e| anyhow!("Failed to serialize handshake metadata: {e}"))?;
        stream.write_msg(&meta_bytes).await?;

        // Read peer's metadata (with timeout tolerance for old nodes).
        let peer_network_id = match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            stream.read_msg(),
        )
        .await
        {
            Ok(Ok(peer_meta_bytes)) => {
                let peer_meta: HandshakeMetadata =
                    serde_json::from_slice(&peer_meta_bytes).unwrap_or_default();
                info!("Received peer network_id from metadata: '{}'", peer_meta.network_id);
                peer_meta.network_id
            }
            Ok(Err(e)) => {
                warn!("Peer did not send handshake metadata: {e}");
                String::new()
            }
            Err(_) => {
                warn!("Timeout waiting for peer handshake metadata — assuming old node");
                String::new()
            }
        };

        let _ = stream.finish_send();

        let coordinate = MatrixCoordinate::new(
            result.peer_coordinate.0,
            result.peer_coordinate.1,
            result.peer_coordinate.2,
        )
        .map_err(|e| anyhow!("Invalid peer coordinate: {e}"))?;

        Ok(HandshakeData {
            node: NetworkNode {
                coordinate,
                address: connection.endpoint().to_socket_addr(),
                node_id: result.peer_node_id,
                privacy_mode: PrivacyMode::PUBLIC, // Will be negotiated later
                connection: Some(connection.clone()),
            },
            peer_pubkey: result.peer_pubkey,
            peer_proof: result.peer_proof,
            peer_network_id,
        })
    }

    // mDNS and gossip methods are in peer_discovery.rs.

    // find_matrix_neighbors and find_k_nearest_nodes are in peer_discovery.rs.

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

    /// Get the shared authenticated peers map.
    ///
    /// Callers constructing a [`PeerContext`] should clone this to ensure
    /// the peer context and network manager share the same map.
    pub fn authenticated_peers(&self) -> AuthenticatedPeers {
        self.authenticated_peers.clone()
    }

    /// Get local node ID
    pub(super) fn get_node_id(&self) -> String {
        self.signer.node_id().to_string()
    }

    /// Spawn a background CA certificate enrollment task.
    ///
    /// After a successful bilateral PoS handshake, this replaces the
    /// self-signed bootstrap cert with a TrustChain CA-issued cert.
    /// Failure is non-fatal; the node continues with its bootstrap cert.
    fn spawn_ca_enrollment_if_needed(&self) {
        let node_id = self.signer.node_id().to_string();
        let cert_manager = self.transport.cert_manager.clone();

        tokio::spawn(async move {
            let state_proof = match ca_enrollment::generate_node_state_proof(&node_id).await {
                Ok(sp) => sp,
                Err(e) => {
                    warn!("CA enrollment: failed to generate state proof: {e}");
                    return;
                }
            };
            ca_enrollment::spawn_ca_enrollment(cert_manager, node_id, state_proof);
        });
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
                    let cert_manager = self.transport.cert_manager.clone();
                    let auth_peers = self.authenticated_peers.clone();

                    // Handle connection in background — read discriminator
                    // to decide whether this is a handshake or a peer message.
                    tokio::spawn(async move {
                        if let Err(e) = message_handlers::handle_incoming_connection(
                            connection,
                            nodes,
                            local_coord,
                            signer,
                            proof_provider,
                            cert_manager,
                            ctx,
                            auth_peers,
                        )
                        .await
                        {
                            warn!("Incoming connection handling failed: {e}");
                        }
                    });
                }
                Err(e) => {
                    warn!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}

// ── Peer message loop and handlers ─────────────────────────────────

/// Connection-type discriminator: first byte written on every new STOQ
/// connection to distinguish handshake connections from peer-message
/// connections.  Without this, the acceptor would try to parse a block
/// propagation payload as a length-prefixed handshake message.
pub const CONN_TYPE_HANDSHAKE: u8 = 0x00;
pub const CONN_TYPE_PEER_MESSAGE: u8 = 0x01;
pub const CONN_TYPE_METRICS: u8 = 0x02;
pub const CONN_TYPE_GOSSIP: u8 = 0x03;

use message_handlers::run_peer_message_loop;

// Re-export MetricsReporter at the same path for backwards compatibility.
pub use metrics_reporter::MetricsReporter;

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
            crate::proof_of_state::BlockMatrixProofProvider::new(signer.node_id().to_string(), signer.clone()),
        );
        let manager = NetworkManager::new(
            coord,
            transport,
            PrivacyMode::PRIVATE,
            vec![],
            signer,
            proof_provider,
            "test-network".to_string(),
        )
        .await
        .expect("test: manager creation");

        assert_eq!(manager.get_node_count().await, 0);
    }

    #[tokio::test]
    async fn test_swarm_demand_tracker_records_fetches() {
        let tracker = SwarmDemandTracker::new();
        let hash = hypermesh_lib::ContentHash([0xCC; 32]);

        tracker.record_fetch(hash, "peer-a").await;
        tracker.record_fetch(hash, "peer-b").await;
        tracker.record_fetch(hash, "peer-a").await; // duplicate peer

        let entry = tracker.get(&hash).await.expect("test: entry exists");
        assert_eq!(entry.request_count, 3);
        assert_eq!(entry.requester_ids.len(), 2); // 2 unique peers
        assert!(entry.last_request_us > 0);
    }

    #[tokio::test]
    async fn test_swarm_demand_tracker_snapshot() {
        let tracker = SwarmDemandTracker::new();
        let h1 = hypermesh_lib::ContentHash([0xAA; 32]);
        let h2 = hypermesh_lib::ContentHash([0xBB; 32]);

        tracker.record_fetch(h1, "peer-1").await;
        tracker.record_fetch(h2, "peer-2").await;

        let snap = tracker.snapshot().await;
        assert_eq!(snap.len(), 2);
        assert!(snap.contains_key(&h1));
        assert!(snap.contains_key(&h2));
    }
}
