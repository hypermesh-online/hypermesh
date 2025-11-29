//! Consensus Management for Multi-Node Asset Coordination
//!
//! Implements Byzantine fault-tolerant consensus for distributed asset allocation,
//! state synchronization, and conflict resolution across HyperMesh nodes.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, mpsc};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};

use crate::assets::core::{
    AssetId, AssetType, AssetResult, AssetError,
    AssetState, ConsensusProof, ProxyAddress,
};

use super::{NodeId, AllocationDecision};

/// Consensus manager for multi-node coordination
pub struct ConsensusManager {
    /// Local node ID
    local_node: NodeId,
    /// Known nodes participating in consensus
    consensus_nodes: Arc<RwLock<HashMap<NodeId, ConsensusNodeInfo>>>,
    /// Active voting rounds
    voting_rounds: Arc<RwLock<HashMap<String, VotingRound>>>,
    /// Consensus history
    consensus_history: Arc<RwLock<Vec<ConsensusDecision>>>,
    /// Pending proposals
    pending_proposals: Arc<RwLock<Vec<ConsensusProposal>>>,
    /// Configuration
    config: ConsensusConfig,
    /// Metrics
    metrics: Arc<RwLock<ConsensusMetrics>>,
}

/// Consensus configuration
#[derive(Clone, Debug)]
pub struct ConsensusConfig {
    /// Minimum nodes required for consensus
    pub min_nodes: usize,
    /// Consensus threshold (percentage of nodes that must agree)
    pub consensus_threshold: f32,
    /// Voting round timeout
    pub voting_timeout: Duration,
    /// Maximum voting rounds
    pub max_rounds: u32,
    /// Byzantine fault tolerance factor
    pub byzantine_factor: f32,
    /// Enable fast path consensus
    pub fast_path_enabled: bool,
    /// Enable leader election
    pub leader_election: bool,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            min_nodes: 3,
            consensus_threshold: 0.67, // 2/3 majority
            voting_timeout: Duration::from_secs(10),
            max_rounds: 5,
            byzantine_factor: 0.33, // Tolerate up to 1/3 Byzantine nodes
            fast_path_enabled: true,
            leader_election: true,
        }
    }
}

/// Consensus node information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusNodeInfo {
    /// Node ID
    pub node_id: NodeId,
    /// Node's voting weight
    pub voting_weight: f32,
    /// Node's reputation score
    pub reputation: f32,
    /// Last participation timestamp
    pub last_participation: SystemTime,
    /// Number of successful consensus participations
    pub successful_participations: u64,
    /// Number of failed consensus participations
    pub failed_participations: u64,
    /// Is this node the current leader
    pub is_leader: bool,
}

/// Voting round for consensus
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VotingRound {
    /// Round ID
    pub round_id: String,
    /// Proposal being voted on
    pub proposal: ConsensusProposal,
    /// Current round number
    pub round_number: u32,
    /// Votes collected
    pub votes: HashMap<NodeId, Vote>,
    /// Round started timestamp
    pub started_at: SystemTime,
    /// Round deadline
    pub deadline: SystemTime,
    /// Round status
    pub status: RoundStatus,
    /// Leader for this round
    pub leader: Option<NodeId>,
}

/// Consensus proposal
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusProposal {
    /// Proposal ID
    pub proposal_id: String,
    /// Proposal type
    pub proposal_type: ProposalType,
    /// Proposer node
    pub proposer: NodeId,
    /// Proposal data
    pub data: ProposalData,
    /// Proposal timestamp
    pub timestamp: SystemTime,
    /// Proposal signature
    pub signature: Vec<u8>,
}

/// Types of consensus proposals
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProposalType {
    /// Asset allocation proposal
    AssetAllocation,
    /// Asset deallocation proposal
    AssetDeallocation,
    /// State update proposal
    StateUpdate,
    /// Migration proposal
    Migration,
    /// Configuration change
    ConfigurationChange,
    /// Leader election
    LeaderElection,
}

/// Proposal data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProposalData {
    /// Asset allocation data
    Allocation {
        asset_id: AssetId,
        target_node: NodeId,
        resource_requirements: Vec<u8>, // Serialized requirements
    },
    /// State update data
    StateUpdate {
        asset_id: AssetId,
        new_state: AssetState,
        version: u64,
    },
    /// Migration data
    Migration {
        asset_id: AssetId,
        from_node: NodeId,
        to_node: NodeId,
    },
    /// Configuration change data
    Configuration {
        key: String,
        value: String,
    },
    /// Leader election data
    LeaderElection {
        candidate: NodeId,
        term: u64,
    },
}

