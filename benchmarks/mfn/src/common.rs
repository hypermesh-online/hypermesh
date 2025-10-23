/*!
# Common benchmarking infrastructure and utilities

Shared types, configuration, and utilities used across all MFN benchmarks.
*/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

/// Global benchmark configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    pub warmup_iterations: usize,
    pub measurement_iterations: usize,
    pub statistical_confidence: f64,
    pub regression_threshold: f64,
    pub memory_limit_mb: usize,
    pub timeout_seconds: u64,
    pub parallel_workers: usize,
    pub output_format: OutputFormat,
    pub enable_flamegraph: bool,
    pub enable_perf_counters: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    Json,
    Yaml,
    Csv,
    Html,
}

/// Performance metrics for a single benchmark run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub benchmark_id: String,
    pub layer: MfnLayer,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub duration: Duration,
    pub throughput_ops_per_sec: f64,
    pub latency_percentiles: LatencyPercentiles,
    pub memory_usage_mb: f64,
    pub cpu_utilization: f64,
    pub error_rate: f64,
    pub custom_metrics: HashMap<String, f64>,
}

/// MFN layer identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MfnLayer {
    Layer1Ifr,  // Immediate Flow Registry
    Layer2Dsr,  // Dynamic Similarity Resolution
    Layer3Alm,  // Adaptive Link Management
    Layer4Cpe,  // Context Prediction Engine
    Integration, // End-to-end integration
}

impl std::fmt::Display for MfnLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MfnLayer::Layer1Ifr => write!(f, "Layer1-IFR"),
            MfnLayer::Layer2Dsr => write!(f, "Layer2-DSR"),
            MfnLayer::Layer3Alm => write!(f, "Layer3-ALM"),
            MfnLayer::Layer4Cpe => write!(f, "Layer4-CPE"),
            MfnLayer::Integration => write!(f, "Integration"),
        }
    }
}

/// Latency percentile measurements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyPercentiles {
    pub p50: Duration,
    pub p75: Duration,
    pub p90: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub p999: Duration,
    pub max: Duration,
    pub min: Duration,
    pub mean: Duration,
    pub stddev: Duration,
}

/// Benchmark execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub id: String,
    pub name: String,
    pub layer: MfnLayer,
    pub config: BenchmarkConfig,
    pub metrics: PerformanceMetrics,
    pub target_validation: TargetValidation,
    pub baseline_comparison: Option<BaselineComparison>,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Performance target validation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetValidation {
    pub latency_target_met: bool,
    pub throughput_target_met: bool,
    pub memory_target_met: bool,
    pub improvement_target_met: bool,
    pub overall_success: bool,
    pub target_details: HashMap<String, TargetResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetResult {
    pub target_value: f64,
    pub actual_value: f64,
    pub unit: String,
    pub met: bool,
    pub improvement_percent: Option<f64>,
}

/// Comparison with baseline measurements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineComparison {
    pub baseline_metrics: PerformanceMetrics,
    pub improvement_percent: f64,
    pub statistical_significance: f64,
    pub confidence_interval: (f64, f64),
    pub significant_improvement: bool,
}

/// Performance test harness
pub struct BenchmarkHarness {
    pub config: BenchmarkConfig,
    pub results: Vec<BenchmarkResult>,
    start_time: Instant,
}

