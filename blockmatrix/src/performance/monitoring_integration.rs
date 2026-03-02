// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Monitoring Integration
//!
//! Lightweight monitoring wrapper that tracks key system metrics with
//! configurable export intervals and alert thresholds.

use std::collections::HashMap;
use std::time::{Instant, SystemTime};

use serde::{Deserialize, Serialize};

/// Configuration for the monitoring system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Whether monitoring is enabled.
    pub enabled: bool,
    /// How often to export metrics (seconds).
    pub export_interval_secs: u64,
    /// Maximum overhead percentage tolerated (0.0..1.0).
    pub max_overhead_pct: f64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            export_interval_secs: 30,
            max_overhead_pct: 1.0,
        }
    }
}

/// Core monitoring integration.
pub struct MonitoringIntegration {
    config: MonitoringConfig,
    samples: Vec<MetricSample>,
    started_at: Instant,
}

/// A single metric sample.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSample {
    pub name: String,
    pub value: f64,
    pub timestamp: SystemTime,
}

/// Exports collected metrics (placeholder for real exporters).
pub struct MetricsExporter;

impl MetricsExporter {
    pub fn new() -> Self {
        Self
    }

    /// Export the given samples. In production this would push to Prometheus / OTLP.
    pub fn export(&self, samples: &[MetricSample]) -> usize {
        samples.len()
    }
}

impl Default for MetricsExporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Alerting manager that checks metric values against thresholds.
pub struct AlertingManager {
    thresholds: HashMap<String, f64>,
}

impl AlertingManager {
    pub fn new() -> Self {
        Self {
            thresholds: HashMap::new(),
        }
    }

    /// Set a threshold for a metric. Alerts fire when value exceeds this.
    pub fn set_threshold(&mut self, metric: &str, limit: f64) {
        self.thresholds.insert(metric.to_string(), limit);
    }

    /// Check samples against thresholds. Returns names of metrics that breach.
    pub fn check(&self, samples: &[MetricSample]) -> Vec<String> {
        let mut alerts = Vec::new();
        for sample in samples {
            if let Some(&limit) = self.thresholds.get(&sample.name) {
                if sample.value > limit {
                    alerts.push(sample.name.clone());
                }
            }
        }
        alerts
    }
}

impl Default for AlertingManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Dashboard manager (placeholder).
pub struct DashboardManager;

impl DashboardManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DashboardManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MonitoringIntegration {
    /// Create a new monitoring integration.
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            samples: Vec::new(),
            started_at: Instant::now(),
        }
    }

    /// Record a metric sample.
    pub fn record(&mut self, name: &str, value: f64) {
        if !self.config.enabled {
            return;
        }
        self.samples.push(MetricSample {
            name: name.to_string(),
            value,
            timestamp: SystemTime::now(),
        });
    }

    /// Get collected samples.
    pub fn samples(&self) -> &[MetricSample] {
        &self.samples
    }

    /// Flush and return all samples, clearing the internal buffer.
    pub fn flush(&mut self) -> Vec<MetricSample> {
        std::mem::take(&mut self.samples)
    }

    /// Uptime in seconds.
    pub fn uptime_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitoring_record_and_flush() {
        let config = MonitoringConfig::default();
        let mut mon = MonitoringIntegration::new(config);

        mon.record("cpu_pct", 45.0);
        mon.record("mem_mb", 2048.0);

        assert_eq!(mon.samples().len(), 2);

        let flushed = mon.flush();
        assert_eq!(flushed.len(), 2);
        assert!(mon.samples().is_empty());
    }

    #[test]
    fn test_alerting_manager_breach() {
        let mut am = AlertingManager::new();
        am.set_threshold("cpu_pct", 80.0);

        let samples = vec![
            MetricSample {
                name: "cpu_pct".to_string(),
                value: 90.0,
                timestamp: SystemTime::now(),
            },
            MetricSample {
                name: "mem_mb".to_string(),
                value: 1024.0,
                timestamp: SystemTime::now(),
            },
        ];

        let alerts = am.check(&samples);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0], "cpu_pct");
    }
}
