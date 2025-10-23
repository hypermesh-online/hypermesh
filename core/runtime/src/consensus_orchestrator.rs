//! Consensus-coordinated Container Orchestration
//!
//! This module implements Byzantine fault-tolerant container orchestration by integrating
//! the HyperMesh container runtime with the validated PBFT consensus system. All container
//! lifecycle operations are coordinated through consensus to ensure consistency across
//! distributed nodes while maintaining performance requirements.
//!
//! # Architecture Overview
//!
//! The ConsensusContainerOrchestrator acts as a bridge between:
//! - Container Runtime: Low-level container management and execution
//! - PBFT Consensus: Byzantine fault-tolerant agreement on operations
//! - Byzantine Guard: Malicious node detection and reputation management
//!
//! # Performance Requirements
//!
//! - Container operations add <50ms consensus coordination overhead
//! - Maintains existing container startup performance
//! - All state changes achieve cluster-wide consistency
//!
//! # Security Properties
//!
//! - Byzantine fault tolerance for up to f malicious nodes (3f+1 cluster)
//! - Cryptographic validation of all container operations
//! - Automatic isolation of nodes exhibiting malicious behavior
//! - State integrity verification across cluster nodes


use crate::{Runtime, RuntimeConfig, ContainerSpec, RuntimeError, Result};
use crate::consensus_operations::{ContainerConsensusOperation, ContainerOperationResult, OperationMetrics};
use crate::state_sync::{ContainerStateManager, ContainerClusterState, StateSyncError};
use crate::consensus_validation::{ContainerStateValidator, ValidatedContainerState, ValidationMetrics};

use nexus_consensus::pbft::consensus::{PbftNode, ConsensusState};
use nexus_consensus::pbft::messages::{ClientRequest, PbftMessage};
use nexus_consensus::byzantine::{ByzantineGuard, ValidationResult};
use nexus_shared::{NodeId, ResourceId, Timestamp, Result as NexusResult, NexusError};


use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock as AsyncRwLock, Mutex};
use tracing::{debug, info, warn, error, instrument};

/// Byzantine fault-tolerant container orchestrator
///
/// This orchestrator ensures that all container operations are agreed upon
/// by the cluster through PBFT consensus before execution, providing
/// strong consistency guarantees even in the presence of malicious nodes.
pub struct ConsensusContainerOrchestrator {
    /// Node identifier in the cluster
    node_id: NodeId,
    
    /// Container runtime for actual execution
    runtime: Arc<Runtime>,
    
    /// PBFT consensus node for operation coordination
    consensus_node: Arc<Mutex<PbftNode>>,
    
    /// Byzantine fault detection and reputation system
    byzantine_guard: Arc<AsyncRwLock<ByzantineGuard>>,
    
    /// Container state synchronization manager
    state_manager: Arc<ContainerStateManager>,
    
    /// Container state validator for Byzantine fault tolerance
    state_validator: ContainerStateValidator,
    
    /// Pending container operations awaiting consensus
    pending_operations: Arc<RwLock<HashMap<u64, PendingContainerOperation>>>,
    
    /// Operation result callbacks
    operation_callbacks: Arc<RwLock<HashMap<u64, OperationCallback>>>,
    
    /// Performance and reliability metrics
    metrics: Arc<RwLock<OrchestrationMetrics>>,
    
    /// Message sender for consensus communications
    consensus_sender: mpsc::UnboundedSender<(NodeId, PbftMessage)>,
    
    /// Operation sequence counter
    operation_sequence: Arc<RwLock<u64>>,
    
    /// Cluster membership and health status
    cluster_status: Arc<RwLock<ClusterStatus>>,
}

impl std::fmt::Debug for ConsensusContainerOrchestrator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConsensusContainerOrchestrator")
            .field("node_id", &self.node_id)
            .field("runtime", &"<Runtime>")
            .field("consensus_node", &"<PbftNode>")
            .field("byzantine_guard", &"<ByzantineGuard>")
            .field("state_manager", &self.state_manager)
            .field("state_validator", &self.state_validator)
            .field("pending_operations", &self.pending_operations)
            .field("operation_callbacks", &"<Callbacks>")
            .field("metrics", &self.metrics)
            .field("consensus_sender", &"<Sender>")
            .field("operation_sequence", &self.operation_sequence)
            .field("cluster_status", &self.cluster_status)
            .finish()
    }
}

