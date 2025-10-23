//! Export functionality for native monitoring data

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;

use super::{MetricValue, MetricsStorage, SpanData};

/// Export format for metrics and traces
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    /// JSON format
    Json,
    /// Plain text format (Prometheus-compatible)
    Text,
    /// Binary format (custom compact format)
    Binary,
    /// OpenMetrics format
    OpenMetrics,
}

/// Metrics exporter
pub struct MetricsExporter {
    format: ExportFormat,
}

impl MetricsExporter {
    /// Create new metrics exporter
    pub fn new(format: ExportFormat) -> Self {
        Self { format }
    }

    /// Export metrics to string
    pub fn export_to_string(&self, storage: &MetricsStorage) -> String {
        match self.format {
            ExportFormat::Json => self.export_json(storage),
            ExportFormat::Text => self.export_text(storage),
            ExportFormat::Binary => self.export_binary_string(storage),
            ExportFormat::OpenMetrics => self.export_openmetrics(storage),
        }
    }

    /// Export metrics to writer
    pub fn export_to_writer<W: Write>(
        &self,
        storage: &MetricsStorage,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let content = self.export_to_string(storage);
        writer.write_all(content.as_bytes())
    }

    /// Export as JSON
    fn export_json(&self, storage: &MetricsStorage) -> String {
        let export_data = JsonExportData {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metrics: storage.current.clone(),
            metadata: storage
                .metadata
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            stats: JsonStorageStats {
                total_metrics: storage.stats.total_metrics,
                total_points: storage.stats.total_points,
                memory_bytes: storage.stats.memory_bytes,
            },
        };

        serde_json::to_string_pretty(&export_data).unwrap_or_default()
    }

    /// Export as text (Prometheus-compatible)
    fn export_text(&self, storage: &MetricsStorage) -> String {
        let mut output = String::new();

        for (name, value) in &storage.current {
            // Add metadata if available
            if let Some(meta) = storage.metadata.get(name) {
                output.push_str(&format!("# HELP {} {}\n", name, meta.description));
                output.push_str(&format!("# TYPE {} {}\n", name, metric_type_to_string(&meta.metric_type)));
            }

            match value {
                MetricValue::Counter(v) => {
                    output.push_str(&format!("{} {}\n", name, v));
                }
                MetricValue::Gauge(v) => {
                    output.push_str(&format!("{} {}\n", name, v));
                }
                MetricValue::Histogram(h) => {
                    for (upper_bound, count) in &h.buckets {
                        output.push_str(&format!(
                            "{}_bucket{{le=\"{}\"}} {}\n",
                            name, upper_bound, count
                        ));
                    }
                    output.push_str(&format!("{}_bucket{{le=\"+Inf\"}} {}\n", name, h.count));
                    output.push_str(&format!("{}_sum {}\n", name, h.sum));
                    output.push_str(&format!("{}_count {}\n", name, h.count));
                }
                MetricValue::Summary(s) => {
                    for (quantile, value) in &s.quantiles {
                        output.push_str(&format!(
                            "{}{{quantile=\"{}\"}} {}\n",
                            name, quantile, value
                        ));
                    }
                    output.push_str(&format!("{}_sum {}\n", name, s.sum));
                    output.push_str(&format!("{}_count {}\n", name, s.count));
                }
            }
        }

        output
    }

    /// Export as binary (placeholder)
    fn export_binary_string(&self, _storage: &MetricsStorage) -> String {
        // Would implement custom binary format for efficiency
        "Binary export not yet implemented".to_string()
    }

    /// Export as OpenMetrics format
    fn export_openmetrics(&self, storage: &MetricsStorage) -> String {
        let mut output = String::new();

        for (name, value) in &storage.current {
            if let Some(meta) = storage.metadata.get(name) {
                output.push_str(&format!("# TYPE {} {}\n", name, metric_type_to_string(&meta.metric_type)));
                output.push_str(&format!("# UNIT {} {}\n", name, meta.unit));
                output.push_str(&format!("# HELP {} {}\n", name, meta.description));
            }

            match value {
                MetricValue::Counter(v) => {
                    output.push_str(&format!("{}_total {}\n", name, v));
                }
                MetricValue::Gauge(v) => {
                    output.push_str(&format!("{} {}\n", name, v));
                }
                MetricValue::Histogram(h) => {
                    for (upper_bound, count) in &h.buckets {
                        output.push_str(&format!(
                            "{}_bucket{{le=\"{}\"}} {}\n",
                            name, upper_bound, count
                        ));
                    }
                    output.push_str(&format!("{}_bucket{{le=\"+Inf\"}} {}\n", name, h.count));
                    output.push_str(&format!("{}_sum {}\n", name, h.sum));
                    output.push_str(&format!("{}_count {}\n", name, h.count));
                }
                MetricValue::Summary(s) => {
                    for (quantile, value) in &s.quantiles {
                        output.push_str(&format!(
                            "{}{{quantile=\"{}\"}} {}\n",
                            name, quantile, value
                        ));
                    }
                    output.push_str(&format!("{}_sum {}\n", name, s.sum));
                    output.push_str(&format!("{}_count {}\n", name, s.count));
                }
            }
        }

        // Add EOF marker for OpenMetrics
        output.push_str("# EOF\n");
        output
    }
}

/// Trace exporter
pub struct TraceExporter {
    format: ExportFormat,
}

impl TraceExporter {
    /// Create new trace exporter
    pub fn new(format: ExportFormat) -> Self {
        Self { format }
    }

