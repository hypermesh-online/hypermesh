//! Distributed Hash Table (DHT) for Package Discovery
//!
//! Implements a Kademlia-based DHT for decentralized package discovery

use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet, BTreeMap};
use std::time::{Duration, SystemTime};
use sha2::{Sha256, Digest};

use crate::assets::AssetPackageId;
use super::stoq_transport::{StoqTransportLayer, RequestType, ResponseData, PackageAnnouncement};

/// Node ID in the DHT network (256-bit)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId {
    id: [u8; 32],
}

impl NodeId {
    /// Create a new random node ID
    pub fn random() -> Self {
        let mut id = [0u8; 32];
        getrandom::getrandom(&mut id).unwrap();
        Self { id }
    }

    /// Create node ID from address
    pub fn from_address(addr: &std::net::SocketAddr) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(addr.to_string().as_bytes());
        let result = hasher.finalize();
        let mut id = [0u8; 32];
        id.copy_from_slice(&result[..32]);
        Self { id }
    }

    /// Calculate XOR distance between two node IDs
    pub fn distance(&self, other: &NodeId) -> Distance {
        let mut dist = [0u8; 32];
        for i in 0..32 {
            dist[i] = self.id[i] ^ other.id[i];
        }
        Distance(dist)
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(&self.id)
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.to_hex()[..8])
    }
}

/// XOR distance metric for Kademlia
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Distance([u8; 32]);

impl Distance {
    /// Get the bucket index (0-255) for this distance
    pub fn bucket_index(&self) -> usize {
        // Find the first non-zero bit
        for (i, &byte) in self.0.iter().enumerate() {
            if byte != 0 {
                let leading_zeros = byte.leading_zeros() as usize;
                return i * 8 + (7 - leading_zeros);
            }
        }
        0
    }
}

/// DHT Network implementation using Kademlia algorithm
pub struct DhtNetwork {
    /// Our node ID
    local_id: NodeId,
    /// Transport layer for communication
    transport: Arc<StoqTransportLayer>,
    /// Routing table
    routing_table: Arc<RwLock<RoutingTable>>,
    /// Value store (package announcements)
    value_store: Arc<RwLock<ValueStore>>,
    /// Pending queries
    pending_queries: Arc<RwLock<HashMap<QueryId, PendingQuery>>>,
    /// Configuration
    config: DhtConfig,
}

/// DHT configuration
#[derive(Debug, Clone)]
pub struct DhtConfig {
    /// K parameter: bucket size
    pub k: usize,
    /// Alpha parameter: concurrency factor
    pub alpha: usize,
    /// Value expiration time
    pub value_ttl: Duration,
    /// Node expiration time
    pub node_ttl: Duration,
    /// Republish interval
    pub republish_interval: Duration,
    /// Refresh interval
    pub refresh_interval: Duration,
}

impl Default for DhtConfig {
    fn default() -> Self {
        Self {
            k: 20,
            alpha: 3,
            value_ttl: Duration::from_secs(86400), // 24 hours
            node_ttl: Duration::from_secs(3600),   // 1 hour
            republish_interval: Duration::from_secs(3600), // 1 hour
            refresh_interval: Duration::from_secs(900),    // 15 minutes
        }
    }
}

/// Routing table for Kademlia
struct RoutingTable {
    /// K-buckets indexed by distance
    buckets: Vec<KBucket>,
    /// Configuration
    config: DhtConfig,
}

/// K-bucket for storing nodes at a specific distance
struct KBucket {
    /// Nodes in the bucket (most recently seen last)
    nodes: Vec<NodeInfo>,
    /// Replacement cache for full buckets
    replacements: Vec<NodeInfo>,
}

/// Information about a node in the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Node ID
    pub id: NodeId,
    /// Node address
    pub address: std::net::SocketAddr,
    /// Last seen timestamp
    pub last_seen: SystemTime,
    /// Round-trip time (ms)
    pub rtt: Option<u32>,
}

/// Value store for DHT
struct ValueStore {
    /// Stored values by key
    values: HashMap<ValueKey, Vec<StoredValue>>,
    /// Package index
    package_index: HashMap<AssetPackageId, HashSet<ValueKey>>,
}

/// Key for stored values
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ValueKey([u8; 32]);

impl ValueKey {
    /// Create key from package ID
    fn from_package_id(id: &AssetPackageId) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(id.as_bytes());
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result[..32]);
        Self(key)
    }

    /// Create key from search query
    fn from_query(query: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(query.as_bytes());
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result[..32]);
        Self(key)
    }
}

