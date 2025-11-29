//! STOQ Edge Network - CDN edge node management and caching

use std::collections::{HashMap, BTreeMap};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use parking_lot::RwLock;
use dashmap::DashMap;
use bytes::Bytes;
use serde::{Serialize, Deserialize};
use tracing::{info, debug, warn};
use crate::routing::GeoLocation;
use crate::chunking::ChunkId;

pub mod cache;
pub mod replication;
pub mod prefetch;

use cache::{CacheManager, CachePolicy};
use replication::ReplicationManager;
use prefetch::PrefetchEngine;

/// Edge node identifier
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct EdgeNodeId(pub String);

impl EdgeNodeId {
    /// Create a new edge node ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Edge node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeNode {
    /// Unique node identifier
    pub id: EdgeNodeId,
    /// Geographic location
    pub location: GeoLocation,
    /// Node capacity
    pub capacity: EdgeCapacity,
    /// Current status
    pub status: EdgeStatus,
    /// Cache configuration
    pub cache_config: CacheConfig,
    /// Network metrics
    pub metrics: EdgeMetrics,
    /// Supported regions
    pub regions: Vec<String>,
    /// Last health check
    pub last_health_check: SystemTime,
}

impl EdgeNode {
    /// Create a new edge node
    pub fn new(id: EdgeNodeId, location: GeoLocation, capacity: EdgeCapacity) -> Self {
        Self {
            id,
            location,
            capacity,
            status: EdgeStatus::Initializing,
            cache_config: CacheConfig::default(),
            metrics: EdgeMetrics::default(),
            regions: Vec::new(),
            last_health_check: SystemTime::now(),
        }
    }
    
    /// Activate the node (transition from Initializing to Active)
    pub fn activate(&mut self) {
        self.status = EdgeStatus::Active;
        self.last_health_check = SystemTime::now();
    }

    /// Check if node is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self.status, EdgeStatus::Active) &&
        self.last_health_check.elapsed().unwrap_or(Duration::from_secs(3600)) < Duration::from_secs(60)
    }
    
    /// Calculate available capacity
    pub fn available_capacity(&self) -> f64 {
        let used_percent = (self.metrics.cache_used as f64 / self.capacity.cache_size as f64) * 100.0;
        100.0 - used_percent
    }
}

/// Edge node capacity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeCapacity {
    /// Cache size in bytes
    pub cache_size: u64,
    /// Maximum bandwidth in Mbps
    pub bandwidth_mbps: f64,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// CPU cores available
    pub cpu_cores: u32,
    /// Memory in GB
    pub memory_gb: u32,
}

impl Default for EdgeCapacity {
    fn default() -> Self {
        Self {
            cache_size: 10 * 1024 * 1024 * 1024, // 10GB
            bandwidth_mbps: 10000.0, // 10 Gbps
            max_connections: 10000,
            cpu_cores: 8,
            memory_gb: 16,
        }
    }
}

/// Edge node status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeStatus {
    /// Node is initializing
    Initializing,
    /// Node is active and healthy
    Active,
    /// Node is degraded but operational
    Degraded,
    /// Node is draining (preparing for maintenance)
    Draining,
    /// Node is in maintenance
    Maintenance,
    /// Node is offline
    Offline,
}

/// Edge node metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EdgeMetrics {
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Bytes served
    pub bytes_served: u64,
    /// Current cache usage in bytes
    pub cache_used: u64,
    /// Active connections
    pub active_connections: usize,
    /// Average response time in ms
    pub avg_response_time_ms: u64,
    /// Bandwidth usage in Mbps
    pub bandwidth_used_mbps: f64,
}

impl EdgeMetrics {
    /// Calculate cache hit ratio
    pub fn cache_hit_ratio(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Cache policy
    pub policy: CachePolicy,
    /// TTL for cached content
    pub default_ttl: Duration,
    /// Maximum object size to cache
    pub max_object_size: usize,
    /// Enable prefetching
    pub enable_prefetch: bool,
    /// Prefetch threshold (popularity score)
    pub prefetch_threshold: f64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            policy: CachePolicy::Lru,
            default_ttl: Duration::from_secs(3600), // 1 hour
            max_object_size: 100 * 1024 * 1024, // 100MB
            enable_prefetch: true,
            prefetch_threshold: 0.7,
        }
    }
}

