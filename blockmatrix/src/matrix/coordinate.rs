// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Matrix coordinate types and distance calculations
//!
//! Defines [`MatrixCoordinate`], BlockMatrix's `i64`-precision (x,y,z) cell type
//! for exact integer matrix maths — distinct from `hypermesh_lib::MatrixPosition`
//! (`f64`). Provides Euclidean/Manhattan/Chebyshev distance helpers and
//! [`MatrixCoordinate::derive_cell`], which delegates to the single canonical
//! construction in [`base::derive_cell`] so cell derivation cannot drift between
//! the Substrate and BlockMatrix.
//!
//! NOTE: the derived cell is an identity fingerprint (a deterministic content
//! derivation), NOT an authoritative physical location. Where an asset actually
//! lives and replicates is a demand-driven NGauge placement decision
//! (`ngauge::placement::PlacementLease`), not this hash. See VISION.md §5.5.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Maximum coordinate value to prevent overflow in calculations
const MAX_COORD: i64 = i64::MAX / 4;
const MIN_COORD: i64 = i64::MIN / 4;

/// 3D integer coordinate in Block-MATRIX topology.
///
/// Each node in the matrix has an (x,y,z) position used for tensor-based
/// routing, neighbor discovery, and resource allocation. Uses i64 for
/// integer-precision matrix operations (distinct from `hypermesh_lib::MatrixPosition`
/// which uses f64 for geospatial positioning).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MatrixCoordinate {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

impl std::fmt::Display for MatrixCoordinate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{},{})", self.x, self.y, self.z)
    }
}

/// Errors that can occur during coordinate operations
#[derive(Debug, Error, Clone, PartialEq)]
pub enum CoordinateError {
    /// Coordinate value is out of valid bounds
    #[error("Coordinate out of bounds: ({0}, {1}, {2})")]
    OutOfBounds(i64, i64, i64),

    /// Invalid transformation operation
    #[error("Invalid transformation: {0}")]
    InvalidTransformation(String),

    /// Overflow occurred during coordinate calculation
    #[error("Overflow in coordinate calculation")]
    Overflow,

    /// Invalid scale factor
    #[error("Invalid scale factor: {0}")]
    InvalidScale(i64),

    /// Invalid rotation angle (NaN or infinite)
    #[error("Invalid rotation angle: {0}")]
    InvalidRotation(f64),
}

impl MatrixCoordinate {
    /// Create a new coordinate with validation
    pub fn new(x: i64, y: i64, z: i64) -> Result<MatrixCoordinate, CoordinateError> {
        let coord = MatrixCoordinate { x, y, z };
        coord.validate()?;
        Ok(coord)
    }

    /// Create the origin coordinate (0, 0, 0)
    pub fn origin() -> MatrixCoordinate {
        MatrixCoordinate { x: 0, y: 0, z: 0 }
    }

    /// Validate that coordinate is within bounds
    pub fn validate(&self) -> Result<(), CoordinateError> {
        if self.x < MIN_COORD
            || self.x > MAX_COORD
            || self.y < MIN_COORD
            || self.y > MAX_COORD
            || self.z < MIN_COORD
            || self.z > MAX_COORD
        {
            return Err(CoordinateError::OutOfBounds(self.x, self.y, self.z));
        }
        Ok(())
    }

