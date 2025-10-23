//! Health monitoring components for HyperMesh runtime
//!
//! This module contains components responsible for performance degradation detection,
//! metrics aggregation, and cluster-wide health coordination.

use crate::{Result, RuntimeError};
use crate::health::{
    MetricsRetentionConfig, ClusterCoordinationConfig,
    SystemHealthStatus, HealthAlert, HealthSnapshot
};
use crate::health::config::DegradationConfig;
use nexus_shared::{NodeId, Timestamp};

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use tracing::{debug, info, warn, error, instrument};

/// Performance degradation detector for proactive health monitoring
#[derive(Debug, Clone)]
pub struct PerformanceDegradationDetector {
    config: DegradationConfig,
    historical_metrics: Arc<RwLock<VecDeque<MetricsSnapshot>>>,
}

/// Historical metrics snapshot for degradation analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetricsSnapshot {
    timestamp: Timestamp,
    metrics: HashMap<String, f64>,
}

impl PerformanceDegradationDetector {
    /// Create a new performance degradation detector
    pub fn new(config: &DegradationConfig) -> Self {
        Self {
            config: config.clone(),
            historical_metrics: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// Analyze current metrics for performance degradation patterns
    #[instrument(skip(self, health_status))]
    pub async fn analyze_metrics(&self, health_status: &SystemHealthStatus) -> Result<Vec<HealthAlert>> {
        let mut alerts = Vec::new();
        
        // Capture current metrics snapshot
        let current_snapshot = self.create_metrics_snapshot(health_status)?;
        
        // Update historical data
        {
            let mut history = self.historical_metrics.write()
                .map_err(|e| RuntimeError::LockPoisoned(format!("Historical metrics: {}", e)))?;
            
            history.push_back(current_snapshot.clone());
            
            // Maintain rolling window (use analysis window to calculate size)
            let max_history_size = (self.config.analysis_window.as_secs() / 60).max(10) as usize; // At least 10 entries
            if history.len() > max_history_size {
                history.pop_front();
            }
        }

        // Analyze degradation patterns
        if let Some(alert) = self.detect_performance_degradation(&current_snapshot)? {
            alerts.push(alert);
        }

        Ok(alerts)
    }

    /// Create metrics snapshot from current health status
    fn create_metrics_snapshot(&self, health_status: &SystemHealthStatus) -> Result<MetricsSnapshot> {
        let mut metrics = HashMap::new();
        
        // Extract key performance metrics
        for (component_name, component_health) in &health_status.components {
            // CPU utilization
            if let Some(cpu) = component_health.metrics.get("cpu_utilization") {
                metrics.insert(format!("{}_cpu", component_name), *cpu);
            }
            
            // Memory utilization
            if let Some(memory) = component_health.metrics.get("memory_utilization") {
                metrics.insert(format!("{}_memory", component_name), *memory);
            }
            
            // Response time
            if let Some(response_time) = component_health.metrics.get("response_time_ms") {
                metrics.insert(format!("{}_response_time", component_name), *response_time);
            }
        }

        Ok(MetricsSnapshot {
            timestamp: SystemTime::now().into(),
            metrics,
        })
    }

    /// Detect performance degradation patterns
    fn detect_performance_degradation(&self, current: &MetricsSnapshot) -> Result<Option<HealthAlert>> {
        let history = self.historical_metrics.read()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Historical metrics: {}", e)))?;
        
        // Need at least some historical data for comparison
        if history.len() < 3 {
            return Ok(None);
        }

        // Calculate baseline from recent history
        let recent_snapshots: Vec<_> = history.iter()
            .rev()
            .take(10) // Use a fixed baseline window size
            .collect();

        for (metric_name, current_value) in &current.metrics {
            if let Some(baseline_avg) = self.calculate_baseline_average(metric_name, &recent_snapshots) {
                let deviation_percent = ((current_value - baseline_avg) / baseline_avg * 100.0).abs();
                
                if deviation_percent > self.config.min_degradation_percent {
                    return Ok(Some(HealthAlert {
                        id: format!("degradation_{}", metric_name),
                        severity: crate::health::AlertSeverity::Warning,
                        message: format!(
                            "Performance degradation detected in {}: {}% deviation from baseline",
                            metric_name, deviation_percent
                        ),
                        component: metric_name.split('_').next().unwrap_or("unknown").to_string(),
                        timestamp: current.timestamp,
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert("baseline_average".to_string(), baseline_avg.to_string());
                            meta.insert("current_value".to_string(), current_value.to_string());
                            meta.insert("deviation_percent".to_string(), deviation_percent.to_string());
                            meta
                        },
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Calculate baseline average for a specific metric
    fn calculate_baseline_average(&self, metric_name: &str, snapshots: &[&MetricsSnapshot]) -> Option<f64> {
        let values: Vec<f64> = snapshots.iter()
            .filter_map(|snapshot| snapshot.metrics.get(metric_name))
            .copied()
            .collect();

        if values.is_empty() {
            return None;
        }

        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

/// Health metrics aggregator for long-term storage and analysis
#[derive(Debug, Clone)]
pub struct HealthMetricsAggregator {
    config: MetricsRetentionConfig,
    stored_snapshots: Arc<RwLock<Vec<HealthSnapshot>>>,
}

impl HealthMetricsAggregator {
    /// Create a new health metrics aggregator
    pub fn new(config: &MetricsRetentionConfig) -> Self {
        Self {
            config: config.clone(),
            stored_snapshots: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Store health snapshot for long-term retention
    #[instrument(skip(self, snapshot))]
    pub async fn store_snapshot(&self, snapshot: HealthSnapshot) -> Result<()> {
        let mut stored = self.stored_snapshots.write()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Stored snapshots: {}", e)))?;
        
        stored.push(snapshot);
        
        // Apply retention policy
        self.apply_retention_policy(&mut stored)?;
        
        debug!("Stored health snapshot, total snapshots: {}", stored.len());
        Ok(())
    }

    /// Apply retention policy to stored snapshots
    fn apply_retention_policy(&self, snapshots: &mut Vec<HealthSnapshot>) -> Result<()> {
        let cutoff_time = SystemTime::now() - self.config.retention_duration;
        
        snapshots.retain(|snapshot| {
            SystemTime::from(snapshot.timestamp) > cutoff_time
        });
        
        // Ensure we don't exceed max snapshots
        if snapshots.len() > self.config.max_snapshots {
            let excess = snapshots.len() - self.config.max_snapshots;
            snapshots.drain(0..excess);
        }
        
        Ok(())
    }

    /// Get stored snapshots within a time range
    pub async fn get_snapshots_in_range(&self, start: Timestamp, end: Timestamp) -> Result<Vec<HealthSnapshot>> {
        let stored = self.stored_snapshots.read()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Stored snapshots: {}", e)))?;
        
        Ok(stored.iter()
            .filter(|snapshot| snapshot.timestamp >= start && snapshot.timestamp <= end)
            .cloned()
            .collect())
    }
}

/// Cluster health coordinator for distributed health monitoring
#[derive(Debug, Clone)]
pub struct ClusterHealthCoordinator {
    node_id: NodeId,
    config: ClusterCoordinationConfig,
    peer_health_status: Arc<RwLock<HashMap<NodeId, SystemHealthStatus>>>,
}

impl ClusterHealthCoordinator {
    /// Create a new cluster health coordinator
    pub async fn new(node_id: NodeId, config: &ClusterCoordinationConfig) -> Result<Self> {
        Ok(Self {
            node_id,
            config: config.clone(),
            peer_health_status: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Share health status with cluster peers
    #[instrument(skip(self, health_status))]
    pub async fn share_health_status(&self, health_status: SystemHealthStatus) -> Result<()> {
        // TODO: Implement cluster health sharing through consensus
        // This would integrate with the consensus layer to share health information
        info!(
            node_id = %self.node_id,
            "Sharing health status with cluster peers"
        );
        Ok(())
    }

    /// Receive health status from peer nodes
    #[instrument(skip(self, health_status))]
    pub async fn receive_peer_health_status(&self, peer_id: NodeId, health_status: SystemHealthStatus) -> Result<()> {
        let mut peer_statuses = self.peer_health_status.write()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Peer health status: {}", e)))?;
        
        peer_statuses.insert(peer_id, health_status);
        
        debug!(
            peer_id = %peer_id,
            "Received health status from peer"
        );
        
        Ok(())
    }

    /// Get cluster-wide health summary
    pub async fn get_cluster_health_summary(&self) -> Result<ClusterHealthSummary> {
        let peer_statuses = self.peer_health_status.read()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Peer health status: {}", e)))?;
        
        let total_nodes = peer_statuses.len() + 1; // +1 for this node
        let healthy_nodes = peer_statuses.values()
            .filter(|status| status.overall_status == crate::health::HealthStatus::Healthy)
            .count() + 1; // Assume this node is healthy for now
        
        Ok(ClusterHealthSummary {
            total_nodes,
            healthy_nodes,
            unhealthy_nodes: total_nodes - healthy_nodes,
            cluster_health_percentage: (healthy_nodes as f64 / total_nodes as f64) * 100.0,
            last_updated: SystemTime::now().into(),
        })
    }
}

/// Cluster-wide health summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterHealthSummary {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub unhealthy_nodes: usize,
    pub cluster_health_percentage: f64,
    pub last_updated: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_performance_degradation_detector() {
        let config = DegradationConfig {
            enabled: true,
            degradation_threshold_percent: 20.0,
            history_window_size: 10,
            baseline_window_size: 5,
        };
        
        let detector = PerformanceDegradationDetector::new(&config);
        
        // Test with mock health status
        let health_status = SystemHealthStatus {
            node_id: "test-node".into(),
            overall_status: crate::health::HealthStatus::Healthy,
            components: HashMap::new(),
            last_updated: SystemTime::now().into(),
            alerts: Vec::new(),
        };
        
        let alerts = detector.analyze_metrics(&health_status).await.unwrap();
        assert!(alerts.is_empty()); // No alerts with empty metrics
    }

    #[tokio::test]
    async fn test_health_metrics_aggregator() {
        let config = MetricsRetentionConfig {
            retention_duration: Duration::from_secs(3600),
            max_snapshots: 1000,
            compression: None,
        };
        
        let aggregator = HealthMetricsAggregator::new(&config);
        
        let snapshot = HealthSnapshot {
            timestamp: SystemTime::now().into(),
            system_status: SystemHealthStatus {
                node_id: "test-node".into(),
                overall_status: crate::health::HealthStatus::Healthy,
                components: HashMap::new(),
                last_updated: SystemTime::now().into(),
                alerts: Vec::new(),
            },
            resource_utilization: Default::default(),
        };
        
        aggregator.store_snapshot(snapshot).await.unwrap();
    }
}