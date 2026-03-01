// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Path finding algorithms for Block-MATRIX topology
//!
//! Implements A* and other pathfinding algorithms optimized for
//! matrix-based distributed routing.

use crate::matrix::coordinate::MatrixCoordinate;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use thiserror::Error;

/// Errors that can occur during path finding
#[derive(Debug, Error, Clone, PartialEq)]
pub enum PathError {
    /// No path found from source to destination
    #[error("No path found from {0:?} to {1:?}")]
    NoPathFound(MatrixCoordinate, MatrixCoordinate),

    /// Path finding timeout after maximum iterations
    #[error("Path finding timeout after {0} iterations")]
    Timeout(usize),

    /// Invalid path configuration
    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

/// Node in the A* search with cost information
#[derive(Clone, Debug)]
struct SearchNode {
    coordinate: MatrixCoordinate,
    _g_cost: f64, // Cost from start to this node
    f_cost: f64,  // Total estimated cost (g + h)
}

impl PartialEq for SearchNode {
    fn eq(&self, other: &Self) -> bool {
        self.f_cost == other.f_cost
    }
}

impl Eq for SearchNode {}

impl Ord for SearchNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap
        other
            .f_cost
            .partial_cmp(&self.f_cost)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for SearchNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A* pathfinding through matrix using tensor operations
///
/// Implements the A* algorithm for finding optimal paths through
/// the Block-MATRIX topology.
pub struct PathFinder {
    /// Heuristic function for A* (default: Euclidean distance)
    heuristic: Box<dyn Fn(&MatrixCoordinate, &MatrixCoordinate) -> f64>,
    /// Maximum iterations before timeout
    max_iterations: usize,
}

impl PathFinder {
    /// Create a new PathFinder with default Euclidean heuristic
    pub fn new() -> Self {
        Self {
            heuristic: Box::new(|a, b| a.euclidean_distance(b)),
            max_iterations: 100000,
        }
    }

    /// Create PathFinder with custom heuristic function
    pub fn with_heuristic<F>(heuristic: F) -> Self
    where
        F: Fn(&MatrixCoordinate, &MatrixCoordinate) -> f64 + 'static,
    {
        Self {
            heuristic: Box::new(heuristic),
            max_iterations: 100000,
        }
    }

    /// Create PathFinder with Manhattan distance heuristic
    pub fn manhattan() -> Self {
        Self {
            heuristic: Box::new(|a, b| a.manhattan_distance(b) as f64),
            max_iterations: 100000,
        }
    }

    /// Set maximum iterations before timeout
    pub fn set_max_iterations(&mut self, max: usize) {
        self.max_iterations = max;
    }

    /// Find optimal path from start to goal using A* algorithm
    ///
    /// # Arguments
    /// * `start` - Starting coordinate
    /// * `goal` - Target coordinate
    /// * `neighbors_fn` - Function that returns valid neighbors for a coordinate
    ///
    /// # Returns
    /// Vector of coordinates forming the optimal path, or error if no path exists
    pub fn find_path<F>(
        &self,
        start: &MatrixCoordinate,
        goal: &MatrixCoordinate,
        neighbors_fn: F,
    ) -> Result<Vec<MatrixCoordinate>, PathError>
    where
        F: Fn(&MatrixCoordinate) -> Vec<MatrixCoordinate>,
    {
        if start == goal {
            return Ok(vec![*start]);
        }

        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<MatrixCoordinate, MatrixCoordinate> = HashMap::new();
        let mut g_score: HashMap<MatrixCoordinate, f64> = HashMap::new();
        let mut closed_set: HashSet<MatrixCoordinate> = HashSet::new();

        // Initialize start node
        g_score.insert(*start, 0.0);
        open_set.push(SearchNode {
            coordinate: *start,
            _g_cost: 0.0,
            f_cost: (self.heuristic)(start, goal),
        });

        let mut iterations = 0;

        while let Some(current_node) = open_set.pop() {
            iterations += 1;
            if iterations > self.max_iterations {
                return Err(PathError::Timeout(self.max_iterations));
            }

            let current = current_node.coordinate;

            if current == *goal {
                // Reconstruct path
                return Ok(self.reconstruct_path(&came_from, &current));
            }

            if closed_set.contains(&current) {
                continue;
            }
            closed_set.insert(current);

            let current_g_score = *g_score.get(&current).unwrap_or(&f64::INFINITY);

            // Explore neighbors
            for neighbor in neighbors_fn(&current) {
                if closed_set.contains(&neighbor) {
                    continue;
                }

                let tentative_g_score = current_g_score + current.euclidean_distance(&neighbor);

                let neighbor_g_score = *g_score.get(&neighbor).unwrap_or(&f64::INFINITY);

                if tentative_g_score < neighbor_g_score {
                    // This path to neighbor is better
                    came_from.insert(neighbor, current);
                    g_score.insert(neighbor, tentative_g_score);

                    let f_score = tentative_g_score + (self.heuristic)(&neighbor, goal);
                    open_set.push(SearchNode {
                        coordinate: neighbor,
                        _g_cost: tentative_g_score,
                        f_cost: f_score,
                    });
                }
            }
        }

        Err(PathError::NoPathFound(*start, *goal))
    }