/// Edge network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeConfig {
    /// Maximum number of edge nodes
    pub max_nodes: usize,
    /// Replication factor
    pub replication_factor: u32,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Cache synchronization interval
    pub sync_interval: Duration,
    /// Geographic distribution strategy
    pub distribution_strategy: DistributionStrategy,
}

impl Default for EdgeConfig {
    fn default() -> Self {
        Self {
            max_nodes: 1000,
            replication_factor: 3,
            health_check_interval: Duration::from_secs(30),
            sync_interval: Duration::from_secs(10),
            distribution_strategy: DistributionStrategy::Geographic,
        }
    }
}

/// Distribution strategy for edge nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DistributionStrategy {
    /// Distribute based on geographic location
    Geographic,
    /// Distribute based on network topology
    NetworkTopology,
    /// Distribute based on load
    LoadBased,
    /// Hybrid strategy
    Hybrid,
}

/// Edge cache for content storage
pub struct EdgeCache {
    node_id: EdgeNodeId,
    cache: Arc<CacheManager>,
    metrics: Arc<RwLock<EdgeMetrics>>,
}

impl EdgeCache {
    /// Create a new edge cache
    pub fn new(node_id: EdgeNodeId, config: CacheConfig) -> Result<Self> {
        let cache = Arc::new(CacheManager::new(config.policy, config.max_object_size)?);
        let metrics = Arc::new(RwLock::new(EdgeMetrics::default()));
        
        Ok(Self {
            node_id,
            cache,
            metrics,
        })
    }
    
    /// Store content in cache
    pub async fn store(&self, key: String, content: Bytes, ttl: Duration) -> Result<()> {
        self.cache.put(key, content, ttl).await?;
        let mut metrics = self.metrics.write();
        metrics.cache_used = self.cache.size();
        Ok(())
    }
    
    /// Retrieve content from cache
    pub async fn get(&self, key: &str) -> Option<Bytes> {
        let result = self.cache.get(key).await;
        let mut metrics = self.metrics.write();
        
        if result.is_some() {
            metrics.cache_hits += 1;
        } else {
            metrics.cache_misses += 1;
        }
        
        result
    }
    
    /// Invalidate cached content
    pub async fn invalidate(&self, key: &str) -> Result<()> {
        self.cache.remove(key).await;
        let mut metrics = self.metrics.write();
        metrics.cache_used = self.cache.size();
        Ok(())
    }
    
    /// Get cache metrics
    pub fn metrics(&self) -> EdgeMetrics {
        self.metrics.read().clone()
    }
}

/// Main edge network implementation
pub struct StoqEdgeNetwork {
    config: EdgeConfig,
    nodes: Arc<DashMap<EdgeNodeId, Arc<EdgeNode>>>,
    node_cache: Arc<DashMap<EdgeNodeId, Arc<EdgeCache>>>,
    geo_index: Arc<RwLock<BTreeMap<String, Vec<EdgeNodeId>>>>,
    replication: Arc<ReplicationManager>,
    prefetch: Arc<PrefetchEngine>,
    stats: Arc<RwLock<crate::EdgeStats>>,
}

impl StoqEdgeNetwork {
    /// Create a new edge network
    pub async fn new(config: EdgeConfig) -> Result<Self> {
        info!("Initializing STOQ edge network with {} strategy", 
              match config.distribution_strategy {
                  DistributionStrategy::Geographic => "Geographic",
                  DistributionStrategy::NetworkTopology => "Network Topology",
                  DistributionStrategy::LoadBased => "Load Based",
                  DistributionStrategy::Hybrid => "Hybrid",
              });
        
        let nodes = Arc::new(DashMap::new());
        let node_cache = Arc::new(DashMap::new());
        let geo_index = Arc::new(RwLock::new(BTreeMap::new()));
        let replication = Arc::new(ReplicationManager::new(config.replication_factor)?);
        let prefetch = Arc::new(PrefetchEngine::new()?);
        let stats = Arc::new(RwLock::new(crate::EdgeStats {
            edge_nodes: 0,
            total_cache_size: 0,
            cache_hit_ratio: 0.0,
            avg_response_time_ms: 0,
        }));
        
        Ok(Self {
            config,
            nodes,
            node_cache,
            geo_index,
            replication,
            prefetch,
            stats,
        })
    }
    
