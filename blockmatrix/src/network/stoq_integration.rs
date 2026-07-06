// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Integration Layer for Matrix Foundation
//!
//! This module provides the integration between the Matrix Foundation and STOQ transport layer.
//! All matrix node communication goes through STOQ for secure, efficient transport.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::bootstrap::PrivacyMode;
use crate::matrix::coordinate::MatrixCoordinate;
use stoq::{Connection, StoqTransport};

/// Service discovery tag for matrix nodes
const MATRIX_SERVICE_TAG: &str = "blockmatrix.node";

/// Protocol version for matrix communication
const MATRIX_PROTOCOL_VERSION: &str = "1.0.0";

/// Matrix node announcement message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixNodeAnnouncement {
    /// Matrix coordinate of the node
    pub coordinate: MatrixCoordinate,
    /// Node ID (hash of genesis block)
    pub node_id: String,
    /// Privacy mode of the node
    pub privacy_mode: String,
    /// Protocol version
    pub protocol_version: String,
    /// Optional PoS validation token
    pub pos_token: Option<Vec<u8>>,
    /// Service capabilities
    pub services: Vec<String>,
}

/// Matrix neighbor discovery message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeighborDiscoveryRequest {
    /// Requesting node's coordinate
    pub from_coordinate: MatrixCoordinate,
    /// Maximum distance for neighbors
    pub max_distance: f64,
    /// Maximum number of neighbors to return
    pub max_neighbors: usize,
}

/// Matrix neighbor discovery response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeighborDiscoveryResponse {
    /// List of neighbor nodes
    pub neighbors: Vec<MatrixNodeInfo>,
}

/// Information about a matrix node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixNodeInfo {
    pub coordinate: MatrixCoordinate,
    pub node_id: String,
    pub address: String,
    pub privacy_mode: String,
    pub distance: f64,
}

