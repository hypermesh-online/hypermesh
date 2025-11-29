//! Container resource management and enforcement

use crate::ContainerId;
use super::error::{Result, ContainerError};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, Instant};
use tracing::{info, debug, warn, error};

/// Resource allocation for containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    /// CPU cores allocated
    pub cpu_cores: f64,
    /// Memory in bytes
    pub memory_bytes: u64,
    /// Storage in bytes
    pub storage_bytes: u64,
    /// Network bandwidth in bytes/sec
    pub network_bandwidth: u64,
}

/// Resource constraints for containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConstraints {
    /// Minimum CPU cores required
    pub min_cpu_cores: f64,
    /// Maximum CPU cores allowed
    pub max_cpu_cores: f64,
    /// Minimum memory required
    pub min_memory: u64,
    /// Maximum memory allowed
    pub max_memory: u64,
}

/// Resource quota specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    /// Memory limit in bytes
    pub memory_limit: Option<u64>,
    /// CPU quota (cores)
    pub cpu_quota: Option<f64>,
    /// CPU period for quota enforcement
    pub cpu_period: Duration,
    /// I/O bandwidth limit in bytes/sec
    pub io_bandwidth_limit: Option<u64>,
    /// Network bandwidth limit in bytes/sec
    pub network_bandwidth_limit: Option<u64>,
    /// Maximum file descriptors
    pub max_file_descriptors: Option<u32>,
    /// Maximum processes
    pub max_processes: Option<u32>,
    /// Disk space limit in bytes
    pub disk_space_limit: Option<u64>,
    /// PID namespace limit
    pub pid_limit: Option<u32>,
}

/// Resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Current memory usage in bytes
    pub memory_usage: u64,
    /// Peak memory usage in bytes
    pub memory_peak: u64,
    /// CPU usage percentage (0.0 - 100.0)
    pub cpu_usage_percent: f64,
    /// Total CPU time in nanoseconds
    pub cpu_time_ns: u64,
    /// Current I/O bytes per second
    pub io_bandwidth_current: u64,
    /// Total I/O bytes read
    pub io_bytes_read: u64,
    /// Total I/O bytes written
    pub io_bytes_written: u64,
    /// Current network bytes per second
    pub network_bandwidth_current: u64,
    /// Total network bytes received
    pub network_bytes_rx: u64,
    /// Total network bytes transmitted
    pub network_bytes_tx: u64,
    /// Current file descriptor count
    pub file_descriptors_current: u32,
    /// Current process count
    pub processes_current: u32,
    /// Current disk usage in bytes
    pub disk_usage: u64,
    /// Last update timestamp
    pub timestamp: SystemTime,
}

/// Resource enforcement action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnforcementAction {
    /// Allow operation
    Allow,
    /// Throttle operation
    Throttle { factor: f64 },
    /// Deny operation
    Deny { reason: String },
    /// Kill container
    Kill { reason: String },
}

/// Resource event type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceEvent {
    /// Resource limit exceeded
    LimitExceeded {
        resource: String,
        limit: u64,
        usage: u64,
        action: EnforcementAction,
    },
    /// Resource usage warning
    UsageWarning {
        resource: String,
        threshold: f64,
        usage_percent: f64,
    },
    /// Resource allocation changed
    AllocationChanged {
        resource: String,
        old_limit: Option<u64>,
        new_limit: Option<u64>,
    },
    /// OOM (Out of Memory) event
    OutOfMemory {
        requested: u64,
        available: u64,
    },
}

/// Resource manager trait
#[async_trait]
pub trait ResourceManager: Send + Sync {
    /// Set resource quota for container
    async fn set_quota(&self, id: ContainerId, quota: ResourceQuota) -> Result<()>;
    
    /// Get resource quota for container
    async fn get_quota(&self, id: ContainerId) -> Result<ResourceQuota>;
    
    /// Get current resource usage
    async fn get_usage(&self, id: ContainerId) -> Result<ResourceUsage>;
    
    /// Enforce resource limits
    async fn enforce_limits(&self, id: ContainerId) -> Result<EnforcementAction>;
    
    /// Update resource limits dynamically
    async fn update_limits(&self, id: ContainerId, quota: ResourceQuota) -> Result<()>;
    
    /// Monitor resource usage
    async fn monitor(&self, id: ContainerId) -> Result<Vec<ResourceEvent>>;
    
    /// Cleanup resources for container
    async fn cleanup(&self, id: ContainerId) -> Result<()>;
}

