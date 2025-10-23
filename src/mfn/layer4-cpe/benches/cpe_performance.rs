/*!
# Layer 4 CPE Performance Benchmarks

Comprehensive performance benchmarks for the Context Prediction Engine layer.
Tests complete system performance including ML models, caching, and integration.

Performance Targets:
- <2ms prediction latency 
- >95% prediction accuracy
- >90% cache hit rate
- >50,000 predictions/second
- <200MB memory usage
*/

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mfn_layer4_cpe::{
    CpeSystem, CpeBuilder, ContextVector, 
    models::ModelType,
    cache::CacheStrategy,
    prediction::PredictionConfig,
};
use std::time::Duration;
use tokio::runtime::Runtime;

/// Benchmark configuration for different test scenarios
struct BenchmarkConfig {
    name: &'static str,
    model_type: ModelType,
    context_dimension: usize,
    sequence_length: usize,
    cache_size: usize,
    batch_size: usize,
}

const BENCHMARK_CONFIGS: &[BenchmarkConfig] = &[
    BenchmarkConfig {
        name: "small_lstm",
        model_type: ModelType::Lstm,
        context_dimension: 64,
        sequence_length: 16,
        cache_size: 1000,
        batch_size: 32,
    },
    BenchmarkConfig {
        name: "medium_lstm", 
        model_type: ModelType::Lstm,
        context_dimension: 256,
        sequence_length: 32,
        cache_size: 5000,
        batch_size: 64,
    },
    BenchmarkConfig {
        name: "large_lstm",
        model_type: ModelType::Lstm,
        context_dimension: 512,
        sequence_length: 64,
        cache_size: 10000,
        batch_size: 128,
    },
    BenchmarkConfig {
        name: "small_transformer",
        model_type: ModelType::Transformer,
        context_dimension: 128,
        sequence_length: 16,
        cache_size: 1000,
        batch_size: 16,
    },
    BenchmarkConfig {
        name: "medium_hybrid",
        model_type: ModelType::Hybrid,
        context_dimension: 256,
        sequence_length: 32,
        cache_size: 5000,
        batch_size: 32,
    },
];

/// Create test contexts with realistic patterns
fn create_test_contexts(count: usize, dimension: usize, pattern_type: &str) -> Vec<ContextVector> {
    (0..count).map(|i| {
        let flow_key = [(i % 256) as u8; 32];
        
        let features = match pattern_type {
            "temporal" => {
                // Time-based sinusoidal pattern
                (0..dimension).map(|j| {
                    let time_factor = (i as f32 + j as f32 * 0.1) * 0.05;
                    (time_factor.sin() + 1.0) * 0.5 // Normalize to [0, 1]
                }).collect()
            },
            "load_spike" => {
                // Load spike pattern with bursts
                (0..dimension).map(|j| {
                    let base_load = 0.2;
                    let spike = if i % 20 < 3 && j % 10 < 2 { 0.6 } else { 0.0 };
                    let noise = (i + j) as f32 * 0.001 % 0.1;
                    (base_load + spike + noise).min(1.0)
                }).collect()
            },
            "network_burst" => {
                // Network traffic burst pattern
                (0..dimension).map(|j| {
                    let burst_factor = if (i / 10) % 5 < 2 { 0.8 } else { 0.3 };
                    let freq_component = ((i + j * 3) as f32 * 0.02).cos() * 0.2;
                    (burst_factor + freq_component + 0.5).max(0.0).min(1.0)
                }).collect()
            },
            _ => {
                // Random pattern
                (0..dimension).map(|j| {
                    ((i * 7 + j * 3) % 1000) as f32 / 1000.0
                }).collect()
            }
        };
        
        ContextVector::new(flow_key, features)
            .with_metadata("pattern_type".to_string(), match pattern_type {
                "temporal" => 1.0,
                "load_spike" => 2.0,
                "network_burst" => 3.0,
                _ => 0.0,
            })
    }).collect()
}

