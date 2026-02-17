// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Comprehensive tests for path finding algorithms

use crate::matrix::tensor::path_finding::*;
use crate::matrix::coordinate::MatrixCoordinate;

fn grid_neighbors(coord: &MatrixCoordinate) -> Vec<MatrixCoordinate> {
    let mut neighbors = Vec::new();
    let offsets = [
        (-1, 0, 0), (1, 0, 0),
        (0, -1, 0), (0, 1, 0),
        (0, 0, -1), (0, 0, 1),
    ];

    for (dx, dy, dz) in offsets.iter() {
        if let Ok(neighbor) = MatrixCoordinate::new(
            coord.x + dx,
            coord.y + dy,
            coord.z + dz,
        ) {
            neighbors.push(neighbor);
        }
    }

    neighbors
}

fn diagonal_neighbors(coord: &MatrixCoordinate) -> Vec<MatrixCoordinate> {
    let mut neighbors = Vec::new();

    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                if dx == 0 && dy == 0 && dz == 0 {
                    continue;
                }

                if let Ok(neighbor) = MatrixCoordinate::new(
                    coord.x + dx,
                    coord.y + dy,
                    coord.z + dz,
                ) {
                    neighbors.push(neighbor);
                }
            }
        }
    }

    neighbors
}

#[test]
fn test_pathfinder_straight_line() {
    let finder = PathFinder::new();
    let start = MatrixCoordinate::new(0, 0, 0).unwrap();
    let goal = MatrixCoordinate::new(5, 0, 0).unwrap();

    let path = finder.find_path(&start, &goal, grid_neighbors).unwrap();

    assert_eq!(path.len(), 6);
    assert_eq!(path[0], start);
    assert_eq!(path[5], goal);

    // Check path is straight
    for i in 0..path.len() {
        assert_eq!(path[i].y, 0);
        assert_eq!(path[i].z, 0);
        assert_eq!(path[i].x, i as i64);
    }
}

#[test]
fn test_pathfinder_diagonal_movement() {
    let finder = PathFinder::new();
    let start = MatrixCoordinate::new(0, 0, 0).unwrap();
    let goal = MatrixCoordinate::new(3, 3, 0).unwrap();

    let path = finder.find_path(&start, &goal, grid_neighbors).unwrap();

    assert!(path.len() >= 7); // Minimum Manhattan distance
    assert_eq!(path[0], start);
    assert_eq!(path[path.len() - 1], goal);
}

#[test]
fn test_pathfinder_3d_movement() {
    let finder = PathFinder::new();
    let start = MatrixCoordinate::new(0, 0, 0).unwrap();
    let goal = MatrixCoordinate::new(2, 2, 2).unwrap();

    let path = finder.find_path(&start, &goal, grid_neighbors).unwrap();

    assert!(path.len() >= 7); // Minimum Manhattan distance
    assert_eq!(path[0], start);
    assert_eq!(path[path.len() - 1], goal);
}

#[test]
fn test_pathfinder_no_path() {
    let finder = PathFinder::new();
    let start = MatrixCoordinate::new(0, 0, 0).unwrap();
    let goal = MatrixCoordinate::new(100, 100, 100).unwrap();

    // No neighbors = no path possible
    let result = finder.find_path(&start, &goal, |_| vec![]);

    match result {
        Err(PathError::NoPathFound(s, g)) => {
            assert_eq!(s, start);
            assert_eq!(g, goal);
        }
        _ => panic!("Expected NoPathFound error"),
    }
}

#[test]
fn test_pathfinder_same_start_and_goal() {
    let finder = PathFinder::new();
    let coord = MatrixCoordinate::new(5, 5, 5).unwrap();

    let path = finder.find_path(&coord, &coord, grid_neighbors).unwrap();

    assert_eq!(path.len(), 1);
    assert_eq!(path[0], coord);
}

#[test]
fn test_pathfinder_manhattan_heuristic() {
    let finder = PathFinder::manhattan();
    let start = MatrixCoordinate::new(0, 0, 0).unwrap();
    let goal = MatrixCoordinate::new(4, 4, 0).unwrap();

    let path = finder.find_path(&start, &goal, grid_neighbors).unwrap();

    assert!(path.len() >= 9); // Manhattan distance is 8
    assert_eq!(path[0], start);
    assert_eq!(path[path.len() - 1], goal);
}

