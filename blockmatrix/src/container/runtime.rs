//! Core container runtime implementation

use super::{
    types::{ContainerId, ContainerSpec, ContainerStatus, ContainerState},
    lifecycle::{ContainerLifecycle, DefaultContainerLifecycle},
    image::{ImageManager, DefaultImageManager},
    network::{ContainerNetwork, DefaultContainerNetwork},
    filesystem::{ContainerFilesystem, DefaultContainerFilesystem},
    resources::{ResourceManager, CgroupResourceManager, ResourceUsage},
    migration::{MigrationManager, DefaultMigrationManager, MigrationRequest},
    monitoring::{ContainerMonitor, DefaultContainerMonitor, ContainerMetrics},
    config::ContainerConfig,
    error::{Result, ContainerError},
    types::CreateOptions,
};

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn, instrument};

/// Container runtime handle for managing container operations
#[derive(Clone)]
pub struct ContainerHandle {
    /// Container ID
    pub id: ContainerId,
    /// Container specification
    pub spec: ContainerSpec,
    /// Creation options
    pub options: CreateOptions,
    /// Creation timestamp
    pub created_at: Instant,
    /// Reference to runtime
    runtime: Arc<ContainerRuntime>,
}

impl ContainerHandle {
    /// Get container status
    pub async fn status(&self) -> Result<ContainerStatus> {
        self.runtime.status(self.id).await
    }
    
    /// Start the container
    pub async fn start(&self) -> Result<()> {
        self.runtime.start(self.id).await
    }
    
    /// Stop the container
    pub async fn stop(&self, timeout: Option<Duration>) -> Result<()> {
        self.runtime.stop(self.id, timeout).await
    }
    
    /// Pause the container
    pub async fn pause(&self) -> Result<()> {
        self.runtime.pause(self.id).await
    }
    
    /// Resume the container
    pub async fn resume(&self) -> Result<()> {
        self.runtime.resume(self.id).await
    }
    
    /// Delete the container
    pub async fn delete(&self) -> Result<()> {
        self.runtime.delete(self.id).await
    }
    
    /// Get resource usage
    pub async fn usage(&self) -> Result<ResourceUsage> {
        self.runtime.get_usage(self.id).await
    }
    
    /// Get metrics
    pub async fn metrics(&self) -> Result<ContainerMetrics> {
        self.runtime.get_metrics(self.id).await
    }
    
    /// Migrate container to another node
    pub async fn migrate(&self, request: MigrationRequest) -> Result<()> {
        self.runtime.migrate(request).await
    }
}

/// Main container runtime implementation
pub struct ContainerRuntime {
    /// Runtime configuration
    config: ContainerConfig,
    /// Container lifecycle manager
    lifecycle: Arc<dyn ContainerLifecycle>,
    /// Image manager
    image_manager: Arc<dyn ImageManager>,
    /// Network manager
    network_usage: Arc<dyn ContainerNetwork>,
    /// Filesystem manager
    filesystem: Arc<dyn ContainerFilesystem>,
    /// Resource manager
    resource_manager: Arc<dyn ResourceManager>,
    /// Migration manager
    migration_manager: Arc<dyn MigrationManager>,
    /// Container monitor
    monitor: Arc<dyn ContainerMonitor>,
    /// Container registry (handles to created containers)
    containers: Arc<RwLock<HashMap<ContainerId, ContainerHandle>>>,
    /// Performance metrics
    metrics: Arc<RwLock<RuntimeMetrics>>,
}

/// Runtime performance metrics
#[derive(Debug, Default)]
pub struct RuntimeMetrics {
    /// Total containers created
    pub containers_created: u64,
    /// Total containers started
    pub containers_started: u64,
    /// Total containers stopped
    pub containers_stopped: u64,
    /// Average startup time
    pub avg_startup_time: Duration,
    /// Average shutdown time
    pub avg_shutdown_time: Duration,
    /// Current running containers
    pub running_containers: u32,
    /// Resource utilization
    pub resource_utilization: f64,
}

