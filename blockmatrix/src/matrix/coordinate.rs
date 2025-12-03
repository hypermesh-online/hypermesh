//! Matrix coordinate types and distance calculations
//!
//! Provides the core `MatrixCoordinate` type representing positions in the
//! Block-MATRIX 3D coordinate space with various distance metrics.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Maximum coordinate value to prevent overflow in calculations
const MAX_COORD: i64 = i64::MAX / 4;
const MIN_COORD: i64 = i64::MIN / 4;

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

/// Represents a position in the Block-MATRIX 3D coordinate space
///
/// Each node in the Block-MATRIX network has a unique geospatial position
/// defined by (x, y, z) coordinates. These coordinates enable:
/// - Distance-based neighbor discovery
/// - Tensor operations for routing
/// - Matrix-aware shard distribution
/// - Hierarchical addressing through transformations
///
/// # Coordinate Bounds
///
/// Coordinates are bounded to prevent overflow in distance calculations:
/// - Min: i64::MIN / 4
/// - Max: i64::MAX / 4
///
/// # Examples
///
/// ```
/// use blockmatrix::matrix::MatrixCoordinate;
///
/// // Create coordinates
/// let origin = MatrixCoordinate::origin();
/// let node = MatrixCoordinate::new(10, 20, 30).unwrap();
///
/// // Calculate distances
/// let euclidean = origin.euclidean_distance(&node);
/// let manhattan = origin.manhattan_distance(&node);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MatrixCoordinate {
    /// X coordinate
    pub x: i64,
    /// Y coordinate
    pub y: i64,
    /// Z coordinate
    pub z: i64,
}

impl MatrixCoordinate {
    /// Create a new coordinate with validation
    ///
    /// Validates that coordinates are within bounds to prevent overflow
    /// in distance calculations and transformations.
    ///
    /// # Errors
    ///
    /// Returns `CoordinateError::OutOfBounds` if any coordinate is outside
    /// the valid range [MIN_COORD, MAX_COORD].
    ///
    /// # Examples
    ///
    /// ```
    /// use blockmatrix::matrix::MatrixCoordinate;
    ///
    /// let coord = MatrixCoordinate::new(100, 200, 300).unwrap();
    /// assert_eq!(coord.x, 100);
    /// ```
    pub fn new(x: i64, y: i64, z: i64) -> Result<Self, CoordinateError> {
        let coord = Self { x, y, z };
        coord.validate()?;
        Ok(coord)
    }

    /// Create the origin coordinate (0, 0, 0)
    ///
    /// # Examples
    ///
    /// ```
    /// use blockmatrix::matrix::MatrixCoordinate;
    ///
    /// let origin = MatrixCoordinate::origin();
    /// assert_eq!(origin.x, 0);
    /// assert_eq!(origin.y, 0);
    /// assert_eq!(origin.z, 0);
    /// ```
    pub fn origin() -> Self {
        Self { x: 0, y: 0, z: 0 }
    }

    /// Validate that coordinate is within bounds
    ///
    /// # Errors
    ///
    /// Returns `CoordinateError::OutOfBounds` if any coordinate exceeds bounds.
    pub fn validate(&self) -> Result<(), CoordinateError> {
        if self.x < MIN_COORD || self.x > MAX_COORD
            || self.y < MIN_COORD || self.y > MAX_COORD
            || self.z < MIN_COORD || self.z > MAX_COORD
        {
            return Err(CoordinateError::OutOfBounds(self.x, self.y, self.z));
        }
        Ok(())
    }

