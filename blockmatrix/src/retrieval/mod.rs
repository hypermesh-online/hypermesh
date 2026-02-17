// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Instruction-Based Retrieval System - Revolutionary Concept #6
//!
//! Revolutionary approach: Instead of transferring entire files (potentially multi-GB),
//! send tiny instruction maps (<1KB) that tell clients:
//! - Content hash (what to retrieve)
//! - Shard hashes (which pieces)
//! - Matrix positions (where each shard is stored)
//!
//! Client fetches shards directly from matrix positions and reconstructs the file.
//!
//! ## Key Innovation
//!
//! Traditional CDN: Transfer 1GB file → 1GB network traffic
//! BlockMatrix: Transfer 1KB instruction map → Client fetches shards directly
//!
//! ## Performance Targets
//!
//! - Instruction size: <1KB for any file size (even 1TB+)
//! - Fallback tolerance: Handle 30% shard unavailability
//! - Reconstruction speed: Limited only by network bandwidth
//! - Distance optimization: Prioritize nearest replicas

use anyhow::Result;
use serde::{Serialize, Deserialize};

use crate::matrix::MatrixCoordinate;
use crate::assets::storage::{Hash, ContentAddress};

// Sub-modules
pub mod shard_map;
pub mod instruction_generator;
pub mod transmission;
pub mod client_assembly;
pub mod fallback;

// Re-exports
pub use shard_map::{ShardLocation, ShardMapEntry, CompleteShardMap};
pub use instruction_generator::{InstructionGenerator, GeneratorConfig};
pub use transmission::{InstructionTransmitter, TransmissionStats, CompressionFormat};
pub use client_assembly::{ClientAssembler, AssemblyProgress, AssemblyStats};
pub use fallback::{FallbackManager, FallbackStrategy, ReplicaSelector};

/// Retrieval plan with complete instructions for client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalPlan {
    /// Content hash to retrieve
    pub content_hash: Hash,

    /// Complete shard map with all locations
    pub shard_map: CompleteShardMap,

    /// Recommended retrieval order (optimized for client position)
    pub retrieval_order: Vec<usize>,

    /// Minimum shards needed for reconstruction (Reed-Solomon threshold)
    pub min_shards_required: usize,

    /// Total original file size
    pub original_size: usize,

    /// Metadata for reconstruction
    pub metadata: RetrievalMetadata,
}

/// Metadata needed for reconstruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalMetadata {
    /// Reed-Solomon configuration (data + parity shards)
    pub erasure_coding: (usize, usize),

    /// Compression algorithm used
    pub compression: String,

    /// Encryption algorithm used
    pub encryption: String,

    /// Content type (MIME)
    pub content_type: String,

    /// Creation timestamp
    pub created_at: i64,
}

impl RetrievalPlan {
    /// Create a new retrieval plan
    pub fn new(
        content_hash: Hash,
        shard_map: CompleteShardMap,
        metadata: RetrievalMetadata,
    ) -> Self {
        let shard_count = shard_map.entries.len();
        let min_shards_required = metadata.erasure_coding.0; // Data shards only

        Self {
            content_hash,
            shard_map,
            retrieval_order: (0..shard_count).collect(),
            min_shards_required,
            original_size: 0, // Will be set by generator
            metadata,
        }
    }

    /// Optimize retrieval order for a specific client position
    pub fn optimize_for_position(&mut self, client_position: &MatrixCoordinate) {
        // Sort shard map entries by minimum distance to client
        let mut indexed: Vec<(usize, f64)> = self.shard_map.entries.iter()
            .enumerate()
            .map(|(idx, entry)| {
                let min_distance = entry.locations.iter()
                    .map(|loc| loc.distance_to(client_position))
                    .fold(f64::INFINITY, f64::min);
                (idx, min_distance)
            })
            .collect();

        // Sort by distance (nearest first)
        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Update retrieval order
        self.retrieval_order = indexed.into_iter().map(|(idx, _)| idx).collect();
    }

    /// Validate the retrieval plan
    pub fn validate(&self) -> Result<()> {
        if self.shard_map.entries.is_empty() {
            return Err(anyhow::anyhow!("Empty shard map"));
        }

        if self.min_shards_required == 0 {
            return Err(anyhow::anyhow!("Invalid min_shards_required"));
        }

        if self.min_shards_required > self.shard_map.entries.len() {
            return Err(anyhow::anyhow!(
                "min_shards_required ({}) exceeds available shards ({})",
                self.min_shards_required,
                self.shard_map.entries.len()
            ));
        }

        // Verify each shard has at least one location
        for (idx, entry) in self.shard_map.entries.iter().enumerate() {
            if entry.locations.is_empty() {
                return Err(anyhow::anyhow!("Shard {} has no locations", idx));
            }
        }

        Ok(())
    }

