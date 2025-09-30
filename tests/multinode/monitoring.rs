// Monitoring and Observability Module
// Real-time monitoring and metrics collection for multi-node tests

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Monitoring system for multi-node tests
pub struct MonitoringSystem {
    metrics_store: Arc<RwLock<MetricsStore>>,
    alert_manager: Arc<AlertManager>,
    dashboards: Arc<RwLock<HashMap<String, Dashboard>>>,
}

impl MonitoringSystem {
    pub fn new() -> Self {
        Self {
            metrics_store: Arc::new(RwLock::new(MetricsStore::new())),
            alert_manager: Arc::new(AlertManager::new()),
            dashboards: Arc::new(RwLock::new(Self::create_default_dashboards())),
        }
    }

    fn create_default_dashboards() -> HashMap<String, Dashboard> {
        let mut dashboards = HashMap::new();

        dashboards.insert(
            "overview".to_string(),
            Dashboard::new("System Overview", vec![
                Panel::new("Node Health", PanelType::NodeHealth),
                Panel::new("Transaction Rate", PanelType::TransactionRate),
                Panel::new("Latency Distribution", PanelType::LatencyDistribution),
                Panel::new("Resource Usage", PanelType::ResourceUsage),
            ]),
        );

        dashboards.insert(
            "performance".to_string(),
            Dashboard::new("Performance Metrics", vec![
                Panel::new("Throughput", PanelType::Throughput),
                Panel::new("Latency Percentiles", PanelType::LatencyPercentiles),
                Panel::new("Error Rate", PanelType::ErrorRate),
                Panel::new("Connection Pool", PanelType::ConnectionPool),
            ]),
        );

        dashboards.insert(
            "security".to_string(),
            Dashboard::new("Security Monitoring", vec![
                Panel::new("Attack Detection", PanelType::AttackDetection),
                Panel::new("Certificate Status", PanelType::CertificateStatus),
                Panel::new("Byzantine Nodes", PanelType::ByzantineNodes),
                Panel::new("Security Events", PanelType::SecurityEvents),
            ]),
        );

        dashboards
    }

    /// Record a metric
    pub async fn record_metric(&self, name: &str, value: f64, tags: HashMap<String, String>) {
        let mut store = self.metrics_store.write().await;
        store.record(name, value, tags);
    }

    /// Record connection count
    pub async fn record_connections(&self, count: usize) {
        self.record_metric(
            "connections.active",
            count as f64,
            HashMap::new(),
        ).await;
    }

    /// Record transaction
    pub async fn record_transaction(&self, latency: Duration, success: bool) {
        let mut tags = HashMap::new();
        tags.insert("status".to_string(), if success { "success" } else { "failure" }.to_string());

        self.record_metric(
            "transaction.latency",
            latency.as_millis() as f64,
            tags,
        ).await;
    }

    /// Check alerts
    pub async fn check_alerts(&self) -> Vec<Alert> {
        let metrics = self.metrics_store.read().await;
        self.alert_manager.evaluate(&metrics)
    }

    /// Get dashboard
    pub async fn get_dashboard(&self, name: &str) -> Option<DashboardView> {
        let dashboards = self.dashboards.read().await;
        let dashboard = dashboards.get(name)?;

        let metrics = self.metrics_store.read().await;
        Some(dashboard.render(&metrics))
    }

    /// Export metrics in Prometheus format
    pub async fn export_prometheus(&self) -> String {
        let metrics = self.metrics_store.read().await;
        metrics.export_prometheus()
    }
}

/// Metrics storage
struct MetricsStore {
    metrics: HashMap<String, TimeSeries>,
    counters: HashMap<String, u64>,
    gauges: HashMap<String, f64>,
    histograms: HashMap<String, Histogram>,
}

impl MetricsStore {
    fn new() -> Self {
        Self {
            metrics: HashMap::new(),
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
        }
    }

    fn record(&mut self, name: &str, value: f64, tags: HashMap<String, String>) {
        let key = format_metric_key(name, &tags);

        // Add to time series
        self.metrics
            .entry(key.clone())
            .or_insert_with(|| TimeSeries::new(1000))
            .add(value);

        // Update counter/gauge/histogram based on metric type
        if name.contains("count") || name.contains("total") {
            *self.counters.entry(key.clone()).or_insert(0) += value as u64;
        } else if name.contains("latency") || name.contains("duration") {
            self.histograms
                .entry(key.clone())
                .or_insert_with(Histogram::new)
                .record(value);
        } else {
            self.gauges.insert(key, value);
        }
    }

