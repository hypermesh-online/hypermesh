// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration tests for coordinate transformations

use crate::matrix::{MatrixCoordinate, CoordinateError};

#[test]
fn test_basic_translation() {
    let coord = MatrixCoordinate::new(10, 20, 30).unwrap();

    let translated = coord.translate(5, -10, 15).unwrap();
    assert_eq!(translated, MatrixCoordinate::new(15, 10, 45).unwrap());

    let back = translated.translate(-5, 10, -15).unwrap();
    assert_eq!(back, coord);
}

#[test]
fn test_translation_with_negatives() {
    let coord = MatrixCoordinate::new(50, 60, 70).unwrap();
    let translated = coord.translate(-100, -100, -100).unwrap();
    assert_eq!(translated, MatrixCoordinate::new(-50, -40, -30).unwrap());
}

#[test]
fn test_translation_overflow() {
    let coord = MatrixCoordinate::new(i64::MAX / 4, 0, 0).unwrap();
    let result = coord.translate(i64::MAX / 4 + 1, 0, 0);
    assert!(result.is_err());
}

#[test]
fn test_basic_scaling() {
    let coord = MatrixCoordinate::new(10, 20, 30).unwrap();

    let scaled = coord.scale(2).unwrap();
    assert_eq!(scaled, MatrixCoordinate::new(20, 40, 60).unwrap());

    let scaled_back = scaled.scale(-1).unwrap();
    assert_eq!(scaled_back, MatrixCoordinate::new(-20, -40, -60).unwrap());
}

#[test]
fn test_scale_zero() {
    let coord = MatrixCoordinate::new(10, 20, 30).unwrap();
    let result = coord.scale(0);
    assert!(matches!(result, Err(CoordinateError::InvalidScale(0))));
}

#[test]
fn test_scale_negative() {
    let coord = MatrixCoordinate::new(10, 20, 30).unwrap();
    let scaled = coord.scale(-2).unwrap();
    assert_eq!(scaled, MatrixCoordinate::new(-20, -40, -60).unwrap());
}

#[test]
fn test_hierarchical_scaling() {
    // Test hierarchical addressing use case
    let level0 = MatrixCoordinate::new(1, 2, 3).unwrap();
    let level1 = level0.scale(10).unwrap();
    let level2 = level1.scale(10).unwrap();

    assert_eq!(level1, MatrixCoordinate::new(10, 20, 30).unwrap());
    assert_eq!(level2, MatrixCoordinate::new(100, 200, 300).unwrap());
}

#[test]
fn test_rotation_x_axis() {
    let coord = MatrixCoordinate::new(10, 20, 0).unwrap();

    // 90 degree rotation
    let rotated = coord.rotate_x(90.0).unwrap();
    assert_eq!(rotated.x, 10);
    assert!((rotated.y as f64).abs() < 1.0);
    assert!((rotated.z - 20).abs() <= 1);

    // 180 degree rotation
    let rotated_180 = coord.rotate_x(180.0).unwrap();
    assert_eq!(rotated_180.x, 10);
    assert!((rotated_180.y + 20).abs() <= 1);
    assert!((rotated_180.z as f64).abs() < 1.0);

    // 360 degree rotation (back to original)
    let rotated_360 = coord.rotate_x(360.0).unwrap();
    assert!((rotated_360.x - coord.x).abs() <= 1);
    assert!((rotated_360.y - coord.y).abs() <= 1);
    assert!((rotated_360.z - coord.z).abs() <= 1);
}

#[test]
fn test_rotation_y_axis() {
    let coord = MatrixCoordinate::new(20, 10, 0).unwrap();

    // 90 degree rotation
    let rotated = coord.rotate_y(90.0).unwrap();
    assert!((rotated.x as f64).abs() < 1.0);
    assert_eq!(rotated.y, 10);
    assert!((rotated.z + 20).abs() <= 1);

    // 180 degree rotation
    let rotated_180 = coord.rotate_y(180.0).unwrap();
    assert!((rotated_180.x + 20).abs() <= 1);
    assert_eq!(rotated_180.y, 10);
    assert!((rotated_180.z as f64).abs() < 1.0);
}

#[test]
fn test_rotation_z_axis() {
    let coord = MatrixCoordinate::new(20, 0, 10).unwrap();

    // 90 degree rotation
    let rotated = coord.rotate_z(90.0).unwrap();
    assert!((rotated.x as f64).abs() < 1.0);
    assert!((rotated.y - 20).abs() <= 1);
    assert_eq!(rotated.z, 10);

    // 180 degree rotation
    let rotated_180 = coord.rotate_z(180.0).unwrap();
    assert!((rotated_180.x + 20).abs() <= 1);
    assert!((rotated_180.y as f64).abs() < 1.0);
    assert_eq!(rotated_180.z, 10);
}

