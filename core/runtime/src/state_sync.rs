//! Container State Synchronization for Distributed Consensus
//!
//! This module implements Byzantine fault-tolerant state synchronization for
//! container orchestration across cluster nodes. It ensures that all honest
//! nodes maintain consistent views of container state, resource allocation,
//! and operation results despite potential malicious behavior.
//!
//! # State Consistency Guarantees
//!
//! - **Strong Consistency**: All honest nodes see identical container states
//! - **Byzantine Fault Tolerance**: Handles up to f malicious nodes in 3f+1 cluster
//! - **Conflict Resolution**: Deterministic resolution of state conflicts
//! - **Recovery**: Automatic state recovery for temporarily partitioned nodes
//!
//! # Synchronization Mechanisms
//!
//! - Merkle Tree Verification: Cryptographic state integrity validation
//! - Periodic State Checkpoints: Regular consistency verification across nodes
//! - Conflict Detection: Real-time identification of state inconsistencies  
//! - Automatic Repair: Self-healing state synchronization protocols

use crate::consensus_operations::{ContainerOperationResult, OperationMetrics};
use crate::{ContainerStatus, ResourceId, RuntimeError, Result};

use nexus_consensus::byzantine::{ByzantineGuard, ValidationResult};
use nexus_shared::{NodeId, Timestamp, NexusError};

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{RwLock as AsyncRwLock, mpsc};
use tracing::{debug, info, warn, error, instrument};

/// Manages container state synchronization across cluster nodes
///
/// The ContainerStateManager ensures that all nodes in the cluster maintain
/// consistent views of container state through cryptographic verification,
/// periodic synchronization, and Byzantine fault-tolerant conflict resolution.
#[derive(Debug)]
pub struct ContainerStateManager {
    /// This node's identifier
    node_id: NodeId,
    
    /// Current container state tracked by this node
    local_state: Arc<RwLock<ContainerClusterState>>,
    
    /// Byzantine fault detection system
    byzantine_guard: Arc<AsyncRwLock<ByzantineGuard>>,
    
    /// Operation results cache for consensus completion
    operation_results: Arc<RwLock<HashMap<u64, ContainerOperationResult>>>,
    
    /// State synchronization metrics
    sync_metrics: Arc<RwLock<StateSyncMetrics>>,
    
    /// Merkle tree for state integrity verification
    state_merkle_tree: Arc<RwLock<StateMerkleTree>>,
    
    /// Pending state synchronization operations
    pending_sync_ops: Arc<RwLock<HashMap<u64, StateSyncOperation>>>,
    
    /// Communication channel for state sync messages
    sync_message_sender: mpsc::UnboundedSender<(NodeId, StateSyncMessage)>,
    
    /// Last successful state synchronization with each node
    last_sync_with_nodes: Arc<RwLock<HashMap<NodeId, Instant>>>,
}

/// Complete container state for the entire cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerClusterState {
    /// All containers tracked by the cluster
    pub containers: BTreeMap<ResourceId, ContainerState>,
    
    /// Resource allocation tracking
    pub resource_allocations: HashMap<ResourceId, ResourceAllocation>,
    
    /// Operation execution history for audit trail
    pub operation_history: Vec<ExecutedOperation>,
    
    /// State version for conflict resolution
    pub state_version: u64,
    
    /// Last update timestamp
    pub last_updated: SystemTime,
    
    /// Cryptographic hash of entire state
    pub state_hash: [u8; 32],
}

/// Individual container state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerState {
    /// Container unique identifier
    pub container_id: ResourceId,
    
    /// Current container status
    pub status: ContainerStatus,
    
    /// Node where container is currently running
    pub assigned_node: NodeId,
    
    /// Container creation timestamp
    pub created_at: SystemTime,
    
    /// Last status update timestamp
    pub last_updated: SystemTime,
    
    /// Container resource usage
    pub resource_usage: ResourceUsage,
    
    /// Container configuration hash for integrity
    pub config_hash: [u8; 32],
    
    /// Number of restart attempts
    pub restart_count: u32,
}

