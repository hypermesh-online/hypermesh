// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Simple test of geospatial module

use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::matrix::geospatial::{
    GpsConverter, GpsCoordinate, ScaleResolution,
    GeographicHierarchy, GeographicClustering,
    GeographicLoadBalancer, LoadBalancingStrategy, NodeLoad,
    NetworkTopology, TopologyNode,
};

fn main() {
    println!("Testing Block-MATRIX Geospatial Integration...\n");

    // Test 1: GPS Conversion
    println!("Test 1: GPS to Matrix Conversion");
    let converter = GpsConverter::new(ScaleResolution::Standard);
    let nyc = GpsCoordinate::at_sea_level(40.7128, -74.0060).unwrap();
    let matrix = converter.gps_to_matrix(&nyc).unwrap();
    println!("  NYC GPS: ({}, {})", nyc.latitude, nyc.longitude);
    println!("  Matrix: ({}, {}, {})", matrix.x, matrix.y, matrix.z);

    // Round trip
    let recovered = converter.matrix_to_gps(&matrix).unwrap();
    println!("  Round-trip: ({}, {})\n", recovered.latitude, recovered.longitude);

    // Test 2: Geographic Hierarchy
    println!("Test 2: Geographic Hierarchy");
    let hierarchy = GeographicHierarchy::with_defaults();
    let zones = hierarchy.find_zones_containing(&nyc);
    println!("  NYC is in {} zones:", zones.len());
    for zone in zones {
        println!("    - {} ({})", zone.name, zone.id);
    }
    println!();

    // Test 3: Clustering
    println!("Test 3: Geographic Clustering");
    let mut clustering = GeographicClustering::new();
    let nodes = vec![
        MatrixCoordinate::new(0, 0, 0).unwrap(),
        MatrixCoordinate::new(10, 10, 0).unwrap(),
        MatrixCoordinate::new(100, 100, 0).unwrap(),
        MatrixCoordinate::new(110, 110, 0).unwrap(),
    ];
    clustering.kmeans(&nodes, 2, 50);
    let clusters = clustering.get_clusters();
    println!("  Created {} clusters from {} nodes", clusters.len(), nodes.len());
    for cluster in clusters {
        println!("    - Cluster {}: {} members", cluster.id, cluster.size());
    }
    let metrics = clustering.calculate_metrics();
    println!("  Metrics: cohesion={:.2}, separation={:.2}\n",
             metrics.avg_cohesion, metrics.avg_separation);

    // Test 4: Load Balancing
    println!("Test 4: Geographic Load Balancing");
    let mut balancer = GeographicLoadBalancer::new();
    for i in 0..3 {
        let coord = MatrixCoordinate::new(i * 50, 0, 0).unwrap();
        let node = NodeLoad::new(coord, 100);
        balancer.register_node(node);
        println!("  Registered node at ({}, {}, {})", coord.x, coord.y, coord.z);
    }

    let source = MatrixCoordinate::origin();
    let target = balancer.distribute(&source, LoadBalancingStrategy::NearestNeighbor);
    match target {
        Some(coord) => println!("  Load distributed to: ({}, {}, {})", coord.x, coord.y, coord.z),
        None => println!("  No available nodes for load distribution"),
    }
    println!();

    // Test 5: Network Topology
    println!("Test 5: Network Topology");
    let mut topology = NetworkTopology::default();

    let cities = vec![
        ("NYC", 40.7128, -74.0060),
        ("LA", 34.0522, -118.2437),
        ("Chicago", 41.8781, -87.6298),
    ];

    for (name, lat, lon) in cities {
        let gps = GpsCoordinate::at_sea_level(lat, lon).unwrap();
        let matrix = converter.gps_to_matrix(&gps).unwrap();
        let mut node = TopologyNode::new(name.to_string(), matrix);
        node.set_gps(gps);
        topology.add_node(node).unwrap();
        println!("  Added {} to topology", name);
    }

    let stats = topology.get_statistics();
    println!("  Topology stats: {} nodes", stats[&"total_nodes"]);

    println!("\n✅ All geospatial tests passed!");
}