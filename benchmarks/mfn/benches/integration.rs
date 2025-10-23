/*!
# Integration Benchmarks

End-to-end benchmarks for the complete MFN system with all layers working together.
Tests system-wide performance, throughput, and coordination efficiency.
*/

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mfn_benchmarks::{common::*, integration::*, layer1::*, layer2::*, layer3::*, layer4::*};
use std::time::Duration;
use tokio::runtime::Runtime;

fn bench_end_to_end_flow_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("end_to_end_flow_processing");
    group.measurement_time(Duration::from_secs(30));
    
    // Test different packet sizes
    let packet_sizes = [64, 256, 512, 1024, 4096, 8192];
    
    for &packet_size in packet_sizes.iter() {
        let config = IntegrationBenchmarkConfig {
            concurrent_flows: 1000,
            test_duration_seconds: 5,
            ..Default::default()
        };
        
        group.throughput(Throughput::Bytes(packet_size as u64));
        group.bench_with_input(
            BenchmarkId::new("mfn_flow_processing", format!("{}bytes", packet_size)),
            &packet_size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let mfn_system = MfnSystem::new(config.clone()).await.unwrap();
                    
                    let flow_key = {
                        let mut key = [0u8; 32];
                        fastrand::fill(&mut key);
                        key
                    };
                    
                    let flow_data = vec![0u8; size];
                    let context_data: Vec<f32> = (0..256).map(|_| fastrand::f32()).collect();
                    
                    let result = mfn_system.process_flow_complete(
                        flow_key, 
                        &flow_data, 
                        &context_data
                    ).await;
                    
                    black_box(result)
                })
            },
        );
    }
    
    group.finish();
}

fn bench_concurrent_flow_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_flow_processing");
    group.measurement_time(Duration::from_secs(45));
    group.sample_size(20); // Fewer samples for long-running benchmarks
    
    let concurrency_levels = [1, 10, 50, 100, 500];
    
    for &concurrency in concurrency_levels.iter() {
        group.throughput(Throughput::Elements(concurrency as u64));
        group.bench_with_input(
            BenchmarkId::new("concurrent_flows", concurrency),
            &concurrency,
            |b, &concurrent_flows| {
                b.to_async(&rt).iter(|| async {
                    let config = IntegrationBenchmarkConfig {
                        concurrent_flows,
                        test_duration_seconds: 1,
                        ..Default::default()
                    };
                    
                    let mfn_system = MfnSystem::new(config).await.unwrap();
                    
                    // Spawn concurrent flow processing tasks
                    let mut handles = Vec::new();
                    
                    for i in 0..concurrent_flows {
                        let system = mfn_system.clone();
                        
                        let handle = tokio::spawn(async move {
                            let flow_key = {
                                let mut key = [0u8; 32];
                                key[..4].copy_from_slice(&(i as u32).to_le_bytes());
                                fastrand::fill(&mut key[4..]);
                                key
                            };
                            
                            let flow_data = vec![(i % 256) as u8; 1024];
                            let context_data: Vec<f32> = (0..256).map(|j| (i + j) as f32 / 1000.0).collect();
                            
                            system.process_flow_complete(flow_key, &flow_data, &context_data).await
                        });
                        
                        handles.push(handle);
                    }
                    
                    // Wait for all flows to complete
                    let results = futures::future::join_all(handles).await;
                    black_box(results)
                })
            },
        );
    }
    
    group.finish();
}

