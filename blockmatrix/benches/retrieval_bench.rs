// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Performance benchmarks for instruction-based retrieval system

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use blockmatrix::retrieval::{
    InstructionTransmitter, CompressionFormat, RetrievalPlan,
    CompleteShardMap, ShardMapEntry, ShardLocation, RetrievalMetadata,
};
use blockmatrix::matrix::MatrixCoordinate;

fn create_test_plan(shard_count: usize, replicas_per_shard: usize) -> RetrievalPlan {
    let content_hash = [1u8; 32];
    let mut shard_map = CompleteShardMap::new();

    for i in 0..shard_count {
        let shard_hash = [(i % 256) as u8; 32];
        let locations: Vec<ShardLocation> = (0..replicas_per_shard)
            .map(|r| {
                ShardLocation::new(
                    MatrixCoordinate::new(i as i64, r as i64, 0).unwrap(),
                    0.9,
                )
            })
            .collect();

        let entry = ShardMapEntry::new(shard_hash, locations);
        shard_map.add_entry(entry);
    }

    let data_shards = shard_count * 10 / 14;
    let parity_shards = shard_count - data_shards;

    let metadata = RetrievalMetadata {
        erasure_coding: (data_shards, parity_shards),
        compression: "brotli".to_string(),
        encryption: "aes-256-gcm".to_string(),
        content_type: "application/octet-stream".to_string(),
        created_at: chrono::Utc::now().timestamp(),
    };

    RetrievalPlan::new(content_hash, shard_map, metadata)
}

fn bench_instruction_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("instruction_generation");

    for shard_count in [14, 28, 56, 112].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(shard_count),
            shard_count,
            |b, &shard_count| {
                let plan = create_test_plan(shard_count, 3);
                b.iter(|| {
                    black_box(&plan);
                });
            },
        );
    }

    group.finish();
}

fn bench_instruction_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("instruction_encoding");

    let plan = create_test_plan(14, 3);

    for format in [
        CompressionFormat::None,
        CompressionFormat::Brotli,
        CompressionFormat::Zstd,
        CompressionFormat::MessagePack,
    ].iter() {
        group.bench_with_input(
            BenchmarkId::new("format", format!("{:?}", format)),
            format,
            |b, format| {
                let transmitter = InstructionTransmitter::new(*format);
                b.iter(|| {
                    let _ = transmitter.encode(black_box(&plan));
                });
            },
        );
    }

    group.finish();
}

fn bench_instruction_decoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("instruction_decoding");

    let plan = create_test_plan(14, 3);

    for format in [
        CompressionFormat::None,
        CompressionFormat::Brotli,
        CompressionFormat::Zstd,
        CompressionFormat::MessagePack,
    ].iter() {
        let transmitter = InstructionTransmitter::new(*format);
        let encoded = transmitter.encode(&plan).unwrap();

        group.bench_with_input(
            BenchmarkId::new("format", format!("{:?}", format)),
            format,
            |b, format| {
                let transmitter = InstructionTransmitter::new(*format);
                b.iter(|| {
                    let _ = transmitter.decode(black_box(&encoded));
                });
            },
        );
    }

    group.finish();
}

fn bench_position_optimization(c: &mut Criterion) {
    let mut group = c.benchmark_group("position_optimization");

    for shard_count in [14, 28, 56].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(shard_count),
            shard_count,
            |b, &shard_count| {
                let mut plan = create_test_plan(shard_count, 3);
                let client_pos = MatrixCoordinate::new(0, 0, 0).unwrap();

                b.iter(|| {
                    plan.optimize_for_position(black_box(&client_pos));
                });
            },
        );
    }

    group.finish();
}

fn bench_instruction_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("instruction_size");

    let transmitter = InstructionTransmitter::new(CompressionFormat::Brotli);

    for shard_count in [14, 28, 56, 112, 224].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(shard_count),
            shard_count,
            |b, &shard_count| {
                let plan = create_test_plan(shard_count, 3);
                b.iter(|| {
                    let encoded = transmitter.encode(black_box(&plan)).unwrap();
                    black_box(encoded.len());
                });
            },
        );
    }

    group.finish();
}

fn bench_compression_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_ratio");

    let plan = create_test_plan(14, 3);

    for format in [
        CompressionFormat::Brotli,
        CompressionFormat::Zstd,
        CompressionFormat::MessagePack,
    ].iter() {
        group.bench_with_input(
            BenchmarkId::new("format", format!("{:?}", format)),
            format,
            |b, format| {
                let transmitter = InstructionTransmitter::new(*format);
                b.iter(|| {
                    let (encoded, _) = transmitter.encode_with_stats(black_box(&plan)).unwrap();
                    black_box(encoded.len());
                });
            },
        );
    }

    group.finish();
}

fn bench_shard_map_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("shard_map_operations");

    let plan = create_test_plan(56, 3);

    group.bench_function("estimate_size", |b| {
        b.iter(|| {
            black_box(plan.shard_map.estimate_size());
        });
    });

    group.bench_function("find_entry", |b| {
        let target_hash = [5u8; 32];
        b.iter(|| {
            black_box(plan.shard_map.find_entry(&target_hash));
        });
    });

    group.bench_function("get_weak_shards", |b| {
        b.iter(|| {
            black_box(plan.shard_map.get_weak_shards(2));
        });
    });

    group.finish();
}

fn bench_replica_selection(c: &mut Criterion) {
    use blockmatrix::retrieval::{FallbackManager, SelectionCriteria, FallbackStrategy};

    let mut group = c.benchmark_group("replica_selection");

    let manager = FallbackManager::new(
        SelectionCriteria::default(),
        FallbackStrategy::Adaptive,
    );

    let shard_hash = [1u8; 32];
    let locations: Vec<ShardLocation> = (0..10)
        .map(|i| {
            let mut loc = ShardLocation::new(
                MatrixCoordinate::new(i, 0, 0).unwrap(),
                0.8 + (i as f64 * 0.02),
            );
            loc.estimated_latency_ms = i as u64 * 10;
            loc
        })
        .collect();

    let entry = ShardMapEntry::new(shard_hash, locations);

    group.bench_function("select_best_3", |b| {
        b.iter(|| {
            black_box(manager.get_alternatives(&entry, 3));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_instruction_generation,
    bench_instruction_encoding,
    bench_instruction_decoding,
    bench_position_optimization,
    bench_instruction_size,
    bench_compression_ratio,
    bench_shard_map_operations,
    bench_replica_selection,
);

criterion_main!(benches);
