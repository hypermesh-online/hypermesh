// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Network manager data types and small connection/query methods.

use super::{ca_enrollment, message_handlers, swarm_provider};
use crate::blockchain::node_chain::NodeBlockchain;
use crate::blockchain::propagation::BlockPropagator;
use crate::blockchain::sync_manager::SyncManager;
use crate::bootstrap::{DnsResolver, PrivacyMode};
use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::gossip::GossipProtocol;
use crate::network::hash_bucket::SpatialBucketAssigner;
use crate::network::peer_auth::AuthenticatedPeers;
use crate::network::reflector_pool::ReflectorPool;
use crate::network::shard_store::ShardStore;
use crate::network::stoq_integration::{MatrixNodeInfo, MatrixStoqIntegration};
use anyhow::{anyhow, Result};
use hypermesh_lib::BlockchainScope;
use message_handlers::run_peer_message_loop;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

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

pub use super::peer_auth::demand::{DemandEntry, SwarmDemandTracker};

/// Shared context for peer message processing.
///
/// Passed to the per-connection message loop so that incoming blocks,
/// shards, and sync messages can reach the appropriate subsystems.
pub struct PeerContext {
    /// The node's BlockMatrix storage container (for indexing received blocks and entries).
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
    /// Session-owned routing hints, independent of authentication/resource grants.
    pub connected_routes: super::connected_peer_routes::ConnectedPeerRoutes,
    /// Shared source metadata, guarded before routes during lifecycle changes.
    pub connected_nodes: super::connected_peer_routes::NetworkNodes,
    /// DNS resolver for populating DNS entries extracted from received blocks.
    pub dns_resolver: Option<DnsResolver>,
    /// Authenticated peers map — only peers in this map can send us data.
    pub authenticated_peers: AuthenticatedPeers,
    /// Gossip protocol instance for mesh coordination.
    pub gossip_protocol: Option<Arc<GossipProtocol>>,
    /// Swarm demand tracker for recording shard fetch requests.
    /// Fed into ngauge SwarmAnalytics when the `intelligence` feature is enabled.
    pub swarm_demand_tracker: Arc<SwarmDemandTracker>,
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
    /// ngauge SwarmAnalytics for processing received metrics and demand data.
    /// When `Some`, `handle_metrics_connection` feeds frames into this pipeline
    /// and the NGaugeBridge reads analytics for propagation weight computation.
    #[cfg(feature = "intelligence")]
    pub ngauge_analytics: Option<Arc<std::sync::Mutex<ngauge::SwarmAnalytics>>>,
    /// ngauge MetricsIngestionPipeline for processing received MetricsFrame payloads.
    /// When `Some`, incoming metrics frames are routed through differential privacy
    /// filtering and stored for trending analysis.
    #[cfg(feature = "intelligence")]
    pub ngauge_ingestion: Option<Arc<std::sync::Mutex<ngauge::MetricsIngestionPipeline>>>,
    /// Phase G.2 — cross-network transfer coordinator.
    ///
    /// When `Some`, incoming `TAG_TRANSFER_REGISTER_REQ` messages are
    /// routed to [`TransferCoordinator::handle_register_request`] and
    /// the resulting `TAG_TRANSFER_REGISTER_ACK` is written back over
    /// the same stream. Incoming acks are delivered to awaiting
    /// `initiate` futures via `deliver_register_ack`.
    ///
    /// Alpha-default inert: when `None`, handlers fall back to the
    /// G.1 log-only behaviour and drop the messages.
    pub transfer_coordinator: Option<Arc<crate::gateway::TransferCoordinator>>,
    /// P5 unification — optional eBPF orchestrator for mirroring peer
    /// authentication into the kernel fast-path maps.
    ///
    /// When `Some`, a successful bilateral PoS handshake ALSO writes the
    /// peer's IPv6 source into the kernel `pos_header_map` (validated=1)
    /// and `policy_map` (requires_pos=1) via
    /// [`hypermesh_ebpf::HyperMeshEbpf::set_peer_pos_validated`], so the
    /// kernel gate keys on the SAME source P1's userspace gate authorized.
    /// When `None` (or when the XDP program is not attached), the node runs
    /// in the userspace-only tier unchanged — graceful degradation.
    pub ebpf: Option<Arc<hypermesh_ebpf::HyperMeshEbpf>>,
}

