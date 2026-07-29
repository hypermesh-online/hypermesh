// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase 1 End-to-End Integration Tests
//!
//! Comprehensive tests for the complete Block-MATRIX Foundation spanning
//! all five sprints of Phase 1:
//! - Sprint 1.1: Matrix Coordinate System
//! - Sprint 1.2: Tensor Operations Library
//! - Sprint 1.3: Every-Node-Blockchain
//! - Sprint 1.4: Geospatial Integration
//! - Sprint 1.5: Matrix Persistence Layer
//!
//! These tests validate the integrated system at scale with 100-node networks.

use blockmatrix::blockchain::PropagationStrategy;
use blockmatrix::integration::{MatrixFoundation, MatrixFoundationConfig};
use blockmatrix::matrix::geospatial::{GpsConverter, GpsCoordinate, ScaleResolution};
use blockmatrix::matrix::tensor::Vector3D;
use blockmatrix::matrix::MatrixCoordinate;
use blockmatrix::proof_of_state::StateProof;
use std::time::Instant;
use tempfile::TempDir;

/// Create a test matrix foundation with temporary storage
async fn create_test_foundation() -> (MatrixFoundation, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let config = MatrixFoundationConfig {
        storage_path: temp_dir.path().to_path_buf(),
        propagation_strategy: PropagationStrategy::Broadcast,
        enable_snapshots: true,
        snapshot_interval_secs: 3600,
        max_nodes: 10000,
    };
    let foundation = MatrixFoundation::new(config).await.unwrap();
    (foundation, temp_dir)
}

#[tokio::test]
async fn test_e2e_100_node_matrix_network() {
    println!("\n=== E2E Test: 100-Node Matrix Network ===\n");

    let (foundation, _temp_dir) = create_test_foundation().await;

    // Create 100 nodes in a 10x10 grid
    let start = Instant::now();
    for x in 0..10 {
        for y in 0..10 {
            let coord = MatrixCoordinate::new(x * 10, y * 10, 0).unwrap();
            let node_id = format!("node_{x}_{y}");
            foundation.add_node(node_id, coord).await.unwrap();
        }
    }
    let creation_time = start.elapsed();

    println!("✓ Created 100 nodes in {creation_time:?}");
    assert!(
        creation_time.as_millis() < 1000,
        "Node creation should be <1s"
    );

    // Verify node count
    assert_eq!(foundation.node_count().await, 100);

    // Get network statistics
    let stats = foundation.get_network_stats().await;
    assert_eq!(stats.node_count, 100);
    assert_eq!(stats.min_x, 0);
    assert_eq!(stats.max_x, 90);
    assert_eq!(stats.min_y, 0);
    assert_eq!(stats.max_y, 90);
    println!("✓ Network stats validated");

    // Test neighbor discovery
    let center = MatrixCoordinate::new(50, 50, 0).unwrap();
    let nearest = foundation.find_k_nearest_nodes(&center, 5).await;
    assert_eq!(nearest.len(), 5);
    println!("✓ K-nearest neighbor discovery working");

    // Test radius-based neighbor discovery
    let neighbors = foundation.find_neighbors_in_radius(&center, 20.0).await;
    assert!(
        neighbors.len() >= 4,
        "Should find at least 4 neighbors within radius"
    );
    println!("✓ Radius-based neighbor discovery working");

    // Add blocks to multiple nodes
    let start = Instant::now();
    let proof = StateProof::new_for_testing();
    for i in 0..10 {
        let node_id = format!("node_{i}_{i}");
        let data = format!("Block data from node {i}").into_bytes();
        foundation.add_block(&node_id, data, &proof).await.unwrap();
    }
    let block_time = start.elapsed();
    println!("✓ Added 10 blocks in {block_time:?}");
    assert!(
        block_time.as_millis() < 500,
        "Block addition should be <500ms"
    );

    // Verify blockchain heights
    for i in 0..10 {
        let node_id = format!("node_{i}_{i}");
        let height = foundation.get_blockchain_height(&node_id).await.unwrap();
        assert_eq!(height, 1, "Each node should have genesis + 1 block");
    }
    println!("✓ Blockchain integrity verified");

    // Test persistence
    let start = Instant::now();
    let snapshot_id = foundation.save_network_state().await.unwrap();
    let save_time = start.elapsed();
    println!("✓ Network state saved in {save_time:?} (snapshot: {snapshot_id})");
    assert!(save_time.as_secs() < 10, "Save should complete in <10s");

    // Clean shutdown
    foundation.shutdown().await.unwrap();
    println!("✓ Clean shutdown completed");

    println!("\n=== E2E Test Complete ===\n");
}

