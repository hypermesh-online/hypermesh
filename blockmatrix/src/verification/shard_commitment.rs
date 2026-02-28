// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Shard commitment computation and privacy-filtered map generation.
//!
//! The commitment is position-based (not identity-based):
//! `shard_commitment = BLAKE3(canonical_serialize(sorted_by_index(placements)))`
//!
//! The commitment hash is identical regardless of privacy mode -- only the
//! preimage disclosure differs (tracked mode reveals NodeIds).

use hypermesh_lib::{MatrixPosition, NodeId};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use super::types::{FilteredShardEntry, FilteredShardMap};

/// Full shard distribution map (node-local, not shared directly).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardDistributionMap {
    pub block_index: u64,
    pub entries: Vec<ShardPlacement>,
    #[serde(with = "super::types::system_time_serde")]
    pub created_at: SystemTime,
}

/// A single shard placement in the distribution map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardPlacement {
    pub shard_index: u16,
    pub is_parity: bool,
    pub target_position: MatrixPosition,
    pub shard_hash: [u8; 32],
    /// The node at this position (always known locally).
    pub target_node_id: NodeId,
}

impl ShardDistributionMap {
    /// Compute the BLAKE3 commitment hash for this shard distribution.
    ///
    /// The commitment is position-based: sorted by shard_index, then hashed.
    /// NodeIds are NOT included in the commitment -- they are metadata.
    pub fn compute_commitment(&self) -> [u8; 32] {
        let mut sorted = self.entries.clone();
        sorted.sort_by_key(|e| e.shard_index);

        let mut hasher = blake3::Hasher::new();
        for entry in &sorted {
            hasher.update(&entry.shard_index.to_le_bytes());
            hasher.update(&[entry.is_parity as u8]);
            hasher.update(&entry.target_position.x.to_le_bytes());
            hasher.update(&entry.target_position.y.to_le_bytes());
            hasher.update(&entry.target_position.z.to_le_bytes());
            hasher.update(&entry.shard_hash);
        }
        *hasher.finalize().as_bytes()
    }

    /// Generate a privacy-filtered map for a verifier.
    ///
    /// - `tracked == true`: include NodeIds (Public/Private mode)
    /// - `tracked == false`: strip NodeIds (Anonymous mode)
    pub fn to_filtered_map(&self, tracked: bool) -> FilteredShardMap {
        let commitment = self.compute_commitment();
        let entries = self
            .entries
            .iter()
            .map(|e| FilteredShardEntry {
                shard_index: e.shard_index,
                is_parity: e.is_parity,
                target_position: e.target_position,
                shard_hash: e.shard_hash,
                target_node_id: if tracked {
                    Some(e.target_node_id.clone())
                } else {
                    None
                },
            })
            .collect();
        FilteredShardMap { entries, commitment }
    }
}

/// Create a `ShardDistributionMap` from distribution results.
///
/// This is the bridge between the distribution pipeline and the verification module.
/// After shards are distributed to matrix positions, call this to build the map
/// that can then produce a commitment hash for anchoring in a block.
pub fn create_from_distribution(
    block_index: u64,
    placements: Vec<ShardPlacement>,
) -> ShardDistributionMap {
    ShardDistributionMap {
        block_index,
        entries: placements,
        created_at: std::time::SystemTime::now(),
    }
}

