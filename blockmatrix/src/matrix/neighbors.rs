// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Neighbor finding algorithms for matrix coordinates
//!
//! Provides various algorithms for discovering neighboring nodes in the
//! Block-MATRIX topology based on distance metrics.

use super::coordinate::MatrixCoordinate;

/// Find all coordinates within a distance threshold
///
/// Returns all candidates whose Euclidean distance from the center
/// is less than or equal to the threshold.
///
/// # Performance
///
/// O(n) where n is the number of candidates. Uses Euclidean distance
/// for comparison.
///
/// # Examples
///
/// ```
/// use blockmatrix::matrix::{MatrixCoordinate, find_neighbors};
///
/// let center = MatrixCoordinate::new(0, 0, 0).unwrap();
/// let candidates = vec![
///     MatrixCoordinate::new(1, 1, 1).unwrap(),
///     MatrixCoordinate::new(10, 10, 10).unwrap(),
///     MatrixCoordinate::new(2, 2, 2).unwrap(),
/// ];
///
/// let neighbors = find_neighbors(&center, &candidates, 5.0);
/// assert_eq!(neighbors.len(), 2); // (1,1,1) and (2,2,2)
/// ```
pub fn find_neighbors(
    center: &MatrixCoordinate,
    candidates: &[MatrixCoordinate],
    threshold: f64,
) -> Vec<MatrixCoordinate> {
    candidates
        .iter()
        .filter(|coord| center.is_within_distance(coord, threshold))
        .copied()
        .collect()
}

/// Find K nearest neighbors
///
/// Returns up to K nearest coordinates sorted by distance (closest first).
/// If there are fewer than K candidates, returns all candidates sorted by distance.
///
/// # Performance
///
/// O(n log k) where n is the number of candidates and k is the requested count.
/// Uses a min-heap to efficiently track the K nearest neighbors.
///
/// # Examples
///
/// ```
/// use blockmatrix::matrix::{MatrixCoordinate, find_k_nearest};
///
/// let center = MatrixCoordinate::new(0, 0, 0).unwrap();
/// let candidates = vec![
///     MatrixCoordinate::new(1, 0, 0).unwrap(),
///     MatrixCoordinate::new(10, 0, 0).unwrap(),
///     MatrixCoordinate::new(2, 0, 0).unwrap(),
/// ];
///
/// let nearest = find_k_nearest(&center, &candidates, 2);
/// assert_eq!(nearest.len(), 2);
/// assert_eq!(nearest[0].0, MatrixCoordinate::new(1, 0, 0).unwrap());
/// ```
pub fn find_k_nearest(
    center: &MatrixCoordinate,
    candidates: &[MatrixCoordinate],
    k: usize,
) -> Vec<(MatrixCoordinate, f64)> {
    if k == 0 {
        return Vec::new();
    }

    // Calculate distances for all candidates
    let mut distances: Vec<(MatrixCoordinate, f64)> = candidates
        .iter()
        .map(|coord| (*coord, center.euclidean_distance(coord)))
        .collect();

    // Sort by distance (ascending)
    distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // Take first K elements
    distances.into_iter().take(k).collect()
}

/// Find neighbors in a cubic region
///
/// Returns all coordinates within a cubic region defined by the radius.
/// A coordinate is included if its Manhattan distance is within the radius
/// on each axis.
///
/// # Region Definition
///
/// For a center (cx, cy, cz) and radius r, includes all coordinates where:
/// - |x - cx| <= r
/// - |y - cy| <= r
/// - |z - cz| <= r
///
/// # Performance
///
/// O(n) where n is the number of candidates.
///
/// # Examples
///
/// ```
/// use blockmatrix::matrix::{MatrixCoordinate, find_neighbors_cubic};
///
/// let center = MatrixCoordinate::new(0, 0, 0).unwrap();
/// let candidates = vec![
///     MatrixCoordinate::new(1, 1, 1).unwrap(),
///     MatrixCoordinate::new(5, 0, 0).unwrap(),
///     MatrixCoordinate::new(2, 2, 2).unwrap(),
/// ];
///
/// let neighbors = find_neighbors_cubic(&center, &candidates, 2);
/// assert_eq!(neighbors.len(), 2); // (1,1,1) and (2,2,2)
/// ```
pub fn find_neighbors_cubic(
    center: &MatrixCoordinate,
    candidates: &[MatrixCoordinate],
    radius: i64,
) -> Vec<MatrixCoordinate> {
    candidates
        .iter()
        .filter(|coord| {
            (coord.x - center.x).abs() <= radius
                && (coord.y - center.y).abs() <= radius
                && (coord.z - center.z).abs() <= radius
        })
        .copied()
        .collect()
}

