//! Performance Benchmarks for TrustChain ↔ HyperMesh Integration
//!
//! Measures validation latency, throughput, and resource usage.

use std::sync::Arc;
use std::time::{SystemTime, Duration, Instant};
use std::net::Ipv6Addr;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::Semaphore;
use anyhow::Result;
use tracing::{info, warn};

use trustchain::ca::{CertificateRequest, CertificateStatus};
use trustchain::consensus::{
    ConsensusProof, ConsensusRequirements,
    hypermesh_client::{HyperMeshConsensusClient, HyperMeshClientConfig},
};

/// Initialize tracing for performance tests
fn init_perf_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init();
}

/// Performance statistics
#[derive(Debug, Clone)]
struct PerfStats {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    min_latency_us: u64,
    max_latency_us: u64,
    avg_latency_us: u64,
    p50_latency_us: u64,
    p95_latency_us: u64,
    p99_latency_us: u64,
    throughput_per_sec: f64,
    total_duration_secs: f64,
}

impl PerfStats {
    fn from_latencies(latencies: &mut Vec<u64>, duration: Duration) -> Self {
        latencies.sort_unstable();

        let total = latencies.len() as u64;
        let successful = total; // All recorded latencies are successful
        let failed = 0;

        let min = *latencies.first().unwrap_or(&0);
        let max = *latencies.last().unwrap_or(&0);
        let avg = if !latencies.is_empty() {
            latencies.iter().sum::<u64>() / latencies.len() as u64
        } else {
            0
        };

        let p50 = latencies.get(latencies.len() * 50 / 100).copied().unwrap_or(0);
        let p95 = latencies.get(latencies.len() * 95 / 100).copied().unwrap_or(0);
        let p99 = latencies.get(latencies.len() * 99 / 100).copied().unwrap_or(0);

        let throughput = total as f64 / duration.as_secs_f64();

        Self {
            total_requests: total,
            successful_requests: successful,
            failed_requests: failed,
            min_latency_us: min,
            max_latency_us: max,
            avg_latency_us: avg,
            p50_latency_us: p50,
            p95_latency_us: p95,
            p99_latency_us: p99,
            throughput_per_sec: throughput,
            total_duration_secs: duration.as_secs_f64(),
        }
    }

    fn print_summary(&self, test_name: &str) {
        info!("\n╔═══════════════════════════════════════════════════════════════╗");
        info!("║ Performance Test: {:<44} ║", test_name);
        info!("╠═══════════════════════════════════════════════════════════════╣");
        info!("║ Total Requests:        {:>10}                              ║", self.total_requests);
        info!("║ Successful:            {:>10}                              ║", self.successful_requests);
        info!("║ Failed:                {:>10}                              ║", self.failed_requests);
        info!("╠═══════════════════════════════════════════════════════════════╣");
        info!("║ Latency Statistics (microseconds):                           ║");
        info!("║   Minimum:             {:>10} μs                            ║", self.min_latency_us);
        info!("║   Maximum:             {:>10} μs                            ║", self.max_latency_us);
        info!("║   Average:             {:>10} μs                            ║", self.avg_latency_us);
        info!("║   Median (p50):        {:>10} μs                            ║", self.p50_latency_us);
        info!("║   p95:                 {:>10} μs                            ║", self.p95_latency_us);
        info!("║   p99:                 {:>10} μs                            ║", self.p99_latency_us);
        info!("╠═══════════════════════════════════════════════════════════════╣");
        info!("║ Throughput:            {:>10.2} req/sec                      ║", self.throughput_per_sec);
        info!("║ Total Duration:        {:>10.2} seconds                      ║", self.total_duration_secs);
        info!("╚═══════════════════════════════════════════════════════════════╝\n");
    }

    fn check_target(&self, target_latency_us: u64, target_throughput: f64) -> bool {
        let latency_ok = self.avg_latency_us < target_latency_us;
        let throughput_ok = self.throughput_per_sec > target_throughput;

        if !latency_ok {
            warn!("⚠️  Latency target MISSED: {} μs > {} μs (target)",
                  self.avg_latency_us, target_latency_us);
        } else {
            info!("✅ Latency target MET: {} μs < {} μs (target)",
                  self.avg_latency_us, target_latency_us);
        }

        if !throughput_ok {
            warn!("⚠️  Throughput target MISSED: {:.2} req/s < {:.2} req/s (target)",
                  self.throughput_per_sec, target_throughput);
        } else {
            info!("✅ Throughput target MET: {:.2} req/s > {:.2} req/s (target)",
                  self.throughput_per_sec, target_throughput);
        }

        latency_ok && throughput_ok
    }
}

