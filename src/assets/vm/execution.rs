//! VM Execution Management
//!
//! Handles VM lifecycle, execution monitoring, and output collection.

use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use uuid::Uuid;

use crate::assets::vm::types::*;
use crate::assets::vm::security::{SecurityContext, SecurityValidator, SecurityAuditEntry};
use crate::assets::allocation::{AssetAllocator, AllocationRequest, AllocationPriority, RequesterType};

/// VM execution manager
pub struct ExecutionManager {
    /// Active executions
    executions: Arc<RwLock<HashMap<String, Arc<VmExecution>>>>,

    /// Asset allocator
    allocator: Arc<AssetAllocator>,

    /// Execution history
    history: Arc<RwLock<Vec<Arc<VmExecution>>>>,

    /// Maximum concurrent executions
    max_concurrent: usize,
}

impl ExecutionManager {
    /// Create new execution manager
    pub fn new(allocator: Arc<AssetAllocator>, max_concurrent: usize) -> Self {
        Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
            allocator,
            history: Arc::new(RwLock::new(Vec::new())),
            max_concurrent,
        }
    }

    /// Start VM execution
    pub async fn start_execution(
        &self,
        request: &VmExecutionRequest,
        context: &SecurityContext,
    ) -> Result<String> {
        // Check concurrent execution limit
        let executions = self.executions.read().await;
        if executions.len() >= self.max_concurrent {
            return Err(anyhow!("Maximum concurrent executions reached"));
        }
        drop(executions);

        // Generate execution ID
        let execution_id = Uuid::new_v4().to_string();

        // Validate security configuration
        let validation = SecurityValidator::validate(&request.security_config);
        if !validation.is_valid {
            let violations = validation.violations.iter()
                .map(|v| format!("{}: {}", v.policy, v.description))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(anyhow!("Security validation failed: {}", violations));
        }

        // Log warnings
        for warning in validation.warnings {
            warn!("Security warning for execution {}: {}", execution_id, warning);
        }

        // Allocate resources
        let allocation = self.allocate_resources(request).await?;

        // Create execution record
        let execution = Arc::new(VmExecution {
            execution_id: execution_id.clone(),
            vm_id: request.vm_spec.image_id.clone(),
            status: VmStatus::Starting,
            started_at: SystemTime::now(),
            completed_at: None,
            resource_usage: ResourceUsage::default(),
            output: None,
            error: None,
        });

        // Store execution
        let mut executions = self.executions.write().await;
        executions.insert(execution_id.clone(), execution.clone());

        // Create audit entry
        let mut audit = SecurityAuditEntry::new(
            "start_execution".to_string(),
            context.user_id.clone(),
            request.vm_spec.image_id.clone(),
            "success".to_string(),
        );
        audit.add_detail("execution_id".to_string(), execution_id.clone());
        audit.add_detail("allocation_id".to_string(), allocation);

        info!("Started VM execution {}", execution_id);
        Ok(execution_id)
    }

    /// Allocate resources for VM execution
    async fn allocate_resources(&self, request: &VmExecutionRequest) -> Result<String> {
        let priority = match request.execution_params.priority {
            ExecutionPriority::Low => AllocationPriority::Low,
            ExecutionPriority::Normal => AllocationPriority::Normal,
            ExecutionPriority::High => AllocationPriority::High,
            ExecutionPriority::Critical => AllocationPriority::Critical,
        };

        let allocation_request = AllocationRequest {
            requester_id: request.vm_spec.image_id.clone(),
            requester_type: RequesterType::VmExecution,
            cpu_cores: request.resource_requirements.cpu.cores,
            memory_gb: request.resource_requirements.memory.size_gb,
            storage_gb: request.resource_requirements.storage
                .as_ref()
                .map(|s| s.size_gb),
            gpu_count: request.resource_requirements.gpu
                .as_ref()
                .map(|g| g.count),
            bandwidth_gbps: request.resource_requirements.network
                .as_ref()
                .map(|n| n.bandwidth_gbps),
            duration: request.execution_params.timeout
                .unwrap_or(Duration::from_secs(3600)),
            priority,
            constraints: None,
        };

        self.allocator.request_allocation(&allocation_request).await
    }

    /// Stop VM execution
    pub async fn stop_execution(&self, execution_id: &str) -> Result<()> {
        let mut executions = self.executions.write().await;

        if let Some(execution) = executions.get_mut(execution_id) {
            let mut exec = Arc::make_mut(execution);
            exec.status = VmStatus::Stopping;
            exec.completed_at = Some(SystemTime::now());

            info!("Stopping VM execution {}", execution_id);

            // Move to history
            let history_exec = execution.clone();
            drop(executions);

            let mut history = self.history.write().await;
            history.push(history_exec);

            // Remove from active
            let mut executions = self.executions.write().await;
            executions.remove(execution_id);

            Ok(())
        } else {
            Err(anyhow!("Execution not found: {}", execution_id))
        }
    }

    /// Get execution status
    pub async fn get_status(&self, execution_id: &str) -> Result<VmStatus> {
        let executions = self.executions.read().await;

        if let Some(execution) = executions.get(execution_id) {
            Ok(execution.status.clone())
        } else {
            // Check history
            let history = self.history.read().await;
            for exec in history.iter() {
                if exec.execution_id == execution_id {
                    return Ok(exec.status.clone());
                }
            }
            Err(anyhow!("Execution not found: {}", execution_id))
        }
    }

    /// Get active executions
    pub async fn get_active_executions(&self) -> Vec<Arc<VmExecution>> {
        let executions = self.executions.read().await;
        executions.values().cloned().collect()
    }

    /// Clean up completed executions
    pub async fn cleanup_completed(&self, age: Duration) -> Result<usize> {
        let cutoff = SystemTime::now() - age;
        let mut history = self.history.write().await;
        let initial_len = history.len();

        history.retain(|exec| {
            exec.completed_at
                .map(|completed| completed > cutoff)
                .unwrap_or(true)
        });

        let removed = initial_len - history.len();
        if removed > 0 {
            info!("Cleaned up {} completed executions", removed);
        }

        Ok(removed)
    }
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            cpu_seconds: 0.0,
            memory_gb_hours: 0.0,
            storage_gb_hours: 0.0,
            network_gb: 0.0,
            gpu_hours: 0.0,
        }
    }
}

