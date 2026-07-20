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
    SpaceProof, StakeProof, StateProof, TimeProof, WorkProof,
};
use std::time::Duration;

/// Create a test [`NodeId`] by hashing the given name.
pub fn test_node_id(name: &str) -> NodeId {
    NodeId::from_public_key(name.as_bytes())
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

/// Create a [`PrivacyMode`] by name: `"anonymous"`, `"private"`, or `"public"`.
///
/// Defaults to `ANONYMOUS` for unrecognised strings.
pub fn test_privacy_mode(variant: &str) -> PrivacyMode {
    match variant.to_ascii_lowercase().as_str() {
        "anonymous" => PrivacyMode::ANONYMOUS,
        "private" => PrivacyMode::PRIVATE,
        "public" => PrivacyMode::PUBLIC,
        _ => PrivacyMode::ANONYMOUS,
    }
}

/// Compute a BLAKE3 [`ContentHash`] from arbitrary data.
pub fn test_content_hash_from_data(data: &[u8]) -> ContentHash {
    let hash = blake3::hash(data);
    ContentHash::from_bytes(*hash.as_bytes())
}

/// Bundle of common test fixtures for convenience.
pub struct TestFixtures {
    pub node_id: NodeId,
    pub asset_id: AssetId,
    pub position: MatrixPosition,
    pub content_hash: ContentHash,
    pub network_id: NetworkId,
    pub privacy_mode: PrivacyMode,
}

impl Default for TestFixtures {
    fn default() -> Self {
        Self {
            node_id: test_node_id("fixture-node"),
            asset_id: test_asset_id("fixture-asset"),
            position: test_matrix_position(1.0, 2.0, 3.0),
            content_hash: test_content_hash(0xAA),
            network_id: test_network_id(0x01),
            privacy_mode: PrivacyMode::PUBLIC,
        }
    }
}

/// Create a valid test [`SpaceProof`].
pub fn test_space_proof() -> SpaceProof {
    let mut p = SpaceProof::new("test-node".to_string(), "/hypermesh/test".to_string(), 4096);
    p.total_size = 1024;
    p.file_hash = "test-file-hash".to_string();
    p
}

/// Create a valid test [`StakeProof`] (authorization — no magnitude).
pub fn test_stake_proof() -> StakeProof {
    StakeProof::new("test-holder".to_string(), "test-node".to_string())
}

/// Create a valid test [`WorkProof`] (hash of work done).
pub fn test_work_proof() -> WorkProof {
    WorkProof::from_work("test-node".to_string(), "test-workload".to_string(), b"test-work")
}

/// Create a valid test [`TimeProof`].
pub fn test_time_proof() -> TimeProof {
    TimeProof::new(Duration::from_millis(150))
}

/// Create a valid test [`StateProof`] from all four proofs.
pub fn test_state_proof() -> StateProof {
    StateProof::new(
        test_stake_proof(),
        test_time_proof(),
        test_space_proof(),
        test_work_proof(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factories_produce_valid_types() {
        // test_node_id produces a non-zero BLAKE3 hash, so it's always valid
        assert!(test_node_id("node-1").validate().is_ok());
        assert!(test_asset_id("asset-1").validate().is_ok());
        assert!(test_matrix_position(0.0, 0.0, 0.0).is_finite());
        assert!(!test_network_id(1).is_zero());
    }

    #[test]
    fn test_proof_factories_pass_validation() {
        assert!(test_space_proof().is_structurally_valid());
        assert!(test_stake_proof().is_structurally_valid());
        assert!(test_work_proof().is_structurally_valid());
        assert!(test_time_proof().is_structurally_valid());
        assert!(test_state_proof().is_structurally_valid());
    }

    #[test]
    fn test_privacy_mode_presets() {
        assert_eq!(test_privacy_mode_anonymous(), PrivacyMode::ANONYMOUS);
        assert_eq!(test_privacy_mode_private(), PrivacyMode::PRIVATE);
        assert_eq!(test_privacy_mode_public(), PrivacyMode::PUBLIC);
    }

    #[test]
    fn test_privacy_mode_by_name() {
        assert_eq!(test_privacy_mode("anonymous"), PrivacyMode::ANONYMOUS);
        assert_eq!(test_privacy_mode("private"), PrivacyMode::PRIVATE);
        assert_eq!(test_privacy_mode("public"), PrivacyMode::PUBLIC);
        // Case-insensitive
        assert_eq!(test_privacy_mode("PUBLIC"), PrivacyMode::PUBLIC);
        // Unknown falls back to ANONYMOUS
        assert_eq!(test_privacy_mode("unknown"), PrivacyMode::ANONYMOUS);
    }

    #[test]
    fn test_content_hash_from_data_deterministic() {
        let hash1 = test_content_hash_from_data(b"hello");
        let hash2 = test_content_hash_from_data(b"hello");
        assert_eq!(hash1, hash2);
        // Different data produces different hash
        let hash3 = test_content_hash_from_data(b"world");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_fixtures_default() {
        let f = TestFixtures::default();
        assert!(f.node_id.validate().is_ok());
        assert!(f.asset_id.validate().is_ok());
        assert!(f.position.is_finite());
        assert!(!f.network_id.is_zero());
    }
}
