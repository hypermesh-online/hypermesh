// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Cross-crate validation helpers for shared types.
//!
//! Structural validation only — cryptographic verification lives in TrustChain.

use crate::error::HypermeshError;
use crate::proof::{SpaceProof, StakeProof, StateProof, TimeProof, Validatable, WorkProof};
use crate::types::{AssetId, ContentHash, MatrixPosition, NetworkId, NodeId, PrivacyMode};

// ---------------------------------------------------------------------------
// ValidationError — descriptive validation failure enum
// ---------------------------------------------------------------------------

/// Descriptive validation errors for shared types.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    /// A [`NodeId`] is all-zero (uninitialized).
    #[error("NodeId is all-zero (uninitialized)")]
    ZeroNodeId,
    /// A [`ContentHash`] is all-zero (uninitialized).
    #[error("ContentHash is all-zero (uninitialized)")]
    ZeroContentHash,
    /// A [`MatrixPosition`] contains non-finite coordinates.
    #[error("MatrixPosition has non-finite coordinate on axis '{axis}': {value}")]
    NonFiniteCoordinate {
        /// Which axis failed ("x", "y", or "z").
        axis: &'static str,
        /// The invalid value.
        value: f64,
    },
}

// ---------------------------------------------------------------------------
// Free-standing validation helpers
// ---------------------------------------------------------------------------

/// Validate a [`NodeId`]: must not be all-zero.
pub fn validate_node_id(id: &NodeId) -> Result<(), ValidationError> {
    if id.0 == [0u8; 32] {
        return Err(ValidationError::ZeroNodeId);
    }
    Ok(())
}

/// Validate a [`ContentHash`]: must not be all-zero.
pub fn validate_content_hash(hash: &ContentHash) -> Result<(), ValidationError> {
    if hash.0 == [0u8; 32] {
        return Err(ValidationError::ZeroContentHash);
    }
    Ok(())
}

/// Validate a [`MatrixPosition`]: all coordinates must be finite.
pub fn validate_matrix_position(pos: &MatrixPosition) -> Result<(), ValidationError> {
    if !pos.x.is_finite() {
        return Err(ValidationError::NonFiniteCoordinate {
            axis: "x",
            value: pos.x,
        });
    }
    if !pos.y.is_finite() {
        return Err(ValidationError::NonFiniteCoordinate {
            axis: "y",
            value: pos.y,
        });
    }
    if !pos.z.is_finite() {
        return Err(ValidationError::NonFiniteCoordinate {
            axis: "z",
            value: pos.z,
        });
    }
    Ok(())
}

/// Validate a [`PrivacyMode`]: all modes are structurally valid.
///
/// Always returns `true` because the two-axis model (scope + tracked) has no
/// invalid combinations. Provided for completeness in validation pipelines.
pub fn validate_privacy_mode(_mode: &PrivacyMode) -> bool {
    true
}

// ---------------------------------------------------------------------------
// NodeId validation
// ---------------------------------------------------------------------------

