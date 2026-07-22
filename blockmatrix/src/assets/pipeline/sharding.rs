// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Sharding Stage - Reed-Solomon erasure coding (re-export shim).
//!
//! The Reed-Solomon sharding ENGINE (`Sharder`, `ShardingConfig`) now lives in
//! the `ngauge` crate — NGauge is the sharding authority. The shared shard
//! data structures (`Shard`, `ShardMetadata`, `ShardingStats`) and the
//! `ShardingError` live in `hypermesh-lib` so every consumer can use them
//! without a dependency cycle.
//!
//! This module re-exports both so existing `crate::assets::pipeline::sharding::*`
//! call sites keep resolving unchanged. blockmatrix now CALLS the engine
//! (`ngauge::Sharder`) rather than owning it; everything else in the pipeline
//! (compress, encrypt, hash, distribute) stays in blockmatrix.

pub use ngauge::sharding::{Sharder, ShardingConfig};
pub use hypermesh_lib::{Shard, ShardMetadata, ShardingError, ShardingStats};

#[cfg(test)]
mod tests {
    use super::*;

    /// Seam test: a store->shard->reconstruct round-trip through the re-exported
    /// `ngauge::Sharder` reached via the blockmatrix pipeline path must be
    /// byte-identical. This proves the re-export shim wires the engine
    /// correctly end-to-end from blockmatrix.
    #[test]
    fn test_pipeline_sharding_roundtrip_via_ngauge() {
        let sharder = Sharder::new(ShardingConfig::default()).expect("test: create sharder");
        let data = b"blockmatrix pipeline seam -> ngauge sharding authority".repeat(200);

        let (shards, stats) = sharder.shard(&data).expect("test: shard");
        assert_eq!(shards.len(), 14); // RS(10,4)
        assert_eq!(stats.data_shards, 10);
        assert_eq!(stats.parity_shards, 4);

        // Reconstruct from the minimum data shards (drop all parity).
        let data_only: Vec<Shard> = shards
            .into_iter()
            .filter(|s| !s.metadata.is_parity)
            .collect();
        let reconstructed = sharder
            .reconstruct(&data_only)
            .expect("test: reconstruct");
        assert_eq!(reconstructed, data, "round-trip must be byte-identical");
    }

    /// The shared `Shard`/`ShardMetadata` types re-exported here must be the
    /// canonical `hypermesh_lib` types (single source of truth).
    #[test]
    fn test_reexported_types_are_lib_types() {
        let meta: ShardMetadata = hypermesh_lib::ShardMetadata::default();
        let shard: Shard = hypermesh_lib::Shard {
            data: vec![1, 2, 3],
            metadata: meta,
        };
        // hash is empty by default -> verify() is false until stamped.
        assert!(!shard.verify());
    }
}
