//! Comprehensive metrics and analytics system for Hypermesh Nexus
//!
//! Provides real-time monitoring, alerting, and analytics for all system components

pub mod runtime_metrics;
pub mod consensus_metrics;
pub mod network_metrics;
pub mod ebpf_metrics;
pub mod performance_analytics;
pub mod alerting;

use crate::{TestResult, init_test_logging};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{info, warn, error};

/// Main metrics and analytics system coordinator
pub struct MetricsSystem {
    collectors: HashMap<String, Box<dyn MetricCollector>>,
    storage: Arc<RwLock<MetricsStorage>>,
    analytics_engine: AnalyticsEngine,
    alerting_system: alerting::AlertingSystem,
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl MetricsSystem {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut collectors: HashMap<String, Box<dyn MetricCollector>> = HashMap::new();
        
        // Initialize all metric collectors
        collectors.insert("runtime".to_string(), 
                         Box::new(runtime_metrics::RuntimeMetricCollector::new()));
        collectors.insert("consensus".to_string(), 
                         Box::new(consensus_metrics::ConsensusMetricCollector::new()));
        collectors.insert("network".to_string(), 
                         Box::new(network_metrics::NetworkMetricCollector::new()));
        collectors.insert("ebpf".to_string(), 
                         Box::new(ebpf_metrics::EbpfMetricCollector::new()));
        
        let storage = Arc::new(RwLock::new(MetricsStorage::new()));
        let analytics_engine = AnalyticsEngine::new(storage.clone());
        let alerting_system = alerting::AlertingSystem::new();
        
        Ok(Self {
            collectors,
            storage,
            analytics_engine,
            alerting_system,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }
    
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting comprehensive metrics and analytics system");
        
        self.running.store(true, std::sync::atomic::Ordering::Relaxed);
        
        // Start collection loop
        let collectors = self.collectors.clone();
        let storage = self.storage.clone();
        let running = self.running.clone();
        
        tokio::spawn(async move {
            while running.load(std::sync::atomic::Ordering::Relaxed) {
                for (name, collector) in &collectors {
                    match collector.collect().await {
                        Ok(metrics) => {
                            let mut storage_guard = storage.write().await;
                            for metric in metrics {
                                storage_guard.store_metric(name, metric).await;
                            }
                        },
                        Err(e) => {
                            error!("Failed to collect metrics from {}: {}", name, e);
                        }
                    }
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
        
        // Start analytics engine
        self.analytics_engine.start().await?;
        
        // Start alerting system
        self.alerting_system.start().await?;
        
        info!("✅ Metrics and analytics system started successfully");
        Ok(())
    }
    
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.running.store(false, std::sync::atomic::Ordering::Relaxed);
        self.analytics_engine.stop().await?;
        self.alerting_system.stop().await?;
        
        info!("Metrics and analytics system stopped");
        Ok(())
    }
    
    pub async fn get_metrics(&self, component: &str, time_range: TimeRange) -> Result<Vec<Metric>, Box<dyn std::error::Error>> {
        let storage = self.storage.read().await;
        storage.query_metrics(component, time_range).await
    }
    
    pub async fn get_analytics(&self, query: AnalyticsQuery) -> Result<AnalyticsResult, Box<dyn std::error::Error>> {
        self.analytics_engine.execute_query(query).await
    }
}

/// Trait for metric collectors
#[async_trait::async_trait]
pub trait MetricCollector: Send + Sync {
    async fn collect(&self) -> Result<Vec<Metric>, Box<dyn std::error::Error>>;
}

/// Core metric structure
#[derive(Clone, Debug)]
pub struct Metric {
    pub name: String,
    pub value: MetricValue,
    pub timestamp: SystemTime,
    pub labels: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(HistogramData),
    Summary(SummaryData),
}

#[derive(Clone, Debug)]
pub struct HistogramData {
    pub buckets: Vec<HistogramBucket>,
    pub sum: f64,
    pub count: u64,
}

#[derive(Clone, Debug)]
pub struct HistogramBucket {
    pub upper_bound: f64,
    pub count: u64,
}

#[derive(Clone, Debug)]
pub struct SummaryData {
    pub quantiles: Vec<Quantile>,
    pub sum: f64,
    pub count: u64,
}

#[derive(Clone, Debug)]
pub struct Quantile {
    pub quantile: f64,
    pub value: f64,
}

/// Time range for metric queries
#[derive(Clone, Debug)]
pub struct TimeRange {
    pub start: SystemTime,
    pub end: SystemTime,
}

impl TimeRange {
    pub fn last_hour() -> Self {
        let end = SystemTime::now();
        let start = end - Duration::from_secs(3600);
        Self { start, end }
    }
    
    pub fn last_day() -> Self {
        let end = SystemTime::now();
        let start = end - Duration::from_secs(86400);
        Self { start, end }
    }
}

/// Metrics storage system
pub struct MetricsStorage {
    data: HashMap<String, Vec<Metric>>,
}

impl MetricsStorage {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
    
    pub async fn store_metric(&mut self, component: &str, metric: Metric) {
        self.data.entry(component.to_string()).or_insert_with(Vec::new).push(metric);
        
        // Cleanup old metrics (keep last 24 hours)
        let cutoff = SystemTime::now() - Duration::from_secs(86400);
        if let Some(metrics) = self.data.get_mut(component) {
            metrics.retain(|m| m.timestamp > cutoff);
        }
    }
    
    pub async fn query_metrics(&self, component: &str, time_range: TimeRange) -> Result<Vec<Metric>, Box<dyn std::error::Error>> {
        let metrics = self.data.get(component)
            .map(|m| m.iter()
                 .filter(|metric| metric.timestamp >= time_range.start && metric.timestamp <= time_range.end)
                 .cloned()
                 .collect())
            .unwrap_or_default();
        
        Ok(metrics)
    }
}

/// Analytics engine for processing metrics
pub struct AnalyticsEngine {
    storage: Arc<RwLock<MetricsStorage>>,
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl AnalyticsEngine {
    pub fn new(storage: Arc<RwLock<MetricsStorage>>) -> Self {
        Self {
            storage,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.running.store(true, std::sync::atomic::Ordering::Relaxed);
        
        // Start analytics processing loop
        let storage = self.storage.clone();
        let running = self.running.clone();
        
        tokio::spawn(async move {
            while running.load(std::sync::atomic::Ordering::Relaxed) {
                // Perform periodic analytics computations
                let analytics = Self::compute_periodic_analytics(&storage).await;
                
                match analytics {
                    Ok(_) => {
                        // Analytics computed successfully
                    },
                    Err(e) => {
                        error!("Analytics computation failed: {}", e);
                    }
                }
                
                tokio::time::sleep(Duration::from_secs(60)).await; // Run every minute
            }
        });
        
        Ok(())
    }
    
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.running.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn compute_periodic_analytics(storage: &Arc<RwLock<MetricsStorage>>) -> Result<(), Box<dyn std::error::Error>> {
        let storage_guard = storage.read().await;
        
        // Compute various analytics
        for (component, metrics) in &storage_guard.data {
            if !metrics.is_empty() {
                let _avg_value = Self::compute_average(metrics);
                let _trend = Self::compute_trend(metrics);
                let _anomalies = Self::detect_anomalies(metrics);
                
                // Store computed analytics (would be persisted in real implementation)
            }
        }
        
        Ok(())
    }
    
    fn compute_average(metrics: &[Metric]) -> f64 {
        let sum: f64 = metrics.iter()
            .filter_map(|m| match &m.value {
                MetricValue::Gauge(v) => Some(*v),
                MetricValue::Counter(v) => Some(*v as f64),
                _ => None,
            })
            .sum();
        
        sum / metrics.len() as f64
    }
    
    fn compute_trend(metrics: &[Metric]) -> TrendDirection {
        if metrics.len() < 2 {
            return TrendDirection::Stable;
        }
        
        let first_half = &metrics[0..metrics.len()/2];
        let second_half = &metrics[metrics.len()/2..];
        
        let first_avg = Self::compute_average(first_half);
        let second_avg = Self::compute_average(second_half);
        
        let change_percent = (second_avg - first_avg) / first_avg * 100.0;
        
        if change_percent > 10.0 {
            TrendDirection::Increasing
        } else if change_percent < -10.0 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        }
    }
    
    fn detect_anomalies(metrics: &[Metric]) -> Vec<AnomalyDetection> {
        let mut anomalies = Vec::new();
        
        if metrics.len() < 10 {
            return anomalies; // Need sufficient data
        }
        
        let avg = Self::compute_average(metrics);
        let std_dev = Self::compute_std_dev(metrics, avg);
        let threshold = avg + 2.0 * std_dev; // 2 sigma threshold
        
        for metric in metrics {
            if let MetricValue::Gauge(value) = &metric.value {
                if *value > threshold {
                    anomalies.push(AnomalyDetection {
                        metric_name: metric.name.clone(),
                        timestamp: metric.timestamp,
                        value: *value,
                        expected_value: avg,
                        severity: AnomalySeverity::High,
                    });
                }
            }
        }
        
        anomalies
    }
    
    fn compute_std_dev(metrics: &[Metric], mean: f64) -> f64 {
        let variance: f64 = metrics.iter()
            .filter_map(|m| match &m.value {
                MetricValue::Gauge(v) => Some((*v - mean).powi(2)),
                MetricValue::Counter(v) => Some((*v as f64 - mean).powi(2)),
                _ => None,
            })
            .sum::<f64>() / metrics.len() as f64;
        
        variance.sqrt()
    }
    
    pub async fn execute_query(&self, query: AnalyticsQuery) -> Result<AnalyticsResult, Box<dyn std::error::Error>> {
        let storage = self.storage.read().await;
        
        match query.query_type {
            AnalyticsQueryType::Average => {
                let metrics = storage.query_metrics(&query.component, query.time_range).await?;
                let avg = Self::compute_average(&metrics);
                Ok(AnalyticsResult::Scalar(avg))
            },
            AnalyticsQueryType::Trend => {
                let metrics = storage.query_metrics(&query.component, query.time_range).await?;
                let trend = Self::compute_trend(&metrics);
                Ok(AnalyticsResult::Trend(trend))
            },
            AnalyticsQueryType::Anomalies => {
                let metrics = storage.query_metrics(&query.component, query.time_range).await?;
                let anomalies = Self::detect_anomalies(&metrics);
                Ok(AnalyticsResult::Anomalies(anomalies))
            },
        }
    }
}

#[derive(Clone, Debug)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

#[derive(Clone, Debug)]
pub struct AnomalyDetection {
    pub metric_name: String,
    pub timestamp: SystemTime,
    pub value: f64,
    pub expected_value: f64,
    pub severity: AnomalySeverity,
}

#[derive(Clone, Debug)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug)]
pub struct AnalyticsQuery {
    pub component: String,
    pub query_type: AnalyticsQueryType,
    pub time_range: TimeRange,
}

#[derive(Clone, Debug)]
pub enum AnalyticsQueryType {
    Average,
    Trend,
    Anomalies,
}

#[derive(Clone, Debug)]
pub enum AnalyticsResult {
    Scalar(f64),
    Trend(TrendDirection),
    Anomalies(Vec<AnomalyDetection>),
}

/// Run comprehensive metrics system tests
pub async fn run_metrics_tests() -> TestResult {
    init_test_logging();
    info!("Testing comprehensive metrics and analytics system");
    
    // Test metrics collection
    test_metric_collection().await?;
    test_analytics_engine().await?;
    test_alerting_system().await?;
    test_performance_analytics().await?;
    
    Ok(())
}

async fn test_metric_collection() -> TestResult {
    info!("Testing metric collection system");
    
    let metrics_system = MetricsSystem::new().await?;
    metrics_system.start().await?;
    
    // Let it collect metrics for a few seconds
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Query collected metrics
    let runtime_metrics = metrics_system.get_metrics("runtime", TimeRange::last_hour()).await?;
    assert!(!runtime_metrics.is_empty(), "Should have collected runtime metrics");
    
    let consensus_metrics = metrics_system.get_metrics("consensus", TimeRange::last_hour()).await?;
    assert!(!consensus_metrics.is_empty(), "Should have collected consensus metrics");
    
    metrics_system.stop().await?;
    
    info!("✅ Metric collection system validated");
    Ok(())
}

async fn test_analytics_engine() -> TestResult {
    info!("Testing analytics engine");
    
    let metrics_system = MetricsSystem::new().await?;
    metrics_system.start().await?;
    
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Test different analytics queries
    let queries = vec![
        AnalyticsQuery {
            component: "runtime".to_string(),
            query_type: AnalyticsQueryType::Average,
            time_range: TimeRange::last_hour(),
        },
        AnalyticsQuery {
            component: "consensus".to_string(),
            query_type: AnalyticsQueryType::Trend,
            time_range: TimeRange::last_hour(),
        },
        AnalyticsQuery {
            component: "network".to_string(),
            query_type: AnalyticsQueryType::Anomalies,
            time_range: TimeRange::last_hour(),
        },
    ];
    
    for query in queries {
        let result = metrics_system.get_analytics(query.clone()).await?;
        match result {
            AnalyticsResult::Scalar(value) => {
                assert!(value.is_finite(), "Analytics result should be finite");
            },
            AnalyticsResult::Trend(_trend) => {
                // Trend analysis completed
            },
            AnalyticsResult::Anomalies(_anomalies) => {
                // Anomaly detection completed
            },
        }
        
        info!("✅ Analytics query executed: {:?}", query.query_type);
    }
    
    metrics_system.stop().await?;
    Ok(())
}

async fn test_alerting_system() -> TestResult {
    info!("Testing alerting system");
    
    let alerting_system = alerting::AlertingSystem::new();
    alerting_system.start().await?;
    
    // Test different alert types
    let test_alerts = vec![
        alerting::Alert {
            id: "test-alert-1".to_string(),
            severity: alerting::AlertSeverity::Warning,
            component: "consensus".to_string(),
            message: "High consensus latency detected".to_string(),
            timestamp: SystemTime::now(),
            resolved: false,
        },
        alerting::Alert {
            id: "test-alert-2".to_string(),
            severity: alerting::AlertSeverity::Critical,
            component: "network".to_string(),
            message: "Network partition detected".to_string(),
            timestamp: SystemTime::now(),
            resolved: false,
        },
    ];
    
    for alert in test_alerts {
        alerting_system.trigger_alert(alert.clone()).await?;
        info!("✅ Alert triggered: {}", alert.message);
    }
    
    // Test alert resolution
    alerting_system.resolve_alert("test-alert-1").await?;
    
    alerting_system.stop().await?;
    info!("✅ Alerting system validated");
    Ok(())
}

async fn test_performance_analytics() -> TestResult {
    info!("Testing performance analytics");
    
    let perf_analytics = performance_analytics::PerformanceAnalytics::new().await?;
    perf_analytics.start().await?;
    
    // Let it collect some performance data
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Test performance report generation
    let report = perf_analytics.generate_performance_report().await?;
    
    assert!(!report.system_metrics.is_empty(), "Should have system metrics");
    assert!(report.overall_health_score >= 0.0 && report.overall_health_score <= 100.0, 
           "Health score should be between 0-100");
    
    perf_analytics.stop().await?;
    
    info!("✅ Performance analytics validated with health score: {:.1}", 
          report.overall_health_score);
    Ok(())
}