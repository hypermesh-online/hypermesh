//! Container Operations for Byzantine Fault-Tolerant Consensus
//!
//! This module defines all container lifecycle operations that can be proposed
//! and agreed upon through the PBFT consensus protocol. Each operation includes
//! comprehensive metadata for validation, execution, and auditing.
//!
//! # Operation Types
//!
//! - Container Creation: Full specification and resource allocation
//! - Container Lifecycle: Start, stop, pause, resume operations
//! - Resource Management: Scaling and resource quota adjustments  
//! - State Operations: Snapshots, migrations, and checkpoints
//!
//! # Consensus Properties
//!
//! All operations are designed to be:
//! - Deterministic: Same inputs produce identical results across nodes
//! - Serializable: Can be encoded/decoded for network transmission
//! - Validatable: Include cryptographic proofs and checksums
//! - Auditable: Comprehensive logging and traceability information

use crate::{ContainerSpec, RuntimeError, Result};
use nexus_shared::{NodeId, ResourceId, Timestamp};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// All container operations that require cluster consensus
///
/// Each operation variant includes complete information needed for
/// deterministic execution across all cluster nodes, along with
/// cryptographic validation data and audit trails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContainerConsensusOperation {
    /// Create a new container with specified configuration
    CreateContainer {
        /// Unique operation identifier
        operation_id: u64,
        /// Complete container specification
        spec: ContainerSpec,
        /// Node initiating the operation
        initiator: NodeId,
        /// Operation timestamp for ordering
        timestamp: Timestamp,
    },

    /// Start an existing container
    StartContainer {
        /// Unique operation identifier
        operation_id: u64,
        /// Container to start
        container_id: ResourceId,
        /// Node initiating the operation
        initiator: NodeId,
        /// Operation timestamp
        timestamp: Timestamp,
    },

    /// Stop a running container
    StopContainer {
        /// Unique operation identifier
        operation_id: u64,
        /// Container to stop
        container_id: ResourceId,
        /// Graceful shutdown timeout
        timeout: Option<Duration>,
        /// Node initiating the operation
        initiator: NodeId,
        /// Operation timestamp
        timestamp: Timestamp,
    },

    /// Scale container replicas or resources
    ScaleContainer {
        /// Unique operation identifier
        operation_id: u64,
        /// Container to scale
        container_id: ResourceId,
        /// New replica count
        replicas: u32,
        /// Node initiating the operation
        initiator: NodeId,
        /// Operation timestamp
        timestamp: Timestamp,
    },

    /// Update container resource quotas
    UpdateResources {
        /// Unique operation identifier
        operation_id: u64,
        /// Container to update
        container_id: ResourceId,
        /// New CPU allocation in millicores
        cpu_millicores: Option<u64>,
        /// New memory limit in bytes
        memory_bytes: Option<u64>,
        /// New storage quota in bytes
        storage_bytes: Option<u64>,
        /// Node initiating the operation
        initiator: NodeId,
        /// Operation timestamp
        timestamp: Timestamp,
    },

    /// Create container snapshot for backup/migration
    CreateSnapshot {
        /// Unique operation identifier
        operation_id: u64,
        /// Container to snapshot
        container_id: ResourceId,
        /// Snapshot name/identifier
        snapshot_name: String,
        /// Include memory state in snapshot
        include_memory: bool,
        /// Node initiating the operation
        initiator: NodeId,
        /// Operation timestamp
        timestamp: Timestamp,
    },

    /// Restore container from snapshot
    RestoreSnapshot {
        /// Unique operation identifier
        operation_id: u64,
        /// Target container for restoration
        container_id: ResourceId,
        /// Snapshot to restore from
        snapshot_name: String,
        /// Node initiating the operation
        initiator: NodeId,
        /// Operation timestamp
        timestamp: Timestamp,
    },

    /// Migrate container to different node
    MigrateContainer {
        /// Unique operation identifier
        operation_id: u64,
        /// Container to migrate
        container_id: ResourceId,
        /// Destination node
        target_node: NodeId,
        /// Live migration (preserve running state)
        live_migration: bool,
        /// Node initiating the operation
        initiator: NodeId,
        /// Operation timestamp
        timestamp: Timestamp,
    },

    /// Remove container and cleanup resources
    RemoveContainer {
        /// Unique operation identifier
        operation_id: u64,
        /// Container to remove
        container_id: ResourceId,
        /// Force removal even if running
        force: bool,
        /// Remove associated volumes
        remove_volumes: bool,
        /// Node initiating the operation
        initiator: NodeId,
        /// Operation timestamp
        timestamp: Timestamp,
    },

    /// Update container network configuration
    UpdateNetworking {
        /// Unique operation identifier
        operation_id: u64,
        /// Container to update
        container_id: ResourceId,
        /// New port mappings
        port_mappings: HashMap<u16, u16>,
        /// Network aliases to add
        network_aliases: Vec<String>,
        /// Node initiating the operation
        initiator: NodeId,
        /// Operation timestamp
        timestamp: Timestamp,
    },

    /// Execute command in running container
    ExecuteCommand {
        /// Unique operation identifier
        operation_id: u64,
        /// Target container
        container_id: ResourceId,
        /// Command to execute
        command: Vec<String>,
        /// Environment variables
        environment: HashMap<String, String>,
        /// Working directory
        working_dir: Option<String>,
        /// Node initiating the operation
        initiator: NodeId,
        /// Operation timestamp
        timestamp: Timestamp,
    },
}

