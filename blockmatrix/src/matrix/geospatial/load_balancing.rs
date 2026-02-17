// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Geographic-aware load balancing
//!
//! Implements load distribution algorithms that consider
//! geographic proximity and zone boundaries.

use crate::matrix::coordinate::MatrixCoordinate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Load balancing strategy
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    /// Round-robin distribution
    RoundRobin,
    /// Nearest neighbor (geographic proximity)
    NearestNeighbor,
    /// Zone-based (keep within geographic zone)
    ZoneBased,
    /// Latency-aware (prefer low-latency nodes)
    LatencyAware,
    /// Weighted by capacity
    WeightedCapacity,
}

/// Node load information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeLoad {
    /// Node coordinate
    pub coordinate: MatrixCoordinate,
    /// Current load (0.0 to 1.0)
    pub current_load: f64,
    /// Maximum capacity
    pub max_capacity: usize,
    /// Current active connections
    pub active_connections: usize,
    /// Average response time in milliseconds
    pub avg_response_time: f64,
    /// Geographic zone ID
    pub zone_id: Option<String>,
}

impl NodeLoad {
    /// Create new node load info
    pub fn new(coordinate: MatrixCoordinate, max_capacity: usize) -> Self {
        Self {
            coordinate,
            current_load: 0.0,
            max_capacity,
            active_connections: 0,
            avg_response_time: 0.0,
            zone_id: None,
        }
    }

    /// Check if node is available for new connections
    pub fn is_available(&self) -> bool {
        self.active_connections < self.max_capacity && self.current_load < 0.95
    }

    /// Get available capacity
    pub fn available_capacity(&self) -> usize {
        self.max_capacity.saturating_sub(self.active_connections)
    }

    /// Update load based on active connections
    pub fn update_load(&mut self) {
        if self.max_capacity > 0 {
            self.current_load = self.active_connections as f64 / self.max_capacity as f64;
        } else {
            self.current_load = 1.0;
        }
    }

    /// Add a connection
    pub fn add_connection(&mut self) {
        self.active_connections += 1;
        self.update_load();
    }

    /// Remove a connection
    pub fn remove_connection(&mut self) {
        if self.active_connections > 0 {
            self.active_connections -= 1;
            self.update_load();
        }
    }
}

/// Zone load statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ZoneLoadStats {
    /// Zone ID
    pub zone_id: String,
    /// Total nodes in zone
    pub total_nodes: usize,
    /// Available nodes
    pub available_nodes: usize,
    /// Average load across zone
    pub avg_load: f64,
    /// Total capacity
    pub total_capacity: usize,
    /// Total active connections
    pub total_connections: usize,
}

impl ZoneLoadStats {
    /// Calculate load percentage
    pub fn load_percentage(&self) -> f64 {
        if self.total_capacity > 0 {
            self.total_connections as f64 / self.total_capacity as f64
        } else {
            1.0
        }
    }
}

/// Load balancing statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoadBalancingStats {
    /// Total requests distributed
    pub total_requests: usize,
    /// Successful distributions
    pub successful_distributions: usize,
    /// Failed distributions (no available nodes)
    pub failed_distributions: usize,
    /// Average distribution time in microseconds
    pub avg_distribution_time: f64,
    /// Load variance across nodes
    pub load_variance: f64,
    /// Zone statistics
    pub zone_stats: HashMap<String, ZoneLoadStats>,
}

/// Geographic load balancer
pub struct GeographicLoadBalancer {
    /// Node load information
    nodes: HashMap<MatrixCoordinate, NodeLoad>,
    /// Round-robin index
    round_robin_index: usize,
    /// Load balancing statistics
    stats: LoadBalancingStats,
}

