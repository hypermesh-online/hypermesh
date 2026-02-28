// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Geospatial integration for Block-MATRIX
//!
//! This module provides geographic awareness to the Block-MATRIX system by:
//! - Converting between GPS coordinates and matrix coordinates
//! - Organizing nodes into hierarchical geographic zones
//! - Clustering nodes based on geographic proximity
//! - Balancing load with geographic awareness
//! - Managing network topology with real-world locations
//!
//! # Architecture
//!
//! The geospatial system consists of several components:
//!
//! ## GPS Conversion (`converter`)
//! Translates between real-world GPS coordinates (latitude/longitude) and
//! Block-MATRIX coordinates (x,y,z). Supports multiple scale resolutions
//! from fine (100m) to regional (100km) precision.
//!
//! ## Geographic Hierarchy (`hierarchy`)
//! Organizes the world into hierarchical zones from Global to Local levels.
//! Pre-populated with major continents, countries, and cities for immediate use.
//!
//! ## Clustering (`clustering`)
//! Groups nodes using K-means, DBSCAN, or hierarchical algorithms.
//! Provides metrics for cluster quality and supports dynamic updates.
//!
//! ## Load Balancing (`load_balancing`)
//! Distributes requests considering geographic proximity, zone boundaries,
//! and network latency. Supports multiple strategies from simple round-robin
//! to complex latency-aware distribution.
//!
//! ## Network Topology (`topology`)
//! Manages the complete network structure with GPS-aware node positioning,
//! connection tracking, and integration with the blockchain layer.
//!
//! # Examples
//!
//! ## Basic GPS to Matrix Conversion
//! ```no_run
//! use blockmatrix::matrix::geospatial::{GpsConverter, GpsCoordinate, ScaleResolution};
//!
//! let converter = GpsConverter::new(ScaleResolution::Standard); // 1 unit = 1km
//! let nyc = GpsCoordinate::at_sea_level(40.7128, -74.0060).unwrap();
//! let matrix_coord = converter.gps_to_matrix(&nyc).unwrap();
//! ```
//!
//! ## Geographic Clustering
//! ```ignore
//! // Requires pre-existing `nodes: &[MatrixCoordinate]` slice.
//! use blockmatrix::matrix::geospatial::GeographicClustering;
//!
//! let mut clustering = GeographicClustering::new();
//! clustering.kmeans(&nodes, 5, 100); // 5 clusters, 100 iterations
//! let metrics = clustering.calculate_metrics();
//! ```
//!
//! ## Load Balancing with Geography
//! ```ignore
//! // Requires pre-existing `source: &MatrixCoordinate`.
//! use blockmatrix::matrix::geospatial::{GeographicLoadBalancer, LoadBalancingStrategy};
//!
//! let mut balancer = GeographicLoadBalancer::new();
//! let target = balancer.distribute(&source, LoadBalancingStrategy::NearestNeighbor);
//! ```

pub mod converter;
pub mod hierarchy;
pub mod clustering;
pub mod load_balancing;
pub mod topology;

// Re-export main types
pub use converter::{
    GpsConverter, GpsCoordinate, GpsError, ScaleResolution
};

pub use hierarchy::{
    GeographicHierarchy, GeographicZone, GeographicLevel, GeographicBounds
};

pub use clustering::{
    GeographicClustering, Cluster, ClusteringAlgorithm, ClusterMetrics
};

pub use load_balancing::{
    GeographicLoadBalancer, LoadBalancingStrategy, NodeLoad,
    ZoneLoadStats, LoadBalancingStats
};