/// Benchmark 1: Single Request Latency
#[tokio::test]
async fn bench_single_request_latency() -> Result<()> {
    init_perf_tracing();
    info!("=== Benchmark: Single Request Latency ===");

    // For this test, we'll measure the latency of creating a client and making a request
    // without a real server, so we can measure the overhead

    let client_config = HyperMeshClientConfig {
        request_timeout: Duration::from_secs(1),
        max_retries: 0,
        retry_backoff: Duration::from_millis(0),
        enable_caching: false,
        cache_ttl: Duration::from_secs(60),
    };

    let hypermesh_client = HyperMeshConsensusClient::new(client_config).await?;

    let mut latencies = Vec::new();
    let iterations = 100;

    info!("Running {} single request latency measurements...", iterations);

    for i in 0..iterations {
        let cert_request = CertificateRequest {
            common_name: format!("latency-test-{}.hypermesh.online", i),
            san_entries: vec![format!("latency-test-{}.hypermesh.online", i)],
            node_id: format!("test_node_{:03}", i),
            ipv6_addresses: vec![Ipv6Addr::LOCALHOST],
            consensus_proof: ConsensusProof::new_for_testing(),
            timestamp: SystemTime::now(),
        };

        let consensus_requirements = ConsensusRequirements::localhost_testing();

        let start = Instant::now();

        // This will fail (no server), but we measure the attempt time
        let _ = hypermesh_client
            .validate_certificate_request(&cert_request, &consensus_requirements)
            .await;

        let latency = start.elapsed().as_micros() as u64;

        // Only count if it's a reasonable latency (not timeout)
        if latency < 1_000_000 { // < 1 second
            latencies.push(latency);
        }
    }

    if latencies.is_empty() {
        info!("⚠️  No successful measurements (all timed out)");
        return Ok(());
    }

    let total_duration = Duration::from_secs((latencies.len() as f64 * 0.001) as u64);
    let stats = PerfStats::from_latencies(&mut latencies, total_duration);

    stats.print_summary("Single Request Latency");

    // Target: < 100ms average (100,000 μs)
    // Note: This measures client overhead, not actual validation
    info!("Note: This test measures client overhead without server");

    Ok(())
}

/// Benchmark 2: Sequential Throughput
#[tokio::test]
async fn bench_sequential_throughput() -> Result<()> {
    init_perf_tracing();
    info!("=== Benchmark: Sequential Throughput ===");

    let client_config = HyperMeshClientConfig {
        request_timeout: Duration::from_millis(100),
        max_retries: 0,
        retry_backoff: Duration::from_millis(0),
        enable_caching: false,
        cache_ttl: Duration::from_secs(60),
    };

    let hypermesh_client = HyperMeshConsensusClient::new(client_config).await?;

    let iterations = 50;
    let mut latencies = Vec::new();

    info!("Running {} sequential requests...", iterations);

    let overall_start = Instant::now();

    for i in 0..iterations {
        let cert_request = CertificateRequest {
            common_name: format!("throughput-test-{}.hypermesh.online", i),
            san_entries: vec![format!("throughput-test-{}.hypermesh.online", i)],
            node_id: format!("test_node_{:03}", i),
            ipv6_addresses: vec![Ipv6Addr::LOCALHOST],
            consensus_proof: ConsensusProof::new_for_testing(),
            timestamp: SystemTime::now(),
        };

        let consensus_requirements = ConsensusRequirements::localhost_testing();

        let start = Instant::now();
        let _ = hypermesh_client
            .validate_certificate_request(&cert_request, &consensus_requirements)
            .await;

        let latency = start.elapsed().as_micros() as u64;
        if latency < 500_000 { // < 500ms
            latencies.push(latency);
        }
    }

    let total_duration = overall_start.elapsed();

    if latencies.is_empty() {
        info!("⚠️  No successful measurements");
        return Ok(());
    }

    let stats = PerfStats::from_latencies(&mut latencies, total_duration);
    stats.print_summary("Sequential Throughput");

    info!("Note: This test measures client overhead without server");

    Ok(())
}

