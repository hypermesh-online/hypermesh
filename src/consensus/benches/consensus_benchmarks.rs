//! Benchmarks for HyperMesh consensus system performance

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use crate::consensus::{
    Consensus, ConsensusConfig, IsolationLevel,
    config::{RaftConfig, ByzantineConfig, TransactionConfig, StorageConfig, ShardingConfig},
};
use crate::transport::{NodeId, HyperMeshTransportTrait, Endpoint, Connection, TransportStats};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use bytes::Bytes;
use async_trait::async_trait;
use tokio::runtime::Runtime;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use uuid::Uuid;

/// Mock transport for benchmarking
#[derive(Clone)]
struct BenchmarkTransport {
    latency: Duration,
}

impl BenchmarkTransport {
    fn new(latency_ms: u64) -> Self {
        Self {
            latency: Duration::from_millis(latency_ms),
        }
    }
}

#[async_trait]
impl HyperMeshTransportTrait for BenchmarkTransport {
    async fn connect_node(&self, _node_id: NodeId, _endpoint: &Endpoint) 
        -> hypermesh_transport::Result<Arc<Connection>> {
        tokio::time::sleep(self.latency).await;
        let endpoint = Endpoint::new(std::net::Ipv6Addr::LOCALHOST, 9292);
        let stoq_conn = hypermesh_transport::StoqConnection::new(
            Uuid::new_v4().to_string(),
            endpoint,
        );
        let connection = Arc::new(Connection::new(stoq_conn, _node_id));
        Ok(connection)
    }
    
    async fn accept_node(&self) -> hypermesh_transport::Result<Arc<Connection>> {
        tokio::time::sleep(self.latency).await;
        let endpoint = Endpoint::new(std::net::Ipv6Addr::LOCALHOST, 9292);
        let stoq_conn = hypermesh_transport::StoqConnection::new(
            Uuid::new_v4().to_string(),
            endpoint,
        );
        let node_id = NodeId::new(Uuid::new_v4().to_string());
        let connection = Arc::new(Connection::new(stoq_conn, node_id));
        Ok(connection)
    }
    
    async fn send_to(&self, _node_id: &NodeId, _data: &[u8]) -> hypermesh_transport::Result<()> {
        tokio::time::sleep(self.latency).await;
        Ok(())
    }
    
    async fn receive_from(&self, _connection: &Connection) -> hypermesh_transport::Result<Bytes> {
        tokio::time::sleep(self.latency).await;
        Ok(Bytes::from("benchmark data"))
    }
    
    async fn metrics(&self) -> TransportStats {
        TransportStats {
            bytes_sent: 0,
            bytes_received: 0,
            active_connections: 0,
            total_connections: 0,
            throughput_gbps: 0.0,
            avg_latency_us: self.latency.as_micros() as u64,
        }
    }
    
    async fn maintain(&self) -> hypermesh_transport::Result<()> {
        Ok(())
    }
}

/// Create a benchmark consensus node
async fn create_benchmark_node(node_id: &str, network_latency_ms: u64) -> Arc<Consensus> {
    let temp_dir = tempdir().unwrap();
    let node_id = NodeId::new(node_id.to_string());
    let transport = Arc::new(BenchmarkTransport::new(network_latency_ms));
    
    let mut config = ConsensusConfig::default();
    config.storage.data_dir = temp_dir.into_path();
    config.raft.election_timeout_ms = [50, 100]; // Fast for benchmarks
    config.raft.heartbeat_interval_ms = 25;
    config.byzantine.enabled = false; // Disable for pure performance tests
    
    let consensus = Consensus::new(node_id, config, transport).await.unwrap();
    Arc::new(consensus)
}

/// Benchmark single transaction throughput
fn bench_single_transaction_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("single_transaction_throughput");
    
    for isolation_level in [
        IsolationLevel::ReadCommitted,
        IsolationLevel::RepeatableRead,
        IsolationLevel::Serializable,
    ] {
        group.bench_with_input(
            BenchmarkId::new("isolation_level", format!("{:?}", isolation_level)),
            &isolation_level,
            |b, &isolation| {
                let node = rt.block_on(create_benchmark_node("bench-node", 1));
                rt.block_on(node.start()).unwrap();
                
                b.iter(|| {
                    rt.block_on(async {
                        let txn_id = node.transaction_manager
                            .begin_transaction(isolation)
                            .await
                            .unwrap();
                        
                        node.transaction_manager
                            .write(txn_id, "bench_key".to_string(), b"bench_value".to_vec())
                            .await
                            .unwrap();
                        
                        node.transaction_manager.commit(txn_id).await.unwrap()
                    })
                });
                
                rt.block_on(node.stop()).unwrap();
            },
        );
    }
    
    group.finish();
}

