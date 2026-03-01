// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Network topology management
//!
//! Provides data structures and algorithms for managing the
//! geographic network topology of Block-MATRIX nodes.

use crate::matrix::coordinate::MatrixCoordinate;
use crate::matrix::geospatial::converter::{GpsConverter, GpsCoordinate};
use crate::matrix::geospatial::hierarchy::GeographicZone;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A node in the network topology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyNode {
    /// Unique node identifier
    pub id: String,
    /// Matrix coordinate
    pub matrix_coord: MatrixCoordinate,
    /// GPS coordinate (if known)
    pub gps_coord: Option<GpsCoordinate>,
    /// Geographic zone ID
    pub zone_id: Option<String>,
    /// Node blockchain reference
    pub blockchain_id: Option<String>,
    /// Node metadata
    pub metadata: HashMap<String, String>,
    /// Connected peer IDs
    pub peers: HashSet<String>,
}

impl TopologyNode {
    /// Create a new topology node
    pub fn new(id: String, matrix_coord: MatrixCoordinate) -> Self {
        Self {
            id,
            matrix_coord,
            gps_coord: None,
            zone_id: None,
            blockchain_id: None,
            metadata: HashMap::new(),
            peers: HashSet::new(),
        }
    }

    /// Set GPS coordinate
    pub fn set_gps(&mut self, gps: GpsCoordinate) {
        self.gps_coord = Some(gps);
    }

    /// Add a peer connection
    pub fn add_peer(&mut self, peer_id: String) {
        self.peers.insert(peer_id);
    }

    /// Remove a peer connection
    pub fn remove_peer(&mut self, peer_id: &str) -> bool {
        self.peers.remove(peer_id)
    }

    /// Get peer count
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }
}

/// Network topology edge (connection between nodes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyEdge {
    /// Source node ID
    pub source: String,
    /// Target node ID
    pub target: String,
    /// Edge weight (e.g., latency, bandwidth)
    pub weight: f64,
    /// Edge metadata
    pub metadata: HashMap<String, String>,
}

impl TopologyEdge {
    /// Create a new topology edge
    pub fn new(source: String, target: String, weight: f64) -> Self {
        Self {
            source,
            target,
            weight,
            metadata: HashMap::new(),
        }
    }
}

/// Geographic density information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeographicDensity {
    /// Zone ID
    pub zone_id: String,
    /// Number of nodes
    pub node_count: usize,
    /// Area in square kilometers
    pub area_sq_km: f64,
    /// Density (nodes per square km)
    pub density: f64,
    /// Average connections per node
    pub avg_connections: f64,
}

/// Topology query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyQueryResult {
    /// Matching nodes
    pub nodes: Vec<TopologyNode>,
    /// Total count (for pagination)
    pub total_count: usize,
    /// Query execution time in milliseconds
    pub query_time_ms: f64,
}

/// Network topology manager
#[derive(Debug)]
pub struct NetworkTopology {
    /// All nodes indexed by ID
    nodes: HashMap<String, TopologyNode>,
    /// Edges between nodes
    edges: Vec<TopologyEdge>,
    /// Nodes indexed by matrix coordinate
    nodes_by_coord: HashMap<MatrixCoordinate, String>,
    /// Nodes indexed by zone
    nodes_by_zone: HashMap<String, HashSet<String>>,
    /// GPS converter
    gps_converter: GpsConverter,
}

