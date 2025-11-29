//! Platform integration performance benchmarks

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Duration;
use tokio::runtime::Runtime;
use hypermesh_integration::{
    HyperMeshPlatform, HyperMeshConfig, 
    IntegrationMetrics, ServiceRegistry,
    services::ServiceEndpoint,
};

/// Benchmark platform initialization time
fn bench_platform_initialization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("platform_initialization", |b| {
        b.to_async(&rt).iter(|| async {
            let config = create_benchmark_config();
            let platform = HyperMeshPlatform::new(config)
                .await
                .expect("Platform should initialize");
            
            let start = std::time::Instant::now();
            platform.initialize().await.expect("Platform should start");
            let duration = start.elapsed();
            
            platform.shutdown().await.expect("Platform should shutdown");
            duration
        })
    });
}

/// Benchmark service registry operations
fn bench_service_registry(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let registry = ServiceRegistry::new();
    
    let mut group = c.benchmark_group("service_registry");
    
    // Benchmark service registration
    group.bench_function("service_registration", |b| {
        b.to_async(&rt).iter_batched(
            || create_test_service_endpoint(),
            |endpoint| async {
                registry.register_service("test-service".to_string(), endpoint)
                    .await
                    .expect("Service should register");
            },
            criterion::BatchSize::SmallInput,
        )
    });
    
    // Benchmark service discovery
    group.bench_function("service_discovery", |b| {
        b.to_async(&rt).iter(|| async {
            let query = hypermesh_integration::services::ServiceQuery {
                service_type: "test-service".to_string(),
                required_tags: None,
                preferred_tags: None,
                min_health: None,
                limit: Some(10),
            };
            
            registry.discover_services(query).await
        })
    });
    
    group.finish();
}

/// Benchmark metrics collection
fn bench_metrics_collection(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let metrics = IntegrationMetrics::new();
    
    c.bench_function("metrics_collection", |b| {
        b.to_async(&rt).iter(|| async {
            // Record some sample metrics
            metrics.record_component_init("test-component", Duration::from_millis(100));
            metrics.update_component_health("test-component", 0.95);
            metrics.update_active_services(5);
            
            // Collect platform metrics
            metrics.collect_platform_metrics().await
        })
    });
}

/// Benchmark configuration validation
fn bench_config_validation(c: &mut Criterion) {
    c.bench_function("config_validation", |b| {
        b.iter(|| {
            let config = create_benchmark_config();
            config.validate().expect("Config should be valid")
        })
    });
}

/// Benchmark concurrent service operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let registry = ServiceRegistry::new();
    
    let mut group = c.benchmark_group("concurrent_operations");
    
    for concurrent_ops in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_service_registration", concurrent_ops),
            concurrent_ops,
            |b, &concurrent_ops| {
                b.to_async(&rt).iter(|| async {
                    let mut handles = Vec::new();
                    
                    for i in 0..concurrent_ops {
                        let registry = &registry;
                        let endpoint = create_test_service_endpoint();
                        
                        let handle = tokio::spawn(async move {
                            registry.register_service(
                                format!("service-{}", i),
                                endpoint
                            ).await.expect("Service should register");
                        });
                        
                        handles.push(handle);
                    }
                    
                    for handle in handles {
                        handle.await.expect("Task should complete");
                    }
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark platform metrics under load
fn bench_platform_metrics_load(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("platform_metrics_load", |b| {
        b.to_async(&rt).iter(|| async {
            let config = create_benchmark_config();
            let platform = HyperMeshPlatform::new(config)
                .await
                .expect("Platform should initialize");
            
            // Simulate load by collecting metrics multiple times
            let mut metrics = Vec::new();
            for _ in 0..10 {
                metrics.push(platform.metrics().await);
            }
            
            metrics
        })
    });
}

/// Helper function to create benchmark configuration
fn create_benchmark_config() -> HyperMeshConfig {
    let mut config = HyperMeshConfig::default();
    
    // Use different ports for benchmarks
    config.transport.bind_port = 9100;
    config.consensus.port = 9101;
    config.integration.metrics.prometheus_port = 9102;
    
    // Optimize for performance
    config.integration.health_check_interval_secs = 1;
    config.integration.communication_timeout_secs = 1;
    
    config
}

/// Helper function to create test service endpoint
fn create_test_service_endpoint() -> ServiceEndpoint {
    ServiceEndpoint {
        service_type: "benchmark-service".to_string(),
        address: "127.0.0.1".to_string(),
        port: 8080,
        health_check_path: "/health".to_string(),
    }
}

criterion_group!(
    benches,
    bench_platform_initialization,
    bench_service_registry,
    bench_metrics_collection,
    bench_config_validation,
    bench_concurrent_operations,
    bench_platform_metrics_load,
);

criterion_main!(benches);