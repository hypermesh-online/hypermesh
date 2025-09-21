//! VM Executor for HyperMesh Asset Integration
//!
//! Executes VMs through HyperMesh asset allocation with consensus validation.
//! Integrates with Catalog for VM provisioning and execution.

pub mod types;
pub mod security;
pub mod execution;

use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};

use crate::config::VmConfig;
use crate::assets::allocation::AssetAllocator;
use crate::transport::StoqTransportLayer;

pub use types::*;
pub use security::*;
pub use execution::*;

/// VM executor for HyperMesh integration
pub struct VmExecutor {
    /// Configuration
    config: VmConfig,

    /// Asset allocator for resource management
    allocator: Arc<AssetAllocator>,

    /// STOQ transport for VM communication
    stoq_transport: Arc<StoqTransportLayer>,

    /// Execution manager
    execution_manager: Arc<ExecutionManager>,

    /// VM templates and images
    vm_registry: Arc<RwLock<VmRegistry>>,

    /// VM execution statistics
    stats: Arc<RwLock<VmStats>>,

    /// Output collector
    output_collector: Arc<OutputCollector>,

    /// Execution monitor
    monitor: Arc<ExecutionMonitor>,
}

impl VmExecutor {
    /// Create new VM executor
    pub async fn new(
        config: VmConfig,
        allocator: Arc<AssetAllocator>,
        stoq_transport: Arc<StoqTransportLayer>,
    ) -> Result<Self> {
        let execution_manager = Arc::new(ExecutionManager::new(
            allocator.clone(),
            config.max_concurrent_vms,
        ));

        let output_collector = Arc::new(OutputCollector::new(
            config.max_output_size,
            config.artifact_path.clone(),
        ));

        let monitor = Arc::new(ExecutionMonitor::new(
            execution_manager.clone(),
            Duration::from_secs(config.monitoring_interval),
        ));

        Ok(Self {
            config,
            allocator,
            stoq_transport,
            execution_manager,
            vm_registry: Arc::new(RwLock::new(VmRegistry::default())),
            stats: Arc::new(RwLock::new(VmStats::default())),
            output_collector,
            monitor,
        })
    }

    /// Initialize the VM executor
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing VM executor");

        // Load VM registry
        self.load_registry().await?;

        // Start execution monitor
        self.monitor.start().await;

        // Initialize STOQ transport handlers
        self.setup_transport_handlers().await?;

