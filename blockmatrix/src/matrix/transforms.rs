// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Coordinate transformation operations
//!
//! Provides translation, rotation, and scaling operations for matrix coordinates.
//! These transformations enable hierarchical addressing and topology manipulations.

use super::coordinate::{CoordinateError, MatrixCoordinate};

impl MatrixCoordinate {
    /// Translate coordinate by offset
    ///
    /// Adds the specified offsets to each axis. Validates the result
    /// is within bounds.
    ///
    /// # Errors
    ///
    /// Returns `CoordinateError::Overflow` if addition would overflow.
    /// Returns `CoordinateError::OutOfBounds` if result exceeds valid range.
    ///
    /// # Examples
    ///
    /// ```
    /// use blockmatrix::matrix::MatrixCoordinate;
    ///
    /// let coord = MatrixCoordinate::new(10, 20, 30).unwrap();
    /// let translated = coord.translate(5, -10, 15).unwrap();
    /// assert_eq!(translated, MatrixCoordinate::new(15, 10, 45).unwrap());
    /// ```
    pub fn translate(&self, dx: i64, dy: i64, dz: i64) -> Result<Self, CoordinateError> {
        let x = self.x.checked_add(dx).ok_or(CoordinateError::Overflow)?;
        let y = self.y.checked_add(dy).ok_or(CoordinateError::Overflow)?;
        let z = self.z.checked_add(dz).ok_or(CoordinateError::Overflow)?;

        Self::new(x, y, z)
    }

    /// Scale coordinate by factor
    ///
    /// Multiplies each coordinate by the scale factor. Useful for
    /// hierarchical addressing where different layers use different scales.
    ///
    /// # Errors
    ///
    /// Returns `CoordinateError::InvalidScale` if factor is 0.
    /// Returns `CoordinateError::Overflow` if multiplication would overflow.
    /// Returns `CoordinateError::OutOfBounds` if result exceeds valid range.
    ///
    /// # Examples
    ///
    /// ```
    /// use blockmatrix::matrix::MatrixCoordinate;
    ///
    /// let coord = MatrixCoordinate::new(10, 20, 30).unwrap();
    /// let scaled = coord.scale(2).unwrap();
    /// assert_eq!(scaled, MatrixCoordinate::new(20, 40, 60).unwrap());
    /// ```
    pub fn scale(&self, factor: i64) -> Result<Self, CoordinateError> {
        if factor == 0 {
            return Err(CoordinateError::InvalidScale(factor));
        }

        let x = self
            .x
            .checked_mul(factor)
            .ok_or(CoordinateError::Overflow)?;
        let y = self
            .y
            .checked_mul(factor)
            .ok_or(CoordinateError::Overflow)?;
        let z = self
            .z
            .checked_mul(factor)
            .ok_or(CoordinateError::Overflow)?;

        Self::new(x, y, z)
    }

    /// Rotate coordinate around X-axis
    ///
    /// Performs a right-hand rotation around the X-axis by the specified
    /// angle in degrees. Y and Z coordinates are affected, X remains unchanged.
    ///
    /// # Rotation Matrix (X-axis)
    /// ```text
    /// [1    0           0       ]
    /// [0    cos(θ)     -sin(θ) ]
    /// [0    sin(θ)      cos(θ) ]
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `CoordinateError::InvalidRotation` if angle is NaN or infinite.
    /// Returns `CoordinateError::OutOfBounds` if result exceeds valid range.
    ///
    /// # Examples
    ///
    /// ```
    /// use blockmatrix::matrix::MatrixCoordinate;
    ///
    /// let coord = MatrixCoordinate::new(10, 20, 0).unwrap();
    /// let rotated = coord.rotate_x(90.0).unwrap();
    /// // After 90° rotation around X: (10, 0, 20)
    /// ```
    pub fn rotate_x(&self, degrees: f64) -> Result<Self, CoordinateError> {
        if !degrees.is_finite() {
            return Err(CoordinateError::InvalidRotation(degrees));
        }

        let radians = degrees.to_radians();
        let cos_theta = radians.cos();
        let sin_theta = radians.sin();

        let y_f = (self.y as f64) * cos_theta - (self.z as f64) * sin_theta;
        let z_f = (self.y as f64) * sin_theta + (self.z as f64) * cos_theta;

        let y = y_f.round() as i64;
        let z = z_f.round() as i64;

        Self::new(self.x, y, z)
    }

