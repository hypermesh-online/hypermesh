// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Spatial hash-bucket assignment for Public mode block sync.
//!
//! In Public mode, nodes don't validate every block in the network.
//! Instead, each node is responsible for blocks whose shard placements
//! fall within its matrix neighborhood -- the "hash bucket" is spatial.
//!
//! The Block-MATRIX coordinate space acts as a 3D hash volume. Each
//! node's bucket is determined by its position and the positions of its
//! peers. This is analogous to octree spatial partitioning with
//! k-means refinement (future work).
//!
//! BLAKE3 shard hashes -> matrix placements -> spatial neighborhoods.
//! If you hold the shards, you validate the blocks that reference them.

use crate::blockchain::block::{Block, StoragePointer};
use crate::matrix::coordinate::MatrixCoordinate;

/// Default neighborhood radius when no peers are known.
const DEFAULT_RADIUS: f64 = 10.0;

/// Assigns block responsibility based on spatial proximity of shard
/// placements to this node's matrix coordinate.
///
/// The hash space maps to the 3D matrix through shard placements.
/// A node accepts blocks whose entries have shards placed within its
/// neighborhood radius -- determined by peer density in its region.
pub struct SpatialBucketAssigner {
    /// Our position in the matrix.
    our_coordinate: MatrixCoordinate,
    /// Effective neighborhood radius (adapts based on peer density).
    neighborhood_radius: f64,
    /// Known peer coordinates for density-based radius calculation.
    peer_coordinates: Vec<MatrixCoordinate>,
}

impl SpatialBucketAssigner {
    /// Create a new assigner at the given coordinate with default radius.
    pub fn new(our_coordinate: MatrixCoordinate) -> Self {
        Self {
            our_coordinate,
            neighborhood_radius: DEFAULT_RADIUS,
            peer_coordinates: Vec::new(),
        }
    }

    /// Current neighborhood radius.
    pub fn radius(&self) -> f64 {
        self.neighborhood_radius
    }

    /// Recalculate radius based on peer density.
    ///
    /// Algorithm: sort peers by distance, take k nearest (k = sqrt(n)),
    /// radius = distance to the k-th nearest. This gives octree-like
    /// adaptive partitioning -- dense regions get small radii, sparse
    /// regions get large ones.
    pub fn update_peers(&mut self, peers: Vec<MatrixCoordinate>) {
        self.peer_coordinates = peers;

        if self.peer_coordinates.is_empty() {
            self.neighborhood_radius = DEFAULT_RADIUS;
            return;
        }

        let mut distances: Vec<f64> = self
            .peer_coordinates
            .iter()
            .map(|p| self.our_coordinate.euclidean_distance(p))
            .collect();

        distances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // k = sqrt(total_peers), minimum 1
        let k = (distances.len() as f64).sqrt().ceil() as usize;
        let k = k.min(distances.len());

        // Radius = distance to the k-th nearest peer (1-indexed, so index k-1)
        let candidate = distances[k - 1];

        // Floor at 1.0 to avoid degenerate zero-radius buckets
        self.neighborhood_radius = candidate.max(1.0);
    }

    /// Check if any entry's shard placements are within our neighborhood.
    ///
    /// Only considers `StoragePointer::Sharded` entries. Local/Genesis
    /// entries are Device-scope and are never in a Public bucket.
    pub fn block_in_our_neighborhood(&self, block: &Block) -> bool {
        block.entries.iter().any(|entry| {
            let StoragePointer::Sharded { ref placements, .. } = entry.storage_pointer else {
                return false;
            };
            placements.iter().any(|p| self.is_placement_local(p))
        })
    }

    /// Check if a single placement is within our neighborhood radius.
    pub fn is_placement_local(&self, placement: &MatrixCoordinate) -> bool {
        self.our_coordinate
            .is_within_distance(placement, self.neighborhood_radius)
    }

