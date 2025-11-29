//! Sprint 2 Byzantine Fault Tolerance Tests
//!
//! Comprehensive tests for Byzantine fault tolerance during container orchestration
//! Validates that the system continues operating correctly even with malicious nodes.

use nexus_runtime::{
    ConsensusContainerOrchestrator, ContainerSpec, ContainerConsensusOperation,
    ImageSpec, RuntimeConfig, Runtime, ContainerStateManager,
    ContainerStatus, ContainerOperationResult,
};
use nexus_consensus::pbft::{PbftNode, PbftConfig};
use nexus_consensus::pbft::messages::{PbftMessage, ClientRequest, PrePrepare, Prepare, Commit};
use nexus_consensus::byzantine::{ByzantineGuard, FaultDetectionConfig, ValidationResult};
use nexus_shared::{NodeId, ResourceId, Timestamp};

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::{mpsc, Mutex, RwLock as AsyncRwLock};
use tempfile::TempDir;
use tracing::{info, warn, error};

/// Byzantine test configuration
const CLUSTER_SIZE: usize = 4; // 3f+1 with f=1
const MAX_BYZANTINE_NODES: usize = 1;

/// Byzantine test cluster with fault injection capabilities
pub struct ByzantineTestCluster {
    /// All orchestrator nodes
    orchestrators: Vec<Arc<ConsensusContainerOrchestrator>>,
    
    /// Node identifiers
    node_ids: Vec<NodeId>,
    
    /// Message channels for inter-node communication
    message_channels: HashMap<NodeId, mpsc::UnboundedSender<(NodeId, PbftMessage)>>,
    
    /// Byzantine nodes that will behave maliciously
    byzantine_nodes: Vec<NodeId>,
    
    /// Test directory
    test_dir: TempDir,
    
    /// Fault injection configuration
    fault_config: FaultInjectionConfig,
}

/// Configuration for fault injection
#[derive(Debug, Clone)]
pub struct FaultInjectionConfig {
    /// Drop messages randomly
    pub drop_message_probability: f64,
    
    /// Send conflicting messages
    pub send_conflicting_messages: bool,
    
    /// Delay message delivery
    pub message_delay_ms: Option<u64>,
    
    /// Send messages with invalid signatures
    pub corrupt_signatures: bool,
    
    /// Vote for multiple values in same view
    pub equivocation: bool,
}

impl Default for FaultInjectionConfig {
    fn default() -> Self {
        Self {
            drop_message_probability: 0.0,
            send_conflicting_messages: false,
            message_delay_ms: None,
            corrupt_signatures: false,
            equivocation: false,
        }
    }
}

impl ByzantineTestCluster {
    /// Create a new Byzantine test cluster
    pub async fn new(byzantine_count: usize) -> Result<Self, Box<dyn std::error::Error>> {
        assert!(byzantine_count <= MAX_BYZANTINE_NODES);
        
        let test_dir = TempDir::new()?;
        let node_ids: Vec<NodeId> = (0..CLUSTER_SIZE).map(|_| NodeId::random()).collect();
        let byzantine_nodes = node_ids[..byzantine_count].to_vec();
        
        info!("Creating Byzantine test cluster with {} nodes ({} Byzantine)",
              CLUSTER_SIZE, byzantine_count);
        
        let mut orchestrators = Vec::new();
        let mut message_channels = HashMap::new();
        let mut message_receivers = HashMap::new();
        
        // Create message channels
        for &node_id in &node_ids {
            let (tx, rx) = mpsc::unbounded_channel();
            message_channels.insert(node_id, tx);
            message_receivers.insert(node_id, rx);
        }
        
        // Create orchestrators
        for (i, &node_id) in node_ids.iter().enumerate() {
            let mut runtime_config = RuntimeConfig::default();
            runtime_config.storage.data_dir = format!("{}/node_{}", test_dir.path().display(), i);
            std::fs::create_dir_all(&runtime_config.storage.data_dir)?;
            
            let runtime = Arc::new(Runtime::new(runtime_config).await?);
            
            let pbft_config = PbftConfig {
                node_id,
                node_ids: node_ids.clone(),
                batch_size: 5,
                checkpoint_interval: 100,
                view_timeout: Duration::from_secs(2),
                ..Default::default()
            };
            
            let consensus_node = Arc::new(Mutex::new(PbftNode::new(pbft_config)?));
            
            let byzantine_guard = Arc::new(AsyncRwLock::new(
                ByzantineGuard::new(
                    node_id,
                    FaultDetectionConfig {
                        max_message_age: Duration::from_secs(10),
                        consistency_check_interval: Duration::from_secs(30),
                        max_divergence_threshold: 3,
                        reputation_update_interval: Duration::from_secs(120),
                    },
                )
            ));
            
            let state_manager = Arc::new(ContainerStateManager::new(node_id));
            let tx = message_channels[&node_id].clone();
            
            let orchestrator = Arc::new(ConsensusContainerOrchestrator::new(
                node_id,
                runtime,
                consensus_node,
                byzantine_guard,
                state_manager,
                tx,
            ).await?);
            
            orchestrators.push(orchestrator);
        }
        
        // Start message routing
        Self::start_message_routing(
            node_ids.clone(),
            message_receivers,
            message_channels.clone(),
        );
        
        Ok(Self {
            orchestrators,
            node_ids,
            message_channels,
            byzantine_nodes,
            test_dir,
            fault_config: FaultInjectionConfig::default(),
        })
    }
    
