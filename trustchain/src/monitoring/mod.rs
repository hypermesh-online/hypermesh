//! Native monitoring system for TrustChain
//!
//! Built-in monitoring without external dependencies for production deployment

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{info, warn, error, debug};

pub mod metrics;
pub mod health;
pub mod export;

pub use metrics::{Metrics, MetricsSnapshot, ComponentMetrics};
pub use health::{HealthCheck, HealthStatus, ComponentHealth};
pub use export::{MetricsExporter, ExportFormat};

/// Native monitoring system for TrustChain
pub struct MonitoringSystem {
    /// Metrics collection
    metrics: Arc<Metrics>,
    /// Health check system
    health: Arc<HealthCheck>,
    /// Metrics exporters
    exporters: Arc<RwLock<Vec<Box<dyn MetricsExporter>>>>,
    /// Configuration
    config: MonitoringConfig,
    /// System start time
    start_time: Instant,
}

/// Monitoring system configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable monitoring
    pub enabled: bool,
    /// Metrics collection interval (seconds)
    pub collection_interval: u64,
    /// Health check interval (seconds)
    pub health_check_interval: u64,
    /// Enable metrics export
    pub enable_export: bool,
    /// Export format
    pub export_format: ExportFormat,
    /// Metrics retention (seconds)
    pub retention_seconds: u64,
    /// Alert thresholds
    pub alert_thresholds: AlertThresholds,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval: 10,
            health_check_interval: 30,
            enable_export: true,
            export_format: ExportFormat::Json,
            retention_seconds: 3600, // 1 hour
            alert_thresholds: AlertThresholds::default(),
        }
    }
}

/// Alert thresholds for monitoring
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// Maximum certificate issuance time (ms)
    pub max_cert_issuance_ms: u64,
    /// Minimum success rate
    pub min_success_rate: f64,
    /// Maximum memory usage (MB)
    pub max_memory_mb: u64,
    /// Maximum error rate
    pub max_error_rate: f64,
    /// Minimum availability
    pub min_availability: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            max_cert_issuance_ms: 100, // 100ms threshold
            min_success_rate: 0.95,     // 95% success rate
            max_memory_mb: 4096,        // 4GB memory
            max_error_rate: 0.05,       // 5% error rate
            min_availability: 0.99,      // 99% availability
        }
    }
}

/// Monitoring alert
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonitoringAlert {
    /// Alert ID
    pub id: String,
    /// Alert level
    pub level: AlertLevel,
    /// Component name
    pub component: String,
    /// Alert message
    pub message: String,
    /// Alert timestamp
    pub timestamp: SystemTime,
    /// Associated metric
    pub metric: Option<String>,
    /// Current value
    pub value: Option<f64>,
    /// Threshold value
    pub threshold: Option<f64>,
}

/// Alert level
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

impl MonitoringSystem {
    /// Create new monitoring system
    pub async fn new(config: MonitoringConfig) -> anyhow::Result<Self> {
        info!("Initializing native monitoring system");

        let metrics = Arc::new(Metrics::new());
        let health = Arc::new(HealthCheck::new());
        let exporters = Arc::new(RwLock::new(Vec::new()));

        Ok(Self {
            metrics,
            health,
            exporters,
            config,
            start_time: Instant::now(),
        })
    }

