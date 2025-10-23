//! Container types for HyperMesh
//!
//! Provides core types for container management.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Container unique identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContainerId(pub String);

impl ContainerId {
    /// Create a new container ID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create from existing string
    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    /// Get the inner string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ContainerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Container specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSpec {
    /// Container name
    pub name: String,
    /// Container image
    pub image: String,
    /// Command to run
    pub command: Option<Vec<String>>,
    /// Arguments for the command
    pub args: Option<Vec<String>>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Resource requirements
    pub resources: ResourceRequirements,
    /// Resource limits
    pub limits: Option<ResourceLimits>,
    /// Labels/metadata
    pub labels: HashMap<String, String>,
}

/// Container status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContainerStatus {
    /// Container is running
    Running,
    /// Container is stopped
    Stopped,
    /// Container failed
    Failed(String),
    /// Container is unknown
    Unknown,
}

/// Container handle for runtime operations
#[derive(Debug, Clone)]
pub struct ContainerHandle {
    /// Container ID
    pub id: ContainerId,
    /// Container spec
    pub spec: ContainerSpec,
    /// Current status
    pub status: ContainerStatus,
}

/// Options for creating a container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOptions {
    /// Container name
    pub name: String,
    /// Container image
    pub image: String,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Resource requirements
    pub resources: ResourceRequirements,
}

/// Container state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContainerState {
    /// Container is being created
    Creating,
    /// Container is running
    Running,
    /// Container is paused
    Paused,
    /// Container is stopped
    Stopped,
    /// Container is being removed
    Removing,
}

/// Resource requirements for a container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU requirements in millicores
    pub cpu_millicores: u64,
    /// Memory requirements in bytes
    pub memory_bytes: u64,
    /// Storage requirements in bytes
    pub storage_bytes: u64,
    /// GPU requirements (optional)
    pub gpu_count: Option<u32>,
}

/// Resource limits for a container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum CPU in millicores
    pub cpu_max: u64,
    /// Maximum memory in bytes
    pub memory_max: u64,
    /// Maximum storage in bytes
    pub storage_max: u64,
}

/// Resource usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Current CPU usage in millicores
    pub cpu_current: u64,
    /// Current memory usage in bytes
    pub memory_current: u64,
    /// Current storage usage in bytes
    pub storage_current: u64,
    /// Timestamp of measurement
    pub timestamp: u64,
}