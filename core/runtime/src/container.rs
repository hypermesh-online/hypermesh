//! Container implementation and management

use crate::{Result, RuntimeError};
use crate::{ImageSpec, IsolationManager, SecurityManager};
use crate::resources::{ResourceQuotas, ResourceUsage, ResourceAllocation};
use crate::networking::NetworkConfig;
use crate::config::StorageConfig;
use nexus_shared::ResourceId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::process::{Child, Command};
use tokio::sync::{RwLock, mpsc};

/// Container specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSpec {
    /// Container ID
    pub id: ResourceId,
    
    /// Container image specification
    pub image: ImageSpec,
    
    /// Command to run in container
    pub command: Vec<String>,
    
    /// Environment variables
    pub environment: HashMap<String, String>,
    
    /// Working directory
    pub working_dir: Option<String>,
    
    /// Resource quotas
    pub resources: ResourceQuotas,
    
    /// Network configuration
    pub network: NetworkConfig,
    
    /// Volume specifications
    pub volumes: Vec<VolumeMount>,
    
    /// Security configuration
    pub security: ContainerSecurityConfig,
    
    /// Labels for container
    pub labels: HashMap<String, String>,
    
    /// Container restart policy
    pub restart_policy: RestartPolicy,
}

impl Default for ContainerSpec {
    fn default() -> Self {
        Self {
            id: ResourceId::new("default", "container", "container"),
            image: ImageSpec::default(),
            command: vec!["/bin/sh".to_string()],
            environment: HashMap::new(),
            working_dir: None,
            resources: ResourceQuotas::default(),
            network: NetworkConfig::default(),
            volumes: Vec::new(),
            security: ContainerSecurityConfig::default(),
            labels: HashMap::new(),
            restart_policy: RestartPolicy::Never,
        }
    }
}

/// Volume mount specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    /// Source path on host
    pub source: String,
    
    /// Target path in container
    pub target: String,
    
    /// Mount options
    pub options: Vec<String>,
    
    /// Read-only mount
    pub readonly: bool,
}

/// Container security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSecurityConfig {
    /// Run as user ID
    pub user_id: Option<u32>,
    
    /// Run as group ID
    pub group_id: Option<u32>,
    
    /// Additional group IDs
    pub supplementary_groups: Vec<u32>,
    
    /// Capabilities to add
    pub capabilities_add: Vec<String>,
    
    /// Capabilities to drop
    pub capabilities_drop: Vec<String>,
    
    /// Enable privileged mode
    pub privileged: bool,
    
    /// Read-only root filesystem
    pub readonly_rootfs: bool,
    
    /// SELinux options
    pub selinux_options: HashMap<String, String>,
}

impl Default for ContainerSecurityConfig {
    fn default() -> Self {
        Self {
            user_id: None,
            group_id: None,
            supplementary_groups: Vec::new(),
            capabilities_add: Vec::new(),
            capabilities_drop: vec![
                "ALL".to_string(),
            ],
            privileged: false,
            readonly_rootfs: false,
            selinux_options: HashMap::new(),
        }
    }
}

/// Container restart policy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RestartPolicy {
    /// Never restart
    Never,
    /// Always restart
    Always,
    /// Restart on failure
    OnFailure,
    /// Restart unless stopped
    UnlessStopped,
}

/// Container status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContainerStatus {
    /// Container created but not started
    Created,
    /// Container is running
    Running,
    /// Container is paused
    Paused,
    /// Container stopped normally
    Stopped,
    /// Container exited with error
    Failed,
    /// Container is being removed
    Removing,
}

/// Container implementation
#[derive(Debug)]
pub struct Container {
    spec: ContainerSpec,
    status: Arc<RwLock<ContainerStatus>>,
    created_at: SystemTime,
    started_at: Option<SystemTime>,
    finished_at: Option<SystemTime>,
    exit_code: Arc<RwLock<Option<i32>>>,
    
    // Process handle
    process: Arc<RwLock<Option<Child>>>,
    
    // Resource management
    resource_allocation: ResourceAllocation,
    
    // Managers
    isolation_manager: Arc<IsolationManager>,
    security_manager: Arc<SecurityManager>,
    
