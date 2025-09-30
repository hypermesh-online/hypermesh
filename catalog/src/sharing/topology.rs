//! Network Topology and Routing Module
//!
//! Manages network topology awareness, optimal routing strategies,
//! and fault tolerance for the decentralized sharing network.

use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime};

use super::PeerInfo;

/// Node location information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeLocation {
    /// Geographic region
    pub region: String,
    /// Country code
    pub country: String,
    /// City
    pub city: Option<String>,
    /// Latitude
    pub latitude: Option<f64>,
    /// Longitude
    pub longitude: Option<f64>,
    /// Data center/provider
    pub provider: Option<String>,
    /// Network ASN
    pub asn: Option<u32>,
}

impl NodeLocation {
    /// Calculate distance to another location (in km)
    pub fn distance_to(&self, other: &NodeLocation) -> f64 {
        if let (Some(lat1), Some(lon1), Some(lat2), Some(lon2)) =
            (self.latitude, self.longitude, other.latitude, other.longitude) {
            // Haversine formula
            let r = 6371.0; // Earth radius in km
            let dlat = (lat2 - lat1).to_radians();
            let dlon = (lon2 - lon1).to_radians();
            let a = (dlat / 2.0).sin() * (dlat / 2.0).sin() +
                    lat1.to_radians().cos() * lat2.to_radians().cos() *
                    (dlon / 2.0).sin() * (dlon / 2.0).sin();
            let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
            r * c
        } else {
            // Fallback to region-based distance
            if self.region == other.region {
                100.0 // Same region
            } else if self.country == other.country {
                500.0 // Same country
            } else {
                2000.0 // Different countries
            }
        }
    }
}

/// Routing strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingStrategy {
    /// Shortest path routing
    ShortestPath,
    /// Lowest latency routing
    LowestLatency,
    /// Highest bandwidth routing
    HighestBandwidth,
    /// Geographic proximity
    GeographicProximity,
    /// Load balanced routing
    LoadBalanced,
    /// Fault tolerant routing (multiple paths)
    FaultTolerant { redundancy: u32 },
}

/// Network link between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLink {
    /// Source node ID
    pub from: String,
    /// Destination node ID
    pub to: String,
    /// Link latency (ms)
    pub latency: u64,
    /// Available bandwidth (bytes/sec)
    pub bandwidth: u64,
    /// Packet loss rate (0-1)
    pub packet_loss: f64,
    /// Link reliability score (0-1)
    pub reliability: f64,
    /// Last measured
    pub last_measured: SystemTime,
}

/// Node status in the network
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeStatus {
    /// Node is online and healthy
    Online,
    /// Node is degraded (high latency/packet loss)
    Degraded,
    /// Node is offline
    Offline,
    /// Node is in maintenance
    Maintenance,
    /// Node status unknown
    Unknown,
}

/// Network node information
#[derive(Debug, Clone)]
pub struct NetworkNode {
    /// Node ID
    pub id: String,
    /// Node address
    pub address: String,
    /// Node location
    pub location: Option<NodeLocation>,
    /// Node status
    pub status: NodeStatus,
    /// Connected peers
    pub peers: HashSet<String>,
    /// Node capacity metrics
    pub capacity: NodeCapacity,
    /// Last health check
    pub last_health_check: SystemTime,
}

/// Node capacity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapacity {
    /// CPU utilization (0-1)
    pub cpu_usage: f64,
    /// Memory usage (0-1)
    pub memory_usage: f64,
    /// Storage usage (0-1)
    pub storage_usage: f64,
    /// Network utilization (0-1)
    pub network_usage: f64,
    /// Maximum connections
    pub max_connections: u32,
    /// Current connections
    pub current_connections: u32,
}

impl Default for NodeCapacity {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            storage_usage: 0.0,
            network_usage: 0.0,
            max_connections: 1000,
            current_connections: 0,
        }
    }
}

/// Network topology manager
pub struct NetworkTopology {
    local_node_id: String,
    nodes: Arc<RwLock<HashMap<String, NetworkNode>>>,
    links: Arc<RwLock<Vec<NetworkLink>>>,
    routing_cache: Arc<RwLock<HashMap<(String, String), Vec<String>>>>,
    routing_strategy: Arc<RwLock<RoutingStrategy>>,
    topology_version: Arc<RwLock<u64>>,
}