    /// Rotate coordinate around Y-axis
    ///
    /// Performs a right-hand rotation around the Y-axis by the specified
    /// angle in degrees. X and Z coordinates are affected, Y remains unchanged.
    ///
    /// # Rotation Matrix (Y-axis)
    /// ```text
    /// [ cos(θ)     0    sin(θ) ]
    /// [ 0          1    0      ]
    /// [-sin(θ)     0    cos(θ) ]
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `CoordinateError::InvalidRotation` if angle is NaN or infinite.
    /// Returns `CoordinateError::OutOfBounds` if result exceeds valid range.
    ///
    /// # Examples
    ///
    /// ```
    /// use blockmatrix::matrix::MatrixCoordinate;
    ///
    /// let coord = MatrixCoordinate::new(20, 10, 0).unwrap();
    /// let rotated = coord.rotate_y(90.0).unwrap();
    /// // After 90° rotation around Y: (0, 10, -20)
    /// ```
    pub fn rotate_y(&self, degrees: f64) -> Result<Self, CoordinateError> {
        if !degrees.is_finite() {
            return Err(CoordinateError::InvalidRotation(degrees));
        }

        let radians = degrees.to_radians();
        let cos_theta = radians.cos();
        let sin_theta = radians.sin();

        let x_f = (self.x as f64) * cos_theta + (self.z as f64) * sin_theta;
        let z_f = -(self.x as f64) * sin_theta + (self.z as f64) * cos_theta;

        let x = x_f.round() as i64;
        let z = z_f.round() as i64;

        Self::new(x, self.y, z)
    }

    /// Rotate coordinate around Z-axis
    ///
    /// Performs a right-hand rotation around the Z-axis by the specified
    /// angle in degrees. X and Y coordinates are affected, Z remains unchanged.
    ///
    /// # Rotation Matrix (Z-axis)
    /// ```text
    /// [cos(θ)     -sin(θ)    0]
    /// [sin(θ)      cos(θ)    0]
    /// [0           0         1]
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `CoordinateError::InvalidRotation` if angle is NaN or infinite.
    /// Returns `CoordinateError::OutOfBounds` if result exceeds valid range.
    ///
    /// # Examples
    ///
    /// ```
    /// use blockmatrix::matrix::MatrixCoordinate;
    ///
    /// let coord = MatrixCoordinate::new(20, 0, 10).unwrap();
    /// let rotated = coord.rotate_z(90.0).unwrap();
    /// // After 90° rotation around Z: (0, 20, 10)
    /// ```
    pub fn rotate_z(&self, degrees: f64) -> Result<Self, CoordinateError> {
        if !degrees.is_finite() {
            return Err(CoordinateError::InvalidRotation(degrees));
        }

        let radians = degrees.to_radians();
        let cos_theta = radians.cos();
        let sin_theta = radians.sin();

        let x_f = (self.x as f64) * cos_theta - (self.y as f64) * sin_theta;
        let y_f = (self.x as f64) * sin_theta + (self.y as f64) * cos_theta;

        let x = x_f.round() as i64;
        let y = y_f.round() as i64;

        Self::new(x, y, self.z)
    }