impl NodeId {
    /// Validate this node identifier.
    /// Rule: must not be all-zero (uninitialized).
    pub fn validate(&self) -> Result<(), HypermeshError> {
        if self.0 == [0u8; 32] {
            return Err(HypermeshError::Asset(
                "NodeId is all-zero (uninitialized)".into(),
            ));
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
        // WHERE: stored must not exceed advertised capacity; node bound.
        // total_storage is descriptive capacity, NOT gated against a minimum.
        if self.total_size > self.total_storage {
            return Err(HypermeshError::Validation(format!(
                "SpaceProof: total_size ({}) > total_storage ({})",
                self.total_size, self.total_storage
            )));
        }
        if self.node_id.is_empty() {
            return Err(HypermeshError::Validation(
                "SpaceProof: node_id is empty".into(),
            ));
        }
        Ok(())
    }
}

impl Validatable for StakeProof {
    fn validate(&self) -> Result<(), HypermeshError> {
        // WHO / AUTHORIZATION: identity binding, never a magnitude.
        if self.stake_holder_id.is_empty() {
            return Err(HypermeshError::Validation(
                "StakeProof: stake_holder_id (identity) is empty".into(),
            ));
        }
        Ok(())
    }
}

impl Validatable for WorkProof {
    fn validate(&self) -> Result<(), HypermeshError> {
        // WHAT: the hash of the work done (never a capacity number).
        if self.owner_id.is_empty() {
            return Err(HypermeshError::Validation(
                "WorkProof: owner_id is empty".into(),
            ));
        }
        if self.work_hash == [0u8; 32] {
            return Err(HypermeshError::Validation(
                "WorkProof: work_hash is zero (no work hashed)".into(),
            ));
        }
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

impl Validatable for StateProof {
    fn validate(&self) -> Result<(), HypermeshError> {
        self.space_proof.validate().map_err(|e| {
            HypermeshError::Validation(format!("StateProof: space proof invalid: {}", e))
        })?;
        self.stake_proof.validate().map_err(|e| {
            HypermeshError::Validation(format!("StateProof: stake proof invalid: {}", e))
        })?;
        self.work_proof.validate().map_err(|e| {
            HypermeshError::Validation(format!("StateProof: work proof invalid: {}", e))
        })?;
        self.time_proof.validate().map_err(|e| {
            HypermeshError::Validation(format!("StateProof: time proof invalid: {}", e))
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
    use crate::types::{MatrixPosition, NodeId};
    use std::time::Duration;

    fn valid_space() -> SpaceProof {
        let mut p = SpaceProof::new("node-alpha".to_string(), "/hypermesh/a".to_string(), 4096);
        p.total_size = 1024;
        p.file_hash = "abcd".to_string();
        p
    }

    fn valid_stake() -> StakeProof {
        StakeProof::new("holder-alpha".to_string(), "node-alpha".to_string())
    }

    fn valid_work() -> WorkProof {
        WorkProof::from_work("node-alpha".to_string(), "wl-1".to_string(), b"work")
    }

    fn valid_time() -> TimeProof {
        TimeProof::new(Duration::from_millis(150))
    }

    // --- NodeId ---

    #[test]
    fn node_id_valid() {
        let id = NodeId::from_public_key(b"test-key");
        assert!(id.validate().is_ok());
    }

    #[test]
    fn node_id_zeroed_invalid() {
        let id = NodeId::zeroed();
        assert!(id.validate().is_err());
    }

    #[test]
    fn node_id_nonzero_valid() {
        let id = NodeId::from_bytes([1u8; 32]);
        assert!(id.validate().is_ok());
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
    fn space_proof_stored_exceeds_capacity() {
        let mut sp = valid_space();
        sp.total_size = 9999;
        sp.total_storage = 100;
        assert!(sp.validate().is_err());
    }

    #[test]
    fn space_proof_tiny_capacity_is_ok() {
        // Capacity is descriptive, never gated against a minimum.
        let mut sp = valid_space();
        sp.total_storage = 1;
        sp.total_size = 0;
        assert!(sp.validate().is_ok());
    }

    #[test]
    fn stake_proof_valid() {
        assert!(valid_stake().validate().is_ok());
    }

    #[test]
    fn stake_proof_empty_identity_invalid() {
        // Authorization requires a bound identity (WHO), not a magnitude.
        let mut sp = valid_stake();
        sp.stake_holder_id = String::new();
        assert!(sp.validate().is_err());
    }

    #[test]
    fn work_proof_valid() {
        assert!(valid_work().validate().is_ok());
    }

    #[test]
    fn work_proof_zero_hash_invalid() {
        let mut wp = valid_work();
        wp.work_hash = [0u8; 32];
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
    fn state_proof_valid() {
        let pos = StateProof::new(valid_stake(), valid_time(), valid_space(), valid_work());
        assert!(Validatable::validate(&pos).is_ok());
    }

    #[test]
    fn state_proof_invalid_component() {
        let mut bad_stake = valid_stake();
        bad_stake.stake_holder_id = String::new();
        let pos = StateProof::new(bad_stake, valid_time(), valid_space(), valid_work());
        assert!(Validatable::validate(&pos).is_err());
    }

    // --- Free-standing validation helpers ---

    #[test]
    fn validate_node_id_accepts_nonzero() {
        let id = NodeId::from_public_key(b"test-key");
        assert!(super::validate_node_id(&id).is_ok());
    }

    #[test]
    fn validate_node_id_rejects_zero() {
        let id = NodeId::zeroed();
        let err = super::validate_node_id(&id).unwrap_err();
        assert!(matches!(err, super::ValidationError::ZeroNodeId));
    }

    #[test]
    fn validate_content_hash_accepts_nonzero() {
        let hash = ContentHash::from_bytes([0xAB; 32]);
        assert!(super::validate_content_hash(&hash).is_ok());
    }

    #[test]
    fn validate_content_hash_rejects_zero() {
        let hash = ContentHash::zeroed();
        let err = super::validate_content_hash(&hash).unwrap_err();
        assert!(matches!(err, super::ValidationError::ZeroContentHash));
    }

    #[test]
    fn validate_matrix_position_accepts_finite() {
        let pos = MatrixPosition { x: 1.0, y: -2.0, z: 0.0 };
        assert!(super::validate_matrix_position(&pos).is_ok());
    }

    #[test]
    fn validate_matrix_position_rejects_nan() {
        let pos = MatrixPosition { x: f64::NAN, y: 0.0, z: 0.0 };
        let err = super::validate_matrix_position(&pos).unwrap_err();
        match err {
            super::ValidationError::NonFiniteCoordinate { axis, .. } => {
                assert_eq!(axis, "x");
            }
            other => unreachable!("expected NonFiniteCoordinate, got {other:?}"),
        }
    }

    #[test]
    fn validate_matrix_position_rejects_infinity_on_z() {
        let pos = MatrixPosition { x: 0.0, y: 0.0, z: f64::INFINITY };
        let err = super::validate_matrix_position(&pos).unwrap_err();
        match err {
            super::ValidationError::NonFiniteCoordinate { axis, .. } => {
                assert_eq!(axis, "z");
            }
            other => unreachable!("expected NonFiniteCoordinate, got {other:?}"),
        }
    }

    #[test]
    fn validate_privacy_mode_always_true() {
        assert!(super::validate_privacy_mode(&PrivacyMode::ANONYMOUS));
        assert!(super::validate_privacy_mode(&PrivacyMode::PRIVATE));
        assert!(super::validate_privacy_mode(&PrivacyMode::PUBLIC));
    }
}