    fn export_prometheus(&self) -> String {
        let mut output = String::new();

        // Export counters
        for (key, value) in &self.counters {
            output.push_str(&format!("{} {}\n", key, value));
        }

        // Export gauges
        for (key, value) in &self.gauges {
            output.push_str(&format!("{} {}\n", key, value));
        }

        // Export histograms
        for (key, hist) in &self.histograms {
            output.push_str(&format!("{}_count {}\n", key, hist.count));
            output.push_str(&format!("{}_sum {}\n", key, hist.sum));
            output.push_str(&format!("{}_p50 {}\n", key, hist.percentile(0.5)));
            output.push_str(&format!("{}_p99 {}\n", key, hist.percentile(0.99)));
        }

        output
    }
}

/// Time series data
struct TimeSeries {
    max_size: usize,
    data: VecDeque<(u64, f64)>,
}

impl TimeSeries {
    fn new(max_size: usize) -> Self {
        Self {
            max_size,
            data: VecDeque::with_capacity(max_size),
        }
    }

    fn add(&mut self, value: f64) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.data.push_back((timestamp, value));

        if self.data.len() > self.max_size {
            self.data.pop_front();
        }
    }

    fn recent(&self, duration: Duration) -> Vec<(u64, f64)> {
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - duration.as_secs();

        self.data
            .iter()
            .filter(|(ts, _)| *ts >= cutoff)
            .cloned()
            .collect()
    }
}

/// Histogram for latency tracking
struct Histogram {
    values: Vec<f64>,
    count: u64,
    sum: f64,
}

impl Histogram {
    fn new() -> Self {
        Self {
            values: Vec::new(),
            count: 0,
            sum: 0.0,
        }
    }

    fn record(&mut self, value: f64) {
        self.values.push(value);
        self.count += 1;
        self.sum += value;

        // Keep only last 10000 values
        if self.values.len() > 10000 {
            let removed = self.values.remove(0);
            self.sum -= removed;
        }
    }

    fn percentile(&self, p: f64) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }

        let mut sorted = self.values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let index = ((p * sorted.len() as f64) as usize).min(sorted.len() - 1);
        sorted[index]
    }
}

/// Alert management
struct AlertManager {
    rules: Vec<AlertRule>,
}

impl AlertManager {
    fn new() -> Self {
        Self {
            rules: Self::default_rules(),
        }
    }

    fn default_rules() -> Vec<AlertRule> {
        vec![
            AlertRule {
                name: "High Error Rate".to_string(),
                condition: AlertCondition::ThresholdExceeded {
                    metric: "error_rate".to_string(),
                    threshold: 0.05,
                },
                severity: AlertSeverity::Warning,
            },
            AlertRule {
                name: "High Latency".to_string(),
                condition: AlertCondition::ThresholdExceeded {
                    metric: "latency_p99".to_string(),
                    threshold: 1000.0,
                },
                severity: AlertSeverity::Critical,
            },
            AlertRule {
                name: "Node Failures".to_string(),
                condition: AlertCondition::ThresholdExceeded {
                    metric: "failed_nodes".to_string(),
                    threshold: 2.0,
                },
                severity: AlertSeverity::Critical,
            },
            AlertRule {
                name: "Memory Pressure".to_string(),
                condition: AlertCondition::ThresholdExceeded {
                    metric: "memory_usage".to_string(),
                    threshold: 90.0,
                },
                severity: AlertSeverity::Warning,
            },
        ]
    }

    fn evaluate(&self, metrics: &MetricsStore) -> Vec<Alert> {
        let mut alerts = Vec::new();

        for rule in &self.rules {
            if let Some(alert) = rule.evaluate(metrics) {
                alerts.push(alert);
            }
        }

        alerts
    }
}

/// Alert rule definition
struct AlertRule {
    name: String,
    condition: AlertCondition,
    severity: AlertSeverity,
}