impl ContainerConsensusOperation {
    /// Get the unique operation identifier
    pub fn operation_id(&self) -> u64 {
        match self {
            Self::CreateContainer { operation_id, .. } => *operation_id,
            Self::StartContainer { operation_id, .. } => *operation_id,
            Self::StopContainer { operation_id, .. } => *operation_id,
            Self::ScaleContainer { operation_id, .. } => *operation_id,
            Self::UpdateResources { operation_id, .. } => *operation_id,
            Self::CreateSnapshot { operation_id, .. } => *operation_id,
            Self::RestoreSnapshot { operation_id, .. } => *operation_id,
            Self::MigrateContainer { operation_id, .. } => *operation_id,
            Self::RemoveContainer { operation_id, .. } => *operation_id,
            Self::UpdateNetworking { operation_id, .. } => *operation_id,
            Self::ExecuteCommand { operation_id, .. } => *operation_id,
        }
    }

    /// Get the node that initiated this operation
    pub fn initiator(&self) -> NodeId {
        match self {
            Self::CreateContainer { initiator, .. } => *initiator,
            Self::StartContainer { initiator, .. } => *initiator,
            Self::StopContainer { initiator, .. } => *initiator,
            Self::ScaleContainer { initiator, .. } => *initiator,
            Self::UpdateResources { initiator, .. } => *initiator,
            Self::CreateSnapshot { initiator, .. } => *initiator,
            Self::RestoreSnapshot { initiator, .. } => *initiator,
            Self::MigrateContainer { initiator, .. } => *initiator,
            Self::RemoveContainer { initiator, .. } => *initiator,
            Self::UpdateNetworking { initiator, .. } => *initiator,
            Self::ExecuteCommand { initiator, .. } => *initiator,
        }
    }

    /// Get operation timestamp
    pub fn timestamp(&self) -> Timestamp {
        match self {
            Self::CreateContainer { timestamp, .. } => *timestamp,
            Self::StartContainer { timestamp, .. } => *timestamp,
            Self::StopContainer { timestamp, .. } => *timestamp,
            Self::ScaleContainer { timestamp, .. } => *timestamp,
            Self::UpdateResources { timestamp, .. } => *timestamp,
            Self::CreateSnapshot { timestamp, .. } => *timestamp,
            Self::RestoreSnapshot { timestamp, .. } => *timestamp,
            Self::MigrateContainer { timestamp, .. } => *timestamp,
            Self::RemoveContainer { timestamp, .. } => *timestamp,
            Self::UpdateNetworking { timestamp, .. } => *timestamp,
            Self::ExecuteCommand { timestamp, .. } => *timestamp,
        }
    }

    /// Get the target container ID (if applicable)
    pub fn target_container(&self) -> Option<ResourceId> {
        match self {
            Self::CreateContainer { .. } => None, // Container doesn't exist yet
            Self::StartContainer { container_id, .. } => Some(container_id.clone()),
            Self::StopContainer { container_id, .. } => Some(container_id.clone()),
            Self::ScaleContainer { container_id, .. } => Some(container_id.clone()),
            Self::UpdateResources { container_id, .. } => Some(container_id.clone()),
            Self::CreateSnapshot { container_id, .. } => Some(container_id.clone()),
            Self::RestoreSnapshot { container_id, .. } => Some(container_id.clone()),
            Self::MigrateContainer { container_id, .. } => Some(container_id.clone()),
            Self::RemoveContainer { container_id, .. } => Some(container_id.clone()),
            Self::UpdateNetworking { container_id, .. } => Some(container_id.clone()),
            Self::ExecuteCommand { container_id, .. } => Some(container_id.clone()),
        }
    }