// Type definitions module
pub mod types {
    use super::*;
    
    /// Pending container operation awaiting consensus completion
    #[derive(Debug, Clone)]
    pub struct PendingContainerOperation {
        /// Unique operation identifier
        pub operation_id: u64,
        /// Container operation details
        pub operation: ContainerConsensusOperation,
        /// Node that initiated the operation
        pub initiator: NodeId,
        /// Timestamp when operation was submitted
        pub submitted_at: Instant,
        /// Current consensus state
        pub consensus_state: ConsensusPhase,
        /// Number of retry attempts
        pub retry_count: u32,
        /// Maximum allowed retries
        pub max_retries: u32,
    }

    /// Current phase of consensus for an operation
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ConsensusPhase {
        /// Operation submitted, awaiting pre-prepare
        Submitted,
        /// Pre-prepare received, awaiting prepare votes
        PrePrepare,
        /// Prepare phase complete, awaiting commit votes
        Prepared,
        /// Consensus achieved, operation ready for execution
        Committed,
        /// Operation has been executed
        Executed,
        /// Operation failed consensus or execution
        Failed,
    }

    /// Callback for operation completion notification
    pub type OperationCallback = Box<dyn Fn(Result<ContainerOperationResult>) + Send + Sync>;

    /// Comprehensive orchestration performance metrics
    #[derive(Debug, Clone)]
    pub struct OrchestrationMetrics {
        /// Total container operations processed
        pub operations_processed: u64,
        /// Operations currently in consensus
        pub operations_in_consensus: u64,
        /// Successfully completed operations
        pub operations_successful: u64,
        /// Failed operations (consensus or execution failures)
        pub operations_failed: u64,
        /// Average consensus coordination time in milliseconds
        pub avg_consensus_time_ms: f64,
        /// Average container execution time in milliseconds  
        pub avg_execution_time_ms: f64,
        /// Consensus timeout occurrences
        pub consensus_timeouts: u64,
        /// Byzantine faults detected during operation
        pub byzantine_faults_detected: u64,
        /// State synchronization conflicts resolved
        pub state_sync_conflicts: u64,
        /// Last metrics update timestamp
        pub last_updated: Option<Instant>,
    }

    impl Default for OrchestrationMetrics {
        fn default() -> Self {
            Self {
                operations_processed: 0,
                operations_in_consensus: 0,
                operations_successful: 0,
                operations_failed: 0,
                avg_consensus_time_ms: 0.0,
                avg_execution_time_ms: 0.0,
                consensus_timeouts: 0,
                byzantine_faults_detected: 0,
                state_sync_conflicts: 0,
                last_updated: None,
            }
        }
    }

    /// Cluster membership and health tracking
    #[derive(Debug, Clone)]
    pub struct ClusterStatus {
        /// Total nodes in cluster
        pub total_nodes: usize,
        /// Currently active and responsive nodes
        pub active_nodes: usize,
        /// Nodes currently quarantined for Byzantine behavior
        pub quarantined_nodes: Vec<NodeId>,
        /// Maximum tolerable Byzantine faults (f in 3f+1)
        pub byzantine_threshold: usize,
        /// Current cluster health status
        pub health_status: ClusterHealth,
        /// Last health check timestamp
        pub last_health_check: Option<Instant>,
    }

    /// Overall cluster health assessment
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ClusterHealth {
        /// Cluster is healthy and fully operational
        Healthy,
        /// Cluster is operational but approaching Byzantine fault limit
        Warning,
        /// Cluster cannot tolerate additional faults
        Critical,
        /// Cluster has lost Byzantine fault tolerance
        Compromised,
    }
}

// Re-export types
use types::*;