fn bench_layer_coordination_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("layer_coordination_latency");
    
    // Test coordination between different layer combinations
    let layer_combinations = [
        ("L1_only", vec![MfnLayer::Layer1Ifr]),
        ("L1_L2", vec![MfnLayer::Layer1Ifr, MfnLayer::Layer2Dsr]),
        ("L1_L3", vec![MfnLayer::Layer1Ifr, MfnLayer::Layer3Alm]),
        ("L1_L4", vec![MfnLayer::Layer1Ifr, MfnLayer::Layer4Cpe]),
        ("All_layers", vec![MfnLayer::Layer1Ifr, MfnLayer::Layer2Dsr, MfnLayer::Layer3Alm, MfnLayer::Layer4Cpe]),
    ];
    
    for (name, layers) in layer_combinations.iter() {
        group.throughput(Throughput::Elements(layers.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("coordination_latency", name),
            layers,
            |b, layers| {
                b.to_async(&rt).iter(|| async {
                    let config = IntegrationBenchmarkConfig::default();
                    let mfn_system = MfnSystem::new(config).await.unwrap();
                    
                    // Simulate coordination messages between layers
                    let flow_key = {
                        let mut key = [0u8; 32];
                        fastrand::fill(&mut key);
                        key
                    };
                    
                    for &layer in layers {
                        let message = match layer {
                            MfnLayer::Layer1Ifr => {
                                CoordinationMessage::FlowRegistered { flow_key, component_id: 1234 }
                            }
                            MfnLayer::Layer2Dsr => {
                                CoordinationMessage::SimilarityDetected { flow_key, similarity: 0.85 }
                            }
                            MfnLayer::Layer3Alm => {
                                CoordinationMessage::RouteOptimized { flow_key, route: vec![0, 1, 2] }
                            }
                            MfnLayer::Layer4Cpe => {
                                CoordinationMessage::ContextPredicted { flow_key, prediction: vec![0.5; 10] }
                            }
                            _ => continue,
                        };
                        
                        let _ = mfn_system.coordination_channel.send(message);
                    }
                    
                    // Small delay to allow message processing
                    tokio::time::sleep(Duration::from_micros(100)).await;
                })
            },
        );
    }
    
    group.finish();
}

fn bench_network_throughput_scaling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("network_throughput_scaling");
    group.measurement_time(Duration::from_secs(60));
    group.sample_size(10);
    
    let throughput_targets_gbps = [1.0, 10.0, 25.0, 40.0];
    
    for &target_gbps in throughput_targets_gbps.iter() {
        group.throughput(Throughput::Bytes((target_gbps * 1_000_000_000.0 / 8.0) as u64)); // Convert Gbps to Bps
        group.bench_with_input(
            BenchmarkId::new("throughput_test", format!("{}Gbps", target_gbps)),
            &target_gbps,
            |b, &target| {
                b.to_async(&rt).iter(|| async {
                    let config = IntegrationBenchmarkConfig {
                        throughput_target_gbps: target,
                        test_duration_seconds: 5,
                        concurrent_flows: (target * 1000.0) as usize, // Scale flows with target
                        ..Default::default()
                    };
                    
                    let mfn_system = MfnSystem::new(config.clone()).await.unwrap();
                    
                    // Create network throughput simulator
                    let mut simulator = NetworkThroughputSimulator::new(config.network_simulation);
                    
                    let result = simulator.run_throughput_test(
                        &mfn_system, 
                        Duration::from_secs(5)
                    ).await;
                    
                    black_box(result)
                })
            },
        );
    }
    
    group.finish();
}

fn bench_memory_usage_under_load(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_usage_under_load");
    group.measurement_time(Duration::from_secs(30));
    
    let flow_counts = [1000, 10000, 50000, 100000];
    
    for &flow_count in flow_counts.iter() {
        group.throughput(Throughput::Elements(flow_count as u64));
        group.bench_with_input(
            BenchmarkId::new("memory_scaling", format!("{}flows", flow_count)),
            &flow_count,
            |b, &flows| {
                b.to_async(&rt).iter(|| async {
                    let config = IntegrationBenchmarkConfig {
                        concurrent_flows: flows,
                        ..Default::default()
                    };
                    
                    let mfn_system = MfnSystem::new(config).await.unwrap();
                    
                    // Process many flows to test memory scaling
                    let mut handles = Vec::new();
                    
                    for i in 0..flows.min(1000) { // Process up to 1000 flows in benchmark
                        let system = mfn_system.clone();
                        
                        let handle = tokio::spawn(async move {
                            let flow_key = {
                                let mut key = [0u8; 32];
                                key[..4].copy_from_slice(&(i as u32).to_le_bytes());
                                key
                            };
                            
                            let flow_data = vec![(i % 256) as u8; 512];
                            let context_data: Vec<f32> = vec![(i as f32) / 1000.0; 256];
                            
                            system.process_flow_complete(flow_key, &flow_data, &context_data).await
                        });
                        
                        handles.push(handle);
                        
                        // Add small delay to prevent overwhelming
                        if i % 100 == 0 {
                            tokio::time::sleep(Duration::from_millis(1)).await;
                        }
                    }
                    
                    let results = futures::future::join_all(handles).await;
                    black_box(results)
                })
            },
        );
    }
    
    group.finish();
}

