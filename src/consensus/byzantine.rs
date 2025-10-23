//! Byzantine fault detection and mitigation

use super::{
    ConsensusMessage, Vote,
    config::ByzantineConfig,
    error::{ConsensusError, Result},
};

use super::transport::NodeId;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant, SystemTime};
use serde::{Serialize, Deserialize};
use ed25519_dalek::{Signature, VerifyingKey};
use tracing::{warn, debug, info};

/// Byzantine fault detector for identifying malicious nodes
pub struct ByzantineDetector {
    /// Node reputation scores (0.0 = completely malicious, 1.0 = completely trustworthy)
    reputation_scores: HashMap<NodeId, f64>,
    
    /// Evidence of Byzantine behavior
    evidence_store: HashMap<NodeId, Vec<ByzantineEvidence>>,
    
    /// Recent message history for replay detection
    message_history: HashMap<NodeId, VecDeque<MessageRecord>>,
    
    /// Quarantined nodes (temporarily or permanently excluded)
    quarantined_nodes: HashMap<NodeId, QuarantineInfo>,
    
    /// Configuration
    config: ByzantineConfig,
    
    /// Last reputation decay timestamp
    last_decay: Instant,
    
    /// Node public keys for signature verification
    node_keys: HashMap<NodeId, VerifyingKey>,
}

/// Evidence of Byzantine behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ByzantineEvidence {
    /// Node sent conflicting votes in the same term
    ConflictingVotes {
        vote1: Vote,
        vote2: Vote,
        term: u64,
        detected_at: SystemTime,
    },
    
    /// Invalid digital signature on message
    InvalidSignature {
        message_hash: Vec<u8>,
        claimed_signature: Vec<u8>,
        node_key: Vec<u8>,
        detected_at: SystemTime,
    },
    
    /// Message replay attack detected
    MessageReplay {
        original_timestamp: SystemTime,
        replay_timestamp: SystemTime,
        message_hash: Vec<u8>,
        detected_at: SystemTime,
    },
    
    /// Timestamp violation (message too far in future/past)
    TimestampViolation {
        message_timestamp: SystemTime,
        local_timestamp: SystemTime,
        allowed_skew_ms: u64,
        detected_at: SystemTime,
    },
    
    /// Node sending messages at impossible rate
    MessageFlooding {
        message_count: usize,
        time_window_ms: u64,
        rate_limit: usize,
        detected_at: SystemTime,
    },
    
    /// Inconsistent log entries
    LogInconsistency {
        claimed_index: u64,
        claimed_term: u64,
        actual_index: u64,
        actual_term: u64,
        detected_at: SystemTime,
    },
    
    /// Node claiming to be leader when another leader exists
    FalseLeadershipClaim {
        claimed_term: u64,
        actual_leader: String,
        detected_at: SystemTime,
    },
}

/// Record of a message for replay detection
#[derive(Debug, Clone)]
struct MessageRecord {
    message_hash: Vec<u8>,
    timestamp: SystemTime,
    message_type: String,
}

/// Information about a quarantined node
#[derive(Debug, Clone)]
struct QuarantineInfo {
    quarantined_at: SystemTime,
    quarantine_duration: Duration,
    reason: String,
    evidence_count: usize,
    is_permanent: bool,
}

/// Behavior score for reputation tracking
#[derive(Debug, Clone)]
pub enum BehaviorScore {
    /// Good behavior (increases reputation)
    Good(f64),
    /// Suspicious behavior (slightly decreases reputation)
    Suspicious(f64),
    /// Malicious behavior (significantly decreases reputation)
    Malicious(f64),
    /// Confirmed Byzantine behavior (major reputation penalty)
    Byzantine(f64),
}

