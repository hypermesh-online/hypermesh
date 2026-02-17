// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Comprehensive tests for routing algorithms

use crate::matrix::tensor::routing::*;
use crate::matrix::tensor::vector::Vector3D;
use crate::matrix::coordinate::MatrixCoordinate;

#[test]
fn test_calculate_routing_vector_simple() {
    let source = MatrixCoordinate::new(0, 0, 0).unwrap();
    let dest = MatrixCoordinate::new(10, 0, 0).unwrap();
    let direction = calculate_routing_vector(&source, &dest);

    assert!((direction.magnitude() - 1.0).abs() < 0.001);
    assert!((direction.x - 1.0).abs() < 0.001);
    assert!((direction.y - 0.0).abs() < 0.001);
}

#[test]
fn test_calculate_routing_vector_diagonal() {
    let source = MatrixCoordinate::new(0, 0, 0).unwrap();
    let dest = MatrixCoordinate::new(10, 10, 10).unwrap();
    let direction = calculate_routing_vector(&source, &dest);

    assert!((direction.magnitude() - 1.0).abs() < 0.001);

    let expected = 1.0 / 3.0_f64.sqrt();
    assert!((direction.x - expected).abs() < 0.001);
    assert!((direction.y - expected).abs() < 0.001);
    assert!((direction.z - expected).abs() < 0.001);
}

#[test]
fn test_calculate_routing_path_single_hop() {
    let source = MatrixCoordinate::new(0, 0, 0).unwrap();
    let dest = MatrixCoordinate::new(5, 0, 0).unwrap();

    let path = calculate_routing_path(&source, &dest, 10.0);

    assert_eq!(path.len(), 2);
    assert_eq!(path[0], source);
    assert_eq!(path[1], dest);
}

#[test]
fn test_calculate_routing_path_multi_hop() {
    let source = MatrixCoordinate::new(0, 0, 0).unwrap();
    let dest = MatrixCoordinate::new(100, 0, 0).unwrap();

    let path = calculate_routing_path(&source, &dest, 30.0);

    assert!(path.len() > 2);
    assert_eq!(path[0], source);
    assert_eq!(path[path.len() - 1], dest);

    // Verify no hop exceeds max distance
    for i in 1..path.len() {
        let hop_distance = path[i - 1].euclidean_distance(&path[i]);
        assert!(hop_distance <= 35.0); // Some tolerance for rounding
    }
}

#[test]
fn test_calculate_routing_path_zero_max_distance() {
    let source = MatrixCoordinate::new(0, 0, 0).unwrap();
    let dest = MatrixCoordinate::new(100, 0, 0).unwrap();

    let path = calculate_routing_path(&source, &dest, 0.0);

    assert_eq!(path.len(), 2);
    assert_eq!(path[0], source);
    assert_eq!(path[1], dest);
}

#[test]
fn test_routing_similarity_same_direction() {
    let dir1 = Vector3D::new(1.0, 0.0, 0.0);
    let dir2 = Vector3D::new(2.0, 0.0, 0.0); // Same direction, different magnitude

    let similarity = routing_similarity(&dir1, &dir2);
    assert!((similarity - 1.0).abs() < 0.001);
}

#[test]
fn test_routing_similarity_perpendicular() {
    let dir1 = Vector3D::new(1.0, 0.0, 0.0);
    let dir2 = Vector3D::new(0.0, 1.0, 0.0);

    let similarity = routing_similarity(&dir1, &dir2);
    assert!((similarity - 0.0).abs() < 0.001);
}

#[test]
fn test_routing_similarity_opposite() {
    let dir1 = Vector3D::new(1.0, 0.0, 0.0);
    let dir2 = Vector3D::new(-1.0, 0.0, 0.0);

    let similarity = routing_similarity(&dir1, &dir2);
    assert!((similarity - (-1.0)).abs() < 0.001);
}