    /// Calculate Euclidean distance to another coordinate
    ///
    /// Returns the straight-line distance in 3D space:
    /// √((x₂-x₁)² + (y₂-y₁)² + (z₂-z₁)²)
    ///
    /// # Examples
    ///
    /// ```
    /// use blockmatrix::matrix::MatrixCoordinate;
    ///
    /// let a = MatrixCoordinate::new(0, 0, 0).unwrap();
    /// let b = MatrixCoordinate::new(3, 4, 0).unwrap();
    /// assert_eq!(a.euclidean_distance(&b), 5.0);
    /// ```
    pub fn euclidean_distance(&self, other: &Self) -> f64 {
        let dx = (other.x - self.x) as f64;
        let dy = (other.y - self.y) as f64;
        let dz = (other.z - self.z) as f64;

        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Calculate Manhattan distance to another coordinate
    ///
    /// Returns the sum of absolute differences along each axis:
    /// |x₂-x₁| + |y₂-y₁| + |z₂-z₁|
    ///
    /// Useful for grid-based routing where diagonal movement isn't allowed.
    ///
    /// # Examples
    ///
    /// ```
    /// use blockmatrix::matrix::MatrixCoordinate;
    ///
    /// let a = MatrixCoordinate::new(0, 0, 0).unwrap();
    /// let b = MatrixCoordinate::new(3, 4, 5).unwrap();
    /// assert_eq!(a.manhattan_distance(&b), 12);
    /// ```
    pub fn manhattan_distance(&self, other: &Self) -> i64 {
        (other.x - self.x).abs()
            + (other.y - self.y).abs()
            + (other.z - self.z).abs()
    }

    /// Calculate Chebyshev distance to another coordinate
    ///
    /// Returns the maximum absolute difference along any axis:
    /// max(|x₂-x₁|, |y₂-y₁|, |z₂-z₁|)
    ///
    /// Useful for determining minimum steps when diagonal movement is allowed.
    ///
    /// # Examples
    ///
    /// ```
    /// use blockmatrix::matrix::MatrixCoordinate;
    ///
    /// let a = MatrixCoordinate::new(0, 0, 0).unwrap();
    /// let b = MatrixCoordinate::new(3, 4, 2).unwrap();
    /// assert_eq!(a.chebyshev_distance(&b), 4);
    /// ```
    pub fn chebyshev_distance(&self, other: &Self) -> i64 {
        let dx = (other.x - self.x).abs();
        let dy = (other.y - self.y).abs();
        let dz = (other.z - self.z).abs();

        dx.max(dy).max(dz)
    }

    /// Check if coordinate is within distance threshold of another
    ///
    /// Uses Euclidean distance for comparison.
    ///
    /// # Examples
    ///
    /// ```
    /// use blockmatrix::matrix::MatrixCoordinate;
    ///
    /// let a = MatrixCoordinate::new(0, 0, 0).unwrap();
    /// let b = MatrixCoordinate::new(3, 4, 0).unwrap();
    /// assert!(a.is_within_distance(&b, 10.0));
    /// assert!(!a.is_within_distance(&b, 4.0));
    /// ```
    pub fn is_within_distance(&self, other: &Self, threshold: f64) -> bool {
        self.euclidean_distance(other) <= threshold
    }

    /// Calculate squared Euclidean distance (avoids sqrt for performance)
    ///
    /// Useful for comparisons where relative distance is sufficient.
    ///
    /// # Examples
    ///
    /// ```
    /// use blockmatrix::matrix::MatrixCoordinate;
    ///
    /// let a = MatrixCoordinate::new(0, 0, 0).unwrap();
    /// let b = MatrixCoordinate::new(3, 4, 0).unwrap();
    /// assert_eq!(a.squared_euclidean_distance(&b), 25);
    /// ```
    pub fn squared_euclidean_distance(&self, other: &Self) -> i64 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        let dz = other.z - self.z;

        dx * dx + dy * dy + dz * dz
    }
}

impl std::fmt::Display for MatrixCoordinate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_creation() {
        let coord = MatrixCoordinate::new(10, 20, 30).unwrap();
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
        let a = MatrixCoordinate::new(0, 0, 0).unwrap();
        let b = MatrixCoordinate::new(3, 4, 0).unwrap();
        assert_eq!(a.euclidean_distance(&b), 5.0);

        let c = MatrixCoordinate::new(1, 2, 3).unwrap();
        let d = MatrixCoordinate::new(4, 6, 8).unwrap();
        let dist = c.euclidean_distance(&d);
        assert!((dist - 7.0710678).abs() < 0.0001);
    }

    #[test]
    fn test_manhattan_distance() {
        let a = MatrixCoordinate::new(0, 0, 0).unwrap();
        let b = MatrixCoordinate::new(3, 4, 5).unwrap();
        assert_eq!(a.manhattan_distance(&b), 12);

        let c = MatrixCoordinate::new(-1, -2, -3).unwrap();
        let d = MatrixCoordinate::new(1, 2, 3).unwrap();
        assert_eq!(c.manhattan_distance(&d), 12);
    }

    #[test]
    fn test_chebyshev_distance() {
        let a = MatrixCoordinate::new(0, 0, 0).unwrap();
        let b = MatrixCoordinate::new(3, 4, 2).unwrap();
        assert_eq!(a.chebyshev_distance(&b), 4);

        let c = MatrixCoordinate::new(10, 20, 30).unwrap();
        let d = MatrixCoordinate::new(15, 22, 35).unwrap();
        assert_eq!(c.chebyshev_distance(&d), 5);
    }

    #[test]
    fn test_is_within_distance() {
        let a = MatrixCoordinate::new(0, 0, 0).unwrap();
        let b = MatrixCoordinate::new(3, 4, 0).unwrap();

        assert!(a.is_within_distance(&b, 10.0));
        assert!(a.is_within_distance(&b, 5.0));
        assert!(!a.is_within_distance(&b, 4.0));
    }

    #[test]
    fn test_squared_euclidean_distance() {
        let a = MatrixCoordinate::new(0, 0, 0).unwrap();
        let b = MatrixCoordinate::new(3, 4, 0).unwrap();
        assert_eq!(a.squared_euclidean_distance(&b), 25);
    }

    #[test]
    fn test_coordinate_equality() {
        let a = MatrixCoordinate::new(10, 20, 30).unwrap();
        let b = MatrixCoordinate::new(10, 20, 30).unwrap();
        let c = MatrixCoordinate::new(10, 20, 31).unwrap();

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_coordinate_display() {
        let coord = MatrixCoordinate::new(10, 20, 30).unwrap();
        assert_eq!(format!("{}", coord), "(10, 20, 30)");
    }

    #[test]
    fn test_negative_coordinates() {
        let a = MatrixCoordinate::new(-10, -20, -30).unwrap();
        let b = MatrixCoordinate::new(10, 20, 30).unwrap();

        let dist = a.euclidean_distance(&b);
        assert!((dist - 63.245553).abs() < 0.0001);
    }
}
