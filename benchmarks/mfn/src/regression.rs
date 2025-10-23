/*!
# Performance Regression Detection and Monitoring

Automated performance regression detection with:
- Historical performance tracking
- Statistical regression analysis  
- Automated alerting for performance degradation
- Trend analysis and prediction
- CI/CD integration for automated testing
*/

use crate::common::*;
use crate::analysis::{StatisticalAnalysis, AnalysisConfig, PerformanceRegression, RegressionType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

/// Regression detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionConfig {
    pub regression_threshold: f64,     // 5% degradation triggers alert
    pub history_window_days: u32,      // Look back 30 days for comparison
    pub min_samples_for_detection: usize, // Need at least 10 samples
    pub statistical_confidence: f64,   // 95% confidence for regression
    pub enable_trend_analysis: bool,
    pub alert_on_detection: bool,
    pub storage_path: String,
}

impl Default for RegressionConfig {
    fn default() -> Self {
        Self {
            regression_threshold: 0.05,
            history_window_days: 30,
            min_samples_for_detection: 10,
            statistical_confidence: 0.95,
            enable_trend_analysis: true,
            alert_on_detection: true,
            storage_path: "./benchmark_history".to_string(),
        }
    }
}

/// Performance regression testing framework
pub struct RegressionTest {
    config: RegressionConfig,
    historical_data: HistoricalData,
    analysis_engine: StatisticalAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalData {
    pub benchmarks: Vec<HistoricalBenchmark>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalBenchmark {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub layer: MfnLayer,
    pub benchmark_name: String,
    pub latency_ms: f64,
    pub throughput_ops_sec: f64,
    pub memory_mb: f64,
    pub success: bool,
    pub target_met: bool,
    pub git_commit: Option<String>,
    pub build_info: Option<String>,
}

impl RegressionTest {
    pub fn new(config: RegressionConfig) -> anyhow::Result<Self> {
        let historical_data = Self::load_historical_data(&config)?;
        let analysis_config = AnalysisConfig {
            confidence_level: config.statistical_confidence,
            regression_threshold: config.regression_threshold,
            ..Default::default()
        };
        let analysis_engine = StatisticalAnalysis::new(analysis_config);

        Ok(Self {
            config,
            historical_data,
            analysis_engine,
        })
    }

    fn load_historical_data(config: &RegressionConfig) -> anyhow::Result<HistoricalData> {
        let storage_path = Path::new(&config.storage_path);
        let data_file = storage_path.join("historical_data.json");

        if data_file.exists() {
            let content = fs::read_to_string(&data_file)?;
            let data: HistoricalData = serde_json::from_str(&content)?;
            Ok(data)
        } else {
            Ok(HistoricalData {
                benchmarks: Vec::new(),
                last_updated: chrono::Utc::now(),
                version: crate::VERSION.to_string(),
            })
        }
    }

    /// Store new benchmark results for future regression analysis
    pub fn store_benchmark_results(&mut self, results: &[BenchmarkResult]) -> anyhow::Result<()> {
        let git_commit = self.get_current_git_commit();
        let build_info = self.get_build_info();

        for result in results {
            let historical_benchmark = HistoricalBenchmark {
                timestamp: chrono::Utc::now(),
                layer: result.layer,
                benchmark_name: result.name.clone(),
                latency_ms: result.metrics.latency_percentiles.mean.as_secs_f64() * 1000.0,
                throughput_ops_sec: result.metrics.throughput_ops_per_sec,
                memory_mb: result.metrics.memory_usage_mb,
                success: result.success,
                target_met: result.target_validation.overall_success,
                git_commit: git_commit.clone(),
                build_info: build_info.clone(),
            };
            
            self.historical_data.benchmarks.push(historical_benchmark);
        }

        self.historical_data.last_updated = chrono::Utc::now();
        self.cleanup_old_data();
        self.save_historical_data()?;

        Ok(())
    }

    fn cleanup_old_data(&mut self) {
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(self.config.history_window_days as i64);
        self.historical_data.benchmarks.retain(|b| b.timestamp > cutoff_date);
    }

    fn save_historical_data(&self) -> anyhow::Result<()> {
        let storage_path = Path::new(&self.config.storage_path);
        fs::create_dir_all(storage_path)?;
        
        let data_file = storage_path.join("historical_data.json");
        let content = serde_json::to_string_pretty(&self.historical_data)?;
        fs::write(data_file, content)?;

        Ok(())
    }

    /// Detect regressions in new benchmark results compared to historical data
    pub fn detect_regressions(&self, current_results: &[BenchmarkResult]) -> anyhow::Result<RegressionReport> {
        let mut detected_regressions = Vec::new();
        let mut layer_trends = HashMap::new();

        for current_result in current_results {
            // Get historical data for this benchmark
            let historical_results = self.get_historical_for_benchmark(
                current_result.layer,
                &current_result.name
            );

            if historical_results.len() < self.config.min_samples_for_detection {
                continue; // Not enough historical data
            }

            // Detect latency regression
            if let Some(latency_regression) = self.detect_latency_regression(current_result, &historical_results)? {
                detected_regressions.push(latency_regression);
            }

            // Detect throughput regression
            if let Some(throughput_regression) = self.detect_throughput_regression(current_result, &historical_results)? {
                detected_regressions.push(throughput_regression);
            }

            // Detect memory regression
            if let Some(memory_regression) = self.detect_memory_regression(current_result, &historical_results)? {
                detected_regressions.push(memory_regression);
            }

            // Analyze trends if enabled
            if self.config.enable_trend_analysis {
                let trend = self.analyze_layer_trend(current_result.layer, &historical_results);
                layer_trends.insert(current_result.layer, trend);
            }
        }

        let report = RegressionReport {
            timestamp: chrono::Utc::now(),
            regressions_detected: detected_regressions.len(),
            regressions: detected_regressions,
            layer_trends,
            total_benchmarks_analyzed: current_results.len(),
            historical_window_days: self.config.history_window_days,
            regression_threshold: self.config.regression_threshold,
            statistical_confidence: self.config.statistical_confidence,
        };

        // Send alerts if configured
        if self.config.alert_on_detection && !report.regressions.is_empty() {
            self.send_regression_alerts(&report)?;
        }

        Ok(report)
    }

    fn get_historical_for_benchmark(&self, layer: MfnLayer, name: &str) -> Vec<&HistoricalBenchmark> {
        self.historical_data.benchmarks.iter()
            .filter(|b| b.layer == layer && b.benchmark_name == name)
            .collect()
    }

    fn detect_latency_regression(
        &self,
        current: &BenchmarkResult,
        historical: &[&HistoricalBenchmark]
    ) -> anyhow::Result<Option<PerformanceRegression>> {
        let current_latency = current.metrics.latency_percentiles.mean.as_secs_f64() * 1000.0;
        let historical_latencies: Vec<f64> = historical.iter().map(|h| h.latency_ms).collect();
        
        let historical_mean = historical_latencies.iter().sum::<f64>() / historical_latencies.len() as f64;
        let regression_percent = ((current_latency - historical_mean) / historical_mean) * 100.0;

        if regression_percent > self.config.regression_threshold * 100.0 {
            // Perform statistical test
            let is_significant = self.perform_regression_test(current_latency, &historical_latencies)?;
            
            Ok(Some(PerformanceRegression {
                regression_type: RegressionType::LatencyIncrease,
                layer: current.layer,
                metric_name: format!("{}_latency", current.name),
                current_value: current_latency,
                historical_value: historical_mean,
                regression_percent,
                is_significant,
                confidence_level: self.config.statistical_confidence,
                detection_timestamp: chrono::Utc::now(),
            }))
        } else {
            Ok(None)
        }
    }

    fn detect_throughput_regression(
        &self,
        current: &BenchmarkResult,
        historical: &[&HistoricalBenchmark]
    ) -> anyhow::Result<Option<PerformanceRegression>> {
        let current_throughput = current.metrics.throughput_ops_per_sec;
        let historical_throughputs: Vec<f64> = historical.iter().map(|h| h.throughput_ops_sec).collect();
        
        let historical_mean = historical_throughputs.iter().sum::<f64>() / historical_throughputs.len() as f64;
        let regression_percent = ((historical_mean - current_throughput) / historical_mean) * 100.0;

        if regression_percent > self.config.regression_threshold * 100.0 {
            let is_significant = self.perform_regression_test(current_throughput, &historical_throughputs)?;
            
            Ok(Some(PerformanceRegression {
                regression_type: RegressionType::ThroughputDecrease,
                layer: current.layer,
                metric_name: format!("{}_throughput", current.name),
                current_value: current_throughput,
                historical_value: historical_mean,
                regression_percent,
                is_significant,
                confidence_level: self.config.statistical_confidence,
                detection_timestamp: chrono::Utc::now(),
            }))
        } else {
            Ok(None)
        }
    }

    fn detect_memory_regression(
        &self,
        current: &BenchmarkResult,
        historical: &[&HistoricalBenchmark]
    ) -> anyhow::Result<Option<PerformanceRegression>> {
        let current_memory = current.metrics.memory_usage_mb;
        let historical_memory: Vec<f64> = historical.iter().map(|h| h.memory_mb).collect();
        
        let historical_mean = historical_memory.iter().sum::<f64>() / historical_memory.len() as f64;
        let regression_percent = ((current_memory - historical_mean) / historical_mean) * 100.0;

        // Memory increase of more than threshold is considered regression
        if regression_percent > self.config.regression_threshold * 100.0 {
            let is_significant = self.perform_regression_test(current_memory, &historical_memory)?;
            
            Ok(Some(PerformanceRegression {
                regression_type: RegressionType::MemoryIncrease,
                layer: current.layer,
                metric_name: format!("{}_memory", current.name),
                current_value: current_memory,
                historical_value: historical_mean,
                regression_percent,
                is_significant,
                confidence_level: self.config.statistical_confidence,
                detection_timestamp: chrono::Utc::now(),
            }))
        } else {
            Ok(None)
        }
    }

    fn perform_regression_test(&self, current_value: f64, historical_values: &[f64]) -> anyhow::Result<bool> {
        if historical_values.len() < 2 {
            return Ok(false); // Can't perform statistical test
        }

        // Simple statistical test: is current value outside confidence interval?
        let mean = historical_values.iter().sum::<f64>() / historical_values.len() as f64;
        let variance = historical_values.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / (historical_values.len() - 1) as f64;
        let std_dev = variance.sqrt();
        
        // Use 2-sigma rule (approximately 95% confidence)
        let margin = 2.0 * std_dev;
        let upper_bound = mean + margin;
        let lower_bound = mean - margin;
        
        // For latency/memory: regression if current > upper_bound
        // For throughput: regression if current < lower_bound
        Ok(current_value > upper_bound || current_value < lower_bound)
    }

    fn analyze_layer_trend(&self, layer: MfnLayer, historical: &[&HistoricalBenchmark]) -> TrendAnalysis {
        if historical.len() < 5 {
            return TrendAnalysis {
                trend_direction: "Insufficient data".to_string(),
                slope: 0.0,
                r_squared: 0.0,
                is_significant_trend: false,
                data_points: historical.len(),
                trend_strength: TrendStrength::None,
            };
        }

        // Sort by timestamp
        let mut sorted_data: Vec<_> = historical.iter().collect();
        sorted_data.sort_by_key(|h| h.timestamp);

        // Simple linear regression on latency over time
        let latencies: Vec<f64> = sorted_data.iter().map(|h| h.latency_ms).collect();
        let n = latencies.len() as f64;
        
        // Use indices as x values (time progression)
        let x_mean = (n - 1.0) / 2.0;
        let y_mean = latencies.iter().sum::<f64>() / n;

        let mut numerator = 0.0;
        let mut denominator = 0.0;

        for (i, &y) in latencies.iter().enumerate() {
            let x = i as f64;
            numerator += (x - x_mean) * (y - y_mean);
            denominator += (x - x_mean).powi(2);
        }

        let slope = if denominator != 0.0 { numerator / denominator } else { 0.0 };
        
        // Calculate R-squared
        let ss_total: f64 = latencies.iter().map(|&y| (y - y_mean).powi(2)).sum();
        let ss_residual: f64 = latencies.iter().enumerate()
            .map(|(i, &y)| {
                let predicted = slope * (i as f64) + (y_mean - slope * x_mean);
                (y - predicted).powi(2)
            })
            .sum();

        let r_squared = if ss_total != 0.0 { 1.0 - (ss_residual / ss_total) } else { 0.0 };

        let trend_direction = if slope > 0.01 {
            "Degrading".to_string()
        } else if slope < -0.01 {
            "Improving".to_string()
        } else {
            "Stable".to_string()
        };

        let is_significant = r_squared > 0.5 && slope.abs() > 0.01;
        
        let trend_strength = if r_squared > 0.8 {
            TrendStrength::Strong
        } else if r_squared > 0.5 {
            TrendStrength::Moderate
        } else if r_squared > 0.2 {
            TrendStrength::Weak
        } else {
            TrendStrength::None
        };

        TrendAnalysis {
            trend_direction,
            slope,
            r_squared,
            is_significant_trend: is_significant,
            data_points: historical.len(),
            trend_strength,
        }
    }

    fn send_regression_alerts(&self, report: &RegressionReport) -> anyhow::Result<()> {
        // In a real implementation, this would send alerts via:
        // - Email notifications
        // - Slack/Teams messages  
        // - GitHub issue creation
        // - PagerDuty alerts
        // - Custom webhook calls

        println!("🚨 PERFORMANCE REGRESSION DETECTED!");
        println!("═══════════════════════════════════");
        println!("Detected {} regressions at {}", 
                 report.regressions_detected, 
                 report.timestamp.format("%Y-%m-%d %H:%M:%S UTC"));

        for regression in &report.regressions {
            println!("\n❌ {} Regression in {}", 
                     format!("{:?}", regression.regression_type), 
                     regression.metric_name);
            println!("   Current: {:.3} | Historical: {:.3} | Regression: {:.1}%",
                     regression.current_value,
                     regression.historical_value,
                     regression.regression_percent);
            println!("   Statistically Significant: {}", 
                     if regression.is_significant { "Yes" } else { "No" });
        }

        println!("\n📊 Analysis Summary:");
        println!("   Threshold: {:.1}%", report.regression_threshold * 100.0);
        println!("   Confidence: {:.1}%", report.statistical_confidence * 100.0);
        println!("   Historical Window: {} days", report.historical_window_days);

        Ok(())
    }

    fn get_current_git_commit(&self) -> Option<String> {
        std::process::Command::new("git")
            .args(&["rev-parse", "HEAD"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout).ok().map(|s| s.trim().to_string())
                } else {
                    None
                }
            })
    }

