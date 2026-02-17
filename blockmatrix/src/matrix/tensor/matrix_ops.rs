// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! 3x3 Matrix operations for Block-MATRIX transformations
//!
//! Provides matrix mathematics for rotations, transformations, and coordinate
//! system operations within the Block-MATRIX topology.

use super::vector::{TensorError, Vector3D};

/// 3x3 matrix for rotation and transformation in Block-MATRIX space
///
/// Used for coordinate transformations, rotations, and projections
/// in the distributed matrix topology.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix3x3 {
    /// Matrix data in row-major order
    pub data: [[f64; 3]; 3],
}

impl Matrix3x3 {
    /// Create a new matrix from raw data
    pub fn new(data: [[f64; 3]; 3]) -> Self {
        Self { data }
    }

    /// Identity matrix (no transformation)
    ///
    /// Returns the 3x3 identity matrix:
    /// ```text
    /// [ 1  0  0 ]
    /// [ 0  1  0 ]
    /// [ 0  0  1 ]
    /// ```
    pub fn identity() -> Self {
        Self {
            data: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    /// Create rotation matrix around X axis
    ///
    /// Rotates vectors around the X axis by the given angle in radians.
    /// Positive angles rotate from Y toward Z.
    pub fn rotation_x(angle_radians: f64) -> Self {
        let cos_a = angle_radians.cos();
        let sin_a = angle_radians.sin();

        Self {
            data: [
                [1.0, 0.0, 0.0],
                [0.0, cos_a, -sin_a],
                [0.0, sin_a, cos_a],
            ],
        }
    }

    /// Create rotation matrix around Y axis
    ///
    /// Rotates vectors around the Y axis by the given angle in radians.
    /// Positive angles rotate from Z toward X.
    pub fn rotation_y(angle_radians: f64) -> Self {
        let cos_a = angle_radians.cos();
        let sin_a = angle_radians.sin();

        Self {
            data: [
                [cos_a, 0.0, sin_a],
                [0.0, 1.0, 0.0],
                [-sin_a, 0.0, cos_a],
            ],
        }
    }

    /// Create rotation matrix around Z axis
    ///
    /// Rotates vectors around the Z axis by the given angle in radians.
    /// Positive angles rotate from X toward Y.
    pub fn rotation_z(angle_radians: f64) -> Self {
        let cos_a = angle_radians.cos();
        let sin_a = angle_radians.sin();

        Self {
            data: [
                [cos_a, -sin_a, 0.0],
                [sin_a, cos_a, 0.0],
                [0.0, 0.0, 1.0],
            ],
        }
    }

    /// Create scaling matrix
    ///
    /// Scales vectors by the given factors along each axis.
    pub fn scaling(scale_x: f64, scale_y: f64, scale_z: f64) -> Self {
        Self {
            data: [
                [scale_x, 0.0, 0.0],
                [0.0, scale_y, 0.0],
                [0.0, 0.0, scale_z],
            ],
        }
    }

    /// Create uniform scaling matrix
    ///
    /// Scales vectors uniformly along all axes.
    pub fn uniform_scaling(scale: f64) -> Self {
        Self::scaling(scale, scale, scale)
    }

    /// Matrix multiplication
    ///
    /// Multiplies this matrix by another, returning the result.
    /// Used to compose multiple transformations.
    pub fn multiply(&self, other: &Self) -> Self {
        let mut result = [[0.0; 3]; 3];

        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    result[i][j] += self.data[i][k] * other.data[k][j];
                }
            }
        }

