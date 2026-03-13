// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Distributed Hash Table (DHT) for Package Discovery
//!
//! Implements a Kademlia-based DHT for decentralized package discovery

mod routing_table;
mod value_store;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
// BLAKE3 used via blake3::hash() for DHT node identity

use super::stoq_transport::{PackageAnnouncement, RequestType, ResponseData, StoqTransportLayer};
use crate::assets::AssetPackageId;

use routing_table::RoutingTable;
use value_store::{StoredValue, ValueData, ValueKey, ValueStore};


/// Kademlia DHT node identity (256-bit, XOR-distance).
/// Distinct from hypermesh_lib::NodeId which is a human-readable string identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DhtNodeId {
    pub(crate) id: [u8; 32],
}

impl DhtNodeId {
    /// Create a new random node ID
    pub fn random() -> Self {
        let mut id = [0u8; 32];
        // Use system time as fallback if getrandom fails
        if getrandom::getrandom(&mut id).is_err() {
            use std::time::{SystemTime, UNIX_EPOCH};
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            for (i, byte) in id.iter_mut().enumerate() {
                *byte = ((nanos >> (i * 8)) & 0xFF) as u8;
            }
        }
        Self { id }
    }

    /// Create node ID from address
    pub fn from_address(addr: &std::net::SocketAddr) -> Self {
        let hash = blake3::hash(addr.to_string().as_bytes());
        Self {
            id: *hash.as_bytes(),
        }
    }

    /// Calculate XOR distance between two node IDs
    pub fn distance(&self, other: &DhtNodeId) -> Distance {
        let mut dist = [0u8; 32];
        for (i, byte) in dist.iter_mut().enumerate() {
            *byte = self.id[i] ^ other.id[i];
        }
        Distance(dist)
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.id)
    }
}

impl std::fmt::Display for DhtNodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.to_hex()[..8])
    }
}

/// XOR distance metric for Kademlia
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Distance(pub(crate) [u8; 32]);

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
            refresh_interval: Duration::from_secs(900), // 15 minutes
        }
    }
}

/// Information about a node in the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Node ID
    pub id: DhtNodeId,
    /// Node address
    pub address: std::net::SocketAddr,
    /// Last seen timestamp
    pub last_seen: SystemTime,
    /// Round-trip time (ms)
    pub rtt: Option<u32>,
}

/// DHT Network implementation using Kademlia algorithm
pub struct DhtNetwork {
    /// Our node ID
    local_id: DhtNodeId,
    /// Transport layer for communication
    transport: Arc<StoqTransportLayer>,
    /// Routing table
    routing_table: Arc<RwLock<RoutingTable>>,
    /// Value store (package announcements)
    value_store: Arc<RwLock<ValueStore>>,
    /// Pending queries
    _pending_queries: Arc<RwLock<HashMap<value_store::QueryId, value_store::PendingQuery>>>,
    /// Configuration
    config: DhtConfig,
}

