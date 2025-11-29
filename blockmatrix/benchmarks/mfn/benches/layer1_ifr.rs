/*!
# Layer 1 (IFR) Criterion Benchmarks

High-precision benchmarks for the Immediate Flow Registry layer using criterion.rs
for detailed statistical analysis and performance regression detection.
*/

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mfn_benchmarks::{common::*, layer1::*};
use std::time::Duration;

fn bench_exact_matcher_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("exact_matcher_lookup");
    
    // Test different table sizes and load factors
    for table_size in [1024, 10240, 102400, 1048576].iter() {
        for load_factor in [0.25, 0.5, 0.75].iter() {
            let mut hash_table = RobinHoodHashTable::new(*table_size);
            let record_count = (*table_size as f64 * load_factor) as usize;
            
            // Pre-populate hash table
            let records = generate_test_flows(record_count);
            for record in &records {
                hash_table.insert(record.clone());
            }
            
            let test_keys: Vec<_> = records.iter().take(100).map(|r| r.key).collect();
            
            group.throughput(Throughput::Elements(test_keys.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("robin_hood_lookup", format!("{}k_{}%", table_size / 1024, (load_factor * 100.0) as u32)),
                &(&hash_table, &test_keys),
                |b, (table, keys)| {
                    b.iter(|| {
                        for key in keys.iter() {
                            black_box(table.lookup(key));
                        }
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_exact_matcher_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("exact_matcher_insert");
    group.measurement_time(Duration::from_secs(10));
    
    for table_size in [1024, 10240, 102400].iter() {
        let records = generate_test_flows(1000);
        
        group.throughput(Throughput::Elements(records.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("robin_hood_insert", format!("{}k", table_size / 1024)),
            &(*table_size, &records),
            |b, (size, records)| {
                b.iter_batched(
                    || RobinHoodHashTable::new(*size),
                    |mut table| {
                        for record in records {
                            black_box(table.insert(record.clone()));
                        }
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }
    
    group.finish();
}

fn bench_bloom_filter_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_filter_operations");
    
    // Test different filter configurations
    for filter_count in [4, 8, 16].iter() {
        for bits_per_filter in [65536, 131072, 262144].iter() {
            let mut bloom_bank = BloomFilterBank::new(*filter_count, *bits_per_filter, 3);
            
            // Pre-populate with some data
            let test_keys = generate_test_flow_keys(1000);
            for key in &test_keys[..500] {
                bloom_bank.add(key);
            }
            
            group.throughput(Throughput::Elements(500));
            group.bench_with_input(
                BenchmarkId::new("bloom_lookup", format!("{}filters_{}kb", filter_count, bits_per_filter / 1024)),
                &(&bloom_bank, &test_keys[500..]),
                |b, (bank, keys)| {
                    b.iter(|| {
                        for key in keys.iter() {
                            black_box(bank.might_contain(key));
                        }
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_unix_socket_ipc(c: &mut Criterion) {
    let mut group = c.benchmark_group("unix_socket_ipc");
    group.measurement_time(Duration::from_secs(15));
    
    // This would benchmark Unix socket operations in a real implementation
    // For now, we simulate the latency characteristics
    
    let message_sizes = [64, 256, 1024, 4096];
    
    for size in message_sizes.iter() {
        let test_data = vec![0u8; *size];
        
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("unix_socket_echo", format!("{}bytes", size)),
            &test_data,
            |b, data| {
                b.iter(|| {
                    // Simulate Unix socket round-trip latency
                    std::thread::sleep(Duration::from_nanos(50000)); // 50µs target
                    black_box(data.clone())
                })
            },
        );
    }
    
    group.finish();
}

fn bench_flow_cache_operations(c: &mut Criterion) {
    use std::collections::HashMap;
    use ahash::AHashMap;
    
    let mut group = c.benchmark_group("flow_cache_operations");
    
    // Compare different cache implementations
    let cache_sizes = [1000, 10000, 100000];
    let test_keys = generate_test_flow_keys(1000);
    
    for &cache_size in cache_sizes.iter() {
        // Test AHashMap (our current choice)
        let mut ahash_cache = AHashMap::new();
        for (i, key) in test_keys.iter().enumerate() {
            if i >= cache_size { break; }
            let record = FlowRecord {
                key: *key,
                component_id: i as u32,
                timestamp: i as u64,
                metadata: [0; 8],
            };
            ahash_cache.insert(*key, record);
        }
        
        group.throughput(Throughput::Elements(100));
        group.bench_with_input(
            BenchmarkId::new("ahash_lookup", cache_size),
            &(&ahash_cache, &test_keys[..100]),
            |b, (cache, keys)| {
                b.iter(|| {
                    for key in keys.iter() {
                        black_box(cache.get(key));
                    }
                })
            },
        );
        
        // Compare with std::HashMap
        let mut std_cache = HashMap::new();
        for (i, key) in test_keys.iter().enumerate() {
            if i >= cache_size { break; }
            let record = FlowRecord {
                key: *key,
                component_id: i as u32,
                timestamp: i as u64,
                metadata: [0; 8],
            };
            std_cache.insert(*key, record);
        }
        
        group.bench_with_input(
            BenchmarkId::new("std_hashmap_lookup", cache_size),
            &(&std_cache, &test_keys[..100]),
            |b, (cache, keys)| {
                b.iter(|| {
                    for key in keys.iter() {
                        black_box(cache.get(key));
                    }
                })
            },
        );
    }
    
    group.finish();
}

fn bench_integrated_ifr_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("integrated_ifr_lookup");
    group.measurement_time(Duration::from_secs(20));
    
    // Benchmark the complete IFR lookup pipeline
    let table_size = 1048576; // 1M entries
    let mut hash_table = RobinHoodHashTable::new(table_size);
    let mut bloom_bank = BloomFilterBank::new(16, 131072, 3);
    let mut cache = ahash::AHashMap::new();
    
    // Pre-populate all components
    let records = generate_test_flows(100000);
    for record in &records {
        hash_table.insert(record.clone());
        bloom_bank.add(&record.key);
        if cache.len() < 10000 {
            cache.insert(record.key, record.clone());
        }
    }
    
    let test_keys: Vec<_> = records.iter().take(1000).map(|r| r.key).collect();
    
    group.throughput(Throughput::Elements(test_keys.len() as u64));
    group.bench_function("full_pipeline", |b| {
        b.iter(|| {
            for key in &test_keys {
                let key = black_box(*key);
                
                // Simulate full IFR lookup pipeline
                let result = if let Some(cached) = cache.get(&key) {
                    black_box(Some(cached.clone()))
                } else if bloom_bank.might_contain(&key) {
                    black_box(hash_table.lookup(&key).cloned())
                } else {
                    black_box(None)
                };
                
                black_box(result);
            }
        })
    });
    
    group.finish();
}

fn bench_component_coordination(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_coordination");
    
    // Benchmark component discovery and message broadcasting
    let component_count = [5, 10, 20];
    
    for &count in component_count.iter() {
        group.bench_with_input(
            BenchmarkId::new("component_broadcast", count),
            &count,
            |b, &component_count| {
                b.iter(|| {
                    // Simulate broadcasting to N components
                    for i in 0..component_count {
                        // Simulate message serialization and socket write
                        let message = format!("flow_update_{}", i);
                        black_box(message.as_bytes());
                        
                        // Simulate Unix socket write latency (10µs per component)
                        std::thread::sleep(Duration::from_nanos(10000));
                    }
                })
            },
        );
    }
    
    group.finish();
}

fn bench_metrics_collection(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics_collection");
    
    // Benchmark metrics collection and aggregation
    let operation_counts = [1000, 10000, 100000];
    
    for &op_count in operation_counts.iter() {
        // Generate sample latency data
        let latencies: Vec<Duration> = (0..op_count)
            .map(|i| Duration::from_nanos(50000 + (i % 10000) as u64))
            .collect();
        
        group.throughput(Throughput::Elements(op_count as u64));
        group.bench_with_input(
            BenchmarkId::new("metrics_aggregation", op_count),
            &latencies,
            |b, latencies| {
                b.iter(|| {
                    // Simulate metrics calculation
                    let mut sorted = latencies.clone();
                    sorted.sort();
                    
                    let count = sorted.len();
                    let p95 = black_box(sorted[(count as f64 * 0.95) as usize]);
                    let mean = black_box(Duration::from_nanos(
                        sorted.iter().map(|d| d.as_nanos()).sum::<u128>() / count as u128
                    ) as u64);
                    
                    black_box((p95, mean));
                })
            },
        );
    }
    
    group.finish();
}

// Helper functions for benchmark data generation
fn generate_test_flows(count: usize) -> Vec<FlowRecord> {
    (0..count)
        .map(|i| {
            let mut key = [0u8; 32];
            let i_bytes = (i as u64).to_le_bytes();
            key[..8].copy_from_slice(&i_bytes);
            
            // Add some randomness
            for j in 8..32 {
                key[j] = fastrand::u8(0..=255);
            }
            
            FlowRecord {
                key,
                component_id: fastrand::u32(1000..9999),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64,
                metadata: [0; 8],
            }
        })
        .collect()
}

fn generate_test_flow_keys(count: usize) -> Vec<[u8; 32]> {
    (0..count)
        .map(|i| {
            let mut key = [0u8; 32];
            let i_bytes = (i as u64).to_le_bytes();
            key[..8].copy_from_slice(&i_bytes);
            
            for j in 8..32 {
                key[j] = fastrand::u8(0..=255);
            }
            
            key
        })
        .collect()
}

criterion_group!(
    ifr_benchmarks,
    bench_exact_matcher_lookup,
    bench_exact_matcher_insert,
    bench_bloom_filter_operations,
    bench_unix_socket_ipc,
    bench_flow_cache_operations,
    bench_integrated_ifr_lookup,
    bench_component_coordination,
    bench_metrics_collection
);

criterion_main!(ifr_benchmarks);