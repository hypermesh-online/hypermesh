//! Decentralized Asset Library Sharing Module
//!
//! Enables cross-node library sharing, synchronization, and mirroring
//! across the HyperMesh network for fully decentralized asset distribution.

use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime};

pub mod synchronization;
pub mod mirroring;
pub mod discovery;
pub mod protocols;
pub mod topology;

pub use synchronization::{SyncManager, SyncStrategy, SyncState};
pub use mirroring::{MirrorManager, MirrorStrategy, ReplicationConfig};
pub use discovery::{DiscoveryService, AssetIndex, SearchCapabilities};
pub use protocols::{SharingProtocol, SharePermission, BandwidthAllocation};
pub use topology::{NetworkTopology, NodeLocation, RoutingStrategy};

use crate::{AssetId, AssetPackage, AssetMetadata};

/// Sharing configuration for decentralized library operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingConfig {
    /// Node identity in the HyperMesh network
    pub node_id: String,
    /// Maximum storage for mirrored packages (bytes)
    pub max_mirror_storage: u64,
    /// Maximum bandwidth for sharing (bytes/sec)
    pub max_bandwidth: u64,
    /// Replication factor for important packages
    pub replication_factor: u32,
    /// Synchronization interval
    pub sync_interval: Duration,
    /// Discovery cache TTL
    pub discovery_cache_ttl: Duration,
    /// Geographic location for optimal routing
    pub geographic_location: Option<NodeLocation>,
    /// Sharing permissions default
    pub default_permission: SharePermission,
    /// Enable automatic mirroring of popular packages
    pub auto_mirror_popular: bool,
    /// Minimum popularity score for auto-mirroring
    pub auto_mirror_threshold: f64,
    /// Enable bandwidth contribution incentives
    pub enable_incentives: bool,
    /// Fair-use bandwidth limit per peer (bytes/sec)
    pub fair_use_limit: u64,
}

impl Default for SharingConfig {
    fn default() -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
            max_mirror_storage: 10 * 1024 * 1024 * 1024, // 10GB
            max_bandwidth: 10 * 1024 * 1024, // 10MB/s
            replication_factor: 3,
            sync_interval: Duration::from_secs(300), // 5 minutes
            discovery_cache_ttl: Duration::from_secs(3600), // 1 hour
            geographic_location: None,
            default_permission: SharePermission::Public,
            auto_mirror_popular: true,
            auto_mirror_threshold: 0.8,
            enable_incentives: true,
            fair_use_limit: 1024 * 1024, // 1MB/s per peer
        }
    }
}

/// Library sharing statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SharingStats {
    /// Total packages shared
    pub packages_shared: u64,
    /// Total packages mirrored
    pub packages_mirrored: u64,
    /// Total bandwidth contributed (bytes)
    pub bandwidth_contributed: u64,
    /// Total bandwidth consumed (bytes)
    pub bandwidth_consumed: u64,
    /// Active peer connections
    pub active_peers: u32,
    /// Total sync operations
    pub sync_operations: u64,
    /// Failed sync operations
    pub failed_syncs: u64,
    /// Cache hit ratio
    pub cache_hit_ratio: f64,
    /// Average response time (ms)
    pub avg_response_time: u64,
    /// Network health score (0-1)
    pub network_health: f64,
}

/// Peer information for sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Peer node ID
    pub node_id: String,
    /// Peer network address
    pub address: String,
    /// Peer's available packages
    pub available_packages: HashSet<AssetId>,
    /// Peer's storage capacity
    pub storage_capacity: u64,
    /// Peer's bandwidth capacity
    pub bandwidth_capacity: u64,
    /// Peer's reputation score
    pub reputation: f64,
    /// Last seen timestamp
    pub last_seen: SystemTime,
    /// Geographic location
    pub location: Option<NodeLocation>,
    /// Supported protocols
    pub supported_protocols: Vec<String>,
}

/// Package availability information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageAvailability {
    /// Package asset ID
    pub asset_id: AssetId,
    /// Nodes that have this package
    pub available_nodes: Vec<String>,
    /// Replication count
    pub replication_count: u32,
    /// Geographic distribution
    pub geographic_distribution: HashMap<String, u32>,
    /// Last update timestamp
    pub last_updated: SystemTime,
    /// Package popularity score
    pub popularity: f64,
    /// Average download speed
    pub avg_download_speed: u64,
}

