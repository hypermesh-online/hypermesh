//! Layer-to-Layer Integration Benchmarks
//!
//! Specialized benchmarks for validating performance of layer-to-layer communication
//! and data flow through the MFN pipeline.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::HashMap;
use std::time::Duration;
use tokio::runtime::Runtime;

use mfn_integration_tests::*;

/// Benchmark Layer 1 -> Layer 2 integration
fn benchmark_layer1_to_layer2(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = rt.block_on(async { MfnIntegrationEngine::new() });
    
    let flow_key = FlowKey {
        source_ip: "layer1.test".to_string(),
        dest_ip: "layer2.test".to_string(),
        source_port: 8080,
        dest_port: 443,
        protocol: "TCP".to_string(),
    };
    
    c.bench_function("layer1_to_layer2_handoff", |b| {
        b.to_async(&rt).iter(|| async {
            // Simulate Layer 1 processing
            let layer1_result = engine.layer1.lookup_flow(black_box(&flow_key)).await.unwrap();
            
            // Simulate Layer 2 receiving Layer 1 data
            let context = ContextVector {
                features: vec![0.5, 0.6, 0.7, 0.8, 0.9],
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                metadata: HashMap::new(),
            };
            
            let layer2_result = engine.layer2
                .process_similarity(black_box(&flow_key), black_box(Some(&context)))
                .await
                .unwrap();
            
            black_box((layer1_result, layer2_result))
        })
    });
}

/// Benchmark Layer 2 -> Layer 3 integration
fn benchmark_layer2_to_layer3(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = rt.block_on(async { MfnIntegrationEngine::new() });
    
    let flow_key = FlowKey {
        source_ip: "layer2.test".to_string(),
        dest_ip: "layer3.test".to_string(),
        source_port: 8080,
        dest_port: 443,
        protocol: "TCP".to_string(),
    };
    
    c.bench_function("layer2_to_layer3_handoff", |b| {
        b.to_async(&rt).iter(|| async {
            // Simulate Layer 2 processing
            let context = ContextVector {
                features: vec![0.7, 0.8, 0.6, 0.9, 0.5],
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                metadata: HashMap::new(),
            };
            
            let layer2_result = engine.layer2
                .process_similarity(black_box(&flow_key), black_box(Some(&context)))
                .await
                .unwrap();
            
            // Simulate Layer 3 using Layer 2 results
            let layer3_result = engine.layer3
                .optimize_routing(black_box(&flow_key), black_box(&layer2_result))
                .await
                .unwrap();
            
            black_box((layer2_result, layer3_result))
        })
    });
}

/// Benchmark Layer 3 -> Layer 4 integration
fn benchmark_layer3_to_layer4(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = rt.block_on(async { MfnIntegrationEngine::new() });
    
    let flow_key = FlowKey {
        source_ip: "layer3.test".to_string(),
        dest_ip: "layer4.test".to_string(),
        source_port: 8080,
        dest_port: 443,
        protocol: "TCP".to_string(),
    };
    
    c.bench_function("layer3_to_layer4_handoff", |b| {
        b.to_async(&rt).iter(|| async {
            // Create context history for Layer 4
            let context_history = vec![
                ContextVector {
                    features: vec![0.6, 0.7, 0.8, 0.5, 0.4],
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() - 10,
                    metadata: HashMap::new(),
                },
                ContextVector {
                    features: vec![0.7, 0.8, 0.9, 0.6, 0.5],
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    metadata: HashMap::new(),
                }
            ];
            
            // Simulate Layer 4 prediction
            let history_refs: Vec<&ContextVector> = context_history.iter().collect();
            let layer4_result = engine.layer4
                .predict_context(black_box(&flow_key), black_box(history_refs))
                .await
                .unwrap();
            
            black_box(layer4_result)
        })
    });
}

/// Benchmark full pipeline with different data sizes
fn benchmark_pipeline_data_sizes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = rt.block_on(async { MfnIntegrationEngine::new() });
    
    let mut group = c.benchmark_group("pipeline_data_sizes");
    
    for context_size in [5, 10, 20, 50, 100].iter() {
        group.throughput(Throughput::Elements(*context_size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("context_features", context_size),
            context_size,
            |b, &context_size| {
                b.to_async(&rt).iter(|| async {
                    let flow_key = FlowKey {
                        source_ip: "pipeline.test".to_string(),
                        dest_ip: "pipeline.dest".to_string(),
                        source_port: 8080,
                        dest_port: 443,
                        protocol: "TCP".to_string(),
                    };
                    
                    let features: Vec<f64> = (0..context_size)
                        .map(|i| (i as f64) / (context_size as f64))
                        .collect();
                    
                    let context = ContextVector {
                        features,
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
            },
        );
    }
    
    group.finish();
}

/// Benchmark different flow patterns
fn benchmark_flow_patterns(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = rt.block_on(async { MfnIntegrationEngine::new() });
    
    let mut group = c.benchmark_group("flow_patterns");
    
    // Burst pattern - many flows in short time
    group.bench_function("burst_pattern", |b| {
        b.to_async(&rt).iter(|| async {
            let mut results = Vec::new();
            
            for i in 0..50 {
                let flow_key = FlowKey {
                    source_ip: format!("burst.{}", i),
                    dest_ip: "burst.dest".to_string(),
                    source_port: (8000 + i) as u16,
                    dest_port: 443,
                    protocol: "TCP".to_string(),
                };
                
                let context = ContextVector {
                    features: vec![0.5, 0.6, 0.7, 0.8, 0.9],
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
    });
    
    // Steady pattern - consistent flow rate
    group.bench_function("steady_pattern", |b| {
        b.to_async(&rt).iter(|| async {
            let flow_key = FlowKey {
                source_ip: "steady.source".to_string(),
                dest_ip: "steady.dest".to_string(),
                source_port: 8080,
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

/// Benchmark error handling and recovery
fn benchmark_error_handling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = rt.block_on(async { MfnIntegrationEngine::new() });
    
    let mut group = c.benchmark_group("error_handling");
    
    // Normal operation (baseline)
    group.bench_function("normal_operation", |b| {
        b.to_async(&rt).iter(|| async {
            let flow_key = FlowKey {
                source_ip: "normal.source".to_string(),
                dest_ip: "normal.dest".to_string(),
                source_port: 8080,
                dest_port: 443,
                protocol: "TCP".to_string(),
            };
            
            let context = ContextVector {
                features: vec![0.5, 0.6, 0.7, 0.8, 0.9],
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
    
    // Edge case - very large feature vectors
    group.bench_function("large_feature_vectors", |b| {
        b.to_async(&rt).iter(|| async {
            let flow_key = FlowKey {
                source_ip: "large.source".to_string(),
                dest_ip: "large.dest".to_string(),
                source_port: 8080,
                dest_port: 443,
                protocol: "TCP".to_string(),
            };
            
            let large_features: Vec<f64> = (0..1000)
                .map(|i| (i as f64) / 1000.0)
                .collect();
            
            let context = ContextVector {
                features: large_features,
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

criterion_group!(
    layer_integration_benches,
    benchmark_layer1_to_layer2,
    benchmark_layer2_to_layer3,
    benchmark_layer3_to_layer4,
    benchmark_pipeline_data_sizes,
    benchmark_flow_patterns,
    benchmark_error_handling
);

criterion_main!(layer_integration_benches);
