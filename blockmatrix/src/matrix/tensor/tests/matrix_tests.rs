// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Comprehensive tests for Matrix3x3 operations

use crate::matrix::tensor::matrix_ops::Matrix3x3;
use crate::matrix::tensor::vector::{TensorError, Vector3D};
use std::f64::consts::PI;

const EPSILON: f64 = 1e-10;

#[test]
fn test_identity_matrix_creation() {
    let identity = Matrix3x3::identity();

    for i in 0..3 {
        for j in 0..3 {
            if i == j {
                assert_eq!(identity.data[i][j], 1.0);
            } else {
                assert_eq!(identity.data[i][j], 0.0);
            }
        }
    }
}

#[test]
#[allow(clippy::approx_constant)]
fn test_identity_matrix_transform() {
    let identity = Matrix3x3::identity();
    let vec = Vector3D::new(3.14, 2.71, 1.41);
    let transformed = identity.transform_vector(&vec);

    assert_eq!(transformed.x, vec.x);
    assert_eq!(transformed.y, vec.y);
    assert_eq!(transformed.z, vec.z);
}

#[test]
fn test_rotation_x_90_degrees() {
    let rot = Matrix3x3::rotation_x(PI / 2.0);
    let vec = Vector3D::new(0.0, 1.0, 0.0);
    let rotated = rot.transform_vector(&vec);

    assert!((rotated.x - 0.0).abs() < EPSILON);
    assert!((rotated.y - 0.0).abs() < EPSILON);
    assert!((rotated.z - 1.0).abs() < EPSILON);
}

#[test]
fn test_rotation_y_90_degrees() {
    let rot = Matrix3x3::rotation_y(PI / 2.0);
    let vec = Vector3D::new(1.0, 0.0, 0.0);
    let rotated = rot.transform_vector(&vec);

    assert!((rotated.x - 0.0).abs() < EPSILON);
    assert!((rotated.y - 0.0).abs() < EPSILON);
    assert!((rotated.z - -1.0).abs() < EPSILON);
}

