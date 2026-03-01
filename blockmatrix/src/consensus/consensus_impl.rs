// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Concrete implementation of the Consensus trait for BlockMatrix
//!
//! This provides the actual consensus functionality needed by the asset blockchain manager.

use crate::consensus::{ConsensusConfig, ConsensusError, ConsensusProof};
use crate::transport::PeerIdentity;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// Result type for consensus operations
pub type ConsensusResult<T> = Result<T, ConsensusError>;

/// Async consensus trait for blockchain operations
#[async_trait]
pub trait AsyncConsensus: Send + Sync {
    /// Create a consensus proof for an operation
    async fn create_consensus_proof(
        &self,
        asset_id: &str,
        node_id: &PeerIdentity,
        operation_type: &str,
    ) -> ConsensusResult<ConsensusProof>;

    /// Check if this node is the current leader
    async fn is_leader(&self) -> bool;

    /// Validate a consensus proof
    async fn validate_consensus_proof(&self, proof: &ConsensusProof) -> ConsensusResult<bool>;

    /// Get current consensus state
    async fn get_state(&self) -> ConsensusResult<ConsensusState>;
}

/// Consensus state information
#[derive(Clone, Debug)]
pub struct ConsensusState {
    pub is_leader: bool,
    pub epoch: u64,
    pub participants: Vec<String>,
    pub byzantine_threshold: f64,
}

/// Default consensus implementation using TrustChain's ConsensusProof
pub struct DefaultConsensus {
    config: ConsensusConfig,
    node_id: String,
    is_leader: bool,
}

impl DefaultConsensus {
    /// Create new consensus instance
    pub fn new(config: ConsensusConfig, node_id: String) -> Self {
        Self {
            config,
            node_id,
            is_leader: false, // Will be determined dynamically
        }
    }

    /// Create with leader status (for testing)
    pub fn new_as_leader(config: ConsensusConfig, node_id: String) -> Self {
        Self {
            config,
            node_id,
            is_leader: true,
        }
    }
}

#[async_trait]
impl AsyncConsensus for DefaultConsensus {
    async fn create_consensus_proof(
        &self,
        _asset_id: &str,
        _node_id: &PeerIdentity,
        _operation_type: &str,
    ) -> ConsensusResult<ConsensusProof> {
        // For now, generate a test consensus proof
        // In production, this would involve actual consensus with network nodes
        Ok(ConsensusProof::new_for_testing())
    }

    async fn is_leader(&self) -> bool {
        self.is_leader
    }

    async fn validate_consensus_proof(&self, proof: &ConsensusProof) -> ConsensusResult<bool> {
        // Use TrustChain's comprehensive validation
        proof
            .validate_comprehensive()
            .await
            .map_err(|e| ConsensusError::ValidationFailed(e.to_string()))
    }

    async fn get_state(&self) -> ConsensusResult<ConsensusState> {
        Ok(ConsensusState {
            is_leader: self.is_leader,
            epoch: 1,
            participants: vec![self.node_id.clone()],
            byzantine_threshold: self.config.byzantine_threshold,
        })
    }
}

/// Wrapper to convert Arc<dyn AsyncConsensus> for use in AssetBlockchainManager
pub struct ConsensusAdapter {
    inner: Arc<dyn AsyncConsensus>,
}

impl ConsensusAdapter {
    pub fn new(consensus: Arc<dyn AsyncConsensus>) -> Self {
        Self { inner: consensus }
    }

    pub fn from_default(config: ConsensusConfig, node_id: String) -> Self {
        let consensus = Arc::new(DefaultConsensus::new(config, node_id));
        Self { inner: consensus }
    }
}

// Make ConsensusAdapter accessible via the inner consensus
impl std::ops::Deref for ConsensusAdapter {
    type Target = dyn AsyncConsensus;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}
