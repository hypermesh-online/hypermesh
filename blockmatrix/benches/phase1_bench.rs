// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase 1 Performance Benchmarks
//!
//! Comprehensive benchmarks for the Block-MATRIX Foundation covering:
//! - Matrix operations throughput
//! - Blockchain operations at scale
//! - Geospatial conversions
//! - Persistence save/load times
//! - Network propagation latency
//!
//! These benchmarks establish baseline metrics for Phase 2 comparison.

use blockmatrix::integration::{MatrixFoundation, MatrixFoundationConfig};
use blockmatrix::matrix::geospatial::{GpsConverter, GpsCoordinate, ScaleResolution};
use blockmatrix::matrix::tensor::{Matrix3x3, PathFinder, Vector3D};
use blockmatrix::matrix::{find_k_nearest, find_neighbors, find_neighbors_cubic, MatrixCoordinate};
use blockmatrix::StateProof;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::sync::Arc;
use tempfile::TempDir;

// Helper to create test foundation
fn create_bench_foundation() -> (Arc<tokio::runtime::Runtime>, MatrixFoundation, TempDir) {
    let rt = Arc::new(tokio::runtime::Runtime::new().unwrap());
    let temp_dir = TempDir::new().unwrap();
    let config = MatrixFoundationConfig {
        storage_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    };
    let foundation = rt.block_on(async { MatrixFoundation::new(config).await.unwrap() });
    (rt, foundation, temp_dir)
}

// Benchmark 1: Matrix Coordinate Operations
fn bench_matrix_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("matrix_operations");

    // Coordinate creation
    group.bench_function("coordinate_creation", |b| {
        b.iter(|| MatrixCoordinate::new(black_box(100), black_box(200), black_box(300)).unwrap())
    });

    // Distance calculations
    let coord1 = MatrixCoordinate::new(0, 0, 0).unwrap();
    let coord2 = MatrixCoordinate::new(1000, 2000, 3000).unwrap();

    group.bench_function("euclidean_distance", |b| {
        b.iter(|| coord1.euclidean_distance(black_box(&coord2)))
    });

    group.bench_function("manhattan_distance", |b| {
        b.iter(|| coord1.manhattan_distance(black_box(&coord2)))
    });

    group.bench_function("chebyshev_distance", |b| {
        b.iter(|| coord1.chebyshev_distance(black_box(&coord2)))
    });

    // Transformations
    let coord = MatrixCoordinate::new(100, 200, 300).unwrap();

    group.bench_function("translation", |b| {
        b.iter(|| {
            coord
                .translate(black_box(10), black_box(20), black_box(30))
                .unwrap()
        })
    });

    group.bench_function("scaling", |b| b.iter(|| coord.scale(black_box(2)).unwrap()));

    group.bench_function("rotation_x", |b| {
        b.iter(|| coord.rotate_x(black_box(45.0)).unwrap())
    });

    group.finish();
}

// Benchmark 2: Neighbor Discovery
fn bench_neighbor_discovery(c: &mut Criterion) {
    let mut group = c.benchmark_group("neighbor_discovery");

    // Create test coordinates
    let center = MatrixCoordinate::new(500, 500, 500).unwrap();
    let mut candidates = Vec::new();
    for i in 0..1000 {
        candidates.push(MatrixCoordinate::new(i, i, i).unwrap());
    }

    // K-nearest neighbors
    for k in [5, 10, 20, 50].iter() {
        group.bench_with_input(BenchmarkId::new("k_nearest", k), k, |b, &k| {
            b.iter(|| find_k_nearest(black_box(&center), black_box(&candidates), black_box(k)))
        });
    }

    // Radius-based discovery
    for radius in [10.0, 50.0, 100.0, 500.0].iter() {
        group.bench_with_input(BenchmarkId::new("radius", radius), radius, |b, &radius| {
            b.iter(|| {
                find_neighbors(
                    black_box(&center),
                    black_box(&candidates),
                    black_box(radius),
                )
            })
        });
    }

    // Cubic neighbors
    for size in [5, 10, 20, 50].iter() {
        group.bench_with_input(BenchmarkId::new("cubic", size), size, |b, &size| {
            b.iter(|| {
                find_neighbors_cubic(black_box(&center), black_box(&candidates), black_box(size))
            })
        });
    }

    group.finish();
}