impl NetworkTopology {
    /// Create a new network topology
    pub fn new(gps_converter: GpsConverter) -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            nodes_by_coord: HashMap::new(),
            nodes_by_zone: HashMap::new(),
            gps_converter,
        }
    }

    /// Add a node to the topology
    pub fn add_node(&mut self, node: TopologyNode) -> Result<(), String> {
        let node_id = node.id.clone();
        let coord = node.matrix_coord;
        let zone_id = node.zone_id.clone();

        // Check for duplicate ID
        if self.nodes.contains_key(&node_id) {
            return Err(format!("Node {node_id} already exists"));
        }

        // Check for duplicate coordinate
        if self.nodes_by_coord.contains_key(&coord) {
            return Err(format!("Coordinate {coord:?} already occupied"));
        }

        // Add to main index
        self.nodes.insert(node_id.clone(), node);

        // Add to coordinate index
        self.nodes_by_coord.insert(coord, node_id.clone());

        // Add to zone index
        if let Some(zone) = zone_id {
            self.nodes_by_zone.entry(zone).or_default().insert(node_id);
        }

        Ok(())
    }

    /// Remove a node from the topology
    pub fn remove_node(&mut self, node_id: &str) -> Option<TopologyNode> {
        if let Some(node) = self.nodes.remove(node_id) {
            // Remove from coordinate index
            self.nodes_by_coord.remove(&node.matrix_coord);

            // Remove from zone index
            if let Some(zone_id) = &node.zone_id {
                if let Some(zone_nodes) = self.nodes_by_zone.get_mut(zone_id) {
                    zone_nodes.remove(node_id);
                }
            }

            // Remove edges involving this node
            self.edges
                .retain(|e| e.source != node_id && e.target != node_id);

            // Remove from other nodes' peer lists
            for other_node in self.nodes.values_mut() {
                other_node.remove_peer(node_id);
            }

            Some(node)
        } else {
            None
        }
    }

    /// Add an edge between nodes
    pub fn add_edge(&mut self, edge: TopologyEdge) -> Result<(), String> {
        // Verify both nodes exist
        if !self.nodes.contains_key(&edge.source) {
            return Err(format!("Source node {} not found", edge.source));
        }
        if !self.nodes.contains_key(&edge.target) {
            return Err(format!("Target node {} not found", edge.target));
        }

        // Update peer lists
        if let Some(source_node) = self.nodes.get_mut(&edge.source) {
            source_node.add_peer(edge.target.clone());
        }
        if let Some(target_node) = self.nodes.get_mut(&edge.target) {
            target_node.add_peer(edge.source.clone());
        }

        self.edges.push(edge);
        Ok(())
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: &str) -> Option<&TopologyNode> {
        self.nodes.get(node_id)
    }

    /// Get a node by coordinate
    pub fn get_node_at_coord(&self, coord: &MatrixCoordinate) -> Option<&TopologyNode> {
        self.nodes_by_coord
            .get(coord)
            .and_then(|id| self.nodes.get(id))
    }

    /// Find nodes in a geographic zone
    pub fn find_nodes_in_zone(&self, zone_id: &str) -> Vec<&TopologyNode> {
        self.nodes_by_zone
            .get(zone_id)
            .map(|ids| ids.iter().filter_map(|id| self.nodes.get(id)).collect())
            .unwrap_or_default()
    }

    /// Find nodes within radius of a coordinate
    pub fn find_nodes_in_radius(
        &self,
        center: &MatrixCoordinate,
        radius: f64,
    ) -> Vec<&TopologyNode> {
        self.nodes
            .values()
            .filter(|node| center.euclidean_distance(&node.matrix_coord) <= radius)
            .collect()
    }

    /// Find nearest nodes to a GPS coordinate
    pub fn find_nearest_to_gps(&self, gps: &GpsCoordinate, count: usize) -> Vec<&TopologyNode> {
        // Convert GPS to matrix coordinate
        let matrix = match self.gps_converter.gps_to_matrix(gps) {
            Ok(coord) => coord,
            Err(_) => return Vec::new(),
        };

        // Find nearest by matrix distance
        let mut nodes_with_distance: Vec<_> = self
            .nodes
            .values()
            .map(|node| {
                let dist = matrix.euclidean_distance(&node.matrix_coord);
                (node, dist)
            })
            .collect();

        nodes_with_distance
            .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        nodes_with_distance.truncate(count);

        nodes_with_distance
            .into_iter()
            .map(|(node, _)| node)
            .collect()
    }

    /// Calculate geographic density for each zone
    pub fn calculate_geographic_density(&self, zones: &[GeographicZone]) -> Vec<GeographicDensity> {
        zones
            .iter()
            .filter_map(|zone| {
                let nodes = self.find_nodes_in_zone(&zone.id);
                if nodes.is_empty() {
                    return None;
                }

                // Calculate approximate area from bounds
                let bounds = &zone.bounds;
                let lat_range = bounds.max_latitude - bounds.min_latitude;
                let lon_range = bounds.max_longitude - bounds.min_longitude;

                // Rough approximation (more accurate would use proper geographic calculations)
                let avg_lat = (bounds.max_latitude + bounds.min_latitude) / 2.0;
                let lat_km = lat_range * 111.0; // 1 degree latitude ≈ 111km
                let lon_km = lon_range * 111.0 * (avg_lat * std::f64::consts::PI / 180.0).cos();
                let area_sq_km = lat_km * lon_km;

                let node_count = nodes.len();
                let density = node_count as f64 / area_sq_km;

                let total_connections: usize = nodes.iter().map(|n| n.peer_count()).sum();
                let avg_connections = total_connections as f64 / node_count as f64;

                Some(GeographicDensity {
                    zone_id: zone.id.clone(),
                    node_count,
                    area_sq_km,
                    density,
                    avg_connections,
                })
            })
            .collect()
    }

    /// Get topology statistics
    pub fn get_statistics(&self) -> HashMap<&str, usize> {
        let mut stats = HashMap::new();
        stats.insert("total_nodes", self.nodes.len());
        stats.insert("total_edges", self.edges.len());
        stats.insert("total_zones", self.nodes_by_zone.len());

        let connected_nodes = self.nodes.values().filter(|n| !n.peers.is_empty()).count();
        stats.insert("connected_nodes", connected_nodes);

        let isolated_nodes = self.nodes.len() - connected_nodes;
        stats.insert("isolated_nodes", isolated_nodes);

        stats
    }

    /// Export topology for visualization
    pub fn export_for_visualization(&self) -> TopologyVisualization {
        TopologyVisualization {
            nodes: self.nodes.values().cloned().collect(),
            edges: self.edges.clone(),
            statistics: self
                .get_statistics()
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        }
    }

    /// Find connected components
    pub fn find_connected_components(&self) -> Vec<HashSet<String>> {
        let mut visited = HashSet::new();
        let mut components = Vec::new();

        for node_id in self.nodes.keys() {
            if !visited.contains(node_id) {
                let mut component = HashSet::new();
                self.dfs_component(node_id, &mut visited, &mut component);
                components.push(component);
            }
        }

        components
    }

    /// DFS helper for finding connected components
    fn dfs_component(
        &self,
        node_id: &str,
        visited: &mut HashSet<String>,
        component: &mut HashSet<String>,
    ) {
        if visited.contains(node_id) {
            return;
        }

        visited.insert(node_id.to_string());
        component.insert(node_id.to_string());

        if let Some(node) = self.nodes.get(node_id) {
            for peer_id in &node.peers {
                self.dfs_component(peer_id, visited, component);
            }
        }
    }

    /// Calculate network diameter (longest shortest path)
    pub fn calculate_diameter(&self) -> Option<usize> {
        if self.nodes.is_empty() {
            return None;
        }

        let mut max_distance = 0;

        // Use BFS from each node to find longest shortest path
        for start_id in self.nodes.keys() {
            if let Some(distances) = self.bfs_distances(start_id) {
                if let Some(max) = distances.values().max() {
                    max_distance = max_distance.max(*max);
                }
            }
        }

        Some(max_distance)
    }

    /// BFS to calculate distances from a node
    fn bfs_distances(&self, start_id: &str) -> Option<HashMap<String, usize>> {
        let mut distances = HashMap::new();
        let mut queue = std::collections::VecDeque::new();

        distances.insert(start_id.to_string(), 0);
        queue.push_back(start_id.to_string());

        while let Some(current_id) = queue.pop_front() {
            if let Some(current_node) = self.nodes.get(&current_id) {
                // Safe: we only add to queue if distance is known
                let current_distance = match distances.get(&current_id) {
                    Some(&dist) => dist,
                    None => continue, // Skip if distance unknown
                };

                for peer_id in &current_node.peers {
                    if !distances.contains_key(peer_id) {
                        distances.insert(peer_id.clone(), current_distance + 1);
                        queue.push_back(peer_id.clone());
                    }
                }
            }
        }

        Some(distances)
    }

    /// Integrate with blockchain nodes
    pub fn integrate_blockchain_node(
        &mut self,
        node_id: &str,
        coordinate: &MatrixCoordinate,
    ) -> Result<(), String> {
        if let Some(node) = self.nodes.get_mut(node_id) {
            // Use node_id as blockchain_id and coordinate for reference
            node.blockchain_id = Some(node_id.to_string());
            node.metadata.insert(
                "blockchain_coordinate".to_string(),
                format!("({},{},{})", coordinate.x, coordinate.y, coordinate.z),
            );
            Ok(())
        } else {
            Err(format!("Node {node_id} not found"))
        }
    }
}