    /// Register an edge node
    pub async fn register_edge(&self, mut node: EdgeNode) -> Result<()> {
        info!("Registering edge node {:?} at {:?}", node.id, node.location);
        
        // Validate node
        if self.nodes.len() >= self.config.max_nodes {
            return Err(anyhow!("Maximum edge nodes reached"));
        }
        
        // Update status
        node.status = EdgeStatus::Active;
        node.last_health_check = SystemTime::now();
        
        // Create cache for node
        let cache = Arc::new(EdgeCache::new(node.id.clone(), node.cache_config.clone())?);
        self.node_cache.insert(node.id.clone(), cache);
        
        // Update geographic index
        {
            let mut geo_index = self.geo_index.write();
            geo_index.entry(node.location.country.clone())
                .or_insert_with(Vec::new)
                .push(node.id.clone());
        }
        
        // Store node
        self.nodes.insert(node.id.clone(), Arc::new(node.clone()));
        
        // Update replication topology
        self.replication.add_node(node.id.clone(), node.location.clone())?;
        
        // Update stats
        self.update_stats();
        
        Ok(())
    }
    
    /// Find nearest edge node to a location
    pub async fn find_nearest(&self, location: GeoLocation) -> Result<EdgeNode> {
        debug!("Finding nearest edge node to {:?}", location);
        
        let mut nearest: Option<(EdgeNode, f64)> = None;
        
        for entry in self.nodes.iter() {
            let node = entry.value();
            
            // Skip unhealthy nodes
            if !node.is_healthy() {
                continue;
            }
            
            // Calculate distance
            let distance = location.distance_km(&node.location);
            
            // Check if this is the nearest so far
            if nearest.is_none() || distance < nearest.as_ref().unwrap().1 {
                // Also consider load
                let load_factor = 1.0 + (node.metrics.active_connections as f64 / 
                                        node.capacity.max_connections as f64);
                let weighted_distance = distance * load_factor;
                
                if nearest.is_none() || weighted_distance < nearest.as_ref().unwrap().1 {
                    nearest = Some(((**node).clone(), weighted_distance));
                }
            }
        }
        
        nearest.map(|(node, _)| node)
            .ok_or_else(|| anyhow!("No healthy edge nodes available"))
    }
    
    /// Find multiple edge nodes for redundancy
    pub async fn find_multiple(&self, location: GeoLocation, count: usize) -> Result<Vec<EdgeNode>> {
        debug!("Finding {} edge nodes near {:?}", count, location);
        
        // Collect all healthy nodes with distances
        let mut nodes_with_distance: Vec<(EdgeNode, f64)> = Vec::new();
        
        for entry in self.nodes.iter() {
            let node = entry.value();
            
            if node.is_healthy() {
                let distance = location.distance_km(&node.location);
                let load_factor = 1.0 + (node.metrics.active_connections as f64 / 
                                        node.capacity.max_connections as f64);
                let weighted_distance = distance * load_factor;
                nodes_with_distance.push(((**node).clone(), weighted_distance));
            }
        }
        
        // Sort by weighted distance
        nodes_with_distance.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        
        // Take requested count
        let result: Vec<EdgeNode> = nodes_with_distance
            .into_iter()
            .take(count)
            .map(|(node, _)| node)
            .collect();
        
        if result.is_empty() {
            Err(anyhow!("No healthy edge nodes available"))
        } else {
            Ok(result)
        }
    }
    
    /// Cache content at edge nodes
    pub async fn cache_content(&self, content: crate::Content) -> Result<()> {
        debug!("Caching content {} across edge network", content.id.0);
        
        // Determine replication targets
        let targets = self.replication.get_targets(&content.id.0)?;
        
        // Cache at each target
        for node_id in targets {
            if let Some(cache) = self.node_cache.get(&EdgeNodeId::new(node_id)) {
                cache.store(
                    content.id.0.clone(),
                    content.data.clone(),
                    Duration::from_secs(content.ttl),
                ).await?;
            }
        }
        
        // Update prefetch predictions if enabled
        if self.config.replication_factor > 1 {
            self.prefetch.update_access_pattern(&content.id.0).await?;
        }
        
        Ok(())
    }
    
