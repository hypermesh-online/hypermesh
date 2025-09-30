//! Metrics export functionality

use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use serde_json;
use std::collections::HashMap;
use super::MetricsSnapshot;

/// Export format for metrics
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ExportFormat {
    Json,
    Prometheus,
    PlainText,
    Csv,
}

/// Metrics exporter trait
#[async_trait]
pub trait MetricsExporter: Send + Sync {
    /// Export metrics
    async fn export(&self, snapshot: &MetricsSnapshot) -> anyhow::Result<String>;

    /// Get export format
    fn format(&self) -> ExportFormat;
}

/// JSON metrics exporter
pub struct JsonExporter;

#[async_trait]
impl MetricsExporter for JsonExporter {
    async fn export(&self, snapshot: &MetricsSnapshot) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(snapshot)?)
    }

    fn format(&self) -> ExportFormat {
        ExportFormat::Json
    }
}

/// Prometheus-compatible metrics exporter
pub struct PrometheusExporter {
    prefix: String,
}

impl PrometheusExporter {
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
        }
    }
}

#[async_trait]
impl MetricsExporter for PrometheusExporter {
    async fn export(&self, snapshot: &MetricsSnapshot) -> anyhow::Result<String> {
        let mut output = Vec::new();

        // Add header
        output.push(format!("# HELP {}up TrustChain service status", self.prefix));
        output.push(format!("# TYPE {}up gauge", self.prefix));
        output.push(format!("{}up 1", self.prefix));

        // Export component metrics
        for (component, metrics) in &snapshot.components {
            let component_prefix = format!("{}_{}", self.prefix, component);

            // Total operations
            output.push(format!("# HELP {}_total Total operations", component_prefix));
            output.push(format!("# TYPE {}_total counter", component_prefix));
            output.push(format!("{}_total {}", component_prefix, metrics.total_operations));

            // Success rate
            output.push(format!("# HELP {}_success_rate Success rate", component_prefix));
            output.push(format!("# TYPE {}_success_rate gauge", component_prefix));
            output.push(format!("{}_success_rate {}", component_prefix, metrics.success_rate));

            // Average operation time
            output.push(format!("# HELP {}_avg_ms Average operation time in milliseconds", component_prefix));
            output.push(format!("# TYPE {}_avg_ms gauge", component_prefix));
            output.push(format!("{}_avg_ms {}", component_prefix, metrics.avg_operation_time_ms));

            // Additional metrics
            for (name, value) in &metrics.additional_metrics {
                let metric_name = format!("{}_{}", component_prefix, name);
                output.push(format!("# HELP {} {}", metric_name, name));
                output.push(format!("# TYPE {} gauge", metric_name));
                output.push(format!("{} {}", metric_name, value));
            }
        }

        // Export counters
        for (name, value) in &snapshot.counters {
            let counter_name = format!("{}_{}", self.prefix, name);
            output.push(format!("# HELP {} Counter: {}", counter_name, name));
            output.push(format!("# TYPE {} counter", counter_name));
            output.push(format!("{} {}", counter_name, value));
        }

        // Export timing statistics
        for (name, stats) in &snapshot.timing_stats {
            let timing_prefix = format!("{}_timing_{}", self.prefix, name);

            output.push(format!("# HELP {}_min_ms Minimum time", timing_prefix));
            output.push(format!("# TYPE {}_min_ms gauge", timing_prefix));
            output.push(format!("{}_min_ms {}", timing_prefix, stats.min_ms));

            output.push(format!("# HELP {}_max_ms Maximum time", timing_prefix));
            output.push(format!("# TYPE {}_max_ms gauge", timing_prefix));
            output.push(format!("{}_max_ms {}", timing_prefix, stats.max_ms));

            output.push(format!("# HELP {}_avg_ms Average time", timing_prefix));
            output.push(format!("# TYPE {}_avg_ms gauge", timing_prefix));
            output.push(format!("{}_avg_ms {}", timing_prefix, stats.avg_ms));

            output.push(format!("# HELP {}_p95_ms 95th percentile", timing_prefix));
            output.push(format!("# TYPE {}_p95_ms gauge", timing_prefix));
            output.push(format!("{}_p95_ms {}", timing_prefix, stats.p95_ms));

            output.push(format!("# HELP {}_p99_ms 99th percentile", timing_prefix));
            output.push(format!("# TYPE {}_p99_ms gauge", timing_prefix));
            output.push(format!("{}_p99_ms {}", timing_prefix, stats.p99_ms));
        }

        Ok(output.join("\n"))
    }

    fn format(&self) -> ExportFormat {
        ExportFormat::Prometheus
    }
}

/// Plain text metrics exporter
pub struct PlainTextExporter;

