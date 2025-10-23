//! VM integration with HyperMesh

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMIntegrationConfig {
    pub enabled: bool,
    pub max_concurrent_vms: usize,
}

impl Default for VMIntegrationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_concurrent_vms: 10,
        }
    }
}

pub struct VMIntegrationManager {
    config: VMIntegrationConfig,
    active_vms: HashMap<String, VMInstance>,
}

#[derive(Debug, Clone)]
pub struct VMInstance {
    pub id: String,
    pub status: VMStatus,
}

#[derive(Debug, Clone)]
pub enum VMStatus {
    Starting,
    Running,
    Stopped,
    Error(String),
}

impl VMIntegrationManager {
    pub fn new(config: VMIntegrationConfig) -> Self {
        Self {
            config,
            active_vms: HashMap::new(),
        }
    }
}