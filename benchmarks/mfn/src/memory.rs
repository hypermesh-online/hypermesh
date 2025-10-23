/*!
# Memory Profiling and Analysis

Memory usage profiling and analysis for MFN components:
- Heap allocation tracking
- Memory leak detection
- Peak memory usage monitoring
- Memory efficiency analysis
- Garbage collection impact (where applicable)
*/

use crate::common::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
use procfs::{process::Process, ProcResult};

/// Memory profiling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub enable_heap_profiling: bool,
    pub enable_leak_detection: bool,
    pub sampling_interval_ms: u64,
    pub max_samples: usize,
    pub memory_limit_mb: f64,
    pub leak_threshold_mb: f64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enable_heap_profiling: true,
            enable_leak_detection: true,
            sampling_interval_ms: 100, // Sample every 100ms
            max_samples: 1000,
            memory_limit_mb: 1024.0, // 1GB limit
            leak_threshold_mb: 10.0,  // 10MB growth without release
        }
    }
}

/// Memory profiler for tracking memory usage during benchmarks
pub struct MemoryProfiler {
    config: MemoryConfig,
    samples: Vec<MemorySample>,
    start_time: Instant,
    baseline_memory: Option<MemorySnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySample {
    pub timestamp: Duration, // Time since profiling started
    pub memory_snapshot: MemorySnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub rss_mb: f64,        // Resident Set Size
    pub vms_mb: f64,        // Virtual Memory Size
    pub heap_mb: f64,       // Heap usage (approximation)
    pub stack_mb: f64,      // Stack usage
    pub shared_mb: f64,     // Shared memory
    pub peak_rss_mb: f64,   // Peak RSS since start
}

impl MemoryProfiler {
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            config,
            samples: Vec::new(),
            start_time: Instant::now(),
            baseline_memory: None,
        }
    }

    /// Start memory profiling
    pub fn start_profiling(&mut self) -> anyhow::Result<()> {
        self.baseline_memory = Some(self.take_memory_snapshot()?);
        self.start_time = Instant::now();
        self.samples.clear();
        
        println!("🔍 Memory profiling started - baseline: {:.2} MB RSS", 
                 self.baseline_memory.as_ref().unwrap().rss_mb);
        
        Ok(())
    }

    /// Take a memory sample
    pub fn sample_memory(&mut self) -> anyhow::Result<()> {
        let snapshot = self.take_memory_snapshot()?;
        let elapsed = self.start_time.elapsed();
        
        self.samples.push(MemorySample {
            timestamp: elapsed,
            memory_snapshot: snapshot,
        });

        // Limit sample size to prevent unbounded growth
        if self.samples.len() > self.config.max_samples {
            self.samples.drain(0..100); // Remove oldest 100 samples
        }

        Ok(())
    }

    /// Stop profiling and generate report
    pub fn stop_profiling(&mut self) -> MemoryProfileReport {
        let final_snapshot = self.take_memory_snapshot().ok();
        let total_duration = self.start_time.elapsed();

        let analysis = self.analyze_memory_usage();
        let leaks = if self.config.enable_leak_detection {
            self.detect_memory_leaks()
        } else {
            Vec::new()
        };

        println!("📊 Memory profiling completed - final: {:.2} MB RSS", 
                 final_snapshot.as_ref().map(|s| s.rss_mb).unwrap_or(0.0));

        MemoryProfileReport {
            total_duration,
            samples_collected: self.samples.len(),
            baseline_memory: self.baseline_memory.clone(),
            final_memory: final_snapshot,
            memory_analysis: analysis,
            detected_leaks: leaks,
            config: self.config.clone(),
        }
    }

    #[cfg(target_os = "linux")]
    fn take_memory_snapshot(&self) -> anyhow::Result<MemorySnapshot> {
        let process = Process::myself()?;
        let stat = process.stat()?;
        let statm = process.statm()?;
        
        // Convert from pages to MB (assuming 4KB pages)
        let page_size_mb = 4.0 / 1024.0;
        
        Ok(MemorySnapshot {
            rss_mb: statm.resident as f64 * page_size_mb,
            vms_mb: statm.size as f64 * page_size_mb,
            heap_mb: (statm.data + statm.stack) as f64 * page_size_mb,
            stack_mb: statm.stack as f64 * page_size_mb,
            shared_mb: statm.shared as f64 * page_size_mb,
            peak_rss_mb: stat.rss_max as f64 * page_size_mb,
        })
    }

    #[cfg(not(target_os = "linux"))]
    fn take_memory_snapshot(&self) -> anyhow::Result<MemorySnapshot> {
        // Fallback implementation for non-Linux systems
        use sysinfo::{System, SystemExt, ProcessExt, PidExt};
        
        let mut system = System::new_all();
        system.refresh_all();
        
        let pid = sysinfo::get_current_pid()
            .map_err(|e| anyhow::anyhow!("Failed to get PID: {}", e))?;
        
        if let Some(process) = system.process(pid) {
            let memory_kb = process.memory();
            let virtual_memory_kb = process.virtual_memory();
            
            Ok(MemorySnapshot {
                rss_mb: memory_kb as f64 / 1024.0,
                vms_mb: virtual_memory_kb as f64 / 1024.0,
                heap_mb: memory_kb as f64 / 1024.0 * 0.8, // Estimate
                stack_mb: memory_kb as f64 / 1024.0 * 0.1, // Estimate
                shared_mb: 0.0, // Not available
                peak_rss_mb: memory_kb as f64 / 1024.0, // Approximation
            })
        } else {
            Err(anyhow::anyhow!("Process not found"))
        }
    }

    fn analyze_memory_usage(&self) -> MemoryAnalysis {
        if self.samples.is_empty() {
            return MemoryAnalysis::default();
        }

        let rss_values: Vec<f64> = self.samples.iter().map(|s| s.memory_snapshot.rss_mb).collect();
        let vms_values: Vec<f64> = self.samples.iter().map(|s| s.memory_snapshot.vms_mb).collect();
        
        let peak_rss = rss_values.iter().fold(0.0f64, |a, &b| a.max(b));
        let min_rss = rss_values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let avg_rss = rss_values.iter().sum::<f64>() / rss_values.len() as f64;
        
        let peak_vms = vms_values.iter().fold(0.0f64, |a, &b| a.max(b));
        let avg_vms = vms_values.iter().sum::<f64>() / vms_values.len() as f64;

        // Calculate memory growth rate
        let growth_rate = if self.samples.len() > 1 {
            let first_rss = self.samples[0].memory_snapshot.rss_mb;
            let last_rss = self.samples[self.samples.len() - 1].memory_snapshot.rss_mb;
            let duration_seconds = self.samples[self.samples.len() - 1].timestamp.as_secs_f64();
            
            if duration_seconds > 0.0 {
                (last_rss - first_rss) / duration_seconds // MB/second
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Memory efficiency metrics
        let baseline_rss = self.baseline_memory.as_ref().map(|b| b.rss_mb).unwrap_or(0.0);
        let memory_overhead = avg_rss - baseline_rss;
        let efficiency_score = if avg_rss > 0.0 {
            (1.0 - (memory_overhead / avg_rss)).max(0.0) * 100.0
        } else {
            100.0
        };

        MemoryAnalysis {
            peak_rss_mb: peak_rss,
            min_rss_mb: min_rss,
            avg_rss_mb: avg_rss,
            peak_vms_mb: peak_vms,
            avg_vms_mb: avg_vms,
            memory_growth_rate_mb_per_sec: growth_rate,
            memory_overhead_mb: memory_overhead,
            efficiency_score_percent: efficiency_score,
            exceeded_limit: peak_rss > self.config.memory_limit_mb,
            sample_count: self.samples.len(),
        }
    }

    fn detect_memory_leaks(&self) -> Vec<MemoryLeak> {
        let mut leaks = Vec::new();

        if self.samples.len() < 10 {
            return leaks; // Not enough data
        }

        // Look for sustained memory growth without corresponding decreases
        let window_size = 10;
        for i in window_size..self.samples.len() {
            let current_window = &self.samples[i - window_size..i];
            let current_avg = current_window.iter()
                .map(|s| s.memory_snapshot.rss_mb)
                .sum::<f64>() / window_size as f64;

            let previous_window = &self.samples[i - window_size * 2..i - window_size];
            let previous_avg = previous_window.iter()
                .map(|s| s.memory_snapshot.rss_mb)
                .sum::<f64>() / window_size as f64;

            let growth = current_avg - previous_avg;
            
            if growth > self.config.leak_threshold_mb {
                // Check if this growth is sustained (no significant decrease)
                let next_samples = if i + window_size < self.samples.len() {
                    &self.samples[i..i + window_size]
                } else {
                    &self.samples[i..]
                };

                let next_avg = next_samples.iter()
                    .map(|s| s.memory_snapshot.rss_mb)
                    .sum::<f64>() / next_samples.len() as f64;

                // If memory doesn't decrease significantly, it's likely a leak
                if next_avg >= current_avg - 1.0 { // Allow 1MB decrease as normal
                    leaks.push(MemoryLeak {
                        detected_at: current_window[window_size - 1].timestamp,
                        growth_mb: growth,
                        leak_rate_mb_per_sec: growth / (window_size as f64 * self.config.sampling_interval_ms as f64 / 1000.0),
                        confidence: self.calculate_leak_confidence(growth, &self.samples[i - window_size * 2..i]),
                        description: format!(
                            "Sustained memory growth of {:.2} MB detected over {} samples",
                            growth, window_size
                        ),
                    });
                }
            }
        }

        leaks
    }

    fn calculate_leak_confidence(&self, growth: f64, samples: &[MemorySample]) -> f64 {
        // Simple confidence calculation based on:
        // 1. Growth magnitude
        // 2. Consistency of growth
        // 3. Sample size

        let rss_values: Vec<f64> = samples.iter().map(|s| s.memory_snapshot.rss_mb).collect();
        
        // Calculate variance to measure consistency
        let mean = rss_values.iter().sum::<f64>() / rss_values.len() as f64;
        let variance = rss_values.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / rss_values.len() as f64;
        let coefficient_of_variation = if mean > 0.0 { variance.sqrt() / mean } else { 1.0 };

        // Lower CV = more consistent = higher confidence
        let consistency_score = (1.0 - coefficient_of_variation.min(1.0)) * 100.0;
        
        // Growth magnitude score
        let magnitude_score = (growth / self.config.leak_threshold_mb).min(1.0) * 100.0;
        
        // Sample size score
        let sample_score = (samples.len() as f64 / 20.0).min(1.0) * 100.0;
        
        // Weighted average
        (consistency_score * 0.5 + magnitude_score * 0.3 + sample_score * 0.2).min(100.0)
    }
}

/// Memory benchmark specifically for testing memory usage patterns
pub async fn run_memory_benchmark(
    benchmark_name: &str,
    layer: MfnLayer,
    benchmark_fn: impl std::future::Future<Output = anyhow::Result<()>>,
    memory_config: MemoryConfig,
) -> anyhow::Result<MemoryBenchmarkResult> {
    let mut profiler = MemoryProfiler::new(memory_config);
    profiler.start_profiling()?;

    let start_time = Instant::now();
    
    // Background memory sampling
    let profiler = std::sync::Arc::new(std::sync::Mutex::new(profiler));
    let profiler_clone = profiler.clone();
    
    let sampling_task = tokio::spawn(async move {
        let sampling_interval = Duration::from_millis(profiler_clone.lock().unwrap().config.sampling_interval_ms);
        loop {
            tokio::time::sleep(sampling_interval).await;
            if let Ok(mut p) = profiler_clone.try_lock() {
                let _ = p.sample_memory();
            }
        }
    });

    // Run the benchmark
    let benchmark_result = benchmark_fn.await;
    let benchmark_duration = start_time.elapsed();
    
    // Stop sampling
    sampling_task.abort();
    
    let memory_report = {
        let mut p = profiler.lock().unwrap();
        let _ = p.sample_memory(); // Final sample
        p.stop_profiling()
    };

    Ok(MemoryBenchmarkResult {
        benchmark_name: benchmark_name.to_string(),
        layer,
        duration: benchmark_duration,
        benchmark_success: benchmark_result.is_ok(),
        error_message: benchmark_result.err().map(|e| e.to_string()),
        memory_report,
    })
}

// Data structures

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryAnalysis {
    pub peak_rss_mb: f64,
    pub min_rss_mb: f64,
    pub avg_rss_mb: f64,
    pub peak_vms_mb: f64,
    pub avg_vms_mb: f64,
    pub memory_growth_rate_mb_per_sec: f64,
    pub memory_overhead_mb: f64,
    pub efficiency_score_percent: f64,
    pub exceeded_limit: bool,
    pub sample_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLeak {
    pub detected_at: Duration,
    pub growth_mb: f64,
    pub leak_rate_mb_per_sec: f64,
    pub confidence: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryProfileReport {
    pub total_duration: Duration,
    pub samples_collected: usize,
    pub baseline_memory: Option<MemorySnapshot>,
    pub final_memory: Option<MemorySnapshot>,
    pub memory_analysis: MemoryAnalysis,
    pub detected_leaks: Vec<MemoryLeak>,
    pub config: MemoryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBenchmarkResult {
    pub benchmark_name: String,
    pub layer: MfnLayer,
    pub duration: Duration,
    pub benchmark_success: bool,
    pub error_message: Option<String>,
    pub memory_report: MemoryProfileReport,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_memory_profiler_creation() {
        let config = MemoryConfig::default();
        let profiler = MemoryProfiler::new(config);
        assert_eq!(profiler.samples.len(), 0);
    }

    #[test]
    fn test_memory_snapshot() {
        let config = MemoryConfig::default();
        let profiler = MemoryProfiler::new(config);
        let snapshot = profiler.take_memory_snapshot();
        
        match snapshot {
            Ok(snap) => {
                assert!(snap.rss_mb >= 0.0);
                assert!(snap.vms_mb >= snap.rss_mb);
            }
            Err(_) => {
                // Memory profiling may not be available on all systems
                println!("Memory profiling not available on this system");
            }
        }
    }

    #[tokio::test]
    async fn test_memory_benchmark() {
        let config = MemoryConfig {
            sampling_interval_ms: 10,
            max_samples: 100,
            ..Default::default()
        };

        let result = run_memory_benchmark(
            "test_memory_benchmark",
            MfnLayer::Layer1Ifr,
            async {
                // Simulate some work that uses memory
                let _data: Vec<u8> = vec![0; 1024 * 1024]; // Allocate 1MB
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(())
            },
            config
        ).await;

        match result {
            Ok(benchmark_result) => {
                assert!(benchmark_result.benchmark_success);
                assert!(benchmark_result.memory_report.samples_collected > 0);
                assert!(benchmark_result.memory_report.memory_analysis.avg_rss_mb > 0.0);
            }
            Err(_) => {
                // Memory profiling may not be available
                println!("Memory profiling not available on this system");
            }
        }
    }

    #[test]
    fn test_leak_detection() {
        let config = MemoryConfig {
            leak_threshold_mb: 1.0,
            ..Default::default()
        };
        let mut profiler = MemoryProfiler::new(config);
        
        // Simulate memory growth pattern
        let base_time = Duration::from_secs(0);
        for i in 0..20 {
            let memory_mb = 10.0 + (i as f64 * 0.5); // Steady growth
            profiler.samples.push(MemorySample {
                timestamp: base_time + Duration::from_secs(i),
                memory_snapshot: MemorySnapshot {
                    rss_mb: memory_mb,
                    vms_mb: memory_mb * 1.5,
                    heap_mb: memory_mb * 0.8,
                    stack_mb: memory_mb * 0.1,
                    shared_mb: 0.0,
                    peak_rss_mb: memory_mb,
                },
            });
        }
        
        let leaks = profiler.detect_memory_leaks();
        assert!(!leaks.is_empty(), "Should detect memory leak pattern");
        
        if let Some(leak) = leaks.first() {
            assert!(leak.growth_mb > 0.0);
            assert!(leak.confidence > 0.0);
        }
    }
}