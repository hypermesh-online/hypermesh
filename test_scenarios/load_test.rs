//! Performance and load testing for Catalog plugin system
//!
//! This module provides comprehensive load testing to validate:
//! - Plugin loading performance
//! - Concurrent operation handling
//! - Resource usage under stress
//! - Scalability limits
//! - Performance regression detection

use catalog::CatalogExtension;
use hypermesh::assets::core::AssetManager;
use hypermesh::extensions::{ExtensionLoader, ExtensionRequest, LoaderConfig, ResourceLimits};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::Semaphore;
use tokio::time::{sleep, timeout};
use tracing::{info, debug, warn, error};

/// Performance metrics collector
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub total_duration: Duration,
    pub min_latency: Duration,
    pub max_latency: Duration,
    pub avg_latency: Duration,
    pub p50_latency: Duration,
    pub p95_latency: Duration,
    pub p99_latency: Duration,
    pub throughput_ops_per_sec: f64,
    pub cpu_usage_percent: f32,
    pub memory_usage_mb: f32,
    pub errors: HashMap<String, u32>,
}

impl PerformanceMetrics {
    fn new() -> Self {
        Self {
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            total_duration: Duration::ZERO,
            min_latency: Duration::MAX,
            max_latency: Duration::ZERO,
            avg_latency: Duration::ZERO,
            p50_latency: Duration::ZERO,
            p95_latency: Duration::ZERO,
            p99_latency: Duration::ZERO,
            throughput_ops_per_sec: 0.0,
            cpu_usage_percent: 0.0,
            memory_usage_mb: 0.0,
            errors: HashMap::new(),
        }
    }

    fn calculate_percentiles(&mut self, latencies: &mut Vec<Duration>) {
        if latencies.is_empty() {
            return;
        }

        latencies.sort();

        self.min_latency = latencies[0];
        self.max_latency = latencies[latencies.len() - 1];

        let sum: Duration = latencies.iter().sum();
        self.avg_latency = sum / latencies.len() as u32;

        self.p50_latency = latencies[latencies.len() * 50 / 100];
        self.p95_latency = latencies[latencies.len() * 95 / 100];
        self.p99_latency = latencies[latencies.len() * 99 / 100];
    }

    fn print_summary(&self) {
        println!("\n=== Performance Test Results ===");
        println!("Total Operations: {}", self.total_operations);
        println!("Successful: {} ({:.1}%)",
                 self.successful_operations,
                 self.successful_operations as f64 / self.total_operations as f64 * 100.0);
        println!("Failed: {} ({:.1}%)",
                 self.failed_operations,
                 self.failed_operations as f64 / self.total_operations as f64 * 100.0);
        println!("\n=== Latency Statistics ===");
        println!("Min: {:.2}ms", self.min_latency.as_secs_f64() * 1000.0);
        println!("Avg: {:.2}ms", self.avg_latency.as_secs_f64() * 1000.0);
        println!("Max: {:.2}ms", self.max_latency.as_secs_f64() * 1000.0);
        println!("P50: {:.2}ms", self.p50_latency.as_secs_f64() * 1000.0);
        println!("P95: {:.2}ms", self.p95_latency.as_secs_f64() * 1000.0);
        println!("P99: {:.2}ms", self.p99_latency.as_secs_f64() * 1000.0);
        println!("\n=== Throughput ===");
        println!("Rate: {:.2} ops/sec", self.throughput_ops_per_sec);
        println!("Duration: {:.2}s", self.total_duration.as_secs_f64());
        println!("\n=== Resource Usage ===");
        println!("CPU: {:.1}%", self.cpu_usage_percent);
        println!("Memory: {:.1}MB", self.memory_usage_mb);

        if !self.errors.is_empty() {
            println!("\n=== Errors ===");
            for (error, count) in &self.errors {
                println!("{}: {}", error, count);
            }
        }
    }
}

/// Load test configuration
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    pub total_operations: u64,
    pub concurrent_workers: usize,
    pub operations_per_second: Option<u64>,
    pub test_duration_secs: Option<u64>,
    pub warmup_operations: u64,
    pub operation_timeout_ms: u64,
    pub measure_resources: bool,
}

impl Default for LoadTestConfig {
    fn default() -> Self {
        Self {
            total_operations: 1000,
            concurrent_workers: 10,
            operations_per_second: None,
            test_duration_secs: None,
            warmup_operations: 100,
            operation_timeout_ms: 5000,
            measure_resources: true,
        }
    }
}