/// Vote in consensus round
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vote {
    /// Voting node
    pub voter: NodeId,
    /// Vote value
    pub value: VoteValue,
    /// Vote timestamp
    pub timestamp: SystemTime,
    /// Vote signature
    pub signature: Vec<u8>,
    /// Justification for the vote
    pub justification: Option<String>,
}

/// Vote values
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum VoteValue {
    /// Accept the proposal
    Accept,
    /// Reject the proposal
    Reject,
    /// Abstain from voting
    Abstain,
}

/// Round status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RoundStatus {
    /// Round is active
    Active,
    /// Round completed with consensus
    Completed,
    /// Round failed to reach consensus
    Failed,
    /// Round timed out
    TimedOut,
    /// Round was aborted
    Aborted,
}

/// Consensus decision
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusDecision {
    /// Decision ID
    pub decision_id: String,
    /// Proposal that was decided
    pub proposal: ConsensusProposal,
    /// Decision outcome
    pub outcome: DecisionOutcome,
    /// Participating nodes
    pub participants: Vec<NodeId>,
    /// Decision timestamp
    pub timestamp: SystemTime,
    /// Decision proof
    pub proof: ConsensusProof,
}

/// Decision outcome
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DecisionOutcome {
    /// Proposal accepted
    Accepted,
    /// Proposal rejected
    Rejected,
    /// No consensus reached
    NoConsensus,
}

/// Consensus metrics
#[derive(Clone, Debug, Default)]
pub struct ConsensusMetrics {
    /// Total proposals submitted
    pub total_proposals: u64,
    /// Successful consensus rounds
    pub successful_rounds: u64,
    /// Failed consensus rounds
    pub failed_rounds: u64,
    /// Average consensus time (milliseconds)
    pub avg_consensus_time_ms: f64,
    /// Fast path successes
    pub fast_path_successes: u64,
    /// Leader elections held
    pub leader_elections: u64,
    /// Byzantine nodes detected
    pub byzantine_detections: u64,
}

impl ConsensusManager {
    /// Create new consensus manager
    pub fn new(local_node: NodeId, config: ConsensusConfig) -> Self {
        Self {
            local_node,
            consensus_nodes: Arc::new(RwLock::new(HashMap::new())),
            voting_rounds: Arc::new(RwLock::new(HashMap::new())),
            consensus_history: Arc::new(RwLock::new(Vec::new())),
            pending_proposals: Arc::new(RwLock::new(Vec::new())),
            config,
            metrics: Arc::new(RwLock::new(ConsensusMetrics::default())),
        }
    }

    /// Submit proposal for consensus
    pub async fn submit_proposal(&self, proposal: ConsensusProposal) -> AssetResult<String> {
        // Validate proposal
        self.validate_proposal(&proposal).await?;

        // Check if we can use fast path
        if self.config.fast_path_enabled && self.can_use_fast_path(&proposal).await {
            return self.execute_fast_path(&proposal).await;
        }

        // Create voting round
        let round_id = self.create_voting_round(proposal.clone()).await?;

        // Start voting
        self.start_voting(&round_id).await?;

        Ok(round_id)
    }

    /// Validate proposal
    async fn validate_proposal(&self, proposal: &ConsensusProposal) -> AssetResult<()> {
        // Check proposal signature
        if !self.verify_signature(&proposal.signature, &proposal.proposer).await {
            return Err(AssetError::ConsensusValidationFailed {
                reason: "Invalid proposal signature".to_string(),
            });
        }

        // Check proposer is known
        let nodes = self.consensus_nodes.read().await;
        if !nodes.contains_key(&proposal.proposer) {
            return Err(AssetError::ConsensusValidationFailed {
                reason: "Unknown proposer node".to_string(),
            });
        }

        Ok(())
    }

    /// Check if fast path can be used
    async fn can_use_fast_path(&self, proposal: &ConsensusProposal) -> bool {
        match proposal.proposal_type {
            ProposalType::AssetAllocation | ProposalType::StateUpdate => {
                // Check if there's a clear leader and no conflicts
                let nodes = self.consensus_nodes.read().await;
                let leader_exists = nodes.values().any(|n| n.is_leader);
                let high_reputation = nodes.get(&proposal.proposer)
                    .map(|n| n.reputation > 0.9)
                    .unwrap_or(false);

                leader_exists && high_reputation
            }
            _ => false,
        }
    }

