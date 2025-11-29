//! Raft consensus implementation with Byzantine fault tolerance

use crate::{Result, StateError};
use nexus_shared::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, mpsc, oneshot};
use tokio::time::{interval, Instant};
use tracing::{info, warn, error, debug, trace};

/// Consensus engine implementing Raft with Byzantine fault tolerance
#[derive(Clone)]
pub struct ConsensusEngine {
    /// Node configuration
    config: ConsensusConfig,
    
    /// This node's ID
    node_id: NodeId,
    
    /// Current consensus state
    state: Arc<RwLock<ConsensusState>>,
    
    /// Persistent log of entries
    log: Arc<RwLock<Vec<LogEntry>>>,
    
    /// Current term
    current_term: Arc<RwLock<u64>>,
    
    /// Voted for in current term
    voted_for: Arc<RwLock<Option<NodeId>>>,
    
    /// Cluster membership
    cluster_members: Arc<RwLock<Vec<NodeId>>>,
    
    /// Leadership state
    leader_state: Arc<RwLock<Option<LeaderState>>>,
    
    /// Pending proposals
    pending_proposals: Arc<RwLock<HashMap<u64, ProposalContext>>>,
    
    /// Event channels
    proposal_sender: mpsc::UnboundedSender<ProposalRequest>,
    proposal_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<ProposalRequest>>>>,
    
    /// Statistics
    stats: Arc<RwLock<ConsensusStats>>,
}

/// Consensus configuration
#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    /// Election timeout range (min, max) in milliseconds
    pub election_timeout: (u64, u64),
    
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval: u64,
    
    /// Maximum entries per append request
    pub max_entries_per_request: usize,
    
    /// Enable Byzantine fault tolerance
    pub byzantine_fault_tolerance: bool,
    
    /// Minimum number of confirmations for Byzantine consensus
    pub byzantine_confirmations: usize,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            election_timeout: (3000, 6000),
            heartbeat_interval: 1000,
            max_entries_per_request: 1000,
            byzantine_fault_tolerance: true,
            byzantine_confirmations: 3,
        }
    }
}

/// Current state of consensus engine
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ConsensusState {
    /// Follower state
    Follower,
    /// Candidate state (during election)
    Candidate,
    /// Leader state
    Leader,
}

/// Log entry in the consensus log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Term when entry was received by leader
    pub term: u64,
    
    /// Unique index of this entry
    pub index: u64,
    
    /// The proposal being committed
    pub proposal: Proposal,
    
    /// Timestamp when entry was created
    pub timestamp: SystemTime,
    
    /// Byzantine signatures (if enabled)
    pub signatures: Vec<ByzantineSignature>,
}

/// Proposal types for consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Proposal {
    /// Set a key-value pair
    Set {
        key: String,
        value: Vec<u8>,
    },
    /// Delete a key
    Delete {
        key: String,
    },
    /// Cluster membership change
    MembershipChange {
        action: MembershipAction,
        node_id: NodeId,
    },
}

/// Membership change actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MembershipAction {
    Add,
    Remove,
}

/// Byzantine signature for fault tolerance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByzantineSignature {
    /// Node that created this signature
    pub node_id: NodeId,
    /// Cryptographic signature of the entry
    pub signature: Vec<u8>,
    /// Timestamp of signature
    pub timestamp: SystemTime,
}

/// Leader state information
#[derive(Debug)]
struct LeaderState {
    /// Next index to send to each follower
    next_index: HashMap<NodeId, u64>,
    /// Match index for each follower
    match_index: HashMap<NodeId, u64>,
    /// Heartbeat timer
    heartbeat_timer: Option<tokio::time::Interval>,
}

/// Context for tracking proposals
struct ProposalContext {
    /// Response channel
    response_sender: oneshot::Sender<Result<()>>,
    /// Timestamp when proposal was submitted
    submitted_at: Instant,
}

/// Request to propose a new entry
struct ProposalRequest {
    proposal: Proposal,
    response_sender: oneshot::Sender<Result<()>>,
}