impl DhtNetwork {
    /// Create a new DHT network
    pub async fn new(
        transport: Arc<StoqTransportLayer>,
        bootstrap_nodes: Vec<String>,
    ) -> Result<Self> {
        let local_id = DhtNodeId::random();
        let config = DhtConfig::default();

        let routing_table = Arc::new(RwLock::new(RoutingTable::new(config.clone())));
        let value_store = Arc::new(RwLock::new(ValueStore::new()));

        let dht = Self {
            local_id: local_id.clone(),
            transport,
            routing_table,
            value_store,
            _pending_queries: Arc::new(RwLock::new(HashMap::new())),
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
        if bootstrap_nodes.is_empty() {
            return Ok(());
        }

        for node_addr in bootstrap_nodes {
            let addr = node_addr
                .parse::<std::net::SocketAddr>()
                .context("Invalid bootstrap node address")?;

            let node_id = self.transport.connect(addr).await?;

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

    /// Refresh routing table by evicting stale entries and promoting replacements.
    pub(crate) async fn refresh_routing_table(
        routing_table: Arc<RwLock<RoutingTable>>,
    ) -> Result<()> {
        let mut rt = routing_table.write().await;
        let now = SystemTime::now();
        let stale_threshold = rt.config.node_ttl;
        let k = rt.config.k;

        for bucket in &mut rt.buckets {
            bucket.nodes.retain(|node| {
                now.duration_since(node.last_seen)
                    .unwrap_or(Duration::from_secs(0))
                    < stale_threshold
            });

            while bucket.nodes.len() < k && !bucket.replacements.is_empty() {
                if let Some(replacement) = bucket.replacements.pop() {
                    if now
                        .duration_since(replacement.last_seen)
                        .unwrap_or(Duration::from_secs(0))
                        < stale_threshold
                    {
                        bucket.nodes.push(replacement);
                    }
                }
            }

            bucket.replacements.retain(|node| {
                now.duration_since(node.last_seen)
                    .unwrap_or(Duration::from_secs(0))
                    < stale_threshold
            });
        }

        Ok(())
    }

    /// Republish locally-owned values approaching expiration.
    pub(crate) async fn republish_values(value_store: Arc<RwLock<ValueStore>>) -> Result<()> {
        let mut store = value_store.write().await;
        let now = SystemTime::now();
        let renewal_fraction = 0.75;

        for values in store.values.values_mut() {
            for value in values.iter_mut() {
                let total_lifetime = value
                    .expires_at
                    .duration_since(value.published_at)
                    .unwrap_or(Duration::from_secs(86400));
                let elapsed = now
                    .duration_since(value.published_at)
                    .unwrap_or(Duration::from_secs(0));

                if elapsed.as_secs_f64() > total_lifetime.as_secs_f64() * renewal_fraction {
                    value.published_at = now;
                    value.expires_at = now + total_lifetime;
                }
            }
        }

        Ok(())
    }

    /// Announce a package on the DHT
    pub async fn announce_package(
        &self,
        package_id: AssetPackageId,
        content_addresses: Vec<String>,
    ) -> Result<()> {
        let key = ValueKey::from_package_id(&package_id);

        let announcement = PackageAnnouncement {
            package_id,
            metadata: super::stoq_transport::PackageMetadata {
                name: String::new(),
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

        self.value_store
            .write()
            .await
            .store(key.clone(), value.clone());

        let closest_nodes = self.find_closest_nodes(&key, self.config.k).await?;
        for node in closest_nodes {
            self.store_value_on_node(&node, key.clone(), value.clone())
                .await?;
        }

        Ok(())
    }

    /// Find peers that have a specific package
    pub async fn find_package_peers(&self, package_id: &AssetPackageId) -> Result<Vec<DhtNodeId>> {
        let key = ValueKey::from_package_id(package_id);

        let values = self.lookup_value(&key).await?;

        let mut peers = Vec::new();
        for value in values {
            if let ValueData::PackagePeers {
                peers: package_peers,
                ..
            } = value.data
            {
                peers.extend(package_peers);
            }
        }

        peers.sort();
        peers.dedup();

        Ok(peers)
    }

    /// Search for packages by query
    pub async fn search_packages(&self, query: &str) -> Result<Vec<AssetPackageId>> {
        let key = ValueKey::from_query(query);

        let values = self.lookup_value(&key).await?;

        let mut packages = Vec::new();
        for value in values {
            if let ValueData::SearchIndex {
                packages: found_packages,
                ..
            } = value.data
            {
                packages.extend(found_packages);
            }
        }

        packages.sort();
        packages.dedup();

        Ok(packages)
    }

    /// Register as a seeder for a package
    pub async fn register_as_seeder(&self, package_id: AssetPackageId) -> Result<()> {
        let key = ValueKey::from_package_id(&package_id);

        let mut peers = self
            .find_package_peers(&package_id)
            .await
            .unwrap_or_default();

        if !peers.contains(&self.local_id) {
            peers.push(self.local_id.clone());
        }

        let value = StoredValue {
            data: ValueData::PackagePeers {
                package_id,
                peers: peers.clone(),
            },
            publisher: self.local_id.clone(),
            published_at: SystemTime::now(),
            expires_at: SystemTime::now() + self.config.value_ttl,
        };

        self.value_store
            .write()
            .await
            .store(key.clone(), value.clone());

        let closest_nodes = self.find_closest_nodes(&key, self.config.k).await?;
        for node in closest_nodes {
            self.store_value_on_node(&node, key.clone(), value.clone())
                .await?;
        }

        Ok(())
    }

    /// Look up nodes closest to a target
    async fn lookup_nodes(&self, target: &DhtNodeId) -> Result<Vec<NodeInfo>> {
        let key = ValueKey(target.id);
        self.find_closest_nodes(&key, self.config.k).await
    }

    /// Look up a value in the DHT
    async fn lookup_value(&self, key: &ValueKey) -> Result<Vec<StoredValue>> {
        if let Some(values) = self.value_store.read().await.get(key) {
            return Ok(values);
        }

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
        let target_id = DhtNodeId { id: key.0 };
        let mut closest = self
            .routing_table
            .read()
            .await
            .get_closest_nodes(&target_id, k);

        let mut queried = HashSet::new();
        let mut to_query = closest.clone();

        while !to_query.is_empty() && closest.len() < k {
            let node = to_query.remove(0);
            if queried.contains(&node.id) {
                continue;
            }
            queried.insert(node.id.clone());

            if let Ok(ResponseData::Peers(peers)) = self
                .transport
                .send_request(&node.id, RequestType::GetPeers)
                .await
            {
                for peer_id in peers {
                    if !queried.contains(&peer_id) {
                        // Peer info retrieval pending full implementation
                    }
                }
            }

            to_query.sort_by_key(|n| target_id.distance(&n.id));
            to_query.truncate(k);

            closest.extend(to_query.iter().cloned());
            closest.sort_by_key(|n| target_id.distance(&n.id));
            closest.truncate(k);
        }

        Ok(closest)
    }

    /// Store a value on a specific node
    async fn store_value_on_node(
        &self,
        _node: &NodeInfo,
        _key: ValueKey,
        _value: StoredValue,
    ) -> Result<()> {
        // Pending: store request implementation
        Ok(())
    }

    /// Query a value from a specific node
    async fn query_value_from_node(
        &self,
        _node: &NodeInfo,
        _key: &ValueKey,
    ) -> Result<Vec<StoredValue>> {
        // Pending: value query implementation
        Ok(Vec::new())
    }
}

// For testing/compilation - implement getrandom
mod getrandom {
    pub fn getrandom(buf: &mut [u8]) -> Result<(), ()> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        for (i, byte) in buf.iter_mut().enumerate().take(8) {
            *byte = ((nanos >> (i * 8)) & 0xFF) as u8;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use routing_table::RoutingTable;

    #[test]
    fn test_node_id_distance() {
        let id1 = DhtNodeId { id: [0u8; 32] };
        let id2 = DhtNodeId { id: [1u8; 32] };

        let distance = id1.distance(&id2);
        assert_eq!(distance.0[0], 1);
    }

    #[test]
    fn test_value_key_generation() {
        let package_id = AssetPackageId::new_v4();
        let key1 = ValueKey::from_package_id(&package_id);
        let key2 = ValueKey::from_package_id(&package_id);

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_bucket_for_key_xor_distance() {
        let local = DhtNodeId { id: [0u8; 32] };
        let mut remote_id = [0u8; 32];
        remote_id[0] = 0x80;
        let remote = DhtNodeId { id: remote_id };
        let bucket = RoutingTable::bucket_for_key(&local, &remote);
        assert_eq!(bucket, 7, "highest bit in byte 0 should be bucket 7");

        let mut remote_id2 = [0u8; 32];
        remote_id2[0] = 0x01;
        let remote2 = DhtNodeId { id: remote_id2 };
        let bucket2 = RoutingTable::bucket_for_key(&local, &remote2);
        assert_eq!(bucket2, 0, "lowest bit in byte 0 should be bucket 0");
    }

    #[test]
    fn test_routing_table_add_and_evict() {
        let local_id = DhtNodeId { id: [0u8; 32] };
        let mut config = DhtConfig::default();
        config.k = 2;
        let mut rt = RoutingTable::new(config);

        for i in 0..3u8 {
            let mut id = [0u8; 32];
            id[0] = 0x80;
            id[31] = i;
            let node = NodeInfo {
                id: DhtNodeId { id },
                address: format!("[::1]:{}", 1000 + i as u16)
                    .parse()
                    .expect("test: parse addr"),
                last_seen: SystemTime::now(),
                rtt: None,
            };
            rt.add_node_with_local_id(&local_id, node);
        }

        let bucket = &rt.buckets[7];
        assert_eq!(bucket.nodes.len(), 2, "bucket should be at capacity k=2");
        assert_eq!(
            bucket.replacements.len(),
            1,
            "overflow should go to replacements"
        );
    }

    #[tokio::test]
    async fn test_refresh_routing_table_evicts_stale() {
        let mut config = DhtConfig::default();
        config.k = 5;
        config.node_ttl = Duration::from_secs(1);
        let rt = Arc::new(RwLock::new(RoutingTable::new(config)));

        {
            let mut table = rt.write().await;
            let node = NodeInfo {
                id: DhtNodeId { id: [1u8; 32] },
                address: "[::1]:2000".parse().expect("test: parse addr"),
                last_seen: SystemTime::now() - Duration::from_secs(10),
                rtt: None,
            };
            table.buckets[0].nodes.push(node);
        }

        DhtNetwork::refresh_routing_table(rt.clone())
            .await
            .expect("test: refresh should succeed");

        let table = rt.read().await;
        assert_eq!(
            table.buckets[0].nodes.len(),
            0,
            "stale node should be evicted"
        );
    }

    #[tokio::test]
    async fn test_republish_values_refreshes_timestamps() {
        let vs = Arc::new(RwLock::new(ValueStore::new()));

        {
            let mut store = vs.write().await;
            let key = ValueKey([42u8; 32]);
            let value = StoredValue {
                data: ValueData::SearchIndex {
                    keyword: "test".to_string(),
                    packages: vec![],
                },
                publisher: DhtNodeId { id: [0u8; 32] },
                published_at: SystemTime::now() - Duration::from_secs(900),
                expires_at: SystemTime::now() + Duration::from_secs(100),
            };
            store.store(key, value);
        }

        DhtNetwork::republish_values(vs.clone())
            .await
            .expect("test: republish should succeed");

        let store = vs.read().await;
        let key = ValueKey([42u8; 32]);
        let values = store.get(&key).expect("test: value should exist");
        assert_eq!(values.len(), 1);
        let remaining = values[0]
            .expires_at
            .duration_since(SystemTime::now())
            .unwrap_or(Duration::from_secs(0));
        assert!(
            remaining.as_secs() > 100,
            "expiration should be extended after republish"
        );
    }
}