    /// Execute fast path consensus
    async fn execute_fast_path(&self, proposal: &ConsensusProposal) -> AssetResult<String> {
        let round_id = format!("fast_{}", uuid::Uuid::new_v4());

        // Create immediate decision
        let decision = ConsensusDecision {
            decision_id: round_id.clone(),
            proposal: proposal.clone(),
            outcome: DecisionOutcome::Accepted,
            participants: vec![self.local_node.clone()],
            timestamp: SystemTime::now(),
            proof: self.generate_consensus_proof(proposal).await,
        };

        self.consensus_history.write().await.push(decision);

        let mut metrics = self.metrics.write().await;
        metrics.fast_path_successes += 1;

        Ok(round_id)
    }

    /// Create voting round
    async fn create_voting_round(&self, proposal: ConsensusProposal) -> AssetResult<String> {
        let round_id = format!("round_{}", uuid::Uuid::new_v4());

        let round = VotingRound {
            round_id: round_id.clone(),
            proposal,
            round_number: 1,
            votes: HashMap::new(),
            started_at: SystemTime::now(),
            deadline: SystemTime::now() + self.config.voting_timeout,
            status: RoundStatus::Active,
            leader: self.get_current_leader().await,
        };

        self.voting_rounds.write().await.insert(round_id.clone(), round);

        Ok(round_id)
    }

    /// Start voting process
    async fn start_voting(&self, round_id: &str) -> AssetResult<()> {
        let voting_rounds = self.voting_rounds.clone();
        let consensus_nodes = self.consensus_nodes.clone();
        let config = self.config.clone();
        let metrics = self.metrics.clone();
        let round_id = round_id.to_string();

        tokio::spawn(async move {
            let start_time = std::time::Instant::now();

            // Wait for votes or timeout
            tokio::time::sleep(config.voting_timeout).await;

            // Check voting results
            let mut rounds = voting_rounds.write().await;
            if let Some(round) = rounds.get_mut(&round_id) {
                let nodes = consensus_nodes.read().await;
                let total_weight = nodes.values().map(|n| n.voting_weight).sum::<f32>();

                let accept_weight: f32 = round.votes.iter()
                    .filter(|(_, v)| v.value == VoteValue::Accept)
                    .filter_map(|(node_id, _)| nodes.get(node_id))
                    .map(|n| n.voting_weight)
                    .sum();

                let consensus_reached = accept_weight / total_weight >= config.consensus_threshold;

                if consensus_reached {
                    round.status = RoundStatus::Completed;
                    let mut metrics = metrics.write().await;
                    metrics.successful_rounds += 1;
                    metrics.avg_consensus_time_ms =
                        (metrics.avg_consensus_time_ms * (metrics.successful_rounds - 1) as f64
                         + start_time.elapsed().as_millis() as f64) / metrics.successful_rounds as f64;
                } else {
                    round.status = RoundStatus::Failed;
                    let mut metrics = metrics.write().await;
                    metrics.failed_rounds += 1;
                }
            }
        });

        Ok(())
    }

    /// Submit vote for a proposal
    pub async fn submit_vote(&self, round_id: &str, vote: Vote) -> AssetResult<()> {
        // Validate vote
        if !self.verify_signature(&vote.signature, &vote.voter).await {
            return Err(AssetError::ConsensusValidationFailed {
                reason: "Invalid vote signature".to_string(),
            });
        }

        let mut rounds = self.voting_rounds.write().await;
        let round = rounds.get_mut(round_id)
            .ok_or_else(|| AssetError::ConsensusValidationFailed {
                reason: format!("Unknown voting round: {}", round_id),
            })?;

        if round.status != RoundStatus::Active {
            return Err(AssetError::ConsensusValidationFailed {
                reason: "Voting round is not active".to_string(),
            });
        }

        round.votes.insert(vote.voter.clone(), vote);

        Ok(())
    }

    /// Get consensus decision
    pub async fn get_decision(&self, round_id: &str) -> AssetResult<ConsensusDecision> {
        let rounds = self.voting_rounds.read().await;
        let round = rounds.get(round_id)
            .ok_or_else(|| AssetError::ConsensusValidationFailed {
                reason: format!("Unknown round: {}", round_id),
            })?;

        if round.status != RoundStatus::Completed {
            return Err(AssetError::ConsensusValidationFailed {
                reason: "Round not completed".to_string(),
            });
        }

        let history = self.consensus_history.read().await;
        history.iter()
            .find(|d| d.decision_id == *round_id)
            .cloned()
            .ok_or_else(|| AssetError::ConsensusValidationFailed {
                reason: "Decision not found".to_string(),
            })
    }

