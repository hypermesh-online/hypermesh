//! HyperMesh Performance Monitoring Dashboard
//!
//! Comprehensive performance monitoring dashboard split into logical modules.

pub mod config;
pub mod metrics;
pub mod analysis;
pub mod alerts;
pub mod web_server;

use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::collections::{HashMap, BTreeMap, VecDeque};
use serde::{Deserialize, Serialize};
use tracing::info;
use nexus_shared::{NodeId, Timestamp, Result};

pub use config::{DashboardConfig, PerformanceThresholds};
pub use metrics::{
    MetricsStorage, AggregatedMetrics, PerformanceMetricsCollector,
    RealTimeMetricsAggregator,
};
pub use analysis::{AnalysisResults, PerformanceAnalyzer, PerformanceBenchmarks};
pub use alerts::{ActiveAlert, AlertManager};
pub use web_server::DashboardWebServer;

use crate::health::HealthMonitor;
use crate::networking::NetworkManager;
use crate::consensus_orchestrator::ConsensusContainerOrchestrator;

/// Performance monitoring dashboard
pub struct PerformanceDashboard {
    node_id: NodeId,
    config: DashboardConfig,
    health_monitor: Arc<HealthMonitor>,
    network_manager: Arc<NetworkManager>,
    orchestrator: Arc<tokio::sync::Mutex<ConsensusContainerOrchestrator>>,
    metrics_collector: Arc<PerformanceMetricsCollector>,
    metrics_aggregator: Arc<RealTimeMetricsAggregator>,
    performance_analyzer: Arc<PerformanceAnalyzer>,
    alert_manager: Arc<AlertManager>,
    web_server: Arc<DashboardWebServer>,
    metrics_storage: Arc<StdRwLock<MetricsStorage>>,
    benchmarks: Arc<PerformanceBenchmarks>,
}

impl PerformanceDashboard {
    /// Create a new performance dashboard
    pub async fn new(
        node_id: NodeId,
        config: DashboardConfig,
        health_monitor: Arc<HealthMonitor>,
        network_manager: Arc<NetworkManager>,
        orchestrator: Arc<tokio::sync::Mutex<ConsensusContainerOrchestrator>>,
    ) -> Result<Self> {
        info!("Initializing HyperMesh performance dashboard for node {}", node_id);

        let metrics_storage = Arc::new(StdRwLock::new(MetricsStorage {
            time_series: BTreeMap::new(),
            aggregated: HashMap::new(),
            metadata: HashMap::new(),
            storage_stats: metrics::StorageStats::default(),
        }));

        let metrics_collector = Arc::new(
            PerformanceMetricsCollector::new(&config.metrics_collection, Arc::clone(&metrics_storage))
        );

        let metrics_aggregator = Arc::new(
            RealTimeMetricsAggregator::new(Arc::clone(&metrics_storage))
        );

        let performance_analyzer = Arc::new(
            PerformanceAnalyzer::new(&config.performance_analysis, Arc::clone(&metrics_storage))
        );

        let alert_manager = Arc::new(
            AlertManager::new(&config.alerting)
        );

        let web_server = Arc::new(
            DashboardWebServer::new(&config.web_server, Arc::clone(&metrics_storage))
        );

        let benchmarks = Arc::new(
            PerformanceBenchmarks::new(&config.performance_analysis.thresholds)
        );

        let dashboard = Self {
            node_id,
            config,
            health_monitor,
            network_manager,
            orchestrator,
            metrics_collector,
            metrics_aggregator,
            performance_analyzer,
            alert_manager,
            web_server,
            metrics_storage,
            benchmarks,
        };

        dashboard.start_services().await?;
        info!("Performance dashboard initialized successfully");
        Ok(dashboard)
    }

    async fn start_services(&self) -> Result<()> {
        self.metrics_collector.start_collection(
            Arc::clone(&self.health_monitor),
            Arc::clone(&self.network_manager),
            Arc::clone(&self.orchestrator),
        ).await?;

        self.metrics_aggregator.start_aggregation().await?;
        self.performance_analyzer.start_analysis().await?;
        self.alert_manager.start_monitoring().await?;
        self.web_server.start_server().await?;

        Ok(())
    }