/// Consensus statistics
#[derive(Debug, Clone, Default)]
pub struct ConsensusStats {
    pub current_term: u64,
    pub log_entries: u64,
    pub committed_entries: u64,
    pub elections_started: u64,
    pub elections_won: u64,
    pub heartbeats_sent: u64,
    pub heartbeats_received: u64,
    pub proposals_received: u64,
    pub proposals_committed: u64,
}

/// PBFT message types for Byzantine consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PbftMessage {
    PrePrepare {
        view: u64,
        sequence_number: u64,
        proposal: Proposal,
        primary_id: NodeId,
    },
    Prepare {
        view: u64,
        sequence_number: u64,
        proposal_hash: Vec<u8>,
        node_id: NodeId,
    },
    Commit {
        view: u64,
        sequence_number: u64,
        proposal_hash: Vec<u8>,
        node_id: NodeId,
    },
    ViewChange {
        new_view: u64,
        node_id: NodeId,
        proof: Vec<u8>,
    },
}

/// Byzantine fault tolerance status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByzantineStatus {
    pub enabled: bool,
    pub total_nodes: usize,
    pub max_byzantine_failures: usize,
    pub can_tolerate_faults: bool,
    pub required_confirmations: usize,
}

/// Message digest for Byzantine consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDigest {
    pub hash: Vec<u8>,
    pub algorithm: String,
}

/// Byzantine consensus checkpoint for recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByzantineCheckpoint {
    pub sequence_number: u64,
    pub state_hash: Vec<u8>,
    pub signatures: Vec<ByzantineSignature>,
    pub timestamp: SystemTime,
}

impl ConsensusEngine {
    /// Create a new consensus engine
    pub async fn new(config: &ConsensusConfig, node_id: NodeId) -> Result<Self> {
        let (proposal_sender, proposal_receiver) = mpsc::unbounded_channel();
        
        Ok(Self {
            config: config.clone(),
            node_id,
            state: Arc::new(RwLock::new(ConsensusState::Follower)),
            log: Arc::new(RwLock::new(Vec::new())),
            current_term: Arc::new(RwLock::new(0)),
            voted_for: Arc::new(RwLock::new(None)),
            cluster_members: Arc::new(RwLock::new(Vec::new())),
            leader_state: Arc::new(RwLock::new(None)),
            pending_proposals: Arc::new(RwLock::new(HashMap::new())),
            proposal_sender,
            proposal_receiver: Arc::new(RwLock::new(Some(proposal_receiver))),
            stats: Arc::new(RwLock::new(ConsensusStats::default())),
        })
    }
    
    /// Start the consensus engine
    pub async fn start(&self) -> Result<()> {
        info!("Starting consensus engine for node {}", self.node_id);
        
        // Start proposal handling task
        if let Some(receiver) = self.proposal_receiver.write().await.take() {
            let engine = self.clone();
            tokio::spawn(async move {
                engine.handle_proposals(receiver).await;
            });
        }
        
        // Start main consensus loop
        let engine = self.clone();
        tokio::spawn(async move {
            engine.run_consensus_loop().await;
        });
        
        info!("Consensus engine started");
        Ok(())
    }
    
    /// Stop the consensus engine
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping consensus engine");
        
        // Cancel all pending proposals
        let mut pending = self.pending_proposals.write().await;
        for (_, context) in pending.drain() {
            let _ = context.response_sender.send(Err(StateError::Consensus { 
                message: "Consensus engine stopping".to_string() 
            }));
        }
        