/// Verify that a filtered shard map is consistent with a commitment hash.
///
/// Recomputes the commitment from the filtered entries (without NodeIds,
/// since NodeIds are never part of the commitment) and checks equality.
pub fn verify_commitment(map: &FilteredShardMap) -> bool {
    let mut sorted: Vec<&FilteredShardEntry> = map.entries.iter().collect();
    sorted.sort_by_key(|e| e.shard_index);

    let mut hasher = blake3::Hasher::new();
    for entry in &sorted {
        hasher.update(&entry.shard_index.to_le_bytes());
        hasher.update(&[entry.is_parity as u8]);
        hasher.update(&entry.target_position.x.to_le_bytes());
        hasher.update(&entry.target_position.y.to_le_bytes());
        hasher.update(&entry.target_position.z.to_le_bytes());
        hasher.update(&entry.shard_hash);
    }
    let computed = *hasher.finalize().as_bytes();
    computed == map.commitment
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_position(x: f64, y: f64, z: f64) -> MatrixPosition {
        MatrixPosition { x, y, z }
    }

    fn sample_map() -> ShardDistributionMap {
        ShardDistributionMap {
            block_index: 42,
            entries: vec![
                ShardPlacement {
                    shard_index: 0,
                    is_parity: false,
                    target_position: test_position(1.0, 2.0, 3.0),
                    shard_hash: [0xAA; 32],
                    target_node_id: NodeId::from("node-alpha"),
                },
                ShardPlacement {
                    shard_index: 1,
                    is_parity: false,
                    target_position: test_position(4.0, 5.0, 6.0),
                    shard_hash: [0xBB; 32],
                    target_node_id: NodeId::from("node-beta"),
                },
                ShardPlacement {
                    shard_index: 10,
                    is_parity: true,
                    target_position: test_position(7.0, 8.0, 9.0),
                    shard_hash: [0xCC; 32],
                    target_node_id: NodeId::from("node-gamma"),
                },
            ],
            created_at: SystemTime::now(),
        }
    }

    #[test]
    fn commitment_is_deterministic() {
        let map = sample_map();
        let c1 = map.compute_commitment();
        let c2 = map.compute_commitment();
        assert_eq!(c1, c2, "commitment must be deterministic");
    }

    #[test]
    fn commitment_changes_on_different_data() {
        let mut map = sample_map();
        let c1 = map.compute_commitment();
        map.entries[0].shard_hash = [0xFF; 32];
        let c2 = map.compute_commitment();
        assert_ne!(c1, c2, "different shard hash must yield different commitment");
    }

    #[test]
    fn commitment_independent_of_entry_order() {
        let map = sample_map();
        let c1 = map.compute_commitment();

        // Reverse entries -- commitment sorts by shard_index internally
        let mut reversed_map = map.clone();
        reversed_map.entries.reverse();
        let c2 = reversed_map.compute_commitment();
        assert_eq!(c1, c2, "commitment must be independent of entry order");
    }

    #[test]
    fn filtered_map_tracked_includes_node_ids() {
        let map = sample_map();
        let filtered = map.to_filtered_map(true);
        for entry in &filtered.entries {
            assert!(
                entry.target_node_id.is_some(),
                "tracked mode must include NodeIds"
            );
        }
    }

    #[test]
    fn filtered_map_untracked_strips_node_ids() {
        let map = sample_map();
        let filtered = map.to_filtered_map(false);
        for entry in &filtered.entries {
            assert!(
                entry.target_node_id.is_none(),
                "untracked mode must strip NodeIds"
            );
        }
    }

    #[test]
    fn verify_commitment_roundtrip() {
        let map = sample_map();
        let filtered_tracked = map.to_filtered_map(true);
        assert!(
            verify_commitment(&filtered_tracked),
            "tracked map must verify"
        );

        let filtered_anon = map.to_filtered_map(false);
        assert!(
            verify_commitment(&filtered_anon),
            "anonymous map must verify"
        );
    }

    #[test]
    fn verify_commitment_detects_tampering() {
        let map = sample_map();
        let mut filtered = map.to_filtered_map(true);
        filtered.entries[0].shard_hash = [0xFF; 32]; // tamper
        assert!(
            !verify_commitment(&filtered),
            "tampered map must fail verification"
        );
    }

    #[test]
    fn commitment_independent_of_node_ids() {
        let map = sample_map();
        let tracked = map.to_filtered_map(true);
        let anon = map.to_filtered_map(false);
        assert_eq!(
            tracked.commitment, anon.commitment,
            "commitment must be identical regardless of tracked/untracked"
        );
    }

    #[test]
    fn empty_map_produces_valid_commitment() {
        let map = ShardDistributionMap {
            block_index: 0,
            entries: vec![],
            created_at: SystemTime::now(),
        };
        let commitment = map.compute_commitment();
        let filtered = map.to_filtered_map(false);
        assert!(verify_commitment(&filtered));
        // Empty hash should be BLAKE3 of empty input
        assert_eq!(commitment, *blake3::hash(b"").as_bytes());
    }
}
