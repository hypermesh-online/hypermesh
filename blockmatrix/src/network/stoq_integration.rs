// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Integration Layer for Matrix Foundation
//!
//! This module provides the integration between the Matrix Foundation and STOQ transport layer.
//! All matrix node communication goes through STOQ for secure, efficient transport.

use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::net::SocketAddr;

use crate::matrix::coordinate::MatrixCoordinate;
use crate::bootstrap::PrivacyMode;
use stoq::{StoqTransport, Connection, Endpoint};

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
    PositionBroadcast { coordinate: MatrixCoordinate, node_id: String },
    /// Heartbeat/keepalive
    Heartbeat { coordinate: MatrixCoordinate, timestamp: u64 },
    /// Error message
    Error { message: String },
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
        if let Err(e) = Self::register_service_discovery(&transport, &local_coordinate, &node_id).await {
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

        debug!("Registering matrix node with STOQ service discovery: {}", metadata);

        // TODO: Call STOQ service discovery registration when API is available
        // transport.register_service(MATRIX_SERVICE_TAG, metadata).await?;

        Ok(())
    }

    /// Connect to a matrix node via STOQ
    pub async fn connect_to_node(&self, address: SocketAddr) -> Result<String> {
        info!("Connecting to matrix node at {} via STOQ", address);

        // Extract IPv6 address
        let ipv6_addr = match address {
            SocketAddr::V6(v6) => *v6.ip(),
            _ => return Err(anyhow!("Matrix nodes require IPv6 addresses")),
        };

        // Create STOQ endpoint
        let endpoint = Endpoint::new(ipv6_addr, address.port());

        // Connect via STOQ transport
        let connection = self.transport.connect(&endpoint).await?;

        // Exchange matrix node information
        let peer_info = self.exchange_node_info(&connection).await?;

        // Store the connection
        let mut nodes = self.connected_nodes.write().await;
        nodes.insert(peer_info.node_id.clone(), MatrixNodeConnection {
            coordinate: peer_info.coordinate,
            node_id: peer_info.node_id.clone(),
            connection: connection.clone(),
            last_heartbeat: Self::current_timestamp(),
        });

        info!(
            "Successfully connected to matrix node {} at ({},{},{})",
            peer_info.node_id,
            peer_info.coordinate.x,
            peer_info.coordinate.y,
            peer_info.coordinate.z
        );

        Ok(peer_info.node_id)
    }

    /// Exchange node information with peer
    async fn exchange_node_info(&self, connection: &Arc<Connection>) -> Result<MatrixNodeInfo> {
        // Open a bidirectional stream
        let mut stream = connection.open_stream().await?;

        // Create our announcement
        let announcement = MatrixNodeAnnouncement {
            coordinate: self.local_coordinate,
            node_id: self.node_id.clone(),
            privacy_mode: format!("{:?}", self.privacy_mode),
            protocol_version: MATRIX_PROTOCOL_VERSION.to_string(),
            pos_token: None, // TODO: Add PoS token when available
            services: vec!["matrix".to_string()],
        };

        let message = MatrixMessage::Announcement(announcement);
        let data = serde_json::to_vec(&message)?;

        // Send our announcement
        stream.send(&data).await?;

        // Receive peer announcement
        let peer_data = stream.receive().await?;
        let peer_message: MatrixMessage = serde_json::from_slice(&peer_data)?;

        match peer_message {
            MatrixMessage::Announcement(peer_announcement) => {
                Ok(MatrixNodeInfo {
                    coordinate: peer_announcement.coordinate,
                    node_id: peer_announcement.node_id,
                    address: connection.endpoint().to_socket_addr().to_string(),
                    privacy_mode: peer_announcement.privacy_mode,
                    distance: self.local_coordinate.euclidean_distance(&peer_announcement.coordinate),
                })
            }
            _ => Err(anyhow!("Expected announcement message from peer")),
        }
    }

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
    pub async fn discover_neighbors(&self, max_distance: f64, max_count: usize) -> Result<Vec<MatrixNodeInfo>> {
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
                unique_neighbors.entry(neighbor.node_id.clone())
                    .or_insert(neighbor);
            }
        }

        let mut result: Vec<_> = unique_neighbors.into_values().collect();
        result.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
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

    /// Handle incoming STOQ connections
    pub async fn handle_incoming_connection(&self, connection: Arc<Connection>) -> Result<()> {
        debug!("Handling incoming STOQ connection");

        // Accept stream and exchange info
        let mut stream = connection.accept_stream().await?;

        // Receive peer announcement
        let peer_data = stream.receive().await?;
        let peer_message: MatrixMessage = serde_json::from_slice(&peer_data)?;

        match peer_message {
            MatrixMessage::Announcement(peer_announcement) => {
                // Send our announcement back
                let our_announcement = MatrixNodeAnnouncement {
                    coordinate: self.local_coordinate,
                    node_id: self.node_id.clone(),
                    privacy_mode: format!("{:?}", self.privacy_mode),
                    protocol_version: MATRIX_PROTOCOL_VERSION.to_string(),
                    pos_token: None,
                    services: vec!["matrix".to_string()],
                };

                let response = MatrixMessage::Announcement(our_announcement);
                let response_data = serde_json::to_vec(&response)?;
                stream.send(&response_data).await?;

                // Store the connection
                let mut nodes = self.connected_nodes.write().await;
                nodes.insert(peer_announcement.node_id.clone(), MatrixNodeConnection {
                    coordinate: peer_announcement.coordinate,
                    node_id: peer_announcement.node_id.clone(),
                    connection,
                    last_heartbeat: Self::current_timestamp(),
                });

                info!(
                    "Accepted incoming connection from matrix node {} at ({},{},{})",
                    peer_announcement.node_id,
                    peer_announcement.coordinate.x,
                    peer_announcement.coordinate.y,
                    peer_announcement.coordinate.z
                );
            }
            _ => {
                warn!("Unexpected initial message from incoming connection");
                return Err(anyhow!("Expected announcement message"));
            }
        }

        Ok(())
    }

    /// Get all connected matrix nodes
    pub async fn get_connected_nodes(&self) -> Vec<MatrixNodeInfo> {
        let nodes = self.connected_nodes.read().await;

        nodes.values().map(|node| MatrixNodeInfo {
            coordinate: node.coordinate,
            node_id: node.node_id.clone(),
            address: String::new(), // Address not stored in connection
            privacy_mode: format!("{:?}", self.privacy_mode),
            distance: self.local_coordinate.euclidean_distance(&node.coordinate),
        }).collect()
    }

    /// Clean up stale connections
    pub async fn cleanup_stale_connections(&self) -> Result<()> {
        let mut nodes = self.connected_nodes.write().await;
        let now = Self::current_timestamp();
        let stale_timeout = 60; // 60 seconds

        let stale_nodes: Vec<_> = nodes.iter()
            .filter(|(_, conn)| now - conn.last_heartbeat > stale_timeout)
            .map(|(id, _)| id.clone())
            .collect();

        for node_id in stale_nodes {
            info!("Removing stale connection to node {}", node_id);
            nodes.remove(&node_id);
        }

        Ok(())
    }

    /// Get current timestamp in seconds
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stoq::TransportConfig;

    #[test]
    fn test_matrix_message_serialization() {
        let coord = MatrixCoordinate::new(10, 20, 30).unwrap();

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
        let json = serde_json::to_string(&message).unwrap();

        // Deserialize
        let deserialized: MatrixMessage = serde_json::from_str(&json).unwrap();

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
        let coord = MatrixCoordinate::new(5, 10, 15).unwrap();

        let request = NeighborDiscoveryRequest {
            from_coordinate: coord,
            max_distance: 100.0,
            max_neighbors: 10,
        };

        let message = MatrixMessage::DiscoveryRequest(request);
        let json = serde_json::to_string(&message).unwrap();

        // Should deserialize correctly
        let _deserialized: MatrixMessage = serde_json::from_str(&json).unwrap();
    }

    #[tokio::test]
    async fn test_stoq_integration_creation() {
        let coord = MatrixCoordinate::new(0, 0, 0).unwrap();
        let config = TransportConfig::default();
        let transport = Arc::new(StoqTransport::new(config).await.unwrap());

        let integration = MatrixStoqIntegration::new(
            coord,
            "test_node".to_string(),
            transport,
            PrivacyMode::PRIVATE,
        ).await.unwrap();

        assert_eq!(integration.local_coordinate, coord);
        assert_eq!(integration.node_id, "test_node");
    }
}