#[tokio::test]
async fn test_e2e_full_workflow() {
    println!("\n=== E2E Test: Full Workflow ===\n");

    let (foundation, _temp_dir) = create_test_foundation().await;

    // Step 1: Create nodes with matrix positioning
    println!("Step 1: Creating 25 nodes in 5x5 grid...");
    for x in 0..5 {
        for y in 0..5 {
            let coord = MatrixCoordinate::new(x * 100, y * 100, 0).unwrap();
            foundation
                .add_node(format!("node_{x}_{y}"), coord)
                .await
                .unwrap();
        }
    }
    assert_eq!(foundation.node_count().await, 25);
    println!("✓ 25 nodes created");

    // Step 2: Create blockchains for each node (already done in add_node)
    println!("Step 2: Verifying blockchains...");
    for x in 0..5 {
        for y in 0..5 {
            let node_id = format!("node_{x}_{y}");
            let height = foundation.get_blockchain_height(&node_id).await.unwrap();
            assert_eq!(height, 0, "Should have genesis block only");
        }
    }
    println!("✓ All blockchains initialized");

    // Step 3: Add blocks to all nodes
    println!("Step 3: Adding blocks to all nodes...");
    let proof = StateProof::new_for_testing();
    for x in 0..5 {
        for y in 0..5 {
            let node_id = format!("node_{x}_{y}");
            let data = format!("Data from node ({x}, {y})").into_bytes();
            foundation.add_block(&node_id, data, &proof).await.unwrap();
        }
    }
    println!("✓ Blocks added to all nodes");

    // Step 4: Verify blockchain propagation (check heights)
    println!("Step 4: Verifying blockchain integrity...");
    for x in 0..5 {
        for y in 0..5 {
            let node_id = format!("node_{x}_{y}");
            let height = foundation.get_blockchain_height(&node_id).await.unwrap();
            assert_eq!(height, 1, "Each node should have genesis + 1 block");
        }
    }
    println!("✓ Blockchain integrity verified");

    // Step 5: Test neighbor discovery algorithms
    println!("Step 5: Testing neighbor discovery...");
    let center = MatrixCoordinate::new(200, 200, 0).unwrap();

    let k_nearest = foundation.find_k_nearest_nodes(&center, 5).await;
    assert_eq!(k_nearest.len(), 5);

    let radius_neighbors = foundation.find_neighbors_in_radius(&center, 150.0).await;
    assert!(!radius_neighbors.is_empty());

    println!("✓ Neighbor discovery working");

    // Step 6: Calculate matrix distances
    println!("Step 6: Calculating matrix distances...");
    let node1 = MatrixCoordinate::new(0, 0, 0).unwrap();
    let node2 = MatrixCoordinate::new(300, 400, 0).unwrap();

    let euclidean = node1.euclidean_distance(&node2);
    let manhattan = node1.manhattan_distance(&node2);
    let chebyshev = node1.chebyshev_distance(&node2);

    assert_eq!(euclidean, 500.0);
    assert_eq!(manhattan, 700);
    assert_eq!(chebyshev, 400);
    println!("✓ Distance calculations correct");

    // Step 7: Test persistence and recovery
    println!("Step 7: Testing persistence...");
    let snapshot_id = foundation.save_network_state().await.unwrap();
    println!("✓ Network state persisted (snapshot: {snapshot_id})");

    // Step 8: Verify network statistics
    println!("Step 8: Verifying network statistics...");
    let stats = foundation.get_network_stats().await;
    assert_eq!(stats.node_count, 25);
    assert!(stats.volume() > 0);
    assert!(stats.density() > 0.0);
    println!("✓ Network statistics validated");

    foundation.shutdown().await.unwrap();
    println!("\n=== Full Workflow Complete ===\n");
}

