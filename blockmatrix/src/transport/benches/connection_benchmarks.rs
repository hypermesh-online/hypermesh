//! Connection benchmarks for HyperMesh Transport

use criterion::{black_box, criterion_group, criterion_main, Criterion, BatchSize};
use crate::transport::{
    HyperMeshTransport, HyperMeshTransportConfig, NodeId
};
use stoq::Endpoint;
use std::net::Ipv6Addr;
use tokio::runtime::Runtime;

/// Benchmark connection establishment
fn bench_connection_establishment(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("connection_establishment", |b| {
        b.to_async(&rt).iter_batched(
            || {
                let config = HyperMeshTransportConfig::default();
                let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9292);
                (config, endpoint)
            },
            |(config, endpoint)| async move {
                let transport = HyperMeshTransport::new_async(config).await.unwrap();
                let node_id = NodeId::new("benchmark-node".to_string());
                
                // This would fail in practice without a server, but we're measuring setup time
                let result = transport.connect_to_node(node_id, &endpoint).await;
                black_box(result);
            },
            BatchSize::SmallInput,
        );
    });
}

/// Benchmark data sending
fn bench_data_sending(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let data_sizes = [64, 1024, 8192, 65536]; // Different message sizes
    
    for &size in &data_sizes {
        c.bench_function(&format!("send_data_{}_bytes", size), |b| {
            b.to_async(&rt).iter_batched(
                || {
                    let config = HyperMeshTransportConfig::default();
                    let data = vec![0u8; size];
                    (config, data)
                },
                |(config, data)| async move {
                    let transport = HyperMeshTransport::new_async(config).await.unwrap();
                    let node_id = NodeId::new("benchmark-node".to_string());
                    
                    // This would fail without connection, but measures serialization overhead
                    let result = transport.send_to_node(&node_id, &data).await;
                    black_box(result);
                },
                BatchSize::SmallInput,
            );
        });
    }
}

/// Benchmark connection pool operations
fn bench_connection_pool(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("connection_pool_lookup", |b| {
        b.to_async(&rt).iter_batched(
            || {
                let config = HyperMeshTransportConfig::default();
                (config, NodeId::new("test-node".to_string()))
            },
            |(config, node_id)| async move {
                let transport = HyperMeshTransport::new_async(config).await.unwrap();
                
                // Benchmark connection lookup (will return None, but measures lookup time)
                let result = transport.connection_pool.get_connection(&node_id).await;
                black_box(result);
            },
            BatchSize::SmallInput,
        );
    });
}

/// Benchmark metrics collection
fn bench_metrics_collection(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("metrics_collection", |b| {
        b.to_async(&rt).iter_batched(
            || HyperMeshTransportConfig::default(),
            |config| async move {
                let transport = HyperMeshTransport::new_async(config).await.unwrap();
                let stats = transport.stats().await;
                black_box(stats);
            },
            BatchSize::SmallInput,
        );
    });
}

/// Benchmark certificate validation
fn bench_certificate_validation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("certificate_validation", |b| {
        b.to_async(&rt).iter_batched(
            || {
                let config = HyperMeshTransportConfig::default();
                let node_id = NodeId::new("benchmark-node".to_string());
                let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9292);
                (config, node_id, endpoint)
            },
            |(config, node_id, endpoint)| async move {
                let transport = HyperMeshTransport::new_async(config).await.unwrap();
                let result = transport.authenticator.verify_node(&node_id, &endpoint).await;
                black_box(result);
            },
            BatchSize::SmallInput,
        );
    });
}

/// Benchmark concurrent connections
fn bench_concurrent_connections(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let concurrent_counts = [1, 10, 100];
    
    for &count in &concurrent_counts {
        c.bench_function(&format!("concurrent_connections_{}", count), |b| {
            b.to_async(&rt).iter_batched(
                || HyperMeshTransportConfig::default(),
                |config| async move {
                    let transport = HyperMeshTransport::new_async(config).await.unwrap();
                    
                    let mut handles = Vec::new();
                    for i in 0..count {
                        let transport = transport.clone();
                        let handle = tokio::spawn(async move {
                            let node_id = NodeId::new(format!("node-{}", i));
                            let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9292 + i as u16);
                            
                            // Benchmark connection attempt
                            let result = transport.connect_to_node(node_id, &endpoint).await;
                            black_box(result);
                        });
                        handles.push(handle);
                    }
                    
                    for handle in handles {
                        let _ = handle.await;
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }
}

/// Benchmark memory usage patterns
fn bench_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("memory_allocation", |b| {
        b.to_async(&rt).iter_batched(
            || HyperMeshTransportConfig::default(),
            |config| async move {
                // Measure memory allocation patterns
                let transport = HyperMeshTransport::new_async(config).await.unwrap();
                
                // Simulate typical usage patterns
                for i in 0..10 {
                    let node_id = NodeId::new(format!("memory-test-{}", i));
                    let data = vec![i as u8; 1024];
                    
                    let _ = transport.send_to_node(&node_id, &data).await;
                }
                
                black_box(transport);
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_connection_establishment,
    bench_data_sending,
    bench_connection_pool,
    bench_metrics_collection,
    bench_certificate_validation,
    bench_concurrent_connections,
    bench_memory_usage
);
criterion_main!(benches);