/// Single prediction latency benchmark
fn bench_single_prediction_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("single_prediction_latency");
    group.measurement_time(Duration::from_secs(30));
    
    for config in BENCHMARK_CONFIGS {
        let system = rt.block_on(async {
            CpeBuilder::new()
                .with_model_type(config.model_type)
                .with_context_dimension(config.context_dimension)
                .with_sequence_length(config.sequence_length)
                .with_cache_size(config.cache_size)
                .with_prediction_timeout(10) // 10ms timeout
                .build().await.unwrap()
        });
        
        let test_contexts = create_test_contexts(
            config.sequence_length * 2, 
            config.context_dimension, 
            "temporal"
        );
        
        group.bench_with_input(
            BenchmarkId::new("prediction_latency", config.name),
            &(&system, &test_contexts),
            |b, (system, contexts)| {
                b.to_async(&rt).iter(|| async {
                    let flow_key = [fastrand::u8(..); 32];
                    let sequence = contexts.iter()
                        .cycle()
                        .take(config.sequence_length)
                        .cloned()
                        .collect::<Vec<_>>();
                    
                    let result = system.predict_context(flow_key, &sequence).await;
                    black_box(result)
                })
            }
        );
    }
    
    group.finish();
}

/// Throughput benchmark - predictions per second
fn bench_prediction_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("prediction_throughput");
    group.measurement_time(Duration::from_secs(45));
    
    for config in BENCHMARK_CONFIGS {
        let system = rt.block_on(async {
            CpeBuilder::new()
                .with_model_type(config.model_type)
                .with_context_dimension(config.context_dimension)
                .with_sequence_length(config.sequence_length)
                .with_cache_size(config.cache_size)
                .build().await.unwrap()
        });
        
        let test_contexts = create_test_contexts(
            1000, 
            config.context_dimension, 
            "temporal"
        );
        
        group.throughput(Throughput::Elements(config.batch_size as u64));
        group.bench_with_input(
            BenchmarkId::new("throughput", config.name),
            &(&system, &test_contexts, config.batch_size),
            |b, (system, contexts, batch_size)| {
                b.to_async(&rt).iter(|| async {
                    let mut results = Vec::with_capacity(*batch_size);
                    
                    for i in 0..*batch_size {
                        let flow_key = [i as u8; 32];
                        let start_idx = (i * 13) % contexts.len();
                        let sequence = contexts.iter()
                            .cycle()
                            .skip(start_idx)
                            .take(config.sequence_length)
                            .cloned()
                            .collect::<Vec<_>>();
                        
                        let result = system.predict_context(flow_key, &sequence).await;
                        results.push(black_box(result));
                    }
                    
                    results
                })
            }
        );
    }
    
    group.finish();
}

/// Cache performance benchmark
fn bench_cache_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("cache_performance");
    
    let cache_strategies = [
        CacheStrategy::Lru,
        CacheStrategy::Lfu,
        CacheStrategy::Adaptive,
    ];
    
    for strategy in cache_strategies {
        for config in &BENCHMARK_CONFIGS[..3] { // Test with first 3 configs
            let system = rt.block_on(async {
                CpeBuilder::new()
                    .with_model_type(config.model_type)
                    .with_context_dimension(config.context_dimension)
                    .with_cache_size(config.cache_size)
                    .build().await.unwrap()
            });
            
            let test_contexts = create_test_contexts(
                100, 
                config.context_dimension, 
                "temporal"
            );
            
            // Pre-populate cache with some predictions
            rt.block_on(async {
                for i in 0..20 {
                    let flow_key = [i as u8; 32];
                    let sequence = test_contexts.iter()
                        .cycle()
                        .take(config.sequence_length)
                        .cloned()
                        .collect::<Vec<_>>();
                    let _ = system.predict_context(flow_key, &sequence).await;
                }
            });
            
            group.bench_with_input(
                BenchmarkId::new(
                    format!("cache_{:?}", strategy).to_lowercase(),
                    format!("{}_{}", config.name, strategy as u8)
                ),
                &(&system, &test_contexts),
                |b, (system, contexts)| {
                    b.to_async(&rt).iter(|| async {
                        // Mix of cache hits and misses
                        let flow_key = if fastrand::f32() < 0.7 {
                            [fastrand::u8(..20); 32] // Likely cache hit
                        } else {
                            [fastrand::u8(..); 32] // Likely cache miss
                        };
                        
                        let sequence = contexts.iter()
                            .cycle()
                            .take(config.sequence_length)
                            .cloned()
                            .collect::<Vec<_>>();
                        
                        let result = system.predict_context(flow_key, &sequence).await;
                        black_box(result)
                    })
                }
            );
        }
    }
    
    group.finish();
}