/// Stored value in DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredValue {
    /// The actual value data
    data: ValueData,
    /// Publisher node ID
    publisher: NodeId,
    /// Publication timestamp
    published_at: SystemTime,
    /// Expiration time
    expires_at: SystemTime,
}

/// Value data types
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ValueData {
    /// Package announcement
    PackageAnnouncement(PackageAnnouncement),
    /// Peer list for a package
    PackagePeers {
        package_id: AssetPackageId,
        peers: Vec<NodeId>,
    },
    /// Search index entry
    SearchIndex {
        keyword: String,
        packages: Vec<AssetPackageId>,
    },
}

/// Query ID for tracking pending queries
type QueryId = [u8; 16];

/// Pending query information
struct PendingQuery {
    /// Query type
    query_type: QueryType,
    /// Target key
    target: ValueKey,
    /// Nodes to query
    to_query: Vec<NodeId>,
    /// Nodes already queried
    queried: HashSet<NodeId>,
    /// Best nodes found so far
    best_nodes: BTreeMap<Distance, NodeInfo>,
    /// Values found
    values: Vec<StoredValue>,
    /// Query start time
    started_at: std::time::Instant,
}

/// Query types
enum QueryType {
    FindNode,
    FindValue,
    Store,
}

impl DhtNetwork {
    /// Create a new DHT network
    pub async fn new(
        transport: Arc<StoqTransportLayer>,
        bootstrap_nodes: Vec<String>,
    ) -> Result<Self> {
        let local_id = NodeId::random();
        let config = DhtConfig::default();

        let routing_table = Arc::new(RwLock::new(RoutingTable::new(config.clone())));
        let value_store = Arc::new(RwLock::new(ValueStore::new()));

        let dht = Self {
            local_id: local_id.clone(),
            transport,
            routing_table,
            value_store,
            pending_queries: Arc::new(RwLock::new(HashMap::new())),
            config,
        };

        // Bootstrap the network
        dht.bootstrap(bootstrap_nodes).await?;

        // Start maintenance tasks
        dht.start_maintenance_tasks();

        Ok(dht)
    }

    /// Bootstrap the DHT by connecting to known nodes
    async fn bootstrap(&self, bootstrap_nodes: Vec<String>) -> Result<()> {
        for node_addr in bootstrap_nodes {
            let addr = node_addr.parse::<std::net::SocketAddr>()
                .context("Invalid bootstrap node address")?;

            // Connect to bootstrap node
            let node_id = self.transport.connect(addr).await?;

            // Add to routing table
            let node_info = NodeInfo {
                id: node_id,
                address: addr,
                last_seen: SystemTime::now(),
                rtt: None,
            };

            self.routing_table.write().await.add_node(node_info);

            // Perform initial node lookup for our own ID
            self.lookup_nodes(&self.local_id).await?;
        }

        Ok(())
    }