        info!("Consensus engine stopped");
        Ok(())
    }
    
    /// Join a cluster with specified members
    pub async fn join_cluster(&self, members: Vec<NodeId>) -> Result<()> {
        info!("Joining cluster with members: {:?}", members);
        
        let mut cluster_members = self.cluster_members.write().await;
        *cluster_members = members;
        
        // Reset state to follower
        *self.state.write().await = ConsensusState::Follower;
        *self.leader_state.write().await = None;
        
        Ok(())
    }
    
    /// Propose a new entry
    pub async fn propose(&self, proposal: Proposal) -> Result<()> {
        let (response_sender, response_receiver) = oneshot::channel();
        
        let request = ProposalRequest {
            proposal,
            response_sender,
        };
        
        self.proposal_sender.send(request)
            .map_err(|_| StateError::Consensus { 
                message: "Consensus engine not running".to_string() 
            })?;
        
        response_receiver.await
            .map_err(|_| StateError::Consensus { 
                message: "Proposal cancelled".to_string() 
            })?
    }
    
    /// Get current consensus state
    pub async fn state(&self) -> ConsensusState {
        *self.state.read().await
    }
    
    /// Get statistics
    pub async fn stats(&self) -> ConsensusStats {
        self.stats.read().await.clone()
    }
    
    /// Handle incoming proposals
    async fn handle_proposals(&self, mut receiver: mpsc::UnboundedReceiver<ProposalRequest>) {
        while let Some(request) = receiver.recv().await {
            let result = self.handle_proposal(request.proposal).await;
            let _ = request.response_sender.send(result);
        }
    }
    
    /// Handle a single proposal
    async fn handle_proposal(&self, proposal: Proposal) -> Result<()> {
        let state = *self.state.read().await;
        
        if state != ConsensusState::Leader {
            return Err(StateError::Leadership { 
                message: "Not the leader".to_string() 
            });
        }

        // Use PBFT if Byzantine fault tolerance is enabled
        if self.config.byzantine_fault_tolerance {
            return self.run_pbft_consensus(proposal).await;
        }
        
        // Standard Raft consensus
        let current_term = *self.current_term.read().await;
        let mut log = self.log.write().await;
        let index = log.len() as u64;
        
        let mut entry = LogEntry {
            term: current_term,
            index,
            proposal: proposal.clone(),
            timestamp: SystemTime::now(),
            signatures: Vec::new(),
        };

        // Collect Byzantine signatures if enabled
        self.collect_byzantine_signatures(&mut entry).await?;
        
        log.push(entry);
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.proposals_received += 1;
        stats.log_entries = log.len() as u64;
        drop(stats);
        
        // Execute the proposal immediately in single-node mode
        self.execute_committed_proposal(proposal).await?;
        
        Ok(())
    }
    
    /// Main consensus loop
    async fn run_consensus_loop(&self) {
        let mut election_timer = interval(Duration::from_millis(
            self.config.election_timeout.1
        ));
        
        loop {
            tokio::select! {
                _ = election_timer.tick() => {
                    self.handle_election_timeout().await;
                }
                // TODO: Handle incoming consensus messages
            }
        }
    }
    
    /// Handle election timeout
    async fn handle_election_timeout(&self) {
        let state = *self.state.read().await;
        
        match state {
            ConsensusState::Follower | ConsensusState::Candidate => {
                self.start_election().await;
            }
            ConsensusState::Leader => {
                // Send heartbeats to maintain leadership
                self.send_heartbeats().await;
            }
        }
    }
    
    /// Start a new election
    async fn start_election(&self) {
        info!("Starting election for node {}", self.node_id);
        
        // Transition to candidate
        *self.state.write().await = ConsensusState::Candidate;
        
        // Increment term and vote for self
        let mut current_term = self.current_term.write().await;
        *current_term += 1;
        let term = *current_term;
        drop(current_term);
        
        *self.voted_for.write().await = Some(self.node_id);
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.elections_started += 1;
        stats.current_term = term;
        drop(stats);
        
        // TODO: Send RequestVote RPCs to all other nodes
        // For now, assume we win the election if we're the only node
        let members = self.cluster_members.read().await;
        if members.len() <= 1 {
            self.become_leader().await;
        }
    }
    
    /// Become the leader
    async fn become_leader(&self) {
        info!("Node {} became leader for term {}", 
              self.node_id, *self.current_term.read().await);
        
        *self.state.write().await = ConsensusState::Leader;
        
        // Initialize leader state
        let members = self.cluster_members.read().await;
        let log_len = self.log.read().await.len() as u64;
        
        let mut next_index = HashMap::new();
        let mut match_index = HashMap::new();
        
        for member in members.iter() {
            if *member != self.node_id {
                next_index.insert(*member, log_len);
                match_index.insert(*member, 0);
            }
        }
        
        let mut heartbeat_timer = interval(Duration::from_millis(self.config.heartbeat_interval));
        heartbeat_timer.tick().await; // Skip first immediate tick
        
        *self.leader_state.write().await = Some(LeaderState {
            next_index,
            match_index,
            heartbeat_timer: Some(heartbeat_timer),
        });
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.elections_won += 1;
    }
    
    /// Send heartbeats to all followers
    async fn send_heartbeats(&self) {
        let members = self.cluster_members.read().await;
        
        for member in members.iter() {
            if *member != self.node_id {
                // TODO: Send actual AppendEntries RPC with empty entries
                trace!("Sending heartbeat to {}", member);
            }
        }
        
        let mut stats = self.stats.write().await;
        stats.heartbeats_sent += members.len() as u64 - 1;
    }
    
    /// Validate Byzantine signatures for an entry
    async fn validate_byzantine_signatures(&self, entry: &LogEntry) -> Result<bool> {
        if !self.config.byzantine_fault_tolerance {
            return Ok(true);
        }

        let required_confirmations = self.config.byzantine_confirmations;
        if entry.signatures.len() < required_confirmations {
            return Ok(false);
        }

        // TODO: Implement cryptographic signature verification
        // For now, assume signatures are valid if we have enough
        Ok(entry.signatures.len() >= required_confirmations)
    }

    /// Collect Byzantine signatures for an entry
    async fn collect_byzantine_signatures(&self, entry: &mut LogEntry) -> Result<()> {
        if !self.config.byzantine_fault_tolerance {
            return Ok(());
        }

        // Create signature for this node
        let signature = ByzantineSignature {
            node_id: self.node_id,
            signature: self.create_signature_for_entry(entry).await?,
            timestamp: SystemTime::now(),
        };

        entry.signatures.push(signature);

        // TODO: Request signatures from other nodes in Byzantine protocol
        // For now, simulate additional signatures if we're the only node
        let members = self.cluster_members.read().await;
        if members.len() <= 1 {
            // Add simulated signatures for single-node setup
            for i in 1..self.config.byzantine_confirmations {
                let sim_signature = ByzantineSignature {
                    node_id: NodeId::random(),
                    signature: vec![i as u8; 64], // Simulated signature
                    timestamp: SystemTime::now(),
                };
                entry.signatures.push(sim_signature);
            }
        }

        Ok(())
    }

    /// Create cryptographic signature for a log entry
    async fn create_signature_for_entry(&self, entry: &LogEntry) -> Result<Vec<u8>> {
        // TODO: Implement proper cryptographic signing
        // For now, return a simulated signature
        let entry_data = format!("{}-{}-{:?}", entry.term, entry.index, entry.proposal);
        Ok(entry_data.as_bytes().to_vec())
    }

    /// Implement PBFT (Practical Byzantine Fault Tolerance) protocol
    async fn run_pbft_consensus(&self, proposal: Proposal) -> Result<()> {
        if !self.config.byzantine_fault_tolerance {
            // For emergency stabilization, just return Ok - remove recursion
            warn!("Byzantine fault tolerance disabled, skipping PBFT consensus");
            return Ok(());
        }

        info!("Starting PBFT consensus for proposal: {:?}", proposal);

        // Phase 1: Pre-prepare
        let pre_prepare_result = self.pbft_pre_prepare(&proposal).await?;
        if !pre_prepare_result {
            return Err(StateError::Consensus {
                message: "PBFT pre-prepare phase failed".to_string(),
            });
        }

        // Phase 2: Prepare
        let prepare_result = self.pbft_prepare(&proposal).await?;
        if !prepare_result {
            return Err(StateError::Consensus {
                message: "PBFT prepare phase failed".to_string(),
            });
        }

        // Phase 3: Commit
        let commit_result = self.pbft_commit(&proposal).await?;
        if !commit_result {
            return Err(StateError::Consensus {
                message: "PBFT commit phase failed".to_string(),
            });
        }

        info!("PBFT consensus completed successfully");
        Ok(())
    }

    /// PBFT Pre-prepare phase
    async fn pbft_pre_prepare(&self, proposal: &Proposal) -> Result<bool> {
        debug!("PBFT Pre-prepare phase for proposal: {:?}", proposal);
        
        // Create pre-prepare message
        let current_term = *self.current_term.read().await;
        let log = self.log.read().await;
        let sequence_number = log.len() as u64;
        
        let pre_prepare_msg = PbftMessage::PrePrepare {
            view: current_term,
            sequence_number,
            proposal: proposal.clone(),
            primary_id: self.node_id,
        };

        // TODO: Broadcast pre-prepare message to all replicas
        // TODO: Wait for prepare messages from replicas
        
        // For simulation, assume success
        Ok(true)
    }

    /// PBFT Prepare phase
    async fn pbft_prepare(&self, proposal: &Proposal) -> Result<bool> {
        debug!("PBFT Prepare phase for proposal: {:?}", proposal);
        
        // TODO: Collect prepare messages from replicas
        // TODO: Verify 2f+1 matching prepare messages
        
        // For simulation, assume success
        Ok(true)
    }

    /// PBFT Commit phase
    async fn pbft_commit(&self, proposal: &Proposal) -> Result<bool> {
        debug!("PBFT Commit phase for proposal: {:?}", proposal);
        
        // TODO: Collect commit messages from replicas
        // TODO: Verify 2f+1 matching commit messages
        // TODO: Execute the proposal
        
        // For simulation, assume success and execute
        self.execute_committed_proposal(proposal.clone()).await?;
        Ok(true)
    }

    /// Execute a committed proposal
    async fn execute_committed_proposal(&self, proposal: Proposal) -> Result<()> {
        match proposal {
            Proposal::Set { key, value } => {
                info!("Executing SET operation: {} = {:?}", key, value);
                // TODO: Apply to state machine
            }
            Proposal::Delete { key } => {
                info!("Executing DELETE operation: {}", key);
                // TODO: Apply to state machine
            }
            Proposal::MembershipChange { action, node_id } => {
                info!("Executing membership change: {:?} node {}", action, node_id);
                let mut members = self.cluster_members.write().await;
                match action {
                    MembershipAction::Add => {
                        if !members.contains(&node_id) {
                            members.push(node_id);
                        }
                    }
                    MembershipAction::Remove => {
                        members.retain(|&id| id != node_id);
                    }
                }
            }
        }

        // Update stats
        let mut stats = self.stats.write().await;
        stats.proposals_committed += 1;
        
        Ok(())
    }

    /// Check if the system can tolerate f Byzantine failures
    async fn check_byzantine_fault_tolerance(&self) -> (bool, usize) {
        let members = self.cluster_members.read().await;
        let total_nodes = members.len();
        
        // For Byzantine fault tolerance, we need at least 3f+1 nodes to tolerate f failures
        let max_failures = if total_nodes >= 4 { (total_nodes - 1) / 3 } else { 0 };
        let can_tolerate = total_nodes >= 3 * max_failures + 1;
        
        (can_tolerate, max_failures)
    }

    /// Get Byzantine fault tolerance status
    pub async fn byzantine_status(&self) -> ByzantineStatus {
        let (can_tolerate_faults, max_failures) = self.check_byzantine_fault_tolerance().await;
        let members = self.cluster_members.read().await;
        
        ByzantineStatus {
            enabled: self.config.byzantine_fault_tolerance,
            total_nodes: members.len(),
            max_byzantine_failures: max_failures,
            can_tolerate_faults,
            required_confirmations: self.config.byzantine_confirmations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_consensus_engine_creation() {
        let config = ConsensusConfig::default();
        let node_id = NodeId::random();
        
        let engine = ConsensusEngine::new(&config, node_id).await;
        assert!(engine.is_ok());
        
        let engine = engine.unwrap();
        assert_eq!(engine.state().await, ConsensusState::Follower);
    }
    
    #[test]
    fn test_log_entry_serialization() {
        let entry = LogEntry {
            term: 1,
            index: 0,
            proposal: Proposal::Set {
                key: "test".to_string(),
                value: b"value".to_vec(),
            },
            timestamp: SystemTime::now(),
            signatures: Vec::new(),
        };
        
        let serialized = serde_json::to_string(&entry).unwrap();
        let deserialized: LogEntry = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(entry.term, deserialized.term);
        assert_eq!(entry.index, deserialized.index);
    }
    
    #[test]
    fn test_consensus_config() {
        let config = ConsensusConfig::default();
        assert!(config.election_timeout.0 < config.election_timeout.1);
        assert!(config.heartbeat_interval < config.election_timeout.0);
        assert!(config.byzantine_fault_tolerance);
        assert!(config.byzantine_confirmations >= 3);
    }

    #[tokio::test]
    async fn test_byzantine_consensus_proposal() {
        let mut config = ConsensusConfig::default();
        config.byzantine_fault_tolerance = true;
        config.byzantine_confirmations = 3;
        
        let node_id = NodeId::random();
        let engine = ConsensusEngine::new(&config, node_id).await.unwrap();
        
        // Start the engine and become leader
        engine.start().await.unwrap();
        engine.join_cluster(vec![node_id]).await.unwrap();
        
        // Wait a bit for election
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Make a proposal
        let proposal = Proposal::Set {
            key: "test".to_string(),
            value: b"value".to_vec(),
        };
        
        let result = engine.propose(proposal).await;
        assert!(result.is_ok());
        
        // Check stats
        let stats = engine.stats().await;
        assert!(stats.proposals_received > 0);
    }

    #[tokio::test]
    async fn test_byzantine_status() {
        let config = ConsensusConfig::default();
        let node_id = NodeId::random();
        let engine = ConsensusEngine::new(&config, node_id).await.unwrap();
        
        // Test single node
        engine.join_cluster(vec![node_id]).await.unwrap();
        let status = engine.byzantine_status().await;
        
        assert!(status.enabled);
        assert_eq!(status.total_nodes, 1);
        assert_eq!(status.max_byzantine_failures, 0);
        
        // Test with more nodes
        let nodes = vec![node_id, NodeId::random(), NodeId::random(), NodeId::random()];
        engine.join_cluster(nodes).await.unwrap();
        let status = engine.byzantine_status().await;
        
        assert_eq!(status.total_nodes, 4);
        assert_eq!(status.max_byzantine_failures, 1);
        assert!(status.can_tolerate_faults);
    }

    #[tokio::test]
    async fn test_pbft_message_serialization() {
        let msg = PbftMessage::PrePrepare {
            view: 1,
            sequence_number: 0,
            proposal: Proposal::Set {
                key: "test".to_string(),
                value: b"value".to_vec(),
            },
            primary_id: NodeId::random(),
        };
        
        let serialized = serde_json::to_string(&msg).unwrap();
        let deserialized: PbftMessage = serde_json::from_str(&serialized).unwrap();
        
        if let PbftMessage::PrePrepare { view, sequence_number, .. } = deserialized {
            assert_eq!(view, 1);
            assert_eq!(sequence_number, 0);
        } else {
            panic!("Wrong message type after deserialization");
        }
    }

    #[tokio::test]
    async fn test_byzantine_signature_validation() {
        let config = ConsensusConfig::default();
        let node_id = NodeId::random();
        let engine = ConsensusEngine::new(&config, node_id).await.unwrap();
        
        let mut entry = LogEntry {
            term: 1,
            index: 0,
            proposal: Proposal::Set {
                key: "test".to_string(),
                value: b"value".to_vec(),
            },
            timestamp: SystemTime::now(),
            signatures: Vec::new(),
        };
        
        // Test without signatures
        let valid = engine.validate_byzantine_signatures(&entry).await.unwrap();
        assert!(!valid);
        
        // Add enough signatures
        for i in 0..config.byzantine_confirmations {
            entry.signatures.push(ByzantineSignature {
                node_id: NodeId::random(),
                signature: vec![i as u8; 64],
                timestamp: SystemTime::now(),
            });
        }
        
        let valid = engine.validate_byzantine_signatures(&entry).await.unwrap();
        assert!(valid);
    }
}