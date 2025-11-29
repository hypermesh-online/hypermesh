//! Workload affinity and anti-affinity rules

use nexus_shared::{ResourceId, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffinityRule {
    pub rule_type: AffinityType,
    pub target: ResourceId,
    pub weight: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AffinityType {
    NodeAffinity,
    PodAffinity,
    AntiAffinity,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AffinityRules {
    pub node_affinity: Vec<NodeAffinity>,
    pub pod_affinity: Vec<PodAffinity>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AntiAffinityRules {
    pub pod_anti_affinity: Vec<PodAffinity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAffinity {
    pub node_selector: HashMap<String, String>,
    pub preferred_nodes: Vec<NodeId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodAffinity {
    pub label_selector: HashMap<String, String>,
    pub topology_key: String,
}