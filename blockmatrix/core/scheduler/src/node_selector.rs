//! Node selection for workload placement

use nexus_shared::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug)]
pub struct NodeSelector {
    node_scores: HashMap<NodeId, f64>,
}

impl NodeSelector {
    pub fn new() -> Self {
        Self {
            node_scores: HashMap::new(),
        }
    }
    
    pub fn select_node(&self, _requirements: &NodeRequirements) -> Option<NodeId> {
        self.node_scores.keys().next().cloned()
    }
    
    pub async fn select_candidates(&self, _workload: &crate::workload::Workload) -> Vec<NodeId> {
        self.node_scores.keys().cloned().collect()
    }
}

#[derive(Debug, Default)]
pub struct NodeRequirements {
    pub cpu_cores: u32,
    pub memory_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeScore {
    pub node_id: NodeId,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionCriteria {
    pub strategy: String,
    pub weights: HashMap<String, f64>,
}