/// Pattern recognition accuracy benchmark
fn bench_pattern_recognition(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("pattern_recognition");
    group.measurement_time(Duration::from_secs(60));
    
    let pattern_types = ["temporal", "load_spike", "network_burst"];
    
    for pattern in pattern_types {
        for config in &BENCHMARK_CONFIGS[1..3] { // Medium configs
            let system = rt.block_on(async {
                CpeBuilder::new()
                    .with_model_type(config.model_type)
                    .with_context_dimension(config.context_dimension)
                    .with_sequence_length(config.sequence_length)
                    .build().await.unwrap()
            });
            
            let test_contexts = create_test_contexts(
                200, 
                config.context_dimension, 
                pattern
            );
            
            group.bench_with_input(
                BenchmarkId::new("pattern_recognition", format!("{}_{}", config.name, pattern)),
                &(&system, &test_contexts),
                |b, (system, contexts)| {
                    b.to_async(&rt).iter(|| async {
                        let mut total_confidence = 0.0;
                        let batch_size = 20;
                        
                        for i in 0..batch_size {
                            let flow_key = [i as u8; 32];
                            let start_idx = (i * 7) % (contexts.len() - config.sequence_length);
                            let sequence = contexts[start_idx..start_idx + config.sequence_length].to_vec();
                            
                            if let Ok(result) = system.predict_context(flow_key, &sequence).await {
                                total_confidence += result.confidence;
                            }
                        }
                        
                        let avg_confidence = total_confidence / batch_size as f32;
                        black_box(avg_confidence)
                    })
                }
            );
        }
    }
    
    group.finish();
}

/// Memory usage benchmark
fn bench_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_usage");
    
    for config in BENCHMARK_CONFIGS {
        let system = rt.block_on(async {
            CpeBuilder::new()
                .with_model_type(config.model_type)
                .with_context_dimension(config.context_dimension)
                .with_sequence_length(config.sequence_length)
                .with_cache_size(config.cache_size)
                .build().await.unwrap()
        });
        
        let test_contexts = create_test_contexts(
            1000, 
            config.context_dimension, 
            "temporal"
        );
        
        group.bench_with_input(
            BenchmarkId::new("memory_pressure", config.name),
            &(&system, &test_contexts),
            |b, (system, contexts)| {
                b.to_async(&rt).iter(|| async {
                    // Simulate high memory pressure scenario
                    let mut results = Vec::with_capacity(100);
                    
                    for i in 0..100 {
                        let flow_key = [i as u8; 32];
                        let sequence = contexts.iter()
                            .cycle()
                            .skip(i * 3)
                            .take(config.sequence_length)
                            .cloned()
                            .collect::<Vec<_>>();
                        
                        let result = system.predict_context(flow_key, &sequence).await;
                        results.push(result);
                    }
                    
                    // Get performance stats to check memory usage
                    let stats = system.get_performance_stats().await;
                    black_box((results, stats))
                })
            }
        );
    }
    
    group.finish();
}