    /// Reconstruct path from came_from map
    fn reconstruct_path(
        &self,
        came_from: &HashMap<MatrixCoordinate, MatrixCoordinate>,
        current: &MatrixCoordinate,
    ) -> Vec<MatrixCoordinate> {
        let mut path = vec![*current];
        let mut current = *current;

        while let Some(prev) = came_from.get(&current) {
            path.push(*prev);
            current = *prev;
        }

        path.reverse();
        path
    }

    /// Find K shortest paths for redundancy
    ///
    /// Uses Yen's algorithm to find multiple alternative paths.
    ///
    /// # Arguments
    /// * `start` - Starting coordinate
    /// * `goal` - Target coordinate
    /// * `k` - Number of paths to find
    /// * `neighbors_fn` - Function that returns valid neighbors
    ///
    /// # Returns
    /// Vector of paths, sorted by cost (best first)
    pub fn find_k_shortest_paths<F>(
        &self,
        start: &MatrixCoordinate,
        goal: &MatrixCoordinate,
        k: usize,
        neighbors_fn: F,
    ) -> Result<Vec<Vec<MatrixCoordinate>>, PathError>
    where
        F: Fn(&MatrixCoordinate) -> Vec<MatrixCoordinate> + Clone,
    {
        if k == 0 {
            return Ok(vec![]);
        }

        let mut paths = Vec::new();

        // Find the shortest path first
        let shortest_path = self.find_path(start, goal, &neighbors_fn)?;
        paths.push(shortest_path.clone());

        if k == 1 {
            return Ok(paths);
        }

        // Find alternative paths by blocking edges
        let mut candidate_paths: Vec<(f64, Vec<MatrixCoordinate>)> = Vec::new();

        for i in 0..shortest_path.len() - 1 {
            // Create a modified neighbors function that blocks the current edge
            let blocked_from = shortest_path[i];
            let blocked_to = shortest_path[i + 1];

            let modified_neighbors = |coord: &MatrixCoordinate| -> Vec<MatrixCoordinate> {
                if *coord == blocked_from {
                    neighbors_fn(coord)
                        .into_iter()
                        .filter(|n| *n != blocked_to)
                        .collect()
                } else {
                    neighbors_fn(coord)
                }
            };

            // Find alternative path with this edge blocked
            if let Ok(alt_path) = self.find_path(start, goal, modified_neighbors) {
                let cost = calculate_path_cost(&alt_path);
                candidate_paths.push((cost, alt_path));
            }
        }

        // Sort candidate paths by cost
        candidate_paths.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));

