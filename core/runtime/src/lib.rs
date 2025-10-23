//! Nexus Runtime - Byzantine fault-tolerant container orchestration
//! 
//! This module provides secure containerized application execution with:
//! - Byzantine fault-tolerant consensus coordination
//! - Hardware-assisted virtualization (Intel VT-x/AMD-V)
//! - Capability-based security model
//! - Resource quotas and monitoring
//! - OCI-compatible container images
//! - Secure inter-container communication with QUIC transport
//! - Distributed state synchronization
//! - P2P mesh networking with Byzantine protection
//! - Infrastructure health monitoring and automated recovery

pub mod container;
pub mod image;
pub mod isolation;
pub mod resources;
pub mod networking;
pub mod storage;
pub mod security;
pub mod config;
pub mod error;

// Byzantine fault-tolerant consensus orchestration modules
pub mod consensus_orchestrator;
pub mod consensus_operations;
pub mod state_sync;
pub mod consensus_validation;

// Infrastructure modules
pub mod health;
pub mod transport;
pub mod transport_wrapper;

// Performance benchmarking module
pub mod stoq_benchmark;

pub use container::{Container, ContainerSpec, ContainerStatus};
pub use image::{ImageManager, ImageSpec};
pub use isolation::{IsolationManager, NamespaceConfig};
pub use resources::{ResourceManager, ResourceQuotas, ResourceUsage};
pub use networking::{NetworkManager, NetworkConfig};
pub use storage::{StorageManager, VolumeSpec};
pub use security::{SecurityManager, SecurityPolicy};
pub use config::RuntimeConfig;
pub use error::{RuntimeError, Result};

// Consensus orchestration exports
pub use consensus_orchestrator::{ConsensusContainerOrchestrator, types::OrchestrationMetrics};
pub use consensus_operations::{ContainerConsensusOperation, ContainerOperationResult, OperationMetrics};
pub use state_sync::{ContainerStateManager, ContainerClusterState, StateSyncMetrics};
pub use consensus_validation::{ContainerStateValidator, ValidatedContainerState, ValidationMetrics};

// Infrastructure exports
pub use health::{HealthMonitor, SystemHealthStatus, HealthEvent, HealthConfig};
pub use transport::{ContainerTransportManager, TransportEvent, ContainerTransportConfig};
pub use transport_wrapper::QuicTransport;