    fn get_build_info(&self) -> Option<String> {
        // This would typically include:
        // - Compiler version
        // - Build flags
        // - Dependencies
        // - Architecture
        Some(format!("rustc-{}", env!("CARGO_PKG_VERSION")))
    }

    /// Generate continuous integration report
    pub fn generate_ci_report(&self, current_results: &[BenchmarkResult]) -> anyhow::Result<CiReport> {
        let regression_report = self.detect_regressions(current_results)?;
        
        let success_rate = if !current_results.is_empty() {
            current_results.iter().filter(|r| r.success).count() as f64 / current_results.len() as f64
        } else {
            0.0
        };

        let target_achievement_rate = if !current_results.is_empty() {
            current_results.iter().filter(|r| r.target_validation.overall_success).count() as f64 / current_results.len() as f64
        } else {
            0.0
        };

        // CI passes if no significant regressions and good success rates
        let ci_pass = regression_report.regressions.iter()
            .filter(|r| r.is_significant)
            .count() == 0 
            && success_rate >= 0.95 
            && target_achievement_rate >= 0.90;

        Ok(CiReport {
            timestamp: chrono::Utc::now(),
            ci_pass,
            success_rate,
            target_achievement_rate,
            regressions_found: regression_report.regressions_detected,
            significant_regressions: regression_report.regressions.iter()
                .filter(|r| r.is_significant)
                .count(),
            regression_details: regression_report.regressions,
            exit_code: if ci_pass { 0 } else { 1 },
            summary: format!(
                "CI {}: {}/{} benchmarks passed, {}/{} targets met, {} significant regressions",
                if ci_pass { "PASSED" } else { "FAILED" },
                (success_rate * current_results.len() as f64) as usize,
                current_results.len(),
                (target_achievement_rate * current_results.len() as f64) as usize,
                current_results.len(),
                regression_report.regressions.iter().filter(|r| r.is_significant).count()
            ),
        })
    }
}