#[test]
fn test_pathfinder_custom_heuristic() {
    // Use Chebyshev distance as heuristic
    let finder = PathFinder::with_heuristic(|a, b| a.chebyshev_distance(b) as f64);

    let start = MatrixCoordinate::new(0, 0, 0).unwrap();
    let goal = MatrixCoordinate::new(3, 3, 3).unwrap();

    let path = finder.find_path(&start, &goal, diagonal_neighbors).unwrap();

    assert!(path.len() >= 4); // Chebyshev distance is 3
    assert_eq!(path[0], start);
    assert_eq!(path[path.len() - 1], goal);
}

#[test]
fn test_pathfinder_timeout() {
    let mut finder = PathFinder::new();
    finder.set_max_iterations(5); // Very low limit

    let start = MatrixCoordinate::new(0, 0, 0).unwrap();
    let goal = MatrixCoordinate::new(100, 100, 100).unwrap();

    let result = finder.find_path(&start, &goal, grid_neighbors);

    match result {
        Err(PathError::Timeout(iterations)) => {
            assert_eq!(iterations, 5);
        }
        _ => panic!("Expected Timeout error"),
    }
}

#[test]
fn test_k_shortest_paths_single() {
    let finder = PathFinder::new();
    let start = MatrixCoordinate::new(0, 0, 0).unwrap();
    let goal = MatrixCoordinate::new(3, 0, 0).unwrap();

    let paths = finder.find_k_shortest_paths(&start, &goal, 1, grid_neighbors).unwrap();

    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0][0], start);
    assert_eq!(paths[0][paths[0].len() - 1], goal);
}

#[test]
fn test_k_shortest_paths_multiple() {
    let finder = PathFinder::new();
    let start = MatrixCoordinate::new(0, 0, 0).unwrap();
    let goal = MatrixCoordinate::new(2, 2, 0).unwrap();

    let paths = finder.find_k_shortest_paths(&start, &goal, 3, grid_neighbors).unwrap();

    assert!(!paths.is_empty());

    // All paths should start at start and end at goal
    for path in &paths {
        assert_eq!(path[0], start);
        assert_eq!(path[path.len() - 1], goal);
    }

    // Paths should be different
    if paths.len() > 1 {
        assert_ne!(paths[0], paths[1]);
    }
}

#[test]
fn test_k_shortest_paths_zero_k() {
    let finder = PathFinder::new();
    let start = MatrixCoordinate::new(0, 0, 0).unwrap();
    let goal = MatrixCoordinate::new(3, 0, 0).unwrap();

    let paths = finder.find_k_shortest_paths(&start, &goal, 0, grid_neighbors).unwrap();

    assert_eq!(paths.len(), 0);
}

#[test]
fn test_calculate_path_cost_straight() {
    let path = vec![
        MatrixCoordinate::new(0, 0, 0).unwrap(),
        MatrixCoordinate::new(1, 0, 0).unwrap(),
        MatrixCoordinate::new(2, 0, 0).unwrap(),
        MatrixCoordinate::new(3, 0, 0).unwrap(),
    ];

    let cost = calculate_path_cost(&path);
    assert_eq!(cost, 3.0);
}

#[test]
fn test_calculate_path_cost_diagonal() {
    let path = vec![
        MatrixCoordinate::new(0, 0, 0).unwrap(),
        MatrixCoordinate::new(1, 1, 0).unwrap(),
        MatrixCoordinate::new(2, 2, 0).unwrap(),
    ];

    let cost = calculate_path_cost(&path);
    let expected = 2.0 * 2.0_f64.sqrt();
    assert!((cost - expected).abs() < 0.001);
}

#[test]
fn test_calculate_path_cost_empty() {
    let path = vec![];
    let cost = calculate_path_cost(&path);
    assert_eq!(cost, 0.0);
}