fn bench_baseline_vs_mfn_comparison(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("baseline_vs_mfn");
    group.measurement_time(Duration::from_secs(45));
    
    let flow_counts = [100, 1000, 5000];
    
    for &flow_count in flow_counts.iter() {
        // Benchmark baseline system (without MFN optimizations)
        group.bench_with_input(
            BenchmarkId::new("baseline_system", flow_count),
            &flow_count,
            |b, &flows| {
                b.to_async(&rt).iter(|| async {
                    let baseline_config = mfn_benchmarks::baseline::BaselineConfig {
                        enable_network_calls: true,
                        simulate_database_lookups: true,
                        simulate_ml_inference: true,
                        network_latency_ms: 2.0,
                        database_latency_ms: 5.0,
                        ml_inference_latency_ms: 50.0,
                        error_rate_percent: 1.0,
                    };
                    
                    let mut baseline_system = mfn_benchmarks::baseline::HyperMeshBaseline::new(baseline_config);
                    baseline_system.initialize_baseline_topology(1000, 0.3);
                    
                    let mut results = Vec::new();
                    
                    for i in 0..flows {
                        let flow_key = {
                            let mut key = [0u8; 32];
                            key[..4].copy_from_slice(&(i as u32).to_le_bytes());
                            key
                        };
                        
                        let flow_data = vec![(i % 256) as u8; 1024];
                        let context_data: Vec<f32> = vec![i as f32 / 1000.0; 256];
                        
                        let result = baseline_system.baseline_process_flow(
                            flow_key, 
                            &flow_data, 
                            &context_data
                        ).await;
                        
                        results.push(result);
                    }
                    
                    black_box(results)
                })
            },
        );
        
        // Benchmark MFN system
        group.bench_with_input(
            BenchmarkId::new("mfn_system", flow_count),
            &flow_count,
            |b, &flows| {
                b.to_async(&rt).iter(|| async {
                    let config = IntegrationBenchmarkConfig {
                        concurrent_flows: flows,
                        test_duration_seconds: 10,
                        ..Default::default()
                    };
                    
                    let mfn_system = MfnSystem::new(config).await.unwrap();
                    
                    let mut results = Vec::new();
                    
                    for i in 0..flows {
                        let flow_key = {
                            let mut key = [0u8; 32];
                            key[..4].copy_from_slice(&(i as u32).to_le_bytes());
                            key
                        };
                        
                        let flow_data = vec![(i % 256) as u8; 1024];
                        let context_data: Vec<f32> = vec![i as f32 / 1000.0; 256];
                        
                        let result = mfn_system.process_flow_complete(
                            flow_key, 
                            &flow_data, 
                            &context_data
                        ).await;
                        
                        results.push(result);
                    }
                    
                    black_box(results)
                })
            },
        );
    }
    
    group.finish();
}

fn bench_error_handling_and_recovery(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("error_handling_recovery");
    
    let error_rates = [0.01, 0.05, 0.10]; // 1%, 5%, 10% error rates
    
    for &error_rate in error_rates.iter() {
        group.bench_with_input(
            BenchmarkId::new("error_recovery", format!("{}%", (error_rate * 100.0) as u32)),
            &error_rate,
            |b, &err_rate| {
                b.to_async(&rt).iter(|| async {
                    let config = IntegrationBenchmarkConfig::default();
                    let mfn_system = MfnSystem::new(config).await.unwrap();
                    
                    let mut successful = 0;
                    let mut failed = 0;
                    
                    for i in 0..100 {
                        let flow_key = {
                            let mut key = [0u8; 32];
                            key[..4].copy_from_slice(&(i as u32).to_le_bytes());
                            key
                        };
                        
                        // Introduce errors based on error rate
                        let should_error = fastrand::f64() < err_rate;
                        
                        let flow_data = if should_error {
                            vec![] // Empty data to trigger error
                        } else {
                            vec![(i % 256) as u8; 1024]
                        };
                        
                        let context_data: Vec<f32> = vec![i as f32 / 1000.0; 256];
                        
                        match mfn_system.process_flow_complete(flow_key, &flow_data, &context_data).await {
                            Ok(_) => successful += 1,
                            Err(_) => failed += 1,
                        }
                    }
                    
                    black_box((successful, failed))
                })
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    integration_benchmarks,
    bench_end_to_end_flow_processing,
    bench_concurrent_flow_processing,
    bench_layer_coordination_latency,
    bench_network_throughput_scaling,
    bench_memory_usage_under_load,
    bench_baseline_vs_mfn_comparison,
    bench_error_handling_and_recovery
);

criterion_main!(integration_benchmarks);