    // Container filesystem root
    rootfs_path: PathBuf,
    
    // Log channels
    log_sender: mpsc::UnboundedSender<crate::LogEntry>,
    log_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<crate::LogEntry>>>>,
}

impl Container {
    /// Create a new container instance
    pub async fn new(
        spec: ContainerSpec,
        image: Arc<crate::image::Image>,
        resource_allocation: ResourceAllocation,
        network_config: NetworkConfig,
        storage_config: StorageConfig,
        isolation_manager: Arc<IsolationManager>,
        security_manager: Arc<SecurityManager>,
    ) -> Result<Self> {
        let created_at = SystemTime::now();
        let (log_sender, log_receiver) = mpsc::unbounded_channel();
        
        // Create container root filesystem
        let rootfs_path = PathBuf::from(storage_config.rootfs_path.clone());
        
        // Extract image to rootfs
        image.as_ref().extract_to(&rootfs_path).await?;
        
        Ok(Self {
            spec,
            status: Arc::new(RwLock::new(ContainerStatus::Created)),
            created_at,
            started_at: None,
            finished_at: None,
            exit_code: Arc::new(RwLock::new(None)),
            process: Arc::new(RwLock::new(None)),
            resource_allocation,
            isolation_manager,
            security_manager,
            rootfs_path,
            log_sender,
            log_receiver: Arc::new(RwLock::new(Some(log_receiver))),
        })
    }
    
    /// Get container ID
    pub fn id(&self) -> &ResourceId {
        &self.spec.id
    }
    
    /// Get container creation time
    pub fn created_at(&self) -> SystemTime {
        self.created_at
    }
    
    /// Get image name
    pub fn image_name(&self) -> &String {
        &self.spec.image.name
    }
    
    /// Get current container status
    pub async fn status(&self) -> ContainerStatus {
        self.status.read().await.clone()
    }
    
    /// Start the container
    pub async fn start(&self) -> Result<()> {
        let mut status = self.status.write().await;
        
        if *status != ContainerStatus::Created {
            return Err(RuntimeError::ContainerRunning { id: self.spec.id.clone() });
        }
        
        // Create namespaces and isolation
        let namespace_config = self.isolation_manager
            .create_namespaces(&self.spec.id)
            .await?;
        
        // Apply security policies
        self.security_manager
            .apply_security_policy(&self.spec.id, &self.spec.security)
            .await?;
        
        // Set up resource limits
        self.isolation_manager
            .apply_resource_limits(&self.spec.id, &self.spec.resources)
            .await?;
        
        // Prepare container command
        let mut command = Command::new(&self.spec.command[0]);
        if self.spec.command.len() > 1 {
            command.args(&self.spec.command[1..]);
        }
        
        // Set environment
        command.envs(&self.spec.environment);
        
        // Set working directory
        if let Some(ref wd) = self.spec.working_dir {
            command.current_dir(wd);
        } else {
            command.current_dir("/");
        }
        
        // Configure stdio
        command.stdin(Stdio::null());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        
        // TODO: Apply namespace configuration to command
        // This would require platform-specific implementation
        
        // Start the process
        let child = command.spawn()
            .map_err(|e| RuntimeError::ProcessExecution {
                command: self.spec.command.join(" "),
                exit_code: -1,
            })?;
        
        *self.process.write().await = Some(child);
        *status = ContainerStatus::Running;
        
        tracing::info!("Container started: {}", self.spec.id);
        Ok(())
    }
    