    /// Apply multiple transformations in sequence
    ///
    /// Convenience method for chaining transformations. Transformations
    /// are applied in the order specified.
    ///
    /// # Examples
    ///
    /// ```
    /// use blockmatrix::matrix::MatrixCoordinate;
    ///
    /// let coord = MatrixCoordinate::new(10, 10, 10).unwrap();
    /// let result = coord
    ///     .translate(5, 5, 5).unwrap()
    ///     .scale(2).unwrap()
    ///     .rotate_z(45.0).unwrap();
    /// ```
    pub fn apply_transform<F>(&self, transform: F) -> Result<Self, CoordinateError>
    where
        F: FnOnce(&Self) -> Result<Self, CoordinateError>,
    {
        transform(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate() {
        let coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
        let translated = coord.translate(5, -10, 15).expect("test: expected success");
        assert_eq!(translated, MatrixCoordinate::new(15, 10, 45).expect("test: valid coordinate"));
    }

    #[test]
    fn test_translate_negative() {
        let coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
        let translated = coord.translate(-20, -30, -40).expect("test: expected success");
        assert_eq!(translated, MatrixCoordinate::new(-10, -10, -10).expect("test: valid coordinate"));
    }

    #[test]
    fn test_translate_overflow() {
        let coord = MatrixCoordinate::new(i64::MAX / 4, 0, 0).expect("test: valid coordinate");
        let result = coord.translate(i64::MAX / 4 + 1, 0, 0);
        assert!(matches!(
            result,
            Err(CoordinateError::Overflow) | Err(CoordinateError::OutOfBounds(_, _, _))
        ));
    }

    #[test]
    fn test_scale() {
        let coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
        let scaled = coord.scale(2).expect("test: expected success");
        assert_eq!(scaled, MatrixCoordinate::new(20, 40, 60).expect("test: valid coordinate"));
    }

    #[test]
    fn test_scale_negative() {
        let coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
        let scaled = coord.scale(-1).expect("test: expected success");
        assert_eq!(scaled, MatrixCoordinate::new(-10, -20, -30).expect("test: valid coordinate"));
    }

    #[test]
    fn test_scale_zero() {
        let coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
        let result = coord.scale(0);
        assert!(matches!(result, Err(CoordinateError::InvalidScale(0))));
    }

    #[test]
    fn test_rotate_x_90() {
        let coord = MatrixCoordinate::new(10, 20, 0).expect("test: valid coordinate");
        let rotated = coord.rotate_x(90.0).expect("test: expected success");
        // After 90° rotation around X: y' = 0, z' = 20
        assert_eq!(rotated.x, 10);
        assert!((rotated.y as f64).abs() < 1.0); // Close to 0
        assert!((rotated.z - 20).abs() < 1);
    }

    #[test]
    fn test_rotate_y_90() {
        let coord = MatrixCoordinate::new(20, 10, 0).expect("test: valid coordinate");
        let rotated = coord.rotate_y(90.0).expect("test: expected success");
        // After 90° rotation around Y: x' = 0, z' = -20
        assert!((rotated.x as f64).abs() < 1.0); // Close to 0
        assert_eq!(rotated.y, 10);
        assert!((rotated.z + 20).abs() < 1);
    }

    #[test]
    fn test_rotate_z_90() {
        let coord = MatrixCoordinate::new(20, 0, 10).expect("test: valid coordinate");
        let rotated = coord.rotate_z(90.0).expect("test: expected success");
        // After 90° rotation around Z: x' = 0, y' = 20
        assert!((rotated.x as f64).abs() < 1.0); // Close to 0
        assert!((rotated.y - 20).abs() < 1);
        assert_eq!(rotated.z, 10);
    }

    #[test]
    fn test_rotate_360() {
        let coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
        let rotated = coord.rotate_x(360.0).expect("test: expected success");
        // 360° rotation should return to original position
        assert!((rotated.x - coord.x).abs() < 1);
        assert!((rotated.y - coord.y).abs() < 1);
        assert!((rotated.z - coord.z).abs() < 1);
    }

    #[test]
    fn test_rotate_invalid() {
        let coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
        assert!(matches!(
            coord.rotate_x(f64::NAN),
            Err(CoordinateError::InvalidRotation(_))
        ));
        assert!(matches!(
            coord.rotate_y(f64::INFINITY),
            Err(CoordinateError::InvalidRotation(_))
        ));
    }

    #[test]
    fn test_chained_transformations() {
        let coord = MatrixCoordinate::new(10, 10, 10).expect("test: valid coordinate");
        let result = coord.translate(5, 5, 5).expect("test: expected result").scale(2).expect("test: expected result");

        assert_eq!(result, MatrixCoordinate::new(30, 30, 30).expect("test: valid coordinate"));
    }

    #[test]
    fn test_apply_transform() {
        let coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
        let result = coord.apply_transform(|c| c.scale(2)).expect("test: expected result");
        assert_eq!(result, MatrixCoordinate::new(20, 40, 60).expect("test: valid coordinate"));
    }
}
