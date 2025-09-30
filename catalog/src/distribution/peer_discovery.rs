//! Peer Discovery Service for P2P Distribution
//!
//! Implements multiple discovery mechanisms for finding peers in the network

use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use std::net::{SocketAddr, Ipv6Addr};
use std::time::{Duration, SystemTime};

use super::{
    stoq_transport::StoqTransportLayer,
    dht::{DhtNetwork, NodeId, NodeInfo},
};

/// Peer discovery service
pub struct PeerDiscovery {
    /// Transport layer
    transport: Arc<StoqTransportLayer>,
    /// DHT network
    dht: Arc<DhtNetwork>,
    /// Known peers
    known_peers: Arc<RwLock<PeerRegistry>>,
    /// Discovery configuration
    config: DiscoveryConfig,
    /// mDNS discovery (for local network)
    mdns: Option<Arc<MdnsDiscovery>>,
    /// Bootstrap nodes
    bootstrap_nodes: Vec<SocketAddr>,
}

/// Discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Enable mDNS discovery
    pub enable_mdns: bool,
    /// Enable DHT discovery
    pub enable_dht: bool,
    /// Enable bootstrap nodes
    pub enable_bootstrap: bool,
    /// Discovery interval
    pub discovery_interval: Duration,
    /// Peer expiration time
    pub peer_ttl: Duration,
    /// Maximum peers to maintain
    pub max_peers: usize,
    /// Minimum peers to maintain
    pub min_peers: usize,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            enable_mdns: true,
            enable_dht: true,
            enable_bootstrap: true,
            discovery_interval: Duration::from_secs(30),
            peer_ttl: Duration::from_secs(300),
            max_peers: 100,
            min_peers: 10,
        }
    }
}

/// Peer registry for managing known peers
struct PeerRegistry {
    /// Known peers by node ID
    peers: HashMap<NodeId, PeerInfo>,
    /// Peers by capability
    peers_by_capability: HashMap<PeerCapability, HashSet<NodeId>>,
    /// Connected peers
    connected: HashSet<NodeId>,
    /// Blacklisted peers
    blacklist: HashSet<NodeId>,
}

/// Information about a peer
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Node ID
    pub id: NodeId,
    /// Network addresses
    pub addresses: Vec<SocketAddr>,
    /// Peer capabilities
    pub capabilities: HashSet<PeerCapability>,
    /// Last seen timestamp
    pub last_seen: SystemTime,
    /// Connection quality score (0.0 - 1.0)
    pub quality_score: f64,
    /// Round-trip time (ms)
    pub rtt: Option<u32>,
    /// Bandwidth estimate (bytes/sec)
    pub bandwidth: Option<u64>,
    /// Discovery source
    pub discovery_source: DiscoverySource,
}

/// Peer capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PeerCapability {
    /// Can serve packages
    PackageProvider,
    /// Can relay connections
    Relay,
    /// Has DHT routing capability
    DhtNode,
    /// High bandwidth node
    HighBandwidth,
    /// Low latency node
    LowLatency,
}

/// Discovery source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoverySource {
    /// Discovered via mDNS
    Mdns,
    /// Discovered via DHT
    Dht,
    /// Bootstrap node
    Bootstrap,
    /// Manual configuration
    Manual,
    /// Peer exchange
    PeerExchange,
}

/// mDNS discovery for local network
struct MdnsDiscovery {
    /// Service name
    service_name: String,
    /// Local addresses
    local_addresses: Vec<SocketAddr>,
}

impl PeerDiscovery {
    /// Create a new peer discovery service
    pub async fn new(
        transport: Arc<StoqTransportLayer>,
        dht: Arc<DhtNetwork>,
    ) -> Result<Self> {
        let config = DiscoveryConfig::default();
        let known_peers = Arc::new(RwLock::new(PeerRegistry::new()));

        // Initialize mDNS if enabled
        let mdns = if config.enable_mdns {
            Some(Arc::new(MdnsDiscovery::new().await?))
        } else {
            None
        };

        // Default bootstrap nodes
        let bootstrap_nodes = vec![
            SocketAddr::from((Ipv6Addr::from_str("2001:db8::1").unwrap(), 8080)),
            SocketAddr::from((Ipv6Addr::from_str("2001:db8::2").unwrap(), 8080)),
        ];

        let discovery = Self {
            transport,
            dht,
            known_peers,
            config,
            mdns,
            bootstrap_nodes,
        };

        // Start discovery tasks
        discovery.start_discovery_tasks();

        Ok(discovery)
    }

