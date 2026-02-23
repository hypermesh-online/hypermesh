// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Cross-crate validation helpers for shared types.
//!
//! Structural validation only — cryptographic verification lives in TrustChain.

use crate::error::HypermeshError;
use crate::proof::{ProofOfState, SpaceProof, StakeProof, TimeProof, Validatable, WorkProof};
use crate::types::{AssetId, MatrixPosition, NetworkId, NodeId};

// ---------------------------------------------------------------------------
// NodeId validation
// ---------------------------------------------------------------------------

impl NodeId {
    /// Validate this node identifier.
    /// Rules: non-empty, max 128 chars, alphanumeric + hyphen/underscore/dot.
    pub fn validate(&self) -> Result<(), HypermeshError> {
        if self.0.is_empty() {
            return Err(HypermeshError::Asset("NodeId is empty".into()));
        }
        if self.0.len() > 128 {
            return Err(HypermeshError::Asset(format!(
                "NodeId too long: {} > 128 chars",
                self.0.len()
            )));
        }
        if !self
            .0
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(HypermeshError::Asset(format!(
                "NodeId contains invalid characters: '{}'",
                self.0
            )));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// AssetId validation
// ---------------------------------------------------------------------------

impl AssetId {
    /// Validate this asset identifier. Must be non-empty.
    pub fn validate(&self) -> Result<(), HypermeshError> {
        if self.0.is_empty() {
            return Err(HypermeshError::Asset("AssetId is empty".into()));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// MatrixPosition validation
// ---------------------------------------------------------------------------

impl MatrixPosition {
    /// Check whether all coordinates are finite (not NaN or infinity).
    pub fn is_finite(&self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    /// Validate this matrix position. All coordinates must be finite.
    pub fn validate(&self) -> Result<(), HypermeshError> {
        if !self.is_finite() {
            return Err(HypermeshError::Asset(format!(
                "MatrixPosition has non-finite coordinates: ({}, {}, {})",
                self.x, self.y, self.z
            )));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// NetworkId validation
// ---------------------------------------------------------------------------

impl NetworkId {
    /// Check whether this is the zero (uninitialized) network ID.
    pub fn is_zero(&self) -> bool {
        self.0 == [0u8; 16]
    }

    /// Validate this network identifier. Must not be all-zero.
    pub fn validate(&self) -> Result<(), HypermeshError> {
        if self.is_zero() {
            return Err(HypermeshError::Network(
                "NetworkId is all-zero (uninitialized)".into(),
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Validatable impls — structural validation only
// ---------------------------------------------------------------------------

impl Validatable for SpaceProof {
    fn validate(&self) -> Result<(), HypermeshError> {
        if self.stored_bytes > self.committed_bytes {
            return Err(HypermeshError::Validation(format!(
                "SpaceProof: stored_bytes ({}) > committed_bytes ({})",
                self.stored_bytes, self.committed_bytes
            )));
        }
        if self.committed_bytes == 0 {
            return Err(HypermeshError::Validation(
                "SpaceProof: committed_bytes is zero".into(),
            ));
        }
        self.node_id.validate().map_err(|e| {
            HypermeshError::Validation(format!("SpaceProof: invalid node_id: {}", e))
        })?;
        self.matrix_position.validate().map_err(|e| {
            HypermeshError::Validation(format!("SpaceProof: invalid matrix_position: {}", e))
        })?;
        Ok(())
    }
}

impl Validatable for StakeProof {
    fn validate(&self) -> Result<(), HypermeshError> {
        if self.stake_amount == 0 {
            return Err(HypermeshError::Validation(
                "StakeProof: stake_amount is zero".into(),
            ));
        }
        self.node_id.validate().map_err(|e| {
            HypermeshError::Validation(format!("StakeProof: invalid node_id: {}", e))
        })?;
        if self.signature.is_empty() {
            return Err(HypermeshError::Validation(
                "StakeProof: signature is empty".into(),
            ));
        }
        Ok(())
    }
}

impl Validatable for WorkProof {
    fn validate(&self) -> Result<(), HypermeshError> {
        if self.compute_units == 0 {
            return Err(HypermeshError::Validation(
                "WorkProof: compute_units is zero".into(),
            ));
        }
        self.node_id.validate().map_err(|e| {
            HypermeshError::Validation(format!("WorkProof: invalid node_id: {}", e))
        })?;
        Ok(())
    }
}

impl Validatable for TimeProof {
    fn validate(&self) -> Result<(), HypermeshError> {
        if self.proof_hash.is_empty() {
            return Err(HypermeshError::Validation(
                "TimeProof: proof_hash is empty".into(),
            ));
        }
        Ok(())
    }
}

impl Validatable for ProofOfState {
    fn validate(&self) -> Result<(), HypermeshError> {
        self.space.validate().map_err(|e| {
            HypermeshError::Validation(format!("ProofOfState: space proof invalid: {}", e))
        })?;
        self.stake.validate().map_err(|e| {
            HypermeshError::Validation(format!("ProofOfState: stake proof invalid: {}", e))
        })?;
        self.work.validate().map_err(|e| {
            HypermeshError::Validation(format!("ProofOfState: work proof invalid: {}", e))
        })?;
        self.time.validate().map_err(|e| {
            HypermeshError::Validation(format!("ProofOfState: time proof invalid: {}", e))
        })?;
        Ok(())
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof::WorkCategory;
    use crate::types::{ContentHash, MatrixPosition, NodeId};
    use std::time::Duration;

    fn valid_space() -> SpaceProof {
        SpaceProof {
            node_id: NodeId::from("node-alpha"),
            matrix_position: MatrixPosition {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            stored_bytes: 1024,
            committed_bytes: 4096,
            content_hash: ContentHash::from_bytes([0xAB; 32]),
            timestamp_ms: 1700000000000,
        }
    }

    fn valid_stake() -> StakeProof {
        StakeProof {
            node_id: NodeId::from("node-alpha"),
            asset_id: Some(AssetId::from("asset-001")),
            stake_amount: 500,
            signature: vec![0xDE, 0xAD],
            timestamp_ms: 1700000000000,
        }
    }

    fn valid_work() -> WorkProof {
        WorkProof {
            node_id: NodeId::from("node-alpha"),
            compute_units: 42,
            work_category: WorkCategory::Compute,
            challenge_proof: vec![0xCA, 0xFE],
            timestamp_ms: 1700000000000,
        }
    }

    fn valid_time() -> TimeProof {
        TimeProof {
            time_offset: Duration::from_millis(150),
            nonce: 99,
            proof_hash: vec![0xBE, 0xEF],
            timestamp_ms: 1700000000000,
        }
    }

    // --- NodeId ---

    #[test]
    fn node_id_valid() {
        assert!(NodeId::from("node-alpha_01.test").validate().is_ok());
    }

    #[test]
    fn node_id_empty() {
        assert!(NodeId::from("").validate().is_err());
    }

    #[test]
    fn node_id_too_long() {
        let long = "a".repeat(129);
        assert!(NodeId::from(long.as_str()).validate().is_err());
    }

    #[test]
    fn node_id_invalid_chars() {
        assert!(NodeId::from("node alpha!").validate().is_err());
    }

    // --- AssetId ---

    #[test]
    fn asset_id_valid() {
        assert!(AssetId::from("asset-001").validate().is_ok());
    }

    #[test]
    fn asset_id_empty() {
        assert!(AssetId::from("").validate().is_err());
    }

    // --- MatrixPosition ---

    #[test]
    fn matrix_position_valid() {
        let pos = MatrixPosition {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        assert!(pos.is_finite());
        assert!(pos.validate().is_ok());
    }

    #[test]
    fn matrix_position_nan() {
        let pos = MatrixPosition {
            x: f64::NAN,
            y: 0.0,
            z: 0.0,
        };
        assert!(!pos.is_finite());
        assert!(pos.validate().is_err());
    }

    #[test]
    fn matrix_position_infinity() {
        let pos = MatrixPosition {
            x: 0.0,
            y: f64::INFINITY,
            z: 0.0,
        };
        assert!(pos.validate().is_err());
    }

    // --- NetworkId ---

    #[test]
    fn network_id_valid() {
        let id = NetworkId([1u8; 16]);
        assert!(!id.is_zero());
        assert!(id.validate().is_ok());
    }

    #[test]
    fn network_id_zero() {
        let id = NetworkId([0u8; 16]);
        assert!(id.is_zero());
        assert!(id.validate().is_err());
    }

    // --- Validatable impls ---

    #[test]
    fn space_proof_valid() {
        assert!(valid_space().validate().is_ok());
    }

    #[test]
    fn space_proof_stored_exceeds_committed() {
        let mut sp = valid_space();
        sp.stored_bytes = 9999;
        sp.committed_bytes = 100;
        assert!(sp.validate().is_err());
    }

    #[test]
    fn space_proof_zero_committed() {
        let mut sp = valid_space();
        sp.committed_bytes = 0;
        sp.stored_bytes = 0;
        assert!(sp.validate().is_err());
    }

    #[test]
    fn stake_proof_valid() {
        assert!(valid_stake().validate().is_ok());
    }

    #[test]
    fn stake_proof_zero_amount() {
        let mut sp = valid_stake();
        sp.stake_amount = 0;
        assert!(sp.validate().is_err());
    }

    #[test]
    fn stake_proof_empty_signature() {
        let mut sp = valid_stake();
        sp.signature = vec![];
        assert!(sp.validate().is_err());
    }

    #[test]
    fn work_proof_valid() {
        assert!(valid_work().validate().is_ok());
    }

    #[test]
    fn work_proof_zero_units() {
        let mut wp = valid_work();
        wp.compute_units = 0;
        assert!(wp.validate().is_err());
    }

    #[test]
    fn time_proof_valid() {
        assert!(valid_time().validate().is_ok());
    }

    #[test]
    fn time_proof_empty_hash() {
        let mut tp = valid_time();
        tp.proof_hash = vec![];
        assert!(tp.validate().is_err());
    }

    #[test]
    fn proof_of_state_valid() {
        let pos = ProofOfState::new(valid_space(), valid_stake(), valid_work(), valid_time());
        assert!(pos.validate().is_ok());
    }

    #[test]
    fn proof_of_state_invalid_component() {
        let mut bad_stake = valid_stake();
        bad_stake.stake_amount = 0;
        let pos = ProofOfState::new(valid_space(), bad_stake, valid_work(), valid_time());
        assert!(pos.validate().is_err());
    }
}