    /// Start monitoring system
    pub async fn start(&self) -> anyhow::Result<()> {
        if !self.config.enabled {
            info!("Monitoring system disabled");
            return Ok(());
        }

        info!("Starting monitoring system");

        // Start metrics collection
        let metrics = self.metrics.clone();
        let collection_interval = self.config.collection_interval;
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(collection_interval));
            loop {
                interval.tick().await;
                metrics.collect().await;
            }
        });

        // Start health checks
        let health = self.health.clone();
        let health_interval = self.config.health_check_interval;
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(health_interval));
            loop {
                interval.tick().await;
                health.check_all().await;
            }
        });

        // Start alert monitoring
        let alert_monitor = self.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                alert_monitor.check_alerts().await;
            }
        });

        info!("Monitoring system started");
        Ok(())
    }

    /// Record certificate issuance
    pub async fn record_cert_issuance(&self, duration_ms: u64, success: bool) {
        self.metrics.record_cert_issuance(duration_ms, success).await;
    }

    /// Record DNS resolution
    pub async fn record_dns_resolution(&self, duration_ms: u64, success: bool) {
        self.metrics.record_dns_resolution(duration_ms, success).await;
    }

    /// Record CT log entry
    pub async fn record_ct_log_entry(&self, duration_ms: u64, success: bool) {
        self.metrics.record_ct_log_entry(duration_ms, success).await;
    }

    /// Record consensus validation
    pub async fn record_consensus_validation(&self, duration_ms: u64, success: bool) {
        self.metrics.record_consensus_validation(duration_ms, success).await;
    }

    /// Get current metrics snapshot
    pub async fn get_metrics(&self) -> MetricsSnapshot {
        self.metrics.snapshot().await
    }

    /// Get health status
    pub async fn get_health(&self) -> HealthStatus {
        self.health.get_status().await
    }

    /// Get system uptime
    pub fn get_uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get system info
    pub async fn get_system_info(&self) -> SystemInfo {
        SystemInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: self.get_uptime().as_secs(),
            start_time: SystemTime::now() - self.get_uptime(),
            metrics: self.get_metrics().await,
            health: self.get_health().await,
        }
    }

    /// Check for alerts based on thresholds
    async fn check_alerts(&self) {
        let metrics = self.get_metrics().await;
        let mut alerts = Vec::new();

        // Check certificate issuance time
        if let Some(ca_metrics) = metrics.components.get("ca") {
            if let Some(avg_time) = ca_metrics.additional_metrics.get("avg_issuance_time_ms") {
                if *avg_time > self.config.alert_thresholds.max_cert_issuance_ms as f64 {
                    alerts.push(MonitoringAlert {
                        id: uuid::Uuid::new_v4().to_string(),
                        level: AlertLevel::Warning,
                        component: "ca".to_string(),
                        message: format!("Certificate issuance time exceeds threshold: {}ms > {}ms",
                            avg_time, self.config.alert_thresholds.max_cert_issuance_ms),
                        timestamp: SystemTime::now(),
                        metric: Some("avg_issuance_time_ms".to_string()),
                        value: Some(*avg_time),
                        threshold: Some(self.config.alert_thresholds.max_cert_issuance_ms as f64),
                    });
                }
            }

            // Check success rate
            if ca_metrics.success_rate < self.config.alert_thresholds.min_success_rate {
                alerts.push(MonitoringAlert {
                    id: uuid::Uuid::new_v4().to_string(),
                    level: AlertLevel::Error,
                    component: "ca".to_string(),
                    message: format!("CA success rate below threshold: {:.2}% < {:.2}%",
                        ca_metrics.success_rate * 100.0,
                        self.config.alert_thresholds.min_success_rate * 100.0),
                    timestamp: SystemTime::now(),
                    metric: Some("success_rate".to_string()),
                    value: Some(ca_metrics.success_rate),
                    threshold: Some(self.config.alert_thresholds.min_success_rate),
                });
            }
        }

        // Process alerts
        for alert in alerts {
            match alert.level {
                AlertLevel::Info => debug!("{}", alert.message),
                AlertLevel::Warning => warn!("{}", alert.message),
                AlertLevel::Error => error!("{}", alert.message),
                AlertLevel::Critical => error!("CRITICAL: {}", alert.message),
            }
        }
    }

    /// Add metrics exporter
    pub async fn add_exporter(&self, exporter: Box<dyn MetricsExporter>) {
        let mut exporters = self.exporters.write().await;
        exporters.push(exporter);
    }

    /// Export metrics using all configured exporters
    pub async fn export_metrics(&self) -> anyhow::Result<()> {
        let snapshot = self.get_metrics().await;
        let exporters = self.exporters.read().await;

        for exporter in exporters.iter() {
            if let Err(e) = exporter.export(&snapshot).await {
                error!("Failed to export metrics: {}", e);
            }
        }

        Ok(())
    }

    /// Shutdown monitoring system
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        info!("Shutting down monitoring system");
        // Cleanup tasks handled by drop
        Ok(())
    }
}

impl Clone for MonitoringSystem {
    fn clone(&self) -> Self {
        Self {
            metrics: self.metrics.clone(),
            health: self.health.clone(),
            exporters: self.exporters.clone(),
            config: self.config.clone(),
            start_time: self.start_time,
        }
    }
}

/// System information summary
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    /// System version
    pub version: String,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// System start time
    pub start_time: SystemTime,
    /// Current metrics
    pub metrics: MetricsSnapshot,
    /// Health status
    pub health: HealthStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitoring_system_creation() {
        let config = MonitoringConfig::default();
        let monitoring = MonitoringSystem::new(config).await.unwrap();

        let health = monitoring.get_health().await;
        assert!(health.is_healthy);
    }

    #[tokio::test]
    async fn test_metrics_recording() {
        let config = MonitoringConfig::default();
        let monitoring = MonitoringSystem::new(config).await.unwrap();

        // Record some metrics
        monitoring.record_cert_issuance(35, true).await;
        monitoring.record_dns_resolution(10, true).await;
        monitoring.record_ct_log_entry(5, true).await;

        let metrics = monitoring.get_metrics().await;
        assert!(metrics.components.contains_key("ca"));
        assert!(metrics.components.contains_key("dns"));
        assert!(metrics.components.contains_key("ct"));
    }

    #[tokio::test]
    async fn test_system_info() {
        let config = MonitoringConfig::default();
        let monitoring = MonitoringSystem::new(config).await.unwrap();

        let info = monitoring.get_system_info().await;
        assert!(!info.version.is_empty());
        assert!(info.uptime_seconds >= 0);
    }
}