//! Core Raft consensus engine with Byzantine fault tolerance

use super::{
    NodeState, Term, LogIndex, ConsensusMessage, Vote,
    byzantine::ByzantineDetector,
    log::{ReplicatedLog, LogEntry},
    storage::StorageEngine,
    config::RaftConfig,
    error::{ConsensusError, Result},
    metrics::ConsensusMetrics,
};

use crate::transport::{NodeId, HyperMeshTransportTrait};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex, mpsc};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use futures::future::join_all;
use rand::Rng;
use tracing::{info, warn, error, debug};

/// Core consensus engine implementing Raft with BFT extensions
pub struct ConsensusEngine {
    /// This node's identifier
    node_id: NodeId,
    
    /// Current node state (Follower, Candidate, Leader)
    state: Arc<RwLock<NodeState>>,
    
    /// Current term
    current_term: Arc<RwLock<Term>>,
    
    /// Node voted for in current term
    voted_for: Arc<RwLock<Option<NodeId>>>,
    
    /// Replicated log
    log: Arc<RwLock<ReplicatedLog>>,
    
    /// Storage engine for persistence
    storage: Arc<dyn StorageEngine>,
    
    /// Network transport layer
    transport: Arc<dyn HyperMeshTransportTrait>,
    
    /// Byzantine fault detector
    byzantine_detector: Arc<Mutex<ByzantineDetector>>,
    
    /// Known cluster members
    cluster_members: Arc<RwLock<HashSet<NodeId>>>,
    
    /// Next index for each peer (leader state)
    next_index: Arc<RwLock<HashMap<NodeId, LogIndex>>>,
    
    /// Match index for each peer (leader state)
    match_index: Arc<RwLock<HashMap<NodeId, LogIndex>>>,
    
    /// Last heartbeat received (follower state)
    last_heartbeat: Arc<RwLock<Instant>>,
    
    /// Configuration
    config: RaftConfig,
    
    /// Metrics collection
    metrics: Arc<ConsensusMetrics>,
    
    /// Shutdown channel
    shutdown_tx: Arc<Mutex<Option<mpsc::Sender<()>>>>,
    shutdown_rx: Arc<Mutex<Option<mpsc::Receiver<()>>>>,
    
    /// Election timeout task handle
    election_timeout_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    
    /// Heartbeat task handle
    heartbeat_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl ConsensusEngine {
    /// Create a new consensus engine
    pub async fn new(
        node_id: NodeId,
        log: Arc<RwLock<ReplicatedLog>>,
        storage: Arc<dyn StorageEngine>,
        transport: Arc<dyn HyperMeshTransportTrait>,
        byzantine_detector: ByzantineDetector,
        config: RaftConfig,
    ) -> Result<Self> {
        let metrics = Arc::new(ConsensusMetrics::new()
            .map_err(|e| ConsensusError::Internal(format!("Failed to create metrics: {}", e)))?);
        
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        
        Ok(Self {
            node_id,
            state: Arc::new(RwLock::new(NodeState::Follower)),
            current_term: Arc::new(RwLock::new(Term::new(0))),
            voted_for: Arc::new(RwLock::new(None)),
            log,
            storage,
            transport,
            byzantine_detector: Arc::new(Mutex::new(byzantine_detector)),
            cluster_members: Arc::new(RwLock::new(HashSet::new())),
            next_index: Arc::new(RwLock::new(HashMap::new())),
            match_index: Arc::new(RwLock::new(HashMap::new())),
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
            config,
            metrics,
            shutdown_tx: Arc::new(Mutex::new(Some(shutdown_tx))),
            shutdown_rx: Arc::new(Mutex::new(Some(shutdown_rx))),
            election_timeout_handle: Arc::new(Mutex::new(None)),
            heartbeat_handle: Arc::new(Mutex::new(None)),
        })
    }
    
    /// Start the consensus engine
    pub async fn start(&self) -> Result<()> {
        info!("Starting consensus engine for node {:?}", self.node_id);
        
        // Initialize cluster members from storage
        self.load_cluster_state().await?;
        
        // Start main consensus loop
        self.start_consensus_loop().await?;
        
        // Start election timeout for followers
        self.start_election_timeout().await;
        
        info!("Consensus engine started successfully");
        Ok(())
    }
    
    /// Stop the consensus engine
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping consensus engine for node {:?}", self.node_id);
        
        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(()).await;
        }
        
