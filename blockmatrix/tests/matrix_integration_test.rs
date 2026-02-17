// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration tests for Matrix Coordinate System
//!
//! This file tests the matrix module in isolation to ensure 100% functionality
//! independent of other blockmatrix modules.

use blockmatrix::matrix::{
    MatrixCoordinate, CoordinateError, find_neighbors, find_k_nearest, find_neighbors_cubic,
};

#[test]
fn test_matrix_coordinate_creation() {
    let coord = MatrixCoordinate::new(10, 20, 30).unwrap();
    assert_eq!(coord.x, 10);
    assert_eq!(coord.y, 20);
    assert_eq!(coord.z, 30);
}

#[test]
fn test_matrix_origin() {
    let origin = MatrixCoordinate::origin();
    assert_eq!(origin.x, 0);
    assert_eq!(origin.y, 0);
    assert_eq!(origin.z, 0);
}

#[test]
fn test_euclidean_distance() {
    let a = MatrixCoordinate::new(0, 0, 0).unwrap();
    let b = MatrixCoordinate::new(3, 4, 0).unwrap();
    assert_eq!(a.euclidean_distance(&b), 5.0);
}

#[test]
fn test_manhattan_distance() {
    let a = MatrixCoordinate::new(0, 0, 0).unwrap();
    let b = MatrixCoordinate::new(3, 4, 5).unwrap();
    assert_eq!(a.manhattan_distance(&b), 12);
}

#[test]
fn test_chebyshev_distance() {
    let a = MatrixCoordinate::new(0, 0, 0).unwrap();
    let b = MatrixCoordinate::new(3, 4, 2).unwrap();
    assert_eq!(a.chebyshev_distance(&b), 4);
}

#[test]
fn test_translation() {
    let coord = MatrixCoordinate::new(10, 20, 30).unwrap();
    let translated = coord.translate(5, -10, 15).unwrap();
    assert_eq!(translated, MatrixCoordinate::new(15, 10, 45).unwrap());
}

#[test]
fn test_scaling() {
    let coord = MatrixCoordinate::new(10, 20, 30).unwrap();
    let scaled = coord.scale(2).unwrap();
    assert_eq!(scaled, MatrixCoordinate::new(20, 40, 60).unwrap());
}

#[test]
fn test_rotation_x() {
    let coord = MatrixCoordinate::new(10, 20, 0).unwrap();
    let rotated = coord.rotate_x(90.0).unwrap();
    assert_eq!(rotated.x, 10);
    assert!((rotated.y as f64).abs() < 1.0);
    assert!((rotated.z - 20).abs() <= 1);
}

#[test]
fn test_find_neighbors() {
    let center = MatrixCoordinate::new(0, 0, 0).unwrap();
    let candidates = vec![
        MatrixCoordinate::new(1, 1, 1).unwrap(),
        MatrixCoordinate::new(10, 10, 10).unwrap(),
        MatrixCoordinate::new(2, 2, 2).unwrap(),
    ];

    let neighbors = find_neighbors(&center, &candidates, 5.0);
    assert!(neighbors.len() >= 2);
}

#[test]
fn test_find_k_nearest() {
    let center = MatrixCoordinate::new(0, 0, 0).unwrap();
    let candidates = vec![
        MatrixCoordinate::new(1, 0, 0).unwrap(),
        MatrixCoordinate::new(10, 0, 0).unwrap(),
        MatrixCoordinate::new(2, 0, 0).unwrap(),
    ];

    let nearest = find_k_nearest(&center, &candidates, 2);
    assert_eq!(nearest.len(), 2);
    assert_eq!(nearest[0].0, MatrixCoordinate::new(1, 0, 0).unwrap());
}

#[test]
fn test_find_neighbors_cubic() {
    let center = MatrixCoordinate::new(0, 0, 0).unwrap();
    let candidates = vec![
        MatrixCoordinate::new(1, 1, 1).unwrap(),
        MatrixCoordinate::new(5, 0, 0).unwrap(),
        MatrixCoordinate::new(2, 2, 2).unwrap(),
    ];

    let neighbors = find_neighbors_cubic(&center, &candidates, 2);
    assert_eq!(neighbors.len(), 2);
}

#[test]
fn test_coordinate_validation() {
    let max_coord = i64::MAX / 4;
    let min_coord = i64::MIN / 4;

    assert!(MatrixCoordinate::new(max_coord, 0, 0).is_ok());
    assert!(MatrixCoordinate::new(0, min_coord, 0).is_ok());
    assert!(MatrixCoordinate::new(max_coord + 1, 0, 0).is_err());
}