// Data structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionReport {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub regressions_detected: usize,
    pub regressions: Vec<PerformanceRegression>,
    pub layer_trends: HashMap<MfnLayer, TrendAnalysis>,
    pub total_benchmarks_analyzed: usize,
    pub historical_window_days: u32,
    pub regression_threshold: f64,
    pub statistical_confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub trend_direction: String,
    pub slope: f64,
    pub r_squared: f64,
    pub is_significant_trend: bool,
    pub data_points: usize,
    pub trend_strength: TrendStrength,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendStrength {
    None,
    Weak,
    Moderate,
    Strong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiReport {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub ci_pass: bool,
    pub success_rate: f64,
    pub target_achievement_rate: f64,
    pub regressions_found: usize,
    pub significant_regressions: usize,
    pub regression_details: Vec<PerformanceRegression>,
    pub exit_code: i32,
    pub summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regression_config() {
        let config = RegressionConfig::default();
        assert_eq!(config.regression_threshold, 0.05);
        assert_eq!(config.history_window_days, 30);
    }

    #[tokio::test]
    async fn test_regression_detection() {
        let config = RegressionConfig {
            storage_path: "/tmp/test_regression".to_string(),
            min_samples_for_detection: 2,
            ..Default::default()
        };

        let mut regression_test = RegressionTest::new(config).unwrap();

        // Store some historical data
        let historical_result = create_test_result(MfnLayer::Layer1Ifr, 1.0, 1000000.0);
        regression_test.store_benchmark_results(&[historical_result]).unwrap();

        // Test with a regressed result
        let current_result = create_test_result(MfnLayer::Layer1Ifr, 2.0, 500000.0); // Much worse
        let report = regression_test.detect_regressions(&[current_result]).unwrap();

        // Should detect regressions
        assert!(report.regressions_detected > 0);
    }

    #[test]
    fn test_trend_analysis() {
        let config = RegressionConfig::default();
        let regression_test = RegressionTest::new(config).unwrap();

        // Create historical data with upward trend (degrading performance)
        let historical_data: Vec<HistoricalBenchmark> = (0..10)
            .map(|i| HistoricalBenchmark {
                timestamp: chrono::Utc::now() - chrono::Duration::days(10 - i),
                layer: MfnLayer::Layer1Ifr,
                benchmark_name: "test".to_string(),
                latency_ms: 1.0 + (i as f64 * 0.1), // Increasing latency
                throughput_ops_sec: 1000000.0,
                memory_mb: 10.0,
                success: true,
                target_met: true,
                git_commit: None,
                build_info: None,
            })
            .collect();

        let historical_refs: Vec<&HistoricalBenchmark> = historical_data.iter().collect();
        let trend = regression_test.analyze_layer_trend(MfnLayer::Layer1Ifr, &historical_refs);

        assert_eq!(trend.trend_direction, "Degrading");
        assert!(trend.slope > 0.0);
        assert!(trend.r_squared > 0.8); // Should be highly correlated
    }

    fn create_test_result(layer: MfnLayer, latency_ms: f64, throughput: f64) -> BenchmarkResult {
        BenchmarkResult {
            id: "test".to_string(),
            name: "test_benchmark".to_string(),
            layer,
            config: BenchmarkConfig {
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
            },
            metrics: PerformanceMetrics {
                benchmark_id: "test".to_string(),
                layer,
                timestamp: chrono::Utc::now(),
                duration: Duration::from_secs(1),
                throughput_ops_per_sec: throughput,
                latency_percentiles: LatencyPercentiles {
                    p50: Duration::from_secs_f64(latency_ms / 1000.0),
                    p75: Duration::from_secs_f64(latency_ms / 1000.0),
                    p90: Duration::from_secs_f64(latency_ms / 1000.0),
                    p95: Duration::from_secs_f64(latency_ms / 1000.0),
                    p99: Duration::from_secs_f64(latency_ms / 1000.0),
                    p999: Duration::from_secs_f64(latency_ms / 1000.0),
                    max: Duration::from_secs_f64(latency_ms / 1000.0),
                    min: Duration::from_secs_f64(latency_ms / 1000.0),
                    mean: Duration::from_secs_f64(latency_ms / 1000.0),
                    stddev: Duration::from_secs_f64(0.1),
                },
                memory_usage_mb: 10.0,
                cpu_utilization: 25.0,
                error_rate: 0.0,
                custom_metrics: HashMap::new(),
            },
            target_validation: TargetValidation {
                latency_target_met: latency_ms < 0.1,
                throughput_target_met: throughput > 1000000.0,
                memory_target_met: true,
                improvement_target_met: true,
                overall_success: true,
                target_details: HashMap::new(),
            },
            baseline_comparison: None,
            success: true,
            error_message: None,
        }
    }
}