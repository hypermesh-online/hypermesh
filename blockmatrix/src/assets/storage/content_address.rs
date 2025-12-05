//! Content Addressing System
//!
//! Instruction-based content retrieval - Revolutionary Concept #6.
//! Send retrieval instructions (shard maps), not the actual files.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use anyhow::Result;

use crate::matrix::MatrixCoordinate;
use super::Hash;

/// Shard map entry (hash -> positions)
pub type ShardMap = Vec<(Hash, Vec<MatrixCoordinate>)>;

/// Content address with retrieval instructions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentAddress {
    /// SHA-256 hash of entire file
    pub content_hash: Hash,

    /// Hash of each shard
    pub shard_hashes: Vec<Hash>,

    /// Retrieval instructions
    pub retrieval_instructions: RetrievalInstructions,

    /// Metadata
    pub metadata: ContentMetadata,
}

/// Content metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentMetadata {
    /// Original file size (before processing)
    pub original_size: usize,

    /// Content type (MIME)
    pub content_type: String,

    /// Creation timestamp
    pub created_at: i64,

    /// Number of shards
    pub shard_count: usize,

    /// Reed-Solomon configuration (data + parity)
    pub erasure_coding: (usize, usize),

    /// Compression algorithm used
    pub compression: String,

    /// Encryption algorithm used
    pub encryption: String,
}

impl Default for ContentMetadata {
    fn default() -> Self {
        Self {
            original_size: 0,
            content_type: "application/octet-stream".to_string(),
            created_at: chrono::Utc::now().timestamp(),
            shard_count: 0,
            erasure_coding: (10, 4), // Default Reed-Solomon 10+4
            compression: "brotli".to_string(),
            encryption: "aes-256-gcm".to_string(),
        }
    }
}

/// Retrieval instructions for content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalInstructions {
    /// Map of shard hash to matrix positions
    pub shard_map: ShardMap,

    /// Order to reconstruct shards
    pub reconstruction_order: Vec<usize>,

    /// Minimum shards needed for reconstruction
    pub min_shards_required: usize,

    /// Optimal retrieval strategy
    pub strategy: RetrievalStrategy,

    /// Network hints for retrieval
    pub network_hints: NetworkHints,
}

/// Retrieval strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetrievalStrategy {
    /// Retrieve from nearest nodes first
    NearestFirst,

    /// Parallel retrieval from all nodes
    Parallel,

    /// Sequential retrieval (for low bandwidth)
    Sequential,

    /// Adaptive based on network conditions
    Adaptive {
        bandwidth_threshold: u64,
        latency_threshold: u64,
    },
}

impl Default for RetrievalStrategy {
    fn default() -> Self {
        Self::NearestFirst
    }
}

/// Network hints for optimized retrieval
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkHints {
    /// Estimated total transfer size
    pub estimated_size: usize,

    /// Recommended parallel connections
    pub parallel_connections: usize,

    /// Preferred matrix regions
    pub preferred_regions: Vec<MatrixRegion>,

    /// Avoid these overloaded nodes
    pub avoid_nodes: Vec<MatrixCoordinate>,

    /// Cache-friendly nodes (recently accessed)
    pub cache_nodes: Vec<MatrixCoordinate>,
}

/// Matrix region definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixRegion {
    /// Region center
    pub center: MatrixCoordinate,

    /// Region radius
    pub radius: f64,

    /// Region priority (higher = preferred)
    pub priority: u32,
}

impl ContentAddress {
    /// Create new content address
    pub fn new(
        content_hash: Hash,
        shard_hashes: Vec<Hash>,
        shard_map: ShardMap,
    ) -> Self {
        let reconstruction_order: Vec<usize> = (0..shard_hashes.len()).collect();

        let retrieval_instructions = RetrievalInstructions::new(shard_map);

        let mut metadata = ContentMetadata::default();
        metadata.shard_count = shard_hashes.len();

        Self {
            content_hash,
            shard_hashes,
            retrieval_instructions,
            metadata,
        }
    }

    /// Create with full metadata
    pub fn with_metadata(
        content_hash: Hash,
        shard_hashes: Vec<Hash>,
        shard_map: ShardMap,
        metadata: ContentMetadata,
    ) -> Self {
        let retrieval_instructions = RetrievalInstructions::new(shard_map);

        Self {
            content_hash,
            shard_hashes,
            retrieval_instructions,
            metadata,
        }
    }