    /// Calculate Euclidean distance to another coordinate
    pub fn euclidean_distance(&self, other: &MatrixCoordinate) -> f64 {
        let dx = (other.x - self.x) as f64;
        let dy = (other.y - self.y) as f64;
        let dz = (other.z - self.z) as f64;

        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Calculate Manhattan distance to another coordinate
    pub fn manhattan_distance(&self, other: &MatrixCoordinate) -> i64 {
        let dx = other.x.saturating_sub(self.x).saturating_abs();
        let dy = other.y.saturating_sub(self.y).saturating_abs();
        let dz = other.z.saturating_sub(self.z).saturating_abs();
        dx.saturating_add(dy).saturating_add(dz)
    }

    /// Calculate Chebyshev distance to another coordinate
    pub fn chebyshev_distance(&self, other: &MatrixCoordinate) -> i64 {
        let dx = other.x.saturating_sub(self.x).saturating_abs();
        let dy = other.y.saturating_sub(self.y).saturating_abs();
        let dz = other.z.saturating_sub(self.z).saturating_abs();

        dx.max(dy).max(dz)
    }

    /// Check if coordinate is within distance threshold of another
    pub fn is_within_distance(&self, other: &MatrixCoordinate, threshold: f64) -> bool {
        self.euclidean_distance(other) <= threshold
    }

    /// Calculate squared Euclidean distance (avoids sqrt for performance).
    /// Uses saturating arithmetic to prevent overflow at extreme coordinates.
    pub fn squared_euclidean_distance(&self, other: &MatrixCoordinate) -> i64 {
        let dx = other.x.saturating_sub(self.x);
        let dy = other.y.saturating_sub(self.y);
        let dz = other.z.saturating_sub(self.z);

        dx.saturating_mul(dx)
            .saturating_add(dy.saturating_mul(dy))
            .saturating_add(dz.saturating_mul(dz))
    }

    /// Deterministically derive a matrix cell from a device node ID.
    ///
    /// Device-auth invariant: the node's cell is DERIVED from its identity, not
    /// freely self-declared. `device_node_id` is the canonical node ID
    /// (`BLAKE3(falcon_pubkey)` hex).
    ///
    /// The BLAKE3 construction lives in ONE place — [`base::derive_cell`] — so the
    /// cell derivation cannot drift between the Substrate and BlockMatrix. This
    /// method delegates there and wraps the returned `i16` axes in a
    /// [`MatrixCoordinate`].
    ///
    /// The coordinate space matches [`hypermesh_lib::AssetAddress`], which encodes
    /// matrix coords as `i16` big-endian (bytes 4-9). `i16` axes are always inside
    /// [`MatrixCoordinate`] bounds, so `new()` cannot fail; the fallback to origin
    /// is defensive and never taken.
    ///
    /// This cell is an identity fingerprint, NOT an authoritative physical
    /// location. It is a deterministic content/identity derivation; where the
    /// asset actually lives and replicates is a demand-driven NGauge placement
    /// decision (`ngauge::placement::PlacementLease`), not this hash. See
    /// VISION.md §5.5.
    pub fn derive_cell(device_node_id: &str) -> MatrixCoordinate {
        let (x, y, z) = base::derive_cell(device_node_id);
        MatrixCoordinate::new(x as i64, y as i64, z as i64)
            .unwrap_or_else(|_| MatrixCoordinate::origin())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_creation() {
        let coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
        assert_eq!(coord.x, 10);
        assert_eq!(coord.y, 20);
        assert_eq!(coord.z, 30);
    }

    #[test]
    fn test_origin() {
        let origin = MatrixCoordinate::origin();
        assert_eq!(origin.x, 0);
        assert_eq!(origin.y, 0);
        assert_eq!(origin.z, 0);
    }

    #[test]
    fn test_coordinate_validation() {
        // Valid coordinates
        assert!(MatrixCoordinate::new(0, 0, 0).is_ok());
        assert!(MatrixCoordinate::new(100, 200, 300).is_ok());
        assert!(MatrixCoordinate::new(-100, -200, -300).is_ok());

        // Out of bounds
        assert!(MatrixCoordinate::new(MAX_COORD + 1, 0, 0).is_err());
        assert!(MatrixCoordinate::new(0, MIN_COORD - 1, 0).is_err());
    }

    #[test]
    fn test_euclidean_distance() {
        let a = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let b = MatrixCoordinate::new(3, 4, 0).expect("test: valid coordinate");
        assert_eq!(a.euclidean_distance(&b), 5.0);

        let c = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        let d = MatrixCoordinate::new(4, 6, 8).expect("test: valid coordinate");
        let dist = c.euclidean_distance(&d);
        assert!((dist - 7.0710678).abs() < 0.0001);
    }

    #[test]
    fn test_manhattan_distance() {
        let a = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let b = MatrixCoordinate::new(3, 4, 5).expect("test: valid coordinate");
        assert_eq!(a.manhattan_distance(&b), 12);

        let c = MatrixCoordinate::new(-1, -2, -3).expect("test: valid coordinate");
        let d = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        assert_eq!(c.manhattan_distance(&d), 12);
    }

    #[test]
    fn test_chebyshev_distance() {
        let a = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let b = MatrixCoordinate::new(3, 4, 2).expect("test: valid coordinate");
        assert_eq!(a.chebyshev_distance(&b), 4);

        let c = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
        let d = MatrixCoordinate::new(15, 22, 35).expect("test: valid coordinate");
        assert_eq!(c.chebyshev_distance(&d), 5);
    }

    #[test]
    fn test_is_within_distance() {
        let a = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let b = MatrixCoordinate::new(3, 4, 0).expect("test: valid coordinate");

        assert!(a.is_within_distance(&b, 10.0));
        assert!(a.is_within_distance(&b, 5.0));
        assert!(!a.is_within_distance(&b, 4.0));
    }

    #[test]
    fn test_squared_euclidean_distance() {
        let a = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let b = MatrixCoordinate::new(3, 4, 0).expect("test: valid coordinate");
        assert_eq!(a.squared_euclidean_distance(&b), 25);
    }

    #[test]
    fn test_coordinate_equality() {
        let a = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
        let b = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
        let c = MatrixCoordinate::new(10, 20, 31).expect("test: valid coordinate");

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_coordinate_display() {
        let coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
        // hypermesh_lib Display uses no-space format: (10,20,30)
        assert_eq!(format!("{coord}"), "(10,20,30)");
    }

    #[test]
    fn test_negative_coordinates() {
        let a = MatrixCoordinate::new(-10, -20, -30).expect("test: valid coordinate");
        let b = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");

        let dist = a.euclidean_distance(&b);
        // sqrt(20^2 + 40^2 + 60^2) = sqrt(5600) = 74.83315...
        assert!((dist - 74.83315).abs() < 0.001);
    }

    #[test]
    fn test_derive_cell_delegates_to_base_byte_identically() {
        // Delegation must produce exactly the axes base::derive_cell computes —
        // proving the single canonical construction survives the crate boundary.
        for id in ["9f4fc6ed4ba7", "node-a", ""] {
            let (bx, by, bz) = base::derive_cell(id);
            let coord = MatrixCoordinate::derive_cell(id);
            assert_eq!(coord.x, bx as i64);
            assert_eq!(coord.y, by as i64);
            assert_eq!(coord.z, bz as i64);
        }
    }

    #[test]
    fn test_derive_cell_is_deterministic() {
        assert_eq!(
            MatrixCoordinate::derive_cell("stable-id"),
            MatrixCoordinate::derive_cell("stable-id")
        );
    }
}