/// Main load test executor
pub struct LoadTester {
    config: LoadTestConfig,
    loader: Arc<ExtensionLoader>,
    extension_id: String,
    metrics: Arc<tokio::sync::Mutex<PerformanceMetrics>>,
    operation_counter: Arc<AtomicU64>,
    latencies: Arc<tokio::sync::Mutex<Vec<Duration>>>,
}

impl LoadTester {
    pub async fn new(config: LoadTestConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize loader
        let loader_config = LoaderConfig {
            search_paths: vec![
                PathBuf::from("../catalog/target/release"),
                PathBuf::from("../catalog/target/debug"),
            ],
            enable_wasm: false,
            verify_signatures: false,
            max_extensions: 10,
            default_limits: ResourceLimits::unlimited(),
            trustchain_cert_path: None,
        };

        let loader = Arc::new(ExtensionLoader::new(loader_config));

        // Load catalog extension
        let extension_path = PathBuf::from("../catalog/target/release");
        let extension_id = loader.load_extension(&extension_path).await?;

        Ok(Self {
            config,
            loader,
            extension_id,
            metrics: Arc::new(tokio::sync::Mutex::new(PerformanceMetrics::new())),
            operation_counter: Arc::new(AtomicU64::new(0)),
            latencies: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        })
    }

    /// Run the load test
    pub async fn run(&self) -> Result<PerformanceMetrics, Box<dyn std::error::Error>> {
        info!("Starting load test with {} workers", self.config.concurrent_workers);

        // Warmup phase
        if self.config.warmup_operations > 0 {
            info!("Running {} warmup operations...", self.config.warmup_operations);
            self.run_warmup().await?;
        }

        // Main test phase
        let start = Instant::now();
        let rate_limiter = self.create_rate_limiter();

        let mut handles = vec![];
        for worker_id in 0..self.config.concurrent_workers {
            let handle = self.spawn_worker(worker_id, rate_limiter.clone());
            handles.push(handle);
        }

        // Wait for completion
        for handle in handles {
            handle.await?;
        }

        let total_duration = start.elapsed();

        // Calculate final metrics
        let mut metrics = self.metrics.lock().await;
        let mut latencies = self.latencies.lock().await;

        metrics.total_duration = total_duration;
        metrics.throughput_ops_per_sec =
            metrics.successful_operations as f64 / total_duration.as_secs_f64();
        metrics.calculate_percentiles(&mut latencies);

        if self.config.measure_resources {
            self.measure_resources(&mut metrics).await;
        }

        Ok(metrics.clone())
    }

    /// Run warmup operations
    async fn run_warmup(&self) -> Result<(), Box<dyn std::error::Error>> {
        let extension = self.loader.get_extension(&self.extension_id).await
            .ok_or("Extension not found")?;

        for i in 0..self.config.warmup_operations {
            let request = self.create_test_request(i);
            let _ = timeout(
                Duration::from_millis(self.config.operation_timeout_ms),
                extension.handle_request(request)
            ).await;

            if i % 10 == 0 {
                debug!("Warmup progress: {}/{}", i, self.config.warmup_operations);
            }
        }

        info!("Warmup completed");
        Ok(())
    }

    /// Spawn a worker task
    fn spawn_worker(&self, worker_id: usize, rate_limiter: Arc<Semaphore>) -> tokio::task::JoinHandle<()> {
        let loader = self.loader.clone();
        let extension_id = self.extension_id.clone();
        let metrics = self.metrics.clone();
        let counter = self.operation_counter.clone();
        let latencies = self.latencies.clone();
        let total_ops = self.config.total_operations;
        let timeout_ms = self.config.operation_timeout_ms;

        tokio::spawn(async move {
            let extension = match loader.get_extension(&extension_id).await {
                Some(ext) => ext,
                None => {
                    error!("Worker {} failed to get extension", worker_id);
                    return;
                }
            };

            loop {
                // Check if we've reached the target
                let op_num = counter.fetch_add(1, Ordering::SeqCst);
                if op_num >= total_ops {
                    break;
                }

                // Rate limiting
                let _permit = rate_limiter.acquire().await.ok();

                // Execute operation
                let request = Self::create_test_request(op_num);
                let start = Instant::now();

                let result = timeout(
                    Duration::from_millis(timeout_ms),
                    extension.handle_request(request)
                ).await;

                let latency = start.elapsed();

                // Update metrics
                let mut metrics = metrics.lock().await;
                let mut lats = latencies.lock().await;

                metrics.total_operations += 1;
                lats.push(latency);

                match result {
                    Ok(Ok(response)) if response.success => {
                        metrics.successful_operations += 1;
                    }
                    Ok(Ok(response)) => {
                        metrics.failed_operations += 1;
                        if let Some(error) = response.error {
                            *metrics.errors.entry(error).or_insert(0) += 1;
                        }
                    }
                    Ok(Err(e)) => {
                        metrics.failed_operations += 1;
                        *metrics.errors.entry(e.to_string()).or_insert(0) += 1;
                    }
                    Err(_) => {
                        metrics.failed_operations += 1;
                        *metrics.errors.entry("timeout".to_string()).or_insert(0) += 1;
                    }
                }

                if op_num % 100 == 0 {
                    debug!("Worker {} progress: {}/{}", worker_id, op_num, total_ops);
                }
            }

            debug!("Worker {} completed", worker_id);
        })
    }