/// Execution monitor for tracking resource usage
pub struct ExecutionMonitor {
    /// Execution manager reference
    manager: Arc<ExecutionManager>,

    /// Monitoring interval
    interval: Duration,
}

impl ExecutionMonitor {
    /// Create new execution monitor
    pub fn new(manager: Arc<ExecutionManager>, interval: Duration) -> Self {
        Self { manager, interval }
    }

    /// Start monitoring loop
    pub async fn start(&self) {
        let manager = self.manager.clone();
        let interval = self.interval;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                ticker.tick().await;

                // Update resource usage for active executions
                let executions = manager.get_active_executions().await;

                for execution in executions {
                    debug!("Monitoring execution {}", execution.execution_id);
                    // Resource monitoring would happen here
                }

                // Clean up old completed executions
                let _ = manager.cleanup_completed(Duration::from_secs(86400)).await;
            }
        });
    }
}

/// Execution output collector
pub struct OutputCollector {
    /// Output buffer size limit
    max_output_size: usize,

    /// Artifact storage path
    artifact_path: String,
}

impl OutputCollector {
    /// Create new output collector
    pub fn new(max_output_size: usize, artifact_path: String) -> Self {
        Self {
            max_output_size,
            artifact_path,
        }
    }

    /// Collect execution output
    pub async fn collect(
        &self,
        execution_id: &str,
        stdout: String,
        stderr: String,
        exit_code: i32,
    ) -> ExecutionOutput {
        // Truncate output if needed
        let stdout = if stdout.len() > self.max_output_size {
            format!(
                "{}... (truncated, {} bytes total)",
                &stdout[..self.max_output_size],
                stdout.len()
            )
        } else {
            stdout
        };

        let stderr = if stderr.len() > self.max_output_size {
            format!(
                "{}... (truncated, {} bytes total)",
                &stderr[..self.max_output_size],
                stderr.len()
            )
        } else {
            stderr
        };

        // Collect artifacts
        let artifacts = self.collect_artifacts(execution_id).await;

        ExecutionOutput {
            stdout,
            stderr,
            exit_code,
            artifacts,
        }
    }

    /// Collect execution artifacts
    async fn collect_artifacts(&self, execution_id: &str) -> Vec<String> {
        // In a real implementation, this would scan the artifact directory
        vec![format!("{}/{}", self.artifact_path, execution_id)]
    }
}