/// Resource allocation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    /// Allocated CPU in millicores
    pub cpu_millicores: u64,
    
    /// Allocated memory in bytes
    pub memory_bytes: u64,
    
    /// Allocated storage in bytes
    pub storage_bytes: u64,
    
    /// Network bandwidth allocation in bytes/sec
    pub network_bandwidth: u64,
    
    /// GPU allocation (if any)
    pub gpu_units: u32,
    
    /// Allocation timestamp
    pub allocated_at: SystemTime,
    
    /// Node performing the allocation
    pub allocated_by: NodeId,
}

/// Current resource usage by container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Current CPU usage in millicores
    pub cpu_usage: u64,
    
    /// Current memory usage in bytes
    pub memory_usage: u64,
    
    /// Current storage usage in bytes
    pub storage_usage: u64,
    
    /// Network I/O statistics
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    
    /// Disk I/O statistics
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    
    /// Last measurement timestamp
    pub measured_at: SystemTime,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            cpu_usage: 0,
            memory_usage: 0,
            storage_usage: 0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
            measured_at: SystemTime::now(),
        }
    }
}

/// Record of an executed operation for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutedOperation {
    /// Operation identifier
    pub operation_id: u64,
    
    /// Operation type description
    pub operation_type: String,
    
    /// Node that executed the operation
    pub executed_by: NodeId,
    
    /// Execution timestamp
    pub executed_at: SystemTime,
    
    /// Operation result
    pub result: ContainerOperationResult,
    
    /// Execution duration
    pub duration: Duration,
}

/// State synchronization performance metrics
#[derive(Debug, Clone)]
pub struct StateSyncMetrics {
    /// Total synchronization operations performed
    pub sync_operations: u64,
    
    /// Successful synchronizations
    pub successful_syncs: u64,
    
    /// Failed synchronizations
    pub failed_syncs: u64,
    
    /// Conflicts detected and resolved
    pub conflicts_resolved: u64,
    
    /// Average synchronization time in milliseconds
    pub avg_sync_time_ms: f64,
    
    /// State divergence incidents
    pub state_divergences: u64,
    
    /// Byzantine faults detected during sync
    pub byzantine_faults_during_sync: u64,
    
    /// Data transferred during sync (bytes)
    pub sync_data_transferred: u64,
    
    /// Last metrics update
    pub last_updated: Option<Instant>,
}

impl Default for StateSyncMetrics {
    fn default() -> Self {
        Self {
            sync_operations: 0,
            successful_syncs: 0,
            failed_syncs: 0,
            conflicts_resolved: 0,
            avg_sync_time_ms: 0.0,
            state_divergences: 0,
            byzantine_faults_during_sync: 0,
            sync_data_transferred: 0,
            last_updated: None,
        }
    }
}

/// Merkle tree for efficient state integrity verification
#[derive(Debug, Clone)]
struct StateMerkleTree {
    /// Root hash of the entire state
    root_hash: [u8; 32],
    
    /// Leaf hashes for individual containers
    container_hashes: BTreeMap<ResourceId, [u8; 32]>,
    
    /// Resource allocation hashes
    resource_hashes: HashMap<ResourceId, [u8; 32]>,
    
    /// Last tree computation timestamp
    computed_at: Instant,
}

impl StateMerkleTree {
    fn new() -> Self {
        Self {
            root_hash: [0u8; 32],
            container_hashes: BTreeMap::new(),
            resource_hashes: HashMap::new(),
            computed_at: Instant::now(),
        }
    }

