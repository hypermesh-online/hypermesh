//! Performance Monitoring for Internet 2.0 Protocol Stack
//!
//! Comprehensive monitoring system that tracks performance across all layers
//! and ensures the 40 Gbps throughput target and other performance goals.

pub mod metrics;
pub mod collector;
pub mod alerting;

use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{info, debug, warn};

use crate::config::HyperMeshServerConfig;

pub use metrics::*;
pub use collector::*;
pub use alerting::*;

/// Performance Monitor for Internet 2.0 Protocol Stack
///
/// Tracks performance metrics across all layers:
/// - STOQ Transport: 40 Gbps throughput target
/// - HyperMesh Assets: Consensus validation timing (<100ms)
/// - TrustChain Authority: Certificate operations (<35ms)
/// - Integration: Cross-layer coordination efficiency
pub struct PerformanceMonitor {
    /// Configuration
    config: Arc<HyperMeshServerConfig>,

    /// Metrics collector
    collector: Arc<MetricsCollector>,

    /// Alert manager
    alert_manager: Arc<RwLock<AlertManager>>,

    /// Metrics aggregator
    aggregator: Arc<RwLock<MetricsAggregator>>,

    /// Monitoring state
    monitoring_state: Arc<RwLock<MonitoringState>>,
}

impl PerformanceMonitor {
    /// Create new performance monitor
    pub async fn new(config: Arc<HyperMeshServerConfig>) -> Result<Self> {
        let collection_interval = Duration::from_secs(config.monitoring_interval);

        let collector = Arc::new(MetricsCollector::new(collection_interval));

        let thresholds = AlertThresholds::default();
        let mut alert_manager = AlertManager::new(thresholds.clone());

        // Add alert handlers
        alert_manager.add_handler(Box::new(LogAlertHandler));
        alert_manager.add_handler(Box::new(AutoRemediationHandler));

        let aggregator = MetricsAggregator::new(1000); // Keep 1000 snapshots

        let monitoring_state = MonitoringState {
            monitoring_enabled: true,
            collection_interval,
            retention_period: Duration::from_secs(86400), // 24 hours
            alert_thresholds: thresholds,
            export_enabled: config.enable_metrics_export,
            export_interval: Duration::from_secs(60),
        };

        Ok(Self {
            config,
            collector,
            alert_manager: Arc::new(RwLock::new(alert_manager)),
            aggregator: Arc::new(RwLock::new(aggregator)),
            monitoring_state: Arc::new(RwLock::new(monitoring_state)),
        })
    }

    /// Initialize monitoring system
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing performance monitoring");

        // Start collection loop
        self.start_collection_loop().await;

        // Start alert checking loop
        self.start_alert_loop().await;

        // Start export loop if enabled
        let state = self.monitoring_state.read().await;
        if state.export_enabled {
            self.start_export_loop().await;
        }

