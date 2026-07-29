// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Performance benchmarks for tensor operations

use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::matrix::tensor::{calculate_routing_vector, Vector3D};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

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

criterion_group!(benches, bench_vector_operations, bench_routing_operations);
criterion_main!(benches);