impl ByzantineDetector {
    /// Create a new Byzantine detector
    pub fn new(config: ByzantineConfig) -> Self {
        Self {
            reputation_scores: HashMap::new(),
            evidence_store: HashMap::new(),
            message_history: HashMap::new(),
            quarantined_nodes: HashMap::new(),
            config,
            last_decay: Instant::now(),
            node_keys: HashMap::new(),
        }
    }
    
    /// Add a node's public key for signature verification
    pub fn add_node_key(&mut self, node_id: NodeId, public_key: VerifyingKey) {
        self.node_keys.insert(node_id, public_key);
    }
    
    /// Detect Byzantine behavior in a message
    pub fn detect_byzantine_behavior(
        &mut self,
        message: &ConsensusMessage,
        sender: &NodeId,
    ) -> Option<ByzantineEvidence> {
        if !self.config.enabled {
            return None;
        }
        
        // Update message history
        self.update_message_history(sender, message);
        
        // Check for various types of Byzantine behavior
        self.check_timestamp_violation(message, sender)
            .or_else(|| self.check_message_replay(message, sender))
            .or_else(|| self.check_conflicting_votes(message, sender))
            .or_else(|| self.check_message_flooding(sender))
            .or_else(|| self.check_signature_validity(message, sender))
    }
    
    /// Record evidence of Byzantine behavior
    pub fn record_evidence(&mut self, node_id: NodeId, evidence: ByzantineEvidence) {
        debug!("Recording Byzantine evidence for node {:?}: {:?}", node_id, evidence);
        
        // Add to evidence store
        self.evidence_store
            .entry(node_id.clone())
            .or_insert_with(Vec::new)
            .push(evidence.clone());
        
        // Update reputation score
        let penalty = self.calculate_reputation_penalty(&evidence);
        self.update_reputation(&node_id, BehaviorScore::Byzantine(penalty));
        
        // Check if node should be quarantined
        if self.should_quarantine(&node_id) {
            self.quarantine_node(node_id, evidence);
        }
    }
    
    /// Update node reputation score
    pub fn update_reputation(&mut self, node_id: &NodeId, behavior: BehaviorScore) {
        let current_score = self.reputation_scores.get(node_id).copied().unwrap_or(1.0);
        
        let new_score = match behavior {
            BehaviorScore::Good(boost) => (current_score + boost).min(1.0),
            BehaviorScore::Suspicious(penalty) => (current_score - penalty).max(0.0),
            BehaviorScore::Malicious(penalty) => (current_score - penalty).max(0.0),
            BehaviorScore::Byzantine(penalty) => (current_score - penalty).max(0.0),
        };
        
        self.reputation_scores.insert(node_id.clone(), new_score);
        
        if new_score < self.config.detection_threshold {
            warn!("Node {:?} reputation dropped to {} (threshold: {})", 
                  node_id, new_score, self.config.detection_threshold);
        }
    }
    
    /// Check if a node is considered Byzantine
    pub fn is_node_byzantine(&self, node_id: &NodeId) -> bool {
        if !self.config.enabled {
            return false;
        }
        
        // Check if quarantined
        if self.quarantined_nodes.contains_key(node_id) {
            return true;
        }
        
        // Check reputation score
        if let Some(&score) = self.reputation_scores.get(node_id) {
            score < self.config.detection_threshold
        } else {
            false // Unknown nodes are not considered Byzantine
        }
    }
    
    /// Check if a node is quarantined
    pub fn is_node_quarantined(&self, node_id: &NodeId) -> bool {
        if let Some(info) = self.quarantined_nodes.get(node_id) {
            if info.is_permanent {
                return true;
            }
            
            // Check if temporary quarantine has expired
            let elapsed = info.quarantined_at.elapsed().unwrap_or(Duration::ZERO);
            elapsed < info.quarantine_duration
        } else {
            false
        }
    }
    
