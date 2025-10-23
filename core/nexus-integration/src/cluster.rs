//! Cluster management and membership

use nexus_shared::NodeId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfo {
    pub node_count: u32,
    pub leader_id: Option<NodeId>,
    pub cluster_id: String,
    pub status: ClusterStatus,
    pub members: Vec<ClusterMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterStatus {
    Healthy,
    Degraded,
    Critical,
    Forming,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMember {
    pub node_id: NodeId,
    pub endpoint: String,
    pub status: MemberStatus,
    pub joined_at: chrono::DateTime<chrono::Utc>,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemberStatus {
    Active,
    Suspected,
    Failed,
    Left,
}