/// Learning adaptation benchmark
fn bench_learning_adaptation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("learning_adaptation");
    group.measurement_time(Duration::from_secs(30));
    
    for config in &BENCHMARK_CONFIGS[..2] { // Test with smaller configs
        let system = rt.block_on(async {
            CpeBuilder::new()
                .with_model_type(config.model_type)
                .with_context_dimension(config.context_dimension)
                .with_sequence_length(config.sequence_length)
                .build().await.unwrap()
        });
        
        let training_contexts = create_test_contexts(
            50, 
            config.context_dimension, 
            "temporal"
        );
        let target_contexts = create_test_contexts(
            50, 
            config.context_dimension, 
            "temporal"
        );
        
        group.bench_with_input(
            BenchmarkId::new("adaptation_speed", config.name),
            &(&system, &training_contexts, &target_contexts),
            |b, (system, predicted_contexts, actual_contexts)| {
                b.to_async(&rt).iter(|| async {
                    // Simulate learning from prediction errors
                    let mut adaptation_count = 0;
                    
                    for (predicted, actual) in predicted_contexts.iter().zip(actual_contexts.iter()) {
                        let flow_key = predicted.flow_key;
                        
                        if let Ok(_) = system.learn_from_feedback(flow_key, predicted, actual).await {
                            adaptation_count += 1;
                        }
                    }
                    
                    black_box(adaptation_count)
                })
            }
        );
    }
    
    group.finish();
}

/// Integration performance benchmark (mocked)
fn bench_integration_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("integration_performance");
    
    for config in &BENCHMARK_CONFIGS[1..3] {
        let system = rt.block_on(async {
            CpeBuilder::new()
                .with_model_type(config.model_type)
                .with_context_dimension(config.context_dimension)
                .with_sequence_length(config.sequence_length)
                .build().await.unwrap()
        });
        
        let test_contexts = create_test_contexts(
            100, 
            config.context_dimension, 
            "network_burst"
        );
        
        group.bench_with_input(
            BenchmarkId::new("routing_recommendations", config.name),
            &(&system, &test_contexts),
            |b, (system, contexts)| {
                b.to_async(&rt).iter(|| async {
                    let mut recommendations = Vec::new();
                    
                    for context in contexts.iter().take(10) {
                        let flow_key = context.flow_key;
                        if let Ok(routing_recs) = system.get_routing_recommendations(flow_key, context).await {
                            recommendations.extend(routing_recs);
                        }
                    }
                    
                    black_box(recommendations)
                })
            }
        );
    }
    
    group.finish();
}

/// Stress test - high concurrent load
fn bench_concurrent_load(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_load");
    group.measurement_time(Duration::from_secs(45));
    
    let system = rt.block_on(async {
        CpeBuilder::new()
            .with_model_type(ModelType::Lstm)
            .with_context_dimension(256)
            .with_sequence_length(32)
            .with_cache_size(10000)
            .build().await.unwrap()
    });
    
    let test_contexts = create_test_contexts(500, 256, "temporal");
    
    group.throughput(Throughput::Elements(100));
    group.bench_function("high_concurrency", |b| {
        b.to_async(&rt).iter(|| async {
            // Simulate 100 concurrent prediction requests
            let mut handles = Vec::with_capacity(100);
            
            for i in 0..100 {
                let system = &system;
                let contexts = &test_contexts;
                let flow_key = [i as u8; 32];
                
                let handle = tokio::spawn(async move {
                    let start_idx = (i * 3) % (contexts.len() - 32);
                    let sequence = contexts[start_idx..start_idx + 32].to_vec();
                    system.predict_context(flow_key, &sequence).await
                });
                
                handles.push(handle);
            }
            
            // Wait for all predictions to complete
            let mut results = Vec::with_capacity(100);
            for handle in handles {
                if let Ok(result) = handle.await {
                    results.push(result);
                }
            }
            
            black_box(results)
        })
    });
    
    group.finish();
}

criterion_group!(
    cpe_benchmarks,
    bench_single_prediction_latency,
    bench_prediction_throughput,
    bench_cache_performance,
    bench_pattern_recognition,
    bench_memory_usage,
    bench_learning_adaptation,
    bench_integration_performance,
    bench_concurrent_load
);

criterion_main!(cpe_benchmarks);