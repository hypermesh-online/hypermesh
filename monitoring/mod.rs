//! HyperMesh Monitoring Module
//!
//! Provides comprehensive monitoring capabilities for the HyperMesh system
//! including metrics collection, distributed tracing, and performance dashboards.
//!
//! This module uses native monitoring implementations instead of external
//! dependencies like Prometheus or OpenTelemetry, providing:
//!
//! - **Native Metrics**: Built-in metrics collection using eBPF and system APIs
//! - **Native Tracing**: Distributed tracing without external dependencies
//! - **Performance Dashboards**: Real-time monitoring dashboards
//! - **Export Capabilities**: Export to various formats for compatibility

pub mod dashboards;
pub mod native;

// Re-export commonly used types
pub use native::{
    NativeMetricsCollector,
    NativeTracer,
    MetricsStorage,
    MetricPoint,
    MetricValue,
    SpanData,
    ExportFormat,
};

pub use native::metrics::{
    Counter,
    Gauge,
    Histogram,
    Summary,
    MetricRegistry,
    METRICS,
};

pub use native::trace::{
    Span,
    SpanContext,
    TraceId,
    SpanId,
    Tracer,
    ContextPropagator,
};

pub use native::export::{
    MetricsExporter,
    TraceExporter,
    RemoteExporter,
};

pub use native::collector::{
    SystemMetricsCollector,
    HyperMeshCollector,
};

/// Initialize the monitoring system
pub async fn initialize() -> Result<MonitoringSystem, Box<dyn std::error::Error>> {
    let system = MonitoringSystem::new();
    system.start().await?;
    Ok(system)
}

/// Main monitoring system coordinator
pub struct MonitoringSystem {
    /// Metrics collector
    pub metrics_collector: NativeMetricsCollector,

    /// Distributed tracer
    pub tracer: NativeTracer,

    /// System metrics collector
    pub system_collector: SystemMetricsCollector,

    /// HyperMesh-specific collector
    pub hypermesh_collector: HyperMeshCollector,

    /// Metrics registry
    pub registry: &'static MetricRegistry,
}

impl MonitoringSystem {
    /// Create new monitoring system
    pub fn new() -> Self {
        Self {
            metrics_collector: NativeMetricsCollector::new(std::time::Duration::from_secs(5)),
            tracer: NativeTracer::new(10000),
            system_collector: SystemMetricsCollector::new(),
            hypermesh_collector: HyperMeshCollector::new(),
            registry: &METRICS,
        }
    }

    /// Start the monitoring system
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Start metrics collection would happen here
        // In production, this would spawn background tasks
        Ok(())
    }

    /// Stop the monitoring system
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Stop all background tasks
        Ok(())
    }

    /// Export all metrics
    pub fn export_metrics(&self, format: ExportFormat) -> String {
        match format {
            ExportFormat::Native | ExportFormat::Text => self.registry.export_text(),
            ExportFormat::Json => {
                // Convert to JSON format
                serde_json::to_string_pretty(&self.get_current_metrics()).unwrap_or_default()
            }
        }
    }

    /// Get current metrics as a map
    pub fn get_current_metrics(&self) -> std::collections::HashMap<String, f64> {
        let mut metrics = std::collections::HashMap::new();

        // This would collect from all registered metrics
        // Simplified for now
        metrics.insert("system.uptime".to_string(), 0.0);

        metrics
    }

    /// Create a new span for tracing
    pub fn start_span(&self, operation: &str) -> Span {
        self.tracer.start_span(operation)
    }

    /// Record a completed span
    pub fn record_span(&self, span: &Span) {
        self.tracer.record_span(span);
    }

    /// Export trace data
    pub fn export_traces(&self) -> Vec<SpanData> {
        self.tracer.export()
    }
}

/// Global monitoring instance
lazy_static::lazy_static! {
    pub static ref MONITORING: tokio::sync::RwLock<Option<MonitoringSystem>> =
        tokio::sync::RwLock::new(None);
}

/// Initialize global monitoring
pub async fn init() -> Result<(), Box<dyn std::error::Error>> {
    let system = MonitoringSystem::new();
    system.start().await?;
    *MONITORING.write().await = Some(system);
    Ok(())
}

/// Get reference to global monitoring system
pub async fn get() -> Option<tokio::sync::RwLockReadGuard<'static, Option<MonitoringSystem>>> {
    Some(MONITORING.read().await)
}

// Helper macros for easy metric recording

/// Record a counter metric
#[macro_export]
macro_rules! record_counter {
    ($name:expr, $value:expr) => {
        $crate::monitoring::METRICS.counter($name).inc_by($value)
    };
    ($name:expr) => {
        $crate::monitoring::METRICS.counter($name).inc()
    };
}

/// Record a gauge metric
#[macro_export]
macro_rules! record_gauge {
    ($name:expr, $value:expr) => {
        $crate::monitoring::METRICS.gauge($name).set($value)
    };
}

/// Record a histogram observation
#[macro_export]
macro_rules! record_histogram {
    ($name:expr, $value:expr) => {
        $crate::monitoring::METRICS.histogram($name).observe($value)
    };
}

/// Time a function and record to histogram
#[macro_export]
macro_rules! time_operation {
    ($name:expr, $op:expr) => {
        $crate::monitoring::METRICS.histogram($name).time(|| $op)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitoring_system() {
        let system = MonitoringSystem::new();
        assert!(system.start().await.is_ok());

        // Record some metrics
        system.registry.counter("test_counter").inc();
        system.registry.gauge("test_gauge").set(42.0);

        // Export metrics
        let text_export = system.export_metrics(ExportFormat::Text);
        assert!(text_export.contains("test_counter"));
        assert!(text_export.contains("test_gauge"));
    }

    #[test]
    fn test_metric_macros() {
        record_counter!("test_macro_counter");
        record_counter!("test_macro_counter_by", 5);
        record_gauge!("test_macro_gauge", 3.14);
        record_histogram!("test_macro_histogram", 100.0);

        let result = time_operation!("test_macro_timer", {
            std::thread::sleep(std::time::Duration::from_millis(10));
            42
        });
        assert_eq!(result, 42);
    }
}