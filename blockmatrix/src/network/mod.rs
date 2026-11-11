// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Network layer for multi-node communication
//!
//! This module provides the actual networking implementation for Block-MATRIX nodes
//! to discover and communicate with each other using STOQ transport.

#[cfg(test)]
#[path = "tests/accept_lifecycle.rs"]
mod accept_lifecycle_tests;
mod accept_loop;
mod manager_types;
pub use manager_types::{
    DemandEntry, NetworkManager, NetworkNode, PeerContext, SwarmDemandTracker,
};

pub mod asset_chain_wire;
pub mod attestation_wire;
pub mod blockchain_integration;
pub mod ca_enrollment;
pub mod chunk;
pub mod cluster;
pub mod consumer_provider;
pub mod discovery;
pub mod gossip;
pub mod hash_bucket;
pub mod locality;
pub mod message_handlers;
pub mod metrics_reporter;
pub mod peer_auth;
mod peer_discovery;
pub mod placement;
pub mod reflector_pool;
/// Replication / placement background loops (H3 demand feed, H4/H5 propagation,
/// E.2 replication-poll). Extracted from the connect command (P8); gated on the
/// `intelligence` feature since the loops consume ngauge's analytics surface.
#[cfg(feature = "intelligence")]
pub mod replication_service;
pub mod shard_dedup;
pub mod shard_distribution;
pub mod shard_locate;
pub mod shard_store;
pub mod shard_transport;
pub mod stoq_integration;
pub mod stoq_transfer_transport;
pub mod swarm_provider;
pub mod sync_dispatch;
pub mod validation;

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::net::{Ipv6Addr, SocketAddr};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::bootstrap::PrivacyMode;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::stoq_integration::MatrixStoqIntegration;
use hypermesh_lib::NetworkId;

/// Result of a bilateral handshake including both network node info
/// and the raw cryptographic proof data needed for peer authentication.
struct HandshakeData {
    node: NetworkNode,
    /// FALCON-1024 public key of the peer
    peer_pubkey: Vec<u8>,
    /// Validated state proof bytes from the peer
    peer_proof: Vec<u8>,
    /// Canonical network the peer advertised during post-handshake metadata
    /// exchange, mapped from the wire string via [`NetworkId::from_wire_str`] at
    /// the ingress boundary. An older peer that sends no metadata yields
    /// `from_wire_str("")` — a network id that will not match ours, exactly as
    /// the empty-string mismatch behaved before this retype.
    peer_network_id: NetworkId,
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

    /// Gracefully shut down the network layer.
    ///
    /// Delegates to [`stoq::StoqTransport::shutdown`], which closes every active
    /// connection and the quinn endpoint. Closing the endpoint makes the
    /// `accept_connections` accept loop and every per-peer
    /// `run_peer_message_loop` return an error and break, dropping their
    /// `Arc<Connection>`s and the connection driver so the runtime can tear
    /// down promptly instead of parking on quinn's idle timeout.
    pub async fn shutdown(&self) {
        self.transport.shutdown().await;
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
                info!(
                    "Received peer network_id from metadata: '{}'",
                    peer_meta.network_id
                );
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
            // Ingress boundary: the peer's advertised wire string becomes its
            // canonical NetworkId here (and only here on this path).
            peer_network_id: NetworkId::from_wire_str(&peer_network_id),
        })
    }

    // mDNS and gossip methods are in peer_discovery.rs.

    // find_matrix_neighbors and find_k_nearest_nodes are in peer_discovery.rs.
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
        let proof_provider: Arc<dyn hypermesh_lib::StateProofProvider> =
            Arc::new(crate::proof_of_state::BlockMatrixProofProvider::new(
                signer.node_id().to_string(),
                signer.clone(),
            ));
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