#[test]
fn test_rotation_z_90_degrees() {
    let rot = Matrix3x3::rotation_z(PI / 2.0);
    let vec = Vector3D::new(1.0, 0.0, 0.0);
    let rotated = rot.transform_vector(&vec);

    assert!((rotated.x - 0.0).abs() < EPSILON);
    assert!((rotated.y - 1.0).abs() < EPSILON);
    assert!((rotated.z - 0.0).abs() < EPSILON);
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
fn test_uniform_scaling() {
    let scale = Matrix3x3::uniform_scaling(2.5);
    let vec = Vector3D::new(1.0, 2.0, 3.0);
    let scaled = scale.transform_vector(&vec);

    assert_eq!(scaled.x, 2.5);
    assert_eq!(scaled.y, 5.0);
    assert_eq!(scaled.z, 7.5);
}

#[test]
fn test_matrix_multiplication_associativity() {
    let m1 = Matrix3x3::rotation_x(PI / 4.0);
    let m2 = Matrix3x3::rotation_y(PI / 3.0);
    let m3 = Matrix3x3::rotation_z(PI / 6.0);

    let result1 = m1.multiply(&m2).multiply(&m3);
    let result2 = m1.multiply(&m2.multiply(&m3));

    for i in 0..3 {
        for j in 0..3 {
            assert!((result1.data[i][j] - result2.data[i][j]).abs() < EPSILON);
        }
    }
}

#[test]
fn test_matrix_multiplication_identity() {
    let mat = Matrix3x3::rotation_x(PI / 4.0);
    let identity = Matrix3x3::identity();

    let result1 = mat.multiply(&identity);
    let result2 = identity.multiply(&mat);

    for i in 0..3 {
        for j in 0..3 {
            assert!((result1.data[i][j] - mat.data[i][j]).abs() < EPSILON);
            assert!((result2.data[i][j] - mat.data[i][j]).abs() < EPSILON);
        }
    }
}

#[test]
fn test_transpose() {
    let mat = Matrix3x3::new([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]);

    let transposed = mat.transpose();

    for i in 0..3 {
        for j in 0..3 {
            assert_eq!(transposed.data[i][j], mat.data[j][i]);
        }
    }
}

#[test]
fn test_transpose_twice_is_identity() {
    let mat = Matrix3x3::new([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]);

    let twice_transposed = mat.transpose().transpose();

    for i in 0..3 {
        for j in 0..3 {
            assert_eq!(twice_transposed.data[i][j], mat.data[i][j]);
        }
    }
}

#[test]
fn test_determinant_identity() {
    let identity = Matrix3x3::identity();
    assert!((identity.determinant() - 1.0).abs() < EPSILON);
}

#[test]
fn test_determinant_singular_matrix() {
    let singular = Matrix3x3::new([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]);
    assert!(singular.determinant().abs() < EPSILON);
}

#[test]
fn test_determinant_known_value() {
    let mat = Matrix3x3::new([[2.0, 1.0, 3.0], [1.0, 0.0, 1.0], [0.0, 2.0, 4.0]]);
    assert!((mat.determinant() - (-2.0)).abs() < EPSILON);
}

#[test]
fn test_inverse_identity() {
    let identity = Matrix3x3::identity();
    let inverse = identity.inverse().expect("test: expected success");

    for i in 0..3 {
        for j in 0..3 {
            assert!((inverse.data[i][j] - identity.data[i][j]).abs() < EPSILON);
        }
    }
}

#[test]
fn test_inverse_product_is_identity() {
    let mat = Matrix3x3::new([[2.0, 1.0, 3.0], [1.0, 0.0, 1.0], [0.0, 2.0, 4.0]]);

    let inverse = mat.inverse().expect("test: expected success");
    let product = mat.multiply(&inverse);

    for i in 0..3 {
        for j in 0..3 {
            let expected = if i == j { 1.0 } else { 0.0 };
            assert!((product.data[i][j] - expected).abs() < EPSILON);
        }
    }
}

#[test]
fn test_inverse_singular_matrix_error() {
    let singular = Matrix3x3::new([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]);

    let result = singular.inverse();
    assert!(matches!(result, Err(TensorError::SingularMatrix)));
}

#[test]
fn test_orthogonal_rotation_matrix() {
    let rot = Matrix3x3::rotation_z(PI / 4.0);
    assert!(rot.is_orthogonal(EPSILON));
}

#[test]
fn test_non_orthogonal_scaling_matrix() {
    let scale = Matrix3x3::scaling(2.0, 2.0, 2.0);
    assert!(!scale.is_orthogonal(EPSILON));
}

#[test]
fn test_get_element() {
    let mat = Matrix3x3::new([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]);

    assert_eq!(mat.get(0, 0), Some(1.0));
    assert_eq!(mat.get(1, 1), Some(5.0));
    assert_eq!(mat.get(2, 2), Some(9.0));
    assert_eq!(mat.get(3, 0), None);
    assert_eq!(mat.get(0, 3), None);
}

#[test]
fn test_set_element() {
    let mut mat = Matrix3x3::identity();

    mat.set(0, 1, 5.0).expect("test: expected success");
    assert_eq!(mat.data[0][1], 5.0);

    let result = mat.set(3, 0, 1.0);
    assert!(result.is_err());
}

#[test]
fn test_rotation_axis_angle_z_axis() {
    let axis = Vector3D::new(0.0, 0.0, 1.0);
    let angle = PI / 2.0;
    let rot = Matrix3x3::rotation_axis_angle(&axis, angle).expect("test: expected success");

    let vec = Vector3D::new(1.0, 0.0, 0.0);
    let rotated = rot.transform_vector(&vec);

    assert!((rotated.x - 0.0).abs() < EPSILON);
    assert!((rotated.y - 1.0).abs() < EPSILON);
    assert!((rotated.z - 0.0).abs() < EPSILON);
}

#[test]
fn test_rotation_axis_angle_arbitrary() {
    let axis = Vector3D::new(1.0, 1.0, 1.0).normalize().expect("test: creation");
    let angle = 2.0 * PI / 3.0;
    let rot = Matrix3x3::rotation_axis_angle(&axis, angle).expect("test: expected success");

    // Rotating a vector 120 degrees around (1,1,1) axis
    let vec = Vector3D::new(1.0, 0.0, 0.0);
    let rotated = rot.transform_vector(&vec);

    // After 120° rotation around (1,1,1), (1,0,0) -> (0,1,0)
    assert!((rotated.x - 0.0).abs() < 0.001);
    assert!((rotated.y - 1.0).abs() < 0.001);
    assert!((rotated.z - 0.0).abs() < 0.001);
}

#[test]
fn test_rotation_axis_angle_zero_axis() {
    let axis = Vector3D::new(0.0, 0.0, 0.0);
    let result = Matrix3x3::rotation_axis_angle(&axis, PI / 4.0);
    assert!(matches!(result, Err(TensorError::ZeroVector)));
}

#[test]
fn test_matrix_default() {
    let mat = Matrix3x3::default();
    let identity = Matrix3x3::identity();

    for i in 0..3 {
        for j in 0..3 {
            assert_eq!(mat.data[i][j], identity.data[i][j]);
        }
    }
}

#[test]
fn test_matrix_display() {
    let mat = Matrix3x3::identity();
    let display = format!("{mat}");

    assert!(display.contains("1.000"));
    assert!(display.contains("0.000"));
}

// Composition and chaining tests

#[test]
fn test_rotation_composition_euler_angles() {
    let rot_x = Matrix3x3::rotation_x(PI / 6.0);
    let rot_y = Matrix3x3::rotation_y(PI / 4.0);
    let rot_z = Matrix3x3::rotation_z(PI / 3.0);

    let combined = rot_z.multiply(&rot_y).multiply(&rot_x);

    // Test that it's still orthogonal
    assert!(combined.is_orthogonal(1e-9));

    // Test determinant is 1 (rotation preserves orientation)
    assert!((combined.determinant() - 1.0).abs() < EPSILON);
}

#[test]
fn test_scaling_then_rotation() {
    let scale = Matrix3x3::scaling(2.0, 1.0, 1.0);
    let rot = Matrix3x3::rotation_z(PI / 2.0);
    let combined = rot.multiply(&scale);

    let vec = Vector3D::new(1.0, 0.0, 0.0);
    let result = combined.transform_vector(&vec);

    // First scale: (1,0,0) -> (2,0,0)
    // Then rotate: (2,0,0) -> (0,2,0)
    assert!((result.x - 0.0).abs() < EPSILON);
    assert!((result.y - 2.0).abs() < EPSILON);
    assert!((result.z - 0.0).abs() < EPSILON);
}

#[test]
fn test_rotation_preserves_length() {
    let rot =
        Matrix3x3::rotation_axis_angle(&Vector3D::new(1.0, 2.0, 3.0).normalize().expect("test: creation"), 1.234)
            .expect("test: expected success");

    let vec = Vector3D::new(3.0, 4.0, 5.0);
    let rotated = rot.transform_vector(&vec);

    assert!((vec.magnitude() - rotated.magnitude()).abs() < EPSILON);
}