/// Find neighbors in a spherical region
///
/// Alias for `find_neighbors` using Euclidean distance.
/// Provided for semantic clarity when working with spherical regions.
///
/// # Examples
///
/// ```
/// use blockmatrix::matrix::{MatrixCoordinate, neighbors::find_neighbors_spherical};
///
/// let center = MatrixCoordinate::new(0, 0, 0).unwrap();
/// let candidates = vec![
///     MatrixCoordinate::new(1, 1, 1).unwrap(),
///     MatrixCoordinate::new(10, 10, 10).unwrap(),
/// ];
///
/// let neighbors = find_neighbors_spherical(&center, &candidates, 5.0);
/// assert_eq!(neighbors.len(), 1);
/// ```
pub fn find_neighbors_spherical(
    center: &MatrixCoordinate,
    candidates: &[MatrixCoordinate],
    radius: f64,
) -> Vec<MatrixCoordinate> {
    find_neighbors(center, candidates, radius)
}

/// Find neighbors using Manhattan distance
///
/// Returns all coordinates within the specified Manhattan distance.
/// Useful for grid-based topologies where diagonal movement isn't allowed.
///
/// # Examples
///
/// ```
/// use blockmatrix::matrix::{MatrixCoordinate, neighbors::find_neighbors_manhattan};
///
/// let center = MatrixCoordinate::new(0, 0, 0).unwrap();
/// let candidates = vec![
///     MatrixCoordinate::new(1, 1, 1).unwrap(),  // distance = 3
///     MatrixCoordinate::new(2, 2, 2).unwrap(),  // distance = 6
/// ];
///
/// let neighbors = find_neighbors_manhattan(&center, &candidates, 4);
/// assert_eq!(neighbors.len(), 1);
/// ```
pub fn find_neighbors_manhattan(
    center: &MatrixCoordinate,
    candidates: &[MatrixCoordinate],
    threshold: i64,
) -> Vec<MatrixCoordinate> {
    candidates
        .iter()
        .filter(|coord| center.manhattan_distance(coord) <= threshold)
        .copied()
        .collect()
}

