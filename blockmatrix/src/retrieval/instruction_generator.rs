// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Instruction Generator
//!
//! Generates compact retrieval instructions from content addresses.

use anyhow::Result;
use std::sync::Arc;

use crate::matrix::MatrixCoordinate;
use crate::assets::storage::{ContentAddress, ContentAddressedStorage, Hash};
use crate::integration::phase1_foundation::MatrixFoundation;

use super::{RetrievalPlan, RetrievalMetadata, CompleteShardMap, ShardMapEntry, ShardLocation};

/// Configuration for instruction generation
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    /// Include health scores for replica selection
    pub include_health_scores: bool,

    /// Include estimated latency metrics
    pub include_latency: bool,

    /// Optimize for specific client position
    pub optimize_for_client: Option<MatrixCoordinate>,

    /// Minimum replicas to include per shard
    pub min_replicas: usize,

    /// Maximum replicas to include per shard
    pub max_replicas: usize,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            include_health_scores: true,
            include_latency: true,
            optimize_for_client: None,
            min_replicas: 1,
            max_replicas: 3,
        }
    }
}

/// Instruction generator that creates retrieval plans
pub struct InstructionGenerator {
    /// Configuration
    config: GeneratorConfig,

    /// Matrix foundation for distance calculations
    foundation: Arc<MatrixFoundation>,

    /// Content-addressed storage for data access
    storage: Arc<ContentAddressedStorage>,
}

impl InstructionGenerator {
    /// Create a new instruction generator
    pub fn new(
        config: GeneratorConfig,
        foundation: Arc<MatrixFoundation>,
        storage: Arc<ContentAddressedStorage>,
    ) -> Self {
        Self {
            config,
            foundation,
            storage,
        }
    }

    /// Generate retrieval instructions for a content hash
    pub async fn generate(
        &self,
        content_hash: Hash,
    ) -> Result<RetrievalPlan> {
        // Get content address from storage
        let content_address = self.get_content_address(content_hash).await?;

        // Build complete shard map with locations
        let shard_map = self.build_shard_map(&content_address).await?;

        // Create retrieval metadata
        let metadata = self.create_metadata(&content_address);

        // Create initial plan
        let mut plan = RetrievalPlan::new(content_hash, shard_map, metadata);
        plan.original_size = content_address.metadata.original_size;

        // Optimize for client position if specified
        if let Some(client_pos) = &self.config.optimize_for_client {
            plan.optimize_for_position(client_pos);
        }

        // Validate before returning
        plan.validate()?;

        Ok(plan)
    }

    /// Generate with custom client position optimization
    pub async fn generate_for_client(
        &self,
        content_hash: Hash,
        client_position: MatrixCoordinate,
    ) -> Result<RetrievalPlan> {
        let mut config = self.config.clone();
        config.optimize_for_client = Some(client_position);

        let generator = Self::new(config, self.foundation.clone(), self.storage.clone());
        generator.generate(content_hash).await
    }

    /// Get content address from storage
    async fn get_content_address(&self, content_hash: Hash) -> Result<ContentAddress> {
        // In production, this would query the storage layer
        // For now, we reconstruct from retrieval instructions
        let retrieval_instructions = self.storage.retrieve(content_hash).await?;

        // Convert to ContentAddress
        Ok(ContentAddress::new(
            content_hash,
            retrieval_instructions.shard_map.iter()
                .map(|(hash, _)| *hash)
                .collect(),
            retrieval_instructions.shard_map.clone(),
        ))
    }

    /// Build complete shard map with all replica locations
    async fn build_shard_map(
        &self,
        content_address: &ContentAddress,
    ) -> Result<CompleteShardMap> {
        let mut entries = Vec::new();

        for (shard_hash, positions) in &content_address.retrieval_instructions.shard_map {
            // Limit replicas based on config
            let replica_count = positions.len()
                .min(self.config.max_replicas)
                .max(self.config.min_replicas.min(positions.len()));

            let locations: Vec<ShardLocation> = positions.iter()
                .take(replica_count)
                .map(|pos| {
                    let health_score = self.estimate_node_health(pos);
                    let mut location = ShardLocation::new(pos.clone(), health_score);

                    // Add latency if configured
                    if self.config.include_latency {
                        location.estimated_latency_ms = self.estimate_latency(pos);
                    }

                    // Set priority based on position
                    location.priority = self.calculate_priority(pos);

                    location
                })
                .collect();

            let entry = ShardMapEntry::new(*shard_hash, locations);
            entries.push(entry);
        }

        Ok(CompleteShardMap::from_entries(entries))
    }

