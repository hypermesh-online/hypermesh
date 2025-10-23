//! End-to-End Latency Benchmarks
//!
//! Specialized benchmarks for measuring and validating end-to-end latency
//! performance across the complete MFN 4-layer system.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use hdrhistogram::Histogram;

use mfn_integration_tests::*;

/// Benchmark latency distribution for end-to-end processing
fn benchmark_latency_distribution(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = rt.block_on(async { MfnIntegrationEngine::new() });
    
    let mut group = c.benchmark_group("latency_distribution");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(1000);
    
    group.bench_function("end_to_end_latency_p99", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let mut histogram = Histogram::<u64>::new(3).unwrap();
            
            for i in 0..iters {
                let flow_key = FlowKey {
                    source_ip: format!("latency.test.{}", i % 1000),
                    dest_ip: format!("latency.dest.{}", i % 1000),
                    source_port: (8000 + (i % 1000)) as u16,
                    dest_port: 443,
                    protocol: "TCP".to_string(),
                };
                
                let context = ContextVector {
                    features: vec![
                        ((i % 10) as f64) / 10.0,
                        ((i % 20) as f64) / 20.0,
                        0.5,
                        0.7,
                        0.9
                    ],
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    metadata: HashMap::new(),
                };
                
                let start = Instant::now();
                let _result = engine
                    .process_flow(flow_key, vec![context])
                    .await
                    .unwrap();
                let latency = start.elapsed();
                
                histogram.record(latency.as_micros() as u64).unwrap();
            }
            
            // Return P99 latency for criterion to track
            Duration::from_micros(histogram.value_at_quantile(0.99))
        })
    });
    
    group.finish();
}

