// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Common test factories for HyperMesh types.
//!
//! Available when compiled with `cfg(test)` or the `test-utils` feature.
//! Other crates can enable `test-utils` in dev-dependencies to reuse these
//! factories and avoid duplicating setup code across test modules.

use crate::types::{
    AssetId, ContentHash, MatrixPosition, NetworkId, NodeId, PrivacyMode,
};
use crate::proof::{
    ProofOfState, SpaceProof, StakeProof, TimeProof, WorkCategory, WorkProof,
};
use std::time::Duration;

/// Create a test [`NodeId`] with the given name.
pub fn test_node_id(name: &str) -> NodeId {
    NodeId::from(name)
}

/// Create a test [`AssetId`] with the given name.
pub fn test_asset_id(name: &str) -> AssetId {
    AssetId::from(name)
}

/// Create a test [`MatrixPosition`] at the given coordinates.
pub fn test_matrix_position(x: f64, y: f64, z: f64) -> MatrixPosition {
    MatrixPosition { x, y, z }
}

/// Create a test [`NetworkId`] filled with the given seed byte.
pub fn test_network_id(seed: u8) -> NetworkId {
    NetworkId([seed; 16])
}

/// Create a test [`ContentHash`] filled with the given seed byte.
pub fn test_content_hash(seed: u8) -> ContentHash {
    ContentHash::from_bytes([seed; 32])
}

/// Anonymous privacy mode (unbounded, untracked).
pub fn test_privacy_mode_anonymous() -> PrivacyMode {
    PrivacyMode::ANONYMOUS
}

/// Private privacy mode (bounded, tracked).
pub fn test_privacy_mode_private() -> PrivacyMode {
    PrivacyMode::PRIVATE
}

/// Public privacy mode (unbounded, tracked).
pub fn test_privacy_mode_public() -> PrivacyMode {
    PrivacyMode::PUBLIC
}

/// Create a valid test [`SpaceProof`].
pub fn test_space_proof() -> SpaceProof {
    SpaceProof {
        node_id: test_node_id("test-node"),
        matrix_position: test_matrix_position(1.0, 2.0, 3.0),
        stored_bytes: 1024,
        committed_bytes: 4096,
        content_hash: test_content_hash(0xAB),
        timestamp_ms: 1700000000000,
    }
}

/// Create a valid test [`StakeProof`].
pub fn test_stake_proof() -> StakeProof {
    StakeProof {
        node_id: test_node_id("test-node"),
        asset_id: Some(test_asset_id("test-asset")),
        stake_amount: 500,
        signature: vec![0xDE, 0xAD],
        timestamp_ms: 1700000000000,
    }
}

/// Create a valid test [`WorkProof`].
pub fn test_work_proof() -> WorkProof {
    WorkProof {
        node_id: test_node_id("test-node"),
        compute_units: 42,
        work_category: WorkCategory::Compute,
        challenge_proof: vec![0xCA, 0xFE],
        timestamp_ms: 1700000000000,
    }
}

/// Create a valid test [`TimeProof`].
pub fn test_time_proof() -> TimeProof {
    TimeProof {
        time_offset: Duration::from_millis(150),
        nonce: 99,
        proof_hash: vec![0xBE, 0xEF],
        timestamp_ms: 1700000000000,
    }
}

/// Create a valid test [`ProofOfState`] from all four proofs.
pub fn test_proof_of_state() -> ProofOfState {
    ProofOfState::new(
        test_space_proof(),
        test_stake_proof(),
        test_work_proof(),
        test_time_proof(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof::Validatable;

    #[test]
    fn test_factories_produce_valid_types() {
        assert!(test_node_id("node-1").validate().is_ok());
        assert!(test_asset_id("asset-1").validate().is_ok());
        assert!(test_matrix_position(0.0, 0.0, 0.0).is_finite());
        assert!(!test_network_id(1).is_zero());
    }

    #[test]
    fn test_proof_factories_pass_validation() {
        assert!(test_space_proof().validate().is_ok());
        assert!(test_stake_proof().validate().is_ok());
        assert!(test_work_proof().validate().is_ok());
        assert!(test_time_proof().validate().is_ok());
        assert!(test_proof_of_state().validate().is_ok());
    }

    #[test]
    fn test_privacy_mode_presets() {
        assert_eq!(test_privacy_mode_anonymous(), PrivacyMode::ANONYMOUS);
        assert_eq!(test_privacy_mode_private(), PrivacyMode::PRIVATE);
        assert_eq!(test_privacy_mode_public(), PrivacyMode::PUBLIC);
    }
}