// Benchmark 3: Tensor Operations
fn bench_tensor_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("tensor_operations");

    let v1 = Vector3D::new(1.0, 2.0, 3.0);
    let v2 = Vector3D::new(4.0, 5.0, 6.0);
    let matrix = Matrix3x3::identity();

    group.bench_function("vector_add", |b| b.iter(|| v1.add(black_box(&v2))));

    group.bench_function("vector_subtract", |b| {
        b.iter(|| v1.subtract(black_box(&v2)))
    });

    group.bench_function("vector_dot_product", |b| b.iter(|| v1.dot(black_box(&v2))));

    group.bench_function("vector_cross_product", |b| {
        b.iter(|| v1.cross(black_box(&v2)))
    });

    group.bench_function("vector_magnitude", |b| b.iter(|| v1.magnitude()));

    group.bench_function("vector_normalize", |b| b.iter(|| v1.normalize()));

    group.bench_function("matrix_multiply_vector", |b| {
        b.iter(|| matrix.transform_vector(black_box(&v1)))
    });

    group.bench_function("matrix_multiply_matrix", |b| {
        b.iter(|| matrix.multiply(black_box(&matrix)))
    });

    // A* pathfinding using MatrixCoordinate-based PathFinder
    let path_start = MatrixCoordinate::new(0, 0, 0).unwrap();
    let path_goal = MatrixCoordinate::new(10, 10, 0).unwrap();
    let finder = PathFinder::new();

    group.bench_function("astar_pathfinding", |b| {
        let grid_neighbors = |coord: &MatrixCoordinate| -> Vec<MatrixCoordinate> {
            let deltas: [(i64, i64, i64); 4] = [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0)];
            deltas
                .iter()
                .filter_map(|(dx, dy, dz)| {
                    MatrixCoordinate::new(coord.x + dx, coord.y + dy, coord.z + dz).ok()
                })
                .collect()
        };
        b.iter(|| {
            finder.find_path(
                black_box(&path_start),
                black_box(&path_goal),
                grid_neighbors,
            )
        })
    });

    group.finish();
}

// Benchmark 4: Geospatial Conversions
fn bench_geospatial_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("geospatial_operations");

    let origin = GpsCoordinate::new(40.7128, -74.0060, 0.0).unwrap();
    let converter = GpsConverter::with_origin(ScaleResolution::Standard, origin);

    let gps = GpsCoordinate::new(51.5074, -0.1278, 0.0).unwrap();
    let matrix = MatrixCoordinate::new(1000, 2000, 0).unwrap();

    group.bench_function("gps_to_matrix", |b| {
        b.iter(|| converter.gps_to_matrix(black_box(&gps)))
    });

    group.bench_function("matrix_to_gps", |b| {
        b.iter(|| converter.matrix_to_gps(black_box(&matrix)))
    });

    group.finish();
}

// Benchmark 5: Node Creation and Management
fn bench_node_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("node_operations");
    group.sample_size(10); // Reduce sample size for expensive operations

    group.bench_function("add_single_node", |b| {
        let (rt, foundation, _temp_dir) = create_bench_foundation();
        let mut counter = 0;

        b.iter(|| {
            let coord = MatrixCoordinate::new(counter, counter, 0).unwrap();
            rt.block_on(async {
                foundation
                    .add_node(format!("node{counter}"), black_box(coord))
                    .await
                    .unwrap()
            });
            counter += 1;
        })
    });

    group.bench_function("add_100_nodes", |b| {
        b.iter(|| {
            let (rt, foundation, _temp_dir) = create_bench_foundation();
            rt.block_on(async {
                for i in 0..100 {
                    let coord = MatrixCoordinate::new(i, i, 0).unwrap();
                    foundation
                        .add_node(format!("node{i}"), coord)
                        .await
                        .unwrap();
                }
            });
        })
    });

    group.finish();
}

