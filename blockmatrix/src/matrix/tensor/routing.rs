// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Routing-specific tensor algorithms for Block-MATRIX
//!
//! Provides intelligent routing calculations based on matrix topology,
//! including direction scoring, path optimization, and load balancing.

use super::vector::Vector3D;
use crate::matrix::coordinate::MatrixCoordinate;

/// Calculate optimal routing direction from source to destination
///
/// Returns a normalized vector pointing from source to destination,
/// representing the optimal routing direction in matrix space.
///
/// # Example
/// ```
/// use blockmatrix::matrix::tensor::calculate_routing_vector;
/// use blockmatrix::matrix::MatrixCoordinate;
///
/// let source = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
/// let dest = MatrixCoordinate::new(100, 50, 25).expect("test: valid coord");
/// let direction = calculate_routing_vector(&source, &dest);
/// ```
pub fn calculate_routing_vector(
    source: &MatrixCoordinate,
    destination: &MatrixCoordinate,
) -> Vector3D {
    let vec = Vector3D::from_coordinates(source, destination);
    vec.normalize().unwrap_or_default()
}

/// Find intermediate hops for multi-hop routing
///
/// Calculates a series of intermediate nodes along the path from source
/// to destination, ensuring no single hop exceeds max_hop_distance.
///
/// # Arguments
/// * `source` - Starting coordinate
/// * `destination` - Target coordinate
/// * `max_hop_distance` - Maximum allowed distance for a single hop
///
/// # Returns
/// Vector of intermediate coordinates including source and destination
pub fn calculate_routing_path(
    source: &MatrixCoordinate,
    destination: &MatrixCoordinate,
    max_hop_distance: f64,
) -> Vec<MatrixCoordinate> {
    if max_hop_distance <= 0.0 {
        return vec![*source, *destination];
    }

    let total_distance = source.euclidean_distance(destination);
    if total_distance <= max_hop_distance {
        return vec![*source, *destination];
    }

    // Calculate number of hops needed
    let num_hops = (total_distance / max_hop_distance).ceil() as usize;
    let mut path = Vec::with_capacity(num_hops + 1);
    path.push(*source);

    // Generate intermediate points
    for i in 1..num_hops {
        let t = i as f64 / num_hops as f64;

        // Linear interpolation in coordinate space
        let x = source.x as f64 + t * (destination.x - source.x) as f64;
        let y = source.y as f64 + t * (destination.y - source.y) as f64;
        let z = source.z as f64 + t * (destination.z - source.z) as f64;

        // Round to nearest integer coordinates
        if let Ok(coord) =
            MatrixCoordinate::new(x.round() as i64, y.round() as i64, z.round() as i64)
        {
            path.push(coord);
        }
    }

    path.push(*destination);
    path
}

/// Score similarity between two routing directions (0.0 to 1.0)
///
/// Used to find nodes routing in similar directions. Returns 1.0 for
/// identical directions, 0.0 for perpendicular, and negative for opposite.
///
/// # Arguments
/// * `direction1` - First routing direction (should be normalized)
/// * `direction2` - Second routing direction (should be normalized)
///
/// # Returns
/// Similarity score in range [-1.0, 1.0], where:
/// - 1.0 = same direction
/// - 0.0 = perpendicular
/// - -1.0 = opposite direction
pub fn routing_similarity(direction1: &Vector3D, direction2: &Vector3D) -> f64 {
    // Normalize vectors to ensure accurate comparison
    let d1 = direction1.normalize().unwrap_or_default();
    let d2 = direction2.normalize().unwrap_or_default();

    // Dot product of unit vectors gives cosine of angle
    d1.dot(&d2)
}

/// Find nodes in matrix that are aligned with routing direction
///
/// Identifies nodes that are positioned along a similar routing path,
/// useful for finding relay nodes or alternative routes.
///
/// # Arguments
/// * `source` - Starting coordinate
/// * `target_direction` - Desired routing direction (will be normalized)
/// * `candidates` - List of potential relay nodes
/// * `alignment_threshold` - Minimum similarity score (0.0 to 1.0)
///
/// # Returns
/// Vector of aligned nodes sorted by alignment score (best first)
pub fn find_aligned_nodes(
    source: &MatrixCoordinate,
    target_direction: &Vector3D,
    candidates: &[MatrixCoordinate],
    alignment_threshold: f64,
) -> Vec<MatrixCoordinate> {
    let normalized_target = target_direction.normalize().unwrap_or_default();

    let mut aligned_nodes: Vec<(MatrixCoordinate, f64)> = candidates
        .iter()
        .filter_map(|candidate| {
            // Skip if candidate is the source itself
            if candidate == source {
                return None;
            }

            // Calculate direction to candidate
            let direction = calculate_routing_vector(source, candidate);

            // Calculate alignment score
            let similarity = routing_similarity(&direction, &normalized_target);

            // Only include if above threshold
            if similarity >= alignment_threshold {
                Some((*candidate, similarity))
            } else {
                None
            }
        })
        .collect();

    // Sort by alignment score (best first)
    aligned_nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Return only the coordinates
    aligned_nodes.into_iter().map(|(coord, _)| coord).collect()
}

