//! Scheduler configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub scheduling_interval: Duration,
    pub max_concurrent_jobs: usize,
    pub retry_delay: Duration,
    pub node_scoring_strategy: String,
    pub placement: PlacementConfig,
    pub autoscaling: AutoscalingConfig,
    pub prediction: PredictionConfig,
    pub optimization: OptimizationConfig,
    pub policies: PolicyConfig,
    pub monitoring: MonitoringConfig,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            scheduling_interval: Duration::from_secs(10),
            max_concurrent_jobs: 100,
            retry_delay: Duration::from_secs(5),
            node_scoring_strategy: "LeastResourceUsage".to_string(),
            placement: PlacementConfig::default(),
            autoscaling: AutoscalingConfig::default(),
            prediction: PredictionConfig::default(),
            optimization: OptimizationConfig::default(),
            policies: PolicyConfig::default(),
            monitoring: MonitoringConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementConfig {
    pub strategy: String,
}

impl Default for PlacementConfig {
    fn default() -> Self {
        Self { strategy: "BestFit".to_string() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoscalingConfig {
    pub enabled: bool,
    pub evaluation_interval: Duration,
}

impl Default for AutoscalingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            evaluation_interval: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionConfig {
    pub enabled: bool,
    pub window: Duration,
}

impl Default for PredictionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            window: Duration::from_secs(300),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    pub enabled: bool,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub enforce_limits: bool,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self { enforce_limits: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub interval: Duration,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self { interval: Duration::from_secs(5) }
    }
}