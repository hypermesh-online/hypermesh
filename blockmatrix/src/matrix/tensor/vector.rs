// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! 3D Vector operations for Block-MATRIX routing calculations
//!
//! Provides vector mathematics for intelligent routing decisions based on
//! matrix topology. These operations enable path finding, direction scoring,
//! and alignment calculations for distributed network routing.

use crate::matrix::coordinate::MatrixCoordinate;
use thiserror::Error;

/// Errors that can occur during tensor operations
#[derive(Debug, Error, Clone, PartialEq)]
pub enum TensorError {
    /// Cannot normalize zero vector
    #[error("Cannot normalize zero vector")]
    ZeroVector,

    /// Matrix is singular (determinant = 0)
    #[error("Matrix is singular (determinant = 0)")]
    SingularMatrix,

    /// Division by zero in tensor operation
    #[error("Division by zero in tensor operation")]
    DivisionByZero,

    /// Invalid operation
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// 3D vector for routing calculations in Block-MATRIX space
///
/// Represents directional information between nodes in the matrix topology.
/// Used for routing decisions, path optimization, and similarity scoring.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector3D {
    /// Create a new vector with given components
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Create vector from two matrix coordinates (direction from a to b)
    ///
    /// # Example
    /// ```
    /// let from = MatrixCoordinate::new(0, 0, 0)?;
    /// let to = MatrixCoordinate::new(10, 5, 2)?;
    /// let direction = Vector3D::from_coordinates(&from, &to);
    /// assert_eq!(direction.x, 10.0);
    /// ```
    pub fn from_coordinates(from: &MatrixCoordinate, to: &MatrixCoordinate) -> Self {
        Self {
            x: (to.x - from.x) as f64,
            y: (to.y - from.y) as f64,
            z: (to.z - from.z) as f64,
        }
    }

    /// Vector magnitude (length)
    ///
    /// Returns the Euclidean length of the vector.
    pub fn magnitude(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Normalize to unit vector
    ///
    /// Returns a vector with the same direction but magnitude 1.0.
    /// Returns error if vector is zero.
    pub fn normalize(&self) -> Result<Self, TensorError> {
        let mag = self.magnitude();
        if mag < f64::EPSILON {
            return Err(TensorError::ZeroVector);
        }
        Ok(Self {
            x: self.x / mag,
            y: self.y / mag,
            z: self.z / mag,
        })
    }

    /// Dot product (for similarity/alignment scoring)
    ///
    /// Returns scalar value indicating how aligned two vectors are.
    /// Result is positive if vectors point in similar directions,
    /// zero if perpendicular, negative if opposite.
    pub fn dot(&self, other: &Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Cross product (for orthogonal routing)
    ///
    /// Returns a vector perpendicular to both input vectors.
    /// Useful for finding alternative routing paths.
    pub fn cross(&self, other: &Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// Scalar multiplication
    ///
    /// Scale vector by given factor.
    pub fn scale(&self, scalar: f64) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }

    /// Vector addition
    pub fn add(&self, other: &Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }

    /// Vector subtraction
    pub fn subtract(&self, other: &Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    /// Angle between vectors (radians)
    ///
    /// Returns the angle in radians between two vectors.
    /// Result is in range [0, π].
    pub fn angle_between(&self, other: &Self) -> f64 {
        let dot = self.dot(other);
        let mag_product = self.magnitude() * other.magnitude();

        if mag_product < f64::EPSILON {
            return 0.0;
        }

        // Clamp to avoid numerical errors with acos
        let cos_angle = (dot / mag_product).clamp(-1.0, 1.0);
        cos_angle.acos()
    }

    /// Project this vector onto another
    ///
    /// Returns the projection of this vector onto the target vector.
    /// Useful for finding the component of movement in a specific direction.
    pub fn project_onto(&self, other: &Self) -> Result<Self, TensorError> {
        let other_mag_sq = other.dot(other);
        if other_mag_sq < f64::EPSILON {
            return Err(TensorError::ZeroVector);
        }

        let scalar = self.dot(other) / other_mag_sq;
        Ok(other.scale(scalar))
    }

    /// Check if vector is zero (within epsilon)
    pub fn is_zero(&self) -> bool {
        self.magnitude() < f64::EPSILON
    }

    /// Linear interpolation between two vectors
    ///
    /// Returns vector at position t between self and other.
    /// t=0 returns self, t=1 returns other.
    pub fn lerp(&self, other: &Self, t: f64) -> Self {
        let t = t.clamp(0.0, 1.0);
        self.scale(1.0 - t).add(&other.scale(t))
    }

    /// Convert vector to unit vector in same direction
    ///
    /// Same as normalize but returns zero vector if magnitude is zero
    pub fn to_unit(&self) -> Self {
        self.normalize().unwrap_or(Self::new(0.0, 0.0, 0.0))
    }
}

impl Default for Vector3D {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}

impl std::fmt::Display for Vector3D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:.2}, {:.2}, {:.2})", self.x, self.y, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::coordinate::MatrixCoordinate;