impl AlertRule {
    fn evaluate(&self, metrics: &MetricsStore) -> Option<Alert> {
        match &self.condition {
            AlertCondition::ThresholdExceeded { metric, threshold } => {
                if let Some(value) = metrics.gauges.get(metric) {
                    if value > threshold {
                        return Some(Alert {
                            name: self.name.clone(),
                            severity: self.severity.clone(),
                            message: format!("{} is {} (threshold: {})", metric, value, threshold),
                            timestamp: Instant::now(),
                        });
                    }
                }
            }
        }

        None
    }
}

/// Alert condition types
enum AlertCondition {
    ThresholdExceeded {
        metric: String,
        threshold: f64,
    },
}

/// Alert severity levels
#[derive(Debug, Clone)]
enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Active alert
#[derive(Debug)]
pub struct Alert {
    pub name: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub timestamp: Instant,
}

/// Dashboard definition
struct Dashboard {
    name: String,
    panels: Vec<Panel>,
}

impl Dashboard {
    fn new(name: &str, panels: Vec<Panel>) -> Self {
        Self {
            name: name.to_string(),
            panels,
        }
    }

    fn render(&self, metrics: &MetricsStore) -> DashboardView {
        DashboardView {
            name: self.name.clone(),
            panels: self.panels.iter().map(|p| p.render(metrics)).collect(),
        }
    }
}

/// Dashboard panel
struct Panel {
    title: String,
    panel_type: PanelType,
}

impl Panel {
    fn new(title: &str, panel_type: PanelType) -> Self {
        Self {
            title: title.to_string(),
            panel_type,
        }
    }

    fn render(&self, metrics: &MetricsStore) -> PanelView {
        let data = match &self.panel_type {
            PanelType::NodeHealth => self.render_node_health(metrics),
            PanelType::TransactionRate => self.render_transaction_rate(metrics),
            PanelType::LatencyDistribution => self.render_latency_distribution(metrics),
            PanelType::ResourceUsage => self.render_resource_usage(metrics),
            PanelType::Throughput => self.render_throughput(metrics),
            PanelType::LatencyPercentiles => self.render_latency_percentiles(metrics),
            PanelType::ErrorRate => self.render_error_rate(metrics),
            PanelType::ConnectionPool => self.render_connection_pool(metrics),
            PanelType::AttackDetection => self.render_attack_detection(metrics),
            PanelType::CertificateStatus => self.render_certificate_status(metrics),
            PanelType::ByzantineNodes => self.render_byzantine_nodes(metrics),
            PanelType::SecurityEvents => self.render_security_events(metrics),
        };

        PanelView {
            title: self.title.clone(),
            data,
        }
    }

    fn render_node_health(&self, metrics: &MetricsStore) -> PanelData {
        let healthy = metrics.gauges.get("nodes.healthy").copied().unwrap_or(0.0) as u32;
        let failed = metrics.gauges.get("nodes.failed").copied().unwrap_or(0.0) as u32;
        let total = healthy + failed;

        PanelData::PieChart {
            values: vec![
                ("Healthy".to_string(), healthy as f64),
                ("Failed".to_string(), failed as f64),
            ],
        }
    }

    fn render_transaction_rate(&self, metrics: &MetricsStore) -> PanelData {
        let rate = metrics.gauges.get("transaction.rate").copied().unwrap_or(0.0);

        PanelData::Gauge {
            value: rate,
            max: 10000.0,
            unit: "tx/s".to_string(),
        }
    }

    fn render_latency_distribution(&self, metrics: &MetricsStore) -> PanelData {
        if let Some(hist) = metrics.histograms.get("transaction.latency") {
            PanelData::Histogram {
                buckets: vec![
                    (0.0, 10.0),
                    (10.0, 50.0),
                    (50.0, 100.0),
                    (100.0, 500.0),
                    (500.0, 1000.0),
                ],
                values: vec![
                    hist.percentile(0.1),
                    hist.percentile(0.25),
                    hist.percentile(0.5),
                    hist.percentile(0.75),
                    hist.percentile(0.9),
                ],
            }
        } else {
            PanelData::NoData
        }
    }

