//! Scheduling policies

use serde::{Deserialize, Serialize};
use nexus_shared::ResourceId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingPolicy {
    pub priority: i32,
    pub preemptible: bool,
    pub max_retries: u32,
}

impl Default for SchedulingPolicy {
    fn default() -> Self {
        Self {
            priority: 0,
            preemptible: false,
            max_retries: 3,
        }
    }
}

#[derive(Debug)]
pub struct PolicyEngine {
    policies: Vec<SchedulingPolicy>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self { policies: Vec::new() }
    }
    
    pub fn evaluate(&self, _resource_id: &ResourceId) -> bool {
        true
    }
    
    pub async fn apply_policies(&self, _workload: &crate::workload::Workload) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(true)
    }
}

#[derive(Debug, Clone)]
pub struct Constraint {
    pub name: String,
    pub value: f64,
}