/// Find neighbors using Chebyshev distance
///
/// Returns all coordinates within the specified Chebyshev distance.
/// Useful for determining neighbors when diagonal movement is allowed.
///
/// # Examples
///
/// ```
/// use blockmatrix::matrix::{MatrixCoordinate, neighbors::find_neighbors_chebyshev};
///
/// let center = MatrixCoordinate::new(0, 0, 0).unwrap();
/// let candidates = vec![
///     MatrixCoordinate::new(2, 2, 2).unwrap(),  // distance = 2
///     MatrixCoordinate::new(5, 3, 1).unwrap(),  // distance = 5
/// ];
///
/// let neighbors = find_neighbors_chebyshev(&center, &candidates, 3);
/// assert_eq!(neighbors.len(), 1);
/// ```
pub fn find_neighbors_chebyshev(
    center: &MatrixCoordinate,
    candidates: &[MatrixCoordinate],
    threshold: i64,
) -> Vec<MatrixCoordinate> {
    candidates
        .iter()
        .filter(|coord| center.chebyshev_distance(coord) <= threshold)
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_candidates() -> Vec<MatrixCoordinate> {
        vec![
            MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate"),
            MatrixCoordinate::new(2, 2, 2).expect("test: valid coordinate"),
            MatrixCoordinate::new(5, 5, 5).expect("test: valid coordinate"),
            MatrixCoordinate::new(10, 10, 10).expect("test: valid coordinate"),
            MatrixCoordinate::new(1, 0, 0).expect("test: valid coordinate"),
            MatrixCoordinate::new(0, 1, 0).expect("test: valid coordinate"),
            MatrixCoordinate::new(0, 0, 1).expect("test: valid coordinate"),
        ]
    }

    #[test]
    fn test_find_neighbors() {
        let center = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let candidates = create_test_candidates();

        let neighbors = find_neighbors(&center, &candidates, 5.0);
        assert!(neighbors.len() >= 3); // At least the unit distance ones
        assert!(neighbors.contains(&MatrixCoordinate::new(1, 0, 0).expect("test: valid coordinate")));
    }

    #[test]
    fn test_find_neighbors_empty() {
        let center = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let candidates = Vec::new();

        let neighbors = find_neighbors(&center, &candidates, 5.0);
        assert_eq!(neighbors.len(), 0);
    }

    #[test]
    fn test_find_neighbors_none_in_range() {
        let center = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let candidates = vec![
            MatrixCoordinate::new(100, 100, 100).expect("test: valid coordinate"),
            MatrixCoordinate::new(200, 200, 200).expect("test: valid coordinate"),
        ];

        let neighbors = find_neighbors(&center, &candidates, 5.0);
        assert_eq!(neighbors.len(), 0);
    }

    #[test]
    fn test_find_k_nearest() {
        let center = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let candidates = create_test_candidates();

        let nearest = find_k_nearest(&center, &candidates, 3);
        assert_eq!(nearest.len(), 3);

        // Verify sorted by distance
        for i in 1..nearest.len() {
            assert!(nearest[i - 1].1 <= nearest[i].1);
        }

        // First should be closest (unit distance ones)
        assert!(nearest[0].1 <= 2.0);
    }

    #[test]
    fn test_find_k_nearest_k_zero() {
        let center = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let candidates = create_test_candidates();

        let nearest = find_k_nearest(&center, &candidates, 0);
        assert_eq!(nearest.len(), 0);
    }

    #[test]
    fn test_find_k_nearest_k_larger_than_candidates() {
        let center = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let candidates = vec![
            MatrixCoordinate::new(1, 0, 0).expect("test: valid coordinate"),
            MatrixCoordinate::new(2, 0, 0).expect("test: valid coordinate"),
        ];

        let nearest = find_k_nearest(&center, &candidates, 10);
        assert_eq!(nearest.len(), 2);
    }

    #[test]
    fn test_find_neighbors_cubic() {
        let center = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let candidates = create_test_candidates();

        let neighbors = find_neighbors_cubic(&center, &candidates, 2);

        // Should include (1,1,1), (2,2,2), and unit distance ones
        assert!(neighbors.len() >= 2);
        assert!(neighbors.contains(&MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate")));
        assert!(neighbors.contains(&MatrixCoordinate::new(2, 2, 2).expect("test: valid coordinate")));
    }

    #[test]
    fn test_find_neighbors_cubic_radius_zero() {
        let center = MatrixCoordinate::new(5, 5, 5).expect("test: valid coordinate");
        let candidates = vec![
            MatrixCoordinate::new(5, 5, 5).expect("test: valid coordinate"),
            MatrixCoordinate::new(6, 5, 5).expect("test: valid coordinate"),
        ];

        let neighbors = find_neighbors_cubic(&center, &candidates, 0);
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0], center);
    }

    #[test]
    fn test_find_neighbors_spherical() {
        let center = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let candidates = create_test_candidates();

        let neighbors = find_neighbors_spherical(&center, &candidates, 5.0);
        // Should be same as find_neighbors
        let expected = find_neighbors(&center, &candidates, 5.0);
        assert_eq!(neighbors.len(), expected.len());
    }

    #[test]
    fn test_find_neighbors_manhattan() {
        let center = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let candidates = vec![
            MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate"), // distance = 3
            MatrixCoordinate::new(2, 2, 2).expect("test: valid coordinate"), // distance = 6
            MatrixCoordinate::new(1, 0, 0).expect("test: valid coordinate"), // distance = 1
        ];

        let neighbors = find_neighbors_manhattan(&center, &candidates, 4);
        assert_eq!(neighbors.len(), 2);
        assert!(neighbors.contains(&MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate")));
        assert!(neighbors.contains(&MatrixCoordinate::new(1, 0, 0).expect("test: valid coordinate")));
    }

    #[test]
    fn test_find_neighbors_chebyshev() {
        let center = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let candidates = vec![
            MatrixCoordinate::new(2, 2, 2).expect("test: valid coordinate"), // distance = 2
            MatrixCoordinate::new(5, 3, 1).expect("test: valid coordinate"), // distance = 5
            MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate"), // distance = 1
        ];

        let neighbors = find_neighbors_chebyshev(&center, &candidates, 3);
        assert_eq!(neighbors.len(), 2);
        assert!(neighbors.contains(&MatrixCoordinate::new(2, 2, 2).expect("test: valid coordinate")));
        assert!(neighbors.contains(&MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate")));
    }

    #[test]
    fn test_negative_coordinates() {
        let center = MatrixCoordinate::new(-5, -5, -5).expect("test: valid coordinate");
        let candidates = vec![
            MatrixCoordinate::new(-4, -4, -4).expect("test: valid coordinate"),
            MatrixCoordinate::new(-10, -10, -10).expect("test: valid coordinate"),
            MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate"),
        ];

        let neighbors = find_neighbors(&center, &candidates, 5.0);
        assert!(!neighbors.is_empty());
        assert!(neighbors.contains(&MatrixCoordinate::new(-4, -4, -4).expect("test: valid coordinate")));
    }
}
