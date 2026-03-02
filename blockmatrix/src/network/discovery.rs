// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! mDNS-based peer discovery for local network.
//!
//! Announces this node on `_hypermesh._udp.local` via multicast and listens
//! for peer announcements. Uses `socket2` for multicast group management.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{Ipv6Addr, SocketAddrV6};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::matrix::coordinate::MatrixCoordinate;

/// mDNS multicast address (link-local all-nodes for IPv6)
const MDNS_MULTICAST_ADDR: Ipv6Addr = Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 0xfb);

/// mDNS port
const MDNS_PORT: u16 = 5353;

/// HyperMesh service name
const SERVICE_NAME: &str = "_hypermesh._udp.local";

/// Peer announcement broadcast over mDNS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerAnnouncement {
    /// Service name tag
    pub service: String,
    /// Node ID (BLAKE3 hex)
    pub node_id: String,
    /// Matrix coordinate
    pub coordinate: MatrixCoordinate,
    /// STOQ port for connection
    pub stoq_port: u16,
    /// Protocol version
    pub protocol_version: String,
    /// Timestamp of announcement
    pub timestamp: u64,
}

/// Discovered peer from mDNS
#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    /// Peer announcement data
    pub announcement: PeerAnnouncement,
    /// Source IPv6 address
    pub source_addr: Ipv6Addr,
    /// When this peer was last seen
    pub last_seen: std::time::Instant,
}

/// mDNS discovery manager
pub struct MdnsDiscovery {
    /// Local node ID
    node_id: String,
    /// Local matrix coordinate
    coordinate: MatrixCoordinate,
    /// STOQ port to advertise
    stoq_port: u16,
    /// Discovered peers
    peers: Arc<RwLock<HashMap<String, DiscoveredPeer>>>,
    /// Whether discovery is running
    running: Arc<RwLock<bool>>,
}

impl MdnsDiscovery {
    /// Create a new mDNS discovery instance.
    pub fn new(node_id: String, coordinate: MatrixCoordinate, stoq_port: u16) -> Self {
        Self {
            node_id,
            coordinate,
            stoq_port,
            peers: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start mDNS discovery (announce + listen).
    ///
    /// Spawns background tasks for periodic announcement and listening.
    /// Returns immediately after spawning.
    pub async fn start(&self) -> Result<()> {
        {
            let mut running = self.running.write().await;
            if *running {
                return Ok(());
            }
            *running = true;
        }

        info!(
            "Starting mDNS discovery on {} for node {}",
            SERVICE_NAME, self.node_id
        );

        // Create the multicast socket
        let socket = Self::create_multicast_socket()?;
        let socket = Arc::new(tokio::net::UdpSocket::from_std(socket.into())?);

        // Spawn announcement task
        let announce_socket = socket.clone();
        let announcement = self.build_announcement();
        let running_flag = self.running.clone();

        tokio::spawn(async move {
            Self::announcement_loop(announce_socket, announcement, running_flag).await;
        });

        // Spawn listener task
        let listen_socket = socket;
        let peers = self.peers.clone();
        let local_node_id = self.node_id.clone();
        let running_flag = self.running.clone();

        tokio::spawn(async move {
            Self::listen_loop(listen_socket, peers, local_node_id, running_flag).await;
        });

        Ok(())
    }

    /// Stop mDNS discovery.
    pub async fn stop(&self) {
        *self.running.write().await = false;
        info!("Stopped mDNS discovery for node {}", self.node_id);
    }

    /// Get all currently discovered peers.
    pub async fn get_peers(&self) -> Vec<DiscoveredPeer> {
        self.peers.read().await.values().cloned().collect()
    }

    /// Get peer count.
    pub async fn peer_count(&self) -> usize {
        self.peers.read().await.len()
    }

    /// Create a UDP socket joined to the mDNS multicast group.
    fn create_multicast_socket() -> Result<std::net::UdpSocket> {
        use socket2::{Domain, Protocol, Socket, Type};

        let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))
            .map_err(|e| anyhow!("failed to create mDNS socket: {e}"))?;

        socket.set_reuse_address(true).ok();

        // Allow port reuse on platforms that support it
        #[cfg(not(target_os = "windows"))]
        socket.set_reuse_port(true).ok();

        socket.set_nonblocking(true)?;

        let bind_addr = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, MDNS_PORT, 0, 0);
        socket
            .bind(&bind_addr.into())
            .map_err(|e| anyhow!("failed to bind mDNS socket to port {MDNS_PORT}: {e}"))?;

        // Join multicast group on all interfaces (interface index 0)
        socket
            .join_multicast_v6(&MDNS_MULTICAST_ADDR, 0)
            .map_err(|e| anyhow!("failed to join mDNS multicast group: {e}"))?;

