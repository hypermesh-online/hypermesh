// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Comprehensive tests for Vector3D operations

use crate::matrix::coordinate::MatrixCoordinate;
use crate::matrix::tensor::vector::{TensorError, Vector3D};
use std::f64::consts::PI;

#[test]
fn test_vector_creation() {
    let vec = Vector3D::new(1.0, 2.0, 3.0);
    assert_eq!(vec.x, 1.0);
    assert_eq!(vec.y, 2.0);
    assert_eq!(vec.z, 3.0);
}

#[test]
fn test_vector_from_coordinates_positive() {
    let from = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
    let to = MatrixCoordinate::new(15, 25, 35).expect("test: valid coordinate");
    let vec = Vector3D::from_coordinates(&from, &to);

    assert_eq!(vec.x, 5.0);
    assert_eq!(vec.y, 5.0);
    assert_eq!(vec.z, 5.0);
}

#[test]
fn test_vector_from_coordinates_negative() {
    let from = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
    let to = MatrixCoordinate::new(5, 15, 25).expect("test: valid coordinate");
    let vec = Vector3D::from_coordinates(&from, &to);

    assert_eq!(vec.x, -5.0);
    assert_eq!(vec.y, -5.0);
    assert_eq!(vec.z, -5.0);
}

#[test]
fn test_vector_magnitude_3_4_5_triangle() {
    let vec = Vector3D::new(3.0, 4.0, 0.0);
    assert_eq!(vec.magnitude(), 5.0);
}

#[test]
fn test_vector_magnitude_unit_vector() {
    let vec = Vector3D::new(1.0, 0.0, 0.0);
    assert_eq!(vec.magnitude(), 1.0);
}

#[test]
fn test_vector_magnitude_3d() {
    let vec = Vector3D::new(2.0, 3.0, 6.0);
    assert!((vec.magnitude() - 7.0).abs() < 0.001);
}

#[test]
fn test_normalize_unit_vector() {
    let vec = Vector3D::new(5.0, 0.0, 0.0);
    let normalized = vec.normalize().expect("test: expected success");
    assert!((normalized.magnitude() - 1.0).abs() < f64::EPSILON);
    assert_eq!(normalized.x, 1.0);
    assert_eq!(normalized.y, 0.0);
    assert_eq!(normalized.z, 0.0);
}

#[test]
fn test_normalize_zero_vector_error() {
    let vec = Vector3D::new(0.0, 0.0, 0.0);
    let result = vec.normalize();
    assert!(matches!(result, Err(TensorError::ZeroVector)));
}

#[test]
fn test_dot_product_perpendicular() {
    let vec1 = Vector3D::new(1.0, 0.0, 0.0);
    let vec2 = Vector3D::new(0.0, 1.0, 0.0);
    assert_eq!(vec1.dot(&vec2), 0.0);
}

#[test]
fn test_dot_product_parallel_same_direction() {
    let vec1 = Vector3D::new(2.0, 0.0, 0.0);
    let vec2 = Vector3D::new(3.0, 0.0, 0.0);
    assert_eq!(vec1.dot(&vec2), 6.0);
}

#[test]
fn test_dot_product_parallel_opposite() {
    let vec1 = Vector3D::new(2.0, 0.0, 0.0);
    let vec2 = Vector3D::new(-3.0, 0.0, 0.0);
    assert_eq!(vec1.dot(&vec2), -6.0);
}

#[test]
fn test_cross_product_standard_basis() {
    let i = Vector3D::new(1.0, 0.0, 0.0);
    let j = Vector3D::new(0.0, 1.0, 0.0);
    let k = i.cross(&j);

    assert_eq!(k.x, 0.0);
    assert_eq!(k.y, 0.0);
    assert_eq!(k.z, 1.0);
}

#[test]
fn test_cross_product_anticommutative() {
    let vec1 = Vector3D::new(1.0, 2.0, 3.0);
    let vec2 = Vector3D::new(4.0, 5.0, 6.0);

    let cross1 = vec1.cross(&vec2);
    let cross2 = vec2.cross(&vec1);

    assert_eq!(cross1.x, -cross2.x);
    assert_eq!(cross1.y, -cross2.y);
    assert_eq!(cross1.z, -cross2.z);
}

#[test]
fn test_angle_between_perpendicular() {
    let vec1 = Vector3D::new(1.0, 0.0, 0.0);
    let vec2 = Vector3D::new(0.0, 1.0, 0.0);
    let angle = vec1.angle_between(&vec2);
    assert!((angle - PI / 2.0).abs() < 0.001);
}

#[test]
fn test_angle_between_45_degrees() {
    let vec1 = Vector3D::new(1.0, 0.0, 0.0);
    let vec2 = Vector3D::new(1.0, 1.0, 0.0);
    let angle = vec1.angle_between(&vec2);
    assert!((angle - PI / 4.0).abs() < 0.001);
}

#[test]
fn test_projection() {
    let vec1 = Vector3D::new(3.0, 4.0, 0.0);
    let vec2 = Vector3D::new(1.0, 0.0, 0.0);
    let proj = vec1.project_onto(&vec2).expect("test: expected success");

    assert_eq!(proj.x, 3.0);
    assert_eq!(proj.y, 0.0);
    assert_eq!(proj.z, 0.0);
}