    /// Estimate instruction size in bytes
    pub fn estimate_size(&self) -> usize {
        // Rough estimation based on structure
        let base_size = 32 + 8 + 8 + 8; // Hash + counts + metadata
        let shard_map_size = self.shard_map.estimate_size();
        let order_size = self.retrieval_order.len() * 8;
        let metadata_size = 200; // Approximate metadata overhead

        base_size + shard_map_size + order_size + metadata_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::storage::ContentMetadata;

    fn create_test_metadata() -> RetrievalMetadata {
        RetrievalMetadata {
            erasure_coding: (10, 4),
            compression: "brotli".to_string(),
            encryption: "aes-256-gcm".to_string(),
            content_type: "application/octet-stream".to_string(),
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    #[test]
    fn test_retrieval_plan_creation() {
        let content_hash = [1u8; 32];
        let shard_map = CompleteShardMap::new();
        let metadata = create_test_metadata();

        let plan = RetrievalPlan::new(content_hash, shard_map, metadata);
        assert_eq!(plan.content_hash, content_hash);
    }

    #[test]
    fn test_retrieval_plan_validation_empty() {
        let content_hash = [1u8; 32];
        let shard_map = CompleteShardMap::new();
        let metadata = create_test_metadata();

        let plan = RetrievalPlan::new(content_hash, shard_map, metadata);
        assert!(plan.validate().is_err()); // Empty shard map should fail
    }

    #[test]
    fn test_retrieval_plan_validation_valid() {
        use crate::assets::storage::ShardMetadata;

        let content_hash = [1u8; 32];
        let mut shard_map = CompleteShardMap::new();

        // Add valid entries
        for i in 0..14 {
            let shard_hash = [i as u8; 32];
            let position = MatrixCoordinate::new(i as i64, 0, 0).unwrap();
            let location = ShardLocation::new(position, 1.0);

            let entry = ShardMapEntry {
                shard_hash,
                locations: vec![location],
            };
            shard_map.add_entry(entry);
        }

        let metadata = create_test_metadata();
        let plan = RetrievalPlan::new(content_hash, shard_map, metadata);
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn test_optimize_for_position() {
        let content_hash = [1u8; 32];
        let mut shard_map = CompleteShardMap::new();

        // Add shards at different distances
        let positions = vec![
            MatrixCoordinate::new(10, 0, 0).unwrap(),
            MatrixCoordinate::new(0, 0, 0).unwrap(), // Nearest
            MatrixCoordinate::new(5, 0, 0).unwrap(),
        ];

        for (i, pos) in positions.iter().enumerate() {
            let shard_hash = [i as u8; 32];
            let location = ShardLocation::new(pos.clone(), 1.0);

            let entry = ShardMapEntry {
                shard_hash,
                locations: vec![location],
            };
            shard_map.add_entry(entry);
        }

        let metadata = create_test_metadata();
        let mut plan = RetrievalPlan::new(content_hash, shard_map, metadata);

        // Optimize for origin
        let client_pos = MatrixCoordinate::new(0, 0, 0).unwrap();
        plan.optimize_for_position(&client_pos);

        // Should prioritize nearest shard (index 1)
        assert_eq!(plan.retrieval_order[0], 1);
    }

    #[test]
    fn test_estimate_size() {
        let content_hash = [1u8; 32];
        let mut shard_map = CompleteShardMap::new();

        // Add 14 shards (typical for Reed-Solomon 10+4)
        for i in 0..14 {
            let shard_hash = [i as u8; 32];
            let position = MatrixCoordinate::new(i as i64, 0, 0).unwrap();
            let location = ShardLocation::new(position, 1.0);

            let entry = ShardMapEntry {
                shard_hash,
                locations: vec![location],
            };
            shard_map.add_entry(entry);
        }

        let metadata = create_test_metadata();
        let plan = RetrievalPlan::new(content_hash, shard_map, metadata);

        let size = plan.estimate_size();
        println!("Estimated instruction size: {} bytes", size);

        // Should be well under 1KB for 14 shards
        assert!(size < 1024, "Instruction size {} exceeds 1KB", size);
    }
}