#[test]
fn test_coordinate_serialization() {
    let coord = MatrixCoordinate::new(10, 20, 30).unwrap();
    let json = serde_json::to_string(&coord).unwrap();
    let deserialized: MatrixCoordinate = serde_json::from_str(&json).unwrap();
    assert_eq!(coord, deserialized);
}

#[test]
fn test_chained_transformations() {
    let coord = MatrixCoordinate::new(10, 10, 10).unwrap();
    let result = coord
        .translate(5, 5, 5).unwrap()
        .scale(2).unwrap();

    assert_eq!(result, MatrixCoordinate::new(30, 30, 30).unwrap());
}

#[test]
fn test_distance_symmetry() {
    let a = MatrixCoordinate::new(10, 20, 30).unwrap();
    let b = MatrixCoordinate::new(50, 60, 70).unwrap();

    assert_eq!(a.euclidean_distance(&b), b.euclidean_distance(&a));
    assert_eq!(a.manhattan_distance(&b), b.manhattan_distance(&a));
    assert_eq!(a.chebyshev_distance(&b), b.chebyshev_distance(&a));
}

#[test]
fn test_negative_coordinates() {
    let a = MatrixCoordinate::new(-10, -20, -30).unwrap();
    let b = MatrixCoordinate::new(10, 20, 30).unwrap();

    let dist = a.euclidean_distance(&b);
    assert!(dist > 0.0);
}

#[test]
fn test_is_within_distance() {
    let center = MatrixCoordinate::new(0, 0, 0).unwrap();
    let near = MatrixCoordinate::new(3, 4, 0).unwrap();

    assert!(center.is_within_distance(&near, 10.0));
    assert!(!center.is_within_distance(&near, 4.0));
}

#[test]
fn test_squared_euclidean_distance() {
    let a = MatrixCoordinate::new(0, 0, 0).unwrap();
    let b = MatrixCoordinate::new(3, 4, 0).unwrap();
    assert_eq!(a.squared_euclidean_distance(&b), 25);
}

#[test]
fn test_scale_zero_error() {
    let coord = MatrixCoordinate::new(10, 20, 30).unwrap();
    let result = coord.scale(0);
    assert!(matches!(result, Err(CoordinateError::InvalidScale(0))));
}

#[test]
fn test_invalid_rotation() {
    let coord = MatrixCoordinate::new(10, 20, 30).unwrap();
    assert!(matches!(
        coord.rotate_x(f64::NAN),
        Err(CoordinateError::InvalidRotation(_))
    ));
}

#[test]
fn test_360_degree_rotation() {
    let coord = MatrixCoordinate::new(10, 20, 30).unwrap();
    let rotated = coord.rotate_x(360.0).unwrap();

    assert!((rotated.x - coord.x).abs() <= 1);
    assert!((rotated.y - coord.y).abs() <= 1);
    assert!((rotated.z - coord.z).abs() <= 1);
}

#[test]
fn test_neighbor_finding_empty() {
    let center = MatrixCoordinate::new(0, 0, 0).unwrap();
    let candidates = Vec::new();

    let neighbors = find_neighbors(&center, &candidates, 5.0);
    assert_eq!(neighbors.len(), 0);
}

#[test]
fn test_k_nearest_k_zero() {
    let center = MatrixCoordinate::new(0, 0, 0).unwrap();
    let candidates = vec![MatrixCoordinate::new(1, 0, 0).unwrap()];

    let nearest = find_k_nearest(&center, &candidates, 0);
    assert_eq!(nearest.len(), 0);
}

#[test]
fn test_coordinate_display() {
    let coord = MatrixCoordinate::new(10, 20, 30).unwrap();
    assert_eq!(format!("{}", coord), "(10, 20, 30)");
}

#[test]
fn test_coordinate_equality() {
    let a = MatrixCoordinate::new(10, 20, 30).unwrap();
    let b = MatrixCoordinate::new(10, 20, 30).unwrap();
    let c = MatrixCoordinate::new(10, 20, 31).unwrap();

    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_coordinate_hash() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    let coord1 = MatrixCoordinate::new(10, 20, 30).unwrap();
    let coord2 = MatrixCoordinate::new(10, 20, 30).unwrap();
    let coord3 = MatrixCoordinate::new(11, 20, 30).unwrap();

    set.insert(coord1);
    set.insert(coord2);
    set.insert(coord3);

    assert_eq!(set.len(), 2);
}
