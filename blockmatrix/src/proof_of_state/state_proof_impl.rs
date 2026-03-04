// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Concrete implementation of the state proof system for BlockMatrix
//!
//! Provides bilateral binary authentication. Each proof is pass/fail.
//! No leader election, no voting, no quorum.

use crate::proof_of_state::{StateProofConfig, StateProofError, StateProof};
use crate::transport::PeerIdentity;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// Result type for state proof operations
pub type StateProofOpResult<T> = Result<T, StateProofError>;

/// Async state proof trait for blockchain operations (bilateral, binary)
#[async_trait]
pub trait AsyncStateProof: Send + Sync {
    /// Create a state proof for an operation
    async fn create_state_proof(
        &self,
        asset_id: &str,
        node_id: &PeerIdentity,
        operation_type: &str,
    ) -> StateProofOpResult<StateProof>;

    /// Validate a state proof (binary pass/fail)
    async fn validate_state_proof(&self, proof: &StateProof) -> StateProofOpResult<bool>;

    /// Get current state proof system state
    async fn get_state(&self) -> StateProofOpResult<StateProofState>;
}

/// State proof system information
#[derive(Clone, Debug)]
pub struct StateProofState {
    pub epoch: u64,
    pub participants: Vec<String>,
}

/// Default state proof implementation using TrustChain's StateProof
pub struct DefaultStateProof {
    config: StateProofConfig,
    node_id: String,
}

impl DefaultStateProof {
    /// Create new state proof instance
    pub fn new(config: StateProofConfig, node_id: String) -> Self {
        Self { config, node_id }
    }
}

#[async_trait]
impl AsyncStateProof for DefaultStateProof {
    async fn create_state_proof(
        &self,
        _asset_id: &str,
        _node_id: &PeerIdentity,
        _operation_type: &str,
    ) -> StateProofOpResult<StateProof> {
        // In production, this would involve actual proof generation from network state
        Ok(StateProof::new_for_testing())
    }

    async fn validate_state_proof(&self, proof: &StateProof) -> StateProofOpResult<bool> {
        proof
            .validate_comprehensive()
            .await
            .map_err(|e| StateProofError::ValidationFailed(e.to_string()))
    }

    async fn get_state(&self) -> StateProofOpResult<StateProofState> {
        Ok(StateProofState {
            epoch: 1,
            participants: vec![self.node_id.clone()],
        })
    }
}

/// Wrapper to convert Arc<dyn AsyncStateProof> for use in AssetBlockchainManager
pub struct StateProofAdapter {
    inner: Arc<dyn AsyncStateProof>,
}

impl StateProofAdapter {
    pub fn new(state_proof: Arc<dyn AsyncStateProof>) -> Self {
        Self { inner: state_proof }
    }

    pub fn from_default(config: StateProofConfig, node_id: String) -> Self {
        let state_proof = Arc::new(DefaultStateProof::new(config, node_id));
        Self { inner: state_proof }
    }
}

// Make StateProofAdapter accessible via the inner trait
impl std::ops::Deref for StateProofAdapter {
    type Target = dyn AsyncStateProof;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}
