// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration tests for MatrixCoordinate

use crate::matrix::MatrixCoordinate;

#[test]
fn test_coordinate_creation_and_validation() {
    // Valid coordinates
    assert!(MatrixCoordinate::new(0, 0, 0).is_ok());
    assert!(MatrixCoordinate::new(100, 200, 300).is_ok());
    assert!(MatrixCoordinate::new(-100, -200, -300).is_ok());

    // Test large valid values
    let max_safe = i64::MAX / 4;
    let min_safe = i64::MIN / 4;
    assert!(MatrixCoordinate::new(max_safe, 0, 0).is_ok());
    assert!(MatrixCoordinate::new(0, min_safe, 0).is_ok());
}

#[test]
fn test_origin() {
    let origin = MatrixCoordinate::origin();
    assert_eq!(origin.x, 0);
    assert_eq!(origin.y, 0);
    assert_eq!(origin.z, 0);
}

#[test]
fn test_distance_calculations() {
    let origin = MatrixCoordinate::origin();
    let point = MatrixCoordinate::new(3, 4, 0).expect("test: valid coordinate");

    // Euclidean distance (3-4-5 triangle)
    assert_eq!(origin.euclidean_distance(&point), 5.0);

    // Manhattan distance
    assert_eq!(origin.manhattan_distance(&point), 7);

    // Chebyshev distance
    assert_eq!(origin.chebyshev_distance(&point), 4);

    // Squared distance (avoid sqrt)
    assert_eq!(origin.squared_euclidean_distance(&point), 25);
}

#[test]
fn test_distance_symmetry() {
    let a = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
    let b = MatrixCoordinate::new(50, 60, 70).expect("test: valid coordinate");

    // Distance should be symmetric
    assert_eq!(a.euclidean_distance(&b), b.euclidean_distance(&a));
    assert_eq!(a.manhattan_distance(&b), b.manhattan_distance(&a));
    assert_eq!(a.chebyshev_distance(&b), b.chebyshev_distance(&a));
}

#[test]
fn test_distance_with_negative_coordinates() {
    let a = MatrixCoordinate::new(-10, -20, -30).expect("test: valid coordinate");
    let b = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");

    let euclidean = a.euclidean_distance(&b);
    assert!(euclidean > 0.0);

    let manhattan = a.manhattan_distance(&b);
    assert_eq!(manhattan, 120);
}

#[test]
fn test_is_within_distance() {
    let center = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
    let near = MatrixCoordinate::new(3, 4, 0).expect("test: valid coordinate");
    let far = MatrixCoordinate::new(100, 100, 100).expect("test: valid coordinate");

    assert!(center.is_within_distance(&near, 10.0));
    assert!(center.is_within_distance(&near, 5.0));
    assert!(!center.is_within_distance(&near, 4.0));
    assert!(!center.is_within_distance(&far, 50.0));
}

#[test]
fn test_coordinate_equality() {
    let a = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
    let b = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
    let c = MatrixCoordinate::new(10, 20, 31).expect("test: valid coordinate");

    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_ne!(b, c);
}

#[test]
fn test_coordinate_display() {
    let coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
    assert_eq!(format!("{coord}"), "(10,20,30)");

    let negative = MatrixCoordinate::new(-5, -10, -15).expect("test: valid coordinate");
    assert_eq!(format!("{negative}"), "(-5,-10,-15)");
}

#[test]
fn test_coordinate_serialization() {
    let coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");

    // Serialize to JSON
    let json = serde_json::to_string(&coord).expect("test: serialization");
    assert!(json.contains("10"));
    assert!(json.contains("20"));
    assert!(json.contains("30"));

    // Deserialize from JSON
    let deserialized: MatrixCoordinate = serde_json::from_str(&json).expect("test: deserialization");
    assert_eq!(coord, deserialized);
}

#[test]
fn test_zero_distance() {
    let coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");

    assert_eq!(coord.euclidean_distance(&coord), 0.0);
    assert_eq!(coord.manhattan_distance(&coord), 0);
    assert_eq!(coord.chebyshev_distance(&coord), 0);
    assert_eq!(coord.squared_euclidean_distance(&coord), 0);
}

#[test]
fn test_triangle_inequality() {
    // Triangle inequality: d(a,c) <= d(a,b) + d(b,c)
    let a = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
    let b = MatrixCoordinate::new(10, 10, 10).expect("test: valid coordinate");
    let c = MatrixCoordinate::new(20, 0, 0).expect("test: valid coordinate");

    let d_ac = a.euclidean_distance(&c);
    let d_ab = a.euclidean_distance(&b);
    let d_bc = b.euclidean_distance(&c);

    assert!(d_ac <= d_ab + d_bc + 0.0001); // Small epsilon for float comparison
}

#[test]
fn test_coordinate_properties() {
    let coord = MatrixCoordinate::new(42, 84, 126).expect("test: valid coordinate");

    // Test that coordinate values are preserved
    assert_eq!(coord.x, 42);
    assert_eq!(coord.y, 84);
    assert_eq!(coord.z, 126);

    // Test clone
    let cloned = coord;
    assert_eq!(coord, cloned);

    // Test copy
    let copied = coord;
    assert_eq!(coord, copied);
}

#[test]
fn test_extreme_coordinates() {
    let max_coord = i64::MAX / 4;
    let min_coord = i64::MIN / 4;

    let max_point = MatrixCoordinate::new(max_coord, max_coord, max_coord).expect("test: valid coordinate");
    let min_point = MatrixCoordinate::new(min_coord, min_coord, min_coord).expect("test: valid coordinate");

    // Should not panic
    let _distance = max_point.euclidean_distance(&min_point);
    let _manhattan = max_point.manhattan_distance(&min_point);
    let _chebyshev = max_point.chebyshev_distance(&min_point);
}

#[test]
fn test_coordinate_hash() {
    use std::collections::HashSet;

    let mut set = HashSet::new();

    let coord1 = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
    let coord2 = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
    let coord3 = MatrixCoordinate::new(11, 20, 30).expect("test: valid coordinate");

    set.insert(coord1);
    set.insert(coord2); // Duplicate, should not increase size
    set.insert(coord3);

    assert_eq!(set.len(), 2);
    assert!(set.contains(&coord1));
    assert!(set.contains(&coord3));
}

#[test]
fn test_mixed_sign_coordinates() {
    let coord = MatrixCoordinate::new(-10, 20, -30).expect("test: valid coordinate");
    let other = MatrixCoordinate::new(10, -20, 30).expect("test: valid coordinate");

    let dist = coord.euclidean_distance(&other);
    assert!(dist > 0.0);

    // Test all combinations of signs
    let combos = vec![
        (1, 1, 1),
        (1, 1, -1),
        (1, -1, 1),
        (1, -1, -1),
        (-1, 1, 1),
        (-1, 1, -1),
        (-1, -1, 1),
        (-1, -1, -1),
    ];

    for (sx, sy, sz) in combos {
        let c = MatrixCoordinate::new(sx * 10, sy * 20, sz * 30).expect("test: valid coordinate");
        assert!(c.validate().is_ok());
    }
}
