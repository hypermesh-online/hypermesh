//! Performance testing suite for Nexus components
//!
//! Benchmarks and load testing for critical performance paths.

pub mod consensus_benchmarks;
pub mod transport_benchmarks;
pub mod state_benchmarks;
pub mod ebpf_benchmarks;

use crate::{TestResult, init_test_logging};
use tracing::{info, warn};
use std::time::{Duration, Instant};

/// Performance test configuration
#[derive(Debug, Clone)]
pub struct PerfTestConfig {
    pub duration_seconds: u64,
    pub concurrent_operations: usize,
    pub warmup_seconds: u64,
    pub target_throughput: Option<u64>, // operations per second
    pub max_latency_ms: Option<u64>,
}

impl Default for PerfTestConfig {
    fn default() -> Self {
        Self {
            duration_seconds: 30,
            concurrent_operations: 100,
            warmup_seconds: 5,
            target_throughput: None,
            max_latency_ms: Some(1000),
        }
    }
}

/// Performance test results
#[derive(Debug, Clone)]
pub struct PerfTestResults {
    pub total_operations: u64,
    pub duration: Duration,
    pub throughput_ops_sec: f64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub error_rate: f64,
}

impl PerfTestResults {
    pub fn meets_targets(&self, config: &PerfTestConfig) -> bool {
        let mut meets_targets = true;
        
        if let Some(target_throughput) = config.target_throughput {
            if self.throughput_ops_sec < target_throughput as f64 {
                warn!("Throughput {} ops/sec below target {}", 
                      self.throughput_ops_sec, target_throughput);
                meets_targets = false;
            }
        }
        
        if let Some(max_latency) = config.max_latency_ms {
            if self.p95_latency_ms > max_latency as f64 {
                warn!("P95 latency {} ms exceeds target {} ms", 
                      self.p95_latency_ms, max_latency);
                meets_targets = false;
            }
        }
        
        meets_targets
    }
    
    pub fn print_summary(&self) {
        println!("\n📊 Performance Test Results:");
        println!("============================");
        println!("Total Operations: {}", self.total_operations);
        println!("Duration: {:.2}s", self.duration.as_secs_f64());
        println!("Throughput: {:.2} ops/sec", self.throughput_ops_sec);
        println!("Avg Latency: {:.2} ms", self.avg_latency_ms);
        println!("P95 Latency: {:.2} ms", self.p95_latency_ms);
        println!("P99 Latency: {:.2} ms", self.p99_latency_ms);
        println!("Error Rate: {:.2}%", self.error_rate * 100.0);
    }
}

/// Run all performance tests
pub async fn run_all_performance_tests() -> TestResult {
    init_test_logging();
    info!("Starting performance test suite");

    // Skip if not enabled
    if std::env::var("NEXUS_PERF_TESTS").is_err() {
        info!("Skipping performance tests (set NEXUS_PERF_TESTS=1 to enable)");
        return Ok(());
    }

    let mut failed_tests = Vec::new();

    let test_suites = vec![
        ("consensus", consensus_benchmarks::run_consensus_benchmarks),
        ("transport", transport_benchmarks::run_transport_benchmarks),
        ("state", state_benchmarks::run_state_benchmarks),
        ("ebpf", ebpf_benchmarks::run_ebpf_benchmarks),
    ];

    for (test_name, test_fn) in test_suites {
        info!("Running {} performance tests", test_name);
        
        match test_fn().await {
            Ok(()) => {
                info!("✅ {} performance tests passed", test_name);
            }
            Err(e) => {
                warn!("⚠️ {} performance tests failed: {}", test_name, e);
                failed_tests.push(test_name);
            }
        }
    }

    if failed_tests.is_empty() {
        info!("🎉 All performance tests completed!");
        Ok(())
    } else {
        warn!("Some performance tests failed: {}", failed_tests.join(", "));
        Ok(()) // Performance test failures are warnings, not errors
    }
}

/// Performance test runner utility
pub struct PerfTestRunner {
    config: PerfTestConfig,
    latency_samples: Vec<Duration>,
    error_count: u64,
    start_time: Option<Instant>,
}