/// Synchronization conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Use newest version
    NewestWins,
    /// Use version with highest consensus
    ConsensusWins,
    /// Merge changes (if possible)
    Merge,
    /// Keep both versions
    KeepBoth,
    /// Manual resolution required
    Manual,
}

/// Library sharing event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SharingEvent {
    /// New peer joined network
    PeerJoined { node_id: String, address: String },
    /// Peer left network
    PeerLeft { node_id: String },
    /// Package shared
    PackageShared { asset_id: AssetId, recipient: String },
    /// Package mirrored
    PackageMirrored { asset_id: AssetId, mirror_node: String },
    /// Synchronization completed
    SyncCompleted { peer: String, packages_synced: u32 },
    /// Synchronization failed
    SyncFailed { peer: String, error: String },
    /// Discovery cache updated
    DiscoveryUpdated { packages_indexed: u32 },
    /// Network topology changed
    TopologyChanged { nodes_added: u32, nodes_removed: u32 },
}

/// Main decentralized sharing manager
pub struct SharingManager {
    config: Arc<SharingConfig>,
    sync_manager: Arc<SyncManager>,
    mirror_manager: Arc<MirrorManager>,
    discovery_service: Arc<DiscoveryService>,
    sharing_protocol: Arc<SharingProtocol>,
    network_topology: Arc<RwLock<NetworkTopology>>,
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    package_availability: Arc<RwLock<HashMap<AssetId, PackageAvailability>>>,
    stats: Arc<RwLock<SharingStats>>,
    event_listeners: Arc<RwLock<Vec<Box<dyn Fn(SharingEvent) + Send + Sync>>>>,
}