#[test]
fn test_routing_similarity_45_degrees() {
    let dir1 = Vector3D::new(1.0, 0.0, 0.0);
    let dir2 = Vector3D::new(1.0, 1.0, 0.0);

    let similarity = routing_similarity(&dir1, &dir2);
    let expected = 1.0 / 2.0_f64.sqrt(); // cos(45°)
    assert!((similarity - expected).abs() < 0.001);
}

#[test]
fn test_find_aligned_nodes_perfect_alignment() {
    let source = MatrixCoordinate::new(0, 0, 0).unwrap();
    let target_direction = Vector3D::new(1.0, 0.0, 0.0);

    let candidates = vec![
        MatrixCoordinate::new(10, 0, 0).unwrap(),  // Perfect alignment
        MatrixCoordinate::new(5, 0, 0).unwrap(),   // Perfect alignment
        MatrixCoordinate::new(0, 10, 0).unwrap(),  // Perpendicular
    ];

    let aligned = find_aligned_nodes(&source, &target_direction, &candidates, 0.99);

    assert_eq!(aligned.len(), 2);
    assert!(aligned.contains(&MatrixCoordinate::new(10, 0, 0).unwrap()));
    assert!(aligned.contains(&MatrixCoordinate::new(5, 0, 0).unwrap()));
}

#[test]
fn test_find_aligned_nodes_with_tolerance() {
    let source = MatrixCoordinate::new(0, 0, 0).unwrap();
    let target_direction = Vector3D::new(1.0, 0.0, 0.0);

    let candidates = vec![
        MatrixCoordinate::new(10, 1, 0).unwrap(),  // Slightly off
        MatrixCoordinate::new(10, 3, 0).unwrap(),  // More off
        MatrixCoordinate::new(10, 10, 0).unwrap(), // 45 degrees
    ];

    let aligned = find_aligned_nodes(&source, &target_direction, &candidates, 0.7);

    // Should include nodes within ~45 degree cone
    assert!(aligned.len() >= 2);
}

#[test]
fn test_find_aligned_nodes_excludes_source() {
    let source = MatrixCoordinate::new(5, 5, 5).unwrap();
    let target_direction = Vector3D::new(1.0, 0.0, 0.0);

    let candidates = vec![
        MatrixCoordinate::new(5, 5, 5).unwrap(),   // Source itself
        MatrixCoordinate::new(10, 5, 5).unwrap(),  // Valid candidate
    ];

    let aligned = find_aligned_nodes(&source, &target_direction, &candidates, 0.5);

    assert_eq!(aligned.len(), 1);
    assert_eq!(aligned[0], MatrixCoordinate::new(10, 5, 5).unwrap());
}

#[test]
fn test_calculate_orthogonal_routes() {
    let primary = Vector3D::new(1.0, 0.0, 0.0);
    let orthogonals = calculate_orthogonal_routes(&primary);

    assert_eq!(orthogonals.len(), 2);

    // Check orthogonality to primary
    assert!((primary.dot(&orthogonals[0])).abs() < 0.001);
    assert!((primary.dot(&orthogonals[1])).abs() < 0.001);

    // Check orthogonality to each other
    assert!((orthogonals[0].dot(&orthogonals[1])).abs() < 0.001);

    // Check unit vectors
    assert!((orthogonals[0].magnitude() - 1.0).abs() < 0.001);
    assert!((orthogonals[1].magnitude() - 1.0).abs() < 0.001);
}

#[test]
fn test_calculate_orthogonal_routes_diagonal() {
    let primary = Vector3D::new(1.0, 1.0, 1.0);
    let orthogonals = calculate_orthogonal_routes(&primary);

    assert_eq!(orthogonals.len(), 2);

    let normalized_primary = primary.normalize().unwrap();
    assert!((normalized_primary.dot(&orthogonals[0])).abs() < 0.001);
    assert!((normalized_primary.dot(&orthogonals[1])).abs() < 0.001);
}

