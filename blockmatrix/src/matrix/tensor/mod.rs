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
//! use blockmatrix::matrix::tensor::{Vector3D, calculate_routing_vector, PathFinder};
//! use blockmatrix::matrix::MatrixCoordinate;
//!
//! // Calculate routing direction
//! let source = MatrixCoordinate::new(0, 0, 0)?;
//! let dest = MatrixCoordinate::new(100, 50, 25)?;
//! let direction = calculate_routing_vector(&source, &dest);
//!
//! // Find optimal path
//! let finder = PathFinder::new();
//! let path = finder.find_path(&source, &dest, |coord| {
//!     // Return valid neighbors for coordinate
//!     vec![]
//! })?;
//! ```

pub mod vector;
pub mod matrix_ops;
pub mod routing;
pub mod path_finding;

// Re-export main types and functions
pub use vector::{Vector3D, TensorError};
pub use matrix_ops::Matrix3x3;
pub use routing::{
    calculate_routing_vector,
    calculate_routing_path,
    routing_similarity,
    find_aligned_nodes,
    calculate_orthogonal_routes,
    calculate_load_balanced_routes,
    score_route_quality,
};
pub use path_finding::{
    PathFinder,
    PathError,
    calculate_path_cost,
    optimize_path,
    bidirectional_search,
};

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;