/// Cgroup-based resource manager implementation
pub struct CgroupResourceManager {
    quotas: std::sync::Arc<tokio::sync::RwLock<HashMap<ContainerId, ResourceQuota>>>,
    usage_history: std::sync::Arc<tokio::sync::RwLock<HashMap<ContainerId, Vec<ResourceUsage>>>>,
    monitoring_enabled: bool,
}

impl CgroupResourceManager {
    /// Create a new cgroup resource manager
    pub fn new() -> Self {
        Self {
            quotas: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            usage_history: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            monitoring_enabled: true,
        }
    }
    
    /// Enable or disable monitoring
    pub fn set_monitoring(&mut self, enabled: bool) {
        self.monitoring_enabled = enabled;
    }
    
    /// Calculate CPU usage percentage
    fn calculate_cpu_usage(&self, previous: Option<&ResourceUsage>, current: &ResourceUsage) -> f64 {
        if let Some(prev) = previous {
            let time_diff = current.timestamp.duration_since(prev.timestamp)
                .unwrap_or(Duration::from_secs(1))
                .as_nanos() as f64;
            let cpu_diff = (current.cpu_time_ns - prev.cpu_time_ns) as f64;
            
            if time_diff > 0.0 {
                (cpu_diff / time_diff) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
    
    /// Check memory limits
    async fn check_memory_limit(&self, id: ContainerId, usage: &ResourceUsage) -> Option<ResourceEvent> {
        let quotas = self.quotas.read().await;
        if let Some(quota) = quotas.get(&id) {
            if let Some(limit) = quota.memory_limit {
                if usage.memory_usage > limit {
                    return Some(ResourceEvent::LimitExceeded {
                        resource: "memory".to_string(),
                        limit,
                        usage: usage.memory_usage,
                        action: EnforcementAction::Kill {
                            reason: "Memory limit exceeded".to_string(),
                        },
                    });
                } else if usage.memory_usage > limit * 90 / 100 {
                    return Some(ResourceEvent::UsageWarning {
                        resource: "memory".to_string(),
                        threshold: 0.9,
                        usage_percent: usage.memory_usage as f64 / limit as f64,
                    });
                }
            }
        }
        None
    }
    
    /// Check CPU limits
    async fn check_cpu_limit(&self, id: ContainerId, usage: &ResourceUsage) -> Option<ResourceEvent> {
        let quotas = self.quotas.read().await;
        if let Some(quota) = quotas.get(&id) {
            if let Some(cpu_limit) = quota.cpu_quota {
                let cpu_limit_percent = cpu_limit * 100.0;
                if usage.cpu_usage_percent > cpu_limit_percent {
                    return Some(ResourceEvent::LimitExceeded {
                        resource: "cpu".to_string(),
                        limit: cpu_limit_percent as u64,
                        usage: usage.cpu_usage_percent as u64,
                        action: EnforcementAction::Throttle { factor: 0.5 },
                    });
                }
            }
        }
        None
    }
    
    /// Simulate reading resource usage from cgroups
    async fn read_cgroup_usage(&self, _id: ContainerId) -> ResourceUsage {
        // In a real implementation, this would read from /sys/fs/cgroup/
        // For simulation, generate realistic usage data
        ResourceUsage {
            memory_usage: 1024 * 1024 * 100, // 100MB
            memory_peak: 1024 * 1024 * 150,  // 150MB
            cpu_usage_percent: 25.0,
            cpu_time_ns: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
            io_bandwidth_current: 1024 * 1024, // 1MB/s
            io_bytes_read: 1024 * 1024 * 50,   // 50MB total
            io_bytes_written: 1024 * 1024 * 25, // 25MB total
            network_bandwidth_current: 1024 * 512, // 512KB/s
            network_bytes_rx: 1024 * 1024 * 10,    // 10MB total
            network_bytes_tx: 1024 * 1024 * 5,     // 5MB total
            file_descriptors_current: 128,
            processes_current: 5,
            disk_usage: 1024 * 1024 * 200, // 200MB
            timestamp: SystemTime::now(),
        }
    }
}

#[async_trait]
impl ResourceManager for CgroupResourceManager {
    async fn set_quota(&self, id: ContainerId, quota: ResourceQuota) -> Result<()> {
        let mut quotas = self.quotas.write().await;
        quotas.insert(id, quota.clone());
        
        // In a real implementation, this would write to cgroup files
        // For now, just log the operation
        info!("Set resource quota for container {}: {:?}", id, quota);
        Ok(())
    }
    
    async fn get_quota(&self, id: ContainerId) -> Result<ResourceQuota> {
        let quotas = self.quotas.read().await;
        quotas.get(&id)
            .cloned()
            .ok_or_else(|| ContainerError::NotFound { 
                id: id.to_string() 
            })
    }
    
    async fn get_usage(&self, id: ContainerId) -> Result<ResourceUsage> {
        let usage = self.read_cgroup_usage(id).await;
        
        // Store usage in history for trend analysis
        if self.monitoring_enabled {
            let mut history = self.usage_history.write().await;
            let container_history = history.entry(id).or_insert_with(Vec::new);
            container_history.push(usage.clone());
            
            // Keep only last 100 entries to prevent memory bloat
            if container_history.len() > 100 {
                container_history.remove(0);
            }
        }
        
        Ok(usage)
    }
    
    async fn enforce_limits(&self, id: ContainerId) -> Result<EnforcementAction> {
        let usage = self.get_usage(id).await?;
        
        // Check memory limits first (highest priority)
        if let Some(event) = self.check_memory_limit(id, &usage).await {
            if let ResourceEvent::LimitExceeded { action, .. } = event {
                warn!("Container {} exceeded memory limit", id);
                return Ok(action);
            }
        }
        
        // Check CPU limits
        if let Some(event) = self.check_cpu_limit(id, &usage).await {
            if let ResourceEvent::LimitExceeded { action, .. } = event {
                debug!("Container {} exceeded CPU limit", id);
                return Ok(action);
            }
        }
        
        Ok(EnforcementAction::Allow)
    }
    
    async fn update_limits(&self, id: ContainerId, quota: ResourceQuota) -> Result<()> {
        let old_quota = {
            let quotas = self.quotas.read().await;
            quotas.get(&id).cloned()
        };
        
        // Update the quota
        self.set_quota(id, quota.clone()).await?;
        
        // Generate allocation change events
        if let Some(old) = old_quota {
            if old.memory_limit != quota.memory_limit {
                info!("Memory limit changed for container {}: {:?} -> {:?}", 
                     id, old.memory_limit, quota.memory_limit);
            }
            if old.cpu_quota != quota.cpu_quota {
                info!("CPU quota changed for container {}: {:?} -> {:?}", 
                     id, old.cpu_quota, quota.cpu_quota);
            }
        }
        
        Ok(())
    }
    
    async fn monitor(&self, id: ContainerId) -> Result<Vec<ResourceEvent>> {
        let mut events = Vec::new();
        let usage = self.get_usage(id).await?;
        
        // Check all resource limits and generate events
        if let Some(event) = self.check_memory_limit(id, &usage).await {
            events.push(event);
        }
        
        if let Some(event) = self.check_cpu_limit(id, &usage).await {
            events.push(event);
        }
        
        // Check for trends in usage history
        let history = self.usage_history.read().await;
        if let Some(container_history) = history.get(&id) {
            if container_history.len() >= 2 {
                let current = container_history.last().unwrap();
                let previous = &container_history[container_history.len() - 2];
                
                // Calculate memory growth rate
                if current.memory_usage > previous.memory_usage {
                    let growth_rate = (current.memory_usage - previous.memory_usage) as f64
                        / previous.memory_usage as f64;
                    
                    if growth_rate > 0.1 { // 10% growth
                        debug!("Rapid memory growth detected for container {}: {:.2}%", 
                              id, growth_rate * 100.0);
                    }
                }
            }
        }
        
        Ok(events)
    }
    
    async fn cleanup(&self, id: ContainerId) -> Result<()> {
        // Remove quota configuration
        let mut quotas = self.quotas.write().await;
        quotas.remove(&id);
        
        // Clear usage history
        let mut history = self.usage_history.write().await;
        history.remove(&id);
        
        // In a real implementation, this would clean up cgroup directories
        info!("Cleaned up resources for container {}", id);
        Ok(())
    }
}

impl Default for CgroupResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            memory_limit: Some(1024 * 1024 * 1024), // 1GB default
            cpu_quota: Some(1.0), // 1 core default
            cpu_period: Duration::from_millis(100),
            io_bandwidth_limit: None,
            network_bandwidth_limit: None,
            max_file_descriptors: Some(1024),
            max_processes: Some(256),
            disk_space_limit: Some(10 * 1024 * 1024 * 1024), // 10GB default
            pid_limit: Some(1024),
        }
    }
}