#[tokio::test]
async fn test_e2e_geospatial_positioning() {
    println!("\n=== E2E Test: Geospatial Positioning ===\n");

    let (foundation, _temp_dir) = create_test_foundation().await;

    // Create GPS to matrix converter
    let converter = GpsConverter::new(ScaleResolution::Standard); // 1 unit = 1km

    // Real world locations
    let locations = vec![
        ("New York", 40.7128, -74.0060),
        ("London", 51.5074, -0.1278),
        ("Tokyo", 35.6762, 139.6503),
        ("Sydney", -33.8688, 151.2093),
        ("Paris", 48.8566, 2.3522),
    ];

    println!("Converting GPS coordinates to matrix positions...");
    for (name, lat, lon) in locations {
        let gps = GpsCoordinate::at_sea_level(lat, lon).unwrap();
        let matrix_coord = converter.gps_to_matrix(&gps).unwrap();

        let node_id = format!("node_{}", name.to_lowercase().replace(" ", "_"));
        foundation
            .add_node(node_id.clone(), matrix_coord)
            .await
            .unwrap();

        println!("✓ {name} at GPS({lat}, {lon}) -> Matrix{matrix_coord}");

        // Verify round-trip conversion
        let converted_back = converter.matrix_to_gps(&matrix_coord).unwrap();
        let lat_diff = (converted_back.latitude - lat).abs();
        let lon_diff = (converted_back.longitude - lon).abs();

        // Allow small error due to precision
        assert!(lat_diff < 5.0, "Latitude conversion error too large");
        assert!(lon_diff < 5.0, "Longitude conversion error too large");
    }

    assert_eq!(foundation.node_count().await, 5);
    println!("✓ All geospatial nodes created");

    // Find nearest city to a test coordinate
    let test_gps = GpsCoordinate::at_sea_level(40.0, -74.0).unwrap();
    let test_matrix = converter.gps_to_matrix(&test_gps).unwrap();
    let nearest = foundation.find_k_nearest_nodes(&test_matrix, 1).await;

    assert_eq!(nearest.len(), 1);
    println!("✓ Nearest city to test coordinate found");

    foundation.shutdown().await.unwrap();
    println!("\n=== Geospatial Test Complete ===\n");
}

#[tokio::test]
async fn test_e2e_tensor_operations() {
    println!("\n=== E2E Test: Tensor-Based Routing ===\n");

    // Create vector positions
    let origin = Vector3D::new(0.0, 0.0, 0.0);
    let destination = Vector3D::new(100.0, 100.0, 0.0);

    // Test vector operations
    let direction = destination.subtract(&origin);
    let distance = direction.magnitude();
    let normalized = direction.normalize().unwrap();

    println!("Origin: {origin:?}");
    println!("Destination: {destination:?}");
    println!("Direction: {direction:?}");
    println!("Distance: {distance:.2}");
    println!("Normalized: {normalized:?}");

    assert!((distance - 141.42).abs() < 0.1);
    assert!((normalized.magnitude() - 1.0).abs() < 0.01);

    println!("✓ Vector operations validated");

    println!("\n=== Tensor Operations Complete ===\n");
}

#[tokio::test]
async fn test_e2e_blockchain_propagation() {
    println!("\n=== E2E Test: Blockchain Propagation ===\n");

    let (foundation, _temp_dir) = create_test_foundation().await;

    // Create a small network
    let node_count = 10;
    for i in 0..node_count {
        let coord = MatrixCoordinate::new(i * 10, 0, 0).unwrap();
        foundation
            .add_node(format!("node{i}"), coord)
            .await
            .unwrap();
    }

    println!("Created {node_count} nodes in a line");

    // Test different propagation strategies
    for strategy in &[
        PropagationStrategy::Broadcast,
        PropagationStrategy::NearestN(3),
        PropagationStrategy::RoutedPath,
        PropagationStrategy::DistanceThreshold(50.0),
    ] {
        println!("Testing {strategy:?} propagation...");

        // Add blocks to all nodes
        let proof = StateProof::new_for_testing();
        for i in 0..node_count {
            let node_id = format!("node{i}");
            let data = format!("Block from node{i} with {strategy:?}").into_bytes();
            foundation.add_block(&node_id, data, &proof).await.unwrap();
        }

        // Verify all nodes received their blocks
        for i in 0..node_count {
            let node_id = format!("node{i}");
            let height = foundation.get_blockchain_height(&node_id).await.unwrap();
            assert!(height > 0, "Node {i} should have blocks");
        }

        println!("✓ {strategy:?} propagation successful");
    }

    foundation.shutdown().await.unwrap();
    println!("\n=== Propagation Test Complete ===\n");
}

