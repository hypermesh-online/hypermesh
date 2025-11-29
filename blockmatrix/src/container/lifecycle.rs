//! Container lifecycle management

use crate::{ContainerId, ContainerSpec};
use super::error::{Result, ContainerError};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::time::{Duration, SystemTime};
use tracing::{info, debug, warn, error};

/// Container states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerState {
    /// Container image loaded, not started
    Created,
    /// Container initialization in progress
    Starting,
    /// Container executing workload
    Running,
    /// Container being paused (live migration prep)
    Pausing,
    /// Container execution suspended
    Paused,
    /// Container being resumed from pause
    Resuming,
    /// Container graceful shutdown in progress
    Stopping,
    /// Container terminated, resources cleaned
    Stopped,
    /// Container failed
    Failed { reason: String },
}

impl std::fmt::Display for ContainerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerState::Created => write!(f, "created"),
            ContainerState::Starting => write!(f, "starting"),
            ContainerState::Running => write!(f, "running"),
            ContainerState::Pausing => write!(f, "pausing"),
            ContainerState::Paused => write!(f, "paused"),
            ContainerState::Resuming => write!(f, "resuming"),
            ContainerState::Stopping => write!(f, "stopping"),
            ContainerState::Stopped => write!(f, "stopped"),
            ContainerState::Failed { reason } => write!(f, "failed: {}", reason),
        }
    }
}

/// Container lifecycle events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContainerEvent {
    /// Container was created
    Created { timestamp: SystemTime },
    /// Container started
    Started { timestamp: SystemTime },
    /// Container stopped
    Stopped { timestamp: SystemTime, exit_code: Option<i32> },
    /// Container paused
    Paused { timestamp: SystemTime },
    /// Container resumed
    Resumed { timestamp: SystemTime },
    /// Container failed
    Failed { timestamp: SystemTime, reason: String },
    /// Container checkpoint created
    CheckpointCreated { timestamp: SystemTime, path: String },
    /// Container restored from checkpoint
    Restored { timestamp: SystemTime, path: String },
}

/// Container status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStatus {
    /// Container ID
    pub id: ContainerId,
    /// Current state
    pub state: ContainerState,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Start timestamp (if started)
    pub started_at: Option<SystemTime>,
    /// Finish timestamp (if stopped)
    pub finished_at: Option<SystemTime>,
    /// Exit code (if stopped)
    pub exit_code: Option<i32>,
    /// Process ID (if running)
    pub pid: Option<u32>,
    /// Resource usage statistics
    pub stats: Option<ContainerStats>,
    /// Recent events
    pub events: Vec<ContainerEvent>,
}

/// Container resource usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStats {
    /// CPU usage in nanoseconds
    pub cpu_usage_ns: u64,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// Memory limit in bytes
    pub memory_limit_bytes: u64,
    /// Network RX bytes
    pub network_rx_bytes: u64,
    /// Network TX bytes
    pub network_tx_bytes: u64,
    /// Filesystem read bytes
    pub filesystem_read_bytes: u64,
    /// Filesystem write bytes
    pub filesystem_write_bytes: u64,
    /// Number of processes
    pub processes: u32,
    /// Number of file descriptors
    pub file_descriptors: u32,
    /// Container uptime in nanoseconds
    pub uptime_ns: u64,
}

/// Container lifecycle management trait
#[async_trait]
pub trait ContainerLifecycle: Send + Sync {
    /// Create a new container
    async fn create(&self, id: ContainerId, spec: ContainerSpec) -> Result<()>;
    
    /// Start a container
    async fn start(&self, id: ContainerId) -> Result<()>;
    
    /// Stop a container
    async fn stop(&self, id: ContainerId, timeout: Option<Duration>) -> Result<()>;
    
    /// Pause a container
    async fn pause(&self, id: ContainerId) -> Result<()>;
    
    /// Resume a paused container
    async fn resume(&self, id: ContainerId) -> Result<()>;
    
    /// Kill a container
    async fn kill(&self, id: ContainerId, signal: Option<i32>) -> Result<()>;
    
    /// Delete a container
    async fn delete(&self, id: ContainerId) -> Result<()>;
    
    /// Get container status
    async fn status(&self, id: ContainerId) -> Result<ContainerStatus>;
    