impl BenchmarkHarness {
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            config,
            results: Vec::new(),
            start_time: Instant::now(),
        }
    }

    /// Execute a single benchmark with warmup and measurement phases
    pub async fn run_benchmark<F, Fut>(
        &mut self,
        name: &str,
        layer: MfnLayer,
        benchmark_fn: F,
    ) -> anyhow::Result<BenchmarkResult>
    where
        F: Fn() -> Fut + Clone + Send + 'static,
        Fut: std::future::Future<Output = anyhow::Result<Duration>> + Send,
    {
        let benchmark_id = format!("{}_{}", layer, Uuid::new_v4().simple());
        
        println!("🚀 Starting benchmark: {} ({})", name, benchmark_id);
        
        // Warmup phase
        println!("  ⚡ Warming up ({} iterations)...", self.config.warmup_iterations);
        for _ in 0..self.config.warmup_iterations {
            let _ = benchmark_fn().await?;
        }
        
        // Measurement phase
        println!("  📊 Measuring ({} iterations)...", self.config.measurement_iterations);
        let mut measurements = Vec::with_capacity(self.config.measurement_iterations);
        let measurement_start = Instant::now();
        
        for i in 0..self.config.measurement_iterations {
            if i % 1000 == 0 && i > 0 {
                println!("    Progress: {}/{}", i, self.config.measurement_iterations);
            }
            let duration = benchmark_fn().await?;
            measurements.push(duration);
        }
        
        let total_duration = measurement_start.elapsed();
        
        // Calculate metrics
        let metrics = self.calculate_metrics(&benchmark_id, layer, &measurements, total_duration)?;
        let target_validation = self.validate_targets(layer, &metrics);
        
        let result = BenchmarkResult {
            id: benchmark_id.clone(),
            name: name.to_string(),
            layer,
            config: self.config.clone(),
            metrics,
            target_validation,
            baseline_comparison: None, // Will be filled by comparison logic
            success: true,
            error_message: None,
        };
        
        println!("  ✅ Benchmark complete: {:.2} ops/sec, {:.3}ms P95", 
                 result.metrics.throughput_ops_per_sec,
                 result.metrics.latency_percentiles.p95.as_secs_f64() * 1000.0);
        
        self.results.push(result.clone());
        Ok(result)
    }

    /// Calculate comprehensive performance metrics from raw measurements
    fn calculate_metrics(
        &self,
        benchmark_id: &str,
        layer: MfnLayer,
        measurements: &[Duration],
        total_duration: Duration,
    ) -> anyhow::Result<PerformanceMetrics> {
        let mut sorted_measurements = measurements.to_vec();
        sorted_measurements.sort();
        
        let count = sorted_measurements.len() as f64;
        let throughput = count / total_duration.as_secs_f64();
        
        // Calculate percentiles
        let percentiles = LatencyPercentiles {
            min: sorted_measurements[0],
            p50: sorted_measurements[(count * 0.5) as usize],
            p75: sorted_measurements[(count * 0.75) as usize],
            p90: sorted_measurements[(count * 0.9) as usize],
            p95: sorted_measurements[(count * 0.95) as usize],
            p99: sorted_measurements[(count * 0.99) as usize],
            p999: sorted_measurements[(count * 0.999) as usize],
            max: sorted_measurements[sorted_measurements.len() - 1],
            mean: Duration::from_nanos(
                (measurements.iter().map(|d| d.as_nanos()).sum::<u128>() / measurements.len() as u128) as u64
            ),
            stddev: self.calculate_stddev(measurements)?,
        };
        
        // Get system metrics
        let (memory_mb, cpu_percent) = self.get_system_metrics()?;
        
        Ok(PerformanceMetrics {
            benchmark_id: benchmark_id.to_string(),
            layer,
            timestamp: chrono::Utc::now(),
            duration: total_duration,
            throughput_ops_per_sec: throughput,
            latency_percentiles: percentiles,
            memory_usage_mb: memory_mb,
            cpu_utilization: cpu_percent,
            error_rate: 0.0, // TODO: Track errors
            custom_metrics: HashMap::new(),
        })
    }

    /// Calculate standard deviation of measurements
    fn calculate_stddev(&self, measurements: &[Duration]) -> anyhow::Result<Duration> {
        let mean = measurements.iter().map(|d| d.as_nanos()).sum::<u128>() as f64 / measurements.len() as f64;
        let variance = measurements.iter()
            .map(|d| {
                let diff = d.as_nanos() as f64 - mean;
                diff * diff
            })
            .sum::<f64>() / measurements.len() as f64;
        
        Ok(Duration::from_nanos(variance.sqrt() as u64))
    }

    /// Get current system resource usage
    fn get_system_metrics(&self) -> anyhow::Result<(f64, f64)> {
        use sysinfo::{System, SystemExt, ProcessExt, PidExt};
        
        let mut system = System::new_all();
        system.refresh_all();
        
        let pid = sysinfo::get_current_pid().map_err(|e| anyhow::anyhow!("Failed to get PID: {}", e))?;
        
        if let Some(process) = system.process(pid) {
            let memory_mb = process.memory() as f64 / 1024.0 / 1024.0;
            let cpu_percent = process.cpu_usage() as f64;
            Ok((memory_mb, cpu_percent))
        } else {
            Ok((0.0, 0.0))
        }
    }

    /// Validate performance against layer-specific targets
    fn validate_targets(&self, layer: MfnLayer, metrics: &PerformanceMetrics) -> TargetValidation {
        let mut target_details = HashMap::new();
        
        match layer {
            MfnLayer::Layer1Ifr => {
                // IFR targets: <0.1ms latency, >10M ops/sec, 88.6% improvement
                let latency_target = Duration::from_micros(100); // 0.1ms
                let throughput_target = 10_000_000.0; // 10M ops/sec
                
                let latency_met = metrics.latency_percentiles.p95 <= latency_target;
                let throughput_met = metrics.throughput_ops_per_sec >= throughput_target;
                
                target_details.insert("latency_p95".to_string(), TargetResult {
                    target_value: latency_target.as_secs_f64() * 1000.0,
                    actual_value: metrics.latency_percentiles.p95.as_secs_f64() * 1000.0,
                    unit: "ms".to_string(),
                    met: latency_met,
                    improvement_percent: None,
                });
                
                target_details.insert("throughput".to_string(), TargetResult {
                    target_value: throughput_target,
                    actual_value: metrics.throughput_ops_per_sec,
                    unit: "ops/sec".to_string(),
                    met: throughput_met,
                    improvement_percent: None,
                });
                
                TargetValidation {
                    latency_target_met: latency_met,
                    throughput_target_met: throughput_met,
                    memory_target_met: metrics.memory_usage_mb <= 10.0,
                    improvement_target_met: true, // Will be set by baseline comparison
                    overall_success: latency_met && throughput_met,
                    target_details,
                }
            }
            MfnLayer::Layer2Dsr => {
                // DSR targets: <1ms neural inference
                let latency_target = Duration::from_millis(1);
                let latency_met = metrics.latency_percentiles.p95 <= latency_target;
                
                target_details.insert("neural_inference_latency".to_string(), TargetResult {
                    target_value: 1.0,
                    actual_value: metrics.latency_percentiles.p95.as_secs_f64() * 1000.0,
                    unit: "ms".to_string(),
                    met: latency_met,
                    improvement_percent: None,
                });
                
                TargetValidation {
                    latency_target_met: latency_met,
                    throughput_target_met: true,
                    memory_target_met: metrics.memory_usage_mb <= 100.0,
                    improvement_target_met: true,
                    overall_success: latency_met,
                    target_details,
                }
            }
            MfnLayer::Layer3Alm => {
                // ALM targets: 777% routing improvement
                target_details.insert("routing_improvement".to_string(), TargetResult {
                    target_value: 777.0,
                    actual_value: 0.0, // Will be set by baseline comparison
                    unit: "percent".to_string(),
                    met: false, // Will be determined by baseline comparison
                    improvement_percent: None,
                });
                
                TargetValidation {
                    latency_target_met: true,
                    throughput_target_met: true,
                    memory_target_met: metrics.memory_usage_mb <= 50.0,
                    improvement_target_met: false, // Will be set by baseline comparison
                    overall_success: false, // Will be determined by baseline comparison
                    target_details,
                }
            }
            MfnLayer::Layer4Cpe => {
                // CPE targets: <2ms context prediction
                let latency_target = Duration::from_millis(2);
                let latency_met = metrics.latency_percentiles.p95 <= latency_target;
                
                target_details.insert("context_prediction_latency".to_string(), TargetResult {
                    target_value: 2.0,
                    actual_value: metrics.latency_percentiles.p95.as_secs_f64() * 1000.0,
                    unit: "ms".to_string(),
                    met: latency_met,
                    improvement_percent: None,
                });
                
                TargetValidation {
                    latency_target_met: latency_met,
                    throughput_target_met: true,
                    memory_target_met: metrics.memory_usage_mb <= 200.0,
                    improvement_target_met: true,
                    overall_success: latency_met,
                    target_details,
                }
            }
            MfnLayer::Integration => {
                // Integration targets: adaptive network tiers throughput, <5% overhead
                let throughput_target = adaptive network tiers in bits/sec
                let throughput_met = metrics.throughput_ops_per_sec * 8.0 >= throughput_target; // Assuming 8 bits per op
                
                target_details.insert("network_throughput".to_string(), TargetResult {
                    target_value: throughput_target / 1_000_000_000.0, // Convert to Gbps
                    actual_value: (metrics.throughput_ops_per_sec * 8.0) / 1_000_000_000.0,
                    unit: "Gbps".to_string(),
                    met: throughput_met,
                    improvement_percent: None,
                });
                
                TargetValidation {
                    latency_target_met: true,
                    throughput_target_met: throughput_met,
                    memory_target_met: metrics.memory_usage_mb <= 512.0,
                    improvement_target_met: true,
                    overall_success: throughput_met,
                    target_details,
                }
            }
        }
    }

    /// Generate summary report for all completed benchmarks
    pub fn generate_summary(&self) -> BenchmarkSummary {
        let successful_count = self.results.iter().filter(|r| r.success).count();
        let total_count = self.results.len();
        
        let avg_throughput = if total_count > 0 {
            self.results.iter()
                .map(|r| r.metrics.throughput_ops_per_sec)
                .sum::<f64>() / total_count as f64
        } else {
            0.0
        };
        
        BenchmarkSummary {
            total_benchmarks: total_count,
            successful_benchmarks: successful_count,
            failed_benchmarks: total_count - successful_count,
            total_duration: self.start_time.elapsed(),
            average_throughput: avg_throughput,
            target_success_rate: if total_count > 0 {
                self.results.iter()
                    .filter(|r| r.target_validation.overall_success)
                    .count() as f64 / total_count as f64
            } else {
                0.0
            },
            results: self.results.clone(),
        }
    }
}