/// Calculate orthogonal routing options for load balancing
///
/// Given a primary routing direction, returns two orthogonal directions
/// that can be used for alternative routing paths or load distribution.
///
/// # Arguments
/// * `primary_direction` - Main routing direction (will be normalized)
///
/// # Returns
/// Vector containing two orthogonal directions to the primary direction
pub fn calculate_orthogonal_routes(primary_direction: &Vector3D) -> Vec<Vector3D> {
    let normalized = primary_direction
        .normalize()
        .unwrap_or(Vector3D::new(1.0, 0.0, 0.0));

    // Find a vector not parallel to the primary direction
    let arbitrary = if normalized.x.abs() < 0.9 {
        Vector3D::new(1.0, 0.0, 0.0)
    } else {
        Vector3D::new(0.0, 1.0, 0.0)
    };

    // First orthogonal vector using cross product
    let ortho1 = normalized.cross(&arbitrary).normalize().unwrap_or_default();

    // Second orthogonal vector, perpendicular to both primary and first orthogonal
    let ortho2 = normalized.cross(&ortho1).normalize().unwrap_or_default();

    vec![ortho1, ortho2]
}

/// Calculate load-balanced routing options
///
/// Given a primary route and current load information, suggests alternative
/// routes that balance load across the matrix topology.
///
/// # Arguments
/// * `source` - Starting coordinate
/// * `destination` - Target coordinate
/// * `num_alternatives` - Number of alternative routes to generate
/// * `spread_factor` - How much to spread routes (0.0 = direct, 1.0 = maximum spread)
///
/// # Returns
/// Vector of alternative routing directions
pub fn calculate_load_balanced_routes(
    source: &MatrixCoordinate,
    destination: &MatrixCoordinate,
    num_alternatives: usize,
    spread_factor: f64,
) -> Vec<Vector3D> {
    let primary = calculate_routing_vector(source, destination);

    if num_alternatives == 0 {
        return vec![primary];
    }

    let mut routes = vec![primary];
    let orthogonals = calculate_orthogonal_routes(&primary);

    if orthogonals.len() < 2 {
        return routes;
    }

    let spread = spread_factor.clamp(0.0, 1.0);

    // Generate alternative routes by combining primary with orthogonal vectors
    for i in 1..=num_alternatives {
        let angle = 2.0 * std::f64::consts::PI * (i as f64) / (num_alternatives as f64);

        // Combine primary direction with orthogonal components
        let deviation = orthogonals[0]
            .scale(angle.cos() * spread)
            .add(&orthogonals[1].scale(angle.sin() * spread));

        let alternative = primary.scale(1.0 - spread * 0.3).add(&deviation);

        if let Ok(normalized) = alternative.normalize() {
            routes.push(normalized);
        }
    }

    routes
}

