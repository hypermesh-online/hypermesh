// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Performance benchmarks for tensor operations

use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::matrix::tensor::{calculate_routing_vector, Matrix3x3, PathFinder, Vector3D};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn bench_vector_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_ops");

    // Benchmark vector magnitude
    group.bench_function("magnitude", |b| {
        #[allow(clippy::approx_constant)]
        let vec = Vector3D::new(3.14, 2.71, 1.41);
        b.iter(|| black_box(vec.magnitude()))
    });

    // Benchmark normalization
    group.bench_function("normalize", |b| {
        #[allow(clippy::approx_constant)]
        let vec = Vector3D::new(3.14, 2.71, 1.41);
        b.iter(|| black_box(vec.normalize()))
    });

    // Benchmark dot product
    group.bench_function("dot_product", |b| {
        let vec1 = Vector3D::new(1.0, 2.0, 3.0);
        let vec2 = Vector3D::new(4.0, 5.0, 6.0);
        b.iter(|| black_box(vec1.dot(&vec2)))
    });

    // Benchmark cross product
    group.bench_function("cross_product", |b| {
        let vec1 = Vector3D::new(1.0, 2.0, 3.0);
        let vec2 = Vector3D::new(4.0, 5.0, 6.0);
        b.iter(|| black_box(vec1.cross(&vec2)))
    });

    group.finish();
}

fn bench_matrix_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("matrix_ops");

    // Benchmark matrix multiplication
    group.bench_function("multiply", |b| {
        let mat1 = Matrix3x3::rotation_x(0.5);
        let mat2 = Matrix3x3::rotation_y(0.7);
        b.iter(|| black_box(mat1.multiply(&mat2)))
    });

    // Benchmark vector transformation
    group.bench_function("transform_vector", |b| {
        let mat = Matrix3x3::rotation_z(0.5);
        let vec = Vector3D::new(1.0, 2.0, 3.0);
        b.iter(|| black_box(mat.transform_vector(&vec)))
    });

    // Benchmark determinant
    group.bench_function("determinant", |b| {
        let mat = Matrix3x3::rotation_x(0.5);
        b.iter(|| black_box(mat.determinant()))
    });

    // Benchmark inverse
    group.bench_function("inverse", |b| {
        let mat = Matrix3x3::rotation_x(0.5);
        b.iter(|| black_box(mat.inverse()))
    });

    group.finish();
}

fn bench_routing_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("routing_ops");

    // Benchmark routing vector calculation
    group.bench_function("calculate_routing_vector", |b| {
        let source = MatrixCoordinate::new(0, 0, 0).unwrap();
        let dest = MatrixCoordinate::new(100, 50, 25).unwrap();
        b.iter(|| black_box(calculate_routing_vector(&source, &dest)))
    });

    group.finish();
}

fn bench_pathfinding(c: &mut Criterion) {
    let mut group = c.benchmark_group("pathfinding");

    // Helper function for grid neighbors
    fn grid_neighbors(coord: &MatrixCoordinate) -> Vec<MatrixCoordinate> {
        let mut neighbors = Vec::new();
        let offsets = [
            (-1, 0, 0),
            (1, 0, 0),
            (0, -1, 0),
            (0, 1, 0),
            (0, 0, -1),
            (0, 0, 1),
        ];

        for (dx, dy, dz) in offsets.iter() {
            if let Ok(neighbor) = MatrixCoordinate::new(coord.x + dx, coord.y + dy, coord.z + dz) {
                neighbors.push(neighbor);
            }
        }
        neighbors
    }

    // Benchmark A* pathfinding with different distances
    for distance in [5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(distance),
            distance,
            |b, &dist| {
                let finder = PathFinder::new();
                let start = MatrixCoordinate::new(0, 0, 0).unwrap();
                let goal = MatrixCoordinate::new(dist, dist, 0).unwrap();
                b.iter(|| black_box(finder.find_path(&start, &goal, grid_neighbors)))
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_vector_operations,
    bench_matrix_operations,
    bench_routing_operations,
    bench_pathfinding
);
criterion_main!(benches);