/// Matrix message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MatrixMessage {
    /// Node announcement
    Announcement(MatrixNodeAnnouncement),
    /// Neighbor discovery request
    DiscoveryRequest(NeighborDiscoveryRequest),
    /// Neighbor discovery response
    DiscoveryResponse(NeighborDiscoveryResponse),
    /// Position broadcast
    PositionBroadcast {
        coordinate: MatrixCoordinate,
        node_id: String,
    },
    /// Heartbeat/keepalive
    Heartbeat {
        coordinate: MatrixCoordinate,
        timestamp: u64,
    },
    /// Error message
    Error { message: String },

    // --- Sync-related messages (Device <-> Network scope) ---
    /// Request blocks from a peer for chain synchronization
    SyncRequest {
        /// Network scope identifier
        network_id: String,
        /// Start syncing from this block height
        from_height: u64,
        /// Maximum blocks to return in the response
        max_blocks: u32,
    },
    /// Response containing blocks for synchronization
    SyncResponse {
        /// Network scope identifier
        network_id: String,
        /// Serialized block hashes (actual block data fetched separately)
        block_hashes: Vec<String>,
        /// The responding peer's current chain height
        peer_height: u64,
    },
    /// Announce a new block to the network
    SyncAnnounce {
        /// Network scope identifier
        network_id: String,
        /// Height of the announced block
        block_height: u64,
        /// Hash of the announced block
        block_hash: String,
    },
    /// Periodic heartbeat from a reflector node advertising availability
    ReflectorHeartbeat {
        /// Network scope identifier
        network_id: String,
        /// Reflector's current block height
        block_height: u64,
        /// Reflector's self-reported health score (0.0 to 1.0)
        health_score: f64,
    },

    /// Request specific blocks by hash from a peer
    BlockFetchRequest {
        /// BLAKE3 hex hashes of blocks to fetch
        block_hashes: Vec<String>,
    },
    /// Response containing requested blocks
    BlockFetchResponse {
        /// Serialized blocks (JSON strings)
        blocks: Vec<String>,
    },

    // --- Genesis adoption and header-based sync ---
    /// Request the genesis block for a network
    GenesisRequest {
        /// Network scope identifier
        network_id: String,
    },
    /// Response with the network's genesis block (JSON-serialized)
    GenesisResponse {
        /// Network scope identifier
        network_id: String,
        /// JSON-serialized genesis block
        genesis_block_json: String,
    },
    /// Request block headers for lightweight chain verification
    HeaderRequest {
        /// Network scope identifier
        network_id: String,
        /// Start from this block height
        from_height: u64,
        /// Maximum number of headers to return
        max_count: u32,
    },
    /// Response with block headers
    HeaderResponse {
        /// Network scope identifier
        network_id: String,
        /// JSON-serialized block headers
        headers_json: Vec<String>,
        /// Peer's current chain height
        peer_height: u64,
    },
    /// Request full blocks by hash for segments the node participates in
    SyncBlockRequest {
        /// Network scope identifier
        network_id: String,
        /// BLAKE3 hex hashes of blocks to fetch
        block_hashes: Vec<String>,
    },
    /// Response with full blocks
    SyncBlockResponse {
        /// Network scope identifier
        network_id: String,
        /// JSON-serialized blocks
        blocks_json: Vec<String>,
    },

    // --- Cross-network asset transfer messages ---
    /// Request to transfer an asset across network scopes.
    TransferRequest {
        /// Unique transfer identifier.
        transfer_id: String,
        /// Asset being transferred.
        asset_id: String,
        /// Scope the asset is leaving (e.g. "Device").
        source_scope: String,
        /// Scope the asset is entering (e.g. "Network").
        target_scope: String,
        /// PoS proof bytes from the source scope.
        #[serde(with = "serde_bytes")]
        proof_bytes: Vec<u8>,
    },
    /// Response to a cross-scope transfer request.
    TransferResponse {
        /// Transfer identifier this response corresponds to.
        transfer_id: String,
        /// Whether the target scope accepted the transfer.
        accepted: bool,
        /// PoS proof bytes from the target scope (empty if rejected).
        #[serde(with = "serde_bytes")]
        target_proof_bytes: Vec<u8>,
    },

    // --- Distributed CA messages ---

    /// A CA key share being distributed to a reflector node.
    KeyShareDistribute {
        /// Share index (1-based).
        share_index: u8,
        /// Serialized share data.
        #[serde(with = "serde_bytes")]
        share_data: Vec<u8>,
        /// CA key fingerprint (BLAKE3 of public key).
        fingerprint: [u8; 32],
    },

    /// Request for a threshold signature from a peer holding a key share.
    ThresholdSignRequest {
        /// Unique request ID.
        request_id: String,
        /// BLAKE3 hash of message to sign.
        message_hash: [u8; 32],
        /// The actual message bytes to sign.
        #[serde(with = "serde_bytes")]
        message: Vec<u8>,
        /// CA fingerprint (which CA's shares).
        ca_fingerprint: [u8; 32],
    },

    /// Response to a threshold signing request (peer's contribution).
    ThresholdSignResponse {
        /// Matches the request_id.
        request_id: String,
        /// The peer's partial signature data (serialized).
        #[serde(with = "serde_bytes")]
        share_data: Vec<u8>,
        /// Whether the peer accepts the request.
        accepted: bool,
    },

    /// Key rotation announcement (§6.2.2 identity distribution).
    KeyRotation {
        /// Hex fingerprint (BLAKE3 hash) of the outgoing FALCON public key.
        old_key_fingerprint: String,
        /// Hex fingerprint (BLAKE3 hash) of the incoming FALCON public key.
        new_key_fingerprint: String,
        /// Reason for the rotation (Scheduled, Compromise, Upgrade, Recovery).
        reason: String,
        /// FALCON-1024 signature proving old key authorized the transition.
        #[serde(with = "serde_bytes")]
        rotation_proof: Vec<u8>,
        /// Block index at which this rotation was recorded.
        block_index: u64,
        /// Unix timestamp (seconds) of the rotation event.
        timestamp: i64,
    },
}

/// STOQ-integrated Matrix communication manager
pub struct MatrixStoqIntegration {
    /// Local matrix coordinate
    local_coordinate: MatrixCoordinate,
    /// Local node ID
    node_id: String,
    /// STOQ transport instance
    transport: Arc<StoqTransport>,
    /// Privacy mode
    privacy_mode: PrivacyMode,
    /// Connected matrix nodes
    connected_nodes: Arc<RwLock<HashMap<String, MatrixNodeConnection>>>,
}

