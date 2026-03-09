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
pub mod config;
pub mod discovery;
pub mod gossip;
pub mod hash_bucket;
pub mod isolation;
pub mod message_handlers;
pub mod metrics_reporter;
pub mod multi_network;
mod peer_discovery;
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
use tracing::{debug, info, warn};

use crate::blockchain::node_chain::NodeBlockchain;
use crate::blockchain::propagation::BlockPropagator;
use crate::blockchain::sync_manager::SyncManager;
use crate::bootstrap::{DnsResolver, PrivacyMode};
use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::hash_bucket::SpatialBucketAssigner;
use crate::network::reflector_pool::ReflectorPool;
use crate::network::shard_store::ShardStore;
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
    /// Blockchain scope determining block handling behavior.
    pub blockchain_scope: BlockchainScope,
    /// Spatial bucket assigner for Public mode (Network scope + Public transport).
    /// When `Some`, blocks are filtered by shard-placement proximity.
    pub spatial_bucket_assigner: Option<Arc<RwLock<SpatialBucketAssigner>>>,
    /// Live list of connected peer coordinates for block re-propagation.
    pub connected_peer_coords: Arc<RwLock<Vec<MatrixCoordinate>>>,
    /// DNS resolver for populating DNS entries extracted from received blocks.
    pub dns_resolver: Option<DnsResolver>,
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
        let endpoint = stoq::Endpoint::new(ipv6, addr.port());

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
    async fn exchange_node_info(&self, connection: &Arc<stoq::Connection>) -> Result<NetworkNode> {
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
        .await;
        let _ = stream.finish_send();
        let result = result?;

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
        )
        .await
        .expect("test: manager creation");

        assert_eq!(manager.get_node_count().await, 0);
    }
}