#[test]
fn test_calculate_path_cost_single() {
    let path = vec![MatrixCoordinate::new(5, 5, 5).unwrap()];
    let cost = calculate_path_cost(&path);
    assert_eq!(cost, 0.0);
}

#[test]
fn test_optimize_path_straight() {
    let path = vec![
        MatrixCoordinate::new(0, 0, 0).unwrap(),
        MatrixCoordinate::new(1, 0, 0).unwrap(),
        MatrixCoordinate::new(2, 0, 0).unwrap(),
        MatrixCoordinate::new(3, 0, 0).unwrap(),
    ];

    let optimized = optimize_path(path.clone());

    // Should optimize to just start and end
    assert!(optimized.len() <= path.len());
    assert_eq!(optimized[0], MatrixCoordinate::new(0, 0, 0).unwrap());
    assert_eq!(
        optimized[optimized.len() - 1],
        MatrixCoordinate::new(3, 0, 0).unwrap()
    );
}

#[test]
fn test_optimize_path_empty() {
    let path = vec![];
    let optimized = optimize_path(path);
    assert_eq!(optimized.len(), 0);
}

#[test]
fn test_optimize_path_two_nodes() {
    let path = vec![
        MatrixCoordinate::new(0, 0, 0).unwrap(),
        MatrixCoordinate::new(10, 0, 0).unwrap(),
    ];

    let optimized = optimize_path(path.clone());
    assert_eq!(optimized, path);
}

#[test]
fn test_bidirectional_search() {
    let start = MatrixCoordinate::new(0, 0, 0).unwrap();
    let goal = MatrixCoordinate::new(5, 0, 0).unwrap();

    let path = bidirectional_search(&start, &goal, grid_neighbors).unwrap();

    assert_eq!(path[0], start);
    assert_eq!(path[path.len() - 1], goal);
}

// Stress tests

#[test]
fn test_pathfinder_large_grid() {
    let finder = PathFinder::new();
    let start = MatrixCoordinate::new(0, 0, 0).unwrap();
    let goal = MatrixCoordinate::new(20, 20, 0).unwrap();

    let path = finder.find_path(&start, &goal, grid_neighbors).unwrap();

    assert!(path.len() >= 41); // Minimum Manhattan distance
    assert_eq!(path[0], start);
    assert_eq!(path[path.len() - 1], goal);
}

#[test]
fn test_pathfinder_negative_coordinates() {
    let finder = PathFinder::new();
    let start = MatrixCoordinate::new(-5, -5, -5).unwrap();
    let goal = MatrixCoordinate::new(5, 5, 5).unwrap();

    let path = finder.find_path(&start, &goal, grid_neighbors).unwrap();

    assert!(path.len() >= 31); // Manhattan distance is 30
    assert_eq!(path[0], start);
    assert_eq!(path[path.len() - 1], goal);
}

#[test]
fn test_pathfinder_with_obstacles() {
    // Create a neighbors function that blocks certain coordinates
    let neighbors_with_obstacle = |coord: &MatrixCoordinate| -> Vec<MatrixCoordinate> {
        let mut neighbors = grid_neighbors(coord);

        // Remove any neighbor at (2, 1, 0) - obstacle
        neighbors.retain(|n| !(n.x == 2 && n.y == 1 && n.z == 0));

        // Remove any neighbor at (1, 2, 0) - obstacle
        neighbors.retain(|n| !(n.x == 1 && n.y == 2 && n.z == 0));

        neighbors
    };

    let finder = PathFinder::new();
    let start = MatrixCoordinate::new(0, 0, 0).unwrap();
    let goal = MatrixCoordinate::new(3, 3, 0).unwrap();

    let path = finder.find_path(&start, &goal, neighbors_with_obstacle).unwrap();

    // Path should avoid the obstacles
    assert!(path.len() >= 7);
    assert_eq!(path[0], start);
    assert_eq!(path[path.len() - 1], goal);

    // Verify path doesn't contain obstacles
    for coord in &path {
        assert!(!(coord.x == 2 && coord.y == 1 && coord.z == 0));
        assert!(!(coord.x == 1 && coord.y == 2 && coord.z == 0));
    }
}