impl NetworkTopology {
    /// Create new network topology
    pub fn new(local_node_id: String) -> Self {
        Self {
            local_node_id,
            nodes: Arc::new(RwLock::new(HashMap::new())),
            links: Arc::new(RwLock::new(Vec::new())),
            routing_cache: Arc::new(RwLock::new(HashMap::new())),
            routing_strategy: Arc::new(RwLock::new(RoutingStrategy::ShortestPath)),
            topology_version: Arc::new(RwLock::new(0)),
        }
    }

    /// Add peer to topology
    pub async fn add_peer(&mut self, node_id: &str, address: &str) -> Result<()> {
        let mut nodes = self.nodes.write().await;

        let node = NetworkNode {
            id: node_id.to_string(),
            address: address.to_string(),
            location: None,
            status: NodeStatus::Online,
            peers: HashSet::new(),
            capacity: NodeCapacity::default(),
            last_health_check: SystemTime::now(),
        };

        nodes.insert(node_id.to_string(), node);

        // Increment topology version
        let mut version = self.topology_version.write().await;
        *version += 1;

        // Clear routing cache
        self.routing_cache.write().await.clear();

        Ok(())
    }

    /// Remove peer from topology
    pub async fn remove_peer(&mut self, node_id: &str) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        nodes.remove(node_id);

        // Remove associated links
        let mut links = self.links.write().await;
        links.retain(|link| link.from != node_id && link.to != node_id);

        // Update topology version
        let mut version = self.topology_version.write().await;
        *version += 1;

        // Clear routing cache
        self.routing_cache.write().await.clear();