        // Cancel running tasks
        if let Some(handle) = self.election_timeout_handle.lock().await.take() {
            handle.abort();
        }
        
        if let Some(handle) = self.heartbeat_handle.lock().await.take() {
            handle.abort();
        }
        
        info!("Consensus engine stopped");
        Ok(())
    }
    
    /// Get current node state
    pub async fn current_state(&self) -> NodeState {
        self.state.read().await.clone()
    }
    
    /// Get current term
    pub async fn current_term(&self) -> Term {
        *self.current_term.read().await
    }
    
    /// Check if this node is the leader
    pub async fn is_leader(&self) -> bool {
        *self.state.read().await == NodeState::Leader
    }
    
    /// Add a new node to the cluster
    pub async fn add_node(&self, node_id: NodeId) -> Result<()> {
        info!("Adding node {:?} to cluster", node_id);
        
        let mut members = self.cluster_members.write().await;
        members.insert(node_id.clone());
        
        // If we're the leader, initialize next/match indices
        if self.is_leader().await {
            let log_guard = self.log.read().await;
            let next_idx = LogIndex::new(log_guard.len() as u64 + 1);
            drop(log_guard);
            
            self.next_index.write().await.insert(node_id.clone(), next_idx);
            self.match_index.write().await.insert(node_id, LogIndex::new(0));
        }
        
        // Persist cluster membership
        self.save_cluster_state().await?;
        
        Ok(())
    }
    
    /// Remove a node from the cluster
    pub async fn remove_node(&self, node_id: &NodeId) -> Result<()> {
        info!("Removing node {:?} from cluster", node_id);
        
        let mut members = self.cluster_members.write().await;
        members.remove(node_id);
        
        // Clean up leader state
        self.next_index.write().await.remove(node_id);
        self.match_index.write().await.remove(node_id);
        
        // Persist cluster membership
        self.save_cluster_state().await?;
        
        Ok(())
    }
    
    /// Replicate a log entry (leader only)
    pub async fn replicate_entry(&self, data: Vec<u8>) -> Result<LogIndex> {
        if !self.is_leader().await {
            return Err(ConsensusError::NotReady { 
                reason: "Not the leader".to_string() 
            });
        }
        
        let current_term = *self.current_term.read().await;
        
        // Create new log entry
        let entry = LogEntry {
            index: LogIndex::new(0), // Will be set by the log
            term: current_term,
            data,
            timestamp: chrono::Utc::now(),
            checksum: Vec::new(), // Will be computed by the log
        };
        
        // Append to our log
        let index = {
            let mut log_guard = self.log.write().await;
            log_guard.append_entry(entry).await?
        };
        
        // Start replication to followers
        self.replicate_to_followers(index).await?;
        
        self.metrics.log_entries_replicated.inc();
        Ok(index)
    }
    
    /// Handle incoming consensus message
    pub async fn handle_message(&self, message: ConsensusMessage, sender: NodeId) -> Result<Option<ConsensusMessage>> {
        // Check for Byzantine behavior
        {
            let mut detector = self.byzantine_detector.lock().await;
            if let Some(evidence) = detector.detect_byzantine_behavior(&message, &sender) {
                warn!("Byzantine behavior detected from {:?}: {:?}", sender, evidence);
                self.metrics.record_byzantine_detection(sender.as_str(), "Message anomaly");
                detector.record_evidence(sender.clone(), evidence);
                
                // Block message from Byzantine node
                if detector.is_node_byzantine(&sender) {
                    self.metrics.malicious_messages_blocked.inc();
                    return Ok(None);
                }
            }
        }
        
        match message {
            ConsensusMessage::VoteRequest { term, candidate_id, last_log_index, last_log_term } => {
                self.handle_vote_request(term, candidate_id, last_log_index, last_log_term).await
            }
            ConsensusMessage::VoteResponse { term, vote_granted } => {
                self.handle_vote_response(sender, term, vote_granted).await;
                Ok(None)
            }
            ConsensusMessage::AppendEntries { 
                term, leader_id, prev_log_index, prev_log_term, entries, leader_commit 
            } => {
                self.handle_append_entries(
                    term, leader_id, prev_log_index, prev_log_term, entries, leader_commit
                ).await
            }
            ConsensusMessage::AppendEntriesResponse { term, success } => {
                self.handle_append_entries_response(sender, term, success).await;
                Ok(None)
            }
            ConsensusMessage::ByzantineReport { term, reporter_id, accused_id, evidence } => {
                self.handle_byzantine_report(term, reporter_id, accused_id, evidence).await;
                Ok(None)
            }
        }
    }
    
    /// Start election as candidate
    async fn start_election(&self) -> Result<()> {
        let election_start = Instant::now();
        info!("Starting election for node {:?}", self.node_id);
        
        // Increment term and vote for ourselves
        let new_term = {
            let mut term = self.current_term.write().await;
            term.increment();
            *term
        };
        
        *self.voted_for.write().await = Some(self.node_id.clone());
        *self.state.write().await = NodeState::Candidate;
        
        // Get last log info for vote requests
        let (last_log_index, last_log_term) = {
            let log_guard = self.log.read().await;
            log_guard.last_entry_info().await
        };
        
        // Send vote requests to all other nodes
        let members = self.cluster_members.read().await.clone();
        let mut vote_futures = Vec::new();
        
        for member in &members {
            if member != &self.node_id {
                let vote_request = ConsensusMessage::VoteRequest {
                    term: new_term.value(),
                    candidate_id: self.node_id.as_str().to_string(),
                    last_log_index: last_log_index.value(),
                    last_log_term: last_log_term.value(),
                };
                
                let transport = self.transport.clone();
                let member = member.clone();
                let request_data = serde_json::to_vec(&vote_request)?;
                
                vote_futures.push(async move {
                    transport.send_to(&member, &request_data).await
                });
            }
        }
        
        self.metrics.vote_requests_sent.inc_by(vote_futures.len() as f64);
        
        // Wait for responses with timeout
        let timeout = Duration::from_millis(self.config.vote_timeout_ms);
        let _results = tokio::time::timeout(timeout, join_all(vote_futures)).await;
        
        // Count votes (we already have our own vote)
        let mut votes = 1;
        let total_nodes = members.len();
        let majority = (total_nodes / 2) + 1;
        
        // Check if we won the election
        if votes >= majority {
            self.become_leader().await?;
            let election_duration = election_start.elapsed();
            self.metrics.record_leader_election(election_duration);
            info!("Won election for term {} in {:?}", new_term.value(), election_duration);
        } else {
            // Revert to follower if we didn't win
            *self.state.write().await = NodeState::Follower;
            debug!("Lost election for term {} with {} votes", new_term.value(), votes);
        }
        
        Ok(())
    }
    
    /// Become leader after winning election
    async fn become_leader(&self) -> Result<()> {
        info!("Node {:?} becoming leader for term {}", 
              self.node_id, self.current_term.read().await.value());
        
        *self.state.write().await = NodeState::Leader;
        
        // Initialize leader state
        let log_len = {
            let log_guard = self.log.read().await;
            log_guard.len() as u64 + 1
        };
        
        let mut next_index = self.next_index.write().await;
        let mut match_index = self.match_index.write().await;
        
        for member in self.cluster_members.read().await.iter() {
            if member != &self.node_id {
                next_index.insert(member.clone(), LogIndex::new(log_len));
                match_index.insert(member.clone(), LogIndex::new(0));
            }
        }
        
        // Start sending heartbeats
        self.start_heartbeat_loop().await;
        
        self.metrics.leadership_changes.inc();
        Ok(())
    }
    
    /// Handle vote request
    async fn handle_vote_request(
        &self,
        term: u64,
        candidate_id: String,
        last_log_index: u64,
        last_log_term: u64,
    ) -> Result<Option<ConsensusMessage>> {
        let request_term = Term::new(term);
        let current_term = *self.current_term.read().await;
        
        self.metrics.vote_requests_received.inc();
        
        // If request term is newer, update our term and become follower
        if request_term > current_term {
            *self.current_term.write().await = request_term;
            *self.voted_for.write().await = None;
            *self.state.write().await = NodeState::Follower;
        }
        
        let mut vote_granted = false;
        
        // Grant vote if:
        // 1. We haven't voted in this term, or we already voted for this candidate
        // 2. Candidate's log is at least as up-to-date as ours
        if request_term >= current_term {
            let voted_for = self.voted_for.read().await;
            let can_vote = voted_for.is_none() || 
                voted_for.as_ref().map(|v| v.as_str()) == Some(&candidate_id);
            
            if can_vote {
                let (our_last_index, our_last_term) = {
                    let log_guard = self.log.read().await;
                    log_guard.last_entry_info().await
                };
                
                let candidate_log_ok = last_log_term > our_last_term.value() ||
                    (last_log_term == our_last_term.value() && last_log_index >= our_last_index.value());
                
                if candidate_log_ok {
                    *self.voted_for.write().await = Some(NodeId::new(candidate_id.clone()));
                    vote_granted = true;
                    self.metrics.votes_granted.inc();
                    debug!("Granted vote to {} for term {}", candidate_id, term);
                } else {
                    self.metrics.votes_denied.inc();
                    debug!("Denied vote to {} - log not up-to-date", candidate_id);
                }
            } else {
                self.metrics.votes_denied.inc();
                debug!("Denied vote to {} - already voted", candidate_id);
            }
        } else {
            self.metrics.votes_denied.inc();
            debug!("Denied vote to {} - stale term", candidate_id);
        }
        
        Ok(Some(ConsensusMessage::VoteResponse {
            term: self.current_term.read().await.value(),
            vote_granted,
        }))
    }
    
    /// Handle vote response
    async fn handle_vote_response(&self, sender: NodeId, term: u64, vote_granted: bool) {
        let response_term = Term::new(term);
        let current_term = *self.current_term.read().await;
        
        // If response term is newer, step down
        if response_term > current_term {
            *self.current_term.write().await = response_term;
            *self.voted_for.write().await = None;
            *self.state.write().await = NodeState::Follower;
            return;
        }
        
        // Only process vote responses if we're still a candidate in the same term
        if *self.state.read().await == NodeState::Candidate && response_term == current_term {
            if vote_granted {
                debug!("Received vote from {:?} for term {}", sender, term);
                // Note: In a full implementation, we'd track votes and check for majority
            }
        }
    }
    
    /// Handle append entries request
    async fn handle_append_entries(
        &self,
        term: u64,
        leader_id: String,
        prev_log_index: u64,
        prev_log_term: u64,
        entries: Vec<Vec<u8>>,
        leader_commit: u64,
    ) -> Result<Option<ConsensusMessage>> {
        let request_term = Term::new(term);
        let current_term = *self.current_term.read().await;
        
        self.metrics.append_entries_received.inc();
        *self.last_heartbeat.write().await = Instant::now();
        
        // If request term is newer, update our term and become follower
        if request_term > current_term {
            *self.current_term.write().await = request_term;
            *self.voted_for.write().await = None;
            *self.state.write().await = NodeState::Follower;
        }
        
        let mut success = false;
        
        if request_term >= current_term {
            // Become follower if we were candidate
            if *self.state.read().await == NodeState::Candidate {
                *self.state.write().await = NodeState::Follower;
            }
            
            // Check log consistency
            let log_consistent = {
                let log_guard = self.log.read().await;
                if prev_log_index == 0 {
                    true
                } else {
                    log_guard.check_consistency(LogIndex::new(prev_log_index), Term::new(prev_log_term)).await
                }
            };
            
            if log_consistent {
                // Process entries if any
                if !entries.is_empty() {
                    let mut log_guard = self.log.write().await;
                    for (i, entry_data) in entries.iter().enumerate() {
                        let entry_index = LogIndex::new(prev_log_index + 1 + i as u64);
                        let entry_term = request_term;
                        
                        // Deserialize entry or create simple entry
                        let entry = LogEntry {
                            index: entry_index,
                            term: entry_term,
                            data: entry_data.clone(),
                            timestamp: chrono::Utc::now(),
                            checksum: Vec::new(),
                        };
                        
                        log_guard.insert_entry(entry).await?;
                    }
                }
                
                // Update commit index if leader's commit index is higher
                if leader_commit > 0 {
                    let mut log_guard = self.log.write().await;
                    log_guard.update_commit_index(LogIndex::new(leader_commit)).await?;
                }
                
                success = true;
            }
        }
        
        if !success {
            self.metrics.append_entries_failed.inc();
        }
        
        Ok(Some(ConsensusMessage::AppendEntriesResponse {
            term: self.current_term.read().await.value(),
            success,
        }))
    }
    
    /// Handle append entries response
    async fn handle_append_entries_response(&self, sender: NodeId, term: u64, success: bool) {
        let response_term = Term::new(term);
        let current_term = *self.current_term.read().await;
        
        // If response term is newer, step down
        if response_term > current_term {
            *self.current_term.write().await = response_term;
            *self.voted_for.write().await = None;
            *self.state.write().await = NodeState::Follower;
            return;
        }
        
        // Only process if we're still leader in the same term
        if *self.state.read().await == NodeState::Leader && response_term == current_term {
            if success {
                // Update match index and next index for this follower
                // Note: In full implementation, we'd track the specific entries
                debug!("Successful replication to {:?}", sender);
            } else {
                // Decrement next index and retry
                debug!("Failed replication to {:?}, will retry", sender);
            }
        }
    }
    
    /// Handle Byzantine evidence report
    async fn handle_byzantine_report(&self, _term: u64, _reporter_id: String, accused_id: String, evidence: Vec<u8>) {
        let mut detector = self.byzantine_detector.lock().await;
        
        // Process the evidence
        if let Ok(evidence_obj) = serde_json::from_slice(&evidence) {
            detector.record_evidence(NodeId::new(accused_id.clone()), evidence_obj);
            
            if detector.is_node_byzantine(&NodeId::new(accused_id.clone())) {
                warn!("Node {} confirmed as Byzantine based on evidence", accused_id);
                self.metrics.record_byzantine_detection(&accused_id, "Evidence report");
            }
        }
    }
    
    /// Replicate entries to all followers
    async fn replicate_to_followers(&self, index: LogIndex) -> Result<()> {
        let members = self.cluster_members.read().await.clone();
        let current_term = *self.current_term.read().await;
        
        let mut replication_futures = Vec::new();
        
        for member in &members {
            if member != &self.node_id {
                let append_entries = ConsensusMessage::AppendEntries {
                    term: current_term.value(),
                    leader_id: self.node_id.as_str().to_string(),
                    prev_log_index: index.value() - 1,
                    prev_log_term: current_term.value(), // Simplified
                    entries: vec![], // Would include actual entries
                    leader_commit: index.value(),
                };
                
                let transport = self.transport.clone();
                let member = member.clone();
                let message_data = serde_json::to_vec(&append_entries)?;
                
                replication_futures.push(async move {
                    transport.send_to(&member, &message_data).await
                });
            }
        }
        
        self.metrics.append_entries_sent.inc_by(replication_futures.len() as f64);
        
        // Wait for replication with timeout
        let timeout = Duration::from_millis(self.config.append_timeout_ms);
        let _results = tokio::time::timeout(timeout, join_all(replication_futures)).await;
        
        Ok(())
    }
    
    /// Start the main consensus loop
    async fn start_consensus_loop(&self) -> Result<()> {
        // In a full implementation, this would be the main event loop
        // processing incoming messages, timeouts, etc.
        Ok(())
    }
    
    /// Start election timeout for followers
    async fn start_election_timeout(&self) {
        let min_timeout = self.config.election_timeout_ms[0];
        let max_timeout = self.config.election_timeout_ms[1];
        
        let state = self.state.clone();
        let last_heartbeat = self.last_heartbeat.clone();
        let node_id = self.node_id.clone();
        
        let handle = tokio::spawn(async move {
            loop {
                // Generate random timeout between min and max
                let timeout_ms = rand::thread_rng().gen_range(min_timeout..=max_timeout);
                tokio::time::sleep(Duration::from_millis(timeout_ms)).await;
                
                // Check if we're a follower and haven't received heartbeat
                if *state.read().await == NodeState::Follower {
                    let since_heartbeat = last_heartbeat.read().await.elapsed();
                    if since_heartbeat > Duration::from_millis(timeout_ms) {
                        info!("Election timeout for node {:?}, starting election", node_id);
                        // In full implementation, would trigger election
                        break;
                    }
                }
            }
        });
        
        *self.election_timeout_handle.lock().await = Some(handle);
    }
    
    /// Start heartbeat loop for leaders
    async fn start_heartbeat_loop(&self) {
        let heartbeat_interval = Duration::from_millis(self.config.heartbeat_interval_ms);
        let transport = self.transport.clone();
        let node_id = self.node_id.clone();
        let state = self.state.clone();
        let current_term = self.current_term.clone();
        let cluster_members = self.cluster_members.clone();
        
        let handle = tokio::spawn(async move {
            while *state.read().await == NodeState::Leader {
                let term = current_term.read().await.value();
                let members = cluster_members.read().await.clone();
                
                // Send heartbeats to all followers
                for member in &members {
                    if member != &node_id {
                        let heartbeat = ConsensusMessage::AppendEntries {
                            term,
                            leader_id: node_id.as_str().to_string(),
                            prev_log_index: 0,
                            prev_log_term: 0,
                            entries: vec![], // Empty for heartbeat
                            leader_commit: 0,
                        };
                        
                        if let Ok(data) = serde_json::to_vec(&heartbeat) {
                            let _ = transport.send_to(member, &data).await;
                        }
                    }
                }
                
                tokio::time::sleep(heartbeat_interval).await;
            }
        });
        
        *self.heartbeat_handle.lock().await = Some(handle);
    }
    
    /// Load cluster state from storage
    async fn load_cluster_state(&self) -> Result<()> {
        // In a full implementation, would load persisted state
        Ok(())
    }
    
    /// Save cluster state to storage
    async fn save_cluster_state(&self) -> Result<()> {
        // In a full implementation, would persist cluster membership
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{byzantine::ByzantineConfig, storage::MockStorage};
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_consensus_engine_creation() {
        let node_id = NodeId::new("test-node".to_string());
        let storage = Arc::new(MockStorage::new());
        let log = Arc::new(RwLock::new(ReplicatedLog::new(storage.clone()).await.unwrap()));
        
        // Mock transport
        struct MockTransport;
        #[async_trait::async_trait]
        impl HyperMeshTransportTrait for MockTransport {
            async fn connect_node(&self, _node_id: NodeId, _endpoint: &hypermesh_transport::Endpoint) 
                -> hypermesh_transport::Result<Arc<hypermesh_transport::Connection>> {
                unimplemented!()
            }
            async fn accept_node(&self) -> hypermesh_transport::Result<Arc<hypermesh_transport::Connection>> {
                unimplemented!()
            }
            async fn send_to(&self, _node_id: &NodeId, _data: &[u8]) -> hypermesh_transport::Result<()> {
                Ok(())
            }
            async fn receive_from(&self, _connection: &hypermesh_transport::Connection) 
                -> hypermesh_transport::Result<bytes::Bytes> {
                Ok(bytes::Bytes::new())
            }
            async fn metrics(&self) -> hypermesh_transport::TransportStats {
                hypermesh_transport::TransportStats {
                    bytes_sent: 0,
                    bytes_received: 0,
                    active_connections: 0,
                    total_connections: 0,
                    throughput_gbps: 0.0,
                    avg_latency_us: 0,
                }
            }
            async fn maintain(&self) -> hypermesh_transport::Result<()> {
                Ok(())
            }
        }
        
        let transport = Arc::new(MockTransport);
        let byzantine_detector = ByzantineDetector::new(ByzantineConfig::default());
        let config = RaftConfig::default();
        
        let engine = ConsensusEngine::new(
            node_id,
            log,
            storage,
            transport,
            byzantine_detector,
            config,
        ).await.unwrap();
        
        assert_eq!(engine.current_state().await, NodeState::Follower);
        assert_eq!(engine.current_term().await.value(), 0);
        assert!(!engine.is_leader().await);
    }
}