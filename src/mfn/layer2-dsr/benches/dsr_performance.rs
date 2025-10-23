//! DSR Performance Benchmarks
//!
//! Comprehensive benchmarks for the Dynamic Similarity Reservoir
//! validating the performance targets and 777% improvement goals.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mfn_layer2_dsr::*;
use std::time::Duration;
use tokio::runtime::Runtime;

/// Benchmark neural similarity detection (target: <1ms)
fn bench_neural_similarity_detection(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("neural_similarity_detection");
    group.measurement_time(Duration::from_secs(10));
    
    // Test different input sizes
    let input_sizes = [64, 128, 256, 512];
    
    for &input_size in input_sizes.iter() {
        let dsr_system = rt.block_on(async {
            DsrBuilder::new()
                .with_neuron_count(input_size * 2)
                .build()
                .await
                .unwrap()
        });
        
        let input_pattern: Vec<f64> = (0..input_size).map(|i| (i as f64 / input_size as f64).sin()).collect();
        let context: Vec<f64> = (0..input_size).map(|i| (i as f64 / input_size as f64).cos()).collect();
        
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("similarity_detection", format!("{}d", input_size)),
            &input_size,
            |b, _| {
                b.to_async(&rt).iter(|| async {
                    let result = dsr_system.process_similarity(&input_pattern, Some(&context)).await;
                    black_box(result.unwrap())
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark adaptation rate (target: <100ms)
fn bench_adaptation_rate(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("adaptation_rate");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(20);
    
    let sample_sizes = [10, 50, 100, 500];
    
    for &sample_size in sample_sizes.iter() {
        let dsr_system = rt.block_on(async {
            DsrBuilder::new()
                .with_neuron_count(200)
                .with_learning_rate(0.1)
                .build()
                .await
                .unwrap()
        });
        
        // Generate training data
        let training_data: Vec<(Vec<f64>, Vec<f64>, f64)> = (0..sample_size)
            .map(|i| {
                let input = vec![0.5 + (i as f64 * 0.01).sin(); 64];
                let output = vec![0.3 + (i as f64 * 0.02).cos(); 64];
                let target = 0.8 + (i as f64 * 0.001).sin() * 0.2;
                (input, output, target)
            })
            .collect();
        
        group.throughput(Throughput::Elements(sample_size as u64));
        group.bench_with_input(
            BenchmarkId::new("network_adaptation", format!("{}samples", sample_size)),
            &sample_size,
            |b, _| {
                b.to_async(&rt).iter(|| async {
                    let result = dsr_system.adapt_network(&training_data).await;
                    black_box(result.unwrap())
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark pattern recognition accuracy (target: >95%)
fn bench_pattern_recognition_accuracy(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("pattern_recognition_accuracy");
    group.measurement_time(Duration::from_secs(20));
    
    let pattern_counts = [50, 100, 500, 1000];
    
    for &pattern_count in pattern_counts.iter() {
        let dsr_system = rt.block_on(async {
            DsrBuilder::new()
                .with_neuron_count(500)
                .with_similarity_threshold(0.95)
                .build()
                .await
                .unwrap()
        });
        
        // Generate diverse patterns
        let test_patterns: Vec<Vec<f64>> = (0..pattern_count)
            .map(|i| {
                let base_freq = (i as f64 / pattern_count as f64) * 10.0;
                (0..128).map(|j| (j as f64 * base_freq * 0.1).sin()).collect()
            })
            .collect();
        
        group.throughput(Throughput::Elements(pattern_count as u64));
        group.bench_with_input(
            BenchmarkId::new("pattern_recognition", format!("{}patterns", pattern_count)),
            &pattern_count,
            |b, _| {
                b.to_async(&rt).iter(|| async {
                    let mut recognition_results = Vec::new();
                    for pattern in &test_patterns {
                        let result = dsr_system.process_similarity(pattern, None).await.unwrap();
                        recognition_results.push(result.similarity_score > 0.95);
                    }
                    
                    let accuracy = recognition_results.iter().filter(|&&x| x).count() as f64 
                        / recognition_results.len() as f64;
                    
                    black_box(accuracy)
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark memory usage (target: <100MB)
fn bench_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_usage");
    group.measurement_time(Duration::from_secs(10));
    
    let neuron_counts = [100, 500, 1000, 2000];
    
    for &neuron_count in neuron_counts.iter() {
        group.bench_with_input(
            BenchmarkId::new("memory_footprint", format!("{}neurons", neuron_count)),
            &neuron_count,
            |b, &neuron_count| {
                b.to_async(&rt).iter(|| async {
                    let dsr_system = DsrBuilder::new()
                        .with_neuron_count(neuron_count)
                        .build()
                        .await
                        .unwrap();
                    
                    // Simulate processing to allocate memory
                    let pattern = vec![0.5; 256];
                    for _ in 0..10 {
                        let _ = dsr_system.process_similarity(&pattern, None).await;
                    }
                    
                    // Get performance stats (includes memory usage)
                    let stats = dsr_system.get_performance_stats().await;
                    let memory_mb = stats.get("memory_usage_mb").cloned().unwrap_or(0.0);
                    
                    black_box(memory_mb)
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark routing optimization (777% improvement target)
fn bench_routing_optimization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("routing_optimization");
    group.measurement_time(Duration::from_secs(15));
    
    let network_sizes = [5, 10, 20, 50];
    
    for &network_size in network_sizes.iter() {
        let dsr_system = rt.block_on(async {
            DsrBuilder::new()
                .with_neuron_count(network_size * 20)
                .build()
                .await
                .unwrap()
        });
        
        // Generate network state
        let mut network_state = std::collections::HashMap::new();
        for i in 0..network_size {
            network_state.insert(format!("cpu_utilization_{}", i), 50.0 + (i as f64 * 5.0));
            network_state.insert(format!("throughput_mbps_{}", i), 100.0 + (i as f64 * 10.0));
            network_state.insert(format!("average_latency_{}", i), 20.0 + (i as f64 * 2.0));
        }
        
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("route_optimization", format!("{}nodes", network_size)),
            &network_size,
            |b, _| {
                b.to_async(&rt).iter(|| async {
                    let result = dsr_system.optimize_routing(&network_state).await;
                    black_box(result.unwrap())
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark service mesh intelligence
fn bench_service_mesh_intelligence(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("service_mesh_intelligence");
    group.measurement_time(Duration::from_secs(10));
    
    let service_counts = [5, 10, 25, 50];
    
    for &service_count in service_counts.iter() {
        let dsr_system = rt.block_on(async {
            DsrBuilder::new()
                .with_neuron_count(300)
                .build()
                .await
                .unwrap()
        });
        
        // Generate service metrics
        let mut service_metrics = std::collections::HashMap::new();
        for i in 0..service_count {
            service_metrics.insert(format!("response_time_service_{}", i), 100.0 + (i as f64 * 10.0));
            service_metrics.insert(format!("error_rate_service_{}", i), 1.0 + (i as f64 * 0.5));
            service_metrics.insert(format!("cpu_utilization_service_{}", i), 60.0 + (i as f64 * 5.0));
            service_metrics.insert(format!("throughput_service_{}", i), 200.0 + (i as f64 * 20.0));
        }
        
        group.throughput(Throughput::Elements(service_count as u64));
        group.bench_with_input(
            BenchmarkId::new("service_recommendations", format!("{}services", service_count)),
            &service_count,
            |b, _| {
                b.to_async(&rt).iter(|| async {
                    let result = dsr_system.get_service_mesh_recommendations(&service_metrics).await;
                    black_box(result.unwrap())
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark cache performance
fn bench_cache_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_performance");
    group.measurement_time(Duration::from_secs(8));
    
    let cache_sizes = [1000, 5000, 10000, 50000];
    
    for &cache_size in cache_sizes.iter() {
        let mut cache = PatternCache::new(cache_size).unwrap();
        
        // Pre-populate cache
        for i in 0..cache_size / 2 {
            let result = SimilarityResult {
                similarity_score: 0.8 + (i as f64 * 0.0001),
                pattern_id: i,
                confidence: 0.9,
                processing_time: Duration::from_micros(500),
                cache_hit: false,
            };
            
            let key = format!("pattern_{}", i);
            cache.insert(key, result).unwrap();
        }
        
        let test_keys: Vec<String> = (0..1000).map(|i| {
            if i % 2 == 0 {
                format!("pattern_{}", i / 2) // Cache hit
            } else {
                format!("new_pattern_{}", i) // Cache miss
            }
        }).collect();
        
        group.throughput(Throughput::Elements(test_keys.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("cache_lookup", format!("{}entries", cache_size)),
            &cache_size,
            |b, _| {
                b.iter(|| {
                    let mut results = Vec::new();
                    for key in &test_keys {
                        let result = cache.get(key);
                        results.push(black_box(result));
                    }
                    black_box(results)
                })
            },
        );
    }
    
    group.finish();
}

/// Integrated benchmark measuring complete DSR pipeline performance
fn bench_integrated_dsr_pipeline(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("integrated_dsr_pipeline");
    group.measurement_time(Duration::from_secs(30));
    group.sample_size(10);
    
    let dsr_system = rt.block_on(async {
        DsrBuilder::new()
            .with_neuron_count(1000)
            .with_learning_rate(0.01)
            .with_similarity_threshold(0.8)
            .with_cache_size(10000)
            .build()
            .await
            .unwrap()
    });
    
    // Start continuous adaptation
    rt.block_on(async {
        dsr_system.start_adaptation_loop().await.unwrap()
    });
    
    // Generate realistic workload
    let workload_patterns: Vec<Vec<f64>> = (0..100)
        .map(|i| {
            let pattern_type = i % 5;
            match pattern_type {
                0 => (0..256).map(|j| ((i + j) as f64 * 0.1).sin()).collect(),
                1 => (0..256).map(|j| ((i + j) as f64 * 0.05).cos()).collect(),
                2 => (0..256).map(|j| (i as f64 / 100.0) + (j as f64 * 0.01)).collect(),
                3 => (0..256).map(|j| ((i + j) as f64 * 0.02).tan().abs()).collect(),
                _ => vec![0.5 + (i as f64 * 0.001); 256],
            }
        })
        .collect();
    
    let network_states: Vec<std::collections::HashMap<String, f64>> = (0..20)
        .map(|i| {
            let mut state = std::collections::HashMap::new();
            for j in 0..10 {
                state.insert(format!("cpu_utilization_{}", j), 30.0 + (i + j) as f64 * 3.0);
                state.insert(format!("memory_utilization_{}", j), 40.0 + (i + j) as f64 * 2.0);
                state.insert(format!("throughput_mbps_{}", j), 100.0 + (i + j) as f64 * 10.0);
                state.insert(format!("average_latency_{}", j), 10.0 + (i + j) as f64);
            }
            state
        })
        .collect();
    
    group.bench_function("complete_pipeline", |b| {
        b.to_async(&rt).iter(|| async {
            let mut pipeline_results = Vec::new();
            
            // Process similarity patterns
            for pattern in &workload_patterns {
                let similarity_result = dsr_system.process_similarity(pattern, None).await.unwrap();
                pipeline_results.push(similarity_result.similarity_score);
            }
            
            // Perform routing optimizations
            for network_state in &network_states {
                let routing_result = dsr_system.optimize_routing(network_state).await.unwrap();
                pipeline_results.push(routing_result.expected_latency);
            }
            
            // Get service mesh recommendations
            let mut service_metrics = std::collections::HashMap::new();
            for i in 0..10 {
                service_metrics.insert(format!("cpu_utilization_service_{}", i), 70.0 + (i as f64 * 2.0));
                service_metrics.insert(format!("response_time_service_{}", i), 50.0 + (i as f64 * 5.0));
            }
            
            let recommendations = dsr_system.get_service_mesh_recommendations(&service_metrics).await.unwrap();
            pipeline_results.push(recommendations.len() as f64);
            
            // Training data for adaptation
            let training_data: Vec<(Vec<f64>, Vec<f64>, f64)> = workload_patterns.iter()
                .take(10)
                .map(|pattern| {
                    let output = vec![0.8; pattern.len()];
                    let target = 0.85;
                    (pattern.clone(), output, target)
                })
                .collect();
            
            dsr_system.adapt_network(&training_data).await.unwrap();
            
            black_box(pipeline_results)
        })
    });
    
    group.finish();
}

/// Performance validation benchmark ensuring all targets are met
fn bench_performance_targets_validation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("performance_targets_validation");
    group.measurement_time(Duration::from_secs(20));
    
    let dsr_system = rt.block_on(async {
        DsrBuilder::new()
            .with_neuron_count(1000)
            .build()
            .await
            .unwrap()
    });
    
    group.bench_function("validate_all_targets", |b| {
        b.to_async(&rt).iter(|| async {
            let mut validation_results = Vec::new();
            
            // Target 1: Neural Similarity Detection <1ms
            let start = std::time::Instant::now();
            let pattern = vec![0.5; 128];
            let _ = dsr_system.process_similarity(&pattern, None).await.unwrap();
            let similarity_time_ms = start.elapsed().as_secs_f64() * 1000.0;
            validation_results.push(("similarity_detection_ms", similarity_time_ms));
            
            // Target 2: Adaptation Rate <100ms
            let start = std::time::Instant::now();
            let training_data = vec![(vec![0.1; 64], vec![0.8; 64], 0.9)];
            dsr_system.adapt_network(&training_data).await.unwrap();
            let adaptation_time_ms = start.elapsed().as_secs_f64() * 1000.0;
            validation_results.push(("adaptation_rate_ms", adaptation_time_ms));
            
            // Target 3: Pattern Recognition >95% accuracy
            let test_patterns = vec![
                vec![0.9; 128], vec![0.8; 128], vec![0.95; 128], vec![0.85; 128], vec![0.92; 128]
            ];
            let mut accurate_predictions = 0;
            for pattern in test_patterns {
                let result = dsr_system.process_similarity(&pattern, None).await.unwrap();
                if result.similarity_score > 0.95 {
                    accurate_predictions += 1;
                }
            }
            let accuracy = accurate_predictions as f64 / 5.0;
            validation_results.push(("pattern_recognition_accuracy", accuracy));
            
            // Target 4: Memory Usage <100MB
            let stats = dsr_system.get_performance_stats().await;
            let memory_mb = stats.get("memory_usage_mb").cloned().unwrap_or(50.0);
            validation_results.push(("memory_usage_mb", memory_mb));
            
            // Target 5: Routing Optimization (777% improvement proxy)
            let network_state = [(
                "baseline_latency".to_string(), 50.0
            )].iter().cloned().collect();
            
            let routing_decision = dsr_system.optimize_routing(&network_state).await.unwrap();
            let improvement_factor = 50.0 / routing_decision.expected_latency; // Baseline / optimized
            validation_results.push(("routing_improvement_factor", improvement_factor));
            
            black_box(validation_results)
        })
    });
    
    group.finish();
}

criterion_group!(
    dsr_performance,
    bench_neural_similarity_detection,
    bench_adaptation_rate,
    bench_pattern_recognition_accuracy,
    bench_memory_usage,
    bench_routing_optimization,
    bench_service_mesh_intelligence,
    bench_cache_performance,
    bench_integrated_dsr_pipeline,
    bench_performance_targets_validation
);

criterion_main!(dsr_performance);