    /// Check if a block's content is relevant to a specific peer coordinate.
    ///
    /// Returns true if any shard placement position is within the peer's
    /// spatial neighborhood. The neighborhood radius is estimated from
    /// peer density: if we have peer coordinates, use the adaptive radius;
    /// otherwise use the default radius scaled by the peer's distance
    /// from us (closer peers get tighter neighborhoods).
    pub fn block_relevant_to_peer(
        &self,
        block_shard_positions: &[MatrixCoordinate],
        peer_coordinate: &MatrixCoordinate,
    ) -> bool {
        if block_shard_positions.is_empty() {
            return false;
        }
        // Use our neighborhood radius as an approximation for the peer's radius.
        // In a uniform distribution, peers at similar density have similar radii.
        let peer_radius = self.neighborhood_radius;
        block_shard_positions.iter().any(|pos| {
            peer_coordinate.is_within_distance(pos, peer_radius)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::AssetRegistration;
    use crate::blockchain::block::{Block, BlockAssetEntry, StoragePointer};
    use trustchain::proof_of_state::StateProof;

    fn make_entry(pointer: StoragePointer) -> BlockAssetEntry {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let reg = AssetRegistration::genesis(coord);
        let content_hash = *blake3::hash(reg.to_string().as_bytes()).as_bytes();
        let state_proof = StateProof::default();
        let proof_bytes = serde_json::to_vec(&state_proof).unwrap_or_default();
        let proof_hash = *blake3::hash(&proof_bytes).as_bytes();
        BlockAssetEntry {
            asset_hash: content_hash,
            proof_hash,
            state_proof,
            signed_proof: None,
            storage_pointer: pointer,
            registration: reg,
        }
    }

    #[test]
    fn test_default_radius() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let assigner = SpatialBucketAssigner::new(coord);
        assert_eq!(assigner.radius(), DEFAULT_RADIUS);
    }

    #[test]
    fn test_update_peers_empty() {
        let coord = MatrixCoordinate::new(5, 5, 5).expect("test: coord");
        let mut assigner = SpatialBucketAssigner::new(coord);
        assigner.update_peers(vec![]);
        assert_eq!(assigner.radius(), DEFAULT_RADIUS);
    }

    #[test]
    fn test_update_peers_adapts_radius() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let mut assigner = SpatialBucketAssigner::new(coord);

        // 4 peers at distances 3, 5, 10, 20
        // k = sqrt(4) = 2, so radius = distance to 2nd nearest = 5
        let peers = vec![
            MatrixCoordinate::new(3, 0, 0).expect("test: coord"),  // dist 3
            MatrixCoordinate::new(0, 5, 0).expect("test: coord"),  // dist 5
            MatrixCoordinate::new(10, 0, 0).expect("test: coord"), // dist 10
            MatrixCoordinate::new(20, 0, 0).expect("test: coord"), // dist 20
        ];
        assigner.update_peers(peers);
        assert!((assigner.radius() - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_update_peers_single_peer() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let mut assigner = SpatialBucketAssigner::new(coord);

        // 1 peer: k = ceil(sqrt(1)) = 1, radius = distance to that peer
        let peers = vec![MatrixCoordinate::new(7, 0, 0).expect("test: coord")];
        assigner.update_peers(peers);
        assert!((assigner.radius() - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_update_peers_minimum_radius() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let mut assigner = SpatialBucketAssigner::new(coord);

        // Peer at same coordinate -> distance 0, but floor is 1.0
        let peers = vec![MatrixCoordinate::new(0, 0, 0).expect("test: coord")];
        assigner.update_peers(peers);
        assert!((assigner.radius() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_placement_local() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let assigner = SpatialBucketAssigner::new(coord); // radius 10

        let near = MatrixCoordinate::new(3, 4, 0).expect("test: coord"); // dist 5
        assert!(assigner.is_placement_local(&near));

        let far = MatrixCoordinate::new(100, 0, 0).expect("test: coord");
        assert!(!assigner.is_placement_local(&far));
    }

    #[test]
    fn test_block_with_sharded_entry_in_neighborhood() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let assigner = SpatialBucketAssigner::new(coord); // radius 10

        let entry = make_entry(StoragePointer::Sharded {
            shard_hashes: vec![[1u8; 32]],
            placements: vec![MatrixCoordinate::new(5, 0, 0).expect("test: coord")],
        });
        let block = Block::new(1, vec![entry], "prev".to_string());
        assert!(assigner.block_in_our_neighborhood(&block));
    }

    #[test]
    fn test_block_with_sharded_entry_outside_neighborhood() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let assigner = SpatialBucketAssigner::new(coord); // radius 10

        let entry = make_entry(StoragePointer::Sharded {
            shard_hashes: vec![[1u8; 32]],
            placements: vec![MatrixCoordinate::new(100, 100, 100).expect("test: coord")],
        });
        let block = Block::new(1, vec![entry], "prev".to_string());
        assert!(!assigner.block_in_our_neighborhood(&block));
    }

    #[test]
    fn test_block_with_local_entry_not_in_bucket() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let assigner = SpatialBucketAssigner::new(coord);

        let entry = make_entry(StoragePointer::Local {
            path: "/data/test".to_string(),
        });
        let block = Block::new(1, vec![entry], "prev".to_string());
        assert!(!assigner.block_in_our_neighborhood(&block));
    }

    #[test]
    fn test_block_with_genesis_entry_not_in_bucket() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let assigner = SpatialBucketAssigner::new(coord);

        let entry = make_entry(StoragePointer::Genesis);
        let block = Block::new(1, vec![entry], "prev".to_string());
        assert!(!assigner.block_in_our_neighborhood(&block));
    }

    #[test]
    fn test_block_mixed_entries_one_matching() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let assigner = SpatialBucketAssigner::new(coord); // radius 10

        let local_entry = make_entry(StoragePointer::Local {
            path: "/data/x".to_string(),
        });
        let far_entry = make_entry(StoragePointer::Sharded {
            shard_hashes: vec![[2u8; 32]],
            placements: vec![MatrixCoordinate::new(999, 999, 999).expect("test: coord")],
        });
        let near_entry = make_entry(StoragePointer::Sharded {
            shard_hashes: vec![[3u8; 32]],
            placements: vec![MatrixCoordinate::new(3, 0, 0).expect("test: coord")],
        });

        let block = Block::new(1, vec![local_entry, far_entry, near_entry], "prev".to_string());
        assert!(assigner.block_in_our_neighborhood(&block));
    }

    // ── Spatial send-side filtering tests ────────────────────────────

    #[test]
    fn test_block_relevant_to_nearby_peer() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let assigner = SpatialBucketAssigner::new(coord); // radius 10

        // Shard at (3,0,0), peer at (5,0,0) — distance from peer to shard is 2, within radius 10
        let shard_positions = vec![MatrixCoordinate::new(3, 0, 0).expect("test: coord")];
        let peer = MatrixCoordinate::new(5, 0, 0).expect("test: coord");

        assert!(
            assigner.block_relevant_to_peer(&shard_positions, &peer),
            "Peer near shard position should be relevant"
        );
    }

    #[test]
    fn test_block_not_relevant_to_distant_peer() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let mut assigner = SpatialBucketAssigner::new(coord);
        // Set a small radius by providing close peers
        assigner.update_peers(vec![
            MatrixCoordinate::new(1, 0, 0).expect("test: coord"),
        ]);
        // radius should be ~1.0

        // Shard at (0,0,0), peer at (100,100,100) — far beyond radius
        let shard_positions = vec![MatrixCoordinate::new(0, 0, 0).expect("test: coord")];
        let peer = MatrixCoordinate::new(100, 100, 100).expect("test: coord");

        assert!(
            !assigner.block_relevant_to_peer(&shard_positions, &peer),
            "Peer far from all shard positions should not be relevant"
        );
    }

    #[test]
    fn test_block_relevant_empty_positions() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let assigner = SpatialBucketAssigner::new(coord);

        let peer = MatrixCoordinate::new(1, 1, 1).expect("test: coord");

        assert!(
            !assigner.block_relevant_to_peer(&[], &peer),
            "Empty shard positions should never be relevant"
        );
    }

    #[test]
    fn test_block_relevant_multiple_positions_one_matches() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let mut assigner = SpatialBucketAssigner::new(coord);
        // Small radius
        assigner.update_peers(vec![
            MatrixCoordinate::new(2, 0, 0).expect("test: coord"),
        ]);

        let shard_positions = vec![
            MatrixCoordinate::new(500, 500, 500).expect("test: coord"), // far
            MatrixCoordinate::new(5, 5, 5).expect("test: coord"),       // near peer
        ];
        let peer = MatrixCoordinate::new(5, 5, 6).expect("test: coord"); // close to second shard

        assert!(
            assigner.block_relevant_to_peer(&shard_positions, &peer),
            "At least one matching shard position should make block relevant"
        );
    }
}