/// Topology visualization export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyVisualization {
    /// All nodes
    pub nodes: Vec<TopologyNode>,
    /// All edges
    pub edges: Vec<TopologyEdge>,
    /// Statistics
    pub statistics: HashMap<String, usize>,
}

impl Default for NetworkTopology {
    fn default() -> Self {
        Self::new(GpsConverter::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::geospatial::converter::ScaleResolution;

    #[test]
    fn test_topology_node() {
        let mut node = TopologyNode::new(
            "node1".to_string(),
            MatrixCoordinate::new(10, 20, 0).expect("test: valid coordinate"),
        );

        // Set GPS
        let gps = GpsCoordinate::at_sea_level(40.7, -74.0).expect("test: expected success");
        node.set_gps(gps);
        assert!(node.gps_coord.is_some());

        // Add peers
        node.add_peer("peer1".to_string());
        node.add_peer("peer2".to_string());
        assert_eq!(node.peer_count(), 2);

        // Remove peer
        assert!(node.remove_peer("peer1"));
        assert_eq!(node.peer_count(), 1);
    }

    #[test]
    fn test_topology_management() {
        let converter = GpsConverter::new(ScaleResolution::Standard);
        let mut topology = NetworkTopology::new(converter);

        // Add nodes
        let node1 = TopologyNode::new("node1".to_string(), MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate"));
        let node2 = TopologyNode::new(
            "node2".to_string(),
            MatrixCoordinate::new(10, 0, 0).expect("test: valid coordinate"),
        );

        topology.add_node(node1).expect("test: insertion");
        topology.add_node(node2).expect("test: insertion");

        // Add edge
        let edge = TopologyEdge::new("node1".to_string(), "node2".to_string(), 1.0);
        topology.add_edge(edge).expect("test: insertion");

        // Verify connectivity
        let n1 = topology.get_node("node1").expect("test: expected success");
        assert!(n1.peers.contains("node2"));

        let n2 = topology.get_node("node2").expect("test: expected success");
        assert!(n2.peers.contains("node1"));
    }

    #[test]
    fn test_duplicate_prevention() {
        let mut topology = NetworkTopology::default();

        let node1 = TopologyNode::new("node1".to_string(), MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate"));
        topology.add_node(node1).expect("test: insertion");

        // Duplicate ID
        let node2 = TopologyNode::new(
            "node1".to_string(),
            MatrixCoordinate::new(10, 0, 0).expect("test: valid coordinate"),
        );
        assert!(topology.add_node(node2).is_err());

        // Duplicate coordinate
        let node3 = TopologyNode::new("node3".to_string(), MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate"));
        assert!(topology.add_node(node3).is_err());
    }

    #[test]
    fn test_zone_indexing() {
        let mut topology = NetworkTopology::default();

        // Add nodes to zones
        for i in 0..3 {
            let mut node = TopologyNode::new(
                format!("node{i}"),
                MatrixCoordinate::new(i * 10, 0, 0).expect("test: valid coordinate"),
            );
            node.zone_id = Some("zone_a".to_string());
            topology.add_node(node).expect("test: insertion");
        }

        let zone_nodes = topology.find_nodes_in_zone("zone_a");
        assert_eq!(zone_nodes.len(), 3);
    }

    #[test]
    fn test_radius_search() {
        let mut topology = NetworkTopology::default();

        // Add nodes at various distances
        for i in 0..5 {
            let node = TopologyNode::new(
                format!("node{i}"),
                MatrixCoordinate::new(i * 10, 0, 0).expect("test: valid coordinate"),
            );
            topology.add_node(node).expect("test: insertion");
        }

        let center = MatrixCoordinate::origin();
        let nearby = topology.find_nodes_in_radius(&center, 25.0);

        // Should find nodes at 0, 10, 20 (within radius 25)
        assert_eq!(nearby.len(), 3);
    }

    #[test]
    fn test_gps_nearest_search() {
        let converter = GpsConverter::new(ScaleResolution::Standard);
        let mut topology = NetworkTopology::new(converter.clone());

        // Add nodes with GPS coordinates
        let gps_coords = [
            GpsCoordinate::at_sea_level(40.7, -74.0).expect("test: expected success"), // NYC
            GpsCoordinate::at_sea_level(34.0, -118.2).expect("test: expected success"), // LA
            GpsCoordinate::at_sea_level(41.9, -87.6).expect("test: expected success"), // Chicago
        ];

        for (i, gps) in gps_coords.iter().enumerate() {
            let matrix = converter.gps_to_matrix(gps).expect("test: expected success");
            let mut node = TopologyNode::new(format!("node{i}"), matrix);
            node.set_gps(*gps);
            topology.add_node(node).expect("test: insertion");
        }

        // Find nearest to Boston
        let boston = GpsCoordinate::at_sea_level(42.3, -71.0).expect("test: expected success");
        let nearest = topology.find_nearest_to_gps(&boston, 2);

        assert_eq!(nearest.len(), 2);
        // NYC should be closest
        assert_eq!(nearest[0].id, "node0");
    }

    #[test]
    fn test_connected_components() {
        let mut topology = NetworkTopology::default();

        // Create two separate components
        // Component 1: nodes 0-2
        for i in 0..3 {
            let node = TopologyNode::new(
                format!("node{i}"),
                MatrixCoordinate::new(i * 10, 0, 0).expect("test: valid coordinate"),
            );
            topology.add_node(node).expect("test: insertion");
        }
        topology
            .add_edge(TopologyEdge::new(
                "node0".to_string(),
                "node1".to_string(),
                1.0,
            ))
            .expect("test: expected success");
        topology
            .add_edge(TopologyEdge::new(
                "node1".to_string(),
                "node2".to_string(),
                1.0,
            ))
            .expect("test: expected success");

        // Component 2: nodes 3-4
        for i in 3..5 {
            let node = TopologyNode::new(
                format!("node{i}"),
                MatrixCoordinate::new(i * 10, 0, 0).expect("test: valid coordinate"),
            );
            topology.add_node(node).expect("test: insertion");
        }
        topology
            .add_edge(TopologyEdge::new(
                "node3".to_string(),
                "node4".to_string(),
                1.0,
            ))
            .expect("test: expected success");

        let mut components = topology.find_connected_components();
        assert_eq!(components.len(), 2);
        // Sort by size descending since HashMap iteration order is non-deterministic
        components.sort_by_key(|a| std::cmp::Reverse(a.len()));
        assert_eq!(components[0].len(), 3);
        assert_eq!(components[1].len(), 2);
    }

    #[test]
    fn test_network_diameter() {
        let mut topology = NetworkTopology::default();

        // Create a linear chain: 0-1-2-3
        for i in 0..4 {
            let node = TopologyNode::new(
                format!("node{i}"),
                MatrixCoordinate::new(i * 10, 0, 0).expect("test: valid coordinate"),
            );
            topology.add_node(node).expect("test: insertion");
        }

        for i in 0..3 {
            topology
                .add_edge(TopologyEdge::new(
                    format!("node{i}"),
                    format!("node{}", i + 1),
                    1.0,
                ))
                .expect("test: expected success");
        }

        let diameter = topology.calculate_diameter();
        assert_eq!(diameter, Some(3)); // Maximum distance is 3 (from node0 to node3)
    }

    #[test]
    fn test_node_removal() {
        let mut topology = NetworkTopology::default();

        // Add connected nodes
        for i in 0..3 {
            let node = TopologyNode::new(
                format!("node{i}"),
                MatrixCoordinate::new(i * 10, 0, 0).expect("test: valid coordinate"),
            );
            topology.add_node(node).expect("test: insertion");
        }

        topology
            .add_edge(TopologyEdge::new(
                "node0".to_string(),
                "node1".to_string(),
                1.0,
            ))
            .expect("test: expected success");
        topology
            .add_edge(TopologyEdge::new(
                "node1".to_string(),
                "node2".to_string(),
                1.0,
            ))
            .expect("test: expected success");

        // Remove middle node
        let removed = topology.remove_node("node1");
        assert!(removed.is_some());

        // Check that edges are removed
        assert_eq!(topology.edges.len(), 0);

        // Check that peer lists are updated
        let node0 = topology.get_node("node0").expect("test: expected success");
        assert!(!node0.peers.contains("node1"));
    }

    #[test]
    fn test_statistics() {
        let mut topology = NetworkTopology::default();

        // Add some nodes and edges
        for i in 0..5 {
            let node = TopologyNode::new(
                format!("node{i}"),
                MatrixCoordinate::new(i * 10, 0, 0).expect("test: valid coordinate"),
            );
            topology.add_node(node).expect("test: insertion");
        }

        // Connect first 3 nodes
        topology
            .add_edge(TopologyEdge::new(
                "node0".to_string(),
                "node1".to_string(),
                1.0,
            ))
            .expect("test: expected success");
        topology
            .add_edge(TopologyEdge::new(
                "node1".to_string(),
                "node2".to_string(),
                1.0,
            ))
            .expect("test: expected success");

        let stats = topology.get_statistics();
        assert_eq!(stats[&"total_nodes"], 5);
        assert_eq!(stats[&"total_edges"], 2);
        assert_eq!(stats[&"connected_nodes"], 3);
        assert_eq!(stats[&"isolated_nodes"], 2);
    }
}
