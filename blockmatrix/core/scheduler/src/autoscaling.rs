//! Autoscaling module

use serde::{Deserialize, Serialize};
use nexus_shared::ResourceId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoscalingPolicy {
    pub min_replicas: u32,
    pub max_replicas: u32,
    pub target_cpu_utilization: f32,
}

impl Default for AutoscalingPolicy {
    fn default() -> Self {
        Self {
            min_replicas: 1,
            max_replicas: 10,
            target_cpu_utilization: 0.75,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingPolicy {
    pub resource_id: ResourceId,
    pub autoscaling: AutoscalingPolicy,
}

#[derive(Debug)]
pub struct AutoScaler {
    policies: Vec<ScalingPolicy>,
}

impl AutoScaler {
    pub fn new() -> Self {
        Self { policies: Vec::new() }
    }
    
    pub async fn evaluate(&self) -> Vec<ScalingDecision> {
        Vec::new()
    }
    
    pub async fn make_scaling_decisions(&self) -> Vec<ScalingDecision> {
        Vec::new()
    }
    
    pub async fn stats(&self) -> AutoScalingStats {
        AutoScalingStats::default()
    }
}

#[derive(Debug, Clone)]
pub struct ScalingDecision {
    pub resource_id: ResourceId,
    pub target_replicas: u32,
}

#[derive(Debug, Default, Clone)]
pub struct AutoScalingStats {
    pub total_evaluations: u64,
    pub scale_ups: u64,
    pub scale_downs: u64,
}