    #[test]
    fn test_vector_from_coordinates() {
        let from = MatrixCoordinate::new(0, 0, 0).unwrap();
        let to = MatrixCoordinate::new(10, 5, 2).unwrap();
        let vec = Vector3D::from_coordinates(&from, &to);

        assert_eq!(vec.x, 10.0);
        assert_eq!(vec.y, 5.0);
        assert_eq!(vec.z, 2.0);
    }

    #[test]
    fn test_vector_magnitude() {
        let vec = Vector3D::new(3.0, 4.0, 0.0);
        assert_eq!(vec.magnitude(), 5.0);

        let vec2 = Vector3D::new(2.0, 3.0, 6.0);
        assert!((vec2.magnitude() - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_vector_normalize() {
        let vec = Vector3D::new(3.0, 4.0, 0.0);
        let normalized = vec.normalize().unwrap();
        assert!((normalized.magnitude() - 1.0).abs() < f64::EPSILON);
        assert!((normalized.x - 0.6).abs() < 0.001);
        assert!((normalized.y - 0.8).abs() < 0.001);

        // Test zero vector
        let zero = Vector3D::new(0.0, 0.0, 0.0);
        assert!(zero.normalize().is_err());
    }

    #[test]
    fn test_vector_dot_product() {
        let vec1 = Vector3D::new(1.0, 2.0, 3.0);
        let vec2 = Vector3D::new(4.0, 5.0, 6.0);
        assert_eq!(vec1.dot(&vec2), 32.0); // 1*4 + 2*5 + 3*6

        // Perpendicular vectors
        let vec3 = Vector3D::new(1.0, 0.0, 0.0);
        let vec4 = Vector3D::new(0.0, 1.0, 0.0);
        assert_eq!(vec3.dot(&vec4), 0.0);
    }

    #[test]
    fn test_vector_cross_product() {
        let x_axis = Vector3D::new(1.0, 0.0, 0.0);
        let y_axis = Vector3D::new(0.0, 1.0, 0.0);
        let z_axis = x_axis.cross(&y_axis);

        assert_eq!(z_axis.x, 0.0);
        assert_eq!(z_axis.y, 0.0);
        assert_eq!(z_axis.z, 1.0);
    }

    #[test]
    fn test_vector_angle_between() {
        let vec1 = Vector3D::new(1.0, 0.0, 0.0);
        let vec2 = Vector3D::new(0.0, 1.0, 0.0);
        let angle = vec1.angle_between(&vec2);
        assert!((angle - PI / 2.0).abs() < 0.001);

        // Same direction
        let vec3 = Vector3D::new(2.0, 0.0, 0.0);
        let angle2 = vec1.angle_between(&vec3);
        assert!(angle2 < 0.001);

        // Opposite direction
        let vec4 = Vector3D::new(-1.0, 0.0, 0.0);
        let angle3 = vec1.angle_between(&vec4);
        assert!((angle3 - PI).abs() < 0.001);
    }

    #[test]
    fn test_vector_projection() {
        let vec1 = Vector3D::new(3.0, 4.0, 0.0);
        let vec2 = Vector3D::new(1.0, 0.0, 0.0);
        let proj = vec1.project_onto(&vec2).unwrap();

        assert_eq!(proj.x, 3.0);
        assert_eq!(proj.y, 0.0);
        assert_eq!(proj.z, 0.0);
    }

    #[test]
    fn test_vector_operations() {
        let vec1 = Vector3D::new(1.0, 2.0, 3.0);
        let vec2 = Vector3D::new(4.0, 5.0, 6.0);

        // Addition
        let sum = vec1.add(&vec2);
        assert_eq!(sum.x, 5.0);
        assert_eq!(sum.y, 7.0);
        assert_eq!(sum.z, 9.0);

        // Subtraction
        let diff = vec2.subtract(&vec1);
        assert_eq!(diff.x, 3.0);
        assert_eq!(diff.y, 3.0);
        assert_eq!(diff.z, 3.0);

        // Scaling
        let scaled = vec1.scale(2.0);
        assert_eq!(scaled.x, 2.0);
        assert_eq!(scaled.y, 4.0);
        assert_eq!(scaled.z, 6.0);
    }

    #[test]
    fn test_vector_lerp() {
        let vec1 = Vector3D::new(0.0, 0.0, 0.0);
        let vec2 = Vector3D::new(10.0, 10.0, 10.0);

        let mid = vec1.lerp(&vec2, 0.5);
        assert_eq!(mid.x, 5.0);
        assert_eq!(mid.y, 5.0);
        assert_eq!(mid.z, 5.0);

        let start = vec1.lerp(&vec2, 0.0);
        assert_eq!(start.x, 0.0);

        let end = vec1.lerp(&vec2, 1.0);
        assert_eq!(end.x, 10.0);
    }
}