    /// Validate content address
    pub fn validate(&self) -> Result<()> {
        if self.shard_hashes.is_empty() {
            return Err(anyhow::anyhow!("No shard hashes"));
        }

        if self.retrieval_instructions.shard_map.is_empty() {
            return Err(anyhow::anyhow!("No shard map"));
        }

        if self.shard_hashes.len() != self.retrieval_instructions.shard_map.len() {
            return Err(anyhow::anyhow!("Shard hash count mismatch"));
        }

        Ok(())
    }

    /// Get total number of matrix positions storing this content
    pub fn total_positions(&self) -> usize {
        self.retrieval_instructions.shard_map
            .iter()
            .map(|(_, positions)| positions.len())
            .sum()
    }

    /// Get unique matrix positions
    pub fn unique_positions(&self) -> Vec<MatrixCoordinate> {
        let mut positions = Vec::new();
        for (_, shard_positions) in &self.retrieval_instructions.shard_map {
            for pos in shard_positions {
                if !positions.contains(pos) {
                    positions.push(*pos);
                }
            }
        }
        positions
    }

    /// Calculate storage overhead (replication factor)
    pub fn storage_overhead(&self) -> f64 {
        let unique_shards = self.shard_hashes.len();
        let total_stored = self.total_positions();

        if unique_shards > 0 {
            total_stored as f64 / unique_shards as f64
        } else {
            0.0
        }
    }