        info!("Performance monitoring initialized");
        Ok(())
    }

    /// Start metrics collection loop
    async fn start_collection_loop(&self) {
        let collector = self.collector.clone();
        let aggregator = self.aggregator.clone();
        let state = self.monitoring_state.clone();

        tokio::spawn(async move {
            loop {
                let monitoring_state = state.read().await;
                if !monitoring_state.monitoring_enabled {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }

                let interval = monitoring_state.collection_interval;
                drop(monitoring_state);

                // Collect metrics
                if let Err(e) = collector.collect().await {
                    warn!("Failed to collect metrics: {}", e);
                }

                // Get snapshot and store
                let snapshot = collector.get_snapshot().await;

                let mut agg = aggregator.write().await;
                agg.add_snapshot(snapshot);

                tokio::time::sleep(interval).await;
            }
        });
    }

    /// Start alert checking loop
    async fn start_alert_loop(&self) {
        let collector = self.collector.clone();
        let alert_manager = self.alert_manager.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));

            loop {
                interval.tick().await;

                // Get current metrics
                let snapshot = collector.get_snapshot().await;

                // Check for alerts
                let mut manager = alert_manager.write().await;
                if let Err(e) = manager.check_metrics(&snapshot).await {
                    warn!("Failed to check alerts: {}", e);
                }

                // Clear old alerts
                manager.clear_old_alerts(Duration::from_secs(3600));
            }
        });
    }

    /// Start metrics export loop
    async fn start_export_loop(&self) {
        let aggregator = self.aggregator.clone();
        let state = self.monitoring_state.clone();

        tokio::spawn(async move {
            loop {
                let monitoring_state = state.read().await;
                let interval = monitoring_state.export_interval;
                drop(monitoring_state);

                // Export metrics
                let agg = aggregator.read().await;
                let stats = agg.calculate_statistics(Duration::from_secs(300)); // 5 min window

                // Log statistics
                info!(
                    "Performance Stats - Throughput: {:.2} Gbps, Latency: {:.2}ms",
                    stats.avg_throughput_gbps,
                    stats.avg_latency_ms
                );

                tokio::time::sleep(interval).await;
            }
        });
    }

    /// Get current performance metrics
    pub async fn get_metrics(&self) -> MetricsSnapshot {
        self.collector.get_snapshot().await
    }

    /// Get performance statistics
    pub async fn get_statistics(&self, window: Duration) -> StackStatistics {
        let aggregator = self.aggregator.read().await;
        aggregator.calculate_statistics(window)
    }

    /// Get active alerts
    pub async fn get_alerts(&self) -> Vec<PerformanceAlert> {
        let manager = self.alert_manager.read().await;
        manager.get_active_alerts()
    }

    /// Get layer health status
    pub async fn get_layer_health(&self) -> Vec<LayerHealth> {
        let snapshot = self.get_metrics().await;

        vec![
            LayerHealth {
                layer_name: "STOQ Transport".to_string(),
                status: self.calculate_health_status(snapshot.stack.stoq_performance_score),
                health_score: snapshot.stack.stoq_performance_score,
                last_check: SystemTime::now(),
            },
            LayerHealth {
                layer_name: "HyperMesh Assets".to_string(),
                status: self.calculate_health_status(snapshot.stack.hypermesh_performance_score),
                health_score: snapshot.stack.hypermesh_performance_score,
                last_check: SystemTime::now(),
            },
            LayerHealth {
                layer_name: "TrustChain Authority".to_string(),
                status: self.calculate_health_status(snapshot.stack.trustchain_performance_score),
                health_score: snapshot.stack.trustchain_performance_score,
                last_check: SystemTime::now(),
            },
            LayerHealth {
                layer_name: "Integration Layer".to_string(),
                status: self.calculate_health_status(snapshot.stack.integration_performance_score),
                health_score: snapshot.stack.integration_performance_score,
                last_check: SystemTime::now(),
            },
        ]
    }

    /// Calculate health status from score
    fn calculate_health_status(&self, score: f64) -> HealthStatus {
        if score >= 90.0 {
            HealthStatus::Healthy
        } else if score >= 70.0 {
            HealthStatus::Degraded
        } else if score >= 50.0 {
            HealthStatus::Unhealthy
        } else {
            HealthStatus::Unknown
        }
    }

    /// Update monitoring configuration
    pub async fn update_config(&self, config: MonitoringState) {
        let mut state = self.monitoring_state.write().await;
        *state = config;
        info!("Monitoring configuration updated");
    }

    /// Generate performance report
    pub async fn generate_report(&self) -> PerformanceReport {
        let snapshot = self.get_metrics().await;
        let stats = self.get_statistics(Duration::from_secs(3600)).await;
        let alerts = self.get_alerts().await;
        let health = self.get_layer_health().await;

        PerformanceReport {
            timestamp: SystemTime::now(),
            current_metrics: snapshot,
            statistics: stats,
            active_alerts: alerts,
            layer_health: health,
            recommendations: self.generate_recommendations(&snapshot),
        }
    }

    /// Generate performance recommendations
    fn generate_recommendations(&self, snapshot: &MetricsSnapshot) -> Vec<String> {
        let mut recommendations = Vec::new();

        if snapshot.stoq.current_throughput_gbps < 40.0 {
            recommendations.push(format!(
                "STOQ throughput at {:.2} Gbps, needs optimization for 40 Gbps target",
                snapshot.stoq.current_throughput_gbps
            ));
        }

        if snapshot.hypermesh.avg_consensus_time_ms > 100.0 {
            recommendations.push(format!(
                "Consensus validation at {:.2}ms, optimize for <100ms target",
                snapshot.hypermesh.avg_consensus_time_ms
            ));
        }

        if snapshot.trustchain.avg_cert_generation_ms > 35.0 {
            recommendations.push(format!(
                "Certificate generation at {:.2}ms, optimize for <35ms target",
                snapshot.trustchain.avg_cert_generation_ms
            ));
        }

        recommendations
    }
}

impl Clone for PerformanceMonitor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            collector: self.collector.clone(),
            alert_manager: self.alert_manager.clone(),
            aggregator: self.aggregator.clone(),
            monitoring_state: self.monitoring_state.clone(),
        }
    }
}

/// Performance report
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub timestamp: SystemTime,
    pub current_metrics: MetricsSnapshot,
    pub statistics: StackStatistics,
    pub active_alerts: Vec<PerformanceAlert>,
    pub layer_health: Vec<LayerHealth>,
    pub recommendations: Vec<String>,
}