    /// Stop the container gracefully
    pub async fn stop(&self, timeout: Option<std::time::Duration>) -> Result<()> {
        let mut status = self.status.write().await;
        
        if *status != ContainerStatus::Running {
            return Err(RuntimeError::ContainerNotRunning { id: self.spec.id.clone() });
        }
        
        let mut process_guard = self.process.write().await;
        if let Some(ref mut child) = *process_guard {
            // Send SIGTERM
            #[cfg(unix)]
            {
                if let Some(pid) = child.id() {
                    unsafe {
                        // Use libc directly for signal handling
                        libc::kill(pid as i32, libc::SIGTERM);
                    }
                }
            }
            
            // Wait for graceful shutdown with timeout
            let timeout_duration = timeout.unwrap_or(std::time::Duration::from_secs(10));
            let result = tokio::time::timeout(timeout_duration, child.wait()).await;
            
            match result {
                Ok(Ok(exit_status)) => {
                    *self.exit_code.write().await = Some(exit_status.code().unwrap_or(-1));
                    *status = if exit_status.success() {
                        ContainerStatus::Stopped
                    } else {
                        ContainerStatus::Failed
                    };
                }
                Ok(Err(e)) => {
                    return Err(RuntimeError::ProcessExecution {
                        command: self.spec.command.join(" "),
                        exit_code: -1,
                    });
                }
                Err(_) => {
                    // Timeout - force kill
                    let _ = child.kill().await;
                    *status = ContainerStatus::Failed;
                }
            }
        }
        
        *process_guard = None;
        tracing::info!("Container stopped: {}", self.spec.id);
        Ok(())
    }
    
    /// Kill the container immediately
    pub async fn kill(&self) -> Result<()> {
        let mut status = self.status.write().await;
        let mut process_guard = self.process.write().await;
        
        if let Some(ref mut child) = *process_guard {
            child.kill().await.map_err(|e| RuntimeError::ProcessExecution {
                command: self.spec.command.join(" "),
                exit_code: -1,
            })?;
        }
        
        *process_guard = None;
        *status = ContainerStatus::Failed;
        
        tracing::info!("Container killed: {}", self.spec.id);
        Ok(())
    }
    
    /// Clean up container resources
    pub async fn cleanup(&self) -> Result<()> {
        // Clean up namespaces
        self.isolation_manager
            .cleanup_namespaces(&self.spec.id)
            .await?;
        
        // Clean up security policies
        self.security_manager
            .cleanup_security_policy(&self.spec.id)
            .await?;
        
        // Clean up filesystem
        if self.rootfs_path.exists() {
            tokio::fs::remove_dir_all(&self.rootfs_path).await
                .map_err(|e| RuntimeError::Storage {
                    message: format!("Failed to clean up rootfs: {}", e),
                })?;
        }
        
        tracing::info!("Container cleaned up: {}", self.spec.id);
        Ok(())
    }
    
    /// Get resource usage
    pub async fn resource_usage(&self) -> Result<ResourceUsage> {
        self.isolation_manager
            .get_resource_usage(&self.spec.id)
            .await
    }
    
    /// Execute command in container
    pub async fn exec(
        &self,
        command: Vec<String>,
        env: HashMap<String, String>,
    ) -> Result<crate::ExecResult> {
        let status = self.status.read().await;
        if *status != ContainerStatus::Running {
            return Err(RuntimeError::ContainerNotRunning { id: self.spec.id.clone() });
        }
        
        // Create command in container namespace
        let mut cmd = Command::new(&command[0]);
        if command.len() > 1 {
            cmd.args(&command[1..]);
        }
        
        // Set environment
        cmd.envs(&env);
        cmd.envs(&self.spec.environment);
        
        // Configure stdio
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        // TODO: Enter container namespace before execution
        
        let output = cmd.output().await
            .map_err(|e| RuntimeError::ProcessExecution {
                command: command.join(" "),
                exit_code: -1,
            })?;
        
        Ok(crate::ExecResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
    
    /// Get container logs
    pub async fn logs(
        &self,
        _follow: bool,
        _tail: Option<usize>,
    ) -> Result<impl tokio_stream::Stream<Item = crate::LogEntry>> {
        // TODO: Implement log streaming
        // For now, return an empty stream
        let (_, receiver) = mpsc::unbounded_channel();
        Ok(tokio_stream::wrappers::UnboundedReceiverStream::new(receiver))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_container_spec_serialization() {
        let spec = ContainerSpec::default();
        let json = serde_json::to_string(&spec).unwrap();
        let parsed: ContainerSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(spec.restart_policy, parsed.restart_policy);
    }
    
    #[test]
    fn test_container_status_transitions() {
        assert_eq!(ContainerStatus::Created, ContainerStatus::Created);
        assert_ne!(ContainerStatus::Created, ContainerStatus::Running);
    }
}