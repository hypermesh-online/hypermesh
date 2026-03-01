// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Network Topology and Routing Module
//!
//! Manages network topology awareness, optimal routing strategies,
//! and fault tolerance for the decentralized sharing network.

mod routing;
mod types;

pub use types::*;

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

use super::PeerInfo;

/// Network topology manager
pub struct NetworkTopology {
    pub(super) _local_node_id: String,
    pub(super) nodes: Arc<RwLock<HashMap<String, NetworkNode>>>,
    pub(super) links: Arc<RwLock<Vec<NetworkLink>>>,
    pub(super) routing_cache: Arc<RwLock<HashMap<(String, String), Vec<String>>>>,
    routing_strategy: Arc<RwLock<RoutingStrategy>>,
    topology_version: Arc<RwLock<u64>>,
}

impl NetworkTopology {
    /// Create new network topology
    pub fn new(local_node_id: String) -> Self {
        Self {
            _local_node_id: local_node_id,
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
            node.capacity.storage_usage = (peer_info.storage_capacity
                - peer_info.bandwidth_capacity) as f64
                / peer_info.storage_capacity as f64;
        }

        Ok(())
    }

    /// Measure link quality between nodes
    pub async fn measure_link(&self, from: &str, to: &str) -> Result<NetworkLink> {
        let nodes = self.nodes.read().await;
        let from_node = nodes.get(from);
        let to_node = nodes.get(to);

        let (distance_km, latency, bandwidth) =
            if let (Some(from_n), Some(to_n)) = (from_node, to_node) {
                if let (Some(from_loc), Some(to_loc)) = (&from_n.location, &to_n.location) {
                    let dist = from_loc.distance_to(to_loc);
                    let base_latency = (dist / 200.0) as u64 + 2;
                    let jitter = (from.len() as u64 + to.len() as u64) % 5;
                    let lat = base_latency + jitter;
                    let bw = if dist < 100.0 {
                        100 * 1024 * 1024
                    } else if dist < 1000.0 {
                        50 * 1024 * 1024
                    } else {
                        10 * 1024 * 1024
                    };
                    (dist, lat, bw)
                } else {
                    (1000.0, 50, 50 * 1024 * 1024)
                }
            } else {
                (2000.0, 100, 10 * 1024 * 1024)
            };

        let packet_loss = if distance_km < 500.0 { 0.001 } else { 0.005 };
        let reliability = 1.0 - packet_loss;

        Ok(NetworkLink {
            from: from.to_string(),
            to: to.to_string(),
            latency,
            bandwidth,
            packet_loss,
            reliability,
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
            RoutingStrategy::ShortestPath => self.dijkstra_shortest_path(from, to).await?,
            RoutingStrategy::LowestLatency => self.lowest_latency_path(from, to).await?,
            RoutingStrategy::HighestBandwidth => self.highest_bandwidth_path(from, to).await?,
            RoutingStrategy::GeographicProximity => self.geographic_routing(from, to).await?,
            RoutingStrategy::LoadBalanced => self.load_balanced_routing(from, to).await?,
            RoutingStrategy::FaultTolerant { redundancy } => {
                self.fault_tolerant_routing(from, to, redundancy).await?
            }
        };

        // Cache the route
        let mut cache = self.routing_cache.write().await;
        cache.insert((from.to_string(), to.to_string()), route.clone());

        Ok(route)
    }

    /// Optimize routing strategy based on network conditions
    pub async fn optimize_routing(&mut self) -> Result<()> {
        let nodes = self.nodes.read().await;
        let links = self.links.read().await;

        // Calculate network metrics
        let avg_latency: f64 =
            links.iter().map(|l| l.latency as f64).sum::<f64>() / links.len().max(1) as f64;

        let avg_load: f64 = nodes
            .values()
            .map(|n| n.capacity.network_usage)
            .sum::<f64>()
            / nodes.len().max(1) as f64;

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

    /// Get distance score between nodes (0-1, higher is closer).
    pub fn get_distance_score(&self, from: &str, to: &str) -> f64 {
        if from == to {
            return 1.0;
        }
        if let Ok(cache) = self.routing_cache.try_read() {
            if let Some(path) = cache.get(&(from.to_string(), to.to_string())) {
                let hops = if path.len() > 1 { path.len() - 1 } else { 1 };
                return 1.0 / (1.0 + hops as f64);
            }
        }
        if let Ok(links) = self.links.try_read() {
            let direct = links
                .iter()
                .any(|l| (l.from == from && l.to == to) || (l.from == to && l.to == from));
            if direct {
                return 0.5;
            }
        }
        0.25
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
        let now = SystemTime::now();

        // Collect peers of recovered nodes for link re-measurement
        let mut links_to_measure: Vec<(String, String)> = Vec::new();
        for node_id in &recovered {
            if let Some(node) = nodes.get_mut(node_id) {
                node.status = NodeStatus::Online;
                node.last_health_check = now;
                for peer_id in &node.peers {
                    links_to_measure.push((node_id.clone(), peer_id.clone()));
                }
            }
        }
        drop(nodes);

        // Re-measure links to recovered nodes
        let mut new_links = Vec::new();
        for (from, to) in &links_to_measure {
            if let Ok(link) = self.measure_link(from, to).await {
                new_links.push(link);
            }
        }

        let mut links = self.links.write().await;
        for new_link in new_links {
            if let Some(existing) = links
                .iter_mut()
                .find(|l| l.from == new_link.from && l.to == new_link.to)
            {
                *existing = new_link;
            } else {
                links.push(new_link);
            }
        }
        drop(links);

        self.routing_cache.write().await.clear();

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