    /// Start background maintenance tasks
    fn start_maintenance_tasks(&self) {
        let routing_table = self.routing_table.clone();
        let value_store = self.value_store.clone();
        let refresh_interval = self.config.refresh_interval;
        let republish_interval = self.config.republish_interval;

        // Refresh routing table periodically
        let rt = routing_table.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(refresh_interval);
            loop {
                interval.tick().await;
                if let Err(e) = Self::refresh_routing_table(rt.clone()).await {
                    tracing::warn!("Failed to refresh routing table: {}", e);
                }
            }
        });

        // Republish values periodically
        let vs = value_store.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(republish_interval);
            loop {
                interval.tick().await;
                if let Err(e) = Self::republish_values(vs.clone()).await {
                    tracing::warn!("Failed to republish values: {}", e);
                }
            }
        });

        // Clean expired values
        let vs = value_store.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                vs.write().await.clean_expired();
            }
        });
    }

    /// Refresh routing table by looking up random nodes
    async fn refresh_routing_table(routing_table: Arc<RwLock<RoutingTable>>) -> Result<()> {
        // Generate random node ID and look it up
        let random_id = NodeId::random();
        // TODO: Implement node lookup
        Ok(())
    }

    /// Republish stored values
    async fn republish_values(value_store: Arc<RwLock<ValueStore>>) -> Result<()> {
        // TODO: Implement value republishing
        Ok(())
    }

    /// Announce a package on the DHT
    pub async fn announce_package(
        &self,
        package_id: AssetPackageId,
        content_addresses: Vec<String>,
    ) -> Result<()> {
        let key = ValueKey::from_package_id(&package_id);

        // Create package announcement
        let announcement = PackageAnnouncement {
            package_id,
            metadata: super::stoq_transport::PackageMetadata {
                name: String::new(), // TODO: Get actual metadata
                version: String::new(),
                size: 0,
                chunk_count: 0,
                chunk_size: 0,
                hash: String::new(),
                created_at: chrono::Utc::now(),
            },
            content_addresses,
        };

        let value = StoredValue {
            data: ValueData::PackageAnnouncement(announcement),
            publisher: self.local_id.clone(),
            published_at: SystemTime::now(),
            expires_at: SystemTime::now() + self.config.value_ttl,
        };

        // Store locally
        self.value_store.write().await.store(key.clone(), value.clone());

        // Store on k closest nodes
        let closest_nodes = self.find_closest_nodes(&key, self.config.k).await?;
        for node in closest_nodes {
            self.store_value_on_node(&node, key.clone(), value.clone()).await?;
        }

        Ok(())
    }

    /// Find peers that have a specific package
    pub async fn find_package_peers(&self, package_id: &AssetPackageId) -> Result<Vec<NodeId>> {
        let key = ValueKey::from_package_id(package_id);

        // Look up the value in DHT
        let values = self.lookup_value(&key).await?;

        let mut peers = Vec::new();
        for value in values {
            if let ValueData::PackagePeers { peers: package_peers, .. } = value.data {
                peers.extend(package_peers);
            }
        }

        // Deduplicate
        peers.sort();
        peers.dedup();

        Ok(peers)
    }

    /// Search for packages by query
    pub async fn search_packages(&self, query: &str) -> Result<Vec<AssetPackageId>> {
        let key = ValueKey::from_query(query);

        // Look up the search index
        let values = self.lookup_value(&key).await?;

        let mut packages = Vec::new();
        for value in values {
            if let ValueData::SearchIndex { packages: found_packages, .. } = value.data {
                packages.extend(found_packages);
            }
        }

        // Deduplicate
        packages.sort();
        packages.dedup();

        Ok(packages)
    }

    /// Register as a seeder for a package
    pub async fn register_as_seeder(&self, package_id: AssetPackageId) -> Result<()> {
        let key = ValueKey::from_package_id(&package_id);

        // Get current peers list
        let mut peers = self.find_package_peers(&package_id).await.unwrap_or_default();

        // Add ourselves
        if !peers.contains(&self.local_id) {
            peers.push(self.local_id.clone());
        }

        // Store updated peers list
        let value = StoredValue {
            data: ValueData::PackagePeers {
                package_id,
                peers: peers.clone(),
            },
            publisher: self.local_id.clone(),
            published_at: SystemTime::now(),
            expires_at: SystemTime::now() + self.config.value_ttl,
        };

        // Store locally
        self.value_store.write().await.store(key.clone(), value.clone());

        // Store on k closest nodes
        let closest_nodes = self.find_closest_nodes(&key, self.config.k).await?;
        for node in closest_nodes {
            self.store_value_on_node(&node, key.clone(), value.clone()).await?;
        }

        Ok(())
    }

    /// Look up nodes closest to a target
    async fn lookup_nodes(&self, target: &NodeId) -> Result<Vec<NodeInfo>> {
        let key = ValueKey(target.id);
        self.find_closest_nodes(&key, self.config.k).await
    }

    /// Look up a value in the DHT
    async fn lookup_value(&self, key: &ValueKey) -> Result<Vec<StoredValue>> {
        // Check local store first
        if let Some(values) = self.value_store.read().await.get(key) {
            return Ok(values);
        }

        // Query network
        let closest_nodes = self.find_closest_nodes(key, self.config.k).await?;

        let mut found_values = Vec::new();
        for node in closest_nodes {
            if let Ok(values) = self.query_value_from_node(&node, key).await {
                found_values.extend(values);
            }
        }

        Ok(found_values)
    }

    /// Find k closest nodes to a key
    async fn find_closest_nodes(&self, key: &ValueKey, k: usize) -> Result<Vec<NodeInfo>> {
        let target_id = NodeId { id: key.0 };
        let mut closest = self.routing_table.read().await.get_closest_nodes(&target_id, k);

        // Iterative lookup
        let mut queried = HashSet::new();
        let mut to_query = closest.clone();

        while !to_query.is_empty() && closest.len() < k {
            let node = to_query.remove(0);
            if queried.contains(&node.id) {
                continue;
            }
            queried.insert(node.id.clone());

            // Query node for closer nodes
            if let Ok(response) = self.transport.send_request(
                &node.id,
                RequestType::GetPeers,
            ).await {
                if let ResponseData::Peers(peers) = response {
                    for peer_id in peers {
                        if !queried.contains(&peer_id) {
                            // TODO: Get full node info
                            // to_query.push(node_info);
                        }
                    }
                }
            }

            // Sort by distance
            to_query.sort_by_key(|n| target_id.distance(&n.id));
            to_query.truncate(k);

            // Update closest
            closest.extend(to_query.iter().cloned());
            closest.sort_by_key(|n| target_id.distance(&n.id));
            closest.truncate(k);
        }

        Ok(closest)
    }

    /// Store a value on a specific node
    async fn store_value_on_node(
        &self,
        node: &NodeInfo,
        key: ValueKey,
        value: StoredValue,
    ) -> Result<()> {
        // TODO: Implement store request
        Ok(())
    }

    /// Query a value from a specific node
    async fn query_value_from_node(
        &self,
        node: &NodeInfo,
        key: &ValueKey,
    ) -> Result<Vec<StoredValue>> {
        // TODO: Implement value query
        Ok(Vec::new())
    }
}