impl ContainerRuntime {
    /// Create a new container runtime with default components
    pub async fn new(config: ContainerConfig) -> Result<Self> {
        let lifecycle = Arc::new(DefaultContainerLifecycle::new());
        let image_manager = Arc::new(DefaultImageManager::new(&config.storage_usage)?);
        let network = Arc::new(DefaultContainerNetwork::new());
        let filesystem = Arc::new(DefaultContainerFilesystem::new(&config.storage_usage)?);
        let resource_manager = Arc::new(CgroupResourceManager::new());
        let migration_manager = Arc::new(DefaultMigrationManager::new());
        let monitor = Arc::new(DefaultContainerMonitor::new());
        
        Ok(Self {
            config,
            lifecycle,
            image_manager,
            network_usage: network,
            filesystem,
            resource_manager,
            migration_manager,
            monitor,
            containers: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(RuntimeMetrics::default())),
        })
    }
    
    /// Create a container runtime with custom components
    pub fn with_components(
        config: ContainerConfig,
        lifecycle: Arc<dyn ContainerLifecycle>,
        image_manager: Arc<dyn ImageManager>,
        network: Arc<dyn ContainerNetwork>,
        filesystem: Arc<dyn ContainerFilesystem>,
        resource_manager: Arc<dyn ResourceManager>,
        migration_manager: Arc<dyn MigrationManager>,
        monitor: Arc<dyn ContainerMonitor>,
    ) -> Self {
        Self {
            config,
            lifecycle,
            image_manager,
            network_usage: network,
            filesystem,
            resource_manager,
            migration_manager,
            monitor,
            containers: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(RuntimeMetrics::default())),
        }
    }
    
    /// Get runtime configuration
    pub fn config(&self) -> &ContainerConfig {
        &self.config
    }
    
    /// Get runtime metrics
    pub async fn metrics(&self) -> RuntimeMetrics {
        self.metrics.read().await.clone()
    }
    
    /// Update runtime metrics
    async fn update_metrics<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut RuntimeMetrics),
    {
        let mut metrics = self.metrics.write().await;
        update_fn(&mut *metrics);
    }
    
    /// Validate container specification
    fn validate_spec(&self, spec: &ContainerSpec) -> Result<()> {
        if spec.image.is_empty() {
            return Err(ContainerError::config("Container image cannot be empty"));
        }
        
        if spec.command.is_empty() {
            return Err(ContainerError::config("Container command cannot be empty"));
        }
        
        // Validate resource limits
        if let Some(memory) = spec.resources.memory_limit {
            if memory > self.config.limits.max_memory_per_container {
                return Err(ContainerError::config(
                    format!("Memory limit {} exceeds maximum {}", 
                           memory, self.config.limits.max_memory_per_container)
                ));
            }
        }
        
        if let Some(cpu) = spec.resources.cpu_quota {
            if cpu > self.config.limits.max_cpu_per_container {
                return Err(ContainerError::config(
                    format!("CPU quota {} exceeds maximum {}", 
                           cpu, self.config.limits.max_cpu_per_container)
                ));
            }
        }
        
        Ok(())
    }
    
    /// Create a new container
    #[instrument(skip(self), fields(image = %spec.image))]
    pub async fn create(&self, spec: ContainerSpec, options: CreateOptions) -> Result<ContainerHandle> {
        let start_time = Instant::now();
        
        // Validate specification
        self.validate_spec(&spec)?;
        
        // Generate container ID
        let id = ContainerId::new();
        
        // Check if we've reached container limit
        let containers = self.containers.read().await;
        if containers.len() >= self.config.runtime.max_containers as usize {
            return Err(ContainerError::resource("Maximum container limit reached"));
        }
        drop(containers);
        
        // Pull image if necessary
        if !self.image_manager.exists(&spec.image).await? {
            info!("Pulling image: {}", spec.image);
            self.image_manager.pull(&spec.image).await?;
        }
        
        // Create container filesystem
        self.filesystem.create_container_filesystem(id, &spec).await?;
        
        // Set up networking - spec doesn't have network field, use defaults
        self.network_usage.create_network_namespace(id, &NetworkConfig::default()).await?;
        
        // Configure resource limits
        self.resource_manager.set_quota(id, spec.resources.clone()).await?;
        
        // Initialize container lifecycle
        self.lifecycle.create(id, spec.clone()).await?;
        
        // Create container handle
        let handle = ContainerHandle {
            id,
            spec: spec.clone(),
            options: options.clone(),
            created_at: Instant::now(),
            runtime: Arc::new(self.clone()),
        };
        
        // Register container
        let mut containers = self.containers.write().await;
        containers.insert(id, handle.clone());
        drop(containers);
        
        // Update metrics
        let creation_time = start_time.elapsed();
        self.update_metrics(|metrics| {
            metrics.containers_created += 1;
        }).await;
        
        info!("Created container {} in {:?}", id, creation_time);

        // CreateOptions doesn't have auto_start field
        // Containers need to be started manually after creation

        Ok(handle)
    }
    
    /// Start a container
    #[instrument(skip(self))]
    pub async fn start(&self, id: ContainerId) -> Result<()> {
        let start_time = Instant::now();
        
        // Start container lifecycle
        self.lifecycle.start(id).await?;
        
        // Start monitoring
        self.monitor.start_monitoring(id).await?;
        
        // Update metrics
        let startup_time = start_time.elapsed();
        self.update_metrics(|metrics| {
            metrics.containers_started += 1;
            metrics.running_containers += 1;
            
            // Update average startup time
            let total_time = metrics.avg_startup_time.as_nanos() * (metrics.containers_started - 1) as u128
                + startup_time.as_nanos();
            metrics.avg_startup_time = Duration::from_nanos((total_time / metrics.containers_started as u128) as u64);
        }).await;
        
        info!("Started container {} in {:?}", id, startup_time);
        Ok(())
    }
    
    /// Stop a container
    #[instrument(skip(self))]
    pub async fn stop(&self, id: ContainerId, timeout: Option<Duration>) -> Result<()> {
        let start_time = Instant::now();
        
        // Stop container lifecycle
        self.lifecycle.stop(id, timeout).await?;
        
        // Stop monitoring
        self.monitor.stop_monitoring(id).await?;
        
        // Update metrics
        let shutdown_time = start_time.elapsed();
        self.update_metrics(|metrics| {
            metrics.containers_stopped += 1;
            metrics.running_containers = metrics.running_containers.saturating_sub(1);
            
            // Update average shutdown time
            let total_time = metrics.avg_shutdown_time.as_nanos() * (metrics.containers_stopped - 1) as u128
                + shutdown_time.as_nanos();
            metrics.avg_shutdown_time = Duration::from_nanos((total_time / metrics.containers_stopped as u128) as u64);
        }).await;
        
        info!("Stopped container {} in {:?}", id, shutdown_time);
        Ok(())
    }
    
    /// Pause a container
    #[instrument(skip(self))]
    pub async fn pause(&self, id: ContainerId) -> Result<()> {
        self.lifecycle.pause(id).await?;
        info!("Paused container {}", id);
        Ok(())
    }
    
    /// Resume a container
    #[instrument(skip(self))]
    pub async fn resume(&self, id: ContainerId) -> Result<()> {
        self.lifecycle.resume(id).await?;
        info!("Resumed container {}", id);
        Ok(())
    }
    
    /// Delete a container
    #[instrument(skip(self))]
    pub async fn delete(&self, id: ContainerId) -> Result<()> {
        // Remove from container registry
        let mut containers = self.containers.write().await;
        let handle = containers.remove(&id)
            .ok_or_else(|| ContainerError::NotFound { id: id.to_string() })?;
        drop(containers);
        
        // Ensure container is stopped
        if let Ok(status) = self.lifecycle.status(id).await {
            if status.state == ContainerState::Running {
                self.stop(id, None).await?;
            }
        }
        
        // Delete container lifecycle
        self.lifecycle.delete(id).await?;
        
        // Cleanup resources
        self.resource_manager.cleanup(id).await?;
        
        // Cleanup networking
        self.network_usage.delete_network_namespace(id).await?;
        
        // Cleanup filesystem
        self.filesystem.delete_container_filesystem(id).await?;
        
        // Auto-remove handling
        info!("Deleted container {}", id);
        Ok(())
    }
    
    /// Get container status
    pub async fn status(&self, id: ContainerId) -> Result<ContainerStatus> {
        self.lifecycle.status(id).await
    }
    
    /// List all containers
    pub async fn list(&self) -> Result<Vec<ContainerId>> {
        let containers = self.containers.read().await;
        Ok(containers.keys().copied().collect())
    }
    
    /// Get container handle by ID
    pub async fn get_handle(&self, id: ContainerId) -> Result<ContainerHandle> {
        let containers = self.containers.read().await;
        containers.get(&id)
            .cloned()
            .ok_or_else(|| ContainerError::NotFound { id: id.to_string() })
    }
    
    /// Get resource usage for container
    pub async fn get_usage(&self, id: ContainerId) -> Result<ResourceUsage> {
        self.resource_manager.get_usage(id).await
    }
    
    /// Get metrics for container
    pub async fn get_metrics(&self, id: ContainerId) -> Result<ContainerMetrics> {
        self.monitor.get_metrics(id).await
    }
    
    /// Migrate a container
    pub async fn migrate(&self, request: MigrationRequest) -> Result<()> {
        let _result = self.migration_manager.migrate(request).await?;
        Ok(())
    }
    
    /// Checkpoint a container
    pub async fn checkpoint(&self, id: ContainerId, path: &str) -> Result<()> {
        self.lifecycle.checkpoint(id, path).await?;
        info!("Created checkpoint for container {} at {}", id, path);
        Ok(())
    }
    
    /// Restore a container from checkpoint
    pub async fn restore(&self, id: ContainerId, path: &str) -> Result<()> {
        self.lifecycle.restore(id, path).await?;
        info!("Restored container {} from checkpoint at {}", id, path);
        Ok(())
    }
    
    /// Shutdown the runtime
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down container runtime");
        
        // Stop all running containers
        let containers = self.containers.read().await;
        let container_ids: Vec<_> = containers.keys().copied().collect();
        drop(containers);
        
        for id in container_ids {
            if let Ok(status) = self.status(id).await {
                match status {
                    ContainerStatus::Running => {
                        warn!("Force stopping container {} during shutdown", id);
                        let _ = self.stop(id, Some(Duration::from_secs(5))).await;
                    },
                    _ => {},
                }
            }
        }
        
        info!("Container runtime shutdown complete");
        Ok(())
    }
}

// Clone implementation for Arc sharing
impl Clone for ContainerRuntime {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            lifecycle: Arc::clone(&self.lifecycle),
            image_manager: Arc::clone(&self.image_manager),
            network_usage: Arc::clone(&self.network),
            filesystem: Arc::clone(&self.filesystem),
            resource_manager: Arc::clone(&self.resource_manager),
            migration_manager: Arc::clone(&self.migration_manager),
            monitor: Arc::clone(&self.monitor),
            containers: Arc::clone(&self.containers),
            metrics: Arc::clone(&self.metrics),
        }
    }
}

impl Clone for RuntimeMetrics {
    fn clone(&self) -> Self {
        Self {
            containers_created: self.containers_created,
            containers_started: self.containers_started,
            containers_stopped: self.containers_stopped,
            avg_startup_time: self.avg_startup_time,
            avg_shutdown_time: self.avg_shutdown_time,
            running_containers: self.running_containers,
            resource_utilization: self.resource_utilization,
        }
    }
}