        Ok(socket.into())
    }

    /// Build an announcement message for this node.
    fn build_announcement(&self) -> PeerAnnouncement {
        PeerAnnouncement {
            service: SERVICE_NAME.to_string(),
            node_id: self.node_id.clone(),
            coordinate: self.coordinate,
            stoq_port: self.stoq_port,
            protocol_version: "1.0.0".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }

    /// Periodically send announcement to multicast group.
    async fn announcement_loop(
        socket: Arc<tokio::net::UdpSocket>,
        mut announcement: PeerAnnouncement,
        running: Arc<RwLock<bool>>,
    ) {
        let target = SocketAddrV6::new(MDNS_MULTICAST_ADDR, MDNS_PORT, 0, 0);

        loop {
            if !*running.read().await {
                break;
            }

            // Update timestamp
            announcement.timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            if let Ok(data) = serde_json::to_vec(&announcement) {
                if let Err(e) = socket.send_to(&data, target).await {
                    debug!("mDNS announcement send failed: {e}");
                }
            }

            // Announce every 10 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    }

    /// Listen for peer announcements on the multicast group.
    async fn listen_loop(
        socket: Arc<tokio::net::UdpSocket>,
        peers: Arc<RwLock<HashMap<String, DiscoveredPeer>>>,
        local_node_id: String,
        running: Arc<RwLock<bool>>,
    ) {
        let mut buf = vec![0u8; 4096];

        loop {
            if !*running.read().await {
                break;
            }

            // Use timeout to allow checking the running flag
            let recv_result = tokio::time::timeout(
                tokio::time::Duration::from_secs(5),
                socket.recv_from(&mut buf),
            )
            .await;

            match recv_result {
                Ok(Ok((len, src_addr))) => {
                    if let Ok(announcement) =
                        serde_json::from_slice::<PeerAnnouncement>(&buf[..len])
                    {
                        // Skip our own announcements
                        if announcement.node_id == local_node_id {
                            continue;
                        }

                        // Validate service name
                        if announcement.service != SERVICE_NAME {
                            continue;
                        }

                        let source_ip = match src_addr {
                            std::net::SocketAddr::V6(v6) => *v6.ip(),
                            _ => continue,
                        };

                        debug!(
                            "mDNS: discovered peer {} at [{source_ip}]:{}",
                            announcement.node_id, announcement.stoq_port
                        );

                        let peer = DiscoveredPeer {
                            announcement: announcement.clone(),
                            source_addr: source_ip,
                            last_seen: std::time::Instant::now(),
                        };

                        peers.write().await.insert(announcement.node_id, peer);
                    }
                }
                Ok(Err(e)) => {
                    debug!("mDNS recv error: {e}");
                }
                Err(_) => {
                    // Timeout — loop continues
                }
            }
        }
    }

    /// Remove peers not seen in the last `stale_secs` seconds.
    pub async fn prune_stale_peers(&self, stale_secs: u64) {
        let cutoff = std::time::Duration::from_secs(stale_secs);
        let mut peers = self.peers.write().await;
        peers.retain(|_, peer| peer.last_seen.elapsed() < cutoff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_announcement_serialization() {
        let coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
        let announcement = PeerAnnouncement {
            service: SERVICE_NAME.to_string(),
            node_id: "abc123".to_string(),
            coordinate: coord,
            stoq_port: 9292,
            protocol_version: "1.0.0".to_string(),
            timestamp: 1700000000,
        };

        let json = serde_json::to_string(&announcement).expect("test: serialize");
        let decoded: PeerAnnouncement = serde_json::from_str(&json).expect("test: deserialize");

        assert_eq!(decoded.node_id, "abc123");
        assert_eq!(decoded.stoq_port, 9292);
        assert_eq!(decoded.coordinate, coord);
    }

    #[tokio::test]
    async fn test_mdns_discovery_creation() {
        let coord = MatrixCoordinate::new(5, 10, 15).expect("test: valid coordinate");
        let discovery = MdnsDiscovery::new("test-node".to_string(), coord, 9292);

        assert_eq!(discovery.peer_count().await, 0);
        assert!(discovery.get_peers().await.is_empty());
    }

    #[tokio::test]
    async fn test_mdns_discovery_prune() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let discovery = MdnsDiscovery::new("local".to_string(), coord, 9292);

        // Insert a peer manually
        {
            let peer = DiscoveredPeer {
                announcement: PeerAnnouncement {
                    service: SERVICE_NAME.to_string(),
                    node_id: "peer1".to_string(),
                    coordinate: coord,
                    stoq_port: 9293,
                    protocol_version: "1.0.0".to_string(),
                    timestamp: 0,
                },
                source_addr: Ipv6Addr::LOCALHOST,
                last_seen: std::time::Instant::now()
                    - std::time::Duration::from_secs(100),
            };
            discovery
                .peers
                .write()
                .await
                .insert("peer1".to_string(), peer);
        }

        assert_eq!(discovery.peer_count().await, 1);

        // Prune with 50-second threshold should remove the stale peer
        discovery.prune_stale_peers(50).await;
        assert_eq!(discovery.peer_count().await, 0);
    }
}