        Self { data: result }
    }

    /// Apply matrix to vector
    ///
    /// Transforms a vector by multiplying it with this matrix.
    pub fn transform_vector(&self, vec: &Vector3D) -> Vector3D {
        Vector3D::new(
            self.data[0][0] * vec.x + self.data[0][1] * vec.y + self.data[0][2] * vec.z,
            self.data[1][0] * vec.x + self.data[1][1] * vec.y + self.data[1][2] * vec.z,
            self.data[2][0] * vec.x + self.data[2][1] * vec.y + self.data[2][2] * vec.z,
        )
    }

    /// Transpose the matrix
    ///
    /// Returns the transpose of this matrix (rows become columns).
    pub fn transpose(&self) -> Self {
        let mut result = [[0.0; 3]; 3];

        for i in 0..3 {
            for j in 0..3 {
                result[i][j] = self.data[j][i];
            }
        }

        Self { data: result }
    }

    /// Calculate the determinant
    ///
    /// Returns the determinant of the 3x3 matrix.
    /// A determinant of zero indicates the matrix is singular (not invertible).
    pub fn determinant(&self) -> f64 {
        let d = &self.data;

        d[0][0] * (d[1][1] * d[2][2] - d[1][2] * d[2][1])
            - d[0][1] * (d[1][0] * d[2][2] - d[1][2] * d[2][0])
            + d[0][2] * (d[1][0] * d[2][1] - d[1][1] * d[2][0])
    }

    /// Calculate the inverse matrix
    ///
    /// Returns the inverse of this matrix if it exists.
    /// Returns error if matrix is singular (determinant = 0).
    pub fn inverse(&self) -> Result<Self, TensorError> {
        let det = self.determinant();

        if det.abs() < f64::EPSILON {
            return Err(TensorError::SingularMatrix);
        }

        let d = &self.data;
        let inv_det = 1.0 / det;

        // Calculate the adjugate matrix and divide by determinant
        let result = [
            [
                (d[1][1] * d[2][2] - d[1][2] * d[2][1]) * inv_det,
                (d[0][2] * d[2][1] - d[0][1] * d[2][2]) * inv_det,
                (d[0][1] * d[1][2] - d[0][2] * d[1][1]) * inv_det,
            ],
            [
                (d[1][2] * d[2][0] - d[1][0] * d[2][2]) * inv_det,
                (d[0][0] * d[2][2] - d[0][2] * d[2][0]) * inv_det,
                (d[0][2] * d[1][0] - d[0][0] * d[1][2]) * inv_det,
            ],
            [
                (d[1][0] * d[2][1] - d[1][1] * d[2][0]) * inv_det,
                (d[0][1] * d[2][0] - d[0][0] * d[2][1]) * inv_det,
                (d[0][0] * d[1][1] - d[0][1] * d[1][0]) * inv_det,
            ],
        ];

        Ok(Self { data: result })
    }

    /// Check if matrix is orthogonal
    ///
    /// Returns true if the matrix is orthogonal (its transpose equals its inverse).
    /// Orthogonal matrices preserve lengths and angles.
    pub fn is_orthogonal(&self, tolerance: f64) -> bool {
        let transpose = self.transpose();
        let product = self.multiply(&transpose);
        let identity = Self::identity();

        for i in 0..3 {
            for j in 0..3 {
                if (product.data[i][j] - identity.data[i][j]).abs() > tolerance {
                    return false;
                }
            }
        }

        true
    }

    /// Get matrix element at position (row, col)
    pub fn get(&self, row: usize, col: usize) -> Option<f64> {
        if row < 3 && col < 3 {
            Some(self.data[row][col])
        } else {
            None
        }
    }

    /// Set matrix element at position (row, col)
    pub fn set(&mut self, row: usize, col: usize, value: f64) -> Result<(), TensorError> {
        if row < 3 && col < 3 {
            self.data[row][col] = value;
            Ok(())
        } else {
            Err(TensorError::InvalidOperation(format!(
                "Index out of bounds: ({}, {})",
                row, col
            )))
        }
    }

    /// Create rotation matrix from axis and angle
    ///
    /// Uses Rodrigues' rotation formula to create a rotation matrix
    /// that rotates around an arbitrary axis.
    pub fn rotation_axis_angle(axis: &Vector3D, angle_radians: f64) -> Result<Self, TensorError> {
        let normalized = axis.normalize()?;
        let cos_a = angle_radians.cos();
        let sin_a = angle_radians.sin();
        let one_minus_cos = 1.0 - cos_a;

        let x = normalized.x;
        let y = normalized.y;
        let z = normalized.z;

        Ok(Self {
            data: [
                [
                    cos_a + x * x * one_minus_cos,
                    x * y * one_minus_cos - z * sin_a,
                    x * z * one_minus_cos + y * sin_a,
                ],
                [
                    y * x * one_minus_cos + z * sin_a,
                    cos_a + y * y * one_minus_cos,
                    y * z * one_minus_cos - x * sin_a,
                ],
                [
                    z * x * one_minus_cos - y * sin_a,
                    z * y * one_minus_cos + x * sin_a,
                    cos_a + z * z * one_minus_cos,
                ],
            ],
        })
    }
}

impl Default for Matrix3x3 {
    fn default() -> Self {
        Self::identity()
    }
}