    /// Get human-readable operation type
    pub fn operation_type(&self) -> &'static str {
        match self {
            Self::CreateContainer { .. } => "CREATE_CONTAINER",
            Self::StartContainer { .. } => "START_CONTAINER",
            Self::StopContainer { .. } => "STOP_CONTAINER",
            Self::ScaleContainer { .. } => "SCALE_CONTAINER",
            Self::UpdateResources { .. } => "UPDATE_RESOURCES",
            Self::CreateSnapshot { .. } => "CREATE_SNAPSHOT",
            Self::RestoreSnapshot { .. } => "RESTORE_SNAPSHOT",
            Self::MigrateContainer { .. } => "MIGRATE_CONTAINER",
            Self::RemoveContainer { .. } => "REMOVE_CONTAINER",
            Self::UpdateNetworking { .. } => "UPDATE_NETWORKING",
            Self::ExecuteCommand { .. } => "EXECUTE_COMMAND",
        }
    }

    /// Check if operation requires container to be in running state
    pub fn requires_running_container(&self) -> bool {
        matches!(
            self,
            Self::StopContainer { .. } |
            Self::CreateSnapshot { .. } |
            Self::ExecuteCommand { .. } |
            Self::UpdateNetworking { .. }
        )
    }

    /// Check if operation modifies container state
    pub fn modifies_container_state(&self) -> bool {
        matches!(
            self,
            Self::StartContainer { .. } |
            Self::StopContainer { .. } |
            Self::ScaleContainer { .. } |
            Self::UpdateResources { .. } |
            Self::RestoreSnapshot { .. } |
            Self::MigrateContainer { .. } |
            Self::RemoveContainer { .. } |
            Self::UpdateNetworking { .. }
        )
    }

    /// Validate operation parameters for correctness
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::CreateContainer { spec, .. } => {
                if spec.image.name.is_empty() {
                    return Err(RuntimeError::InvalidOperation {
                        message: "Container image name cannot be empty".to_string(),
                    }.into());
                }
                // Additional container spec validation would go here
            }

            Self::ScaleContainer { replicas, .. } => {
                if *replicas == 0 {
                    return Err(RuntimeError::InvalidOperation {
                        message: "Replica count must be greater than 0".to_string(),
                    }.into());
                }
                if *replicas > 1000 {
                    return Err(RuntimeError::InvalidOperation {
                        message: "Replica count exceeds maximum limit (1000)".to_string(),
                    }.into());
                }
            }

            Self::UpdateResources { cpu_millicores, memory_bytes, storage_bytes, .. } => {
                if let Some(cpu) = cpu_millicores {
                    if *cpu == 0 || *cpu > 64000 { // Max 64 cores
                        return Err(RuntimeError::InvalidOperation {
                            message: "CPU allocation must be between 1-64000 millicores".to_string(),
                        }.into());
                    }
                }

                if let Some(memory) = memory_bytes {
                    if *memory < 1024 * 1024 || *memory > 1024 * 1024 * 1024 * 1024 { // 1MB to 1TB
                        return Err(RuntimeError::InvalidOperation {
                            message: "Memory allocation must be between 1MB and 1TB".to_string(),
                        }.into());
                    }
                }

                if let Some(storage) = storage_bytes {
                    if *storage < 1024 * 1024 || *storage > 1024 * 1024 * 1024 * 1024 * 10 { // 1MB to 10TB
                        return Err(RuntimeError::InvalidOperation {
                            message: "Storage allocation must be between 1MB and 10TB".to_string(),
                        }.into());
                    }
                }
            }

            Self::CreateSnapshot { snapshot_name, .. } => {
                if snapshot_name.is_empty() {
                    return Err(RuntimeError::InvalidOperation {
                        message: "Snapshot name cannot be empty".to_string(),
                    }.into());
                }
                if snapshot_name.len() > 64 {
                    return Err(RuntimeError::InvalidOperation {
                        message: "Snapshot name too long (max 64 characters)".to_string(),
                    }.into());
                }
            }

            Self::ExecuteCommand { command, .. } => {
                if command.is_empty() {
                    return Err(RuntimeError::InvalidOperation {
                        message: "Execute command cannot be empty".to_string(),
                    }.into());
                }
            }

            _ => {
                // Other operations have minimal validation requirements
            }
        }

        Ok(())
    }

    /// Calculate cryptographic digest of operation for consensus validation
    pub fn digest(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let serialized = bincode::serialize(self).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(&serialized);
        hasher.finalize().into()
    }
}