    /// Calculate consensus threshold accounting for Byzantine nodes
    pub fn calculate_consensus_threshold(&self, total_nodes: usize) -> usize {
        if !self.config.enabled {
            return (total_nodes / 2) + 1; // Standard Raft majority
        }
        
        let byzantine_nodes = self.count_byzantine_nodes();
        let f = byzantine_nodes.min((total_nodes as f64 * self.config.max_byzantine_ratio) as usize);
        
        // Byzantine fault tolerance requires 3f + 1 total nodes and 2f + 1 agreement
        if total_nodes >= 3 * f + 1 {
            2 * f + 1
        } else {
            // Fall back to Raft majority if not enough nodes for BFT
            (total_nodes / 2) + 1
        }
    }
    
    /// Get reputation score for a node
    pub fn get_reputation(&self, node_id: &NodeId) -> f64 {
        self.reputation_scores.get(node_id).copied().unwrap_or(1.0)
    }
    
    /// Get evidence count for a node
    pub fn get_evidence_count(&self, node_id: &NodeId) -> usize {
        self.evidence_store.get(node_id).map(|e| e.len()).unwrap_or(0)
    }
    
    /// Decay reputation scores over time
    pub fn decay_reputations(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_decay);
        
        // Decay every minute
        if elapsed >= Duration::from_secs(60) {
            for score in self.reputation_scores.values_mut() {
                *score = (*score * self.config.reputation_decay_factor).min(1.0);
            }
            self.last_decay = now;
            debug!("Applied reputation decay with factor {}", self.config.reputation_decay_factor);
        }
    }
    
    /// Clean up expired quarantines and old evidence
    pub fn cleanup(&mut self) {
        let now = SystemTime::now();
        
        // Remove expired quarantines
        self.quarantined_nodes.retain(|node_id, info| {
            if info.is_permanent {
                return true;
            }
            
            let elapsed = now.duration_since(info.quarantined_at).unwrap_or(Duration::ZERO);
            let should_keep = elapsed < info.quarantine_duration;
            
            if !should_keep {
                info!("Quarantine expired for node {:?}", node_id);
            }
            
            should_keep
        });
        
        // Clean up old evidence
        let retention_duration = Duration::from_secs(self.config.evidence_retention_hours * 3600);
        for evidence_list in self.evidence_store.values_mut() {
            evidence_list.retain(|evidence| {
                let evidence_age = now.duration_since(evidence.detected_at()).unwrap_or(Duration::ZERO);
                evidence_age < retention_duration
            });
        }
        
        // Remove empty evidence entries
        self.evidence_store.retain(|_, evidence_list| !evidence_list.is_empty());
        
        // Clean up old message history
        for history in self.message_history.values_mut() {
            history.retain(|record| {
                let record_age = now.duration_since(record.timestamp).unwrap_or(Duration::ZERO);
                record_age < Duration::from_secs(300) // Keep 5 minutes of history
            });
        }
    }
    
    /// Check for timestamp violations
    fn check_timestamp_violation(
        &self,
        message: &ConsensusMessage,
        sender: &NodeId,
    ) -> Option<ByzantineEvidence> {
        // Extract timestamp from message if available
        let message_time = SystemTime::now(); // Simplified - would extract from message
        let local_time = SystemTime::now();
        let allowed_skew = Duration::from_secs(60); // 1 minute skew tolerance
        
        if let Ok(diff) = message_time.duration_since(local_time) {
            if diff > allowed_skew {
                return Some(ByzantineEvidence::TimestampViolation {
                    message_timestamp: message_time,
                    local_timestamp: local_time,
                    allowed_skew_ms: allowed_skew.as_millis() as u64,
                    detected_at: SystemTime::now(),
                });
            }
        }
        
        if let Ok(diff) = local_time.duration_since(message_time) {
            if diff > allowed_skew {
                return Some(ByzantineEvidence::TimestampViolation {
                    message_timestamp: message_time,
                    local_timestamp: local_time,
                    allowed_skew_ms: allowed_skew.as_millis() as u64,
                    detected_at: SystemTime::now(),
                });
            }
        }
        
        None
    }
    
    /// Check for message replay attacks
    fn check_message_replay(
        &self,
        message: &ConsensusMessage,
        sender: &NodeId,
    ) -> Option<ByzantineEvidence> {
        let message_hash = self.compute_message_hash(message);
        
        if let Some(history) = self.message_history.get(sender) {
            for record in history {
                if record.message_hash == message_hash {
                    return Some(ByzantineEvidence::MessageReplay {
                        original_timestamp: record.timestamp,
                        replay_timestamp: SystemTime::now(),
                        message_hash,
                        detected_at: SystemTime::now(),
                    });
                }
            }
        }
        
        None
    }
    
    /// Check for conflicting votes
    fn check_conflicting_votes(
        &mut self,
        message: &ConsensusMessage,
        sender: &NodeId,
    ) -> Option<ByzantineEvidence> {
        if let ConsensusMessage::VoteResponse { term, vote_granted } = message {
            // In a full implementation, would track votes and detect conflicts
            // For now, simplified detection
            debug!("Processing vote from {:?} for term {}: {}", sender, term, vote_granted);
        }
        
        None
    }
    
    /// Check for message flooding
    fn check_message_flooding(&self, sender: &NodeId) -> Option<ByzantineEvidence> {
        if let Some(history) = self.message_history.get(sender) {
            let window = Duration::from_secs(10);
            let now = SystemTime::now();
            let rate_limit = 100; // Max 100 messages per 10 seconds
            
            let recent_count = history
                .iter()
                .filter(|record| {
                    now.duration_since(record.timestamp)
                        .unwrap_or(Duration::MAX) < window
                })
                .count();
            
            if recent_count > rate_limit {
                return Some(ByzantineEvidence::MessageFlooding {
                    message_count: recent_count,
                    time_window_ms: window.as_millis() as u64,
                    rate_limit,
                    detected_at: SystemTime::now(),
                });
            }
        }
        
        None
    }
    
    /// Check signature validity
    fn check_signature_validity(
        &self,
        _message: &ConsensusMessage,
        _sender: &NodeId,
    ) -> Option<ByzantineEvidence> {
        // In a full implementation, would verify digital signatures
        // Simplified for now
        None
    }
    
    /// Update message history for replay detection
    fn update_message_history(&mut self, sender: &NodeId, message: &ConsensusMessage) {
        let message_hash = self.compute_message_hash(message);
        let message_type = match message {
            ConsensusMessage::VoteRequest { .. } => "VoteRequest",
            ConsensusMessage::VoteResponse { .. } => "VoteResponse",
            ConsensusMessage::AppendEntries { .. } => "AppendEntries",
            ConsensusMessage::AppendEntriesResponse { .. } => "AppendEntriesResponse",
            ConsensusMessage::ByzantineReport { .. } => "ByzantineReport",
        }.to_string();
        
        let record = MessageRecord {
            message_hash,
            timestamp: SystemTime::now(),
            message_type,
        };
        
        let history = self.message_history.entry(sender.clone()).or_insert_with(VecDeque::new);
        history.push_back(record);
        
        // Keep only recent history
        while history.len() > 1000 {
            history.pop_front();
        }
    }
    
    /// Compute hash of a message for replay detection
    fn compute_message_hash(&self, message: &ConsensusMessage) -> Vec<u8> {
        use sha2::{Sha256, Digest};
        let serialized = serde_json::to_vec(message).unwrap_or_default();
        Sha256::digest(&serialized).to_vec()
    }
    
    /// Calculate reputation penalty for given evidence
    fn calculate_reputation_penalty(&self, evidence: &ByzantineEvidence) -> f64 {
        match evidence {
            ByzantineEvidence::ConflictingVotes { .. } => 0.3,
            ByzantineEvidence::InvalidSignature { .. } => 0.5,
            ByzantineEvidence::MessageReplay { .. } => 0.2,
            ByzantineEvidence::TimestampViolation { .. } => 0.1,
            ByzantineEvidence::MessageFlooding { .. } => 0.25,
            ByzantineEvidence::LogInconsistency { .. } => 0.4,
            ByzantineEvidence::FalseLeadershipClaim { .. } => 0.35,
        }
    }
    
    /// Check if a node should be quarantined
    fn should_quarantine(&self, node_id: &NodeId) -> bool {
        if !self.config.enable_quarantine {
            return false;
        }
        
        let evidence_count = self.get_evidence_count(node_id);
        evidence_count >= self.config.quarantine_evidence_threshold
    }
    
    /// Quarantine a node
    fn quarantine_node(&mut self, node_id: NodeId, evidence: ByzantineEvidence) {
        let evidence_count = self.get_evidence_count(&node_id);
        let is_permanent = evidence_count >= 10; // Permanent after 10 pieces of evidence
        
        let duration = if is_permanent {
            Duration::from_secs(u64::MAX) // Effectively permanent
        } else {
            Duration::from_secs(3600 * evidence_count as u64) // 1 hour per evidence
        };
        
        let info = QuarantineInfo {
            quarantined_at: SystemTime::now(),
            quarantine_duration: duration,
            reason: format!("Byzantine behavior: {:?}", evidence),
            evidence_count,
            is_permanent,
        };
        
        self.quarantined_nodes.insert(node_id.clone(), info);
        
        warn!("Quarantined node {:?} for {} (evidence count: {})", 
              node_id, if is_permanent { "permanent" } else { "temporary" }, evidence_count);
    }
    
    /// Count number of Byzantine nodes
    fn count_byzantine_nodes(&self) -> usize {
        self.reputation_scores
            .values()
            .filter(|&&score| score < self.config.detection_threshold)
            .count()
            + self.quarantined_nodes.len()
    }
    
    /// Get quarantine information for a node
    pub fn get_quarantine_info(&self, node_id: &NodeId) -> Option<&QuarantineInfo> {
        self.quarantined_nodes.get(node_id)
    }
    
    /// Get all evidence for a node
    pub fn get_evidence(&self, node_id: &NodeId) -> Vec<&ByzantineEvidence> {
        self.evidence_store
            .get(node_id)
            .map(|evidence| evidence.iter().collect())
            .unwrap_or_default()
    }
    
    /// Generate Byzantine report for sharing with other nodes
    pub fn generate_byzantine_report(&self, accused_id: &NodeId) -> Option<Vec<u8>> {
        if let Some(evidence_list) = self.evidence_store.get(accused_id) {
            if !evidence_list.is_empty() {
                return serde_json::to_vec(evidence_list).ok();
            }
        }
        None
    }
}

