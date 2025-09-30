//! Performance Regression Detection System
//!
//! This module continuously monitors performance metrics and detects regressions
//! compared to established baselines. It replaces hardcoded claims with real tracking.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::Path;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};
use anyhow::{Result, Context};
use chrono::{DateTime, Utc};

/// Performance baseline for comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBaseline {
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub git_commit: String,
    pub metrics: BaselineMetrics,
}

/// Baseline performance metrics (real measurements, not fantasy)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineMetrics {
    pub throughput: ThroughputBaseline,
    pub latency: LatencyBaseline,
    pub connections: ConnectionBaseline,
    pub memory: MemoryBaseline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputBaseline {
    pub average_gbps: f64,
    pub peak_gbps: f64,
    pub p50_gbps: f64,
    pub p95_gbps: f64,
    pub p99_gbps: f64,
    pub min_acceptable_gbps: f64, // Regression threshold
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyBaseline {
    pub average_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub max_acceptable_ms: f64, // Regression threshold
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionBaseline {
    pub connections_per_sec: f64,
    pub max_concurrent: u64,
    pub success_rate: f64,
    pub min_acceptable_rate: f64, // Regression threshold
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBaseline {
    pub bytes_per_connection: u64,
    pub zero_copy_efficiency: f64,
    pub pool_hit_rate: f64,
    pub max_acceptable_memory: u64, // Regression threshold
}

/// Performance regression detection results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionReport {
    pub timestamp: DateTime<Utc>,
    pub baseline_version: String,
    pub current_version: String,
    pub regressions: Vec<Regression>,
    pub improvements: Vec<Improvement>,
    pub overall_status: RegressionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Regression {
    pub metric: String,
    pub baseline_value: f64,
    pub current_value: f64,
    pub regression_percent: f64,
    pub severity: RegressionSeverity,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Improvement {
    pub metric: String,
    pub baseline_value: f64,
    pub current_value: f64,
    pub improvement_percent: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RegressionSeverity {
    Minor,    // 5-20% regression
    Moderate, // 20-50% regression
    Severe,   // 50%+ regression
    Critical, // Below minimum acceptable threshold
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RegressionStatus {
    Pass,     // No significant regressions
    Warning,  // Minor regressions detected
    Fail,     // Significant regressions detected
    Critical, // Critical regressions, should not deploy
}

/// Performance regression detector
pub struct RegressionDetector {
    baseline: Option<PerformanceBaseline>,
    tolerance_percent: f64,
    baseline_file: String,
}

impl RegressionDetector {
    /// Create new regression detector
    pub fn new(tolerance_percent: f64) -> Self {
        Self {
            baseline: None,
            tolerance_percent,
            baseline_file: "performance_baseline.json".to_string(),
        }
    }

    /// Load baseline from file
    pub fn load_baseline(&mut self) -> Result<()> {
        let path = Path::new(&self.baseline_file);
        if !path.exists() {
            warn!("No baseline file found at {}", self.baseline_file);
            return Ok(());
        }

        let file = File::open(path)
            .context("Failed to open baseline file")?;
        let reader = BufReader::new(file);
        self.baseline = Some(serde_json::from_reader(reader)
            .context("Failed to parse baseline file")?);

        if let Some(baseline) = &self.baseline {
            info!(
                "Loaded performance baseline v{} from {}",
                baseline.version,
                baseline.timestamp
            );
        }

        Ok(())
    }

    /// Save current performance as new baseline
    pub fn save_baseline(&self, baseline: &PerformanceBaseline) -> Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.baseline_file)
            .context("Failed to create baseline file")?;

        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, baseline)
            .context("Failed to write baseline file")?;

        info!(
            "Saved new performance baseline v{} at {}",
            baseline.version,
            baseline.timestamp
        );

        Ok(())
    }

    /// Establish new baseline from current measurements
    pub fn establish_baseline(
        &mut self,
        version: String,
        git_commit: String,
        current_metrics: BaselineMetrics,
    ) -> Result<PerformanceBaseline> {
        let baseline = PerformanceBaseline {
            version,
            timestamp: Utc::now(),
            git_commit,
            metrics: current_metrics,
        };

        self.save_baseline(&baseline)?;
        self.baseline = Some(baseline.clone());

        Ok(baseline)
    }

    /// Detect regressions against baseline
    pub fn detect_regressions(
        &self,
        current_metrics: &BaselineMetrics,
        current_version: &str,
    ) -> RegressionReport {
        let baseline = match &self.baseline {
            Some(b) => b,
            None => {
                return RegressionReport {
                    timestamp: Utc::now(),
                    baseline_version: "none".to_string(),
                    current_version: current_version.to_string(),
                    regressions: vec![],
                    improvements: vec![],
                    overall_status: RegressionStatus::Warning,
                };
            }
        };

        let mut regressions = Vec::new();
        let mut improvements = Vec::new();

        // Check throughput
        let throughput_results = self.check_throughput_regression(
            &baseline.metrics.throughput,
            &current_metrics.throughput,
        );
        regressions.extend(throughput_results.0);
        improvements.extend(throughput_results.1);

        // Check latency
        let latency_results = self.check_latency_regression(
            &baseline.metrics.latency,
            &current_metrics.latency,
        );
        regressions.extend(latency_results.0);
        improvements.extend(latency_results.1);

        // Check connections
        let connection_results = self.check_connection_regression(
            &baseline.metrics.connections,
            &current_metrics.connections,
        );
        regressions.extend(connection_results.0);
        improvements.extend(connection_results.1);

        // Check memory
        let memory_results = self.check_memory_regression(
            &baseline.metrics.memory,
            &current_metrics.memory,
        );
        regressions.extend(memory_results.0);
        improvements.extend(memory_results.1);

        // Determine overall status
        let overall_status = self.determine_overall_status(&regressions);

        RegressionReport {
            timestamp: Utc::now(),
            baseline_version: baseline.version.clone(),
            current_version: current_version.to_string(),
            regressions,
            improvements,
            overall_status,
        }
    }

    fn check_throughput_regression(
        &self,
        baseline: &ThroughputBaseline,
        current: &ThroughputBaseline,
    ) -> (Vec<Regression>, Vec<Improvement>) {
        let mut regressions = Vec::new();
        let mut improvements = Vec::new();

        // Check average throughput
        let diff_percent = (baseline.average_gbps - current.average_gbps) / baseline.average_gbps * 100.0;
        if diff_percent > self.tolerance_percent {
            regressions.push(Regression {
                metric: "average_throughput".to_string(),
                baseline_value: baseline.average_gbps,
                current_value: current.average_gbps,
                regression_percent: diff_percent,
                severity: self.classify_severity(diff_percent, current.average_gbps < baseline.min_acceptable_gbps),
                description: format!(
                    "Average throughput decreased from {:.3} to {:.3} Gbps",
                    baseline.average_gbps, current.average_gbps
                ),
            });
        } else if diff_percent < -self.tolerance_percent {
            improvements.push(Improvement {
                metric: "average_throughput".to_string(),
                baseline_value: baseline.average_gbps,
                current_value: current.average_gbps,
                improvement_percent: -diff_percent,
                description: format!(
                    "Average throughput improved from {:.3} to {:.3} Gbps",
                    baseline.average_gbps, current.average_gbps
                ),
            });
        }

        // Check P95 throughput
        let p95_diff = (baseline.p95_gbps - current.p95_gbps) / baseline.p95_gbps * 100.0;
        if p95_diff > self.tolerance_percent {
            regressions.push(Regression {
                metric: "p95_throughput".to_string(),
                baseline_value: baseline.p95_gbps,
                current_value: current.p95_gbps,
                regression_percent: p95_diff,
                severity: self.classify_severity(p95_diff, false),
                description: format!(
                    "P95 throughput decreased from {:.3} to {:.3} Gbps",
                    baseline.p95_gbps, current.p95_gbps
                ),
            });
        }

        (regressions, improvements)
    }

    fn check_latency_regression(
        &self,
        baseline: &LatencyBaseline,
        current: &LatencyBaseline,
    ) -> (Vec<Regression>, Vec<Improvement>) {
        let mut regressions = Vec::new();
        let mut improvements = Vec::new();

        // For latency, increase is regression, decrease is improvement
        let diff_percent = (current.average_ms - baseline.average_ms) / baseline.average_ms * 100.0;
        if diff_percent > self.tolerance_percent {
            regressions.push(Regression {
                metric: "average_latency".to_string(),
                baseline_value: baseline.average_ms,
                current_value: current.average_ms,
                regression_percent: diff_percent,
                severity: self.classify_severity(diff_percent, current.average_ms > baseline.max_acceptable_ms),
                description: format!(
                    "Average latency increased from {:.2} to {:.2} ms",
                    baseline.average_ms, current.average_ms
                ),
            });
        } else if diff_percent < -self.tolerance_percent {
            improvements.push(Improvement {
                metric: "average_latency".to_string(),
                baseline_value: baseline.average_ms,
                current_value: current.average_ms,
                improvement_percent: -diff_percent,
                description: format!(
                    "Average latency improved from {:.2} to {:.2} ms",
                    baseline.average_ms, current.average_ms
                ),
            });
        }

        // Check P95 latency
        let p95_diff = (current.p95_ms - baseline.p95_ms) / baseline.p95_ms * 100.0;
        if p95_diff > self.tolerance_percent {
            regressions.push(Regression {
                metric: "p95_latency".to_string(),
                baseline_value: baseline.p95_ms,
                current_value: current.p95_ms,
                regression_percent: p95_diff,
                severity: self.classify_severity(p95_diff, false),
                description: format!(
                    "P95 latency increased from {:.2} to {:.2} ms",
                    baseline.p95_ms, current.p95_ms
                ),
            });
        }

        (regressions, improvements)
    }

    fn check_connection_regression(
        &self,
        baseline: &ConnectionBaseline,
        current: &ConnectionBaseline,
    ) -> (Vec<Regression>, Vec<Improvement>) {
        let mut regressions = Vec::new();
        let mut improvements = Vec::new();

        // Check connection rate
        let diff_percent = (baseline.connections_per_sec - current.connections_per_sec)
            / baseline.connections_per_sec * 100.0;
        if diff_percent > self.tolerance_percent {
            regressions.push(Regression {
                metric: "connection_rate".to_string(),
                baseline_value: baseline.connections_per_sec,
                current_value: current.connections_per_sec,
                regression_percent: diff_percent,
                severity: self.classify_severity(diff_percent, false),
                description: format!(
                    "Connection rate decreased from {:.0} to {:.0} per second",
                    baseline.connections_per_sec, current.connections_per_sec
                ),
            });
        }

        // Check success rate
        let success_diff = (baseline.success_rate - current.success_rate) * 100.0;
        if success_diff > 1.0 { // More sensitive for success rate
            regressions.push(Regression {
                metric: "connection_success_rate".to_string(),
                baseline_value: baseline.success_rate * 100.0,
                current_value: current.success_rate * 100.0,
                regression_percent: success_diff,
                severity: self.classify_severity(
                    success_diff * 10.0,
                    current.success_rate < baseline.min_acceptable_rate
                ),
                description: format!(
                    "Connection success rate decreased from {:.1}% to {:.1}%",
                    baseline.success_rate * 100.0, current.success_rate * 100.0
                ),
            });
        }

        (regressions, improvements)
    }

    fn check_memory_regression(
        &self,
        baseline: &MemoryBaseline,
        current: &MemoryBaseline,
    ) -> (Vec<Regression>, Vec<Improvement>) {
        let mut regressions = Vec::new();
        let mut improvements = Vec::new();

        // Check memory per connection (increase is regression)
        if current.bytes_per_connection > baseline.bytes_per_connection {
            let diff_percent = ((current.bytes_per_connection - baseline.bytes_per_connection)
                as f64 / baseline.bytes_per_connection as f64) * 100.0;
            if diff_percent > self.tolerance_percent {
                regressions.push(Regression {
                    metric: "memory_per_connection".to_string(),
                    baseline_value: baseline.bytes_per_connection as f64,
                    current_value: current.bytes_per_connection as f64,
                    regression_percent: diff_percent,
                    severity: self.classify_severity(
                        diff_percent,
                        current.bytes_per_connection > baseline.max_acceptable_memory
                    ),
                    description: format!(
                        "Memory per connection increased from {} to {} bytes",
                        baseline.bytes_per_connection, current.bytes_per_connection
                    ),
                });
            }
        }

        // Check zero-copy efficiency
        let zc_diff = (baseline.zero_copy_efficiency - current.zero_copy_efficiency) * 100.0;
        if zc_diff > self.tolerance_percent {
            regressions.push(Regression {
                metric: "zero_copy_efficiency".to_string(),
                baseline_value: baseline.zero_copy_efficiency * 100.0,
                current_value: current.zero_copy_efficiency * 100.0,
                regression_percent: zc_diff,
                severity: self.classify_severity(zc_diff, false),
                description: format!(
                    "Zero-copy efficiency decreased from {:.1}% to {:.1}%",
                    baseline.zero_copy_efficiency * 100.0, current.zero_copy_efficiency * 100.0
                ),
            });
        }

        (regressions, improvements)
    }

    fn classify_severity(&self, regression_percent: f64, below_threshold: bool) -> RegressionSeverity {
        if below_threshold {
            RegressionSeverity::Critical
        } else if regression_percent >= 50.0 {
            RegressionSeverity::Severe
        } else if regression_percent >= 20.0 {
            RegressionSeverity::Moderate
        } else {
            RegressionSeverity::Minor
        }
    }

    fn determine_overall_status(&self, regressions: &[Regression]) -> RegressionStatus {
        if regressions.is_empty() {
            return RegressionStatus::Pass;
        }

        let has_critical = regressions.iter().any(|r| r.severity == RegressionSeverity::Critical);
        let has_severe = regressions.iter().any(|r| r.severity == RegressionSeverity::Severe);
        let has_moderate = regressions.iter().any(|r| r.severity == RegressionSeverity::Moderate);

        if has_critical {
            RegressionStatus::Critical
        } else if has_severe {
            RegressionStatus::Fail
        } else if has_moderate {
            RegressionStatus::Warning
        } else {
            RegressionStatus::Pass
        }
    }

    /// Generate regression report as markdown
    pub fn generate_report(&self, report: &RegressionReport) -> String {
        let mut output = String::new();

        output.push_str("# Performance Regression Report\n\n");
        output.push_str(&format!("**Date**: {}\n", report.timestamp));
        output.push_str(&format!("**Baseline Version**: {}\n", report.baseline_version));
        output.push_str(&format!("**Current Version**: {}\n", report.current_version));
        output.push_str(&format!("**Status**: {:?}\n\n", report.overall_status));

        if !report.regressions.is_empty() {
            output.push_str("## ❌ Regressions Detected\n\n");
            for regression in &report.regressions {
                output.push_str(&format!(
                    "### {} ({:?})\n",
                    regression.metric, regression.severity
                ));
                output.push_str(&format!("- {}\n", regression.description));
                output.push_str(&format!(
                    "- Regression: **{:.1}%**\n",
                    regression.regression_percent
                ));
                output.push_str(&format!(
                    "- Baseline: {:.3}, Current: {:.3}\n\n",
                    regression.baseline_value, regression.current_value
                ));
            }
        }

        if !report.improvements.is_empty() {
            output.push_str("## ✅ Improvements\n\n");
            for improvement in &report.improvements {
                output.push_str(&format!("### {}\n", improvement.metric));
                output.push_str(&format!("- {}\n", improvement.description));
                output.push_str(&format!(
                    "- Improvement: **{:.1}%**\n\n",
                    improvement.improvement_percent
                ));
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regression_detection() {
        let detector = RegressionDetector::new(5.0);

        let baseline = BaselineMetrics {
            throughput: ThroughputBaseline {
                average_gbps: 2.0,
                peak_gbps: 3.0,
                p50_gbps: 1.8,
                p95_gbps: 2.5,
                p99_gbps: 2.8,
                min_acceptable_gbps: 1.0,
            },
            latency: LatencyBaseline {
                average_ms: 10.0,
                p50_ms: 8.0,
                p95_ms: 15.0,
                p99_ms: 20.0,
                max_acceptable_ms: 25.0,
            },
            connections: ConnectionBaseline {
                connections_per_sec: 1000.0,
                max_concurrent: 10000,
                success_rate: 0.99,
                min_acceptable_rate: 0.95,
            },
            memory: MemoryBaseline {
                bytes_per_connection: 1024 * 1024,
                zero_copy_efficiency: 0.9,
                pool_hit_rate: 0.85,
                max_acceptable_memory: 2 * 1024 * 1024,
            },
        };

        let current = BaselineMetrics {
            throughput: ThroughputBaseline {
                average_gbps: 1.5, // 25% regression
                peak_gbps: 2.8,
                p50_gbps: 1.4,
                p95_gbps: 2.0,
                p99_gbps: 2.5,
                min_acceptable_gbps: 1.0,
            },
            latency: LatencyBaseline {
                average_ms: 12.0, // 20% regression
                p50_ms: 9.0,
                p95_ms: 18.0,
                p99_ms: 25.0,
                max_acceptable_ms: 25.0,
            },
            connections: ConnectionBaseline {
                connections_per_sec: 950.0, // 5% regression
                max_concurrent: 10000,
                success_rate: 0.98,
                min_acceptable_rate: 0.95,
            },
            memory: MemoryBaseline {
                bytes_per_connection: 1024 * 1024,
                zero_copy_efficiency: 0.88,
                pool_hit_rate: 0.85,
                max_acceptable_memory: 2 * 1024 * 1024,
            },
        };

        let report = detector.detect_regressions(&current, "test-version");

        assert!(!report.regressions.is_empty());
        assert!(report.overall_status != RegressionStatus::Pass);
    }
}