    pub async fn get_dashboard_data(&self) -> DashboardData {
        let metrics = self.metrics_aggregator.get_aggregated_metrics().await;
        let analysis = self.performance_analyzer.get_analysis_results().await;
        let alerts = self.alert_manager.get_active_alerts().await;

        DashboardData {
            metrics,
            analysis,
            alerts,
            timestamp: Timestamp::now(),
        }
    }

    pub async fn export_metrics(&self, format: ExportFormat) -> Result<String> {
        let metrics = self.metrics_storage.read().unwrap();

        match format {
            ExportFormat::Native => self.export_native_metrics(&metrics),
            ExportFormat::Json => self.export_json_metrics(&metrics),
            ExportFormat::Text => self.export_text_metrics(&metrics),
        }
    }

    fn export_native_metrics(&self, metrics: &MetricsStorage) -> Result<String> {
        let mut output = String::new();

        for (metric_name, points) in &metrics.time_series {
            if let Some(latest_point) = points.back() {
                if let Some(metadata) = metrics.metadata.get(metric_name) {
                    output.push_str(&format!(
                        "# TYPE {} {}\n",
                        metric_name,
                        match metadata.metric_type {
                            metrics::MetricDataType::Counter => "counter",
                            metrics::MetricDataType::Gauge => "gauge",
                            metrics::MetricDataType::Histogram => "histogram",
                            metrics::MetricDataType::Summary => "summary",
                        }
                    ));
                    output.push_str(&format!("# HELP {} {}\n", metric_name, metadata.description));
                }

                output.push_str(&format!(
                    "{} {}\n",
                    metric_name,
                    latest_point.value
                ));
            }
        }

        Ok(output)
    }

    fn export_json_metrics(&self, metrics: &MetricsStorage) -> Result<String> {
        serde_json::to_string_pretty(metrics)
            .map_err(|e| crate::RuntimeError::SerializationError {
                message: format!("JSON export failed: {}", e)
            }.into())
    }

    fn export_text_metrics(&self, metrics: &MetricsStorage) -> Result<String> {
        let mut output = String::new();

        for (metric_name, points) in &metrics.time_series {
            for point in points {
                output.push_str(&format!(
                    "{}[source={}] = {} @ {}\n",
                    metric_name,
                    point.source,
                    point.value,
                    point.timestamp.as_secs()
                ));
            }
        }

        Ok(output)
    }
}

/// Export formats for metrics
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExportFormat {
    Native,
    Json,
    Text,
}

/// Dashboard data for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub metrics: AggregatedMetrics,
    pub analysis: AnalysisResults,
    pub alerts: Vec<ActiveAlert>,
    pub timestamp: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_config_defaults() {
        let config = DashboardConfig::default();
        assert_eq!(config.web_server.port, 3000);
        assert!(config.performance_analysis.enable_realtime_analysis);
        assert_eq!(config.performance_analysis.thresholds.container_startup_ms, 100.0);
        assert_eq!(config.performance_analysis.thresholds.p2p_connection_ms, 5.0);
        assert_eq!(config.performance_analysis.thresholds.consensus_overhead_ms, 50.0);
    }

    #[test]
    fn test_performance_thresholds() {
        let thresholds = PerformanceThresholds::default();
        assert_eq!(thresholds.container_startup_ms, 100.0);
        assert_eq!(thresholds.p2p_connection_ms, 5.0);
        assert_eq!(thresholds.consensus_overhead_ms, 50.0);
        assert_eq!(thresholds.network_setup_ms, 10.0);
    }

    #[test]
    fn test_metrics_storage() {
        let mut storage = MetricsStorage {
            time_series: BTreeMap::new(),
            aggregated: HashMap::new(),
            metadata: HashMap::new(),
            storage_stats: metrics::StorageStats::default(),
        };

        let point = metrics::MetricPoint {
            timestamp: Timestamp::now(),
            value: 42.0,
            labels: HashMap::new(),
            source: "test".to_string(),
        };

        storage.time_series.entry("test_metric".to_string())
            .or_insert_with(VecDeque::new)
            .push_back(point);

        assert_eq!(storage.time_series.len(), 1);
    }
}