/// Network manager for multi-node communication
pub struct NetworkManager {
    /// Local node coordinate
    pub(crate) local_coordinate: MatrixCoordinate,
    /// Local STOQ transport
    pub(crate) transport: Arc<stoq::StoqTransport>,
    /// Known nodes in the network
    pub(crate) nodes: Arc<RwLock<HashMap<String, NetworkNode>>>,
    pub(crate) connected_routes: super::connected_peer_routes::ConnectedPeerRoutes,
    /// Bootstrap nodes
    pub(crate) bootstrap_nodes: Vec<SocketAddr>,
    /// Current privacy mode
    pub(crate) privacy_mode: Arc<RwLock<PrivacyMode>>,
    /// Matrix-STOQ integration layer
    pub(crate) stoq_integration: Option<Arc<MatrixStoqIntegration>>,
    /// FALCON-1024 node signer (from TrustChain via lib trait)
    pub(crate) signer: Arc<dyn hypermesh_lib::NodeSigner>,
    /// State proof provider (BlockMatrix implementation)
    pub(crate) proof_provider: Arc<dyn hypermesh_lib::StateProofProvider>,
    /// Authenticated peers map shared with PeerContext.
    pub(crate) authenticated_peers: AuthenticatedPeers,
    /// Network ID for multi-node sync (sent in handshake metadata).
    pub(crate) network_id: String,
}

impl NetworkManager {
    /// Get all connected nodes
    pub async fn get_connected_nodes(&self) -> Vec<NetworkNode> {
        self.nodes.read().await.values().cloned().collect()
    }

    /// Send a tag-prefixed payload to a single connected peer by node ID.
    ///
    /// Looks up the peer's live STOQ connection in the `nodes` map (keyed by
    /// the raw string `node_id`), opens a fresh unidirectional stream, and
    /// sends `[tag] ++ payload` — the exact framing used by
    /// [`shard_transport::StoqShardTransport::send_shard`] and
    /// [`consumer_provider::broadcast_announcement`].
    ///
    /// This is the by-`node_id` complement to `broadcast_announcement`
    /// (which fans out to many connections): it delivers a single tagged
    /// message to one already-connected peer. It does NOT auto-dial — the
    /// recipient must be a currently-connected peer. Callers degrade
    /// gracefully on `Err` (e.g. `share.send` still returns the created,
    /// signed invite when delivery is not possible).
    ///
    /// Errors when the peer is not in the connected set, has no cached
    /// connection, the connection is inactive, or the stream send fails.
    pub async fn send_tagged_to_peer(&self, node_id: &str, tag: u8, payload: &[u8]) -> Result<()> {
        let connection = {
            let nodes = self.nodes.read().await;
            let node = nodes
                .get(node_id)
                .ok_or_else(|| anyhow!("peer {node_id} is not connected"))?;
            node.connection
                .clone()
                .ok_or_else(|| anyhow!("no live connection for peer {node_id}"))?
        };

        if !connection.is_active() {
            return Err(anyhow!("connection to peer {node_id} is not active"));
        }

        let mut framed = Vec::with_capacity(1 + payload.len());
        framed.push(tag);
        framed.extend_from_slice(payload);

        let mut stream = connection
            .open_stream()
            .await
            .map_err(|e| anyhow!("failed to open stream to peer {node_id}: {e}"))?;
        stream
            .send(&framed)
            .await
            .map_err(|e| anyhow!("failed to send tagged message to peer {node_id}: {e}"))?;

        Ok(())
    }

    /// Returns the matrix coordinates of all currently connected peers.
    /// Used by BlockPropagator to determine propagation targets.
    pub async fn get_connected_coordinates(&self) -> Vec<MatrixCoordinate> {
        super::connected_peer_routes::refresh_routes(&self.nodes, &self.connected_routes).await;
        super::connected_peer_routes::coordinate_snapshot(&self.connected_routes).await
    }

    pub fn connected_routes(&self) -> super::connected_peer_routes::ConnectedPeerRoutes {
        self.connected_routes.clone()
    }

    pub fn connected_nodes(&self) -> super::connected_peer_routes::NetworkNodes {
        self.nodes.clone()
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
    pub(crate) fn get_node_id(&self) -> String {
        self.signer.node_id().to_string()
    }

    /// Spawn a background CA certificate enrollment task.
    ///
    /// After a successful bilateral PoS handshake, this replaces the
    /// self-signed bootstrap cert with a TrustChain CA-issued cert.
    /// Failure is non-fatal; the node continues with its bootstrap cert.
    pub(super) fn spawn_ca_enrollment_if_needed(&self) {
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
                    info!("Connected to domain '{}' peer: {}", domain_name, node_id);
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
}