    /// List all containers
    async fn list(&self) -> Result<Vec<ContainerId>>;
    
    /// Create a checkpoint
    async fn checkpoint(&self, id: ContainerId, path: &str) -> Result<()>;
    
    /// Restore from checkpoint
    async fn restore(&self, id: ContainerId, path: &str) -> Result<()>;
    
    /// Wait for container to change state
    async fn wait(&self, id: ContainerId) -> Result<ContainerEvent>;
}

/// Default container lifecycle implementation
pub struct DefaultContainerLifecycle {
    containers: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<ContainerId, ContainerStatus>>>,
}

impl DefaultContainerLifecycle {
    /// Create a new lifecycle manager
    pub fn new() -> Self {
        Self {
            containers: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }
    
    /// Validate state transition
    fn can_transition(&self, from: &ContainerState, to: &ContainerState) -> bool {
        match (from, to) {
            (ContainerState::Created, ContainerState::Starting) => true,
            (ContainerState::Starting, ContainerState::Running) => true,
            (ContainerState::Starting, ContainerState::Failed { .. }) => true,
            (ContainerState::Running, ContainerState::Pausing) => true,
            (ContainerState::Running, ContainerState::Stopping) => true,
            (ContainerState::Running, ContainerState::Failed { .. }) => true,
            (ContainerState::Pausing, ContainerState::Paused) => true,
            (ContainerState::Pausing, ContainerState::Failed { .. }) => true,
            (ContainerState::Paused, ContainerState::Resuming) => true,
            (ContainerState::Paused, ContainerState::Stopping) => true,
            (ContainerState::Resuming, ContainerState::Running) => true,
            (ContainerState::Resuming, ContainerState::Failed { .. }) => true,
            (ContainerState::Stopping, ContainerState::Stopped) => true,
            (ContainerState::Stopping, ContainerState::Failed { .. }) => true,
            (ContainerState::Stopped, ContainerState::Starting) => true,
            (ContainerState::Failed { .. }, ContainerState::Starting) => true,
            _ => false,
        }
    }
    
    /// Transition container to new state
    async fn transition_state(&self, id: ContainerId, new_state: ContainerState) -> Result<()> {
        let mut containers = self.containers.write().await;
        if let Some(status) = containers.get_mut(&id) {
            if !self.can_transition(&status.state, &new_state) {
                return Err(ContainerError::InvalidState {
                    expected: format!("valid transition from {}", status.state),
                    actual: new_state.to_string(),
                });
            }
            
            let event = match &new_state {
                ContainerState::Starting => ContainerEvent::Started { timestamp: SystemTime::now() },
                ContainerState::Stopped => ContainerEvent::Stopped { 
                    timestamp: SystemTime::now(), 
                    exit_code: status.exit_code 
                },
                ContainerState::Paused => ContainerEvent::Paused { timestamp: SystemTime::now() },
                ContainerState::Running if status.state == ContainerState::Resuming => {
                    ContainerEvent::Resumed { timestamp: SystemTime::now() }
                },
                ContainerState::Failed { reason } => ContainerEvent::Failed { 
                    timestamp: SystemTime::now(), 
                    reason: reason.clone() 
                },
                _ => return Ok(()),
            };
            
            status.state = new_state;
            status.events.push(event);
            
            // Update timestamps
            match &status.state {
                ContainerState::Running if status.started_at.is_none() => {
                    status.started_at = Some(SystemTime::now());
                },
                ContainerState::Stopped | ContainerState::Failed { .. } => {
                    status.finished_at = Some(SystemTime::now());
                },
                _ => {},
            }
            
            info!("Container {} transitioned to state: {}", id, status.state);
        }
        
        Ok(())
    }
}

#[async_trait]
impl ContainerLifecycle for DefaultContainerLifecycle {
    async fn create(&self, id: ContainerId, _spec: ContainerSpec) -> Result<()> {
        let mut containers = self.containers.write().await;
        
        if containers.contains_key(&id) {
            return Err(ContainerError::AlreadyExists { id: id.to_string() });
        }
        
        let status = ContainerStatus {
            id,
            state: ContainerState::Created,
            created_at: SystemTime::now(),
            started_at: None,
            finished_at: None,
            exit_code: None,
            pid: None,
            stats: None,
            events: vec![ContainerEvent::Created { timestamp: SystemTime::now() }],
        };
        
        containers.insert(id, status);
        info!("Created container {}", id);
        Ok(())
    }
    