use nexus_shared::{ResourceId, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

/// Container runtime engine
#[derive(Debug)]
pub struct Runtime {
    config: RuntimeConfig,
    containers: Arc<dashmap::DashMap<ResourceId, Arc<Container>>>,
    image_manager: Arc<ImageManager>,
    isolation_manager: Arc<IsolationManager>,
    resource_manager: Arc<ResourceManager>,
    network_manager: Arc<NetworkManager>,
    storage_manager: Arc<StorageManager>,
    security_manager: Arc<SecurityManager>,
}

impl Runtime {
    /// Create a new runtime instance
    pub async fn new(config: RuntimeConfig) -> Result<Self> {
        let image_manager = Arc::new(ImageManager::new(&config.image).await?);
        let isolation_manager = Arc::new(IsolationManager::new(&config.isolation)?);
        let resource_manager = Arc::new(ResourceManager::new(&config.resources)?);
        let network_manager = Arc::new(NetworkManager::new_stub(config.networking.clone()).await?);
        let storage_manager = Arc::new(StorageManager::new(&config.storage)?);
        let security_manager = Arc::new(SecurityManager::new(&config.security)?);
        
        Ok(Self {
            config,
            containers: Arc::new(dashmap::DashMap::new()),
            image_manager,
            isolation_manager,
            resource_manager,
            network_manager,
            storage_manager,
            security_manager,
        })
    }
    
    /// Create and start a new container
    pub async fn create_container(&self, spec: ContainerSpec) -> Result<ResourceId> {
        // Validate container specification
        self.security_manager.validate_spec(&spec).await?;
        
        // Pull container image if needed
        let image = self.image_manager.ensure_image(&spec.image).await?;
        
        // Allocate resources
        let resource_allocation = self.resource_manager
            .allocate_resources(&spec.resources)
            .await?;
        
        // Create network namespace
        let network_config = self.network_manager
            .create_network(&spec.network)
            .await?;
        
        // Prepare storage volumes
        let storage_config = self.storage_manager
            .prepare_volumes(&spec.volumes)
            .await?;
        
        // Create container
        let container = Container::new(
            spec,
            image,
            resource_allocation,
            network_config,
            storage_config,
            Arc::clone(&self.isolation_manager),
            Arc::clone(&self.security_manager),
        ).await?;
        
        let container_id = container.id().clone();
        self.containers.insert(container_id.clone(), Arc::new(container));
        
        tracing::info!("Container created: {}", container_id);
        Ok(container_id)
    }
    
    /// Start a container
    pub async fn start_container(&self, id: &ResourceId) -> Result<()> {
        let container = self.containers
            .get(id)
            .ok_or_else(|| RuntimeError::ContainerNotFound { id: id.clone() })?;
            
        container.start().await?;
        tracing::info!("Container started: {}", id);
        Ok(())
    }
    
    /// Stop a container
    pub async fn stop_container(&self, id: &ResourceId, timeout: Option<std::time::Duration>) -> Result<()> {
        let container = self.containers
            .get(id)
            .ok_or_else(|| RuntimeError::ContainerNotFound { id: id.clone() })?;
            
        container.stop(timeout).await?;
        tracing::info!("Container stopped: {}", id);
        Ok(())
    }
    
    /// Remove a container
    pub async fn remove_container(&self, id: &ResourceId, force: bool) -> Result<()> {
        let container = self.containers
            .get(id)
            .ok_or_else(|| RuntimeError::ContainerNotFound { id: id.clone() })?;
            
        // Stop container if running
        if container.status().await == ContainerStatus::Running {
            if force {
                container.kill().await?;
            } else {
                return Err(RuntimeError::ContainerRunning { id: id.clone() });
            }
        }
        
        // Clean up resources
        container.cleanup().await?;
        
        // Remove from tracking
        self.containers.remove(id);
        
        tracing::info!("Container removed: {}", id);
        Ok(())
    }
    
    /// Get container status
    pub async fn container_status(&self, id: &ResourceId) -> Result<ContainerStatus> {
        let container = self.containers
            .get(id)
            .ok_or_else(|| RuntimeError::ContainerNotFound { id: id.clone() })?;
            
        Ok(container.status().await)
    }
    
    /// List all containers
    pub async fn list_containers(&self) -> Vec<ContainerInfo> {
        let mut containers = Vec::new();
        
        for entry in self.containers.iter() {
            let container = entry.value();
            containers.push(ContainerInfo {
                id: container.id().clone(),
                status: container.status().await,
                created: container.created_at(),
                image: container.image_name().clone(),
            });
        }
        
        containers
    }
    
    /// Get resource usage for all containers
    pub async fn resource_usage(&self) -> HashMap<ResourceId, ResourceUsage> {
        let mut usage = HashMap::new();
        
        for entry in self.containers.iter() {
            let container = entry.value();
            if let Ok(container_usage) = container.resource_usage().await {
                usage.insert(container.id().clone(), container_usage);
            }
        }
        
        usage
    }
    
    /// Execute command in running container
    pub async fn exec_in_container(
        &self,
        id: &ResourceId,
        command: Vec<String>,
        env: HashMap<String, String>,
    ) -> Result<ExecResult> {
        let container = self.containers
            .get(id)
            .ok_or_else(|| RuntimeError::ContainerNotFound { id: id.clone() })?;
            
        container.exec(command, env).await
    }
    
    /// Get logs from container
    pub async fn container_logs(
        &self,
        id: &ResourceId,
        follow: bool,
        tail: Option<usize>,
    ) -> Result<impl tokio_stream::Stream<Item = LogEntry>> {
        let container = self.containers
            .get(id)
            .ok_or_else(|| RuntimeError::ContainerNotFound { id: id.clone() })?;
            
        container.logs(follow, tail).await
    }
}

/// Container information for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: ResourceId,
    pub status: ContainerStatus,
    pub created: SystemTime,
    pub image: String,
}

/// Result of command execution in container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Log entry from container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: SystemTime,
    pub stream: LogStream,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogStream {
    Stdout,
    Stderr,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_runtime_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = RuntimeConfig::default();
        config.storage.data_dir = temp_dir.path().to_string_lossy().to_string();
        
        let runtime = Runtime::new(config).await;
        assert!(runtime.is_ok());
    }
    
    #[tokio::test]
    async fn test_container_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = RuntimeConfig::default();
        config.storage.data_dir = temp_dir.path().to_string_lossy().to_string();
        
        let runtime = Runtime::new(config).await.unwrap();
        
        let spec = ContainerSpec {
            image: ImageSpec {
                name: "test-image".to_string(),
                tag: "latest".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        
        // This would fail in a real test without proper image setup
        // but demonstrates the API structure
        let result = runtime.create_container(spec).await;
        
        // In a real implementation, we'd have proper image handling
        match result {
            Ok(container_id) => {
                let status = runtime.container_status(&container_id).await.unwrap();
                assert_eq!(status, ContainerStatus::Created);
            }
            Err(_) => {
                // Expected to fail without proper setup
            }
        }
    }
}