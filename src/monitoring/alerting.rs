//! Performance Alerting System
//!
//! Manages performance alerts and automated responses

use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::SystemTime;
use std::collections::HashMap;
use dashmap::DashMap;
use tracing::{info, warn, error};

use super::metrics::*;
use super::collector::MetricsSnapshot;

/// Alert manager for performance monitoring
pub struct AlertManager {
    /// Active alerts
    active_alerts: Arc<DashMap<String, PerformanceAlert>>,

    /// Alert thresholds
    thresholds: AlertThresholds,

    /// Alert handlers
    handlers: Vec<Box<dyn AlertHandler>>,

    /// Alert history
    history: Vec<PerformanceAlert>,
}

impl AlertManager {
    /// Create new alert manager
    pub fn new(thresholds: AlertThresholds) -> Self {
        Self {
            active_alerts: Arc::new(DashMap::new()),
            thresholds,
            handlers: Vec::new(),
            history: Vec::new(),
        }
    }

    /// Add alert handler
    pub fn add_handler(&mut self, handler: Box<dyn AlertHandler>) {
        self.handlers.push(handler);
    }

    /// Check metrics and trigger alerts
    pub async fn check_metrics(&mut self, snapshot: &MetricsSnapshot) -> Result<()> {
        // Check throughput
        if snapshot.stoq.current_throughput_gbps < self.thresholds.throughput_min_gbps {
            self.trigger_alert(
                AlertLevel::Critical,
                AlertCategory::Throughput,
                format!(
                    "Throughput below target: {:.2} Gbps (target: {:.2} Gbps)",
                    snapshot.stoq.current_throughput_gbps,
                    self.thresholds.throughput_min_gbps
                ),
                "throughput",
                snapshot.stoq.current_throughput_gbps,
                self.thresholds.throughput_min_gbps,
            ).await?;
        }

        // Check latency
        if snapshot.integration.avg_e2e_latency_ms > self.thresholds.latency_max_ms {
            self.trigger_alert(
                AlertLevel::Warning,
                AlertCategory::Latency,
                format!(
                    "Latency above threshold: {:.2}ms (max: {:.2}ms)",
                    snapshot.integration.avg_e2e_latency_ms,
                    self.thresholds.latency_max_ms
                ),
                "latency",
                snapshot.integration.avg_e2e_latency_ms,
                self.thresholds.latency_max_ms,
            ).await?;
        }

        // Check consensus time
        if snapshot.hypermesh.avg_consensus_time_ms > self.thresholds.consensus_time_max_ms {
            self.trigger_alert(
                AlertLevel::Warning,
                AlertCategory::Consensus,
                format!(
                    "Consensus time above threshold: {:.2}ms (max: {:.2}ms)",
                    snapshot.hypermesh.avg_consensus_time_ms,
                    self.thresholds.consensus_time_max_ms
                ),
                "consensus_time",
                snapshot.hypermesh.avg_consensus_time_ms,
                self.thresholds.consensus_time_max_ms,
            ).await?;
        }

        // Check certificate time
        if snapshot.trustchain.avg_cert_generation_ms > self.thresholds.certificate_time_max_ms {
            self.trigger_alert(
                AlertLevel::Warning,
                AlertCategory::Certificates,
                format!(
                    "Certificate generation time above threshold: {:.2}ms (max: {:.2}ms)",
                    snapshot.trustchain.avg_cert_generation_ms,
                    self.thresholds.certificate_time_max_ms
                ),
                "cert_time",
                snapshot.trustchain.avg_cert_generation_ms,
                self.thresholds.certificate_time_max_ms,
            ).await?;
        }

        // Check CPU usage
        if snapshot.stack.total_cpu_usage_percent > self.thresholds.cpu_usage_max_percent {
            self.trigger_alert(
                AlertLevel::Error,
                AlertCategory::Resources,
                format!(
                    "CPU usage above threshold: {:.1}% (max: {:.1}%)",
                    snapshot.stack.total_cpu_usage_percent,
                    self.thresholds.cpu_usage_max_percent
                ),
                "cpu_usage",
                snapshot.stack.total_cpu_usage_percent as f64,
                self.thresholds.cpu_usage_max_percent as f64,
            ).await?;
        }

        Ok(())
    }