#[async_trait]
impl MetricsExporter for PlainTextExporter {
    async fn export(&self, snapshot: &MetricsSnapshot) -> anyhow::Result<String> {
        let mut output = Vec::new();

        output.push(format!("TrustChain Metrics Report"));
        output.push(format!("========================"));
        output.push(format!("Timestamp: {:?}", snapshot.timestamp));
        output.push(String::new());

        // Component metrics
        output.push(format!("Component Metrics:"));
        output.push(format!("------------------"));
        for (component, metrics) in &snapshot.components {
            output.push(format!("{} Component:", component.to_uppercase()));
            output.push(format!("  Total Operations: {}", metrics.total_operations));
            output.push(format!("  Success Rate: {:.2}%", metrics.success_rate * 100.0));
            output.push(format!("  Avg Operation Time: {:.2}ms", metrics.avg_operation_time_ms));
            output.push(String::new());
        }

        // Timing statistics
        if !snapshot.timing_stats.is_empty() {
            output.push(format!("Timing Statistics:"));
            output.push(format!("------------------"));
            for (name, stats) in &snapshot.timing_stats {
                output.push(format!("{} Timing:", name.to_uppercase()));
                output.push(format!("  Min: {}ms", stats.min_ms));
                output.push(format!("  Max: {}ms", stats.max_ms));
                output.push(format!("  Avg: {:.2}ms", stats.avg_ms));
                output.push(format!("  P95: {}ms", stats.p95_ms));
                output.push(format!("  P99: {}ms", stats.p99_ms));
                output.push(format!("  Samples: {}", stats.count));
                output.push(String::new());
            }
        }

        // Counters
        if !snapshot.counters.is_empty() {
            output.push(format!("Counters:"));
            output.push(format!("---------"));
            for (name, value) in &snapshot.counters {
                output.push(format!("  {}: {}", name, value));
            }
        }

        Ok(output.join("\n"))
    }

    fn format(&self) -> ExportFormat {
        ExportFormat::PlainText
    }
}

/// CSV metrics exporter
pub struct CsvExporter;

#[async_trait]
impl MetricsExporter for CsvExporter {
    async fn export(&self, snapshot: &MetricsSnapshot) -> anyhow::Result<String> {
        let mut lines = Vec::new();

        // Header
        lines.push("component,metric,value,timestamp".to_string());

        // Component metrics
        for (component, metrics) in &snapshot.components {
            lines.push(format!("{},total_operations,{},{:?}",
                component, metrics.total_operations, snapshot.timestamp));
            lines.push(format!("{},success_rate,{},{:?}",
                component, metrics.success_rate, snapshot.timestamp));
            lines.push(format!("{},avg_operation_time_ms,{},{:?}",
                component, metrics.avg_operation_time_ms, snapshot.timestamp));

            for (name, value) in &metrics.additional_metrics {
                lines.push(format!("{},{},{},{:?}",
                    component, name, value, snapshot.timestamp));
            }
        }

        // Counters
        for (name, value) in &snapshot.counters {
            lines.push(format!("global,{},{},{:?}",
                name, value, snapshot.timestamp));
        }

        // Timing stats
        for (name, stats) in &snapshot.timing_stats {
            lines.push(format!("{}_timing,min_ms,{},{:?}",
                name, stats.min_ms, snapshot.timestamp));
            lines.push(format!("{}_timing,max_ms,{},{:?}",
                name, stats.max_ms, snapshot.timestamp));
            lines.push(format!("{}_timing,avg_ms,{},{:?}",
                name, stats.avg_ms, snapshot.timestamp));
            lines.push(format!("{}_timing,p95_ms,{},{:?}",
                name, stats.p95_ms, snapshot.timestamp));
            lines.push(format!("{}_timing,p99_ms,{},{:?}",
                name, stats.p99_ms, snapshot.timestamp));
        }

        Ok(lines.join("\n"))
    }

    fn format(&self) -> ExportFormat {
        ExportFormat::Csv
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::metrics::{ComponentMetrics, TimingStats};
    use std::time::SystemTime;

    fn create_test_snapshot() -> MetricsSnapshot {
        let mut components = HashMap::new();
        let mut component = ComponentMetrics {
            name: "ca".to_string(),
            total_operations: 100,
            successful_operations: 95,
            failed_operations: 5,
            success_rate: 0.95,
            avg_operation_time_ms: 35.0,
            last_operation: Some(SystemTime::now()),
            additional_metrics: HashMap::new(),
        };
        component.additional_metrics.insert("cert_count".to_string(), 1000.0);
        components.insert("ca".to_string(), component);

        let mut counters = HashMap::new();
        counters.insert("total_requests".to_string(), 5000);

        let mut timing_stats = HashMap::new();
        timing_stats.insert("ca".to_string(), TimingStats {
            min_ms: 10,
            max_ms: 100,
            avg_ms: 35.0,
            median_ms: 30,
            p95_ms: 80,
            p99_ms: 95,
            count: 100,
        });

        MetricsSnapshot {
            timestamp: SystemTime::now(),
            components,
            counters,
            timing_stats,
        }
    }

    #[tokio::test]
    async fn test_json_exporter() {
        let exporter = JsonExporter;
        let snapshot = create_test_snapshot();

        let result = exporter.export(&snapshot).await.unwrap();
        assert!(result.contains("\"ca\""));
        assert!(result.contains("\"total_operations\": 100"));
    }

    #[tokio::test]
    async fn test_prometheus_exporter() {
        let exporter = PrometheusExporter::new("trustchain");
        let snapshot = create_test_snapshot();

        let result = exporter.export(&snapshot).await.unwrap();
        assert!(result.contains("# HELP"));
        assert!(result.contains("# TYPE"));
        assert!(result.contains("trustchain_ca_total 100"));
    }

    #[tokio::test]
    async fn test_plaintext_exporter() {
        let exporter = PlainTextExporter;
        let snapshot = create_test_snapshot();

        let result = exporter.export(&snapshot).await.unwrap();
        assert!(result.contains("TrustChain Metrics Report"));
        assert!(result.contains("CA Component:"));
        assert!(result.contains("Success Rate: 95.00%"));
    }

    #[tokio::test]
    async fn test_csv_exporter() {
        let exporter = CsvExporter;
        let snapshot = create_test_snapshot();

        let result = exporter.export(&snapshot).await.unwrap();
        assert!(result.contains("component,metric,value,timestamp"));
        assert!(result.contains("ca,total_operations,100,"));
    }
}