impl ByzantineEvidence {
    /// Get the detection timestamp for this evidence
    pub fn detected_at(&self) -> SystemTime {
        match self {
            Self::ConflictingVotes { detected_at, .. } => *detected_at,
            Self::InvalidSignature { detected_at, .. } => *detected_at,
            Self::MessageReplay { detected_at, .. } => *detected_at,
            Self::TimestampViolation { detected_at, .. } => *detected_at,
            Self::MessageFlooding { detected_at, .. } => *detected_at,
            Self::LogInconsistency { detected_at, .. } => *detected_at,
            Self::FalseLeadershipClaim { detected_at, .. } => *detected_at,
        }
    }
    
    /// Get severity level of this evidence
    pub fn severity(&self) -> ByzantineSeverity {
        match self {
            Self::ConflictingVotes { .. } => ByzantineSeverity::High,
            Self::InvalidSignature { .. } => ByzantineSeverity::Critical,
            Self::MessageReplay { .. } => ByzantineSeverity::Medium,
            Self::TimestampViolation { .. } => ByzantineSeverity::Low,
            Self::MessageFlooding { .. } => ByzantineSeverity::Medium,
            Self::LogInconsistency { .. } => ByzantineSeverity::High,
            Self::FalseLeadershipClaim { .. } => ByzantineSeverity::High,
        }
    }
}