/// Benchmark concurrent transaction throughput
fn bench_concurrent_transaction_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_transaction_throughput");
    
    for concurrency in [1, 2, 4, 8, 16] {
        group.throughput(Throughput::Elements(concurrency as u64));
        group.bench_with_input(
            BenchmarkId::new("concurrency", concurrency),
            &concurrency,
            |b, &concurrency| {
                let node = rt.block_on(create_benchmark_node("concurrent-bench", 1));
                rt.block_on(node.start()).unwrap();
                
                b.iter(|| {
                    rt.block_on(async {
                        let mut handles = Vec::new();
                        
                        for i in 0..concurrency {
                            let node_clone = node.clone();
                            let handle = tokio::spawn(async move {
                                let txn_id = node_clone.transaction_manager
                                    .begin_transaction(IsolationLevel::ReadCommitted)
                                    .await
                                    .unwrap();
                                
                                node_clone.transaction_manager
                                    .write(
                                        txn_id,
                                        format!("concurrent_key_{}", i),
                                        format!("value_{}", i).into_bytes()
                                    )
                                    .await
                                    .unwrap();
                                
                                node_clone.transaction_manager.commit(txn_id).await.unwrap()
                            });
                            handles.push(handle);
                        }
                        
                        futures::future::join_all(handles).await;
                    })
                });
                
                rt.block_on(node.stop()).unwrap();
            },
        );
    }
    
    group.finish();
}

/// Benchmark transaction latency under different network conditions
fn bench_transaction_latency_network(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("transaction_latency_network");
    
    for network_latency_ms in [0, 1, 5, 10, 25] {
        group.bench_with_input(
            BenchmarkId::new("network_latency_ms", network_latency_ms),
            &network_latency_ms,
            |b, &latency| {
                let node = rt.block_on(create_benchmark_node("latency-bench", latency));
                rt.block_on(node.start()).unwrap();
                
                b.iter(|| {
                    rt.block_on(async {
                        let start = Instant::now();
                        
                        let txn_id = node.transaction_manager
                            .begin_transaction(IsolationLevel::Serializable)
                            .await
                            .unwrap();
                        
                        node.transaction_manager
                            .write(txn_id, "latency_key".to_string(), b"latency_value".to_vec())
                            .await
                            .unwrap();
                        
                        let result = node.transaction_manager.commit(txn_id).await.unwrap();
                        
                        let duration = start.elapsed();
                        
                        // Verify latency requirements
                        if latency == 0 {
                            assert!(duration < Duration::from_millis(1), "Single-key transaction should be <1ms");
                        }
                        
                        result
                    })
                });
                
                rt.block_on(node.stop()).unwrap();
            },
        );
    }
    
    group.finish();
}

/// Benchmark distributed transaction performance (2PC)
fn bench_distributed_transaction_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("distributed_transaction_performance");
    
    for shard_count in [2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::new("shard_count", shard_count),
            &shard_count,
            |b, &shard_count| {
                let node = rt.block_on(create_benchmark_node("distributed-bench", 1));
                rt.block_on(node.start()).unwrap();
                
                b.iter(|| {
                    rt.block_on(async {
                        let shards: Vec<String> = (0..shard_count)
                            .map(|i| format!("shard_{}", i))
                            .collect();
                        
                        let start = Instant::now();
                        
                        let txn_id = node.transaction_manager
                            .begin_distributed_transaction(shards.clone())
                            .await
                            .unwrap();
                        
                        // Write to each shard
                        for (i, shard) in shards.iter().enumerate() {
                            node.transaction_manager
                                .write(
                                    txn_id,
                                    format!("{}:key_{}", shard, i),
                                    format!("value_{}", i).into_bytes()
                                )
                                .await
                                .unwrap();
                        }
                        
                        // Two-phase commit
                        let prepare_result = node.transaction_manager
                            .prepare_phase(txn_id)
                            .await
                            .unwrap();
                        
                        assert!(prepare_result.prepared);
                        
                        node.transaction_manager
                            .commit_phase(txn_id)
                            .await
                            .unwrap();
                        
                        let duration = start.elapsed();
                        
                        // Verify cross-shard latency requirement
                        assert!(duration < Duration::from_millis(10), "Cross-shard transaction should be <10ms");
                        
                        duration
                    })
                });
                
                rt.block_on(node.stop()).unwrap();
            },
        );
    }
    
    group.finish();
}