        info!("VM executor initialized successfully");
        Ok(())
    }

    /// Execute a VM operation
    pub async fn execute(&self, request: VmExecutionRequest) -> Result<VmExecution> {
        debug!("Executing VM operation: {}", request.operation);

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_executions += 1;
        stats.active_executions += 1;
        drop(stats);

        // Create security context
        let context = SecurityContext::new("system".to_string());

        // Start execution
        let execution_id = self.execution_manager
            .start_execution(&request, &context)
            .await?;

        // Execute based on operation type
        let result = match request.operation.as_str() {
            "run" => self.run_vm(execution_id.clone(), request).await,
            "build" => self.build_vm(execution_id.clone(), request).await,
            "deploy" => self.deploy_vm(execution_id.clone(), request).await,
            _ => Err(anyhow!("Unknown operation: {}", request.operation)),
        };

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.active_executions -= 1;
        if result.is_ok() {
            stats.completed_executions += 1;
        } else {
            stats.failed_executions += 1;
        }
        drop(stats);

        // Get final execution status
        let status = self.execution_manager
            .get_status(&execution_id)
            .await?;

        Ok(VmExecution {
            execution_id,
            vm_id: request.vm_spec.image_id,
            status,
            started_at: std::time::SystemTime::now(),
            completed_at: None,
            resource_usage: ResourceUsage::default(),
            output: None,
            error: result.err().map(|e| e.to_string()),
        })
    }

    /// Run a VM
    async fn run_vm(
        &self,
        execution_id: String,
        request: VmExecutionRequest,
    ) -> Result<()> {
        info!("Running VM: {}", request.vm_spec.image_id);

        // Validate VM image exists
        self.validate_image(&request.vm_spec.image_id).await?;

        // Set up networking
        self.setup_networking(&request.vm_spec.network_config).await?;

        // Mount storage
        self.mount_storage(&request.vm_spec.storage_config).await?;

        // Execute VM based on runtime type
        match request.vm_spec.runtime_config.runtime_type {
            RuntimeType::Docker => self.run_docker(execution_id, request).await,
            RuntimeType::Kvm => self.run_kvm(execution_id, request).await,
            RuntimeType::Wasm => self.run_wasm(execution_id, request).await,
            _ => Err(anyhow!("Runtime not supported")),
        }
    }

    /// Build a VM image
    async fn build_vm(
        &self,
        _execution_id: String,
        request: VmExecutionRequest,
    ) -> Result<()> {
        info!("Building VM image: {}", request.vm_spec.image_id);

        // Register new image
        let entry = VmRegistryEntry {
            image_id: request.vm_spec.image_id.clone(),
            name: format!("vm-{}", request.vm_spec.image_id),
            description: "Custom VM image".to_string(),
            version: "1.0.0".to_string(),
            size_gb: 1.0,
            created_at: std::time::SystemTime::now(),
            tags: vec!["custom".to_string()],
        };

        let mut registry = self.vm_registry.write().await;
        registry.entries.insert(request.vm_spec.image_id.clone(), entry);

        Ok(())
    }

    /// Deploy a VM
    async fn deploy_vm(
        &self,
        _execution_id: String,
        request: VmExecutionRequest,
    ) -> Result<()> {
        info!("Deploying VM: {}", request.vm_spec.image_id);

        // Validate deployment target
        self.validate_deployment(&request).await?;

        // Deploy based on target
        Ok(())
    }

    /// Load VM registry
    async fn load_registry(&self) -> Result<()> {
        debug!("Loading VM registry");

        // In production, this would load from persistent storage
        let mut registry = self.vm_registry.write().await;

        // Add default images
        registry.entries.insert(
            "ubuntu-22.04".to_string(),
            VmRegistryEntry {
                image_id: "ubuntu-22.04".to_string(),
                name: "Ubuntu 22.04 LTS".to_string(),
                description: "Ubuntu 22.04 base image".to_string(),
                version: "22.04".to_string(),
                size_gb: 2.5,
                created_at: std::time::SystemTime::now(),
                tags: vec!["ubuntu".to_string(), "linux".to_string()],
            },
        );

        Ok(())
    }

    /// Set up STOQ transport handlers
    async fn setup_transport_handlers(&self) -> Result<()> {
        debug!("Setting up STOQ transport handlers");
        // Transport handler setup would go here
        Ok(())
    }

    /// Validate VM image exists
    async fn validate_image(&self, image_id: &str) -> Result<()> {
        let registry = self.vm_registry.read().await;
        if registry.entries.contains_key(image_id) {
            Ok(())
        } else {
            Err(anyhow!("VM image not found: {}", image_id))
        }
    }

    /// Set up networking for VM
    async fn setup_networking(&self, _config: &NetworkConfig) -> Result<()> {
        debug!("Setting up VM networking");
        // Network setup would go here
        Ok(())
    }

    /// Mount storage for VM
    async fn mount_storage(&self, _config: &StorageConfig) -> Result<()> {
        debug!("Mounting VM storage");
        // Storage mounting would go here
        Ok(())
    }

    /// Validate deployment configuration
    async fn validate_deployment(&self, _request: &VmExecutionRequest) -> Result<()> {
        debug!("Validating deployment configuration");
        // Deployment validation would go here
        Ok(())
    }

    /// Run Docker container
    async fn run_docker(
        &self,
        execution_id: String,
        request: VmExecutionRequest,
    ) -> Result<()> {
        info!("Running Docker container for execution {}", execution_id);

        // Docker execution logic
        // This would interface with Docker API

        Ok(())
    }

    /// Run KVM virtual machine
    async fn run_kvm(
        &self,
        execution_id: String,
        request: VmExecutionRequest,
    ) -> Result<()> {
        info!("Running KVM VM for execution {}", execution_id);

        // KVM execution logic
        // This would interface with libvirt/QEMU

        Ok(())
    }

    /// Run WebAssembly module
    async fn run_wasm(
        &self,
        execution_id: String,
        request: VmExecutionRequest,
    ) -> Result<()> {
        info!("Running WASM module for execution {}", execution_id);

        // WASM execution logic
        // This would use wasmtime or similar runtime

        Ok(())
    }

    /// Get VM statistics
    pub async fn get_stats(&self) -> VmStats {
        self.stats.read().await.clone()
    }

    /// List available VM images
    pub async fn list_images(&self) -> Vec<VmRegistryEntry> {
        let registry = self.vm_registry.read().await;
        registry.entries.values().cloned().collect()
    }
}

impl Clone for VmExecutor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            allocator: self.allocator.clone(),
            stoq_transport: self.stoq_transport.clone(),
            execution_manager: self.execution_manager.clone(),
            vm_registry: self.vm_registry.clone(),
            stats: self.stats.clone(),
            output_collector: self.output_collector.clone(),
            monitor: self.monitor.clone(),
        }
    }
}