//! Integration tests for neighbor finding algorithms

use crate::matrix::{MatrixCoordinate, find_neighbors, find_k_nearest, find_neighbors_cubic};
use crate::matrix::neighbors::{
    find_neighbors_spherical, find_neighbors_manhattan, find_neighbors_chebyshev
};

fn create_grid_3x3x3() -> Vec<MatrixCoordinate> {
    let mut coords = Vec::new();
    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                coords.push(MatrixCoordinate::new(x, y, z).unwrap());
            }
        }
    }
    coords
}

fn create_sparse_coordinates() -> Vec<MatrixCoordinate> {
    vec![
        MatrixCoordinate::new(1, 1, 1).unwrap(),
        MatrixCoordinate::new(10, 10, 10).unwrap(),
        MatrixCoordinate::new(100, 100, 100).unwrap(),
        MatrixCoordinate::new(5, 0, 0).unwrap(),
        MatrixCoordinate::new(0, 5, 0).unwrap(),
        MatrixCoordinate::new(0, 0, 5).unwrap(),
        MatrixCoordinate::new(2, 2, 2).unwrap(),
    ]
}

#[test]
fn test_find_neighbors_basic() {
    let center = MatrixCoordinate::origin();
    let candidates = create_grid_3x3x3();

    // Find all neighbors within distance 2
    let neighbors = find_neighbors(&center, &candidates, 2.0);

    // Should include origin and immediate neighbors
    assert!(neighbors.len() > 0);
    assert!(neighbors.contains(&center));
}

#[test]
fn test_find_neighbors_empty_candidates() {
    let center = MatrixCoordinate::origin();
    let candidates = Vec::new();

    let neighbors = find_neighbors(&center, &candidates, 10.0);
    assert_eq!(neighbors.len(), 0);
}

#[test]
fn test_find_neighbors_none_in_range() {
    let center = MatrixCoordinate::origin();
    let candidates = vec![
        MatrixCoordinate::new(1000, 1000, 1000).unwrap(),
        MatrixCoordinate::new(2000, 2000, 2000).unwrap(),
    ];

    let neighbors = find_neighbors(&center, &candidates, 10.0);
    assert_eq!(neighbors.len(), 0);
}

#[test]
fn test_find_neighbors_all_in_range() {
    let center = MatrixCoordinate::origin();
    let candidates = vec![
        MatrixCoordinate::new(1, 0, 0).unwrap(),
        MatrixCoordinate::new(0, 1, 0).unwrap(),
        MatrixCoordinate::new(0, 0, 1).unwrap(),
    ];

    let neighbors = find_neighbors(&center, &candidates, 10.0);
    assert_eq!(neighbors.len(), 3);
}

#[test]
fn test_find_neighbors_exact_threshold() {
    let center = MatrixCoordinate::origin();
    let candidates = vec![
        MatrixCoordinate::new(3, 4, 0).unwrap(), // Distance = 5.0
    ];

    let neighbors_inclusive = find_neighbors(&center, &candidates, 5.0);
    assert_eq!(neighbors_inclusive.len(), 1);

    let neighbors_exclusive = find_neighbors(&center, &candidates, 4.99);
    assert_eq!(neighbors_exclusive.len(), 0);
}

#[test]
fn test_find_k_nearest_basic() {
    let center = MatrixCoordinate::origin();
    let candidates = create_sparse_coordinates();

    let nearest = find_k_nearest(&center, &candidates, 3);
    assert_eq!(nearest.len(), 3);

    // Verify sorted by distance
    for i in 1..nearest.len() {
        assert!(nearest[i - 1].1 <= nearest[i].1);
    }
}

#[test]
fn test_find_k_nearest_k_zero() {
    let center = MatrixCoordinate::origin();
    let candidates = create_sparse_coordinates();

    let nearest = find_k_nearest(&center, &candidates, 0);
    assert_eq!(nearest.len(), 0);
}

#[test]
fn test_find_k_nearest_k_larger_than_candidates() {
    let center = MatrixCoordinate::origin();
    let candidates = vec![
        MatrixCoordinate::new(1, 0, 0).unwrap(),
        MatrixCoordinate::new(2, 0, 0).unwrap(),
    ];

    let nearest = find_k_nearest(&center, &candidates, 10);
    assert_eq!(nearest.len(), 2);
}

#[test]
fn test_find_k_nearest_single() {
    let center = MatrixCoordinate::origin();
    let candidates = create_sparse_coordinates();

    let nearest = find_k_nearest(&center, &candidates, 1);
    assert_eq!(nearest.len(), 1);

    // Should be (1,1,1) as it's closest
    assert_eq!(nearest[0].0, MatrixCoordinate::new(1, 1, 1).unwrap());
}