/// Benchmark latency under different load conditions
fn benchmark_latency_under_load(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = rt.block_on(async { std::sync::Arc::new(MfnIntegrationEngine::new()) });
    
    let mut group = c.benchmark_group("latency_under_load");
    
    for concurrent_load in [1, 10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_load", concurrent_load),
            concurrent_load,
            |b, &concurrent_load| {
                b.to_async(&rt).iter(|| async {
                    let engine = engine.clone();
                    let mut handles = Vec::new();
                    let start = Instant::now();
                    
                    for i in 0..concurrent_load {
                        let engine = engine.clone();
                        let handle = tokio::spawn(async move {
                            let flow_key = FlowKey {
                                source_ip: format!("load.test.{}", i),
                                dest_ip: format!("load.dest.{}", i),
                                source_port: (9000 + i % 1000) as u16,
                                dest_port: 443,
                                protocol: "TCP".to_string(),
                            };
                            
                            let context = ContextVector {
                                features: vec![0.6, 0.7, 0.8, 0.5, 0.4],
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                                metadata: HashMap::new(),
                            };
                            
                            let flow_start = Instant::now();
                            let result = engine.process_flow(flow_key, vec![context]).await.unwrap();
                            let flow_latency = flow_start.elapsed();
                            
                            (result, flow_latency)
                        });
                        
                        handles.push(handle);
                    }
                    
                    let results: Vec<_> = futures::future::join_all(handles).await
                        .into_iter()
                        .map(|r| r.unwrap())
                        .collect();
                    
                    let total_time = start.elapsed();
                    
                    black_box((results, total_time))
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark worst-case latency scenarios
fn benchmark_worst_case_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = rt.block_on(async { MfnIntegrationEngine::new() });
    
    let mut group = c.benchmark_group("worst_case_latency");
    
    // Large context vectors
    group.bench_function("large_context_vectors", |b| {
        b.to_async(&rt).iter(|| async {
            let flow_key = FlowKey {
                source_ip: "worst.case.large".to_string(),
                dest_ip: "worst.dest.large".to_string(),
                source_port: 8080,
                dest_port: 443,
                protocol: "TCP".to_string(),
            };
            
            // Very large feature vector
            let large_features: Vec<f64> = (0..10000)
                .map(|i| (i as f64).sin() / 10000.0)
                .collect();
            
            let context = ContextVector {
                features: large_features,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                metadata: {
                    let mut meta = HashMap::new();
                    // Large metadata
                    for i in 0..100 {
                        meta.insert(format!("key_{}", i), format!("value_{}", i));
                    }
                    meta
                },
            };
            
            let result = engine
                .process_flow(black_box(flow_key), black_box(vec![context]))
                .await
                .unwrap();
            
            black_box(result)
        })
    });
    
    // Cold cache scenario
    group.bench_function("cold_cache", |b| {
        b.to_async(&rt).iter(|| async {
            // Generate unique flow key every time to avoid cache hits
            let flow_key = FlowKey {
                source_ip: format!("cold.cache.{}", rand::random::<u64>()),
                dest_ip: format!("cold.dest.{}", rand::random::<u64>()),
                source_port: rand::random::<u16>(),
                dest_port: 443,
                protocol: "TCP".to_string(),
            };
            
            let context = ContextVector {
                features: vec![0.1, 0.2, 0.3, 0.4, 0.5],
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                metadata: HashMap::new(),
            };
            
            let result = engine
                .process_flow(black_box(flow_key), black_box(vec![context]))
                .await
                .unwrap();
            
            black_box(result)
        })
    });
    
    // Complex routing scenario
    group.bench_function("complex_routing", |b| {
        b.to_async(&rt).iter(|| async {
            let flow_key = FlowKey {
                source_ip: "complex.routing".to_string(),
                dest_ip: "complex.dest".to_string(),
                source_port: 8080,
                dest_port: 443,
                protocol: "TCP".to_string(),
            };
            
            // Complex context that should trigger detailed analysis
            let context = ContextVector {
                features: vec![0.99, 0.01, 0.99, 0.01, 0.99], // High variance
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                metadata: HashMap::new(),
            };
            
            let result = engine
                .process_flow(black_box(flow_key), black_box(vec![context]))
                .await
                .unwrap();
            
            black_box(result)
        })
    });
    
    group.finish();
}

/// Benchmark best-case latency scenarios
fn benchmark_best_case_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = rt.block_on(async { MfnIntegrationEngine::new() });
    
    // Prime caches for best-case scenario
    let warm_flow_key = FlowKey {
        source_ip: "warm.cache.key".to_string(),
        dest_ip: "warm.cache.dest".to_string(),
        source_port: 8080,
        dest_port: 443,
        protocol: "TCP".to_string(),
    };
    
    let warm_context = ContextVector {
        features: vec![0.5, 0.5, 0.5, 0.5, 0.5],
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        metadata: HashMap::new(),
    };
    
    // Warm up caches
    rt.block_on(async {
        for _ in 0..10 {
            let _ = engine
                .process_flow(warm_flow_key.clone(), vec![warm_context.clone()])
                .await;
        }
    });
    
    let mut group = c.benchmark_group("best_case_latency");
    
    // Warm cache scenario
    group.bench_function("warm_cache", |b| {
        b.to_async(&rt).iter(|| async {
            let result = engine
                .process_flow(black_box(warm_flow_key.clone()), black_box(vec![warm_context.clone()]))
                .await
                .unwrap();
            
            black_box(result)
        })
    });
    
    // Small context vectors
    group.bench_function("small_context_vectors", |b| {
        b.to_async(&rt).iter(|| async {
            let flow_key = FlowKey {
                source_ip: "small.context".to_string(),
                dest_ip: "small.dest".to_string(),
                source_port: 8080,
                dest_port: 443,
                protocol: "TCP".to_string(),
            };
            
            let context = ContextVector {
                features: vec![0.5], // Minimal feature vector
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                metadata: HashMap::new(),
            };
            
            let result = engine
                .process_flow(black_box(flow_key), black_box(vec![context]))
                .await
                .unwrap();
            
            black_box(result)
        })
    });
    
    group.finish();
}

/// Benchmark latency percentiles validation
fn benchmark_latency_percentiles(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = rt.block_on(async { MfnIntegrationEngine::new() });
    
    let mut group = c.benchmark_group("latency_percentiles");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(2000);
    
    group.bench_function("percentile_validation", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let mut latencies = Vec::new();
            
            for i in 0..iters {
                let flow_key = FlowKey {
                    source_ip: format!("percentile.{}", i % 100),
                    dest_ip: format!("percentile.dest.{}", i % 100),
                    source_port: (8000 + (i % 1000)) as u16,
                    dest_port: 443,
                    protocol: "TCP".to_string(),
                };
                
                let context = ContextVector {
                    features: vec![
                        ((i % 5) as f64) / 5.0,
                        ((i % 7) as f64) / 7.0,
                        ((i % 11) as f64) / 11.0,
                        0.5,
                        0.7
                    ],
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    metadata: HashMap::new(),
                };
                
                let start = Instant::now();
                let _result = engine
                    .process_flow(flow_key, vec![context])
                    .await
                    .unwrap();
                let latency = start.elapsed();
                
                latencies.push(latency);
            }
            
            // Sort latencies for percentile calculation
            latencies.sort();
            
            let p50_index = (latencies.len() as f64 * 0.50) as usize;
            let p95_index = (latencies.len() as f64 * 0.95) as usize;
            let p99_index = (latencies.len() as f64 * 0.99) as usize;
            
            let p50 = latencies[p50_index];
            let p95 = latencies[p95_index];
            let p99 = latencies[p99_index];
            
            // Return average of key percentiles for tracking
            Duration::from_nanos((p50.as_nanos() + p95.as_nanos() + p99.as_nanos()) / 3)
        })
    });
    
    group.finish();
}

criterion_group!(
    latency_benches,
    benchmark_latency_distribution,
    benchmark_latency_under_load,
    benchmark_worst_case_latency,
    benchmark_best_case_latency,
    benchmark_latency_percentiles
);

criterion_main!(latency_benches);