    /// Invalidate cached content
    pub async fn invalidate_cache(&self, content_id: crate::ContentId) -> Result<()> {
        debug!("Invalidating content {} across edge network", content_id.0);
        
        // Invalidate at all nodes
        for cache in self.node_cache.iter() {
            cache.invalidate(&content_id.0).await?;
        }
        
        Ok(())
    }
    
    /// Perform health check on all nodes
    pub async fn health_check(&self) -> Result<()> {
        debug!("Performing health check on {} edge nodes", self.nodes.len());
        
        for mut entry in self.nodes.iter_mut() {
            let node = entry.value_mut();
            
            // Simple health check - in production, would ping the node
            let node_mut = Arc::make_mut(node);
            node_mut.last_health_check = SystemTime::now();
            
            // Update status based on metrics
            if node_mut.metrics.avg_response_time_ms > 1000 {
                node_mut.status = EdgeStatus::Degraded;
            } else if node_mut.available_capacity() < 10.0 {
                node_mut.status = EdgeStatus::Degraded;
            } else {
                node_mut.status = EdgeStatus::Active;
            }
        }
        
        self.update_stats();
        Ok(())
    }
    
    /// Update network statistics
    fn update_stats(&self) {
        let mut total_cache_size = 0u64;
        let mut total_hits = 0u64;
        let mut total_misses = 0u64;
        let mut total_response_time = 0u64;
        let mut node_count = 0usize;
        
        for entry in self.nodes.iter() {
            let node = entry.value();
            if node.is_healthy() {
                total_cache_size += node.capacity.cache_size;
                total_hits += node.metrics.cache_hits;
                total_misses += node.metrics.cache_misses;
                total_response_time += node.metrics.avg_response_time_ms;
                node_count += 1;
            }
        }
        
        let mut stats = self.stats.write();
        stats.edge_nodes = node_count;
        stats.total_cache_size = total_cache_size;
        stats.cache_hit_ratio = if total_hits + total_misses > 0 {
            total_hits as f64 / (total_hits + total_misses) as f64
        } else {
            0.0
        };
        stats.avg_response_time_ms = if node_count > 0 {
            total_response_time / node_count as u64
        } else {
            0
        };
    }
    
    /// Get edge network statistics
    pub fn stats(&self) -> crate::EdgeStats {
        self.stats.read().clone()
    }
}

#[async_trait]
impl crate::EdgeNetwork for StoqEdgeNetwork {
    async fn register_edge(&self, node: EdgeNode) -> Result<()> {
        self.register_edge(node).await
    }
    
    async fn find_nearest(&self, location: GeoLocation) -> Result<EdgeNode> {
        self.find_nearest(location).await
    }
    
    async fn cache_content(&self, content: crate::Content) -> Result<()> {
        self.cache_content(content).await
    }
    
    async fn invalidate_cache(&self, content_id: crate::ContentId) -> Result<()> {
        self.invalidate_cache(content_id).await
    }
    
    fn stats(&self) -> crate::EdgeStats {
        self.stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_edge_node_creation() {
        let location = GeoLocation {
            lat: 37.7749,
            lon: -122.4194,
            country: "US".to_string(),
            city: Some("San Francisco".to_string()),
            asn: None,
        };
        
        let mut node = EdgeNode::new(
            EdgeNodeId::new("edge1"),
            location,
            EdgeCapacity::default(),
        );

        // Activate the node to make it healthy
        node.activate();

        assert_eq!(node.id.0, "edge1");
        assert!(node.is_healthy());
    }
    
    #[tokio::test]
    async fn test_edge_network_creation() {
        let config = EdgeConfig::default();
        let network = StoqEdgeNetwork::new(config).await;
        assert!(network.is_ok());
    }
    
    #[tokio::test]
    async fn test_edge_registration() {
        let config = EdgeConfig::default();
        let network = StoqEdgeNetwork::new(config).await.unwrap();
        
        let location = GeoLocation {
            lat: 37.7749,
            lon: -122.4194,
            country: "US".to_string(),
            city: Some("San Francisco".to_string()),
            asn: None,
        };
        
        let node = EdgeNode::new(
            EdgeNodeId::new("edge1"),
            location,
            EdgeCapacity::default(),
        );
        
        let result = network.register_edge(node).await;
        assert!(result.is_ok());
        
        let stats = network.stats();
        assert_eq!(stats.edge_nodes, 1);
    }
}