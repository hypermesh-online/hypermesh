//! Network layer for multi-node communication
//!
//! This module provides the actual networking implementation for Block-MATRIX nodes
//! to discover and communicate with each other using STOQ transport.

pub mod trust;
pub mod isolation;
pub mod multi_network;
pub mod config;

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::net::{SocketAddr, Ipv6Addr};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, debug, error};

use crate::matrix::coordinate::MatrixCoordinate;
use serde::{Serialize, Deserialize};
use crate::matrix::neighbors::{find_k_nearest, find_neighbors};
use crate::bootstrap::PrivacyMode;

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
}

impl NetworkManager {
    /// Create new network manager
    pub async fn new(
        local_coordinate: MatrixCoordinate,
        transport: Arc<stoq::StoqTransport>,
        privacy_mode: PrivacyMode,
        bootstrap_nodes: Vec<SocketAddr>,
    ) -> Result<Self> {
        info!(
            "Initializing network manager at ({},{},{}) in {:?} mode",
            local_coordinate.x, local_coordinate.y, local_coordinate.z, privacy_mode
        );

        Ok(Self {
            local_coordinate,
            transport,
            nodes: Arc::new(RwLock::new(HashMap::new())),
            bootstrap_nodes,
            privacy_mode: Arc::new(RwLock::new(privacy_mode)),
        })
    }