    /// Start background discovery tasks
    fn start_discovery_tasks(&self) {
        let discovery = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(discovery.config.discovery_interval);
            loop {
                interval.tick().await;
                if let Err(e) = discovery.run_discovery_round().await {
                    tracing::warn!("Discovery round failed: {}", e);
                }
            }
        });

        // Peer maintenance task
        let known_peers = self.known_peers.clone();
        let peer_ttl = self.config.peer_ttl;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let mut registry = known_peers.write().await;
                registry.clean_expired_peers(peer_ttl);
            }
        });
    }

    /// Run a discovery round
    async fn run_discovery_round(&self) -> Result<()> {
        let current_peer_count = self.known_peers.read().await.connected.len();

        // If we have too few peers, actively discover more
        if current_peer_count < self.config.min_peers {
            // Try bootstrap nodes
            if self.config.enable_bootstrap {
                self.discover_from_bootstrap().await?;
            }

            // Try mDNS discovery
            if self.config.enable_mdns {
                if let Some(mdns) = &self.mdns {
                    self.discover_from_mdns(mdns).await?;
                }
            }

            // Try DHT discovery
            if self.config.enable_dht {
                self.discover_from_dht().await?;
            }

            // Try peer exchange
            self.discover_from_peer_exchange().await?;
        }

        // If we have too many peers, prune low-quality ones
        if current_peer_count > self.config.max_peers {
            self.prune_peers().await?;
        }

        Ok(())
    }

    /// Discover peers from bootstrap nodes
    async fn discover_from_bootstrap(&self) -> Result<()> {
        for addr in &self.bootstrap_nodes {
            match self.transport.connect(*addr).await {
                Ok(node_id) => {
                    let peer_info = PeerInfo {
                        id: node_id.clone(),
                        addresses: vec![*addr],
                        capabilities: HashSet::from([
                            PeerCapability::PackageProvider,
                            PeerCapability::DhtNode,
                        ]),
                        last_seen: SystemTime::now(),
                        quality_score: 1.0,
                        rtt: None,
                        bandwidth: None,
                        discovery_source: DiscoverySource::Bootstrap,
                    };

                    self.add_peer(peer_info).await?;
                }
                Err(e) => {
                    tracing::debug!("Failed to connect to bootstrap node {}: {}", addr, e);
                }
            }
        }

        Ok(())
    }

    /// Discover peers via mDNS
    async fn discover_from_mdns(&self, mdns: &MdnsDiscovery) -> Result<()> {
        let discovered = mdns.discover_peers().await?;

        for (addr, capabilities) in discovered {
            let node_id = NodeId::from_address(&addr);
            let peer_info = PeerInfo {
                id: node_id,
                addresses: vec![addr],
                capabilities,
                last_seen: SystemTime::now(),
                quality_score: 0.8, // mDNS peers are usually local, so good quality
                rtt: None,
                bandwidth: None,
                discovery_source: DiscoverySource::Mdns,
            };

            self.add_peer(peer_info).await?;
        }

        Ok(())
    }

    /// Discover peers via DHT
    async fn discover_from_dht(&self) -> Result<()> {
        // Query DHT for random nodes to discover new peers
        let random_id = NodeId::random();
        // Note: This would use the actual DHT lookup method
        // For now, we'll skip the actual implementation
        Ok(())
    }

    /// Discover peers via peer exchange
    async fn discover_from_peer_exchange(&self) -> Result<()> {
        let connected_peers = self.get_connected_peers().await;

        for peer_id in connected_peers {
            // Request peer list from connected peer
            match self.transport.send_request(
                &peer_id,
                super::stoq_transport::RequestType::GetPeers,
            ).await {
                Ok(super::stoq_transport::ResponseData::Peers(peers)) => {
                    for new_peer_id in peers {
                        // Note: We'd need to get full peer info here
                        // For now, just track that we discovered it
                        tracing::debug!("Discovered peer {} via exchange", new_peer_id);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Add a peer to the registry
    async fn add_peer(&self, peer_info: PeerInfo) -> Result<()> {
        let mut registry = self.known_peers.write().await;

        // Check if peer is blacklisted
        if registry.blacklist.contains(&peer_info.id) {
            return Ok(());
        }

        // Check if we're at max peers
        if registry.peers.len() >= self.config.max_peers {
            // Only add if better quality than existing peers
            let min_quality = registry.peers
                .values()
                .map(|p| p.quality_score)
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(0.0);

            if peer_info.quality_score <= min_quality {
                return Ok(());
            }
        }

        // Add to registry
        for capability in &peer_info.capabilities {
            registry.peers_by_capability
                .entry(*capability)
                .or_insert_with(HashSet::new)
                .insert(peer_info.id.clone());
        }

        registry.peers.insert(peer_info.id.clone(), peer_info);

        Ok(())
    }

    /// Prune low-quality peers
    async fn prune_peers(&self) -> Result<()> {
        let mut registry = self.known_peers.write().await;

        // Sort peers by quality score
        let mut peers_by_quality: Vec<_> = registry.peers
            .iter()
            .map(|(id, info)| (id.clone(), info.quality_score))
            .collect();

        peers_by_quality.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Keep only the best peers
        let to_remove: Vec<_> = peers_by_quality
            .iter()
            .skip(self.config.max_peers)
            .map(|(id, _)| id.clone())
            .collect();

        for peer_id in to_remove {
            registry.remove_peer(&peer_id);
        }

        Ok(())
    }

    /// Get connected peers
    pub async fn get_connected_peers(&self) -> Vec<NodeId> {
        let registry = self.known_peers.read().await;
        registry.connected.iter().cloned().collect()
    }

    /// Get peers with specific capability
    pub async fn get_peers_with_capability(&self, capability: PeerCapability) -> Vec<NodeId> {
        let registry = self.known_peers.read().await;
        registry.peers_by_capability
            .get(&capability)
            .map(|peers| peers.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get peer info
    pub async fn get_peer_info(&self, peer_id: &NodeId) -> Option<PeerInfo> {
        let registry = self.known_peers.read().await;
        registry.peers.get(peer_id).cloned()
    }

    /// Mark peer as connected
    pub async fn mark_connected(&self, peer_id: NodeId) {
        let mut registry = self.known_peers.write().await;
        registry.connected.insert(peer_id);
    }

    /// Mark peer as disconnected
    pub async fn mark_disconnected(&self, peer_id: &NodeId) {
        let mut registry = self.known_peers.write().await;
        registry.connected.remove(peer_id);
    }

    /// Blacklist a peer
    pub async fn blacklist_peer(&self, peer_id: NodeId) {
        let mut registry = self.known_peers.write().await;
        registry.blacklist.insert(peer_id.clone());
        registry.remove_peer(&peer_id);
    }

    /// Update peer quality score
    pub async fn update_peer_quality(&self, peer_id: &NodeId, quality: f64) {
        let mut registry = self.known_peers.write().await;
        if let Some(peer) = registry.peers.get_mut(peer_id) {
            peer.quality_score = quality.clamp(0.0, 1.0);
        }
    }
}

impl PeerRegistry {
    fn new() -> Self {
        Self {
            peers: HashMap::new(),
            peers_by_capability: HashMap::new(),
            connected: HashSet::new(),
            blacklist: HashSet::new(),
        }
    }

    fn remove_peer(&mut self, peer_id: &NodeId) {
        if let Some(peer_info) = self.peers.remove(peer_id) {
            // Remove from capability index
            for capability in peer_info.capabilities {
                if let Some(peers) = self.peers_by_capability.get_mut(&capability) {
                    peers.remove(peer_id);
                }
            }

            // Remove from connected set
            self.connected.remove(peer_id);
        }
    }

    fn clean_expired_peers(&mut self, ttl: Duration) {
        let now = SystemTime::now();
        let expired: Vec<_> = self.peers
            .iter()
            .filter_map(|(id, info)| {
                if now.duration_since(info.last_seen).unwrap_or(Duration::MAX) > ttl {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect();

        for peer_id in expired {
            self.remove_peer(&peer_id);
        }
    }
}

impl MdnsDiscovery {
    async fn new() -> Result<Self> {
        Ok(Self {
            service_name: "_catalog-p2p._tcp.local".to_string(),
            local_addresses: Vec::new(),
        })
    }

    async fn discover_peers(&self) -> Result<Vec<(SocketAddr, HashSet<PeerCapability>)>> {
        // Note: Actual mDNS implementation would go here
        // For now, return empty list
        Ok(Vec::new())
    }
}

// Helper for parsing IPv6 addresses
impl Ipv6Addr {
    fn from_str(s: &str) -> Result<Self> {
        s.parse().context("Invalid IPv6 address")
    }
}

// Make PeerDiscovery cloneable for background tasks
impl Clone for PeerDiscovery {
    fn clone(&self) -> Self {
        Self {
            transport: self.transport.clone(),
            dht: self.dht.clone(),
            known_peers: self.known_peers.clone(),
            config: self.config.clone(),
            mdns: self.mdns.clone(),
            bootstrap_nodes: self.bootstrap_nodes.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_registry() {
        let mut registry = PeerRegistry::new();

        let peer_id = NodeId::random();
        let peer_info = PeerInfo {
            id: peer_id.clone(),
            addresses: vec![],
            capabilities: HashSet::from([PeerCapability::PackageProvider]),
            last_seen: SystemTime::now(),
            quality_score: 0.5,
            rtt: None,
            bandwidth: None,
            discovery_source: DiscoverySource::Manual,
        };

        // Add peer
        for capability in &peer_info.capabilities {
            registry.peers_by_capability
                .entry(*capability)
                .or_insert_with(HashSet::new)
                .insert(peer_id.clone());
        }
        registry.peers.insert(peer_id.clone(), peer_info);

        assert_eq!(registry.peers.len(), 1);

        // Remove peer
        registry.remove_peer(&peer_id);
        assert_eq!(registry.peers.len(), 0);
    }
}