    /// Create retrieval metadata from content address
    fn create_metadata(&self, content_address: &ContentAddress) -> RetrievalMetadata {
        RetrievalMetadata {
            erasure_coding: content_address.metadata.erasure_coding,
            compression: content_address.metadata.compression.clone(),
            encryption: content_address.metadata.encryption.clone(),
            content_type: content_address.metadata.content_type.clone(),
            created_at: content_address.metadata.created_at,
        }
    }

    /// Estimate node health score (placeholder)
    fn estimate_node_health(&self, _position: &MatrixCoordinate) -> f64 {
        // In production, this would query actual node health metrics
        // For now, assume all nodes are healthy
        0.95
    }

    /// Estimate latency to position (placeholder)
    fn estimate_latency(&self, position: &MatrixCoordinate) -> u64 {
        // Simple distance-based estimation
        // In production, this would use actual network measurements
        if let Some(client_pos) = &self.config.optimize_for_client {
            let dx = (position.x - client_pos.x) as f64;
            let dy = (position.y - client_pos.y) as f64;
            let dz = (position.z - client_pos.z) as f64;
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();

            // Assume 1ms per unit distance (rough estimation)
            distance as u64
        } else {
            10 // Default 10ms
        }
    }

    /// Calculate replica priority
    fn calculate_priority(&self, position: &MatrixCoordinate) -> u32 {
        // Prioritize based on distance from client
        if let Some(client_pos) = &self.config.optimize_for_client {
            let dx = (position.x - client_pos.x) as f64;
            let dy = (position.y - client_pos.y) as f64;
            let dz = (position.z - client_pos.z) as f64;
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();

            // Convert distance to priority (closer = higher)
            let priority = 100.0 / (1.0 + distance);
            priority as u32
        } else {
            100 // Default priority
        }
    }

    /// Estimate instruction size for a content hash
    pub async fn estimate_instruction_size(&self, content_hash: Hash) -> Result<usize> {
        let plan = self.generate(content_hash).await?;
        Ok(plan.estimate_size())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration::phase1_foundation::MatrixFoundationConfig;
    use tempfile::TempDir;

    async fn create_test_generator() -> (InstructionGenerator, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = MatrixFoundationConfig {
            storage_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let foundation = Arc::new(MatrixFoundation::new(config).await.unwrap());
        let storage = Arc::new(
            ContentAddressedStorage::new(foundation.clone()).await.unwrap()
        );

        let gen_config = GeneratorConfig::default();
        let generator = InstructionGenerator::new(gen_config, foundation, storage);

        (generator, temp_dir)
    }

    #[tokio::test]
    async fn test_generator_creation() {
        let (_generator, _temp_dir) = create_test_generator().await;
        // Successfully created
    }

    #[tokio::test]
    async fn test_config_defaults() {
        let config = GeneratorConfig::default();
        assert!(config.include_health_scores);
        assert!(config.include_latency);
        assert_eq!(config.min_replicas, 1);
        assert_eq!(config.max_replicas, 3);
    }

    #[test]
    fn test_health_estimation() {
        let pos = MatrixCoordinate::new(10, 20, 30).unwrap();

        // Create minimal generator for testing
        let config = GeneratorConfig::default();
        let foundation = Arc::new(
            futures::executor::block_on(async {
                MatrixFoundation::new(MatrixFoundationConfig {
                    storage_path: std::path::PathBuf::from("/tmp/test"),
                    ..Default::default()
                }).await.unwrap()
            })
        );
        let storage = Arc::new(
            futures::executor::block_on(async {
                ContentAddressedStorage::new(foundation.clone()).await.unwrap()
            })
        );

        let generator = InstructionGenerator::new(config, foundation, storage);
        let health = generator.estimate_node_health(&pos);

        assert!(health >= 0.0 && health <= 1.0);
    }

    #[test]
    fn test_priority_calculation() {
        let pos = MatrixCoordinate::new(10, 0, 0).unwrap();
        let client_pos = MatrixCoordinate::new(0, 0, 0).unwrap();

        let mut config = GeneratorConfig::default();
        config.optimize_for_client = Some(client_pos);

        let foundation = Arc::new(
            futures::executor::block_on(async {
                MatrixFoundation::new(MatrixFoundationConfig {
                    storage_path: std::path::PathBuf::from("/tmp/test"),
                    ..Default::default()
                }).await.unwrap()
            })
        );
        let storage = Arc::new(
            futures::executor::block_on(async {
                ContentAddressedStorage::new(foundation.clone()).await.unwrap()
            })
        );

        let generator = InstructionGenerator::new(config, foundation, storage);
        let priority = generator.calculate_priority(&pos);

        assert!(priority > 0 && priority <= 100);
    }
}