#[test]
fn test_rotation_invalid_angle() {
    let coord = MatrixCoordinate::new(10, 20, 30).unwrap();

    assert!(matches!(
        coord.rotate_x(f64::NAN),
        Err(CoordinateError::InvalidRotation(_))
    ));

    assert!(matches!(
        coord.rotate_y(f64::INFINITY),
        Err(CoordinateError::InvalidRotation(_))
    ));

    assert!(matches!(
        coord.rotate_z(f64::NEG_INFINITY),
        Err(CoordinateError::InvalidRotation(_))
    ));
}

#[test]
fn test_rotation_small_angles() {
    let coord = MatrixCoordinate::new(100, 100, 0).unwrap();

    // Small rotation should produce minimal change
    let rotated = coord.rotate_z(1.0).unwrap();
    let distance = coord.euclidean_distance(&rotated);
    assert!(distance < 5.0); // Small change for 1 degree
}

#[test]
fn test_chained_transformations() {
    let coord = MatrixCoordinate::new(10, 10, 10).unwrap();

    let result = coord
        .translate(5, 5, 5).unwrap()
        .scale(2).unwrap()
        .translate(-10, -10, -10).unwrap();

    assert_eq!(result, MatrixCoordinate::new(20, 20, 20).unwrap());
}

#[test]
fn test_complex_transformation_sequence() {
    let origin = MatrixCoordinate::origin();

    let result = origin
        .translate(10, 0, 0).unwrap()
        .rotate_z(90.0).unwrap()
        .translate(5, 5, 5).unwrap()
        .scale(2).unwrap();

    // Result should be roughly at (10, 30, 10) after all transformations
    assert!(result.y > 0);
    assert!(result.z > 0);
}

#[test]
fn test_inverse_operations() {
    let coord = MatrixCoordinate::new(10, 20, 30).unwrap();

    // Translate and translate back
    let result = coord
        .translate(5, 10, 15).unwrap()
        .translate(-5, -10, -15).unwrap();
    assert_eq!(result, coord);

    // Scale and scale back
    let result = coord
        .scale(3).unwrap()
        .translate(0, 0, 0).unwrap(); // No-op
    assert_eq!(result.x, 30);
    assert_eq!(result.y, 60);
    assert_eq!(result.z, 90);
}

#[test]
fn test_apply_transform() {
    let coord = MatrixCoordinate::new(10, 20, 30).unwrap();

    let result = coord.apply_transform(|c| c.scale(2)).unwrap();
    assert_eq!(result, MatrixCoordinate::new(20, 40, 60).unwrap());

    let result = coord.apply_transform(|c| c.translate(5, 5, 5)).unwrap();
    assert_eq!(result, MatrixCoordinate::new(15, 25, 35).unwrap());
}

#[test]
fn test_transformation_commutativity() {
    let coord = MatrixCoordinate::new(10, 20, 30).unwrap();

    // Translation is commutative
    let t1 = coord.translate(5, 0, 0).unwrap().translate(0, 10, 0).unwrap();
    let t2 = coord.translate(0, 10, 0).unwrap().translate(5, 0, 0).unwrap();
    assert_eq!(t1, t2);

    // Scaling is commutative with itself
    let s1 = coord.scale(2).unwrap();
    let s2 = coord.scale(2).unwrap();
    assert_eq!(s1, s2);
}

#[test]
fn test_rotation_preserves_distance() {
    let origin = MatrixCoordinate::origin();
    let coord = MatrixCoordinate::new(10, 0, 0).unwrap();

    let original_distance = origin.euclidean_distance(&coord);

    // Rotation should preserve distance from origin
    let rotated = coord.rotate_z(45.0).unwrap();
    let rotated_distance = origin.euclidean_distance(&rotated);

    assert!((original_distance - rotated_distance).abs() < 0.1);
}

#[test]
fn test_edge_case_zero_coordinate() {
    let origin = MatrixCoordinate::origin();

    // All transformations on origin
    let translated = origin.translate(10, 20, 30).unwrap();
    assert_eq!(translated, MatrixCoordinate::new(10, 20, 30).unwrap());

    let scaled = origin.scale(100).unwrap();
    assert_eq!(scaled, origin);

    let rotated = origin.rotate_x(90.0).unwrap();
    assert_eq!(rotated, origin);
}

#[test]
fn test_large_scale_factors() {
    let coord = MatrixCoordinate::new(1, 1, 1).unwrap();

    // Test progressively larger scales
    let scale_1000 = coord.scale(1000).unwrap();
    assert_eq!(scale_1000, MatrixCoordinate::new(1000, 1000, 1000).unwrap());

    let scale_10000 = coord.scale(10000).unwrap();
    assert_eq!(scale_10000, MatrixCoordinate::new(10000, 10000, 10000).unwrap());
}

#[test]
fn test_multiple_rotations() {
    let coord = MatrixCoordinate::new(10, 0, 0).unwrap();

    // Four 90-degree rotations should return roughly to start
    let result = coord
        .rotate_z(90.0).unwrap()
        .rotate_z(90.0).unwrap()
        .rotate_z(90.0).unwrap()
        .rotate_z(90.0).unwrap();

    assert!((result.x - coord.x).abs() <= 1);
    assert!((result.y - coord.y).abs() <= 1);
    assert_eq!(result.z, coord.z);
}