#[test]
fn test_find_k_nearest_distances_correct() {
    let center = MatrixCoordinate::new(0, 0, 0).unwrap();
    let candidates = vec![
        MatrixCoordinate::new(1, 0, 0).unwrap(),  // distance = 1
        MatrixCoordinate::new(2, 0, 0).unwrap(),  // distance = 2
        MatrixCoordinate::new(3, 0, 0).unwrap(),  // distance = 3
    ];

    let nearest = find_k_nearest(&center, &candidates, 3);

    assert_eq!(nearest[0].1, 1.0);
    assert_eq!(nearest[1].1, 2.0);
    assert_eq!(nearest[2].1, 3.0);
}

#[test]
fn test_find_neighbors_cubic_basic() {
    let center = MatrixCoordinate::origin();
    let candidates = create_grid_3x3x3();

    let neighbors = find_neighbors_cubic(&center, &candidates, 1);

    // Should include 3x3x3 cube = 27 points
    assert_eq!(neighbors.len(), 27);
}

#[test]
fn test_find_neighbors_cubic_radius_zero() {
    let center = MatrixCoordinate::new(5, 5, 5).unwrap();
    let candidates = vec![
        MatrixCoordinate::new(5, 5, 5).unwrap(),
        MatrixCoordinate::new(6, 5, 5).unwrap(),
        MatrixCoordinate::new(5, 6, 5).unwrap(),
    ];

    let neighbors = find_neighbors_cubic(&center, &candidates, 0);
    assert_eq!(neighbors.len(), 1);
    assert_eq!(neighbors[0], center);
}

#[test]
fn test_find_neighbors_cubic_vs_spherical() {
    let center = MatrixCoordinate::origin();
    let candidates = create_grid_3x3x3();

    let cubic = find_neighbors_cubic(&center, &candidates, 1);
    let spherical = find_neighbors_spherical(&center, &candidates, 2.0);

    // Cubic should include more points (corners of cube)
    assert!(cubic.len() >= spherical.len());
}

#[test]
fn test_find_neighbors_spherical() {
    let center = MatrixCoordinate::origin();
    let candidates = create_sparse_coordinates();

    let neighbors = find_neighbors_spherical(&center, &candidates, 5.0);

    // Should be same as find_neighbors
    let expected = find_neighbors(&center, &candidates, 5.0);
    assert_eq!(neighbors.len(), expected.len());
}

#[test]
fn test_find_neighbors_manhattan() {
    let center = MatrixCoordinate::origin();
    let candidates = vec![
        MatrixCoordinate::new(1, 1, 1).unwrap(),  // distance = 3
        MatrixCoordinate::new(2, 2, 2).unwrap(),  // distance = 6
        MatrixCoordinate::new(1, 0, 0).unwrap(),  // distance = 1
        MatrixCoordinate::new(0, 2, 0).unwrap(),  // distance = 2
    ];

    let neighbors = find_neighbors_manhattan(&center, &candidates, 3);
    assert_eq!(neighbors.len(), 3); // (1,1,1), (1,0,0), (0,2,0)

    assert!(neighbors.contains(&MatrixCoordinate::new(1, 1, 1).unwrap()));
    assert!(neighbors.contains(&MatrixCoordinate::new(1, 0, 0).unwrap()));
    assert!(neighbors.contains(&MatrixCoordinate::new(0, 2, 0).unwrap()));
    assert!(!neighbors.contains(&MatrixCoordinate::new(2, 2, 2).unwrap()));
}

#[test]
fn test_find_neighbors_chebyshev() {
    let center = MatrixCoordinate::origin();
    let candidates = vec![
        MatrixCoordinate::new(2, 2, 2).unwrap(),  // distance = 2
        MatrixCoordinate::new(5, 3, 1).unwrap(),  // distance = 5
        MatrixCoordinate::new(1, 1, 1).unwrap(),  // distance = 1
        MatrixCoordinate::new(3, 2, 1).unwrap(),  // distance = 3
    ];

    let neighbors = find_neighbors_chebyshev(&center, &candidates, 3);
    assert_eq!(neighbors.len(), 3);

    assert!(neighbors.contains(&MatrixCoordinate::new(2, 2, 2).unwrap()));
    assert!(neighbors.contains(&MatrixCoordinate::new(1, 1, 1).unwrap()));
    assert!(neighbors.contains(&MatrixCoordinate::new(3, 2, 1).unwrap()));
    assert!(!neighbors.contains(&MatrixCoordinate::new(5, 3, 1).unwrap()));
}

