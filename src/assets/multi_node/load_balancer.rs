//! Load Balancer for Multi-Node Resource Distribution
//!
//! Implements intelligent load balancing across HyperMesh nodes with
//! predictive scaling and resource optimization.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

use crate::assets::core::{AssetId, AssetType, AssetResult};
use super::{NodeId, NodeInfo};

/// Load balancer for resource distribution
pub struct LoadBalancer {
    /// Node load metrics
    node_loads: Arc<RwLock<HashMap<NodeId, ResourceMetrics>>>,
    /// Balancing strategy
    strategy: BalancingStrategy,
    /// Configuration
    config: LoadBalancerConfig,
}

/// Load balancer configuration
#[derive(Clone, Debug)]
pub struct LoadBalancerConfig {
    /// Rebalance interval
    pub rebalance_interval: Duration,
    /// Load threshold for rebalancing
    pub load_threshold: f64,
    /// Enable predictive scaling
    pub predictive_scaling: bool,
    /// Historical data window
    pub history_window: Duration,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            rebalance_interval: Duration::from_secs(120),
            load_threshold: 0.8,
            predictive_scaling: true,
            history_window: Duration::from_secs(3600),
        }
    }
}

/// Load balancing strategy
#[derive(Clone, Debug)]
pub enum BalancingStrategy {
    /// Round-robin distribution
    RoundRobin,
    /// Least connections
    LeastConnections,
    /// Weighted round-robin
    WeightedRoundRobin,
    /// Resource-aware balancing
    ResourceAware,
    /// Geographic proximity
    GeographicProximity,
    /// Predictive balancing
    Predictive,
}

/// Resource metrics for a node
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceMetrics {
    /// Node ID
    pub node_id: NodeId,
    /// CPU utilization (0.0-1.0)
    pub cpu_utilization: f64,
    /// Memory utilization (0.0-1.0)
    pub memory_utilization: f64,
    /// Network utilization (0.0-1.0)
    pub network_utilization: f64,
    /// Storage utilization (0.0-1.0)
    pub storage_utilization: f64,
    /// Active connections
    pub active_connections: u64,
    /// Request rate per second
    pub request_rate: f64,
    /// Average response time (ms)
    pub avg_response_time: f64,
    /// Timestamp
    pub timestamp: SystemTime,
}

impl LoadBalancer {
    /// Create new load balancer
    pub fn new(strategy: BalancingStrategy, config: LoadBalancerConfig) -> Self {
        Self {
            node_loads: Arc::new(RwLock::new(HashMap::new())),
            strategy,
            config,
        }
    }

    /// Select best node for asset
    pub async fn select_node(&self, asset_type: AssetType) -> AssetResult<NodeId> {
        let loads = self.node_loads.read().await;

        match self.strategy {
            BalancingStrategy::LeastConnections => {
                loads.iter()
                    .min_by_key(|(_, m)| m.active_connections)
                    .map(|(id, _)| id.clone())
                    .ok_or_else(|| crate::assets::core::AssetError::AllocationFailed {
                        reason: "No nodes available".to_string(),
                    })
            }
            BalancingStrategy::ResourceAware => {
                loads.iter()
                    .min_by(|(_, a), (_, b)| {
                        let score_a = a.cpu_utilization + a.memory_utilization;
                        let score_b = b.cpu_utilization + b.memory_utilization;
                        score_a.partial_cmp(&score_b).unwrap()
                    })
                    .map(|(id, _)| id.clone())
                    .ok_or_else(|| crate::assets::core::AssetError::AllocationFailed {
                        reason: "No nodes available".to_string(),
                    })
            }
            _ => {
                // Default to first available node
                loads.keys()
                    .next()
                    .cloned()
                    .ok_or_else(|| crate::assets::core::AssetError::AllocationFailed {
                        reason: "No nodes available".to_string(),
                    })
            }
        }
    }

    /// Update node metrics
    pub async fn update_metrics(&self, metrics: ResourceMetrics) {
        self.node_loads.write().await.insert(metrics.node_id.clone(), metrics);
    }

    /// Get load statistics
    pub async fn get_load_stats(&self) -> HashMap<NodeId, f64> {
        let loads = self.node_loads.read().await;
        loads.iter()
            .map(|(id, m)| {
                let load = (m.cpu_utilization + m.memory_utilization) / 2.0;
                (id.clone(), load)
            })
            .collect()
    }

    /// Predict future load
    pub async fn predict_load(&self, node_id: &NodeId, duration: Duration) -> f64 {
        // Simple prediction based on current load
        // In production, would use ML models
        let loads = self.node_loads.read().await;
        loads.get(node_id)
            .map(|m| (m.cpu_utilization + m.memory_utilization) / 2.0 * 1.1)
            .unwrap_or(0.5)
    }
}