#[test]
fn test_projection_onto_zero_vector() {
    let vec1 = Vector3D::new(1.0, 2.0, 3.0);
    let vec2 = Vector3D::new(0.0, 0.0, 0.0);
    let result = vec1.project_onto(&vec2);
    assert!(matches!(result, Err(TensorError::ZeroVector)));
}

#[test]
fn test_vector_addition() {
    let vec1 = Vector3D::new(1.0, 2.0, 3.0);
    let vec2 = Vector3D::new(4.0, 5.0, 6.0);
    let sum = vec1.add(&vec2);

    assert_eq!(sum.x, 5.0);
    assert_eq!(sum.y, 7.0);
    assert_eq!(sum.z, 9.0);
}

#[test]
fn test_vector_subtraction() {
    let vec1 = Vector3D::new(4.0, 5.0, 6.0);
    let vec2 = Vector3D::new(1.0, 2.0, 3.0);
    let diff = vec1.subtract(&vec2);

    assert_eq!(diff.x, 3.0);
    assert_eq!(diff.y, 3.0);
    assert_eq!(diff.z, 3.0);
}

#[test]
fn test_vector_scaling() {
    let vec = Vector3D::new(1.0, 2.0, 3.0);
    let scaled = vec.scale(2.5);

    assert_eq!(scaled.x, 2.5);
    assert_eq!(scaled.y, 5.0);
    assert_eq!(scaled.z, 7.5);
}

#[test]
fn test_lerp_midpoint() {
    let vec1 = Vector3D::new(0.0, 0.0, 0.0);
    let vec2 = Vector3D::new(10.0, 10.0, 10.0);
    let mid = vec1.lerp(&vec2, 0.5);

    assert_eq!(mid.x, 5.0);
    assert_eq!(mid.y, 5.0);
    assert_eq!(mid.z, 5.0);
}

#[test]
fn test_lerp_clamping() {
    let vec1 = Vector3D::new(0.0, 0.0, 0.0);
    let vec2 = Vector3D::new(10.0, 10.0, 10.0);

    let beyond = vec1.lerp(&vec2, 1.5); // Should clamp to 1.0
    assert_eq!(beyond.x, 10.0);

    let before = vec1.lerp(&vec2, -0.5); // Should clamp to 0.0
    assert_eq!(before.x, 0.0);
}

#[test]
fn test_is_zero() {
    let zero = Vector3D::new(0.0, 0.0, 0.0);
    assert!(zero.is_zero());

    let tiny = Vector3D::new(1e-15, 1e-15, 1e-15);
    assert!(tiny.is_zero());

    let non_zero = Vector3D::new(0.1, 0.0, 0.0);
    assert!(!non_zero.is_zero());
}

#[test]
fn test_to_unit() {
    let vec = Vector3D::new(3.0, 4.0, 0.0);
    let unit = vec.to_unit();

    assert!((unit.magnitude() - 1.0).abs() < f64::EPSILON);
    assert!((unit.x - 0.6).abs() < 0.001);
    assert!((unit.y - 0.8).abs() < 0.001);

    let zero = Vector3D::new(0.0, 0.0, 0.0);
    let zero_unit = zero.to_unit();
    assert_eq!(zero_unit.x, 0.0);
    assert_eq!(zero_unit.y, 0.0);
    assert_eq!(zero_unit.z, 0.0);
}

#[test]
fn test_vector_default() {
    let vec = Vector3D::default();
    assert_eq!(vec.x, 0.0);
    assert_eq!(vec.y, 0.0);
    assert_eq!(vec.z, 0.0);
}

#[test]
fn test_vector_display() {
    let vec = Vector3D::new(1.234, 5.678, 9.012);
    let display = format!("{vec}");
    assert!(display.contains("1.23"));
    assert!(display.contains("5.68"));
    assert!(display.contains("9.01"));
}

// Edge cases and numerical stability tests

#[test]
fn test_normalize_very_small_vector() {
    let vec = Vector3D::new(1e-10, 1e-10, 1e-10);
    let normalized = vec.normalize().expect("test: expected success");
    assert!((normalized.magnitude() - 1.0).abs() < 1e-9);
}

#[test]
fn test_angle_between_parallel_vectors() {
    let vec1 = Vector3D::new(1.0, 0.0, 0.0);
    let vec2 = Vector3D::new(2.0, 0.0, 0.0);
    let angle = vec1.angle_between(&vec2);
    assert!(angle < 0.001);
}

#[test]
fn test_angle_between_opposite_vectors() {
    let vec1 = Vector3D::new(1.0, 0.0, 0.0);
    let vec2 = Vector3D::new(-1.0, 0.0, 0.0);
    let angle = vec1.angle_between(&vec2);
    assert!((angle - PI).abs() < 0.001);
}

#[test]
fn test_cross_product_parallel_vectors() {
    let vec1 = Vector3D::new(1.0, 2.0, 3.0);
    let vec2 = Vector3D::new(2.0, 4.0, 6.0);
    let cross = vec1.cross(&vec2);

    assert!(cross.magnitude() < 0.001); // Should be zero vector
}

#[test]
fn test_projection_orthogonal_vectors() {
    let vec1 = Vector3D::new(1.0, 0.0, 0.0);
    let vec2 = Vector3D::new(0.0, 1.0, 0.0);
    let proj = vec1.project_onto(&vec2).expect("test: expected success");

    assert!(proj.magnitude() < 0.001); // Should be zero vector
}