impl PerfTestRunner {
    pub fn new(config: PerfTestConfig) -> Self {
        Self {
            config,
            latency_samples: Vec::new(),
            error_count: 0,
            start_time: None,
        }
    }
    
    pub async fn run_test<F, Fut>(&mut self, test_fn: F) -> PerfTestResults
    where
        F: Fn() -> Fut + Clone + Send + 'static,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send,
    {
        info!("Starting performance test with {} concurrent operations for {}s", 
              self.config.concurrent_operations, self.config.duration_seconds);
        
        // Warmup
        if self.config.warmup_seconds > 0 {
            info!("Warming up for {}s", self.config.warmup_seconds);
            self.warmup(test_fn.clone()).await;
        }
        
        // Reset counters after warmup
        self.latency_samples.clear();
        self.error_count = 0;
        
        // Main test
        let start = Instant::now();
        self.start_time = Some(start);
        
        let mut handles = Vec::new();
        
        for _ in 0..self.config.concurrent_operations {
            let test_fn = test_fn.clone();
            let handle = tokio::spawn(async move {
                let mut local_latencies = Vec::new();
                let mut local_errors = 0u64;
                let test_end = Instant::now() + Duration::from_secs(self.config.duration_seconds);
                
                while Instant::now() < test_end {
                    let op_start = Instant::now();
                    match test_fn().await {
                        Ok(()) => {
                            local_latencies.push(op_start.elapsed());
                        }
                        Err(_) => {
                            local_errors += 1;
                        }
                    }
                }
                
                (local_latencies, local_errors)
            });
            
            handles.push(handle);
        }
        
        // Collect results
        let mut total_operations = 0;
        for handle in handles {
            let (latencies, errors) = handle.await.unwrap();
            total_operations += latencies.len() as u64 + errors;
            self.latency_samples.extend(latencies);
            self.error_count += errors;
        }
        
        let duration = start.elapsed();
        self.calculate_results(total_operations, duration)
    }
    
    async fn warmup<F, Fut>(&self, test_fn: F)
    where
        F: Fn() -> Fut + Clone + Send + 'static,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send,
    {
        let warmup_end = Instant::now() + Duration::from_secs(self.config.warmup_seconds);
        let mut handles = Vec::new();
        
        for _ in 0..std::cmp::min(self.config.concurrent_operations, 10) {
            let test_fn = test_fn.clone();
            let handle = tokio::spawn(async move {
                while Instant::now() < warmup_end {
                    let _ = test_fn().await;
                    tokio::task::yield_now().await;
                }
            });
            handles.push(handle);
        }
        
        futures::future::join_all(handles).await;
    }
    
    fn calculate_results(&mut self, total_operations: u64, duration: Duration) -> PerfTestResults {
        let throughput = total_operations as f64 / duration.as_secs_f64();
        
        // Sort latencies for percentile calculations
        self.latency_samples.sort_unstable();
        
        let avg_latency = if self.latency_samples.is_empty() {
            0.0
        } else {
            let sum: Duration = self.latency_samples.iter().sum();
            sum.as_millis() as f64 / self.latency_samples.len() as f64
        };
        
        let p95_latency = if self.latency_samples.is_empty() {
            0.0
        } else {
            let index = (self.latency_samples.len() as f64 * 0.95) as usize;
            self.latency_samples[index.min(self.latency_samples.len() - 1)].as_millis() as f64
        };
        
        let p99_latency = if self.latency_samples.is_empty() {
            0.0
        } else {
            let index = (self.latency_samples.len() as f64 * 0.99) as usize;
            self.latency_samples[index.min(self.latency_samples.len() - 1)].as_millis() as f64
        };
        
        let error_rate = if total_operations > 0 {
            self.error_count as f64 / total_operations as f64
        } else {
            0.0
        };
        
        PerfTestResults {
            total_operations,
            duration,
            throughput_ops_sec: throughput,
            avg_latency_ms: avg_latency,
            p95_latency_ms: p95_latency,
            p99_latency_ms: p99_latency,
            error_rate,
        }
    }
}