// Benchmark 6: Blockchain Operations
fn bench_blockchain_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("blockchain_operations");
    group.sample_size(10);

    // Setup foundation with nodes
    let (rt, foundation, _temp_dir) = create_bench_foundation();
    rt.block_on(async {
        for i in 0..10 {
            let coord = MatrixCoordinate::new(i * 10, 0, 0).unwrap();
            foundation
                .add_node(format!("node{i}"), coord)
                .await
                .unwrap();
        }
    });

    let proof = StateProof::new_for_testing();

    group.bench_function("add_single_block", |b| {
        let mut counter = 0;
        b.iter(|| {
            let node_id = format!("node{}", counter % 10);
            let data = vec![counter as u8; 1024]; // 1KB block
            rt.block_on(async {
                foundation
                    .add_block(black_box(&node_id), black_box(data), &proof)
                    .await
                    .unwrap()
            });
            counter += 1;
        })
    });

    group.bench_function("add_100_blocks", |b| {
        let proof = StateProof::new_for_testing();
        b.iter(|| {
            let (rt, foundation, _temp_dir) = create_bench_foundation();
            rt.block_on(async {
                // Create nodes
                for i in 0..10 {
                    let coord = MatrixCoordinate::new(i * 10, 0, 0).unwrap();
                    foundation
                        .add_node(format!("node{i}"), coord)
                        .await
                        .unwrap();
                }

                // Add blocks
                for i in 0..100 {
                    let node_id = format!("node{}", i % 10);
                    let data = vec![i as u8; 1024];
                    foundation.add_block(&node_id, data, &proof).await.unwrap();
                }
            });
        })
    });

    group.finish();
}

// Benchmark 7: Network Statistics
fn bench_network_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("network_stats");

    // Create foundations with different node counts
    for node_count in [10, 50, 100, 500].iter() {
        let (rt, foundation, _temp_dir) = create_bench_foundation();
        rt.block_on(async {
            for i in 0..*node_count {
                let coord = MatrixCoordinate::new(i, i, 0).unwrap();
                foundation
                    .add_node(format!("node{i}"), coord)
                    .await
                    .unwrap();
            }
        });

        group.bench_with_input(
            BenchmarkId::new("get_stats", node_count),
            node_count,
            |b, _| b.iter(|| rt.block_on(async { foundation.get_network_stats().await })),
        );

        group.bench_with_input(
            BenchmarkId::new("find_k_nearest", node_count),
            node_count,
            |b, _| {
                let center = MatrixCoordinate::new(250, 250, 0).unwrap();
                b.iter(|| {
                    rt.block_on(async {
                        foundation
                            .find_k_nearest_nodes(black_box(&center), black_box(10))
                            .await
                    })
                })
            },
        );
    }

    group.finish();
}

// Benchmark 8: Persistence Operations
fn bench_persistence_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("persistence_operations");
    group.sample_size(10);

    // Test with different network sizes
    let proof = StateProof::new_for_testing();

    for node_count in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("save_network_state", node_count),
            node_count,
            |b, &node_count| {
                b.iter(|| {
                    let (rt, foundation, _temp_dir) = create_bench_foundation();
                    rt.block_on(async {
                        // Create nodes
                        for i in 0..node_count {
                            let coord = MatrixCoordinate::new(i, i, 0).unwrap();
                            foundation
                                .add_node(format!("node{i}"), coord)
                                .await
                                .unwrap();
                        }

                        // Add blocks to each node
                        for i in 0..node_count {
                            let node_id = format!("node{i}");
                            let data = vec![i as u8; 1024];
                            foundation.add_block(&node_id, data, &proof).await.unwrap();
                        }

                        // Save state
                        foundation.save_network_state().await.unwrap()
                    })
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_matrix_operations,
    bench_neighbor_discovery,
    bench_tensor_operations,
    bench_geospatial_operations,
    bench_node_operations,
    bench_blockchain_operations,
    bench_network_stats,
    bench_persistence_operations,
);

criterion_main!(benches);
