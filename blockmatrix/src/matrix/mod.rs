// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Matrix Coordinate System for Block-MATRIX Topology
//!
//! This module provides the foundational coordinate system for Block-MATRIX's
//! revolutionary matrix-based distributed computing architecture. Each node
//! in the network has a geospatial position (x,y,z coordinates) enabling
//! tensor-based operations for routing, resource allocation, and path finding.
//!
//! # Architecture
//!
//! - **MatrixCoordinate**: 3D position in matrix space with distance calculations
//! - **Transformations**: Translation, rotation, scaling for hierarchical addressing
//! - **Neighbor Discovery**: Distance-based neighbor finding algorithms
//! - **Tensor Operations**: Vector/matrix math for intelligent routing (Sprint 1.2)
//!
//! # Example
//!
//! ```
//! use blockmatrix::matrix::{MatrixCoordinate, find_k_nearest};
//!
//! // Create node positions
//! let node1 = MatrixCoordinate::new(0, 0, 0).unwrap();
//! let node2 = MatrixCoordinate::new(10, 20, 30).unwrap();
//!
//! // Calculate distance
//! let distance = node1.euclidean_distance(&node2);
//!
//! // Find neighbors
//! let candidates = vec![
//!     MatrixCoordinate::new(5, 5, 5).unwrap(),
//!     MatrixCoordinate::new(100, 100, 100).unwrap(),
//! ];
//! let neighbors = find_k_nearest(&node1, &candidates, 1);
//! ```

pub mod coordinate;
pub mod transforms;
pub mod neighbors;
pub mod tensor;
pub mod geospatial;

// Re-export main types
pub use coordinate::{MatrixCoordinate, CoordinateError};
pub use neighbors::{find_neighbors, find_k_nearest, find_neighbors_cubic};

#[cfg(test)]
mod tests;
