// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Tensor operations for Block-MATRIX intelligent routing
//!
//! This module provides mathematical tensor operations that enable Block-MATRIX
//! to make intelligent routing decisions based on matrix topology. These operations
//! are specifically designed for distributed network routing, not general-purpose
//! linear algebra.
//!
//! # Components
//!
//! - **Vector Operations**: 3D vector mathematics for direction and alignment
//! - **Matrix Operations**: 3x3 matrix transformations and rotations
//! - **Routing Algorithms**: Topology-aware routing calculations
//! - **Path Finding**: A* and other pathfinding algorithms
//!
//! # Example
//!
//! ```
//! use blockmatrix::matrix::tensor::{Vector3D, calculate_routing_vector};
//! use blockmatrix::matrix::MatrixCoordinate;
//!
//! // Calculate routing direction
//! let source = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
//! let dest = MatrixCoordinate::new(100, 50, 25).expect("test: valid coord");
//! let direction = calculate_routing_vector(&source, &dest);
//! assert!(direction.magnitude() > 0.0);
//! ```

pub mod matrix_ops;
pub mod path_finding;
pub mod routing;
pub mod transaction_routing;
pub mod vector;

// Re-export main types and functions
pub use matrix_ops::Matrix3x3;
pub use path_finding::{
    bidirectional_search, calculate_path_cost, optimize_path, PathError, PathFinder,
};
pub use routing::{
    calculate_load_balanced_routes, calculate_orthogonal_routes, calculate_routing_path,
    calculate_routing_vector, find_aligned_nodes, routing_similarity, score_route_quality,
};
pub use transaction_routing::{
    RouteHop, RoutingError, RoutingStatistics, ScopeTransition, TransactionRoute,
    TransactionRouter, TransactionRoutingConfig,
};
pub use vector::{TensorError, Vector3D};

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