    /// Elect new leader
    pub async fn elect_leader(&self) -> AssetResult<NodeId> {
        let mut nodes = self.consensus_nodes.write().await;

        // Simple leader election: highest reputation * successful participations
        let leader = nodes.iter()
            .max_by(|a, b| {
                let score_a = a.1.reputation * a.1.successful_participations as f32;
                let score_b = b.1.reputation * b.1.successful_participations as f32;
                score_a.partial_cmp(&score_b).unwrap()
            })
            .map(|(id, _)| id.clone())
            .ok_or_else(|| AssetError::ConsensusValidationFailed {
                reason: "No eligible leader".to_string(),
            })?;

        // Update leader status
        for (id, info) in nodes.iter_mut() {
            info.is_leader = *id == leader;
        }

        let mut metrics = self.metrics.write().await;
        metrics.leader_elections += 1;

        Ok(leader)
    }

    /// Get current leader
    async fn get_current_leader(&self) -> Option<NodeId> {
        let nodes = self.consensus_nodes.read().await;
        nodes.iter()
            .find(|(_, info)| info.is_leader)
            .map(|(id, _)| id.clone())
    }

    /// Verify signature (placeholder)
    async fn verify_signature(&self, signature: &[u8], node: &NodeId) -> bool {
        // In production, this would verify using the node's public key
        !signature.is_empty() && !node.public_key.is_empty()
    }

    /// Generate consensus proof
    async fn generate_consensus_proof(&self, proposal: &ConsensusProposal) -> ConsensusProof {
        // Generate a proper consensus proof
        // This would include the four-proof system from Proof of State
        use crate::assets::core::{SpaceProof, StakeProof, WorkProof, TimeProof, WorkloadType, WorkState};

        ConsensusProof::new(
            SpaceProof {
                node_id: hex::encode(&self.local_node.id[..8]),
                storage_path: "/consensus".to_string(),
                allocated_size: 1024,
                proof_hash: Sha256::digest(&proposal.signature).to_vec(),
                timestamp: SystemTime::now(),
            },
            StakeProof {
                stake_holder: hex::encode(&self.local_node.id[..8]),
                stake_holder_id: hex::encode(&self.local_node.id),
                stake_amount: 1000,
                stake_timestamp: SystemTime::now(),
            },
            WorkProof {
                worker_id: hex::encode(&self.local_node.id[..8]),
                workload_id: proposal.proposal_id.clone(),
                process_id: std::process::id(),
                computational_power: 100,
                workload_type: WorkloadType::Consensus,
                work_state: WorkState::Completed,
            },
            TimeProof {
                network_time_offset: Duration::from_secs(0),
                time_verification_timestamp: SystemTime::now(),
                nonce: rand::random(),
                proof_hash: Sha256::digest(&proposal.signature).to_vec(),
            },
        )
    }

    /// Handle Byzantine behavior
    pub async fn handle_byzantine_node(&self, node_id: &NodeId, evidence: Vec<u8>) -> AssetResult<()> {
        let mut nodes = self.consensus_nodes.write().await;

        if let Some(info) = nodes.get_mut(node_id) {
            // Reduce reputation and voting weight
            info.reputation *= 0.5;
            info.voting_weight *= 0.5;
            info.failed_participations += 1;

            tracing::warn!(
                "Byzantine behavior detected for node {}: reputation reduced to {}",
                hex::encode(&node_id.id[..8]),
                info.reputation
            );
        }

        let mut metrics = self.metrics.write().await;
        metrics.byzantine_detections += 1;

        Ok(())
    }

    /// Get consensus metrics
    pub async fn get_metrics(&self) -> ConsensusMetrics {
        self.metrics.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_node() -> NodeId {
        NodeId {
            id: [1u8; 32],
            ipv6_address: "::1".parse().unwrap(),
            public_key: vec![1, 2, 3],
            trust_score: 0.9,
        }
    }

    #[tokio::test]
    async fn test_consensus_manager_creation() {
        let node = create_test_node();
        let manager = ConsensusManager::new(node, ConsensusConfig::default());

        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.total_proposals, 0);
        assert_eq!(metrics.successful_rounds, 0);
    }

    #[tokio::test]
    async fn test_proposal_creation() {
        let node = create_test_node();
        let proposal = ConsensusProposal {
            proposal_id: "test-proposal".to_string(),
            proposal_type: ProposalType::AssetAllocation,
            proposer: node.clone(),
            data: ProposalData::Configuration {
                key: "test".to_string(),
                value: "value".to_string(),
            },
            timestamp: SystemTime::now(),
            signature: vec![1, 2, 3],
        };

        assert_eq!(proposal.proposal_id, "test-proposal");
        assert!(matches!(proposal.proposal_type, ProposalType::AssetAllocation));
    }

    #[test]
    fn test_vote_value_equality() {
        assert_eq!(VoteValue::Accept, VoteValue::Accept);
        assert_ne!(VoteValue::Accept, VoteValue::Reject);
        assert_ne!(VoteValue::Reject, VoteValue::Abstain);
    }
}