/// Connection to a matrix node
struct MatrixNodeConnection {
    coordinate: MatrixCoordinate,
    node_id: String,
    connection: Arc<Connection>,
    last_heartbeat: u64,
}

impl MatrixStoqIntegration {
    /// Create new Matrix-STOQ integration
    pub async fn new(
        local_coordinate: MatrixCoordinate,
        node_id: String,
        transport: Arc<StoqTransport>,
        privacy_mode: PrivacyMode,
    ) -> Result<Self> {
        info!(
            "Initializing Matrix-STOQ integration at ({},{},{}) with node_id: {}",
            local_coordinate.x, local_coordinate.y, local_coordinate.z, node_id
        );

        // Register with STOQ service discovery
        if let Err(e) =
            Self::register_service_discovery(&transport, &local_coordinate, &node_id).await
        {
            warn!("Failed to register with STOQ service discovery: {}", e);
        }

        Ok(Self {
            local_coordinate,
            node_id,
            transport,
            privacy_mode,
            connected_nodes: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Register with STOQ service discovery
    async fn register_service_discovery(
        _transport: &Arc<StoqTransport>,
        coordinate: &MatrixCoordinate,
        node_id: &str,
    ) -> Result<()> {
        // Create service metadata
        let metadata = serde_json::json!({
            "service": MATRIX_SERVICE_TAG,
            "coordinate": {
                "x": coordinate.x,
                "y": coordinate.y,
                "z": coordinate.z,
            },
            "node_id": node_id,
            "protocol_version": MATRIX_PROTOCOL_VERSION,
        });

        debug!(
            "Registering matrix node with STOQ service discovery: {}",
            metadata
        );

        // TODO: Call STOQ service discovery registration when API is available
        // transport.register_service(MATRIX_SERVICE_TAG, metadata).await?;

        Ok(())
    }

    // ── Legacy unsigned-PoS handshake path REMOVED (F2) ────────────────────
    //
    // `connect_to_node`, `exchange_node_info`, `handle_incoming_connection`,
    // `generate_pos_token`, and `validate_peer_pos_token` implemented the old
    // `MatrixMessage::Announcement` handshake that exchanged a RAW, UNSIGNED
    // `StateProof` and accepted it after only a structural `.validate()` — no
    // signature, no signer↔identity binding. That is exactly the F2 Sybil
    // vector. These methods were never reachable from production: the real
    // handshake goes through `NetworkManager::exchange_node_info`, which uses
    // `stoq::initiate_handshake_on_stream` / `accept_handshake` (the bilateral
    // FALCON-signed path). `connect_to_node` / `handle_incoming_connection`
    // were the only writers of `connected_nodes`, so `broadcast_position` /
    // `discover_neighbors` iterate an always-empty map in production; they were
    // exercised only by `tests/matrix_stoq_integration.rs`. Path deleted.

    /// Broadcast matrix position to all connected nodes
    pub async fn broadcast_position(&self) -> Result<()> {
        let message = MatrixMessage::PositionBroadcast {
            coordinate: self.local_coordinate,
            node_id: self.node_id.clone(),
        };

        let data = serde_json::to_vec(&message)?;
        let nodes = self.connected_nodes.read().await;

        for (node_id, node_conn) in nodes.iter() {
            debug!("Broadcasting position to node {}", node_id);

            match node_conn.connection.open_stream().await {
                Ok(mut stream) => {
                    if let Err(e) = stream.send(&data).await {
                        warn!("Failed to broadcast position to {}: {}", node_id, e);
                    }
                }
                Err(e) => {
                    warn!("Failed to open stream to {}: {}", node_id, e);
                }
            }
        }

        Ok(())
    }

    /// Discover matrix neighbors via STOQ
    pub async fn discover_neighbors(
        &self,
        max_distance: f64,
        max_count: usize,
    ) -> Result<Vec<MatrixNodeInfo>> {
        let mut all_neighbors = Vec::new();

        // Query connected nodes for their neighbors
        let nodes = self.connected_nodes.read().await;

        for (node_id, node_conn) in nodes.iter() {
            debug!("Querying node {} for neighbors", node_id);

            let request = MatrixMessage::DiscoveryRequest(NeighborDiscoveryRequest {
                from_coordinate: self.local_coordinate,
                max_distance,
                max_neighbors: max_count,
            });

            let data = serde_json::to_vec(&request)?;

            match node_conn.connection.open_stream().await {
                Ok(mut stream) => {
                    if let Err(e) = stream.send(&data).await {
                        warn!("Failed to send discovery request to {}: {}", node_id, e);
                        continue;
                    }

                    match stream.receive().await {
                        Ok(response_data) => {
                            match serde_json::from_slice::<MatrixMessage>(&response_data) {
                                Ok(MatrixMessage::DiscoveryResponse(response)) => {
                                    all_neighbors.extend(response.neighbors);
                                }
                                Ok(_) => {
                                    warn!("Unexpected response from {}", node_id);
                                }
                                Err(e) => {
                                    warn!("Failed to parse response from {}: {}", node_id, e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to receive response from {}: {}", node_id, e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to open stream to {}: {}", node_id, e);
                }
            }
        }

        // Filter and deduplicate neighbors
        let mut unique_neighbors = HashMap::new();
        for neighbor in all_neighbors {
            if neighbor.distance <= max_distance {
                unique_neighbors
                    .entry(neighbor.node_id.clone())
                    .or_insert(neighbor);
            }
        }

        let mut result: Vec<_> = unique_neighbors.into_values().collect();
        result.sort_by(|a, b| a.distance.partial_cmp(&b.distance).expect("distance comparison should be valid"));
        result.truncate(max_count);

        Ok(result)
    }

    /// Send heartbeat to all connected nodes
    pub async fn send_heartbeat(&self) -> Result<()> {
        let message = MatrixMessage::Heartbeat {
            coordinate: self.local_coordinate,
            timestamp: Self::current_timestamp(),
        };

        let data = serde_json::to_vec(&message)?;
        let nodes = self.connected_nodes.read().await;

        for (node_id, node_conn) in nodes.iter() {
            match node_conn.connection.open_stream().await {
                Ok(mut stream) => {
                    if let Err(e) = stream.send(&data).await {
                        debug!("Failed to send heartbeat to {}: {}", node_id, e);
                    }
                }
                Err(e) => {
                    debug!("Failed to open stream for heartbeat to {}: {}", node_id, e);
                }
            }
        }

        Ok(())
    }

    /// Get all connected matrix nodes
    pub async fn get_connected_nodes(&self) -> Vec<MatrixNodeInfo> {
        let nodes = self.connected_nodes.read().await;

        nodes
            .values()
            .map(|node| MatrixNodeInfo {
                coordinate: node.coordinate,
                node_id: node.node_id.clone(),
                address: String::new(), // Address not stored in connection
                privacy_mode: format!("{:?}", self.privacy_mode),
                distance: self.local_coordinate.euclidean_distance(&node.coordinate),
            })
            .collect()
    }

    /// Clean up stale connections
    pub async fn cleanup_stale_connections(&self) -> Result<()> {
        let mut nodes = self.connected_nodes.write().await;
        let now = Self::current_timestamp();
        let stale_timeout = 60; // 60 seconds

        let stale_nodes: Vec<_> = nodes
            .iter()
            .filter(|(_, conn)| now - conn.last_heartbeat > stale_timeout)
            .map(|(id, _)| id.clone())
            .collect();

        for node_id in stale_nodes {
            info!("Removing stale connection to node {}", node_id);
            nodes.remove(&node_id);
        }

        Ok(())
    }

    /// Get the local IPv6 address that the STOQ transport is bound to.
    ///
    /// Returns the configured bind address. When the transport is bound
    /// to UNSPECIFIED (`::`) and a `public_ipv6` is configured, the
    /// public address is returned instead.
    pub fn get_local_addr(&self) -> Option<std::net::Ipv6Addr> {
        let bind = self.transport.bind_address();
        if bind.is_unspecified() {
            // Prefer the explicitly configured public address
            self.transport.public_ipv6().or(Some(bind))
        } else {
            Some(bind)
        }
    }

    /// Build a serialized reflector status announcement.
    ///
    /// The announcement advertises this node as a reflector for the
    /// given `network_scope` at the specified `block_height`. It is
    /// encoded as a JSON `MatrixMessage::ReflectorHeartbeat` and
    /// returned as raw bytes ready for STOQ transmission.
    pub fn announce_reflector_status(
        &self,
        network_scope: &str,
        block_height: u64,
    ) -> Vec<u8> {
        let msg = MatrixMessage::ReflectorHeartbeat {
            network_id: network_scope.to_string(),
            block_height,
            health_score: 1.0, // self-reported as healthy
        };

        serde_json::to_vec(&msg).unwrap_or_default()
    }

    /// Get current timestamp in seconds
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after UNIX epoch")
            .as_secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stoq::TransportConfig;

    #[test]
    fn test_matrix_message_serialization() {
        let coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");

        let announcement = MatrixNodeAnnouncement {
            coordinate: coord,
            node_id: "test_node".to_string(),
            privacy_mode: "Public".to_string(),
            protocol_version: MATRIX_PROTOCOL_VERSION.to_string(),
            pos_token: None,
            services: vec!["matrix".to_string()],
        };

        let message = MatrixMessage::Announcement(announcement);

        // Serialize
        let json = serde_json::to_string(&message).expect("test: serialization");

        // Deserialize
        let deserialized: MatrixMessage = serde_json::from_str(&json).expect("test: deserialization");

        match deserialized {
            MatrixMessage::Announcement(ann) => {
                assert_eq!(ann.coordinate, coord);
                assert_eq!(ann.node_id, "test_node");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_neighbor_discovery_message() {
        let coord = MatrixCoordinate::new(5, 10, 15).expect("test: valid coordinate");

        let request = NeighborDiscoveryRequest {
            from_coordinate: coord,
            max_distance: 100.0,
            max_neighbors: 10,
        };

        let message = MatrixMessage::DiscoveryRequest(request);
        let json = serde_json::to_string(&message).expect("test: serialization");

        // Should deserialize correctly
        let _deserialized: MatrixMessage = serde_json::from_str(&json).expect("test: deserialization");
    }

    #[test]
    fn test_announce_reflector_status_serialization() {
        // Build a minimal MatrixStoqIntegration-like scenario by
        // directly testing the announcement message format.
        let msg = MatrixMessage::ReflectorHeartbeat {
            network_id: "test-network".to_string(),
            block_height: 42,
            health_score: 1.0,
        };
        let bytes = serde_json::to_vec(&msg).expect("test: serialize");
        assert!(!bytes.is_empty());

        let parsed: MatrixMessage =
            serde_json::from_slice(&bytes).expect("test: deserialize");
        match parsed {
            MatrixMessage::ReflectorHeartbeat {
                network_id,
                block_height,
                health_score,
            } => {
                assert_eq!(network_id, "test-network");
                assert_eq!(block_height, 42);
                assert!((health_score - 1.0).abs() < f64::EPSILON);
            }
            _ => assert!(false, "Expected ReflectorHeartbeat variant"),
        }
    }

    #[tokio::test]
    async fn test_stoq_integration_creation() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let config = TransportConfig {
            port: 0, // Use OS-assigned port to avoid conflicts
            bind_address: std::net::Ipv6Addr::LOCALHOST,
            ..TransportConfig::default()
        };
        let transport = match StoqTransport::new(config).await {
            Ok(t) => Arc::new(t),
            Err(_) => return, // Skip if socket binding fails in CI
        };

        let integration = MatrixStoqIntegration::new(
            coord,
            "test_node".to_string(),
            transport,
            PrivacyMode::PRIVATE,
        )
        .await
        .expect("test: expected success");

        assert_eq!(integration.local_coordinate, coord);
        assert_eq!(integration.node_id, "test_node");
    }

    #[test]
    fn test_key_share_distribute_serialization() {
        let msg = MatrixMessage::KeyShareDistribute {
            share_index: 3,
            share_data: vec![1, 2, 3, 4],
            fingerprint: [0xAA; 32],
        };
        let bytes = serde_json::to_vec(&msg).expect("test: serialize");
        let decoded: MatrixMessage = serde_json::from_slice(&bytes).expect("test: deserialize");
        match decoded {
            MatrixMessage::KeyShareDistribute {
                share_index,
                share_data,
                fingerprint,
            } => {
                assert_eq!(share_index, 3);
                assert_eq!(share_data, vec![1, 2, 3, 4]);
                assert_eq!(fingerprint, [0xAA; 32]);
            }
            _ => unreachable!("Expected KeyShareDistribute variant"),
        }
    }

    #[test]
    fn test_threshold_sign_request_serialization() {
        let msg = MatrixMessage::ThresholdSignRequest {
            request_id: "req-001".to_string(),
            message_hash: [0xBB; 32],
            message: vec![10, 20, 30],
            ca_fingerprint: [0xCC; 32],
        };
        let bytes = serde_json::to_vec(&msg).expect("test: serialize");
        let decoded: MatrixMessage = serde_json::from_slice(&bytes).expect("test: deserialize");
        match decoded {
            MatrixMessage::ThresholdSignRequest {
                request_id,
                message_hash,
                message,
                ca_fingerprint,
            } => {
                assert_eq!(request_id, "req-001");
                assert_eq!(message_hash, [0xBB; 32]);
                assert_eq!(message, vec![10, 20, 30]);
                assert_eq!(ca_fingerprint, [0xCC; 32]);
            }
            _ => unreachable!("Expected ThresholdSignRequest variant"),
        }
    }

    #[test]
    fn test_threshold_sign_response_serialization() {
        let msg = MatrixMessage::ThresholdSignResponse {
            request_id: "req-001".to_string(),
            share_data: vec![99, 100, 101],
            accepted: true,
        };
        let bytes = serde_json::to_vec(&msg).expect("test: serialize");
        let decoded: MatrixMessage = serde_json::from_slice(&bytes).expect("test: deserialize");
        match decoded {
            MatrixMessage::ThresholdSignResponse {
                request_id,
                share_data,
                accepted,
            } => {
                assert_eq!(request_id, "req-001");
                assert_eq!(share_data, vec![99, 100, 101]);
                assert!(accepted);
            }
            _ => unreachable!("Expected ThresholdSignResponse variant"),
        }
    }

    #[test]
    fn test_key_rotation_message_serialization() {
        let msg = MatrixMessage::KeyRotation {
            old_key_fingerprint: "abcd1234".to_string(),
            new_key_fingerprint: "efgh5678".to_string(),
            reason: "Scheduled".to_string(),
            rotation_proof: vec![1, 2, 3, 4, 5],
            block_index: 42,
            timestamp: 1700000000,
        };
        let bytes = serde_json::to_vec(&msg).expect("test: serialize");
        let decoded: MatrixMessage =
            serde_json::from_slice(&bytes).expect("test: deserialize");
        match decoded {
            MatrixMessage::KeyRotation {
                old_key_fingerprint,
                new_key_fingerprint,
                reason,
                rotation_proof,
                block_index,
                timestamp,
            } => {
                assert_eq!(old_key_fingerprint, "abcd1234");
                assert_eq!(new_key_fingerprint, "efgh5678");
                assert_eq!(reason, "Scheduled");
                assert_eq!(rotation_proof, vec![1, 2, 3, 4, 5]);
                assert_eq!(block_index, 42);
                assert_eq!(timestamp, 1700000000);
            }
            _ => unreachable!("Expected KeyRotation variant"),
        }
    }

    #[test]
    fn test_threshold_sign_response_rejected() {
        let msg = MatrixMessage::ThresholdSignResponse {
            request_id: "req-002".to_string(),
            share_data: Vec::new(),
            accepted: false,
        };
        let bytes = serde_json::to_vec(&msg).expect("test: serialize");
        let decoded: MatrixMessage = serde_json::from_slice(&bytes).expect("test: deserialize");
        match decoded {
            MatrixMessage::ThresholdSignResponse {
                request_id,
                accepted,
                share_data,
            } => {
                assert_eq!(request_id, "req-002");
                assert!(!accepted);
                assert!(share_data.is_empty());
            }
            _ => unreachable!("Expected ThresholdSignResponse variant"),
        }
    }
}