impl RoutingTable {
    fn new(config: DhtConfig) -> Self {
        let mut buckets = Vec::with_capacity(256);
        for _ in 0..256 {
            buckets.push(KBucket {
                nodes: Vec::new(),
                replacements: Vec::new(),
            });
        }

        Self { buckets, config }
    }

    fn add_node(&mut self, node: NodeInfo) {
        // TODO: Calculate distance and add to appropriate bucket
        let bucket_idx = 0; // Placeholder
        let bucket = &mut self.buckets[bucket_idx];

        // Check if node already exists
        if let Some(existing) = bucket.nodes.iter_mut().find(|n| n.id == node.id) {
            existing.last_seen = node.last_seen;
            existing.rtt = node.rtt;
            return;
        }

        // Add new node
        if bucket.nodes.len() < self.config.k {
            bucket.nodes.push(node);
        } else {
            // Bucket full, add to replacements
            bucket.replacements.push(node);
            if bucket.replacements.len() > self.config.k {
                bucket.replacements.remove(0);
            }
        }
    }

    fn get_closest_nodes(&self, target: &NodeId, k: usize) -> Vec<NodeInfo> {
        let mut nodes = Vec::new();

        // Collect all nodes
        for bucket in &self.buckets {
            nodes.extend(bucket.nodes.clone());
        }

        // Sort by distance
        nodes.sort_by_key(|n| target.distance(&n.id));
        nodes.truncate(k);

        nodes
    }
}

impl ValueStore {
    fn new() -> Self {
        Self {
            values: HashMap::new(),
            package_index: HashMap::new(),
        }
    }

    fn store(&mut self, key: ValueKey, value: StoredValue) {
        // Extract package ID if present
        if let ValueData::PackageAnnouncement(ref announcement) = value.data {
            self.package_index
                .entry(announcement.package_id)
                .or_insert_with(HashSet::new)
                .insert(key.clone());
        }

        self.values
            .entry(key)
            .or_insert_with(Vec::new)
            .push(value);
    }

    fn get(&self, key: &ValueKey) -> Option<Vec<StoredValue>> {
        self.values.get(key).cloned()
    }

    fn clean_expired(&mut self) {
        let now = SystemTime::now();

        // Remove expired values
        for values in self.values.values_mut() {
            values.retain(|v| v.expires_at > now);
        }

        // Remove empty entries
        self.values.retain(|_, v| !v.is_empty());

        // Update package index
        self.package_index.retain(|_, keys| {
            keys.retain(|k| self.values.contains_key(k));
            !keys.is_empty()
        });
    }
}

// Helper methods for AssetPackageId
impl AssetPackageId {
    fn as_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

// For testing/compilation - implement getrandom
mod getrandom {
    pub fn getrandom(buf: &mut [u8]) -> Result<(), ()> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        for (i, byte) in buf.iter_mut().enumerate() {
            *byte = ((nanos >> (i * 8)) & 0xFF) as u8;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_distance() {
        let id1 = NodeId { id: [0u8; 32] };
        let id2 = NodeId { id: [1u8; 32] };

        let distance = id1.distance(&id2);
        assert_eq!(distance.0[0], 1);
    }

    #[test]
    fn test_value_key_generation() {
        let package_id = AssetPackageId::new();
        let key1 = ValueKey::from_package_id(&package_id);
        let key2 = ValueKey::from_package_id(&package_id);

        assert_eq!(key1, key2);
    }
}