impl ConsensusContainerOrchestrator {
    /// Create a new consensus container orchestrator
    ///
    /// # Arguments
    ///
    /// * `node_id` - Unique identifier for this node
    /// * `runtime_config` - Configuration for the container runtime
    /// * `consensus_node` - PBFT consensus node instance
    /// * `byzantine_guard` - Byzantine fault detection system
    /// * `consensus_sender` - Channel for sending consensus messages
    ///
    /// # Returns
    ///
    /// A configured orchestrator ready for container operations
    #[instrument(skip(runtime_config, consensus_node, byzantine_guard, consensus_sender))]
    pub async fn new(
        node_id: NodeId,
        runtime_config: RuntimeConfig,
        consensus_node: PbftNode,
        byzantine_guard: Arc<AsyncRwLock<ByzantineGuard>>,
        consensus_sender: mpsc::UnboundedSender<(NodeId, PbftMessage)>,
    ) -> Result<Self> {
        info!("Initializing consensus container orchestrator for node {}", node_id);

        // Initialize container runtime
        let runtime = Arc::new(Runtime::new(runtime_config).await?);
        
        // Initialize state synchronization manager
        let state_manager = Arc::new(
            ContainerStateManager::new(node_id, Arc::clone(&byzantine_guard)).await?
        );

        // Initialize state validator
        let state_validator = ContainerStateValidator::new(node_id, Arc::clone(&byzantine_guard));

        // Determine cluster size and Byzantine fault tolerance
        let consensus_node_guard = consensus_node;
        // TODO: Get actual cluster size from PbftNode public API
        let total_nodes = 4; // Default assumption for 3f+1 = 4 node cluster
        let byzantine_threshold = (total_nodes - 1) / 3; // f in 3f+1 Byzantine fault tolerance

        let cluster_status = ClusterStatus {
            total_nodes,
            active_nodes: total_nodes, // Assume all nodes start active
            quarantined_nodes: Vec::new(),
            byzantine_threshold,
            health_status: ClusterHealth::Healthy,
            last_health_check: Some(Instant::now()),
        };

        info!(
            "Cluster initialized: {} nodes, Byzantine threshold: {}", 
            total_nodes, byzantine_threshold
        );

        Ok(Self {
            node_id,
            runtime,
            consensus_node: Arc::new(Mutex::new(consensus_node_guard)),
            byzantine_guard,
            state_manager,
            state_validator,
            pending_operations: Arc::new(RwLock::new(HashMap::new())),
            operation_callbacks: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(OrchestrationMetrics::default())),
            consensus_sender,
            operation_sequence: Arc::new(RwLock::new(0)),
            cluster_status: Arc::new(RwLock::new(cluster_status)),
        })
    }

    /// Create a new container with Byzantine fault-tolerant consensus
    ///
    /// This method coordinates container creation across the cluster, ensuring
    /// all nodes agree on the container specification and resource allocation
    /// before execution begins.
    ///
    /// # Arguments
    ///
    /// * `spec` - Container specification including image, resources, and configuration
    ///
    /// # Returns
    ///
    /// Container resource identifier if consensus is achieved and creation succeeds
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Container specification validation fails
    /// - Consensus cannot be achieved within timeout
    /// - Byzantine faults prevent operation completion
    /// - Underlying container runtime fails
    #[instrument(skip(self, spec), fields(container_image = %spec.image.name))]
    pub async fn create_container(&self, spec: ContainerSpec) -> Result<ResourceId> {
        let operation_start = Instant::now();
        let operation_id = self.next_operation_id().await;

        info!(
            operation_id = operation_id,
            "Initiating consensus for container creation: {}", spec.image.name
        );

        // Create consensus operation
        let operation = ContainerConsensusOperation::CreateContainer {
            operation_id,
            spec: spec.clone(),
            initiator: self.node_id,
            timestamp: Timestamp::now(),
        };

        // Submit operation for consensus
        let result = self.execute_consensus_operation(operation).await;

        // Update metrics
        {
            let mut metrics = self.metrics.write().unwrap();
            metrics.operations_processed += 1;
            
            let execution_time = operation_start.elapsed().as_millis() as f64;
            metrics.avg_execution_time_ms = 
                (metrics.avg_execution_time_ms * (metrics.operations_processed - 1) as f64 + execution_time) 
                / metrics.operations_processed as f64;

            if result.is_ok() {
                metrics.operations_successful += 1;
            } else {
                metrics.operations_failed += 1;
            }
            
            metrics.last_updated = Some(Instant::now());
        }

        match result {
            Ok(ContainerOperationResult::ContainerCreated { container_id }) => {
                info!(
                    operation_id = operation_id,
                    container_id = %container_id,
                    duration_ms = operation_start.elapsed().as_millis(),
                    "Container creation consensus completed successfully"
                );
                Ok(container_id)
            }
            Ok(_) => {
                error!("Unexpected result type for container creation operation");
                Err(RuntimeError::ConsensusError { 
                    message: "Invalid operation result type".to_string() 
                }.into())
            }
            Err(e) => {
                error!(
                    operation_id = operation_id,
                    error = %e,
                    "Container creation consensus failed"
                );
                Err(e)
            }
        }
    }

    /// Start a container with cluster-wide consensus coordination
    #[instrument(skip(self), fields(container_id = %container_id))]
    pub async fn start_container(&self, container_id: &ResourceId) -> Result<()> {
        let operation_id = self.next_operation_id().await;

        info!(
            operation_id = operation_id,
            container_id = %container_id,
            "Initiating consensus for container start"
        );

        let operation = ContainerConsensusOperation::StartContainer {
            operation_id,
            container_id: container_id.clone(),
            initiator: self.node_id,
            timestamp: Timestamp::now(),
        };

        match self.execute_consensus_operation(operation).await? {
            ContainerOperationResult::ContainerStarted => {
                info!(
                    container_id = %container_id,
                    "Container start consensus completed successfully"
                );
                Ok(())
            }
            _ => Err(RuntimeError::ConsensusError { 
                message: "Invalid result type for start operation".to_string() 
            }.into()),
        }
    }

    /// Stop a container with consensus coordination
    #[instrument(skip(self), fields(container_id = %container_id))]
    pub async fn stop_container(
        &self, 
        container_id: &ResourceId, 
        timeout: Option<Duration>
    ) -> Result<()> {
        let operation_id = self.next_operation_id().await;

        info!(
            operation_id = operation_id,
            container_id = %container_id,
            "Initiating consensus for container stop"
        );

        let operation = ContainerConsensusOperation::StopContainer {
            operation_id,
            container_id: container_id.clone(),
            timeout,
            initiator: self.node_id,
            timestamp: Timestamp::now(),
        };

        match self.execute_consensus_operation(operation).await? {
            ContainerOperationResult::ContainerStopped => {
                info!(
                    container_id = %container_id,
                    "Container stop consensus completed successfully"
                );
                Ok(())
            }
            _ => Err(RuntimeError::ConsensusError { 
                message: "Invalid result type for stop operation".to_string() 
            }.into()),
        }
    }

    /// Scale container resources with consensus agreement
    #[instrument(skip(self), fields(container_id = %container_id))]
    pub async fn scale_container(
        &self, 
        container_id: &ResourceId, 
        replicas: u32
    ) -> Result<()> {
        let operation_id = self.next_operation_id().await;

        info!(
            operation_id = operation_id,
            container_id = %container_id,
            replicas = replicas,
            "Initiating consensus for container scaling"
        );

        let operation = ContainerConsensusOperation::ScaleContainer {
            operation_id,
            container_id: container_id.clone(),
            replicas,
            initiator: self.node_id,
            timestamp: Timestamp::now(),
        };

        match self.execute_consensus_operation(operation).await? {
            ContainerOperationResult::ContainerScaled => {
                info!(
                    container_id = %container_id,
                    replicas = replicas,
                    "Container scaling consensus completed successfully"
                );
                Ok(())
            }
            _ => Err(RuntimeError::ConsensusError { 
                message: "Invalid result type for scale operation".to_string() 
            }.into()),
        }
    }

    /// Execute a consensus operation with full Byzantine fault tolerance
    ///
    /// This is the core method that coordinates container operations through
    /// the PBFT consensus protocol, ensuring all honest nodes agree before
    /// any state changes are applied.
    #[instrument(skip(self, operation))]
    async fn execute_consensus_operation(
        &self,
        operation: ContainerConsensusOperation,
    ) -> Result<ContainerOperationResult> {
        let operation_id = operation.operation_id();
        let consensus_start = Instant::now();

        // Create pending operation tracker
        let pending_op = PendingContainerOperation {
            operation_id,
            operation: operation.clone(),
            initiator: self.node_id,
            submitted_at: consensus_start,
            consensus_state: ConsensusPhase::Submitted,
            retry_count: 0,
            max_retries: 3,
        };

        // Register pending operation
        {
            let mut pending = self.pending_operations.write().unwrap();
            pending.insert(operation_id, pending_op);
        }

        // Update in-consensus metrics
        {
            let mut metrics = self.metrics.write().unwrap();
            metrics.operations_in_consensus += 1;
        }

        // Create client request for consensus
        let operation_bytes = bincode::serialize(&operation)
            .map_err(|e| RuntimeError::SerializationError { 
                message: format!("Failed to serialize operation: {}", e) 
            })?;

        let client_request = ClientRequest::new(
            self.node_id,
            operation_id,
            operation_bytes,
        );

        // Submit to consensus
        let consensus_result = {
            let mut consensus = self.consensus_node.lock().await;
            consensus.handle_client_request(client_request).await
        };

        match consensus_result {
            Ok(_) => {
                // Wait for consensus completion with timeout
                let result = self.wait_for_consensus_completion(operation_id).await;
                
                // Update metrics
                {
                    let mut metrics = self.metrics.write().unwrap();
                    metrics.operations_in_consensus = metrics.operations_in_consensus.saturating_sub(1);
                    
                    let consensus_time = consensus_start.elapsed().as_millis() as f64;
                    if metrics.avg_consensus_time_ms == 0.0 {
                        metrics.avg_consensus_time_ms = consensus_time;
                    } else {
                        metrics.avg_consensus_time_ms = 
                            (metrics.avg_consensus_time_ms * 0.9) + (consensus_time * 0.1);
                    }
                }

                result
            }
            Err(e) => {
                error!(
                    operation_id = operation_id,
                    error = %e,
                    "Failed to submit operation to consensus"
                );

                // Clean up pending operation
                {
                    let mut pending = self.pending_operations.write().unwrap();
                    pending.remove(&operation_id);
                }

                {
                    let mut metrics = self.metrics.write().unwrap();
                    metrics.operations_in_consensus = metrics.operations_in_consensus.saturating_sub(1);
                    metrics.operations_failed += 1;
                }

                Err(RuntimeError::ConsensusError { 
                    message: format!("Consensus submission failed: {}", e) 
                }.into())
            }
        }
    }

    /// Wait for consensus operation to complete with timeout handling
    #[instrument(skip(self))]
    async fn wait_for_consensus_completion(
        &self,
        operation_id: u64,
    ) -> Result<ContainerOperationResult> {
        const CONSENSUS_TIMEOUT: Duration = Duration::from_secs(30);
        let timeout_deadline = Instant::now() + CONSENSUS_TIMEOUT;

        loop {
            // Check if operation completed
            {
                let pending = self.pending_operations.read().unwrap();
                if let Some(op) = pending.get(&operation_id) {
                    match op.consensus_state {
                        ConsensusPhase::Executed => {
                            // Operation completed successfully, get result from state manager
                            drop(pending); // Release lock before async call
                            
                            let result = self.state_manager.get_operation_result(operation_id).await
                                .ok_or_else(|| RuntimeError::ConsensusError { 
                                    message: "Operation result not found".to_string() 
                                })?;
                            
                            // Clean up pending operation
                            {
                                let mut pending_mut = self.pending_operations.write().unwrap();
                                pending_mut.remove(&operation_id);
                            }
                            
                            return Ok(result);
                        }
                        ConsensusPhase::Failed => {
                            drop(pending); // Release lock
                            
                            // Clean up failed operation
                            {
                                let mut pending_mut = self.pending_operations.write().unwrap();
                                pending_mut.remove(&operation_id);
                            }
                            
                            return Err(RuntimeError::ConsensusError { 
                                message: "Operation failed consensus".to_string() 
                            }.into());
                        }
                        _ => {
                            // Still in progress, continue waiting
                        }
                    }
                } else {
                    return Err(RuntimeError::ConsensusError { 
                        message: "Operation tracking lost".to_string() 
                    }.into());
                }
            }

            // Check timeout
            if Instant::now() > timeout_deadline {
                warn!(
                    operation_id = operation_id,
                    "Consensus operation timed out"
                );

                // Update timeout metrics
                {
                    let mut metrics = self.metrics.write().unwrap();
                    metrics.consensus_timeouts += 1;
                }

                return Err(RuntimeError::ConsensusTimeout { 
                    operation_id,
                    timeout: CONSENSUS_TIMEOUT,
                }.into());
            }

            // Brief sleep to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Get the next operation sequence number
    async fn next_operation_id(&self) -> u64 {
        let mut sequence = self.operation_sequence.write().unwrap();
        *sequence += 1;
        *sequence
    }

    /// Get current orchestration metrics
    pub fn get_metrics(&self) -> OrchestrationMetrics {
        self.metrics.read().unwrap().clone()
    }

    /// Get cluster status and health information
    pub fn get_cluster_status(&self) -> ClusterStatus {
        self.cluster_status.read().unwrap().clone()
    }

    /// Get container state validation metrics
    pub fn get_validation_metrics(&self) -> ValidationMetrics {
        self.state_validator.get_validation_metrics()
    }

    /// Perform periodic maintenance tasks
    #[instrument(skip(self))]
    pub async fn periodic_maintenance(&mut self) -> Result<()> {
        debug!("Performing orchestrator periodic maintenance");

        // Cleanup stale pending operations
        self.cleanup_stale_operations().await?;

        // Update cluster health assessment
        self.update_cluster_health().await?;

        // Perform Byzantine guard maintenance
        {
            let mut guard = self.byzantine_guard.write().await;
            guard.periodic_maintenance().await
                .map_err(|e| RuntimeError::ConsensusError { 
                    message: format!("Byzantine guard maintenance failed: {}", e) 
                })?;
        }

        // Synchronize container state across cluster
        self.state_manager.periodic_synchronization().await
            .map_err(|e| RuntimeError::StateError { 
                message: format!("State synchronization failed: {}", e) 
            })?;

        // Cleanup old validated states
        self.state_validator.cleanup_old_states(Duration::from_secs(3600)); // 1 hour

        Ok(())
    }

    /// Clean up operations that have exceeded maximum wait time
    async fn cleanup_stale_operations(&self) -> Result<()> {
        const STALE_OPERATION_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes
        let cutoff_time = Instant::now() - STALE_OPERATION_TIMEOUT;

        let stale_operations: Vec<u64> = {
            let pending = self.pending_operations.read().unwrap();
            pending
                .iter()
                .filter(|(_, op)| op.submitted_at < cutoff_time)
                .map(|(id, _)| *id)
                .collect()
        };

        if !stale_operations.is_empty() {
            warn!(
                "Cleaning up {} stale operations",
                stale_operations.len()
            );

            let mut pending = self.pending_operations.write().unwrap();
            for operation_id in stale_operations {
                pending.remove(&operation_id);
            }
        }

        Ok(())
    }

    /// Update cluster health status based on current conditions
    async fn update_cluster_health(&self) -> Result<()> {
        let quarantined_nodes = {
            let guard = self.byzantine_guard.read().await;
            guard.get_quarantined_nodes().await
                .map_err(|e| RuntimeError::ConsensusError { 
                    message: format!("Failed to get quarantined nodes: {}", e) 
                })?
        };

        let mut status = self.cluster_status.write().unwrap();
        status.quarantined_nodes = quarantined_nodes.clone();
        status.active_nodes = status.total_nodes - quarantined_nodes.len();
        
        // Assess health based on Byzantine fault tolerance
        let byzantine_faults = quarantined_nodes.len();
        status.health_status = if byzantine_faults == 0 {
            ClusterHealth::Healthy
        } else if byzantine_faults < status.byzantine_threshold {
            ClusterHealth::Warning
        } else if byzantine_faults == status.byzantine_threshold {
            ClusterHealth::Critical
        } else {
            ClusterHealth::Compromised
        };

        status.last_health_check = Some(Instant::now());

        if status.health_status != ClusterHealth::Healthy {
            warn!(
                byzantine_faults = byzantine_faults,
                threshold = status.byzantine_threshold,
                health = ?status.health_status,
                "Cluster health degraded due to Byzantine faults"
            );
        }

        Ok(())
    }
}