#[test]
fn test_calculate_load_balanced_routes_direct() {
    let source = MatrixCoordinate::new(0, 0, 0).unwrap();
    let dest = MatrixCoordinate::new(100, 0, 0).unwrap();

    let routes = calculate_load_balanced_routes(&source, &dest, 0, 0.0);

    assert_eq!(routes.len(), 1);
    assert!((routes[0].x - 1.0).abs() < 0.001);
}

#[test]
fn test_calculate_load_balanced_routes_with_spread() {
    let source = MatrixCoordinate::new(0, 0, 0).unwrap();
    let dest = MatrixCoordinate::new(100, 0, 0).unwrap();

    let routes = calculate_load_balanced_routes(&source, &dest, 3, 0.5);

    assert_eq!(routes.len(), 4); // Primary + 3 alternatives

    // All should be unit vectors
    for route in &routes {
        assert!((route.magnitude() - 1.0).abs() < 0.001);
    }

    // Should have some spread
    let primary = &routes[0];
    for i in 1..routes.len() {
        let similarity = routing_similarity(primary, &routes[i]);
        assert!(similarity < 1.0); // Not identical to primary
    }
}

#[test]
fn test_score_route_quality_direct_path() {
    let path = vec![
        MatrixCoordinate::new(0, 0, 0).unwrap(),
        MatrixCoordinate::new(10, 0, 0).unwrap(),
        MatrixCoordinate::new(20, 0, 0).unwrap(),
        MatrixCoordinate::new(30, 0, 0).unwrap(),
    ];

    let score = score_route_quality(&path, 10.0);
    assert!(score > 80.0); // Should score high for perfect path
}

#[test]
fn test_score_route_quality_zigzag() {
    let path = vec![
        MatrixCoordinate::new(0, 0, 0).unwrap(),
        MatrixCoordinate::new(10, 10, 0).unwrap(),
        MatrixCoordinate::new(20, -10, 0).unwrap(),
        MatrixCoordinate::new(30, 0, 0).unwrap(),
    ];

    let score = score_route_quality(&path, 10.0);
    assert!(score < 50.0); // Should score low for zigzag path
}

#[test]
fn test_score_route_quality_inefficient() {
    let path = vec![
        MatrixCoordinate::new(0, 0, 0).unwrap(),
        MatrixCoordinate::new(0, 50, 0).unwrap(),
        MatrixCoordinate::new(10, 50, 0).unwrap(),
        MatrixCoordinate::new(10, 0, 0).unwrap(),
    ];

    let score = score_route_quality(&path, 10.0);

    // Very inefficient path (goes way out of the way)
    assert!(score < 30.0);
}

#[test]
fn test_score_route_quality_empty_path() {
    let path = vec![];
    let score = score_route_quality(&path, 10.0);
    assert_eq!(score, 0.0);
}

#[test]
fn test_score_route_quality_single_node() {
    let path = vec![MatrixCoordinate::new(0, 0, 0).unwrap()];
    let score = score_route_quality(&path, 10.0);
    assert_eq!(score, 0.0);
}

// Edge cases and stress tests

#[test]
fn test_routing_with_large_coordinates() {
    let source = MatrixCoordinate::new(1000000, 1000000, 1000000).unwrap();
    let dest = MatrixCoordinate::new(1000100, 1000100, 1000100).unwrap();

    let direction = calculate_routing_vector(&source, &dest);
    assert!((direction.magnitude() - 1.0).abs() < 0.001);
}

#[test]
fn test_find_aligned_nodes_empty_candidates() {
    let source = MatrixCoordinate::new(0, 0, 0).unwrap();
    let target_direction = Vector3D::new(1.0, 0.0, 0.0);

    let aligned = find_aligned_nodes(&source, &target_direction, &[], 0.5);
    assert_eq!(aligned.len(), 0);
}

#[test]
fn test_orthogonal_routes_zero_vector() {
    let primary = Vector3D::new(0.0, 0.0, 0.0);
    let orthogonals = calculate_orthogonal_routes(&primary);

    // Should handle gracefully, returning some orthogonal vectors
    assert_eq!(orthogonals.len(), 2);
}