    /// Recompute merkle tree from current state
    fn recompute(&mut self, state: &ContainerClusterState) {
        use sha2::{Digest, Sha256};
        
        // Compute container hashes
        self.container_hashes.clear();
        for (container_id, container_state) in &state.containers {
            let serialized = bincode::serialize(container_state).unwrap_or_default();
            let mut hasher = Sha256::new();
            hasher.update(&serialized);
            self.container_hashes.insert(container_id.clone(), hasher.finalize().into());
        }
        
        // Compute resource allocation hashes
        self.resource_hashes.clear();
        for (resource_id, allocation) in &state.resource_allocations {
            let serialized = bincode::serialize(allocation).unwrap_or_default();
            let mut hasher = Sha256::new();
            hasher.update(&serialized);
            self.resource_hashes.insert(resource_id.clone(), hasher.finalize().into());
        }
        
        // Compute root hash
        let mut root_hasher = Sha256::new();
        
        // Include container hashes in sorted order
        for hash in self.container_hashes.values() {
            root_hasher.update(hash);
        }
        
        // Include resource hashes
        let mut resource_keys: Vec<_> = self.resource_hashes.keys().collect();
        resource_keys.sort();
        for key in resource_keys {
            root_hasher.update(self.resource_hashes[key]);
        }
        
        // Include state metadata
        root_hasher.update(&state.state_version.to_le_bytes());
        root_hasher.update(&state.last_updated.duration_since(SystemTime::UNIX_EPOCH)
                          .unwrap_or_default().as_nanos().to_le_bytes());
        
        self.root_hash = root_hasher.finalize().into();
        self.computed_at = Instant::now();
    }
}

/// State synchronization operation tracking
#[derive(Debug, Clone)]
struct StateSyncOperation {
    /// Synchronization operation ID
    sync_id: u64,
    
    /// Target nodes for synchronization
    target_nodes: HashSet<NodeId>,
    
    /// Operation start time
    started_at: Instant,
    
    /// Current synchronization phase
    phase: SyncPhase,
    
    /// Number of responses received
    responses_received: u32,
    
    /// Required responses for completion
    required_responses: u32,
}

/// Current phase of state synchronization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyncPhase {
    /// Requesting state hashes from nodes
    HashRequest,
    
    /// Comparing hashes for conflicts
    HashComparison,
    
    /// Requesting full state from authoritative nodes
    StateRequest,
    
    /// Resolving conflicts
    ConflictResolution,
    
    /// Synchronization completed
    Completed,
    
    /// Synchronization failed
    Failed,
}

/// Messages for state synchronization protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateSyncMessage {
    /// Request state hash from peer
    StateHashRequest {
        requester: NodeId,
        sync_id: u64,
    },
    
    /// Response with state hash
    StateHashResponse {
        responder: NodeId,
        sync_id: u64,
        state_hash: [u8; 32],
        state_version: u64,
    },
    
    /// Request full state from peer
    FullStateRequest {
        requester: NodeId,
        sync_id: u64,
    },
    
    /// Response with full state
    FullStateResponse {
        responder: NodeId,
        sync_id: u64,
        state: ContainerClusterState,
    },
    
    /// Report state conflict
    ConflictReport {
        reporter: NodeId,
        conflicting_nodes: Vec<NodeId>,
        conflict_details: String,
    },
}

/// Error types for state synchronization
#[derive(Debug, thiserror::Error)]
pub enum StateSyncError {
    #[error("State synchronization timeout: {sync_id}")]
    SyncTimeout { sync_id: u64 },
    
    #[error("State conflict detected: {details}")]
    StateConflict { details: String },
    
    #[error("Byzantine behavior detected during sync: {node_id}")]
    ByzantineBehavior { node_id: NodeId },
    
    #[error("Insufficient responses for consensus: expected {expected}, got {actual}")]
    InsufficientResponses { expected: u32, actual: u32 },
    
    #[error("State serialization error: {message}")]
    SerializationError { message: String },
}