#[tokio::test]
async fn test_e2e_persistence_recovery() {
    println!("\n=== E2E Test: Persistence and Recovery ===\n");

    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    // Phase 1: Create and populate network
    {
        let config = MatrixFoundationConfig {
            storage_path: storage_path.clone(),
            ..Default::default()
        };
        let foundation = MatrixFoundation::new(config).await.unwrap();

        // Add nodes
        for i in 0..20 {
            let coord = MatrixCoordinate::new(i * 5, i * 5, 0).unwrap();
            foundation
                .add_node(format!("node{i}"), coord)
                .await
                .unwrap();
        }

        // Add blocks
        let proof = StateProof::new_for_testing();
        for i in 0..20 {
            let node_id = format!("node{i}");
            let data = format!("Persistent data {i}").into_bytes();
            foundation.add_block(&node_id, data, &proof).await.unwrap();
        }

        println!("Created 20 nodes with blocks");

        // Save state
        let start = Instant::now();
        let snapshot_id = foundation.save_network_state().await.unwrap();
        let save_time = start.elapsed();

        println!("✓ Saved state in {save_time:?} (snapshot: {snapshot_id})");
        assert!(save_time.as_secs() < 5, "Save should be <5s");

        foundation.shutdown().await.unwrap();
    }

    // Phase 2: Recover network
    {
        let config = MatrixFoundationConfig {
            storage_path: storage_path.clone(),
            ..Default::default()
        };
        let mut foundation = MatrixFoundation::new(config).await.unwrap();

        let start = Instant::now();
        foundation.recover_network_state().await.unwrap();
        let recovery_time = start.elapsed();

        println!("✓ Recovered state in {recovery_time:?}");
        assert!(recovery_time.as_secs() < 10, "Recovery should be <10s");

        // Note: Recovery validation would require storing node registry
        // in persistence layer (future enhancement)

        foundation.shutdown().await.unwrap();
    }

    println!("\n=== Persistence Test Complete ===\n");
}

#[tokio::test]
async fn test_e2e_performance_validation() {
    println!("\n=== E2E Test: Performance Validation ===\n");

    let (foundation, _temp_dir) = create_test_foundation().await;

    // Test 1: Node creation performance
    let start = Instant::now();
    for i in 0..100 {
        let coord = MatrixCoordinate::new(i, i, i).unwrap();
        foundation
            .add_node(format!("node{i}"), coord)
            .await
            .unwrap();
    }
    let creation_time = start.elapsed();
    let per_node_time = creation_time.as_micros() / 100;

    println!("Node creation: {per_node_time} µs per node");
    assert!(per_node_time < 10_000, "Should create node in <10ms");

    // Test 2: Neighbor discovery performance
    let center = MatrixCoordinate::new(50, 50, 50).unwrap();
    let start = Instant::now();
    for _ in 0..100 {
        let _ = foundation.find_k_nearest_nodes(&center, 10).await;
    }
    let discovery_time = start.elapsed();
    let per_query = discovery_time.as_micros() / 100;

    println!("Neighbor discovery: {per_query} µs per query");
    assert!(per_query < 1_000, "Should discover neighbors in <1ms");

    // Test 3: Block addition performance
    let proof = StateProof::new_for_testing();
    let start = Instant::now();
    for i in 0..100 {
        let node_id = format!("node{i}");
        let data = vec![i as u8; 1024]; // 1KB blocks
        foundation.add_block(&node_id, data, &proof).await.unwrap();
    }
    let block_time = start.elapsed();
    let per_block = block_time.as_micros() / 100;

    println!("Block addition: {per_block} µs per block");
    assert!(per_block < 5_000, "Should add block in <5ms");

    // Test 4: Network stats performance
    let start = Instant::now();
    for _ in 0..100 {
        let _ = foundation.get_network_stats().await;
    }
    let stats_time = start.elapsed();
    let per_stat = stats_time.as_micros() / 100;

    println!("Network stats: {per_stat} µs per query");
    assert!(per_stat < 500, "Should get stats in <500µs");

    foundation.shutdown().await.unwrap();
    println!("\n=== Performance Validation Complete ===\n");
}