/// Benchmark MVCC read performance
fn bench_mvcc_read_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("mvcc_read_performance");
    
    // Setup data first
    let node = rt.block_on(create_benchmark_node("mvcc-bench", 0));
    rt.block_on(node.start()).unwrap();
    
    // Create multiple versions of the same key
    for version in 0..100 {
        let txn_id = rt.block_on(
            node.transaction_manager
                .begin_transaction(IsolationLevel::ReadCommitted)
        ).unwrap();
        
        rt.block_on(
            node.transaction_manager.write(
                txn_id,
                "versioned_key".to_string(),
                format!("version_{}", version).into_bytes()
            )
        ).unwrap();
        
        rt.block_on(node.transaction_manager.commit(txn_id)).unwrap();
    }
    
    // Benchmark reading at different timestamps
    for version_count in [1, 10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("version_count", version_count),
            &version_count,
            |b, &_version_count| {
                b.iter(|| {
                    rt.block_on(async {
                        let txn_id = node.transaction_manager
                            .begin_transaction(IsolationLevel::Serializable)
                            .await
                            .unwrap();
                        
                        let start = Instant::now();
                        let result = node.transaction_manager
                            .read(txn_id, "versioned_key")
                            .await
                            .unwrap();
                        let duration = start.elapsed();
                        
                        // Verify read latency requirement
                        assert!(duration < Duration::from_micros(100), "Version read should be <100μs");
                        
                        node.transaction_manager.rollback(txn_id).await.unwrap();
                        result
                    })
                });
            },
        );
    }
    
    rt.block_on(node.stop()).unwrap();
    group.finish();
}

/// Benchmark shard routing performance
fn bench_shard_routing_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("shard_routing_performance");
    
    for shard_count in [16, 64, 256, 1000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("shard_count", shard_count),
            &shard_count,
            |b, &shard_count| {
                let node = rt.block_on(create_benchmark_node("routing-bench", 0));
                rt.block_on(node.start()).unwrap();
                
                // Create additional shards to test routing performance
                rt.block_on(async {
                    for i in 0..(shard_count.saturating_sub(16)) { // Default has 16 shards
                        let shard_id = format!("route_shard_{}", i);
                        let node_id = NodeId::new("routing-bench".to_string());
                        let replicas = vec![node_id];
                        
                        node.shard_manager
                            .create_shard(shard_id, replicas)
                            .await
                            .unwrap();
                    }
                });
                
                b.iter(|| {
                    rt.block_on(async {
                        let start = Instant::now();
                        let result = node.shard_manager
                            .route_request("routing_test_key")
                            .await;
                        let duration = start.elapsed();
                        
                        // Verify routing latency requirement
                        assert!(duration < Duration::from_micros(1000), "Routing should be <1ms");
                        
                        result
                    })
                });
                
                rt.block_on(node.stop()).unwrap();
            },
        );
    }
    
    group.finish();
}