/// Score route quality based on multiple factors
///
/// Evaluates a routing path considering distance, hop count, and alignment.
///
/// # Arguments
/// * `path` - Sequence of coordinates forming the route
/// * `ideal_hop_distance` - Preferred distance between hops
///
/// # Returns
/// Quality score (higher is better)
pub fn score_route_quality(path: &[MatrixCoordinate], ideal_hop_distance: f64) -> f64 {
    if path.len() < 2 {
        return 0.0;
    }

    let mut total_score = 100.0;
    let mut total_distance = 0.0;
    let mut direction_changes = 0.0;
    let mut prev_direction = None;

    // Analyze each hop in the path
    for i in 1..path.len() {
        let hop_distance = path[i - 1].euclidean_distance(&path[i]);
        total_distance += hop_distance;

        // Penalize deviation from ideal hop distance
        let distance_penalty =
            ((hop_distance - ideal_hop_distance).abs() / ideal_hop_distance).min(1.0);
        total_score -= distance_penalty * 10.0;

        // Check for direction changes (penalize zigzagging)
        if i < path.len() - 1 {
            let current_direction = calculate_routing_vector(&path[i - 1], &path[i]);

            if let Some(prev_dir) = prev_direction {
                let similarity = routing_similarity(&prev_dir, &current_direction);
                direction_changes += (1.0 - similarity).max(0.0);
            }

            prev_direction = Some(current_direction);
        }
    }

    // Penalize excessive direction changes
    total_score -= direction_changes * 15.0;

    // Penalize excessive total distance compared to direct route
    if !path.is_empty() {
        let direct_distance = path[0].euclidean_distance(&path[path.len() - 1]);
        if direct_distance > 0.0 {
            let efficiency = direct_distance / total_distance;
            total_score *= efficiency;
        }
    }

    total_score.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::coordinate::MatrixCoordinate;

    #[test]
    fn test_calculate_routing_vector() {
        let source = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let dest = MatrixCoordinate::new(10, 0, 0).expect("test: valid coordinate");
        let direction = calculate_routing_vector(&source, &dest);

        assert!((direction.x - 1.0).abs() < 0.001);
        assert!((direction.y - 0.0).abs() < 0.001);
        assert!((direction.z - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_routing_path() {
        let source = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let dest = MatrixCoordinate::new(100, 0, 0).expect("test: valid coordinate");

        let path = calculate_routing_path(&source, &dest, 30.0);

        // Should have multiple hops
        assert!(path.len() > 2);
        assert_eq!(path[0], source);
        assert_eq!(path[path.len() - 1], dest);

        // Check that no hop exceeds max distance
        for i in 1..path.len() {
            let hop_distance = path[i - 1].euclidean_distance(&path[i]);
            assert!(hop_distance <= 35.0); // Some tolerance for rounding
        }
    }

    #[test]
    fn test_routing_similarity() {
        let dir1 = Vector3D::new(1.0, 0.0, 0.0);
        let dir2 = Vector3D::new(1.0, 0.0, 0.0);
        assert!((routing_similarity(&dir1, &dir2) - 1.0).abs() < 0.001);

        let dir3 = Vector3D::new(0.0, 1.0, 0.0);
        assert!((routing_similarity(&dir1, &dir3) - 0.0).abs() < 0.001);

        let dir4 = Vector3D::new(-1.0, 0.0, 0.0);
        assert!((routing_similarity(&dir1, &dir4) - -1.0).abs() < 0.001);
    }

    #[test]
    fn test_find_aligned_nodes() {
        let source = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let target_direction = Vector3D::new(1.0, 0.0, 0.0);

        let candidates = vec![
            MatrixCoordinate::new(10, 0, 0).expect("test: valid coordinate"), // Perfectly aligned
            MatrixCoordinate::new(10, 1, 0).expect("test: valid coordinate"), // Slightly off
            MatrixCoordinate::new(0, 10, 0).expect("test: valid coordinate"), // Perpendicular
            MatrixCoordinate::new(-10, 0, 0).expect("test: valid coordinate"), // Opposite
        ];

        let aligned = find_aligned_nodes(&source, &target_direction, &candidates, 0.9);

        // Both (10,0,0) and (10,1,0) are aligned: similarity ~1.0 and ~0.995 respectively
        assert_eq!(aligned.len(), 2);
        assert_eq!(aligned[0], MatrixCoordinate::new(10, 0, 0).expect("test: valid coordinate"));
        assert_eq!(aligned[1], MatrixCoordinate::new(10, 1, 0).expect("test: valid coordinate"));

        let more_aligned = find_aligned_nodes(&source, &target_direction, &candidates, 0.5);
        assert_eq!(more_aligned.len(), 2);
    }

    #[test]
    fn test_calculate_orthogonal_routes() {
        let primary = Vector3D::new(1.0, 0.0, 0.0);
        let orthogonals = calculate_orthogonal_routes(&primary);

        assert_eq!(orthogonals.len(), 2);

        // Check orthogonality
        assert!((primary.dot(&orthogonals[0])).abs() < 0.001);
        assert!((primary.dot(&orthogonals[1])).abs() < 0.001);
        assert!((orthogonals[0].dot(&orthogonals[1])).abs() < 0.001);
    }

    #[test]
    fn test_load_balanced_routes() {
        let source = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let dest = MatrixCoordinate::new(100, 0, 0).expect("test: valid coordinate");

        let routes = calculate_load_balanced_routes(&source, &dest, 3, 0.5);

        assert_eq!(routes.len(), 4); // Primary + 3 alternatives

        // All should be unit vectors
        for route in &routes {
            assert!((route.magnitude() - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_score_route_quality() {
        // Direct path should score high
        let path1 = vec![
            MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate"),
            MatrixCoordinate::new(10, 0, 0).expect("test: valid coordinate"),
            MatrixCoordinate::new(20, 0, 0).expect("test: valid coordinate"),
        ];
        let score1 = score_route_quality(&path1, 10.0);

        // Zigzag path should score lower
        let path2 = vec![
            MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate"),
            MatrixCoordinate::new(10, 10, 0).expect("test: valid coordinate"),
            MatrixCoordinate::new(20, 0, 0).expect("test: valid coordinate"),
        ];
        let score2 = score_route_quality(&path2, 10.0);

        assert!(score1 > score2);
    }
}