    /// Export traces to string
    pub fn export_to_string(&self, spans: &[SpanData]) -> String {
        match self.format {
            ExportFormat::Json => self.export_json(spans),
            ExportFormat::Text => self.export_text(spans),
            _ => "Unsupported format for traces".to_string(),
        }
    }

    /// Export traces as JSON
    fn export_json(&self, spans: &[SpanData]) -> String {
        serde_json::to_string_pretty(spans).unwrap_or_default()
    }

    /// Export traces as text
    fn export_text(&self, spans: &[SpanData]) -> String {
        let mut output = String::new();

        for span in spans {
            output.push_str(&format!("Trace: {} Span: {}\n", span.trace_id, span.span_id));
            output.push_str(&format!("  Operation: {}\n", span.operation));
            if let Some(parent) = &span.parent_span_id {
                output.push_str(&format!("  Parent: {}\n", parent));
            }
            if let Some(duration) = span.duration_ns {
                output.push_str(&format!("  Duration: {}ns\n", duration));
            }
            output.push_str(&format!("  Status: {:?}\n", span.status));

            if !span.tags.is_empty() {
                output.push_str("  Tags:\n");
                for (key, value) in &span.tags {
                    output.push_str(&format!("    {}: {}\n", key, value));
                }
            }

            if !span.events.is_empty() {
                output.push_str("  Events:\n");
                for event in &span.events {
                    output.push_str(&format!("    - {}\n", event.name));
                }
            }

            output.push('\n');
        }

        output
    }
}

/// JSON export data structure
#[derive(Serialize, Deserialize)]
struct JsonExportData {
    timestamp: u64,
    metrics: HashMap<String, MetricValue>,
    metadata: HashMap<String, super::MetricMetadata>,
    stats: JsonStorageStats,
}

/// JSON storage stats
#[derive(Serialize, Deserialize)]
struct JsonStorageStats {
    total_metrics: u64,
    total_points: u64,
    memory_bytes: u64,
}

/// Convert metric type to string
fn metric_type_to_string(metric_type: &super::MetricType) -> &'static str {
    match metric_type {
        super::MetricType::Counter => "counter",
        super::MetricType::Gauge => "gauge",
        super::MetricType::Histogram => "histogram",
        super::MetricType::Summary => "summary",
    }
}

/// Remote export client for sending metrics to external systems
pub struct RemoteExporter {
    endpoint: String,
    format: ExportFormat,
    batch_size: usize,
    timeout: std::time::Duration,
}

impl RemoteExporter {
    /// Create new remote exporter
    pub fn new(endpoint: String, format: ExportFormat) -> Self {
        Self {
            endpoint,
            format,
            batch_size: 1000,
            timeout: std::time::Duration::from_secs(30),
        }
    }

    /// Export metrics to remote endpoint
    pub async fn export_metrics(&self, storage: &MetricsStorage) -> Result<(), ExportError> {
        let exporter = MetricsExporter::new(self.format);
        let data = exporter.export_to_string(storage);

        // In production, would use actual HTTP client
        self.send_data(data).await
    }

    /// Export traces to remote endpoint
    pub async fn export_traces(&self, spans: &[SpanData]) -> Result<(), ExportError> {
        let exporter = TraceExporter::new(self.format);
        let data = exporter.export_to_string(spans);

        self.send_data(data).await
    }

    /// Send data to remote endpoint
    async fn send_data(&self, _data: String) -> Result<(), ExportError> {
        // Placeholder - would implement actual HTTP/gRPC client
        Ok(())
    }
}

/// Export error
#[derive(Debug)]
pub enum ExportError {
    NetworkError(String),
    SerializationError(String),
    Timeout,
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Self::Timeout => write!(f, "Export timeout"),
        }
    }
}

impl std::error::Error for ExportError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::native::{HistogramData, SummaryData};

    #[test]
    fn test_metrics_export_text() {
        let mut storage = MetricsStorage {
            time_series: std::collections::BTreeMap::new(),
            current: HashMap::new(),
            metadata: HashMap::new(),
            stats: super::super::StorageStats::default(),
        };

        storage.current.insert(
            "test_counter".to_string(),
            MetricValue::Counter(42),
        );
        storage.current.insert(
            "test_gauge".to_string(),
            MetricValue::Gauge(3.14),
        );

        let exporter = MetricsExporter::new(ExportFormat::Text);
        let output = exporter.export_to_string(&storage);

        assert!(output.contains("test_counter 42"));
        assert!(output.contains("test_gauge 3.14"));
    }

    #[test]
    fn test_metrics_export_json() {
        let mut storage = MetricsStorage {
            time_series: std::collections::BTreeMap::new(),
            current: HashMap::new(),
            metadata: HashMap::new(),
            stats: super::super::StorageStats::default(),
        };

        storage.current.insert(
            "test_metric".to_string(),
            MetricValue::Counter(100),
        );

        let exporter = MetricsExporter::new(ExportFormat::Json);
        let output = exporter.export_to_string(&storage);

        assert!(output.contains("\"test_metric\""));
        assert!(output.contains("100"));
    }

    #[test]
    fn test_trace_export() {
        let span = SpanData {
            trace_id: "abc123".to_string(),
            span_id: "def456".to_string(),
            parent_span_id: None,
            operation: "test_op".to_string(),
            start_time: 1000,
            end_time: Some(2000),
            duration_ns: Some(1000),
            tags: HashMap::new(),
            events: Vec::new(),
            status: super::super::trace::SpanStatus::Ok,
        };

        let exporter = TraceExporter::new(ExportFormat::Text);
        let output = exporter.export_to_string(&[span]);

        assert!(output.contains("abc123"));
        assert!(output.contains("test_op"));
        assert!(output.contains("1000ns"));
    }
}