impl GeographicLoadBalancer {
    /// Create a new load balancer
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            round_robin_index: 0,
            stats: LoadBalancingStats::default(),
        }
    }

    /// Register a node
    pub fn register_node(&mut self, node: NodeLoad) {
        self.nodes.insert(node.coordinate, node);
    }

    /// Unregister a node
    pub fn unregister_node(&mut self, coordinate: &MatrixCoordinate) -> Option<NodeLoad> {
        self.nodes.remove(coordinate)
    }

    /// Update node load
    pub fn update_node_load(&mut self, coordinate: &MatrixCoordinate, load: f64) {
        if let Some(node) = self.nodes.get_mut(coordinate) {
            node.current_load = load.clamp(0.0, 1.0);
        }
    }

    /// Distribute load using specified strategy
    pub fn distribute(
        &mut self,
        source: &MatrixCoordinate,
        strategy: LoadBalancingStrategy,
    ) -> Option<MatrixCoordinate> {
        self.stats.total_requests += 1;

        let target = match strategy {
            LoadBalancingStrategy::RoundRobin => self.round_robin_distribution(),
            LoadBalancingStrategy::NearestNeighbor => self.nearest_neighbor_distribution(source),
            LoadBalancingStrategy::ZoneBased => self.zone_based_distribution(source),
            LoadBalancingStrategy::LatencyAware => self.latency_aware_distribution(source),
            LoadBalancingStrategy::WeightedCapacity => self.weighted_capacity_distribution(),
        };

        if let Some(coord) = target {
            self.stats.successful_distributions += 1;
            if let Some(node) = self.nodes.get_mut(&coord) {
                node.add_connection();
            }
        } else {
            self.stats.failed_distributions += 1;
        }

        target
    }

    /// Round-robin distribution
    fn round_robin_distribution(&mut self) -> Option<MatrixCoordinate> {
        let available: Vec<_> = self.nodes.values()
            .filter(|n| n.is_available())
            .map(|n| n.coordinate)
            .collect();

        if available.is_empty() {
            return None;
        }

        let selected = available[self.round_robin_index % available.len()];
        self.round_robin_index = (self.round_robin_index + 1) % available.len();

        Some(selected)
    }

    /// Nearest neighbor distribution
    fn nearest_neighbor_distribution(&mut self, source: &MatrixCoordinate) -> Option<MatrixCoordinate> {
        // Simply find the nearest available node
        self.nodes.values()
            .filter(|n| n.is_available())
            .min_by(|a, b| {
                let dist_a = source.euclidean_distance(&a.coordinate);
                let dist_b = source.euclidean_distance(&b.coordinate);
                dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|n| n.coordinate)
    }

    /// Zone-based distribution
    fn zone_based_distribution(&mut self, source: &MatrixCoordinate) -> Option<MatrixCoordinate> {
        // First, try to find source node's zone
        let source_zone = self.nodes.get(source)
            .and_then(|n| n.zone_id.as_ref());

        if let Some(zone_id) = source_zone {
            // Find available nodes in same zone
            let zone_nodes: Vec<_> = self.nodes.values()
                .filter(|n| n.zone_id.as_ref() == Some(zone_id) && n.is_available())
                .collect();

            if !zone_nodes.is_empty() {
                // Select least loaded node in zone
                let selected = zone_nodes.iter()
                    .min_by(|a, b| a.current_load.partial_cmp(&b.current_load).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|n| n.coordinate);
                return selected;
            }
        }

        // Fallback to nearest neighbor if zone-based fails
        self.nearest_neighbor_distribution(source)
    }

    /// Latency-aware distribution
    fn latency_aware_distribution(&mut self, source: &MatrixCoordinate) -> Option<MatrixCoordinate> {
        // Combine distance and response time
        self.nodes.values()
            .filter(|n| n.is_available())
            .min_by(|a, b| {
                // Score = distance * (1 + response_time/1000)
                let dist_a = source.euclidean_distance(&a.coordinate);
                let score_a = dist_a * (1.0 + a.avg_response_time / 1000.0);

                let dist_b = source.euclidean_distance(&b.coordinate);
                let score_b = dist_b * (1.0 + b.avg_response_time / 1000.0);

                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|n| n.coordinate)
    }

    /// Weighted capacity distribution
    fn weighted_capacity_distribution(&mut self) -> Option<MatrixCoordinate> {
        let available: Vec<_> = self.nodes.values()
            .filter(|n| n.is_available())
            .collect();

        if available.is_empty() {
            return None;
        }

        // Calculate total available capacity
        let total_capacity: usize = available.iter()
            .map(|n| n.available_capacity())
            .sum();

        if total_capacity == 0 {
            return None;
        }

        // Weighted random selection
        let mut cumulative_weight = 0;
        let target_weight = (total_capacity as f64 * 0.5) as usize; // Simple selection

        for node in &available {
            cumulative_weight += node.available_capacity();
            if cumulative_weight >= target_weight {
                return Some(node.coordinate);
            }
        }

        // Fallback to first available
        available.first().map(|n| n.coordinate)
    }

    /// Calculate load statistics per zone
    pub fn calculate_zone_stats(&mut self) -> HashMap<String, ZoneLoadStats> {
        let mut zone_stats: HashMap<String, ZoneLoadStats> = HashMap::new();

        for node in self.nodes.values() {
            if let Some(zone_id) = &node.zone_id {
                let stats = zone_stats.entry(zone_id.clone())
                    .or_insert_with(|| {
                        let mut s = ZoneLoadStats::default();
                        s.zone_id = zone_id.clone();
                        s
                    });

                stats.total_nodes += 1;
                if node.is_available() {
                    stats.available_nodes += 1;
                }
                stats.avg_load += node.current_load;
                stats.total_capacity += node.max_capacity;
                stats.total_connections += node.active_connections;
            }
        }

        // Calculate averages
        for stats in zone_stats.values_mut() {
            if stats.total_nodes > 0 {
                stats.avg_load /= stats.total_nodes as f64;
            }
        }

        self.stats.zone_stats = zone_stats.clone();
        zone_stats
    }

    /// Calculate load variance across all nodes
    pub fn calculate_load_variance(&mut self) -> f64 {
        if self.nodes.is_empty() {
            return 0.0;
        }

        let loads: Vec<f64> = self.nodes.values()
            .map(|n| n.current_load)
            .collect();

        let mean = loads.iter().sum::<f64>() / loads.len() as f64;
        let variance = loads.iter()
            .map(|load| (load - mean).powi(2))
            .sum::<f64>() / loads.len() as f64;

        self.stats.load_variance = variance;
        variance
    }

    /// Get current statistics
    pub fn get_stats(&self) -> &LoadBalancingStats {
        &self.stats
    }

    /// Get all node loads
    pub fn get_node_loads(&self) -> Vec<&NodeLoad> {
        self.nodes.values().collect()
    }

    /// Balance load across zones
    pub fn balance_zones(&mut self, threshold: f64) -> Vec<(MatrixCoordinate, MatrixCoordinate)> {
        let mut migrations = Vec::new();
        let zone_stats = self.calculate_zone_stats();

        // Find overloaded and underloaded zones
        let overloaded: Vec<_> = zone_stats.values()
            .filter(|s| s.load_percentage() > threshold)
            .collect();

        let underloaded: Vec<_> = zone_stats.values()
            .filter(|s| s.load_percentage() < threshold * 0.5)
            .collect();

        // Migrate connections from overloaded to underloaded zones
        for over_zone in &overloaded {
            for under_zone in &underloaded {
                // Find candidate nodes for migration
                let source_nodes: Vec<_> = self.nodes.values()
                    .filter(|n| n.zone_id.as_ref() == Some(&over_zone.zone_id))
                    .filter(|n| n.current_load > threshold)
                    .collect();

                let target_nodes: Vec<_> = self.nodes.values()
                    .filter(|n| n.zone_id.as_ref() == Some(&under_zone.zone_id))
                    .filter(|n| n.current_load < threshold * 0.5)
                    .collect();

                // Create migration pairs
                for (source, target) in source_nodes.iter().zip(target_nodes.iter()) {
                    migrations.push((source.coordinate, target.coordinate));
                }
            }
        }

        migrations
    }

    /// Clear all statistics
    pub fn clear_stats(&mut self) {
        self.stats = LoadBalancingStats::default();
    }
}

impl Default for GeographicLoadBalancer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_nodes() -> Vec<NodeLoad> {
        vec![
            NodeLoad::new(MatrixCoordinate::new(0, 0, 0).unwrap(), 100),
            NodeLoad::new(MatrixCoordinate::new(10, 0, 0).unwrap(), 100),
            NodeLoad::new(MatrixCoordinate::new(0, 10, 0).unwrap(), 100),
            NodeLoad::new(MatrixCoordinate::new(100, 100, 0).unwrap(), 50),
        ]
    }

    #[test]
    fn test_node_load_management() {
        let mut node = NodeLoad::new(MatrixCoordinate::origin(), 10);

        assert!(node.is_available());
        assert_eq!(node.available_capacity(), 10);

        // Add connections
        for _ in 0..5 {
            node.add_connection();
        }
        assert_eq!(node.active_connections, 5);
        assert_eq!(node.current_load, 0.5);
        assert!(node.is_available());

        // Add more connections
        for _ in 0..4 {
            node.add_connection();
        }
        assert_eq!(node.active_connections, 9);
        assert!(node.is_available());

        // Near capacity
        node.add_connection();
        assert_eq!(node.active_connections, 10);
        assert!(!node.is_available()); // Full

        // Remove connections
        node.remove_connection();
        assert!(node.is_available());
    }

    #[test]
    fn test_round_robin_distribution() {
        let mut balancer = GeographicLoadBalancer::new();
        let nodes = create_test_nodes();

        for node in nodes {
            balancer.register_node(node);
        }

        // Round-robin should cycle through nodes
        let mut targets = Vec::new();
        for _ in 0..8 {
            let target = balancer.distribute(
                &MatrixCoordinate::origin(),
                LoadBalancingStrategy::RoundRobin,
            );
            assert!(target.is_some());
            targets.push(target.unwrap());
        }

        // Should have visited all nodes
        let unique: std::collections::HashSet<_> = targets.iter().collect();
        assert_eq!(unique.len(), 4);
    }

    #[test]
    fn test_nearest_neighbor_distribution() {
        let mut balancer = GeographicLoadBalancer::new();

        // Register nodes at different distances
        balancer.register_node(NodeLoad::new(
            MatrixCoordinate::new(5, 0, 0).unwrap(), 10
        ));
        balancer.register_node(NodeLoad::new(
            MatrixCoordinate::new(50, 0, 0).unwrap(), 10
        ));
        balancer.register_node(NodeLoad::new(
            MatrixCoordinate::new(100, 0, 0).unwrap(), 10
        ));

        // Should select nearest node
        let source = MatrixCoordinate::origin();
        let target = balancer.distribute(
            &source,
            LoadBalancingStrategy::NearestNeighbor,
        );

        assert_eq!(target, Some(MatrixCoordinate::new(5, 0, 0).unwrap()));
    }

    #[test]
    fn test_zone_based_distribution() {
        let mut balancer = GeographicLoadBalancer::new();

        // Create nodes in different zones
        let mut node1 = NodeLoad::new(MatrixCoordinate::new(0, 0, 0).unwrap(), 10);
        node1.zone_id = Some("zone_a".to_string());

        let mut node2 = NodeLoad::new(MatrixCoordinate::new(10, 0, 0).unwrap(), 10);
        node2.zone_id = Some("zone_a".to_string());

        let mut node3 = NodeLoad::new(MatrixCoordinate::new(100, 100, 0).unwrap(), 10);
        node3.zone_id = Some("zone_b".to_string());

        balancer.register_node(node1);
        balancer.register_node(node2);
        balancer.register_node(node3);

        // Source in zone_a should prefer zone_a nodes
        let mut source_node = NodeLoad::new(MatrixCoordinate::new(5, 5, 0).unwrap(), 1);
        source_node.zone_id = Some("zone_a".to_string());
        balancer.register_node(source_node);

        let source = MatrixCoordinate::new(5, 5, 0).unwrap();
        let target = balancer.distribute(&source, LoadBalancingStrategy::ZoneBased);

        // Should select from zone_a (not zone_b)
        assert!(target.is_some());
        let coord = target.unwrap();
        assert!(coord.x < 50); // Zone A nodes are at x < 50
    }

    #[test]
    fn test_latency_aware_distribution() {
        let mut balancer = GeographicLoadBalancer::new();

        // Near node with high latency
        let mut node1 = NodeLoad::new(MatrixCoordinate::new(10, 0, 0).unwrap(), 10);
        node1.avg_response_time = 500.0; // High latency

        // Far node with low latency
        let mut node2 = NodeLoad::new(MatrixCoordinate::new(50, 0, 0).unwrap(), 10);
        node2.avg_response_time = 10.0; // Low latency

        balancer.register_node(node1);
        balancer.register_node(node2);

        // Might choose far node if latency difference is significant
        let source = MatrixCoordinate::origin();
        let target = balancer.distribute(&source, LoadBalancingStrategy::LatencyAware);

        assert!(target.is_some());
        // With these values, low-latency node might be preferred despite distance
    }

    #[test]
    fn test_weighted_capacity_distribution() {
        let mut balancer = GeographicLoadBalancer::new();

        // High capacity node
        let node1 = NodeLoad::new(MatrixCoordinate::new(0, 0, 0).unwrap(), 100);

        // Low capacity node
        let node2 = NodeLoad::new(MatrixCoordinate::new(10, 0, 0).unwrap(), 10);

        balancer.register_node(node1);
        balancer.register_node(node2);

        // Should prefer high-capacity node
        let mut high_capacity_selections = 0;
        for _ in 0..10 {
            let target = balancer.distribute(
                &MatrixCoordinate::origin(),
                LoadBalancingStrategy::WeightedCapacity,
            );
            if target == Some(MatrixCoordinate::origin()) {
                high_capacity_selections += 1;
            }
        }

        // High capacity node should be selected more often
        assert!(high_capacity_selections > 2);
    }

    #[test]
    fn test_zone_statistics() {
        let mut balancer = GeographicLoadBalancer::new();

        // Create nodes in zones
        for i in 0..3 {
            let mut node = NodeLoad::new(
                MatrixCoordinate::new(i * 10, 0, 0).unwrap(),
                100,
            );
            node.zone_id = Some("zone_a".to_string());
            node.active_connections = i as usize * 10;
            node.update_load();
            balancer.register_node(node);
        }

        let stats = balancer.calculate_zone_stats();
        assert!(stats.contains_key("zone_a"));

        let zone_a = &stats["zone_a"];
        assert_eq!(zone_a.total_nodes, 3);
        assert_eq!(zone_a.total_capacity, 300);
    }

    #[test]
    fn test_load_variance() {
        let mut balancer = GeographicLoadBalancer::new();

        // Create nodes with different loads
        for i in 0..4 {
            let mut node = NodeLoad::new(
                MatrixCoordinate::new(i * 10, 0, 0).unwrap(),
                100,
            );
            node.current_load = (i as f64) * 0.25;
            balancer.register_node(node);
        }

        let variance = balancer.calculate_load_variance();
        assert!(variance > 0.0);
    }

    #[test]
    fn test_zone_balancing() {
        let mut balancer = GeographicLoadBalancer::new();

        // Overloaded zone
        let mut node1 = NodeLoad::new(MatrixCoordinate::new(0, 0, 0).unwrap(), 100);
        node1.zone_id = Some("overloaded".to_string());
        node1.active_connections = 90;
        node1.update_load();

        // Underloaded zone
        let mut node2 = NodeLoad::new(MatrixCoordinate::new(100, 0, 0).unwrap(), 100);
        node2.zone_id = Some("underloaded".to_string());
        node2.active_connections = 10;
        node2.update_load();

        balancer.register_node(node1);
        balancer.register_node(node2);

        let migrations = balancer.balance_zones(0.8);
        assert!(!migrations.is_empty());
    }

    #[test]
    fn test_no_available_nodes() {
        let mut balancer = GeographicLoadBalancer::new();

        // Register a full node
        let mut node = NodeLoad::new(MatrixCoordinate::origin(), 10);
        node.active_connections = 10; // Full
        node.update_load();
        balancer.register_node(node);

        // All strategies should return None
        let strategies = vec![
            LoadBalancingStrategy::RoundRobin,
            LoadBalancingStrategy::NearestNeighbor,
            LoadBalancingStrategy::ZoneBased,
            LoadBalancingStrategy::LatencyAware,
            LoadBalancingStrategy::WeightedCapacity,
        ];

        for strategy in strategies {
            let result = balancer.distribute(&MatrixCoordinate::origin(), strategy);
            assert!(result.is_none());
        }

        // Stats should show failed distributions
        assert_eq!(balancer.get_stats().failed_distributions, 5);
    }
}