    /// Optimize retrieval instructions based on requester position
    pub fn optimize_for_position(&mut self, requester: MatrixCoordinate) {
        // Sort shard map by distance to requester
        self.retrieval_instructions.shard_map.sort_by(|a, b| {
            let dist_a = Self::min_distance(&a.1, &requester);
            let dist_b = Self::min_distance(&b.1, &requester);
            dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Update strategy to nearest first
        self.retrieval_instructions.strategy = RetrievalStrategy::NearestFirst;

        // Add network hints
        self.retrieval_instructions.network_hints.preferred_regions.push(MatrixRegion {
            center: requester,
            radius: 5.0,
            priority: 100,
        });
    }

    /// Calculate minimum distance from positions to target
    fn min_distance(positions: &[MatrixCoordinate], target: &MatrixCoordinate) -> f64 {
        positions.iter()
            .map(|pos| {
                let dx = (pos.x - target.x) as f64;
                let dy = (pos.y - target.y) as f64;
                let dz = (pos.z - target.z) as f64;
                (dx * dx + dy * dy + dz * dz).sqrt()
            })
            .fold(f64::INFINITY, f64::min)
    }
}

impl RetrievalInstructions {
    /// Create new retrieval instructions
    pub fn new(shard_map: ShardMap) -> Self {
        let reconstruction_order: Vec<usize> = (0..shard_map.len()).collect();

        Self {
            shard_map,
            reconstruction_order,
            min_shards_required: 10, // Default for Reed-Solomon 10+4
            strategy: RetrievalStrategy::default(),
            network_hints: NetworkHints::default(),
        }
    }

    /// Create with custom strategy
    pub fn with_strategy(mut self, strategy: RetrievalStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Add network hints
    pub fn with_hints(mut self, hints: NetworkHints) -> Self {
        self.network_hints = hints;
        self
    }

    /// Get retrieval plan based on strategy
    pub fn get_retrieval_plan(&self) -> RetrievalPlan {
        match &self.strategy {
            RetrievalStrategy::NearestFirst => self.plan_nearest_first(),
            RetrievalStrategy::Parallel => self.plan_parallel(),
            RetrievalStrategy::Sequential => self.plan_sequential(),
            RetrievalStrategy::Adaptive { bandwidth_threshold, latency_threshold } => {
                self.plan_adaptive(*bandwidth_threshold, *latency_threshold)
            }
        }
    }

    fn plan_nearest_first(&self) -> RetrievalPlan {
        RetrievalPlan {
            steps: self.shard_map.iter().enumerate().map(|(i, (hash, positions))| {
                RetrievalStep {
                    shard_index: i,
                    shard_hash: *hash,
                    primary_position: positions.first().copied().unwrap_or_else(|| MatrixCoordinate::origin()),
                    fallback_positions: positions[1..].to_vec(),
                    parallel: false,
                }
            }).collect(),
            estimated_time_ms: self.shard_map.len() as u64 * 10, // Rough estimate
        }
    }

    fn plan_parallel(&self) -> RetrievalPlan {
        RetrievalPlan {
            steps: self.shard_map.iter().enumerate().map(|(i, (hash, positions))| {
                RetrievalStep {
                    shard_index: i,
                    shard_hash: *hash,
                    primary_position: positions.first().copied().unwrap_or_else(|| MatrixCoordinate::origin()),
                    fallback_positions: positions[1..].to_vec(),
                    parallel: true,
                }
            }).collect(),
            estimated_time_ms: 50, // All parallel
        }
    }

    fn plan_sequential(&self) -> RetrievalPlan {
        self.plan_nearest_first() // Sequential is same as nearest-first but slower
    }

    fn plan_adaptive(&self, _bandwidth: u64, _latency: u64) -> RetrievalPlan {
        // In production, this would analyze network conditions
        // For now, default to parallel if bandwidth > 100 Mbps
        if _bandwidth > 100_000_000 {
            self.plan_parallel()
        } else {
            self.plan_nearest_first()
        }
    }
}

/// Retrieval plan with steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalPlan {
    /// Retrieval steps in order
    pub steps: Vec<RetrievalStep>,

    /// Estimated retrieval time (ms)
    pub estimated_time_ms: u64,
}

/// Individual retrieval step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalStep {
    /// Shard index in reconstruction order
    pub shard_index: usize,

    /// Shard hash to retrieve
    pub shard_hash: Hash,

    /// Primary position to retrieve from
    pub primary_position: MatrixCoordinate,

    /// Fallback positions if primary fails
    pub fallback_positions: Vec<MatrixCoordinate>,

    /// Can be done in parallel with other steps
    pub parallel: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_address_creation() {
        let content_hash = [1u8; 32];
        let shard_hashes = vec![[2u8; 32], [3u8; 32]];
        let shard_map = vec![
            ([2u8; 32], vec![MatrixCoordinate::new(0, 0, 0).unwrap()]),
            ([3u8; 32], vec![MatrixCoordinate::new(1, 1, 0).unwrap()]),
        ];

        let addr = ContentAddress::new(content_hash, shard_hashes, shard_map);
        assert_eq!(addr.shard_hashes.len(), 2);
        assert_eq!(addr.total_positions(), 2);
        assert!(addr.validate().is_ok());
    }

    #[test]
    fn test_storage_overhead_calculation() {
        let content_hash = [1u8; 32];
        let shard_hashes = vec![[2u8; 32], [3u8; 32]];
        let shard_map = vec![
            ([2u8; 32], vec![
                MatrixCoordinate::new(0, 0, 0).unwrap(),
                MatrixCoordinate::new(1, 0, 0).unwrap(),
                MatrixCoordinate::new(2, 0, 0).unwrap(),
            ]),
            ([3u8; 32], vec![
                MatrixCoordinate::new(0, 1, 0).unwrap(),
                MatrixCoordinate::new(1, 1, 0).unwrap(),
                MatrixCoordinate::new(2, 1, 0).unwrap(),
            ]),
        ];

        let addr = ContentAddress::new(content_hash, shard_hashes, shard_map);
        assert_eq!(addr.storage_overhead(), 3.0); // 6 positions / 2 shards = 3x
    }

    #[test]
    fn test_retrieval_plan_strategies() {
        let shard_map = vec![
            ([1u8; 32], vec![MatrixCoordinate::new(0, 0, 0).unwrap()]),
            ([2u8; 32], vec![MatrixCoordinate::new(1, 0, 0).unwrap()]),
        ];

        let instructions = RetrievalInstructions::new(shard_map);

        // Test nearest-first
        let plan = instructions.with_strategy(RetrievalStrategy::NearestFirst).get_retrieval_plan();
        assert_eq!(plan.steps.len(), 2);
        assert!(!plan.steps[0].parallel);

        // Test parallel
        let plan = instructions.with_strategy(RetrievalStrategy::Parallel).get_retrieval_plan();
        assert_eq!(plan.steps.len(), 2);
        assert!(plan.steps[0].parallel);
    }

    #[test]
    fn test_optimize_for_position() {
        let content_hash = [1u8; 32];
        let shard_hashes = vec![[2u8; 32], [3u8; 32]];
        let shard_map = vec![
            ([2u8; 32], vec![MatrixCoordinate::new(10, 10, 0).unwrap()]),
            ([3u8; 32], vec![MatrixCoordinate::new(0, 0, 0).unwrap()]),
        ];

        let mut addr = ContentAddress::new(content_hash, shard_hashes, shard_map);
        let requester = MatrixCoordinate::new(0, 0, 0).unwrap();

        addr.optimize_for_position(requester);

        // Should reorder to put nearest shard first
        assert_eq!(addr.retrieval_instructions.shard_map[0].0, [3u8; 32]);
        assert_eq!(addr.retrieval_instructions.network_hints.preferred_regions.len(), 1);
    }
}