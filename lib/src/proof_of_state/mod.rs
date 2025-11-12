//! HyperMesh Consensus System
//!
//! This module provides the NKrypt four-proof consensus system implementation
//! for HyperMesh distributed computing.

use serde::{Serialize, Deserialize};
use std::sync::Arc;

pub mod nkrypt_integration;
pub mod proof;
pub mod engine;
pub mod validation_service;
pub mod byzantine;
pub mod sharding;
pub mod storage;
pub mod transaction;
pub mod metrics;
pub mod config;
pub mod error;
// REMOVED: HTTP API server (replaced with STOQ)
// pub mod api_server;
pub mod stoq_api;
pub mod stoq_handlers;
pub mod log;
pub mod detection;
pub mod types;
pub mod benches;
pub mod tests;

// Core type definitions
/// Node states in Raft consensus protocol
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    /// Follower state - accepts entries from leader
    Follower,
    /// Candidate state - requesting votes for leadership
    Candidate,
    /// Leader state - coordinating cluster consensus
    Leader,
}

/// Current term number for Raft consensus
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Term(pub u64);

impl Term {
    /// Create a new term
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Get the term value
    pub fn value(&self) -> u64 {
        self.0
    }

    /// Increment the term
    pub fn increment(&mut self) {
        self.0 += 1;
    }
}

/// Log index for entries in the replicated log
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LogIndex(pub u64);

impl LogIndex {
    /// Create a new log index
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Get the index value
    pub fn value(&self) -> u64 {
        self.0
    }

    /// Increment the index
    pub fn increment(&mut self) {
        self.0 += 1;
    }
}

/// Consensus message types for inter-node communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusMessage {
    /// Vote request message for leader election
    VoteRequest {
        /// Term for this election
        term: u64,
        /// Candidate requesting vote
        candidate_id: String,
        /// Index of candidate's last log entry
        last_log_index: u64,
        /// Term of candidate's last log entry
        last_log_term: u64,
    },
    /// Vote response message
    VoteResponse {
        /// Current term for checking stale request
        term: u64,
        /// True if candidate received vote
        vote_granted: bool,
    },
    /// Append entries message for log replication
    AppendEntries {
        /// Leader's current term
        term: u64,
        /// Leader's node ID
        leader_id: String,
        /// Index of log entry immediately preceding new ones
        prev_log_index: u64,
        /// Term of prev_log_index entry
        prev_log_term: u64,
        /// Log entries to store (empty for heartbeat)
        entries: Vec<Vec<u8>>,
        /// Leader's commit index
        leader_commit: u64,
    },
    /// Append entries response
    AppendEntriesResponse {
        /// Current term for leader to update itself
        term: u64,
        /// True if follower contained entry matching prev_log_index and prev_log_term
        success: bool,
    },
    /// Byzantine evidence reporting
    ByzantineReport {
        /// Term when evidence was collected
        term: u64,
        /// Reporter node ID
        reporter_id: String,
        /// Accused node ID
        accused_id: String,
        /// Evidence of malicious behavior
        evidence: Vec<u8>,
    },
}

/// Vote struct for leader election
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    /// Voting term
    pub term: u64,
    /// Node ID that received the vote
    pub voted_for: String,
    /// Timestamp of vote
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Digital signature of vote
    pub signature: Vec<u8>,
}

// Re-export main types
pub use nkrypt_integration::{
    ConsensusProof, SpaceProof, StakeProof, WorkProof, TimeProof,
    NKryptConsensus,
};

// Legacy aliases for compatibility
pub use SpaceProof as ProofOfSpace;
pub use StakeProof as ProofOfStake;
pub use WorkProof as ProofOfWork;
pub use TimeProof as ProofOfTime;

// Additional exports
pub use engine::ConsensusEngine;
pub use engine::ConsensusEngine as Consensus;
pub use types::NodeId;
pub use log::{ReplicatedLog, LogEntry};
pub use byzantine::{ByzantineDetector, ByzantineEvidence};
pub use transaction::{TransactionManager, Transaction, TransactionId, IsolationLevel};
pub use storage::{MVCCStorage, StorageEngine};
pub use sharding::{ShardManager, ShardId};
pub use config::{ConsensusConfig, RaftConfig};
pub use error::{ConsensusError, Result as ConsensusResult};
pub use metrics::ConsensusMetrics;

pub use proof::{
    ProofGenerator, ProofValidator,
};

pub use validation_service::{
    ValidationService,
};

pub use byzantine::{
    ByzantineFaultTolerance,
};

// Already exported above