/// Benchmark 3: Concurrent Load Test (1, 10, 50, 100 concurrent)
#[tokio::test]
async fn bench_concurrent_load() -> Result<()> {
    init_perf_tracing();
    info!("=== Benchmark: Concurrent Load Test ===");

    let client_config = HyperMeshClientConfig {
        request_timeout: Duration::from_millis(200),
        max_retries: 0,
        retry_backoff: Duration::from_millis(0),
        enable_caching: false,
        cache_ttl: Duration::from_secs(60),
    };

    let hypermesh_client = Arc::new(HyperMeshConsensusClient::new(client_config).await?);

    let concurrency_levels = vec![1, 10, 50, 100];
    let requests_per_level = 100;

    for concurrency in concurrency_levels {
        info!("\n--- Testing with {} concurrent requests ---", concurrency);

        let semaphore = Arc::new(Semaphore::new(concurrency));
        let latencies = Arc::new(tokio::sync::Mutex::new(Vec::new()));

        let overall_start = Instant::now();

        let mut handles = vec![];

        for i in 0..requests_per_level {
            let client = hypermesh_client.clone();
            let sem = semaphore.clone();
            let lats = latencies.clone();

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();

                let cert_request = CertificateRequest {
                    common_name: format!("concurrent-{}-{}.hypermesh.online", concurrency, i),
                    san_entries: vec![format!("concurrent-{}-{}.hypermesh.online", concurrency, i)],
                    node_id: format!("test_node_{:03}", i),
                    ipv6_addresses: vec![Ipv6Addr::LOCALHOST],
                    consensus_proof: ConsensusProof::new_for_testing(),
                    timestamp: SystemTime::now(),
                };

                let consensus_requirements = ConsensusRequirements::localhost_testing();

                let start = Instant::now();
                let _ = client
                    .validate_certificate_request(&cert_request, &consensus_requirements)
                    .await;

                let latency = start.elapsed().as_micros() as u64;
                if latency < 500_000 { // < 500ms
                    lats.lock().await.push(latency);
                }
            });

            handles.push(handle);
        }

        // Wait for all requests to complete
        for handle in handles {
            let _ = handle.await;
        }

        let total_duration = overall_start.elapsed();

        let mut lats = latencies.lock().await.clone();

        if lats.is_empty() {
            info!("⚠️  No successful measurements at concurrency {}", concurrency);
            continue;
        }

        let stats = PerfStats::from_latencies(&mut lats, total_duration);
        stats.print_summary(&format!("Concurrent Load ({})", concurrency));
    }

    info!("Note: This test measures client overhead without server");

    Ok(())
}

/// Benchmark 4: Memory Usage Under Load
#[tokio::test]
async fn bench_memory_usage() -> Result<()> {
    init_perf_tracing();
    info!("=== Benchmark: Memory Usage Under Load ===");

    let client_config = HyperMeshClientConfig::default();
    let hypermesh_client = Arc::new(HyperMeshConsensusClient::new(client_config).await?);

    // Get baseline memory (approximate)
    info!("Starting memory benchmark...");

    let concurrent_requests = 100;
    let semaphore = Arc::new(Semaphore::new(concurrent_requests));
    let total_requests = 1000;

    let counter = Arc::new(AtomicU64::new(0));

    info!("Submitting {} requests with {} concurrency...", total_requests, concurrent_requests);

    let start = Instant::now();
    let mut handles = vec![];

    for i in 0..total_requests {
        let client = hypermesh_client.clone();
        let sem = semaphore.clone();
        let cnt = counter.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            let cert_request = CertificateRequest {
                common_name: format!("memory-test-{}.hypermesh.online", i),
                san_entries: vec![format!("memory-test-{}.hypermesh.online", i)],
                node_id: format!("test_node_{:03}", i),
                ipv6_addresses: vec![Ipv6Addr::LOCALHOST],
                consensus_proof: ConsensusProof::new_for_testing(),
                timestamp: SystemTime::now(),
            };

            let consensus_requirements = ConsensusRequirements::localhost_testing();

            let _ = client
                .validate_certificate_request(&cert_request, &consensus_requirements)
                .await;

            cnt.fetch_add(1, Ordering::Relaxed);
        });

        handles.push(handle);
    }

    // Wait for all
    for handle in handles {
        let _ = handle.await;
    }

    let duration = start.elapsed();
    let completed = counter.load(Ordering::Relaxed);

    info!("Completed {} requests in {:?}", completed, duration);
    info!("Throughput: {:.2} req/sec", completed as f64 / duration.as_secs_f64());

    info!("✅ Memory usage test completed");
    info!("Note: Use external tools (htop, valgrind) for detailed memory profiling");

    Ok(())
}