    fn render_resource_usage(&self, metrics: &MetricsStore) -> PanelData {
        let cpu = metrics.gauges.get("resource.cpu").copied().unwrap_or(0.0);
        let memory = metrics.gauges.get("resource.memory").copied().unwrap_or(0.0);
        let network = metrics.gauges.get("resource.network").copied().unwrap_or(0.0);

        PanelData::BarChart {
            categories: vec![
                "CPU".to_string(),
                "Memory".to_string(),
                "Network".to_string(),
            ],
            values: vec![cpu, memory, network],
        }
    }

    fn render_throughput(&self, _metrics: &MetricsStore) -> PanelData {
        PanelData::NoData
    }

    fn render_latency_percentiles(&self, _metrics: &MetricsStore) -> PanelData {
        PanelData::NoData
    }

    fn render_error_rate(&self, _metrics: &MetricsStore) -> PanelData {
        PanelData::NoData
    }

    fn render_connection_pool(&self, _metrics: &MetricsStore) -> PanelData {
        PanelData::NoData
    }

    fn render_attack_detection(&self, _metrics: &MetricsStore) -> PanelData {
        PanelData::NoData
    }

    fn render_certificate_status(&self, _metrics: &MetricsStore) -> PanelData {
        PanelData::NoData
    }

    fn render_byzantine_nodes(&self, _metrics: &MetricsStore) -> PanelData {
        PanelData::NoData
    }

    fn render_security_events(&self, _metrics: &MetricsStore) -> PanelData {
        PanelData::NoData
    }
}

/// Panel types
enum PanelType {
    NodeHealth,
    TransactionRate,
    LatencyDistribution,
    ResourceUsage,
    Throughput,
    LatencyPercentiles,
    ErrorRate,
    ConnectionPool,
    AttackDetection,
    CertificateStatus,
    ByzantineNodes,
    SecurityEvents,
}

/// Dashboard view for rendering
#[derive(Debug, Serialize)]
pub struct DashboardView {
    pub name: String,
    pub panels: Vec<PanelView>,
}

/// Panel view for rendering
#[derive(Debug, Serialize)]
pub struct PanelView {
    pub title: String,
    pub data: PanelData,
}

/// Panel data types
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum PanelData {
    Gauge {
        value: f64,
        max: f64,
        unit: String,
    },
    PieChart {
        values: Vec<(String, f64)>,
    },
    BarChart {
        categories: Vec<String>,
        values: Vec<f64>,
    },
    Histogram {
        buckets: Vec<(f64, f64)>,
        values: Vec<f64>,
    },
    TimeSeries {
        points: Vec<(u64, f64)>,
    },
    NoData,
}

/// Format metric key with tags
fn format_metric_key(name: &str, tags: &HashMap<String, String>) -> String {
    if tags.is_empty() {
        name.to_string()
    } else {
        let tag_str = tags
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(",");
        format!("{}_{{{}}}", name, tag_str)
    }
}

/// Real-time monitoring reporter
pub struct MonitoringReporter {
    system: Arc<MonitoringSystem>,
}

impl MonitoringReporter {
    pub fn new(system: Arc<MonitoringSystem>) -> Self {
        Self { system }
    }

    /// Start reporting loop
    pub async fn start_reporting(self, interval: Duration) {
        let mut ticker = tokio::time::interval(interval);

        loop {
            ticker.tick().await;

            // Check alerts
            let alerts = self.system.check_alerts().await;
            for alert in alerts {
                match alert.severity {
                    AlertSeverity::Critical => error!("ALERT: {}", alert.message),
                    AlertSeverity::Warning => warn!("ALERT: {}", alert.message),
                    AlertSeverity::Info => info!("ALERT: {}", alert.message),
                }
            }

            // Log key metrics
            if let Some(dashboard) = self.system.get_dashboard("overview").await {
                debug!("Dashboard update: {:?}", dashboard);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitoring_system() {
        let system = MonitoringSystem::new();

        // Record some metrics
        system.record_connections(100).await;
        system.record_transaction(Duration::from_millis(50), true).await;

        // Check dashboard
        let dashboard = system.get_dashboard("overview").await;
        assert!(dashboard.is_some());
    }

    #[test]
    fn test_histogram() {
        let mut hist = Histogram::new();

        for i in 0..100 {
            hist.record(i as f64);
        }

        assert_eq!(hist.count, 100);
        assert!(hist.percentile(0.5) > 45.0 && hist.percentile(0.5) < 55.0);
        assert!(hist.percentile(0.99) > 95.0);
    }
}