    /// Start network discovery based on privacy mode
    pub async fn start_discovery(&self) -> Result<()> {
        let mode = *self.privacy_mode.read().await;

        match mode {
            PrivacyMode::Private => {
                info!("Private mode: No network discovery (localhost only)");
                Ok(())
            }
            PrivacyMode::Anonymous => {
                info!("Anonymous mode: Starting ephemeral discovery");
                self.discover_ephemeral_peers().await
            }
            PrivacyMode::P2P => {
                info!("P2P mode: Starting peer discovery");
                self.discover_peers().await
            }
            PrivacyMode::Public => {
                info!("Public mode: Joining network with full discovery");
                self.join_network().await
            }
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
                warn!("Failed to connect to bootstrap node {}: {}", bootstrap_addr, e);
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
                    info!("Connected to bootstrap node {} ({})", bootstrap_addr, node_id);
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

        // Create endpoint for connection
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

        info!("Successfully connected to node {} at ({},{},{})",
            node_info.node_id,
            node_info.coordinate.x,
            node_info.coordinate.y,
            node_info.coordinate.z
        );

        Ok(node_info.node_id)
    }

    /// Exchange node information with peer
    async fn exchange_node_info(&self, connection: &Arc<stoq::Connection>) -> Result<NetworkNode> {
        // Open a stream for handshake
        let mut stream = connection.open_stream().await?;

        // Send our node info
        let our_info = serde_json::json!({
            "coordinate": {
                "x": self.local_coordinate.x,
                "y": self.local_coordinate.y,
                "z": self.local_coordinate.z,
            },
            "node_id": self.get_node_id(),
            "privacy_mode": format!("{:?}", *self.privacy_mode.read().await),
        });

        stream.send(our_info.to_string().as_bytes()).await?;

        // Receive peer info
        let peer_data = stream.receive().await?;
        let peer_info: serde_json::Value = serde_json::from_slice(&peer_data)?;

        // Parse peer information
        let coordinate = MatrixCoordinate::new(
            peer_info["coordinate"]["x"].as_i64().unwrap_or(0),
            peer_info["coordinate"]["y"].as_i64().unwrap_or(0),
            peer_info["coordinate"]["z"].as_i64().unwrap_or(0),
        )?;

        let node_id = peer_info["node_id"].as_str()
            .ok_or_else(|| anyhow!("Missing node_id"))?
            .to_string();

        let privacy_mode = match peer_info["privacy_mode"].as_str().unwrap_or("Private") {
            "Anonymous" => PrivacyMode::Anonymous,
            "P2P" => PrivacyMode::P2P,
            "Public" => PrivacyMode::Public,
            _ => PrivacyMode::Private,
        };

        Ok(NetworkNode {
            coordinate,
            address: connection.endpoint().to_socket_addr(),
            node_id,
            privacy_mode,
            connection: Some(connection.clone()),
        })
    }

    /// Start mDNS discovery for local network
    async fn start_mdns_discovery(&self) -> Result<()> {
        // TODO: Implement actual mDNS discovery
        // For now, this is a placeholder
        debug!("mDNS discovery would start here");
        Ok(())
    }

    /// Start gossip protocol for network discovery
    async fn start_gossip_protocol(&self) -> Result<()> {
        // TODO: Implement gossip protocol
        // For now, this is a placeholder
        debug!("Gossip protocol would start here");
        Ok(())
    }

    /// Find neighbors in the matrix topology
    pub async fn find_matrix_neighbors(&self, radius: f64) -> Vec<NetworkNode> {
        let nodes = self.nodes.read().await;

        let candidates: Vec<MatrixCoordinate> = nodes.values()
            .map(|n| n.coordinate)
            .collect();

        let neighbors = find_neighbors(&self.local_coordinate, &candidates, radius);

        nodes.values()
            .filter(|n| neighbors.contains(&n.coordinate))
            .cloned()
            .collect()
    }

    /// Find K nearest neighbors
    pub async fn find_k_nearest_nodes(&self, k: usize) -> Vec<(NetworkNode, f64)> {
        let nodes = self.nodes.read().await;

        let candidates: Vec<MatrixCoordinate> = nodes.values()
            .map(|n| n.coordinate)
            .collect();

        let nearest = find_k_nearest(&self.local_coordinate, &candidates, k);

        nearest.into_iter()
            .filter_map(|(coord, dist)| {
                nodes.values()
                    .find(|n| n.coordinate == coord)
                    .map(|n| (n.clone(), dist))
            })
            .collect()
    }

    /// Get all connected nodes
    pub async fn get_connected_nodes(&self) -> Vec<NetworkNode> {
        self.nodes.read().await.values().cloned().collect()
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
        // Use coordinate hash as node ID for now
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(&self.local_coordinate.x.to_le_bytes());
        hasher.update(&self.local_coordinate.y.to_le_bytes());
        hasher.update(&self.local_coordinate.z.to_le_bytes());
        hasher.finalize().to_hex().to_string()
    }

    /// Accept incoming connections
    pub async fn accept_connections(&self) -> Result<()> {
        info!("Starting to accept incoming connections");

        loop {
            match self.transport.accept().await {
                Ok(connection) => {
                    let nodes = self.nodes.clone();
                    let local_coord = self.local_coordinate;
                    let privacy_mode = self.privacy_mode.clone();

                    // Handle connection in background
                    tokio::spawn(async move {
                        debug!("Accepted incoming connection");

                        // Exchange node info
                        if let Ok(mut stream) = connection.accept_stream().await {
                            // Receive peer info
                            if let Ok(peer_data) = stream.receive().await {
                                if let Ok(peer_info) = serde_json::from_slice::<serde_json::Value>(&peer_data) {
                                    // Send our info
                                    let node_id = {
                                        let mut hasher = blake3::Hasher::new();
                                        hasher.update(&local_coord.x.to_le_bytes());
                                        hasher.update(&local_coord.y.to_le_bytes());
                                        hasher.update(&local_coord.z.to_le_bytes());
                                        hasher.finalize().to_hex().to_string()
                                    };
                                    let privacy_str = format!("{:?}", *privacy_mode.read().await);

                                    let our_info = serde_json::json!({
                                        "coordinate": {
                                            "x": local_coord.x,
                                            "y": local_coord.y,
                                            "z": local_coord.z,
                                        },
                                        "node_id": node_id,
                                        "privacy_mode": privacy_str,
                                    });

                                    if let Ok(_) = stream.send(our_info.to_string().as_bytes()).await {
                                        // Parse and store peer
                                        if let (Some(x), Some(y), Some(z)) = (
                                            peer_info["coordinate"]["x"].as_i64(),
                                            peer_info["coordinate"]["y"].as_i64(),
                                            peer_info["coordinate"]["z"].as_i64(),
                                        ) {
                                            if let Ok(coordinate) = MatrixCoordinate::new(x, y, z) {
                                                let node_id = peer_info["node_id"].as_str()
                                                    .unwrap_or("unknown")
                                                    .to_string();

                                                let node = NetworkNode {
                                                    coordinate,
                                                    address: connection.endpoint().to_socket_addr(),
                                                    node_id: node_id.clone(),
                                                    privacy_mode: PrivacyMode::Public, // Default
                                                    connection: Some(connection),
                                                };

                                                nodes.write().await.insert(node_id, node);
                                                info!("Added incoming node to network");
                                            }
                                        }
                                    }
                                }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_manager_creation() {
        let coord = MatrixCoordinate::new(0, 0, 0).unwrap();
        let config = stoq::TransportConfig::default();
        let transport = Arc::new(stoq::StoqTransport::new(config).await.unwrap());

        let manager = NetworkManager::new(
            coord,
            transport,
            PrivacyMode::Private,
            vec![],
        ).await.unwrap();

        assert_eq!(manager.get_node_count().await, 0);
    }
}