impl SharingManager {
    /// Create new sharing manager
    pub async fn new(config: SharingConfig) -> Result<Self> {
        let config = Arc::new(config.clone());

        // Initialize components
        let sync_manager = Arc::new(SyncManager::new(
            config.node_id.clone(),
            config.sync_interval,
        ).await?);

        let mirror_manager = Arc::new(MirrorManager::new(
            config.max_mirror_storage,
            config.replication_factor,
        ).await?);

        let discovery_service = Arc::new(DiscoveryService::new(
            config.discovery_cache_ttl,
        ).await?);

        let sharing_protocol = Arc::new(SharingProtocol::new(
            config.max_bandwidth,
            config.fair_use_limit,
        ).await?);

        let network_topology = Arc::new(RwLock::new(
            NetworkTopology::new(config.node_id.clone())
        ));

        Ok(Self {
            config,
            sync_manager,
            mirror_manager,
            discovery_service,
            sharing_protocol,
            network_topology,
            peers: Arc::new(RwLock::new(HashMap::new())),
            package_availability: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(SharingStats::default())),
            event_listeners: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Connect to peer node
    pub async fn connect_peer(&self, address: &str) -> Result<String> {
        // Establish connection via sharing protocol
        let peer_info = self.sharing_protocol.connect(address).await?;

        // Register peer
        let mut peers = self.peers.write().await;
        let node_id = peer_info.node_id.clone();
        peers.insert(node_id.clone(), peer_info);

        // Update network topology
        let mut topology = self.network_topology.write().await;
        topology.add_peer(&node_id, address).await?;

        // Notify listeners
        self.emit_event(SharingEvent::PeerJoined {
            node_id: node_id.clone(),
            address: address.to_string(),
        }).await;

        // Initiate sync with new peer
        self.sync_with_peer(&node_id).await?;

        Ok(node_id)
    }

    /// Disconnect from peer
    pub async fn disconnect_peer(&self, node_id: &str) -> Result<()> {
        // Remove from peers
        let mut peers = self.peers.write().await;
        peers.remove(node_id);

        // Update topology
        let mut topology = self.network_topology.write().await;
        topology.remove_peer(node_id).await?;

        // Close protocol connection
        self.sharing_protocol.disconnect(node_id).await?;

        // Notify listeners
        self.emit_event(SharingEvent::PeerLeft {
            node_id: node_id.to_string(),
        }).await;

        Ok(())
    }

    /// Share package with network
    pub async fn share_package(
        &self,
        package: &AssetPackage,
        permission: SharePermission,
    ) -> Result<()> {
        let asset_id = package.metadata.id.clone();

        // Register with discovery service
        self.discovery_service.register_package(
            &asset_id,
            &package.metadata,
            permission.clone(),
        ).await?;

        // Update availability
        let mut availability = self.package_availability.write().await;
        availability.entry(asset_id.clone()).or_insert_with(|| {
            PackageAvailability {
                asset_id: asset_id.clone(),
                available_nodes: vec![self.config.node_id.clone()],
                replication_count: 1,
                geographic_distribution: HashMap::new(),
                last_updated: SystemTime::now(),
                popularity: 0.0,
                avg_download_speed: 0,
            }
        });

        // Notify peers about new package
        self.broadcast_package_availability(&asset_id).await?;

        // Update stats
        let mut stats = self.stats.write().await;
        stats.packages_shared += 1;

        Ok(())
    }

    /// Synchronize with specific peer
    pub async fn sync_with_peer(&self, node_id: &str) -> Result<()> {
        let peers = self.peers.read().await;
        let peer = peers.get(node_id)
            .ok_or_else(|| anyhow::anyhow!("Peer not found"))?;

        // Perform synchronization
        let sync_result = self.sync_manager.sync_with_peer(
            peer,
            ConflictResolution::ConsensusWins,
        ).await;

        match sync_result {
            Ok(packages_synced) => {
                // Update stats
                let mut stats = self.stats.write().await;
                stats.sync_operations += 1;

                // Notify listeners
                self.emit_event(SharingEvent::SyncCompleted {
                    peer: node_id.to_string(),
                    packages_synced,
                }).await;
            }
            Err(e) => {
                // Update stats
                let mut stats = self.stats.write().await;
                stats.failed_syncs += 1;

                // Notify listeners
                self.emit_event(SharingEvent::SyncFailed {
                    peer: node_id.to_string(),
                    error: e.to_string(),
                }).await;

                return Err(e);
            }
        }

        Ok(())
    }

    /// Mirror popular packages automatically
    pub async fn auto_mirror_packages(&self) -> Result<u32> {
        if !self.config.auto_mirror_popular {
            return Ok(0);
        }

        // Get popular packages from discovery
        let popular_packages = self.discovery_service
            .get_popular_packages(self.config.auto_mirror_threshold)
            .await?;

        let mut mirrored_count = 0;

        for (asset_id, metadata) in popular_packages {
            // Check if we should mirror this package
            if self.should_mirror_package(&asset_id, &metadata).await? {
                // Mirror the package
                if let Ok(_) = self.mirror_manager.mirror_package(
                    &asset_id,
                    &metadata,
                ).await {
                    mirrored_count += 1;

                    // Notify listeners
                    self.emit_event(SharingEvent::PackageMirrored {
                        asset_id,
                        mirror_node: self.config.node_id.clone(),
                    }).await;
                }
            }
        }

        // Update stats
        let mut stats = self.stats.write().await;
        stats.packages_mirrored += mirrored_count as u64;

        Ok(mirrored_count)
    }

    /// Search for packages across the network
    pub async fn search_packages(&self, query: &str) -> Result<Vec<(AssetId, AssetMetadata)>> {
        // Search local and cached packages first
        let local_results = self.discovery_service.search_local(query).await?;

        // If not enough results, search network
        if local_results.len() < 10 {
            let network_results = self.discovery_service.search_network(
                query,
                &*self.peers.read().await,
            ).await?;

            // Merge and deduplicate results
            let mut all_results = local_results;
            for result in network_results {
                if !all_results.iter().any(|(id, _)| id == &result.0) {
                    all_results.push(result);
                }
            }

            Ok(all_results)
        } else {
            Ok(local_results)
        }
    }

    /// Get package from network
    pub async fn get_package(&self, asset_id: &AssetId) -> Result<AssetPackage> {
        // Check local availability first
        if let Some(package) = self.discovery_service.get_local_package(asset_id).await? {
            return Ok(package);
        }

        // Find nodes that have this package
        let availability = self.package_availability.read().await;
        if let Some(info) = availability.get(asset_id) {
            // Select best node based on location and bandwidth
            let best_node = self.select_best_node(&info.available_nodes).await?;

            // Download from selected node
            let package = self.sharing_protocol.download_package(
                asset_id,
                &best_node,
            ).await?;

            // Update stats
            let mut stats = self.stats.write().await;
            stats.bandwidth_consumed += package.metadata.size as u64;

            Ok(package)
        } else {
            Err(anyhow::anyhow!("Package not found in network"))
        }
    }

    /// Update network topology
    pub async fn update_topology(&self) -> Result<()> {
        let mut topology = self.network_topology.write().await;
        let peers = self.peers.read().await;

        // Update topology with current peer information
        for (node_id, peer_info) in peers.iter() {
            topology.update_node_info(node_id, peer_info).await?;
        }

        // Optimize routing based on new topology
        topology.optimize_routing().await?;

        Ok(())
    }

    /// Get sharing statistics
    pub async fn get_stats(&self) -> SharingStats {
        self.stats.read().await.clone()
    }

    /// Register event listener
    pub async fn on_event<F>(&self, listener: F)
    where
        F: Fn(SharingEvent) + Send + Sync + 'static,
    {
        let mut listeners = self.event_listeners.write().await;
        listeners.push(Box::new(listener));
    }

    // Helper methods

    async fn should_mirror_package(
        &self,
        asset_id: &AssetId,
        metadata: &AssetMetadata,
    ) -> Result<bool> {
        // Check if we already have it
        if self.discovery_service.has_package(asset_id).await? {
            return Ok(false);
        }

        // Check storage capacity
        let current_usage = self.mirror_manager.get_storage_usage().await?;
        if current_usage + metadata.size as u64 > self.config.max_mirror_storage {
            return Ok(false);
        }

        // Check replication needs
        let availability = self.package_availability.read().await;
        if let Some(info) = availability.get(asset_id) {
            if info.replication_count >= self.config.replication_factor {
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn select_best_node(&self, nodes: &[String]) -> Result<String> {
        let peers = self.peers.read().await;
        let topology = self.network_topology.read().await;

        // Score each node based on various factors
        let mut best_node = nodes[0].clone();
        let mut best_score = 0.0;

        for node_id in nodes {
            if let Some(peer) = peers.get(node_id) {
                let mut score = 0.0;

                // Factor 1: Reputation
                score += peer.reputation * 0.3;

                // Factor 2: Bandwidth capacity
                let bandwidth_score = (peer.bandwidth_capacity as f64 / (10 * 1024 * 1024) as f64).min(1.0);
                score += bandwidth_score * 0.3;

                // Factor 3: Network distance
                let distance_score = topology.get_distance_score(&self.config.node_id, node_id);
                score += distance_score * 0.2;

                // Factor 4: Geographic proximity
                if let (Some(our_loc), Some(peer_loc)) =
                    (&self.config.geographic_location, &peer.location) {
                    let geo_score = 1.0 / (1.0 + our_loc.distance_to(peer_loc));
                    score += geo_score * 0.2;
                }

                if score > best_score {
                    best_score = score;
                    best_node = node_id.clone();
                }
            }
        }

        Ok(best_node)
    }

    async fn broadcast_package_availability(&self, asset_id: &AssetId) -> Result<()> {
        let peers = self.peers.read().await;

        for (node_id, _) in peers.iter() {
            // Notify each peer about the new package
            self.sharing_protocol.notify_availability(
                node_id,
                asset_id,
            ).await?;
        }

        Ok(())
    }

    async fn emit_event(&self, event: SharingEvent) {
        let listeners = self.event_listeners.read().await;
        for listener in listeners.iter() {
            listener(event.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sharing_manager_creation() {
        let config = SharingConfig::default();
        let manager = SharingManager::new(config).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_peer_connection() {
        let config = SharingConfig::default();
        let manager = SharingManager::new(config).await.unwrap();

        // Test connecting to a peer (would need mock in real test)
        // let node_id = manager.connect_peer("peer.hypermesh.online").await;
        // assert!(node_id.is_ok());
    }
}