        Ok(())
    }

    /// Update node information
    pub async fn update_node_info(&mut self, node_id: &str, peer_info: &PeerInfo) -> Result<()> {
        let mut nodes = self.nodes.write().await;

        if let Some(node) = nodes.get_mut(node_id) {
            node.location = peer_info.location.clone();
            node.last_health_check = SystemTime::now();

            // Update capacity based on peer info
            node.capacity.current_connections = peer_info.available_packages.len() as u32;
            node.capacity.storage_usage =
                (peer_info.storage_capacity - peer_info.bandwidth_capacity) as f64 /
                peer_info.storage_capacity as f64;
        }

        Ok(())
    }

    /// Measure link quality between nodes
    pub async fn measure_link(&self, from: &str, to: &str) -> Result<NetworkLink> {
        // Would perform actual network measurement
        // This is a simulation

        let nodes = self.nodes.read().await;
        let from_node = nodes.get(from);
        let to_node = nodes.get(to);

        let latency = if let (Some(from_n), Some(to_n)) = (from_node, to_node) {
            if let (Some(from_loc), Some(to_loc)) = (&from_n.location, &to_n.location) {
                // Estimate latency based on distance
                let distance = from_loc.distance_to(to_loc);
                (distance / 100.0) as u64 + 5 // Base 5ms + distance factor
            } else {
                50 // Default latency
            }
        } else {
            100 // High latency for unknown nodes
        };

        Ok(NetworkLink {
            from: from.to_string(),
            to: to.to_string(),
            latency,
            bandwidth: 10 * 1024 * 1024, // 10 MB/s default
            packet_loss: 0.001, // 0.1% default
            reliability: 0.99,
            last_measured: SystemTime::now(),
        })
    }

    /// Find optimal route between nodes
    pub async fn find_route(&self, from: &str, to: &str) -> Result<Vec<String>> {
        // Check cache first
        let cache = self.routing_cache.read().await;
        if let Some(cached_route) = cache.get(&(from.to_string(), to.to_string())) {
            return Ok(cached_route.clone());
        }
        drop(cache);

        let strategy = self.routing_strategy.read().await.clone();

        let route = match strategy {
            RoutingStrategy::ShortestPath => {
                self.dijkstra_shortest_path(from, to).await?
            }
            RoutingStrategy::LowestLatency => {
                self.lowest_latency_path(from, to).await?
            }
            RoutingStrategy::HighestBandwidth => {
                self.highest_bandwidth_path(from, to).await?
            }
            RoutingStrategy::GeographicProximity => {
                self.geographic_routing(from, to).await?
            }
            RoutingStrategy::LoadBalanced => {
                self.load_balanced_routing(from, to).await?
            }
            RoutingStrategy::FaultTolerant { redundancy } => {
                self.fault_tolerant_routing(from, to, redundancy).await?
            }
        };

        // Cache the route
        let mut cache = self.routing_cache.write().await;
        cache.insert((from.to_string(), to.to_string()), route.clone());

        Ok(route)
    }

    /// Dijkstra's shortest path algorithm
    async fn dijkstra_shortest_path(&self, from: &str, to: &str) -> Result<Vec<String>> {
        let nodes = self.nodes.read().await;
        let links = self.links.read().await;

        let mut distances: HashMap<String, u64> = HashMap::new();
        let mut previous: HashMap<String, Option<String>> = HashMap::new();
        let mut unvisited: HashSet<String> = nodes.keys().cloned().collect();

        // Initialize distances
        for node_id in nodes.keys() {
            distances.insert(node_id.clone(), u64::MAX);
            previous.insert(node_id.clone(), None);
        }
        distances.insert(from.to_string(), 0);

        while !unvisited.is_empty() {
            // Find unvisited node with minimum distance
            let current = unvisited.iter()
                .min_by_key(|&n| distances.get(n).unwrap_or(&u64::MAX))
                .cloned();

            if let Some(current_node) = current {
                if current_node == to {
                    break;
                }

                unvisited.remove(&current_node);

                // Update distances to neighbors
                for link in links.iter() {
                    if link.from == current_node && unvisited.contains(&link.to) {
                        let alt = distances[&current_node].saturating_add(1);
                        if alt < distances[&link.to] {
                            distances.insert(link.to.clone(), alt);
                            previous.insert(link.to.clone(), Some(current_node.clone()));
                        }
                    }
                }
            } else {
                break;
            }
        }

        // Reconstruct path
        let mut path = Vec::new();
        let mut current = Some(to.to_string());

        while let Some(node) = current {
            path.push(node.clone());
            current = previous.get(&node).and_then(|p| p.clone());
            if current.as_ref() == Some(&from.to_string()) {
                path.push(from.to_string());
                break;
            }
        }

        path.reverse();

        if path.is_empty() || path[0] != from {
            return Err(anyhow::anyhow!("No route found"));
        }

        Ok(path)
    }

    /// Find lowest latency path
    async fn lowest_latency_path(&self, from: &str, to: &str) -> Result<Vec<String>> {
        let nodes = self.nodes.read().await;
        let links = self.links.read().await;

        let mut distances: HashMap<String, u64> = HashMap::new();
        let mut previous: HashMap<String, Option<String>> = HashMap::new();
        let mut unvisited: HashSet<String> = nodes.keys().cloned().collect();

        // Initialize with max latency
        for node_id in nodes.keys() {
            distances.insert(node_id.clone(), u64::MAX);
            previous.insert(node_id.clone(), None);
        }
        distances.insert(from.to_string(), 0);

        while !unvisited.is_empty() {
            let current = unvisited.iter()
                .min_by_key(|&n| distances.get(n).unwrap_or(&u64::MAX))
                .cloned();

            if let Some(current_node) = current {
                if current_node == to {
                    break;
                }

                unvisited.remove(&current_node);

                // Update based on latency
                for link in links.iter() {
                    if link.from == current_node && unvisited.contains(&link.to) {
                        let alt = distances[&current_node].saturating_add(link.latency);
                        if alt < distances[&link.to] {
                            distances.insert(link.to.clone(), alt);
                            previous.insert(link.to.clone(), Some(current_node.clone()));
                        }
                    }
                }
            } else {
                break;
            }
        }

        // Reconstruct path
        self.reconstruct_path(from, to, &previous)
    }

    /// Find highest bandwidth path
    async fn highest_bandwidth_path(&self, from: &str, to: &str) -> Result<Vec<String>> {
        // Would implement max-flow algorithm
        // Simplified version using greedy approach
        self.dijkstra_shortest_path(from, to).await
    }

    /// Geographic proximity routing
    async fn geographic_routing(&self, from: &str, to: &str) -> Result<Vec<String>> {
        let nodes = self.nodes.read().await;

        let target_location = nodes.get(to)
            .and_then(|n| n.location.as_ref())
            .ok_or_else(|| anyhow::anyhow!("Target location unknown"))?;

        // Route through geographically closer nodes
        let mut current = from.to_string();
        let mut path = vec![current.clone()];
        let mut visited = HashSet::new();

        while current != to {
            visited.insert(current.clone());

            // Find closest neighbor to target
            let current_node = nodes.get(&current)
                .ok_or_else(|| anyhow::anyhow!("Node not found"))?;

            let mut best_next = None;
            let mut best_distance = f64::MAX;

            for peer_id in &current_node.peers {
                if visited.contains(peer_id) {
                    continue;
                }

                if let Some(peer) = nodes.get(peer_id) {
                    if let Some(peer_location) = &peer.location {
                        let distance = peer_location.distance_to(target_location);
                        if distance < best_distance {
                            best_distance = distance;
                            best_next = Some(peer_id.clone());
                        }
                    }
                }
            }

            if let Some(next) = best_next {
                path.push(next.clone());
                current = next;
            } else {
                // Fall back to shortest path
                return self.dijkstra_shortest_path(&current, to).await
                    .map(|mut sub_path| {
                        path.append(&mut sub_path[1..].to_vec());
                        path
                    });
            }
        }

        Ok(path)
    }

    /// Load balanced routing
    async fn load_balanced_routing(&self, from: &str, to: &str) -> Result<Vec<String>> {
        let nodes = self.nodes.read().await;

        // Find path avoiding overloaded nodes
        let mut path = self.dijkstra_shortest_path(from, to).await?;

        // Check for overloaded nodes
        let mut overloaded = Vec::new();
        for node_id in &path[1..path.len()-1] { // Skip source and destination
            if let Some(node) = nodes.get(node_id) {
                if node.capacity.network_usage > 0.8 ||
                   node.capacity.current_connections as f64 / node.capacity.max_connections as f64 > 0.8 {
                    overloaded.push(node_id.clone());
                }
            }
        }

        // If overloaded nodes found, find alternative path
        if !overloaded.is_empty() {
            // Would implement alternative path finding avoiding overloaded nodes
            // For now, return original path
        }

        Ok(path)
    }

    /// Fault tolerant routing with multiple paths
    async fn fault_tolerant_routing(
        &self,
        from: &str,
        to: &str,
        redundancy: u32,
    ) -> Result<Vec<String>> {
        // Find multiple disjoint paths
        let mut paths = Vec::new();
        let mut excluded_nodes = HashSet::new();

        for _ in 0..redundancy {
            // Find path excluding already used nodes
            let path = self.find_disjoint_path(from, to, &excluded_nodes).await?;

            // Add intermediate nodes to excluded set
            for node in &path[1..path.len()-1] {
                excluded_nodes.insert(node.clone());
            }

            paths.push(path);
        }

        // Return primary path (could return all for redundancy)
        Ok(paths.into_iter().next().unwrap_or_else(Vec::new))
    }

    /// Find path avoiding specific nodes
    async fn find_disjoint_path(
        &self,
        from: &str,
        to: &str,
        excluded: &HashSet<String>,
    ) -> Result<Vec<String>> {
        let nodes = self.nodes.read().await;
        let links = self.links.read().await;

        let mut distances: HashMap<String, u64> = HashMap::new();
        let mut previous: HashMap<String, Option<String>> = HashMap::new();
        let mut unvisited: HashSet<String> = nodes.keys()
            .filter(|n| !excluded.contains(*n) || *n == from || *n == to)
            .cloned()
            .collect();

        // Initialize distances
        for node_id in &unvisited {
            distances.insert(node_id.clone(), u64::MAX);
            previous.insert(node_id.clone(), None);
        }
        distances.insert(from.to_string(), 0);

        while !unvisited.is_empty() {
            let current = unvisited.iter()
                .min_by_key(|&n| distances.get(n).unwrap_or(&u64::MAX))
                .cloned();

            if let Some(current_node) = current {
                if current_node == to {
                    break;
                }

                unvisited.remove(&current_node);

                for link in links.iter() {
                    if link.from == current_node &&
                       unvisited.contains(&link.to) &&
                       !excluded.contains(&link.to) {
                        let alt = distances[&current_node].saturating_add(1);
                        if alt < distances[&link.to] {
                            distances.insert(link.to.clone(), alt);
                            previous.insert(link.to.clone(), Some(current_node.clone()));
                        }
                    }
                }
            } else {
                break;
            }
        }

        self.reconstruct_path(from, to, &previous)
    }

    /// Reconstruct path from previous nodes map
    fn reconstruct_path(
        &self,
        from: &str,
        to: &str,
        previous: &HashMap<String, Option<String>>,
    ) -> Result<Vec<String>> {
        let mut path = Vec::new();
        let mut current = Some(to.to_string());

        while let Some(node) = current {
            path.push(node.clone());
            current = previous.get(&node).and_then(|p| p.clone());
            if current.as_ref() == Some(&from.to_string()) {
                path.push(from.to_string());
                break;
            }
        }

        path.reverse();

        if path.is_empty() || path[0] != from {
            return Err(anyhow::anyhow!("No route found"));
        }

        Ok(path)
    }

    /// Optimize routing strategy based on network conditions
    pub async fn optimize_routing(&mut self) -> Result<()> {
        let nodes = self.nodes.read().await;
        let links = self.links.read().await;

        // Calculate network metrics
        let avg_latency: f64 = links.iter()
            .map(|l| l.latency as f64)
            .sum::<f64>() / links.len().max(1) as f64;

        let avg_load: f64 = nodes.values()
            .map(|n| n.capacity.network_usage)
            .sum::<f64>() / nodes.len().max(1) as f64;

        // Choose strategy based on conditions
        let mut strategy = self.routing_strategy.write().await;
        *strategy = if avg_latency > 100.0 {
            RoutingStrategy::LowestLatency
        } else if avg_load > 0.7 {
            RoutingStrategy::LoadBalanced
        } else {
            RoutingStrategy::ShortestPath
        };

        // Clear routing cache for new strategy
        self.routing_cache.write().await.clear();

        Ok(())
    }

    /// Get distance score between nodes (0-1, lower is better)
    pub fn get_distance_score(&self, from: &str, to: &str) -> f64 {
        // Would calculate based on actual routing distance
        // Simplified version
        if from == to {
            0.0
        } else {
            0.5
        }
    }

    /// Handle network partition
    pub async fn handle_partition(&mut self, partition: Vec<String>) -> Result<()> {
        let mut nodes = self.nodes.write().await;

        // Mark partitioned nodes as offline
        for node_id in partition {
            if let Some(node) = nodes.get_mut(&node_id) {
                node.status = NodeStatus::Offline;
            }
        }

        // Clear routing cache
        self.routing_cache.write().await.clear();

        // Increment topology version
        let mut version = self.topology_version.write().await;
        *version += 1;

        Ok(())
    }

    /// Recover from network partition
    pub async fn recover_partition(&mut self, recovered: Vec<String>) -> Result<()> {
        let mut nodes = self.nodes.write().await;

        // Mark recovered nodes as online
        for node_id in recovered {
            if let Some(node) = nodes.get_mut(&node_id) {
                node.status = NodeStatus::Online;
            }
        }

        // Re-measure links
        // Would trigger link quality measurements

        // Clear routing cache
        self.routing_cache.write().await.clear();

        // Increment topology version
        let mut version = self.topology_version.write().await;
        *version += 1;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_distance() {
        let loc1 = NodeLocation {
            region: "US-East".to_string(),
            country: "US".to_string(),
            city: Some("New York".to_string()),
            latitude: Some(40.7128),
            longitude: Some(-74.0060),
            provider: None,
            asn: None,
        };

        let loc2 = NodeLocation {
            region: "US-West".to_string(),
            country: "US".to_string(),
            city: Some("San Francisco".to_string()),
            latitude: Some(37.7749),
            longitude: Some(-122.4194),
            provider: None,
            asn: None,
        };

        let distance = loc1.distance_to(&loc2);
        assert!(distance > 4000.0 && distance < 5000.0); // Approximate distance in km
    }

    #[tokio::test]
    async fn test_topology_creation() {
        let mut topology = NetworkTopology::new("local".to_string());
        let result = topology.add_peer("peer1", "192.168.1.1").await;
        assert!(result.is_ok());
    }
}