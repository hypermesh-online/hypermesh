//! Workload definition and management

use nexus_shared::ResourceId;
use nexus_runtime::resources::ResourceQuotas;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workload {
    pub id: ResourceId,
    pub workload_type: WorkloadType,
    pub priority: i32,
    pub spec: WorkloadSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkloadType {
    Batch,
    Streaming,
    Interactive,
    Background,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadSpec {
    pub id: ResourceId,
    pub name: String,
    pub image: String,
    pub replicas: u32,
    pub resources: ResourceQuotas,
    pub labels: HashMap<String, String>,
    pub workload_type: WorkloadType,
    pub command: Vec<String>,
    pub environment: HashMap<String, String>,
    pub working_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkloadStatus {
    Pending,
    Running,
    Completed,
    Failed,
}