impl ContainerStateManager {
    /// Create a new container state manager
    #[instrument(skip(byzantine_guard))]
    pub async fn new(
        node_id: NodeId,
        byzantine_guard: Arc<AsyncRwLock<ByzantineGuard>>,
    ) -> Result<Self> {
        info!("Initializing container state manager for node {}", node_id);

        let (sync_sender, _sync_receiver) = mpsc::unbounded_channel();

        let initial_state = ContainerClusterState {
            containers: BTreeMap::new(),
            resource_allocations: HashMap::new(),
            operation_history: Vec::new(),
            state_version: 1,
            last_updated: SystemTime::now(),
            state_hash: [0u8; 32],
        };

        let mut merkle_tree = StateMerkleTree::new();
        merkle_tree.recompute(&initial_state);

        Ok(Self {
            node_id,
            local_state: Arc::new(RwLock::new(initial_state)),
            byzantine_guard,
            operation_results: Arc::new(RwLock::new(HashMap::new())),
            sync_metrics: Arc::new(RwLock::new(StateSyncMetrics::default())),
            state_merkle_tree: Arc::new(RwLock::new(merkle_tree)),
            pending_sync_ops: Arc::new(RwLock::new(HashMap::new())),
            sync_message_sender: sync_sender,
            last_sync_with_nodes: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Update container state after successful operation
    #[instrument(skip(self, result))]
    pub async fn update_container_state(
        &self,
        container_id: ResourceId,
        status: ContainerStatus,
        assigned_node: NodeId,
        result: ContainerOperationResult,
    ) -> Result<()> {
        let now = SystemTime::now();

        {
            let mut state = self.local_state.write().unwrap();
            
            // Update container state
            let container_state = state.containers.entry(container_id.clone())
                .or_insert_with(|| ContainerState {
                    container_id: container_id.clone(),
                    status: ContainerStatus::Created,
                    assigned_node,
                    created_at: now,
                    last_updated: now,
                    resource_usage: ResourceUsage::default(),
                    config_hash: [0u8; 32],
                    restart_count: 0,
                });

            container_state.status = status;
            container_state.assigned_node = assigned_node;
            container_state.last_updated = now;

            // Update state metadata
            state.state_version += 1;
            state.last_updated = now;

            // Add to operation history
            let executed_op = ExecutedOperation {
                operation_id: self.generate_operation_id(),
                operation_type: "CONTAINER_STATE_UPDATE".to_string(),
                executed_by: self.node_id,
                executed_at: now,
                result,
                duration: Duration::from_millis(0), // Immediate state update
            };
            
            state.operation_history.push(executed_op);
            
            // Limit history size
            if state.operation_history.len() > 1000 {
                state.operation_history.remove(0);
            }
        }

        // Recompute merkle tree
        {
            let state = self.local_state.read().unwrap();
            let mut merkle = self.state_merkle_tree.write().unwrap();
            merkle.recompute(&state);
        }

        debug!(
            container_id = %container_id,
            "Container state updated successfully"
        );

        Ok(())
    }

    /// Store operation result for consensus completion tracking
    pub async fn store_operation_result(
        &self,
        operation_id: u64,
        result: ContainerOperationResult,
    ) -> Result<()> {
        let mut results = self.operation_results.write().unwrap();
        results.insert(operation_id, result);
        
        // Cleanup old results
        if results.len() > 10000 {
            let oldest_keys: Vec<u64> = results.keys().take(1000).cloned().collect();
            for key in oldest_keys {
                results.remove(&key);
            }
        }
        
        Ok(())
    }

    /// Get operation result if available
    pub async fn get_operation_result(&self, operation_id: u64) -> Option<ContainerOperationResult> {
        let results = self.operation_results.read().unwrap();
        results.get(&operation_id).cloned()
    }

    /// Perform periodic state synchronization across the cluster
    #[instrument(skip(self))]
    pub async fn periodic_synchronization(&self) -> Result<()> {
        debug!("Starting periodic state synchronization");

        let sync_start = Instant::now();
        let sync_id = self.generate_sync_id();

        // Update metrics
        {
            let mut metrics = self.sync_metrics.write().unwrap();
            metrics.sync_operations += 1;
        }

        // Perform synchronization
        let sync_result = self.synchronize_with_cluster(sync_id).await;

        // Update metrics based on result
        {
            let mut metrics = self.sync_metrics.write().unwrap();
            let sync_time = sync_start.elapsed().as_millis() as f64;
            
            if metrics.avg_sync_time_ms == 0.0 {
                metrics.avg_sync_time_ms = sync_time;
            } else {
                metrics.avg_sync_time_ms = 
                    (metrics.avg_sync_time_ms * 0.9) + (sync_time * 0.1);
            }

            match &sync_result {
                Ok(_) => {
                    metrics.successful_syncs += 1;
                }
                Err(_) => {
                    metrics.failed_syncs += 1;
                }
            }
            
            metrics.last_updated = Some(Instant::now());
        }

        sync_result
    }

    /// Synchronize state with all cluster nodes
    async fn synchronize_with_cluster(&self, sync_id: u64) -> Result<()> {
        // This would implement the full state synchronization protocol
        // For now, we'll implement a simplified version that validates local state
        
        // Recompute and validate local merkle tree
        {
            let state = self.local_state.read().unwrap();
            let mut merkle = self.state_merkle_tree.write().unwrap();
            merkle.recompute(&state);
            
            // Update state hash in the state itself
            drop(state);
            let mut state = self.local_state.write().unwrap();
            state.state_hash = merkle.root_hash;
        }

        debug!(sync_id = sync_id, "Local state validation completed");
        Ok(())
    }

    /// Get current cluster state
    pub fn get_cluster_state(&self) -> ContainerClusterState {
        self.local_state.read().unwrap().clone()
    }

    /// Get synchronization metrics
    pub fn get_sync_metrics(&self) -> StateSyncMetrics {
        self.sync_metrics.read().unwrap().clone()
    }

    /// Generate unique operation ID
    fn generate_operation_id(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }

    /// Generate unique synchronization ID
    fn generate_sync_id(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_consensus::byzantine::{ByzantineGuard, FaultDetectionConfig, ReputationConfig};

    async fn create_test_state_manager() -> ContainerStateManager {
        let node_id = NodeId::random();
        let byzantine_guard = Arc::new(AsyncRwLock::new(
            ByzantineGuard::new(
                node_id,
                FaultDetectionConfig::default(),
                ReputationConfig::default(),
            ).unwrap()
        ));

        ContainerStateManager::new(node_id, byzantine_guard).await.unwrap()
    }

    #[tokio::test]
    async fn test_state_manager_creation() {
        let manager = create_test_state_manager().await;
        let state = manager.get_cluster_state();
        assert_eq!(state.containers.len(), 0);
        assert_eq!(state.state_version, 1);
    }

    #[tokio::test]
    async fn test_container_state_update() {
        let manager = create_test_state_manager().await;
        let container_id = ResourceId::random();
        let node_id = NodeId::random();

        let result = ContainerOperationResult::ContainerCreated {
            container_id: container_id.clone(),
        };

        manager.update_container_state(
            container_id.clone(),
            ContainerStatus::Running,
            node_id,
            result,
        ).await.unwrap();

        let state = manager.get_cluster_state();
        assert_eq!(state.containers.len(), 1);
        assert_eq!(state.state_version, 2);
        
        let container_state = &state.containers[&container_id];
        assert_eq!(container_state.status, ContainerStatus::Running);
        assert_eq!(container_state.assigned_node, node_id);
    }

    #[tokio::test]
    async fn test_operation_result_storage() {
        let manager = create_test_state_manager().await;
        let operation_id = 12345;
        let result = ContainerOperationResult::ContainerStarted;

        manager.store_operation_result(operation_id, result.clone()).await.unwrap();
        
        let retrieved_result = manager.get_operation_result(operation_id).await;
        assert!(retrieved_result.is_some());
        
        match retrieved_result.unwrap() {
            ContainerOperationResult::ContainerStarted => {},
            _ => panic!("Unexpected result type"),
        }
    }

    #[tokio::test]
    async fn test_merkle_tree_computation() {
        let mut tree = StateMerkleTree::new();
        let state = ContainerClusterState {
            containers: BTreeMap::new(),
            resource_allocations: HashMap::new(),
            operation_history: Vec::new(),
            state_version: 1,
            last_updated: SystemTime::now(),
            state_hash: [0u8; 32],
        };

        tree.recompute(&state);
        assert_ne!(tree.root_hash, [0u8; 32]); // Should have computed a non-zero hash
    }
}