    /// Create rate limiter if needed
    fn create_rate_limiter(&self) -> Arc<Semaphore> {
        if let Some(ops_per_sec) = self.config.operations_per_second {
            Arc::new(Semaphore::new(ops_per_sec as usize))
        } else {
            Arc::new(Semaphore::new(usize::MAX))
        }
    }

    /// Create a test request
    fn create_test_request(operation_id: u64) -> ExtensionRequest {
        // Vary the request type to simulate real usage
        match operation_id % 10 {
            0..=3 => ExtensionRequest {
                id: format!("load-test-{}", operation_id),
                method: "list_packages".to_string(),
                params: serde_json::json!({
                    "limit": 10,
                    "offset": (operation_id % 100) * 10
                }),
                consensus_proof: None,
            },
            4..=6 => ExtensionRequest {
                id: format!("load-test-{}", operation_id),
                method: "search_packages".to_string(),
                params: serde_json::json!({
                    "query": format!("test-{}", operation_id % 20),
                    "limit": 5
                }),
                consensus_proof: None,
            },
            7..=8 => ExtensionRequest {
                id: format!("load-test-{}", operation_id),
                method: "get_package".to_string(),
                params: serde_json::json!({
                    "id": format!("pkg-{}", operation_id % 50)
                }),
                consensus_proof: None,
            },
            _ => ExtensionRequest {
                id: format!("load-test-{}", operation_id),
                method: "health_check".to_string(),
                params: serde_json::json!({}),
                consensus_proof: None,
            },
        }
    }

    /// Measure resource usage
    async fn measure_resources(&self, metrics: &mut PerformanceMetrics) {
        // Get process statistics
        use sysinfo::{System, SystemExt, ProcessExt, PidExt};

        let mut sys = System::new_all();
        sys.refresh_all();

        let pid = sysinfo::get_current_pid().ok();
        if let Some(pid) = pid {
            if let Some(process) = sys.process(pid) {
                metrics.cpu_usage_percent = process.cpu_usage();
                metrics.memory_usage_mb = process.memory() as f32 / 1024.0 / 1024.0;
            }
        }
    }
}

/// Stress test to find breaking point
pub async fn stress_test() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting stress test to find breaking point...");

    let mut current_load = 10;
    let mut max_sustainable_load = 0;

    loop {
        info!("Testing with {} ops/sec", current_load);

        let config = LoadTestConfig {
            total_operations: current_load * 10,
            concurrent_workers: std::cmp::min(current_load / 10, 100),
            operations_per_second: Some(current_load),
            test_duration_secs: Some(10),
            warmup_operations: 50,
            operation_timeout_ms: 5000,
            measure_resources: true,
        };

        let tester = LoadTester::new(config).await?;
        let metrics = tester.run().await?;

        // Check if system is still stable
        let success_rate = metrics.successful_operations as f64 / metrics.total_operations as f64;
        let p99_acceptable = metrics.p99_latency < Duration::from_millis(1000);

        if success_rate >= 0.99 && p99_acceptable {
            max_sustainable_load = current_load;
            current_load = (current_load as f64 * 1.5) as u64;

            info!("System stable at {} ops/sec, increasing load", max_sustainable_load);
        } else {
            info!("System unstable at {} ops/sec", current_load);
            info!("Maximum sustainable load: {} ops/sec", max_sustainable_load);

            metrics.print_summary();
            break;
        }

        if current_load > 10000 {
            info!("Reached maximum test load of 10000 ops/sec");
            break;
        }
    }

    Ok(())
}