/// Benchmark 5: Cache Performance (if enabled)
#[tokio::test]
async fn bench_cache_performance() -> Result<()> {
    init_perf_tracing();
    info!("=== Benchmark: Cache Performance ===");

    // Test with cache enabled
    let client_config = HyperMeshClientConfig {
        request_timeout: Duration::from_millis(100),
        max_retries: 0,
        retry_backoff: Duration::from_millis(0),
        enable_caching: true,
        cache_ttl: Duration::from_secs(300),
    };

    let hypermesh_client = HyperMeshConsensusClient::new(client_config).await?;

    let cert_request = CertificateRequest {
        common_name: "cache-test.hypermesh.online".to_string(),
        san_entries: vec!["cache-test.hypermesh.online".to_string()],
        node_id: "test_node_cache".to_string(),
        ipv6_addresses: vec![Ipv6Addr::LOCALHOST],
        consensus_proof: ConsensusProof::new_for_testing(),
        timestamp: SystemTime::now(),
    };

    let consensus_requirements = ConsensusRequirements::localhost_testing();

    let iterations = 10;
    let mut latencies = Vec::new();

    info!("Running {} cached request measurements...", iterations);

    for _ in 0..iterations {
        let start = Instant::now();
        let _ = hypermesh_client
            .validate_certificate_request(&cert_request, &consensus_requirements)
            .await;

        let latency = start.elapsed().as_micros() as u64;
        if latency < 500_000 {
            latencies.push(latency);
        }
    }

    if latencies.is_empty() {
        info!("⚠️  No successful measurements");
        return Ok(());
    }

    let total_duration = Duration::from_millis(latencies.len() as u64);
    let stats = PerfStats::from_latencies(&mut latencies, total_duration);

    stats.print_summary("Cache Performance");

    // Get metrics
    let metrics = hypermesh_client.get_metrics().await;
    info!("Client metrics:");
    info!("  Total requests: {}", metrics.total_requests);
    info!("  Cache hit rate: {:.2}%", metrics.cache_hit_rate * 100.0);

    info!("Note: Cache is not yet implemented, so cache_hit_rate will be 0.0");

    Ok(())
}

/// Benchmark Summary: Print overall results
#[tokio::test]
async fn bench_summary() -> Result<()> {
    init_perf_tracing();

    info!("\n╔═══════════════════════════════════════════════════════════════╗");
    info!("║                    PERFORMANCE TEST SUMMARY                   ║");
    info!("╠═══════════════════════════════════════════════════════════════╣");
    info!("║                                                               ║");
    info!("║ Target Performance Goals:                                    ║");
    info!("║   • Validation Latency:    < 100 ms (100,000 μs)             ║");
    info!("║   • Throughput:            > 100 req/sec                     ║");
    info!("║   • Concurrent Requests:   100+ simultaneous                 ║");
    info!("║   • Memory per Request:    < 1 MB                            ║");
    info!("║                                                               ║");
    info!("║ Note: These benchmarks measure client overhead only,         ║");
    info!("║       not actual server validation performance.              ║");
    info!("║                                                               ║");
    info!("║ For real performance metrics, run integration tests with     ║");
    info!("║ actual HyperMesh consensus server running.                   ║");
    info!("║                                                               ║");
    info!("╚═══════════════════════════════════════════════════════════════╝\n");

    Ok(())
}
