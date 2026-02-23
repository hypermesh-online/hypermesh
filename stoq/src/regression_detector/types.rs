// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Types for the performance regression detection system.

use serde::{Deserialize, Serialize};
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