/// Endurance test for long-running stability
pub async fn endurance_test(duration_minutes: u64) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting {}-minute endurance test...", duration_minutes);

    let config = LoadTestConfig {
        total_operations: u64::MAX,
        concurrent_workers: 20,
        operations_per_second: Some(100),
        test_duration_secs: Some(duration_minutes * 60),
        warmup_operations: 1000,
        operation_timeout_ms: 5000,
        measure_resources: true,
    };

    let tester = LoadTester::new(config).await?;

    // Start monitoring task
    let metrics = tester.metrics.clone();
    let monitor_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));

        loop {
            interval.tick().await;

            let m = metrics.lock().await;
            info!("Progress: {} operations, {:.1}% success rate, {:.2} ops/sec",
                  m.total_operations,
                  m.successful_operations as f64 / m.total_operations as f64 * 100.0,
                  m.throughput_ops_per_sec);
        }
    });

    let final_metrics = tester.run().await?;
    monitor_handle.abort();

    final_metrics.print_summary();

    // Check for memory leaks
    if final_metrics.memory_usage_mb > 1000.0 {
        warn!("High memory usage detected: {:.1}MB", final_metrics.memory_usage_mb);
    }

    Ok(())
}

/// Spike test to simulate sudden load increases
pub async fn spike_test() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting spike test...");

    let scenarios = vec![
        ("baseline", 10, 30),
        ("spike", 500, 10),
        ("recovery", 10, 30),
        ("mega_spike", 1000, 5),
        ("cool_down", 5, 60),
    ];

    let mut all_metrics = vec![];

    for (name, ops_per_sec, duration_secs) in scenarios {
        info!("Running scenario: {} ({} ops/sec for {} seconds)", name, ops_per_sec, duration_secs);

        let config = LoadTestConfig {
            total_operations: ops_per_sec * duration_secs,
            concurrent_workers: std::cmp::min(ops_per_sec / 10, 50),
            operations_per_second: Some(ops_per_sec),
            test_duration_secs: Some(duration_secs),
            warmup_operations: 0,
            operation_timeout_ms: 5000,
            measure_resources: true,
        };

        let tester = LoadTester::new(config).await?;
        let metrics = tester.run().await?;

        info!("Scenario '{}' - Success rate: {:.1}%, P99: {:.2}ms",
              name,
              metrics.successful_operations as f64 / metrics.total_operations as f64 * 100.0,
              metrics.p99_latency.as_secs_f64() * 1000.0);

        all_metrics.push((name, metrics));
    }

    // Print summary
    println!("\n=== Spike Test Summary ===");
    for (name, metrics) in all_metrics {
        println!("{}: {:.1}% success, P99: {:.2}ms, CPU: {:.1}%",
                 name,
                 metrics.successful_operations as f64 / metrics.total_operations as f64 * 100.0,
                 metrics.p99_latency.as_secs_f64() * 1000.0,
                 metrics.cpu_usage_percent);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_load() {
        init_test_logging();

        let config = LoadTestConfig {
            total_operations: 100,
            concurrent_workers: 5,
            operations_per_second: None,
            test_duration_secs: None,
            warmup_operations: 10,
            operation_timeout_ms: 5000,
            measure_resources: true,
        };

        let tester = LoadTester::new(config).await.unwrap();
        let metrics = tester.run().await.unwrap();

        assert!(metrics.successful_operations >= 90); // 90% success rate
        assert!(metrics.p99_latency < Duration::from_secs(1));

        metrics.print_summary();
    }

    #[tokio::test]
    async fn test_rate_limited_load() {
        init_test_logging();

        let config = LoadTestConfig {
            total_operations: 50,
            concurrent_workers: 10,
            operations_per_second: Some(10),
            test_duration_secs: None,
            warmup_operations: 0,
            operation_timeout_ms: 5000,
            measure_resources: false,
        };

        let tester = LoadTester::new(config).await.unwrap();
        let start = Instant::now();
        let metrics = tester.run().await.unwrap();
        let duration = start.elapsed();

        // Should take approximately 5 seconds (50 ops at 10 ops/sec)
        assert!(duration >= Duration::from_secs(4));
        assert!(duration <= Duration::from_secs(6));

        assert!(metrics.throughput_ops_per_sec <= 12.0); // Allow some variance
    }

    #[tokio::test]
    #[ignore] // Run with --ignored for full stress test
    async fn test_stress() {
        init_test_logging();
        stress_test().await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Run with --ignored for endurance test
    async fn test_endurance() {
        init_test_logging();
        endurance_test(5).await.unwrap(); // 5 minute test
    }

    #[tokio::test]
    #[ignore] // Run with --ignored for spike test
    async fn test_spikes() {
        init_test_logging();
        spike_test().await.unwrap();
    }

    fn init_test_logging() {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_env_filter("info")
            .try_init();
    }
}

// Make functions available for binary usage
pub use serde_json::json;
pub use sysinfo;