impl std::fmt::Display for Matrix3x3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "[")?;
        for row in &self.data {
            writeln!(f, "  [{:8.3} {:8.3} {:8.3}]", row[0], row[1], row[2])?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    const EPSILON: f64 = 1e-10;

    #[test]
    fn test_identity_matrix() {
        let identity = Matrix3x3::identity();
        let vec = Vector3D::new(1.0, 2.0, 3.0);
        let transformed = identity.transform_vector(&vec);

        assert_eq!(transformed.x, vec.x);
        assert_eq!(transformed.y, vec.y);
        assert_eq!(transformed.z, vec.z);
    }

    #[test]
    fn test_rotation_matrices() {
        // Rotation around Z axis by 90 degrees
        let rot_z = Matrix3x3::rotation_z(PI / 2.0);
        let vec = Vector3D::new(1.0, 0.0, 0.0);
        let rotated = rot_z.transform_vector(&vec);

        assert!((rotated.x - 0.0).abs() < EPSILON);
        assert!((rotated.y - 1.0).abs() < EPSILON);
        assert!((rotated.z - 0.0).abs() < EPSILON);

        // Rotation around X axis by 90 degrees
        let rot_x = Matrix3x3::rotation_x(PI / 2.0);
        let vec2 = Vector3D::new(0.0, 1.0, 0.0);
        let rotated2 = rot_x.transform_vector(&vec2);

        assert!((rotated2.x - 0.0).abs() < EPSILON);
        assert!((rotated2.y - 0.0).abs() < EPSILON);
        assert!((rotated2.z - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_matrix_multiplication() {
        let rot_x = Matrix3x3::rotation_x(PI / 2.0);
        let rot_y = Matrix3x3::rotation_y(PI / 2.0);
        let combined = rot_x.multiply(&rot_y);

        let vec = Vector3D::new(1.0, 0.0, 0.0);
        let result = combined.transform_vector(&vec);

        // First rotate around Y (1,0,0) -> (0,0,-1)
        // Then rotate around X (0,0,-1) -> (0,1,0)
        assert!((result.x - 0.0).abs() < EPSILON);
        assert!((result.y - 0.0).abs() < EPSILON);
        assert!((result.z - -1.0).abs() < EPSILON);
    }

    #[test]
    fn test_matrix_transpose() {
        let mat = Matrix3x3::new([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]);

        let transposed = mat.transpose();

        assert_eq!(transposed.data[0][1], 4.0);
        assert_eq!(transposed.data[1][0], 2.0);
        assert_eq!(transposed.data[2][0], 3.0);
    }

    #[test]
    fn test_matrix_determinant() {
        let identity = Matrix3x3::identity();
        assert!((identity.determinant() - 1.0).abs() < EPSILON);

        let mat = Matrix3x3::new([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]);
        assert!(mat.determinant().abs() < EPSILON); // Singular matrix

        let mat2 = Matrix3x3::new([[2.0, 1.0, 3.0], [1.0, 0.0, 1.0], [0.0, 2.0, 4.0]]);
        assert!((mat2.determinant() - 2.0).abs() < EPSILON);
    }

    #[test]
    fn test_matrix_inverse() {
        let mat = Matrix3x3::new([[2.0, 1.0, 3.0], [1.0, 0.0, 1.0], [0.0, 2.0, 4.0]]);

        let inv = mat.inverse().unwrap();
        let product = mat.multiply(&inv);

        // Check if product is identity
        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((product.data[i][j] - expected).abs() < EPSILON);
            }
        }

        // Test singular matrix
        let singular = Matrix3x3::new([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]);
        assert!(singular.inverse().is_err());
    }

    #[test]
    fn test_scaling_matrix() {
        let scale = Matrix3x3::scaling(2.0, 3.0, 4.0);
        let vec = Vector3D::new(1.0, 1.0, 1.0);
        let scaled = scale.transform_vector(&vec);

        assert_eq!(scaled.x, 2.0);
        assert_eq!(scaled.y, 3.0);
        assert_eq!(scaled.z, 4.0);
    }

    #[test]
    fn test_orthogonal_matrix() {
        let rot = Matrix3x3::rotation_z(PI / 4.0);
        assert!(rot.is_orthogonal(EPSILON));

        let scale = Matrix3x3::scaling(2.0, 2.0, 2.0);
        assert!(!scale.is_orthogonal(EPSILON));
    }

    #[test]
    fn test_rotation_axis_angle() {
        // Rotate around Z axis by 90 degrees
        let axis = Vector3D::new(0.0, 0.0, 1.0);
        let rot = Matrix3x3::rotation_axis_angle(&axis, PI / 2.0).unwrap();

        let vec = Vector3D::new(1.0, 0.0, 0.0);
        let rotated = rot.transform_vector(&vec);

        assert!((rotated.x - 0.0).abs() < EPSILON);
        assert!((rotated.y - 1.0).abs() < EPSILON);
        assert!((rotated.z - 0.0).abs() < EPSILON);
    }
}