    /// Trigger an alert
    async fn trigger_alert(
        &mut self,
        level: AlertLevel,
        category: AlertCategory,
        message: String,
        metric_name: &str,
        metric_value: f64,
        threshold_value: f64,
    ) -> Result<()> {
        let alert_id = format!("{}_{}", category.to_string(), metric_name);

        // Check if alert already exists
        if self.active_alerts.contains_key(&alert_id) {
            return Ok(());
        }

        let alert = PerformanceAlert {
            id: alert_id.clone(),
            level: level.clone(),
            category,
            message: message.clone(),
            metric_name: metric_name.to_string(),
            metric_value,
            threshold_value,
            triggered_at: SystemTime::now(),
            resolved_at: None,
            metadata: HashMap::new(),
            action_taken: None,
        };

        // Log alert
        match level {
            AlertLevel::Critical => error!("CRITICAL ALERT: {}", message),
            AlertLevel::Error => error!("ERROR ALERT: {}", message),
            AlertLevel::Warning => warn!("WARNING ALERT: {}", message),
            AlertLevel::Info => info!("INFO ALERT: {}", message),
        }

        // Notify handlers
        for handler in &self.handlers {
            handler.handle_alert(&alert).await?;
        }

        // Store alert
        self.active_alerts.insert(alert_id, alert.clone());
        self.history.push(alert);

        Ok(())
    }

    /// Resolve an alert
    pub async fn resolve_alert(&mut self, alert_id: &str) -> Result<()> {
        if let Some((_, mut alert)) = self.active_alerts.remove(alert_id) {
            alert.resolved_at = Some(SystemTime::now());
            info!("Alert resolved: {}", alert.message);

            // Notify handlers
            for handler in &self.handlers {
                handler.handle_resolution(&alert).await?;
            }

            self.history.push(alert);
        }

        Ok(())
    }

    /// Get active alerts
    pub fn get_active_alerts(&self) -> Vec<PerformanceAlert> {
        self.active_alerts.iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Clear old alerts
    pub fn clear_old_alerts(&mut self, age: std::time::Duration) {
        let cutoff = SystemTime::now() - age;

        self.active_alerts.retain(|_, alert| {
            alert.triggered_at > cutoff
        });

        self.history.retain(|alert| {
            alert.triggered_at > cutoff
        });
    }
}

/// Alert handler trait
#[async_trait::async_trait]
pub trait AlertHandler: Send + Sync {
    /// Handle new alert
    async fn handle_alert(&self, alert: &PerformanceAlert) -> Result<()>;

    /// Handle alert resolution
    async fn handle_resolution(&self, alert: &PerformanceAlert) -> Result<()>;
}

/// Log-based alert handler
pub struct LogAlertHandler;

#[async_trait::async_trait]
impl AlertHandler for LogAlertHandler {
    async fn handle_alert(&self, alert: &PerformanceAlert) -> Result<()> {
        info!(
            "Alert triggered: {} - {} ({})",
            alert.id,
            alert.message,
            alert.level.to_string()
        );
        Ok(())
    }

    async fn handle_resolution(&self, alert: &PerformanceAlert) -> Result<()> {
        info!("Alert resolved: {} - {}", alert.id, alert.message);
        Ok(())
    }
}

/// Auto-remediation handler
pub struct AutoRemediationHandler;

#[async_trait::async_trait]
impl AlertHandler for AutoRemediationHandler {
    async fn handle_alert(&self, alert: &PerformanceAlert) -> Result<()> {
        match &alert.category {
            AlertCategory::Throughput => {
                // Attempt to optimize throughput
                info!("Auto-remediation: Optimizing throughput settings");
                // Implementation would adjust QUIC parameters
            }
            AlertCategory::Resources => {
                // Attempt to free resources
                info!("Auto-remediation: Clearing caches and freeing resources");
                // Implementation would clear caches
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_resolution(&self, _alert: &PerformanceAlert) -> Result<()> {
        Ok(())
    }
}

impl ToString for AlertLevel {
    fn to_string(&self) -> String {
        match self {
            AlertLevel::Info => "INFO".to_string(),
            AlertLevel::Warning => "WARNING".to_string(),
            AlertLevel::Error => "ERROR".to_string(),
            AlertLevel::Critical => "CRITICAL".to_string(),
        }
    }
}

impl ToString for AlertCategory {
    fn to_string(&self) -> String {
        match self {
            AlertCategory::Throughput => "throughput".to_string(),
            AlertCategory::Latency => "latency".to_string(),
            AlertCategory::Errors => "errors".to_string(),
            AlertCategory::Resources => "resources".to_string(),
            AlertCategory::Security => "security".to_string(),
            AlertCategory::Consensus => "consensus".to_string(),
            AlertCategory::Certificates => "certificates".to_string(),
        }
    }
}