    async fn start(&self, id: ContainerId) -> Result<()> {
        self.transition_state(id, ContainerState::Starting).await?;
        
        // Simulate container startup process
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        self.transition_state(id, ContainerState::Running).await?;
        info!("Started container {}", id);
        Ok(())
    }
    
    async fn stop(&self, id: ContainerId, timeout: Option<Duration>) -> Result<()> {
        let timeout = timeout.unwrap_or(Duration::from_secs(5));
        
        self.transition_state(id, ContainerState::Stopping).await?;
        
        // Simulate graceful shutdown
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        self.transition_state(id, ContainerState::Stopped).await?;
        info!("Stopped container {} with timeout {:?}", id, timeout);
        Ok(())
    }
    
    async fn pause(&self, id: ContainerId) -> Result<()> {
        self.transition_state(id, ContainerState::Pausing).await?;
        
        // Simulate pause operation
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        self.transition_state(id, ContainerState::Paused).await?;
        info!("Paused container {}", id);
        Ok(())
    }
    
    async fn resume(&self, id: ContainerId) -> Result<()> {
        self.transition_state(id, ContainerState::Resuming).await?;
        
        // Simulate resume operation
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        self.transition_state(id, ContainerState::Running).await?;
        info!("Resumed container {}", id);
        Ok(())
    }
    
    async fn kill(&self, id: ContainerId, signal: Option<i32>) -> Result<()> {
        let signal = signal.unwrap_or(9); // SIGKILL
        self.transition_state(id, ContainerState::Stopped).await?;
        info!("Killed container {} with signal {}", id, signal);
        Ok(())
    }
    
    async fn delete(&self, id: ContainerId) -> Result<()> {
        let mut containers = self.containers.write().await;
        
        if let Some(status) = containers.get(&id) {
            match status.state {
                ContainerState::Running | ContainerState::Starting => {
                    return Err(ContainerError::InvalidState {
                        expected: "stopped or failed".to_string(),
                        actual: status.state.to_string(),
                    });
                },
                _ => {},
            }
        } else {
            return Err(ContainerError::NotFound { id: id.to_string() });
        }
        
        containers.remove(&id);
        info!("Deleted container {}", id);
        Ok(())
    }
    
    async fn status(&self, id: ContainerId) -> Result<ContainerStatus> {
        let containers = self.containers.read().await;
        containers.get(&id)
            .cloned()
            .ok_or_else(|| ContainerError::NotFound { id: id.to_string() })
    }
    
    async fn list(&self) -> Result<Vec<ContainerId>> {
        let containers = self.containers.read().await;
        Ok(containers.keys().copied().collect())
    }
    
    async fn checkpoint(&self, id: ContainerId, path: &str) -> Result<()> {
        // Simulate checkpoint creation
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let mut containers = self.containers.write().await;
        if let Some(status) = containers.get_mut(&id) {
            let event = ContainerEvent::CheckpointCreated {
                timestamp: SystemTime::now(),
                path: path.to_string(),
            };
            status.events.push(event);
            info!("Created checkpoint for container {} at {}", id, path);
        }
        
        Ok(())
    }
    
    async fn restore(&self, id: ContainerId, path: &str) -> Result<()> {
        // Simulate restore from checkpoint
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        let mut containers = self.containers.write().await;
        if let Some(status) = containers.get_mut(&id) {
            let event = ContainerEvent::Restored {
                timestamp: SystemTime::now(),
                path: path.to_string(),
            };
            status.events.push(event);
            status.state = ContainerState::Running;
            info!("Restored container {} from checkpoint at {}", id, path);
        }
        
        Ok(())
    }
    
    async fn wait(&self, id: ContainerId) -> Result<ContainerEvent> {
        // Simulate waiting for state change
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            let containers = self.containers.read().await;
            if let Some(status) = containers.get(&id) {
                if let Some(event) = status.events.last() {
                    return Ok(event.clone());
                }
            } else {
                return Err(ContainerError::NotFound { id: id.to_string() });
            }
        }
    }
}

impl Default for DefaultContainerLifecycle {
    fn default() -> Self {
        Self::new()
    }
}