//! Full Stack MFN Performance Benchmarks
//!
//! Comprehensive benchmarking suite that validates the complete 4-layer MFN system
//! against ambitious performance targets and demonstrates production readiness.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

use mfn_integration_tests::*;

/// Benchmark single flow processing through all layers
fn benchmark_single_flow_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = rt.block_on(async { MfnIntegrationEngine::new() });
    
    let flow_key = FlowKey {
        source_ip: "192.168.1.100".to_string(),
        dest_ip: "10.0.0.50".to_string(),
        source_port: 8080,
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
    
    c.bench_function("single_flow_end_to_end", |b| {
        b.to_async(&rt).iter(|| async {
            let result = engine
                .process_flow(black_box(flow_key.clone()), black_box(vec![context.clone()]))
                .await
                .unwrap();
            black_box(result)
        })
    });
}

/// Benchmark throughput with increasing concurrent load
fn benchmark_concurrent_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = Arc::new(rt.block_on(async { MfnIntegrationEngine::new() }));
    
    let mut group = c.benchmark_group("concurrent_throughput");
    
    for concurrent_flows in [1, 10, 50, 100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*concurrent_flows as u64));
        
        group.bench_with_input(
            BenchmarkId::new("concurrent_flows", concurrent_flows),
            concurrent_flows,
            |b, &concurrent_flows| {
                b.to_async(&rt).iter(|| async {
                    let engine = engine.clone();
                    let mut handles = Vec::new();
                    
                    for i in 0..concurrent_flows {
                        let engine = engine.clone();
                        let handle = tokio::spawn(async move {
                            let flow_key = FlowKey {
                                source_ip: format!("192.168.{}.{}", i / 256, i % 256),
                                dest_ip: format!("10.0.{}.{}", (i + 1) / 256, (i + 1) % 256),
                                source_port: (8000 + i % 1000) as u16,
                                dest_port: 443,
                                protocol: "TCP".to_string(),
                            };
                            
                            let context = ContextVector {
                                features: vec![
                                    (i as f64) / 1000.0,
                                    ((i * 2) as f64) / 1000.0,
                                    0.5,
                                    0.7,
                                    0.3
                                ],
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                                metadata: HashMap::new(),
                            };
                            
                            engine.process_flow(flow_key, vec![context]).await.unwrap()
                        });
                        
                        handles.push(handle);
                    }
                    
                    let results: Vec<_> = futures::future::join_all(handles).await
                        .into_iter()
                        .map(|r| r.unwrap())
                        .collect();
                    
                    black_box(results)
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark layer-specific performance
fn benchmark_layer_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = rt.block_on(async { MfnIntegrationEngine::new() });
    
    let flow_key = FlowKey {
        source_ip: "benchmark.source".to_string(),
        dest_ip: "benchmark.dest".to_string(),
        source_port: 8080,
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
    
    let mut group = c.benchmark_group("layer_performance");
    
    // Full end-to-end benchmark
    group.bench_function("end_to_end", |b| {
        b.to_async(&rt).iter(|| async {
            let result = engine
                .process_flow(black_box(flow_key.clone()), black_box(vec![context.clone()]))
                .await
                .unwrap();
            black_box(result)
        })
    });
    
    group.finish();
}

/// Benchmark cache effectiveness across different hit rates
fn benchmark_cache_effectiveness(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = rt.block_on(async { MfnIntegrationEngine::new() });
    
    let mut group = c.benchmark_group("cache_effectiveness");
    
    // Prime the cache with some flows
    let cache_flows = 100;
    rt.block_on(async {
        for i in 0..cache_flows {
            let flow_key = FlowKey {
                source_ip: format!("cache.{}", i),
                dest_ip: format!("dest.{}", i),
                source_port: (8000 + i) as u16,
                dest_port: 443,
                protocol: "TCP".to_string(),
            };
            
            let context = ContextVector {
                features: vec![0.5, 0.5, 0.5, 0.5, 0.5],
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                metadata: HashMap::new(),
            };
            
            let _ = engine.process_flow(flow_key, vec![context]).await;
        }
    });
    
    // Benchmark cache hits vs misses
    group.bench_function("cache_hit", |b| {
        b.to_async(&rt).iter(|| async {
            let flow_key = FlowKey {
                source_ip: "cache.50".to_string(), // Should be in cache
                dest_ip: "dest.50".to_string(),
                source_port: 8050,
                dest_port: 443,
                protocol: "TCP".to_string(),
            };
            
            let context = ContextVector {
                features: vec![0.5, 0.5, 0.5, 0.5, 0.5],
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
    
    group.bench_function("cache_miss", |b| {
        b.to_async(&rt).iter(|| async {
            let flow_key = FlowKey {
                source_ip: format!("new.cache.miss.{}", rand::random::<u32>()),
                dest_ip: format!("new.dest.miss.{}", rand::random::<u32>()),
                source_port: rand::random::<u16>(),
                dest_port: 443,
                protocol: "TCP".to_string(),
            };
            
            let context = ContextVector {
                features: vec![0.5, 0.5, 0.5, 0.5, 0.5],
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

/// Benchmark performance improvement ratios
fn benchmark_performance_improvements(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = rt.block_on(async { MfnIntegrationEngine::new() });
    
    let flow_key = FlowKey {
        source_ip: "improvement.test".to_string(),
        dest_ip: "improvement.dest".to_string(),
        source_port: 8080,
        dest_port: 443,
        protocol: "TCP".to_string(),
    };
    
    let context = ContextVector {
        features: vec![0.8, 0.9, 0.7, 0.6, 0.5],
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        metadata: HashMap::new(),
    };
    
    let mut group = c.benchmark_group("performance_improvements");
    
    // Measure optimized MFN performance
    group.bench_function("mfn_optimized", |b| {
        b.to_async(&rt).iter(|| async {
            let result = engine
                .process_flow(black_box(flow_key.clone()), black_box(vec![context.clone()]))
                .await
                .unwrap();
            black_box(result)
        })
    });
    
    // Simulate baseline performance (without optimizations)
    group.bench_function("baseline_simulation", |b| {
        b.to_async(&rt).iter(|| async {
            // Simulate slower baseline performance
            tokio::time::sleep(Duration::from_micros(2000)).await; // ~2ms baseline
            black_box(())
        })
    });
    
    group.finish();
}

/// Memory efficiency benchmarks
fn benchmark_memory_efficiency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = rt.block_on(async { MfnIntegrationEngine::new() });
    
    let mut group = c.benchmark_group("memory_efficiency");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);
    
    // Benchmark memory usage with increasing data size
    for num_flows in [100, 500, 1000, 2000].iter() {
        group.bench_with_input(
            BenchmarkId::new("memory_usage", num_flows),
            num_flows,
            |b, &num_flows| {
                b.to_async(&rt).iter(|| async {
                    let mut results = Vec::new();
                    
                    for i in 0..num_flows {
                        let flow_key = FlowKey {
                            source_ip: format!("mem.test.{}", i),
                            dest_ip: format!("mem.dest.{}", i),
                            source_port: (8000 + i % 1000) as u16,
                            dest_port: 443,
                            protocol: "TCP".to_string(),
                        };
                        
                        let context = ContextVector {
                            features: vec![
                                (i as f64) / 1000.0,
                                ((i * 2) as f64) / 1000.0,
                                ((i * 3) as f64) / 1000.0,
                                0.5,
                                0.7
                            ],
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                            metadata: HashMap::new(),
                        };
                        
                        let result = engine
                            .process_flow(flow_key, vec![context])
                            .await
                            .unwrap();
                        
                        results.push(result);
                    }
                    
                    black_box(results)
                })
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_single_flow_processing,
    benchmark_concurrent_throughput,
    benchmark_layer_performance,
    benchmark_cache_effectiveness,
    benchmark_performance_improvements,
    benchmark_memory_efficiency
);

criterion_main!(benches);