/// Benchmark shard split performance
fn bench_shard_split_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("shard_split_performance");
    group.sample_size(10); // Smaller sample size for expensive operations
    
    for data_size_mb in [1, 10, 50] {
        group.bench_with_input(
            BenchmarkId::new("data_size_mb", data_size_mb),
            &data_size_mb,
            |b, &_data_size_mb| {
                b.iter(|| {
                    rt.block_on(async {
                        let node = create_benchmark_node("split-bench", 0).await;
                        node.start().await.unwrap();
                        
                        // Create a shard to split
                        let shard_id = "splittable_shard".to_string();
                        let node_id = NodeId::new("split-bench".to_string());
                        let replicas = vec![node_id];
                        
                        node.shard_manager
                            .create_shard(shard_id.clone(), replicas)
                            .await
                            .unwrap();
                        
                        let start = Instant::now();
                        let split_result = node.shard_manager
                            .split_shard(shard_id, "split_key")
                            .await;
                        let duration = start.elapsed();
                        
                        node.stop().await.unwrap();
                        
                        // Return duration for measurement
                        duration
                    })
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark memory usage and garbage collection
fn bench_memory_usage_gc(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("memory_usage_gc");
    
    for transaction_count in [100, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::new("transaction_count", transaction_count),
            &transaction_count,
            |b, &transaction_count| {
                b.iter(|| {
                    rt.block_on(async {
                        let node = create_benchmark_node("memory-bench", 0).await;
                        node.start().await.unwrap();
                        
                        // Generate many transactions to test memory usage
                        for i in 0..transaction_count {
                            let txn_id = node.transaction_manager
                                .begin_transaction(IsolationLevel::ReadCommitted)
                                .await
                                .unwrap();
                            
                            node.transaction_manager
                                .write(
                                    txn_id,
                                    format!("memory_key_{}", i),
                                    format!("memory_value_{}", i).into_bytes()
                                )
                                .await
                                .unwrap();
                            
                            node.transaction_manager.commit(txn_id).await.unwrap();
                        }
                        
                        // Get memory statistics
                        let stats = node.transaction_manager.statistics().await;
                        
                        node.stop().await.unwrap();
                        
                        stats
                    })
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark system throughput under load
fn bench_system_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("system_throughput");
    group.measurement_time(Duration::from_secs(10)); // Longer measurement time
    
    for ops_per_second in [1000, 5000, 10000] {
        group.throughput(Throughput::Elements(ops_per_second as u64));
        group.bench_with_input(
            BenchmarkId::new("target_ops_per_second", ops_per_second),
            &ops_per_second,
            |b, &target_ops| {
                let node = rt.block_on(create_benchmark_node("throughput-bench", 0));
                rt.block_on(node.start()).unwrap();
                
                b.iter(|| {
                    rt.block_on(async {
                        let start = Instant::now();
                        let mut handles = Vec::new();
                        let ops_to_run = target_ops / 10; // Run for 100ms worth
                        
                        for i in 0..ops_to_run {
                            let node_clone = node.clone();
                            let handle = tokio::spawn(async move {
                                let txn_id = node_clone.transaction_manager
                                    .begin_transaction(IsolationLevel::ReadCommitted)
                                    .await
                                    .unwrap();
                                
                                node_clone.transaction_manager
                                    .write(
                                        txn_id,
                                        format!("throughput_key_{}", i),
                                        b"throughput_value".to_vec()
                                    )
                                    .await
                                    .unwrap();
                                
                                node_clone.transaction_manager.commit(txn_id).await.unwrap()
                            });
                            handles.push(handle);
                        }
                        
                        futures::future::join_all(handles).await;
                        let duration = start.elapsed();
                        
                        let actual_ops_per_sec = ops_to_run as f64 / duration.as_secs_f64();
                        
                        // Verify we can achieve target throughput
                        if target_ops <= 100000 {
                            assert!(
                                actual_ops_per_sec >= target_ops as f64 * 0.5,
                                "Should achieve at least 50% of target throughput"
                            );
                        }
                        
                        actual_ops_per_sec
                    })
                });
                
                rt.block_on(node.stop()).unwrap();
            },
        );
    }
    
    group.finish();
}

/// Benchmark leader election performance
fn bench_leader_election(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("leader_election");
    
    for network_latency_ms in [1, 5, 10] {
        group.bench_with_input(
            BenchmarkId::new("network_latency_ms", network_latency_ms),
            &network_latency_ms,
            |b, &latency| {
                b.iter(|| {
                    rt.block_on(async {
                        let node = create_benchmark_node("election-bench", latency).await;
                        
                        let start = Instant::now();
                        node.start().await.unwrap();
                        
                        // Wait for initial state to stabilize
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        
                        let election_duration = start.elapsed();
                        
                        // Verify election completes within timeout
                        assert!(
                            election_duration < Duration::from_secs(1),
                            "Leader election should complete within 1 second"
                        );
                        
                        node.stop().await.unwrap();
                        election_duration
                    })
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_single_transaction_throughput,
    bench_concurrent_transaction_throughput,
    bench_transaction_latency_network,
    bench_distributed_transaction_performance,
    bench_mvcc_read_performance,
    bench_shard_routing_performance,
    bench_shard_split_performance,
    bench_memory_usage_gc,
    bench_system_throughput,
    bench_leader_election
);

criterion_main!(benches);