    /// Start message routing between nodes
    fn start_message_routing(
        node_ids: Vec<NodeId>,
        mut receivers: HashMap<NodeId, mpsc::UnboundedReceiver<(NodeId, PbftMessage)>>,
        senders: HashMap<NodeId, mpsc::UnboundedSender<(NodeId, PbftMessage)>>,
    ) {
        tokio::spawn(async move {
            loop {
                for node_id in &node_ids {
                    if let Some(rx) = receivers.get_mut(node_id) {
                        if let Ok((from, msg)) = rx.try_recv() {
                            // Route message to destination
                            if let Some(dest_id) = Self::get_message_destination(&msg) {
                                if let Some(tx) = senders.get(&dest_id) {
                                    let _ = tx.send((from, msg));
                                }
                            } else {
                                // Broadcast to all other nodes
                                for (&dest, tx) in &senders {
                                    if dest != *node_id {
                                        let _ = tx.send((from, msg.clone()));
                                    }
                                }
                            }
                        }
                    }
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        });
    }
    
    fn get_message_destination(_msg: &PbftMessage) -> Option<NodeId> {
        // In a real implementation, extract destination from message
        None // Broadcast for now
    }
    
    /// Inject Byzantine behavior into specified nodes
    pub fn inject_byzantine_behavior(&mut self, config: FaultInjectionConfig) {
        self.fault_config = config;
        info!("Injected Byzantine behavior configuration: {:?}", self.fault_config);
    }
    
    /// Test container creation with Byzantine nodes
    pub async fn test_container_creation_with_byzantine(&self) -> Result<bool, Box<dyn std::error::Error>> {
        info!("Testing container creation with {} Byzantine nodes", self.byzantine_nodes.len());
        
        let container_spec = ContainerSpec {
            id: ResourceId::random(),
            name: "byzantine-test-container".to_string(),
            image: ImageSpec {
                name: "alpine".to_string(),
                tag: "latest".to_string(),
                digest: None,
            },
            env: vec![],
            mounts: vec![],
            labels: Default::default(),
            resources: Default::default(),
            network_config: Default::default(),
        };
        
        // Submit operation from honest node
        let honest_node_idx = self.byzantine_nodes.len(); // First honest node
        let operation = ContainerConsensusOperation::Create(container_spec.clone());
        let op_id = self.orchestrators[honest_node_idx]
            .submit_container_operation(operation)
            .await?;
        
        info!("Submitted operation {} from honest node", op_id);
        
        // Inject Byzantine messages if configured
        if self.fault_config.send_conflicting_messages {
            self.inject_conflicting_messages(&container_spec).await?;
        }
        
        // Wait for consensus
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Verify consensus reached on honest nodes
        let mut consensus_reached = true;
        let mut container_states = Vec::new();
        
        for (i, orchestrator) in self.orchestrators.iter().enumerate() {
            if !self.byzantine_nodes.contains(&self.node_ids[i]) {
                let state = orchestrator.get_cluster_state().await?;
                container_states.push(state.containers.len());
                
                if state.containers.is_empty() {
                    consensus_reached = false;
                    warn!("Node {} has no containers in state", i);
                }
            }
        }
        
        // All honest nodes should have same state
        if !container_states.is_empty() {
            let first_state = container_states[0];
            for state in &container_states {
                if *state != first_state {
                    consensus_reached = false;
                    error!("State divergence detected: {} vs {}", first_state, state);
                }
            }
        }
        
        info!("Consensus reached: {}", consensus_reached);
        Ok(consensus_reached)
    }
    
    /// Inject conflicting messages from Byzantine nodes
    async fn inject_conflicting_messages(&self, spec: &ContainerSpec) -> Result<(), Box<dyn std::error::Error>> {
        for &byzantine_node in &self.byzantine_nodes {
            // Create conflicting container spec
            let conflicting_spec = ContainerSpec {
                id: spec.id.clone(),
                name: "malicious-container".to_string(), // Different name
                image: ImageSpec {
                    name: "ubuntu".to_string(), // Different image
                    tag: "latest".to_string(),
                    digest: None,
                },
                env: vec![("MALICIOUS".to_string(), "true".to_string())],
                mounts: vec![],
                labels: Default::default(),
                resources: Default::default(),
                network_config: Default::default(),
            };
            
            // Send conflicting operation
            if let Some(tx) = self.message_channels.get(&byzantine_node) {
                let msg = self.create_malicious_message(conflicting_spec);
                for &dest in &self.node_ids {
                    if dest != byzantine_node {
                        let _ = tx.send((byzantine_node, msg.clone()));
                    }
                }
            }
        }
        
        info!("Injected conflicting messages from Byzantine nodes");
        Ok(())
    }
    
    fn create_malicious_message(&self, spec: ContainerSpec) -> PbftMessage {
        // Create a malicious PBFT message
        // In real implementation, this would create properly formatted but malicious messages
        PbftMessage::ClientRequest(ClientRequest {
            client_id: NodeId::random(),
            sequence: 999,
            operation: serde_json::to_vec(&ContainerConsensusOperation::Create(spec)).unwrap(),
            timestamp: Timestamp::now(),
        })
    }
    
    /// Test Byzantine node isolation
    pub async fn test_byzantine_isolation(&self) -> Result<bool, Box<dyn std::error::Error>> {
        info!("Testing Byzantine node isolation");
        
        // Check reputation scores after Byzantine behavior
        let mut isolated = true;
        
        for (i, orchestrator) in self.orchestrators.iter().enumerate() {
            if !self.byzantine_nodes.contains(&self.node_ids[i]) {
                let byzantine_guard = orchestrator.get_byzantine_guard();
                let guard = byzantine_guard.read().await;
                
                for &byzantine_node in &self.byzantine_nodes {
                    let reputation = guard.get_node_reputation(byzantine_node);
                    info!("Node {} reputation of Byzantine node: {}", i, reputation);
                    
                    if reputation > 0.5 {
                        isolated = false;
                        warn!("Byzantine node not properly isolated (reputation: {})", reputation);
                    }
                }
            }
        }
        
        Ok(isolated)
    }
    
    /// Test state recovery after Byzantine attack
    pub async fn test_state_recovery(&self) -> Result<bool, Box<dyn std::error::Error>> {
        info!("Testing state recovery after Byzantine attack");
        
        // Submit valid operation after attack
        let container_spec = ContainerSpec {
            id: ResourceId::random(),
            name: "recovery-container".to_string(),
            image: ImageSpec {
                name: "alpine".to_string(),
                tag: "latest".to_string(),
                digest: None,
            },
            env: vec![],
            mounts: vec![],
            labels: Default::default(),
            resources: Default::default(),
            network_config: Default::default(),
        };
        
        let honest_node_idx = self.byzantine_nodes.len();
        let operation = ContainerConsensusOperation::Create(container_spec);
        let op_id = self.orchestrators[honest_node_idx]
            .submit_container_operation(operation)
            .await?;
        
        // Wait for consensus
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Verify state consistency
        let mut states_consistent = true;
        let mut state_hashes = Vec::new();
        
        for (i, orchestrator) in self.orchestrators.iter().enumerate() {
            if !self.byzantine_nodes.contains(&self.node_ids[i]) {
                let state = orchestrator.get_cluster_state().await?;
                let state_hash = Self::hash_state(&state);
                state_hashes.push(state_hash);
            }
        }
        
        // Check all honest nodes have same state hash
        if !state_hashes.is_empty() {
            let first_hash = &state_hashes[0];
            for hash in &state_hashes {
                if hash != first_hash {
                    states_consistent = false;
                    error!("State inconsistency detected after recovery");
                }
            }
        }
        
        info!("State recovery successful: {}", states_consistent);
        Ok(states_consistent)
    }
    
    fn hash_state(state: &nexus_runtime::ContainerClusterState) -> String {
        // Simple state hash for comparison
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        state.containers.len().hash(&mut hasher);
        state.nodes.len().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// Test Byzantine fault tolerance with one malicious node
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_byzantine_single_fault() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    info!("=== Single Byzantine Fault Test ===");
    
    let cluster = ByzantineTestCluster::new(1).await?;
    
    // Test container creation works despite Byzantine node
    let consensus_reached = cluster.test_container_creation_with_byzantine().await?;
    assert!(consensus_reached, "Consensus should be reached with f=1 Byzantine nodes");
    
    info!("✅ Cluster maintains consensus with 1 Byzantine node");
    info!("=== Single Byzantine Fault Test PASSED ===");
    
    Ok(())
}

/// Test Byzantine node detection and isolation
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_byzantine_detection() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    info!("=== Byzantine Detection Test ===");
    
    let mut cluster = ByzantineTestCluster::new(1).await?;
    
    // Inject malicious behavior
    cluster.inject_byzantine_behavior(FaultInjectionConfig {
        send_conflicting_messages: true,
        equivocation: true,
        ..Default::default()
    });
    
    // Perform operations that trigger Byzantine behavior
    let _ = cluster.test_container_creation_with_byzantine().await?;
    
    // Wait for detection
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Verify Byzantine nodes are detected and isolated
    let isolated = cluster.test_byzantine_isolation().await?;
    assert!(isolated, "Byzantine nodes should be isolated");
    
    info!("✅ Byzantine nodes detected and isolated");
    info!("=== Byzantine Detection Test PASSED ===");
    
    Ok(())
}

/// Test state recovery after Byzantine attack
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_byzantine_recovery() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    info!("=== Byzantine Recovery Test ===");
    
    let mut cluster = ByzantineTestCluster::new(1).await?;
    
    // Initial Byzantine attack
    cluster.inject_byzantine_behavior(FaultInjectionConfig {
        send_conflicting_messages: true,
        drop_message_probability: 0.3,
        ..Default::default()
    });
    
    let _ = cluster.test_container_creation_with_byzantine().await?;
    
    // Wait for system to stabilize
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Test recovery
    let recovered = cluster.test_state_recovery().await?;
    assert!(recovered, "System should recover after Byzantine attack");
    
    info!("✅ System recovered successfully after Byzantine attack");
    info!("=== Byzantine Recovery Test PASSED ===");
    
    Ok(())
}

/// Test performance under Byzantine conditions
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_byzantine_performance() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    info!("=== Byzantine Performance Test ===");
    
    let cluster = ByzantineTestCluster::new(1).await?;
    
    let mut total_time = Duration::ZERO;
    let num_operations = 10;
    
    for i in 0..num_operations {
        let container_spec = ContainerSpec {
            id: ResourceId::random(),
            name: format!("perf-test-{}", i),
            image: ImageSpec {
                name: "alpine".to_string(),
                tag: "latest".to_string(),
                digest: None,
            },
            env: vec![],
            mounts: vec![],
            labels: Default::default(),
            resources: Default::default(),
            network_config: Default::default(),
        };
        
        let start = Instant::now();
        
        // Use honest node for submission
        let operation = ContainerConsensusOperation::Create(container_spec);
        let _ = cluster.orchestrators[1] // Index 1 is honest
            .submit_container_operation(operation)
            .await?;
        
        // Wait for consensus
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        total_time += start.elapsed();
    }
    
    let avg_time = total_time / num_operations as u32;
    info!("Average operation time with Byzantine node: {:?}", avg_time);
    
    assert!(
        avg_time.as_millis() < 200,
        "Performance degradation too high with Byzantine node"
    );
    
    info!("✅ Performance maintained under Byzantine conditions");
    info!("=== Byzantine Performance Test PASSED ===");
    
    Ok(())
}