/// Result of executing a container consensus operation
///
/// Each result variant provides specific information about the
/// outcome of the operation, including any generated identifiers,
/// updated state, or diagnostic information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContainerOperationResult {
    /// Container was successfully created
    ContainerCreated {
        /// Unique identifier of created container
        container_id: ResourceId,
    },

    /// Container was successfully started
    ContainerStarted,

    /// Container was successfully stopped
    ContainerStopped,

    /// Container was successfully scaled
    ContainerScaled,

    /// Container resources were successfully updated
    ResourcesUpdated,

    /// Snapshot was successfully created
    SnapshotCreated {
        /// Snapshot identifier
        snapshot_id: String,
        /// Size of snapshot in bytes
        size_bytes: u64,
    },

    /// Container was successfully restored from snapshot
    SnapshotRestored,

    /// Container was successfully migrated
    ContainerMigrated {
        /// Previous node location
        from_node: NodeId,
        /// New node location  
        to_node: NodeId,
    },

    /// Container was successfully removed
    ContainerRemoved,

    /// Container networking was successfully updated
    NetworkingUpdated,

    /// Command was successfully executed
    CommandExecuted {
        /// Command exit code
        exit_code: i32,
        /// Command output (stdout)
        stdout: Vec<u8>,
        /// Command error output (stderr)
        stderr: Vec<u8>,
        /// Execution duration
        duration: Duration,
    },
}

/// Comprehensive metrics for container operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMetrics {
    /// Total operations by type
    pub operation_counts: HashMap<String, u64>,
    
    /// Average execution times by operation type (milliseconds)
    pub avg_execution_times: HashMap<String, f64>,
    
    /// Success rates by operation type (0.0 to 1.0)
    pub success_rates: HashMap<String, f64>,
    
    /// Resource utilization during operations
    pub resource_utilization: ResourceUtilizationMetrics,
    
    /// Consensus performance metrics
    pub consensus_metrics: ConsensusPerformanceMetrics,
    
    /// Last update timestamp
    pub last_updated: SystemTime,
}

/// Resource utilization metrics during operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilizationMetrics {
    /// Average CPU utilization percentage
    pub avg_cpu_usage: f64,
    
    /// Peak CPU utilization percentage
    pub peak_cpu_usage: f64,
    
    /// Average memory utilization in bytes
    pub avg_memory_usage: u64,
    
    /// Peak memory utilization in bytes
    pub peak_memory_usage: u64,
    
    /// Average network throughput in bytes/sec
    pub avg_network_throughput: u64,
    
    /// Average disk I/O in bytes/sec
    pub avg_disk_io: u64,
}

/// Consensus-specific performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusPerformanceMetrics {
    /// Average time to achieve consensus (milliseconds)
    pub avg_consensus_time: f64,
    
    /// Consensus success rate (0.0 to 1.0)
    pub consensus_success_rate: f64,
    
    /// Number of consensus rounds required on average
    pub avg_consensus_rounds: f64,
    
    /// Byzantine faults detected during operations
    pub byzantine_faults_detected: u64,
    
    /// Network message overhead (bytes per operation)
    pub network_overhead_bytes: u64,
}

impl Default for OperationMetrics {
    fn default() -> Self {
        Self {
            operation_counts: HashMap::new(),
            avg_execution_times: HashMap::new(),
            success_rates: HashMap::new(),
            resource_utilization: ResourceUtilizationMetrics {
                avg_cpu_usage: 0.0,
                peak_cpu_usage: 0.0,
                avg_memory_usage: 0,
                peak_memory_usage: 0,
                avg_network_throughput: 0,
                avg_disk_io: 0,
            },
            consensus_metrics: ConsensusPerformanceMetrics {
                avg_consensus_time: 0.0,
                consensus_success_rate: 1.0,
                avg_consensus_rounds: 3.0,
                byzantine_faults_detected: 0,
                network_overhead_bytes: 0,
            },
            last_updated: SystemTime::now(),
        }
    }
}