/// Summary of all benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    pub total_benchmarks: usize,
    pub successful_benchmarks: usize,
    pub failed_benchmarks: usize,
    pub total_duration: Duration,
    pub average_throughput: f64,
    pub target_success_rate: f64,
    pub results: Vec<BenchmarkResult>,
}

/// Utility functions for test data generation
pub mod test_utils {
    use super::*;
    use rand::Rng;

    pub fn generate_flow_keys(count: usize) -> Vec<[u8; 32]> {
        let mut rng = rand::thread_rng();
        (0..count)
            .map(|_| {
                let mut key = [0u8; 32];
                rng.fill(&mut key);
                key
            })
            .collect()
    }

    pub fn generate_network_packets(count: usize, size: usize) -> Vec<Vec<u8>> {
        let mut rng = rand::thread_rng();
        (0..count)
            .map(|_| {
                let mut packet = vec![0u8; size];
                rng.fill(&mut packet[..]);
                packet
            })
            .collect()
    }

    pub fn generate_neural_vectors(count: usize, dimensions: usize) -> Vec<Vec<f32>> {
        let mut rng = rand::thread_rng();
        (0..count)
            .map(|_| {
                (0..dimensions)
                    .map(|_| rng.gen::<f32>())
                    .collect()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_benchmark_harness() {
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            statistical_confidence: 0.95,
            regression_threshold: 0.05,
            memory_limit_mb: 128,
            timeout_seconds: 60,
            parallel_workers: 1,
            output_format: OutputFormat::Json,
            enable_flamegraph: false,
            enable_perf_counters: false,
        };

        let mut harness = BenchmarkHarness::new(config);
        
        // Simple test benchmark
        let result = harness.run_benchmark(
            "test_benchmark",
            MfnLayer::Layer1Ifr,
            || async {
                // Simulate work
                tokio::time::sleep(Duration::from_micros(10)).await;
                Ok(Duration::from_micros(10))
            }
        ).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
        assert_eq!(result.layer, MfnLayer::Layer1Ifr);
    }

    #[test]
    fn test_target_validation() {
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            statistical_confidence: 0.95,
            regression_threshold: 0.05,
            memory_limit_mb: 128,
            timeout_seconds: 60,
            parallel_workers: 1,
            output_format: OutputFormat::Json,
            enable_flamegraph: false,
            enable_perf_counters: false,
        };

        let harness = BenchmarkHarness::new(config);
        
        let metrics = PerformanceMetrics {
            benchmark_id: "test".to_string(),
            layer: MfnLayer::Layer1Ifr,
            timestamp: chrono::Utc::now(),
            duration: Duration::from_secs(1),
            throughput_ops_per_sec: 15_000_000.0, // Exceeds 10M target
            latency_percentiles: LatencyPercentiles {
                p95: Duration::from_micros(50), // Beats 100µs target
                p50: Duration::from_micros(30),
                p75: Duration::from_micros(40),
                p90: Duration::from_micros(45),
                p99: Duration::from_micros(70),
                p999: Duration::from_micros(90),
                max: Duration::from_micros(100),
                min: Duration::from_micros(20),
                mean: Duration::from_micros(35),
                stddev: Duration::from_micros(5),
            },
            memory_usage_mb: 8.0, // Under 10MB target
            cpu_utilization: 25.0,
            error_rate: 0.0,
            custom_metrics: HashMap::new(),
        };

        let validation = harness.validate_targets(MfnLayer::Layer1Ifr, &metrics);
        
        assert!(validation.latency_target_met);
        assert!(validation.throughput_target_met);
        assert!(validation.memory_target_met);
        assert!(validation.overall_success);
    }
}