#[test]
fn test_neighbor_finding_with_negative_coordinates() {
    let center = MatrixCoordinate::new(-5, -5, -5).unwrap();
    let candidates = vec![
        MatrixCoordinate::new(-4, -4, -4).unwrap(),
        MatrixCoordinate::new(-10, -10, -10).unwrap(),
        MatrixCoordinate::new(0, 0, 0).unwrap(),
        MatrixCoordinate::new(-6, -5, -5).unwrap(),
    ];

    let neighbors = find_neighbors(&center, &candidates, 5.0);
    assert!(neighbors.len() >= 2);

    let k_nearest = find_k_nearest(&center, &candidates, 2);
    assert_eq!(k_nearest.len(), 2);
}

#[test]
fn test_neighbor_consistency_across_methods() {
    let center = MatrixCoordinate::origin();
    let candidates = create_grid_3x3x3();

    // All methods should handle the same input consistently
    let spherical = find_neighbors_spherical(&center, &candidates, 2.0);
    let euclidean = find_neighbors(&center, &candidates, 2.0);

    assert_eq!(spherical.len(), euclidean.len());
}

#[test]
fn test_distance_metric_comparison() {
    let center = MatrixCoordinate::origin();
    let point = MatrixCoordinate::new(1, 1, 1).unwrap();

    // Euclidean: sqrt(3) ≈ 1.732
    // Manhattan: 3
    // Chebyshev: 1

    let candidates = vec![point];

    let euclidean = find_neighbors(&center, &candidates, 2.0);
    assert_eq!(euclidean.len(), 1);

    let manhattan = find_neighbors_manhattan(&center, &candidates, 3);
    assert_eq!(manhattan.len(), 1);

    let chebyshev = find_neighbors_chebyshev(&center, &candidates, 1);
    assert_eq!(chebyshev.len(), 1);
}

#[test]
fn test_large_scale_neighbor_finding() {
    // Create a large grid
    let mut candidates = Vec::new();
    for x in -10..=10 {
        for y in -10..=10 {
            for z in -10..=10 {
                candidates.push(MatrixCoordinate::new(x, y, z).unwrap());
            }
        }
    }

    let center = MatrixCoordinate::origin();

    // Find neighbors within small radius
    let neighbors = find_neighbors(&center, &candidates, 5.0);
    assert!(neighbors.len() > 0);
    assert!(neighbors.len() < candidates.len());

    // K-nearest should work efficiently
    let k_nearest = find_k_nearest(&center, &candidates, 10);
    assert_eq!(k_nearest.len(), 10);
}

#[test]
fn test_neighbor_finding_preserves_candidates() {
    let center = MatrixCoordinate::origin();
    let candidates = create_sparse_coordinates();
    let original_len = candidates.len();

    // All neighbor-finding operations should not modify input
    let _neighbors = find_neighbors(&center, &candidates, 10.0);
    assert_eq!(candidates.len(), original_len);

    let _k_nearest = find_k_nearest(&center, &candidates, 3);
    assert_eq!(candidates.len(), original_len);

    let _cubic = find_neighbors_cubic(&center, &candidates, 5);
    assert_eq!(candidates.len(), original_len);
}

#[test]
fn test_k_nearest_with_equal_distances() {
    let center = MatrixCoordinate::origin();

    // Create points at equal distances
    let candidates = vec![
        MatrixCoordinate::new(1, 0, 0).unwrap(),  // distance = 1
        MatrixCoordinate::new(0, 1, 0).unwrap(),  // distance = 1
        MatrixCoordinate::new(0, 0, 1).unwrap(),  // distance = 1
        MatrixCoordinate::new(10, 0, 0).unwrap(), // distance = 10
    ];

    let nearest = find_k_nearest(&center, &candidates, 3);
    assert_eq!(nearest.len(), 3);

    // All three nearest should have distance 1
    assert_eq!(nearest[0].1, 1.0);
    assert_eq!(nearest[1].1, 1.0);
    assert_eq!(nearest[2].1, 1.0);
}

#[test]
fn test_cubic_boundary_cases() {
    let center = MatrixCoordinate::new(10, 10, 10).unwrap();

    let candidates = vec![
        MatrixCoordinate::new(11, 10, 10).unwrap(), // On boundary (radius 1)
        MatrixCoordinate::new(9, 10, 10).unwrap(),  // On boundary (radius 1)
        MatrixCoordinate::new(10, 11, 10).unwrap(), // On boundary (radius 1)
        MatrixCoordinate::new(12, 10, 10).unwrap(), // Outside (radius 2)
    ];

    let neighbors_r1 = find_neighbors_cubic(&center, &candidates, 1);
    assert_eq!(neighbors_r1.len(), 3);

    let neighbors_r2 = find_neighbors_cubic(&center, &candidates, 2);
    assert_eq!(neighbors_r2.len(), 4);
}