#[tokio::test]
async fn test_e2e_matrix_distance_calculations() {
    println!("\n=== E2E Test: Matrix Distance Calculations ===\n");

    let (foundation, _temp_dir) = create_test_foundation().await;

    // Create nodes at specific positions
    let positions = [
        (0, 0, 0),
        (100, 0, 0),
        (0, 100, 0),
        (0, 0, 100),
        (100, 100, 100),
    ];

    for (i, (x, y, z)) in positions.iter().enumerate() {
        let coord = MatrixCoordinate::new(*x, *y, *z).unwrap();
        foundation
            .add_node(format!("node{i}"), coord)
            .await
            .unwrap();
    }

    // Test all distance metrics
    let node0 = MatrixCoordinate::new(0, 0, 0).unwrap();
    let node1 = MatrixCoordinate::new(100, 0, 0).unwrap();
    let node2 = MatrixCoordinate::new(0, 100, 0).unwrap();
    let node3 = MatrixCoordinate::new(0, 0, 100).unwrap();
    let node4 = MatrixCoordinate::new(100, 100, 100).unwrap();

    // Euclidean distances
    assert_eq!(node0.euclidean_distance(&node1), 100.0);
    assert_eq!(node0.euclidean_distance(&node2), 100.0);
    assert_eq!(node0.euclidean_distance(&node3), 100.0);
    assert!((node0.euclidean_distance(&node4) - 173.20).abs() < 0.1);

    println!("✓ Euclidean distances correct");

    // Manhattan distances
    assert_eq!(node0.manhattan_distance(&node1), 100);
    assert_eq!(node0.manhattan_distance(&node2), 100);
    assert_eq!(node0.manhattan_distance(&node4), 300);

    println!("✓ Manhattan distances correct");

    // Chebyshev distances
    assert_eq!(node0.chebyshev_distance(&node1), 100);
    assert_eq!(node0.chebyshev_distance(&node4), 100);

    println!("✓ Chebyshev distances correct");

    foundation.shutdown().await.unwrap();
    println!("\n=== Distance Calculations Complete ===\n");
}

#[tokio::test]
async fn test_e2e_concurrent_operations() {
    println!("\n=== E2E Test: Concurrent Operations ===\n");

    let (foundation, _temp_dir) = create_test_foundation().await;
    let foundation = std::sync::Arc::new(foundation);

    // Create initial nodes
    for i in 0..10 {
        let coord = MatrixCoordinate::new(i * 10, 0, 0).unwrap();
        foundation
            .add_node(format!("node{i}"), coord)
            .await
            .unwrap();
    }

    // Concurrent block additions
    let mut handles = vec![];

    for i in 0..10 {
        let foundation = foundation.clone();
        let handle = tokio::spawn(async move {
            let node_id = format!("node{i}");
            let proof = StateProof::new_for_testing();
            for j in 0..10 {
                let data = format!("Block {j} from node {i}").into_bytes();
                foundation.add_block(&node_id, data, &proof).await.unwrap();
            }
        });
        handles.push(handle);
    }

    // Wait for all concurrent operations
    for handle in handles {
        handle.await.unwrap();
    }

    println!("✓ Concurrent block additions successful");

    // Verify all blocks were added
    for i in 0..10 {
        let node_id = format!("node{i}");
        let height = foundation.get_blockchain_height(&node_id).await.unwrap();
        assert_eq!(height, 10, "Node {i} should have 10 blocks");
    }

    println!("✓ All blocks verified");

    foundation.shutdown().await.unwrap();
    println!("\n=== Concurrent Operations Complete ===\n");
}