/// Severity levels for Byzantine evidence
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ByzantineSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::config::ByzantineConfig;
    
    #[test]
    fn test_byzantine_detector_creation() {
        let config = ByzantineConfig::default();
        let detector = ByzantineDetector::new(config);
        
        let node_id = NodeId::new("test-node".to_string());
        assert!(!detector.is_node_byzantine(&node_id));
        assert_eq!(detector.get_reputation(&node_id), 1.0);
    }
    
    #[test]
    fn test_reputation_scoring() {
        let config = ByzantineConfig::default();
        let mut detector = ByzantineDetector::new(config);
        let node_id = NodeId::new("test-node".to_string());
        
        // Good behavior should increase reputation
        detector.update_reputation(&node_id, BehaviorScore::Good(0.1));
        assert_eq!(detector.get_reputation(&node_id), 1.0); // Capped at 1.0
        
        // Bad behavior should decrease reputation
        detector.update_reputation(&node_id, BehaviorScore::Malicious(0.5));
        assert_eq!(detector.get_reputation(&node_id), 0.5);
        
        // More bad behavior
        detector.update_reputation(&node_id, BehaviorScore::Byzantine(0.3));
        assert_eq!(detector.get_reputation(&node_id), 0.2);
        
        // Should be considered Byzantine now
        assert!(detector.is_node_byzantine(&node_id));
    }
    
    #[test]
    fn test_consensus_threshold() {
        let config = ByzantineConfig::default();
        let detector = ByzantineDetector::new(config);
        
        // Standard Raft with no Byzantine nodes
        assert_eq!(detector.calculate_consensus_threshold(5), 3); // (5/2) + 1 = 3
        assert_eq!(detector.calculate_consensus_threshold(7), 4); // (7/2) + 1 = 4
        
        // With Byzantine detection disabled
        let mut config = ByzantineConfig::default();
        config.enabled = false;
        let detector = ByzantineDetector::new(config);
        assert_eq!(detector.calculate_consensus_threshold(5), 3);
    }
    
    #[test]
    fn test_evidence_recording() {
        let config = ByzantineConfig::default();
        let mut detector = ByzantineDetector::new(config);
        let node_id = NodeId::new("malicious-node".to_string());
        
        let evidence = ByzantineEvidence::MessageFlooding {
            message_count: 1000,
            time_window_ms: 10000,
            rate_limit: 100,
            detected_at: SystemTime::now(),
        };
        
        detector.record_evidence(node_id.clone(), evidence);
        
        assert_eq!(detector.get_evidence_count(&node_id), 1);
        assert!(detector.get_reputation(&node_id) < 1.0);
    }
    
    #[test]
    fn test_quarantine_logic() {
        let mut config = ByzantineConfig::default();
        config.quarantine_evidence_threshold = 2;
        let mut detector = ByzantineDetector::new(config);
        let node_id = NodeId::new("bad-node".to_string());
        
        // Add evidence below threshold
        let evidence1 = ByzantineEvidence::MessageFlooding {
            message_count: 1000,
            time_window_ms: 10000,
            rate_limit: 100,
            detected_at: SystemTime::now(),
        };
        detector.record_evidence(node_id.clone(), evidence1);
        assert!(!detector.is_node_quarantined(&node_id));
        
        // Add more evidence to trigger quarantine
        let evidence2 = ByzantineEvidence::TimestampViolation {
            message_timestamp: SystemTime::now(),
            local_timestamp: SystemTime::now(),
            allowed_skew_ms: 60000,
            detected_at: SystemTime::now(),
        };
        detector.record_evidence(node_id.clone(), evidence2);
        assert!(detector.is_node_quarantined(&node_id));
    }
}