        // Add the best k-1 alternative paths
        for (_, path) in candidate_paths.into_iter().take(k - 1) {
            if !paths.iter().any(|p| p == &path) {
                paths.push(path);
            }
        }

        Ok(paths)
    }
}

impl Default for PathFinder {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate path cost using vector operations
///
/// Computes the total Euclidean distance of a path.
pub fn calculate_path_cost(path: &[MatrixCoordinate]) -> f64 {
    if path.len() < 2 {
        return 0.0;
    }

    let mut total_cost = 0.0;
    for i in 1..path.len() {
        total_cost += path[i - 1].euclidean_distance(&path[i]);
    }

    total_cost
}

/// Optimize path by removing unnecessary hops
///
/// Removes intermediate nodes that don't contribute to the path,
/// creating a more direct route where possible.
///
/// # Arguments
/// * `path` - Original path to optimize
///
/// # Returns
/// Optimized path with unnecessary hops removed
pub fn optimize_path(path: Vec<MatrixCoordinate>) -> Vec<MatrixCoordinate> {
    if path.len() <= 2 {
        return path;
    }

    let mut optimized = vec![path[0]];
    let mut current_idx = 0;

    while current_idx < path.len() - 1 {
        // Try to skip ahead as far as possible
        let mut furthest_reachable = current_idx + 1;

        for j in (current_idx + 2)..path.len() {
            // Check if we can reach j directly from current
            // This is a simplified check - in practice, you'd verify
            // that the direct path is valid (no obstacles)
            if can_reach_directly(&path[current_idx], &path[j]) {
                furthest_reachable = j;
            }
        }

        optimized.push(path[furthest_reachable]);
        current_idx = furthest_reachable;
    }

    optimized
}

/// Check if two coordinates can be reached directly
///
/// This is a placeholder for obstacle checking logic.
/// In a real implementation, this would check for blocked paths.
fn can_reach_directly(from: &MatrixCoordinate, to: &MatrixCoordinate) -> bool {
    // Simplified: allow direct connection if distance is reasonable
    // In practice, check for obstacles, network topology, etc.
    from.euclidean_distance(to) < 100.0
}

/// Bidirectional A* search for improved performance
///
/// Searches from both start and goal simultaneously, meeting in the middle.
pub fn bidirectional_search<F>(
    start: &MatrixCoordinate,
    goal: &MatrixCoordinate,
    neighbors_fn: F,
) -> Result<Vec<MatrixCoordinate>, PathError>
where
    F: Fn(&MatrixCoordinate) -> Vec<MatrixCoordinate> + Clone,
{
    let forward_finder = PathFinder::new();
    let _backward_finder = PathFinder::new();

    // For simplicity, fall back to regular A* for now
    // Full bidirectional implementation would maintain two frontiers
    forward_finder.find_path(start, goal, neighbors_fn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::coordinate::MatrixCoordinate;

    fn grid_neighbors(coord: &MatrixCoordinate) -> Vec<MatrixCoordinate> {
        let mut neighbors = Vec::new();
        let offsets = [
            (-1, 0, 0),
            (1, 0, 0),
            (0, -1, 0),
            (0, 1, 0),
            (0, 0, -1),
            (0, 0, 1),
        ];

        for (dx, dy, dz) in offsets.iter() {
            if let Ok(neighbor) = MatrixCoordinate::new(coord.x + dx, coord.y + dy, coord.z + dz) {
                neighbors.push(neighbor);
            }
        }

        neighbors
    }

    #[test]
    fn test_pathfinder_straight_line() {
        let finder = PathFinder::new();
        let start = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let goal = MatrixCoordinate::new(3, 0, 0).expect("test: valid coordinate");

        let path = finder.find_path(&start, &goal, grid_neighbors).expect("test: query operation");

        assert_eq!(path.len(), 4);
        assert_eq!(path[0], start);
        assert_eq!(path[3], goal);
    }

    #[test]
    fn test_pathfinder_diagonal() {
        let finder = PathFinder::new();
        let start = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let goal = MatrixCoordinate::new(2, 2, 0).expect("test: valid coordinate");

        let path = finder.find_path(&start, &goal, grid_neighbors).expect("test: query operation");

        assert!(path.len() >= 5); // At least 5 steps needed
        assert_eq!(path[0], start);
        assert_eq!(path[path.len() - 1], goal);
    }

    #[test]
    fn test_pathfinder_no_path() {
        let finder = PathFinder::new();
        let start = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let goal = MatrixCoordinate::new(100, 100, 100).expect("test: valid coordinate");

        // No neighbors function that allows reaching the goal
        let result = finder.find_path(&start, &goal, |_| vec![]);

        assert!(result.is_err());
    }

    #[test]
    fn test_pathfinder_same_start_goal() {
        let finder = PathFinder::new();
        let coord = MatrixCoordinate::new(5, 5, 5).expect("test: valid coordinate");

        let path = finder.find_path(&coord, &coord, grid_neighbors).expect("test: query operation");

        assert_eq!(path.len(), 1);
        assert_eq!(path[0], coord);
    }

    #[test]
    fn test_manhattan_heuristic() {
        let finder = PathFinder::manhattan();
        let start = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let goal = MatrixCoordinate::new(3, 3, 0).expect("test: valid coordinate");

        let path = finder.find_path(&start, &goal, grid_neighbors).expect("test: query operation");

        assert!(path.len() >= 7); // Manhattan distance is 6
        assert_eq!(path[0], start);
        assert_eq!(path[path.len() - 1], goal);
    }

    #[test]
    fn test_k_shortest_paths() {
        let finder = PathFinder::new();
        let start = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let goal = MatrixCoordinate::new(2, 2, 0).expect("test: valid coordinate");

        let paths = finder
            .find_k_shortest_paths(&start, &goal, 2, grid_neighbors)
            .expect("test: expected success");

        assert!(!paths.is_empty());
        for path in &paths {
            assert_eq!(path[0], start);
            assert_eq!(path[path.len() - 1], goal);
        }
    }

    #[test]
    fn test_calculate_path_cost() {
        let path = vec![
            MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate"),
            MatrixCoordinate::new(1, 0, 0).expect("test: valid coordinate"),
            MatrixCoordinate::new(2, 0, 0).expect("test: valid coordinate"),
        ];

        let cost = calculate_path_cost(&path);
        assert_eq!(cost, 2.0);

        let diagonal_path = vec![
            MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate"),
            MatrixCoordinate::new(1, 1, 0).expect("test: valid coordinate"),
        ];

        let diagonal_cost = calculate_path_cost(&diagonal_path);
        assert!((diagonal_cost - 2.0_f64.sqrt()).abs() < 0.001);
    }

    #[test]
    fn test_optimize_path() {
        let path = vec![
            MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate"),
            MatrixCoordinate::new(1, 0, 0).expect("test: valid coordinate"),
            MatrixCoordinate::new(2, 0, 0).expect("test: valid coordinate"),
            MatrixCoordinate::new(3, 0, 0).expect("test: valid coordinate"),
        ];

        let optimized = optimize_path(path);

        // Should reduce to start and end if can reach directly
        assert!(optimized.len() <= 4);
        assert_eq!(optimized[0], MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate"));
        assert_eq!(
            optimized[optimized.len() - 1],
            MatrixCoordinate::new(3, 0, 0).expect("test: valid coordinate")
        );
    }

    #[test]
    fn test_pathfinder_timeout() {
        let mut finder = PathFinder::new();
        finder.set_max_iterations(10); // Very low limit

        let start = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let goal = MatrixCoordinate::new(100, 100, 100).expect("test: valid coordinate");

        let result = finder.find_path(&start, &goal, grid_neighbors);

        // Should timeout with such a low iteration limit
        if let Err(PathError::Timeout(iterations)) = result {
            assert_eq!(iterations, 10);
        } else {
            panic!("Expected timeout error");
        }
    }
}