pub use topology::{
    NetworkTopology, TopologyNode, TopologyEdge, GeographicDensity,
    TopologyQueryResult, TopologyVisualization
};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::matrix::coordinate::MatrixCoordinate;
    use crate::blockchain::node_chain::NodeBlockchain;

    #[test]
    fn test_full_geospatial_pipeline() {
        // 1. GPS Conversion
        let converter = GpsConverter::new(ScaleResolution::Standard);

        // Major cities
        let cities = vec![
            ("NYC", 40.7128, -74.0060),
            ("LA", 34.0522, -118.2437),
            ("Chicago", 41.8781, -87.6298),
            ("Houston", 29.7604, -95.3698),
        ];

        let mut matrix_coords = Vec::new();
        for (name, lat, lon) in &cities {
            let gps = GpsCoordinate::at_sea_level(*lat, *lon).unwrap();
            let matrix = converter.gps_to_matrix(&gps).unwrap();
            matrix_coords.push((name.to_string(), matrix));
        }

        // 2. Geographic Hierarchy
        let hierarchy = GeographicHierarchy::with_defaults();

        // NYC should be in USA
        let nyc_gps = GpsCoordinate::at_sea_level(40.7128, -74.0060).unwrap();
        let zones = hierarchy.find_zones_containing(&nyc_gps);
        let zone_ids: Vec<&str> = zones.iter().map(|z| z.id.as_str()).collect();
        assert!(zone_ids.contains(&"usa"));
        assert!(zone_ids.contains(&"north_america"));

        // 3. Clustering
        let mut clustering = GeographicClustering::new();
        let coords: Vec<MatrixCoordinate> = matrix_coords.iter()
            .map(|(_, coord)| *coord)
            .collect();

        clustering.kmeans(&coords, 2, 50); // East vs West coast
        let clusters = clustering.get_clusters();
        assert_eq!(clusters.len(), 2);

        // 4. Load Balancing
        let mut balancer = GeographicLoadBalancer::new();

        for (name, coord) in &matrix_coords {
            let mut node = NodeLoad::new(*coord, 100);
            if name == "NYC" || name == "Chicago" {
                node.zone_id = Some("east".to_string());
            } else {
                node.zone_id = Some("west".to_string());
            }
            balancer.register_node(node);
        }

        // Request from NYC should prefer east coast
        let nyc_coord = matrix_coords[0].1;
        let target = balancer.distribute(
            &nyc_coord,
            LoadBalancingStrategy::NearestNeighbor
        );
        assert!(target.is_some());

        // 5. Network Topology
        let mut topology = NetworkTopology::new(converter);

        for (name, coord) in &matrix_coords {
            let mut node = TopologyNode::new(name.clone(), *coord);
            let gps = match name.as_str() {
                "NYC" => GpsCoordinate::at_sea_level(40.7128, -74.0060).unwrap(),
                "LA" => GpsCoordinate::at_sea_level(34.0522, -118.2437).unwrap(),
                "Chicago" => GpsCoordinate::at_sea_level(41.8781, -87.6298).unwrap(),
                "Houston" => GpsCoordinate::at_sea_level(29.7604, -95.3698).unwrap(),
                _ => GpsCoordinate::at_sea_level(0.0, 0.0).unwrap(),
            };
            node.set_gps(gps);
            topology.add_node(node).unwrap();
        }

        // Add some connections
        topology.add_edge(TopologyEdge::new(
            "NYC".to_string(),
            "Chicago".to_string(),
            790.0, // ~790 miles
        )).unwrap();

        let stats = topology.get_statistics();
        assert_eq!(stats[&"total_nodes"], 4);
    }

    #[test]
    fn test_gps_matrix_round_trip() {
        let converter = GpsConverter::new(ScaleResolution::Fine);

        // Test various locations
        let locations = vec![
            ("Equator", 0.0, 0.0),
            ("North Pole", 90.0, 0.0),
            ("South Pole", -90.0, 0.0),
            ("Date Line", 0.0, 180.0),
            ("Tokyo", 35.6762, 139.6503),
            ("Sydney", -33.8688, 151.2093),
        ];

        for (name, lat, lon) in locations {
            let original = GpsCoordinate::at_sea_level(lat, lon).unwrap();
            let matrix = converter.gps_to_matrix(&original).unwrap();
            let recovered = converter.matrix_to_gps(&matrix).unwrap();

            // Should be accurate within 0.1 degrees with Fine resolution
            assert!(
                (recovered.latitude - original.latitude).abs() < 0.1,
                "{}: Latitude mismatch", name
            );

            // Longitude needs special handling at poles
            if lat.abs() < 89.0 {
                assert!(
                    (recovered.longitude - original.longitude).abs() < 0.1,
                    "{}: Longitude mismatch", name
                );
            }
        }
    }

    #[test]
    fn test_zone_based_clustering() {
        let mut clustering = GeographicClustering::new();

        // Create zones for US regions
        let zones = vec![
            GeographicZone::new(
                "northeast".to_string(),
                "Northeast".to_string(),
                GeographicLevel::Region,
                GeographicBounds::new(40.0, 45.0, -80.0, -70.0).unwrap(),
            ),
            GeographicZone::new(
                "southeast".to_string(),
                "Southeast".to_string(),
                GeographicLevel::Region,
                GeographicBounds::new(25.0, 35.0, -90.0, -75.0).unwrap(),
            ),
            GeographicZone::new(
                "west".to_string(),
                "West".to_string(),
                GeographicLevel::Region,
                GeographicBounds::new(32.0, 49.0, -125.0, -100.0).unwrap(),
            ),
        ];

        // Create nodes in different regions
        let nodes = vec![
            MatrixCoordinate::new(0, 100, 0).unwrap(),    // Northeast
            MatrixCoordinate::new(-50, -100, 0).unwrap(), // Southeast
            MatrixCoordinate::new(-200, 0, 0).unwrap(),   // West
        ];

        clustering.hierarchical(&nodes, &zones);

        let clusters = clustering.get_clusters();
        assert_eq!(clusters.len(), 3);

        // Each cluster should have zone metadata
        for cluster in clusters {
            assert!(cluster.zone_id.is_some());
            assert!(cluster.metadata.contains_key("zone_name"));
        }
    }

    #[test]
    fn test_load_balancing_strategies() {
        let mut balancer = GeographicLoadBalancer::new();

        // Create nodes with different characteristics
        let mut node1 = NodeLoad::new(MatrixCoordinate::new(0, 0, 0).unwrap(), 100);
        node1.avg_response_time = 10.0;
        node1.zone_id = Some("zone_a".to_string());

        let mut node2 = NodeLoad::new(MatrixCoordinate::new(50, 0, 0).unwrap(), 100);
        node2.avg_response_time = 100.0;
        node2.zone_id = Some("zone_b".to_string());

        let mut node3 = NodeLoad::new(MatrixCoordinate::new(0, 50, 0).unwrap(), 200);
        node3.avg_response_time = 50.0;
        node3.zone_id = Some("zone_a".to_string());

        balancer.register_node(node1);
        balancer.register_node(node2);
        balancer.register_node(node3);

        let source = MatrixCoordinate::new(5, 5, 0).unwrap();

        // Test different strategies
        let strategies = vec![
            LoadBalancingStrategy::RoundRobin,
            LoadBalancingStrategy::NearestNeighbor,
            LoadBalancingStrategy::LatencyAware,
            LoadBalancingStrategy::WeightedCapacity,
        ];

        for strategy in strategies {
            let target = balancer.distribute(&source, strategy);
            assert!(target.is_some(), "Strategy {:?} failed", strategy);
        }

        let stats = balancer.get_stats();
        assert_eq!(stats.successful_distributions, 4);
        assert_eq!(stats.failed_distributions, 0);
    }

    #[test]
    fn test_topology_blockchain_integration() {
        let mut topology = NetworkTopology::default();

        // Create a node with blockchain
        let coord = MatrixCoordinate::new(100, 200, 0).unwrap();
        let node = TopologyNode::new("blockchain_node".to_string(), coord);
        topology.add_node(node).unwrap();

        // Create a blockchain
        let _blockchain = NodeBlockchain::new(coord);

        // Integrate blockchain with topology
        topology.integrate_blockchain_node("blockchain_node", &coord).unwrap();

        let topo_node = topology.get_node("blockchain_node").unwrap();
        assert_eq!(topo_node.blockchain_id, Some("blockchain_node".to_string()));
        assert!(topo_node.metadata.contains_key("blockchain_coordinate"));
    }

    #[test]
    fn test_geographic_density_calculation() {
        let converter = GpsConverter::new(ScaleResolution::Standard);
        let mut topology = NetworkTopology::new(converter.clone());

        // Add nodes in a specific zone
        let zone = GeographicZone::new(
            "test_zone".to_string(),
            "Test Zone".to_string(),
            GeographicLevel::City,
            GeographicBounds::new(40.0, 41.0, -74.5, -73.5).unwrap(), // ~111km x 85km
        );

        // Add nodes to this zone
        for i in 0..10 {
            let gps = GpsCoordinate::at_sea_level(
                40.5 + (i as f64) * 0.05,
                -74.0
            ).unwrap();
            let matrix = converter.gps_to_matrix(&gps).unwrap();

            let mut node = TopologyNode::new(format!("node{}", i), matrix);
            node.zone_id = Some("test_zone".to_string());
            topology.add_node(node).unwrap();
        }

        let densities = topology.calculate_geographic_density(&[zone]);
        assert_eq!(densities.len(), 1);

        let density = &densities[0];
        assert_eq!(density.zone_id, "test_zone");
        assert_eq!(density.node_count, 10);
        assert!(density.density > 0.0);
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use crate::matrix::coordinate::MatrixCoordinate;
    use std::time::Instant;

    #[test]
    fn test_gps_conversion_performance() {
        let converter = GpsConverter::new(ScaleResolution::Standard);
        let gps = GpsCoordinate::at_sea_level(40.7128, -74.0060).unwrap();

        let start = Instant::now();
        for _ in 0..10000 {
            let _ = converter.gps_to_matrix(&gps);
        }
        let elapsed = start.elapsed();

        let per_conversion = elapsed.as_nanos() / 10000;
        assert!(
            per_conversion < 1000, // Should be < 1μs
            "GPS conversion took {}ns, expected < 1000ns", per_conversion
        );
    }

    #[test]
    fn test_clustering_performance() {
        let mut clustering = GeographicClustering::new();

        // Create 1000 nodes
        let mut nodes = Vec::new();
        for i in 0..1000 {
            let x = (i % 100) as i64 * 10;
            let y = (i / 100) as i64 * 10;
            nodes.push(MatrixCoordinate::new(x, y, 0).unwrap());
        }

        let start = Instant::now();
        clustering.kmeans(&nodes, 10, 50);
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() < 100,
            "Clustering 1000 nodes took {}ms, expected < 100ms", elapsed.as_millis()
        );
    }

    #[test]
    fn test_load_balancing_performance() {
        let mut balancer = GeographicLoadBalancer::new();

        // Register 100 nodes
        for i in 0..100 {
            let coord = MatrixCoordinate::new(i * 10, 0, 0).unwrap();
            let node = NodeLoad::new(coord, 100);
            balancer.register_node(node);
        }

        let source = MatrixCoordinate::origin();
        let start = Instant::now();

        // Perform 1000 distributions
        for _ in 0..1000 {
            let _ = balancer.distribute(&source, LoadBalancingStrategy::NearestNeighbor);
        }

        let elapsed = start.elapsed();
        let per_distribution = elapsed.as_micros() / 1000;

        assert!(
            per_distribution < 100,
            "Distribution took {}μs, expected < 100μs", per_distribution
        );
    }
}