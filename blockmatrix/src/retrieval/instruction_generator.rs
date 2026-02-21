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

    /// Base latency in milliseconds (fixed overhead per request)
    pub base_latency_ms: f64,

    /// Additional latency per unit of Euclidean distance
    pub per_hop_latency_ms: f64,

    /// Maximum matrix distance used for normalization in scoring
    pub max_expected_distance: f64,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            include_health_scores: true,
            include_latency: true,
            optimize_for_client: None,
            min_replicas: 1,
            max_replicas: 3,
            base_latency_ms: 5.0,
            per_hop_latency_ms: 2.0,
            max_expected_distance: 100.0,
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
        let data_shard_count = content_address.metadata.erasure_coding.0;

        for (shard_index, (shard_hash, positions)) in
            content_address.retrieval_instructions.shard_map.iter().enumerate()
        {
            // Limit replicas based on config
            let replica_count = positions.len()
                .min(self.config.max_replicas)
                .max(self.config.min_replicas.min(positions.len()));

            let is_data_shard = shard_index < data_shard_count;

            let locations: Vec<ShardLocation> = positions.iter()
                .take(replica_count)
                .map(|pos| {
                    let health_score = self.estimate_node_health(pos);
                    let mut location = ShardLocation::new(pos.clone(), health_score);

                    // Add latency if configured
                    if self.config.include_latency {
                        location.estimated_latency_ms = self.estimate_latency(pos);
                    }

                    // Set priority combining distance, health, and shard type
                    location.priority = self.calculate_priority(
                        pos,
                        health_score,
                        is_data_shard,
                    );

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
            encrypted_blob_size: 0, // Populated during asset processing
        }
    }

    /// Estimate node health score based on matrix distance.
    ///
    /// Uses distance from the requesting client position as a proxy for
    /// connection reliability. Closer nodes tend to have more stable
    /// connections and lower packet loss. Returns 0.8 as a reasonable
    /// baseline when no client position is configured.
    ///
    /// The score follows a decay curve: `0.95 * e^(-distance / max_distance)`
    /// clamped to a minimum of 0.5 to avoid excluding distant but functional nodes.
    fn estimate_node_health(&self, position: &MatrixCoordinate) -> f64 {
        let Some(client_pos) = &self.config.optimize_for_client else {
            // No client position known — use a reasonable baseline
            return 0.8;
        };

        let distance = client_pos.euclidean_distance(position);

        // Exponential decay: nearby nodes score near 0.95, distant nodes decay
        // toward 0.5. The decay rate is tuned by max_expected_distance so that
        // nodes at max distance score roughly 0.5 * 0.95 ≈ 0.475 → clamped to 0.5.
        let decay_rate = 2.0 / self.config.max_expected_distance;
        let raw_score = 0.95 * (-decay_rate * distance).exp();

        // Clamp to [0.5, 1.0] — even far nodes get a fair baseline
        raw_score.clamp(0.5, 1.0)
    }

    /// Estimate network latency to a position using a linear distance model.
    ///
    /// Applies the formula: `base_latency_ms + distance * per_hop_latency_ms`
    /// where distance is the Euclidean distance in matrix coordinate space.
    /// Uses `MatrixCoordinate::euclidean_distance()` for the calculation.
    fn estimate_latency(&self, position: &MatrixCoordinate) -> u64 {
        let Some(client_pos) = &self.config.optimize_for_client else {
            // No client position — return base latency as default
            return self.config.base_latency_ms as u64;
        };

        let distance = client_pos.euclidean_distance(position);
        let latency = self.config.base_latency_ms
            + distance * self.config.per_hop_latency_ms;

        latency as u64
    }

    /// Calculate a normalized priority score (mapped to 0-100 u32 range)
    /// combining multiple factors:
    ///
    /// - **Distance factor** (40% weight): Closer replicas are preferred.
    ///   Normalized via `1 / (1 + distance/max_expected_distance)`.
    /// - **Health factor** (35% weight): Healthier nodes are preferred.
    ///   Uses the health_score directly (already 0.0-1.0).
    /// - **Shard type factor** (25% weight): Data shards get a slight boost
    ///   over parity shards since they are needed for direct reconstruction.
    fn calculate_priority(
        &self,
        position: &MatrixCoordinate,
        health_score: f64,
        is_data_shard: bool,
    ) -> u32 {
        // Distance factor: normalized so nearby = 1.0, distant = approaches 0.0
        let distance_factor = if let Some(client_pos) = &self.config.optimize_for_client {
            let distance = client_pos.euclidean_distance(position);
            let normalized = distance / self.config.max_expected_distance;
            1.0 / (1.0 + normalized)
        } else {
            // No client position — treat all distances as equal
            1.0
        };

        // Health factor: already normalized 0.0-1.0
        let health_factor = health_score;

        // Shard type factor: data shards = 1.0, parity shards = 0.7
        let shard_type_factor = if is_data_shard { 1.0 } else { 0.7 };

        // Weighted combination → normalized 0.0-1.0
        let combined = 0.40 * distance_factor
            + 0.35 * health_factor
            + 0.25 * shard_type_factor;

        // Scale to u32 range 0-100
        let priority = (combined * 100.0).round() as u32;
        priority.min(100)
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

    /// Create a test generator with a real MatrixFoundation backed by a temp directory.
    async fn create_test_generator() -> (InstructionGenerator, TempDir) {
        let temp_dir = TempDir::new().expect("test: create temp dir");
        let config = MatrixFoundationConfig {
            storage_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let foundation = Arc::new(
            MatrixFoundation::new(config).await.expect("test: create foundation"),
        );
        let storage = Arc::new(
            ContentAddressedStorage::new(foundation.clone())
                .await
                .expect("test: create storage"),
        );

        let gen_config = GeneratorConfig::default();
        let generator = InstructionGenerator::new(gen_config, foundation, storage);

        (generator, temp_dir)
    }

    /// Async helper: build a generator with a specific client position.
    async fn build_generator_with_client(
        client_pos: MatrixCoordinate,
    ) -> (InstructionGenerator, TempDir) {
        let temp_dir = TempDir::new().expect("test: create temp dir");
        let mut config = GeneratorConfig::default();
        config.optimize_for_client = Some(client_pos);

        let foundation = Arc::new(
            MatrixFoundation::new(MatrixFoundationConfig {
                storage_path: temp_dir.path().to_path_buf(),
                ..Default::default()
            })
            .await
            .expect("test: create foundation"),
        );
        let storage = Arc::new(
            ContentAddressedStorage::new(foundation.clone())
                .await
                .expect("test: create storage"),
        );

        let generator = InstructionGenerator::new(config, foundation, storage);
        (generator, temp_dir)
    }

    /// Async helper: build a generator with no client position.
    async fn build_generator_no_client() -> (InstructionGenerator, TempDir) {
        let temp_dir = TempDir::new().expect("test: create temp dir");
        let config = GeneratorConfig::default();

        let foundation = Arc::new(
            MatrixFoundation::new(MatrixFoundationConfig {
                storage_path: temp_dir.path().to_path_buf(),
                ..Default::default()
            })
            .await
            .expect("test: create foundation"),
        );
        let storage = Arc::new(
            ContentAddressedStorage::new(foundation.clone())
                .await
                .expect("test: create storage"),
        );

        let generator = InstructionGenerator::new(config, foundation, storage);
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
        assert!((config.base_latency_ms - 5.0).abs() < f64::EPSILON);
        assert!((config.per_hop_latency_ms - 2.0).abs() < f64::EPSILON);
        assert!((config.max_expected_distance - 100.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_health_no_client_returns_baseline() {
        let pos = MatrixCoordinate::new(10, 20, 30).expect("test: coord");
        let (generator, _td) = build_generator_no_client().await;
        let health = generator.estimate_node_health(&pos);

        // Without client position, should return baseline 0.8
        assert!((health - 0.8).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_health_nearby_node_scores_high() {
        let client = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let nearby = MatrixCoordinate::new(1, 1, 0).expect("test: coord");

        let (generator, _td) = build_generator_with_client(client).await;
        let health = generator.estimate_node_health(&nearby);

        assert!(health >= 0.5 && health <= 1.0);
        assert!(health > 0.9, "nearby node health should be > 0.9, got {health}");
    }

    #[tokio::test]
    async fn test_health_distant_node_scores_lower() {
        let client = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let nearby = MatrixCoordinate::new(2, 0, 0).expect("test: coord");
        let far = MatrixCoordinate::new(200, 200, 200).expect("test: coord");

        let (generator, _td) = build_generator_with_client(client).await;

        let health_near = generator.estimate_node_health(&nearby);
        let health_far = generator.estimate_node_health(&far);

        assert!(
            health_near > health_far,
            "nearby health ({health_near}) should exceed distant ({health_far})",
        );
        // Far node should be clamped at minimum 0.5
        assert!(health_far >= 0.5);
    }

    #[tokio::test]
    async fn test_latency_no_client_returns_base() {
        let pos = MatrixCoordinate::new(50, 50, 50).expect("test: coord");
        let (generator, _td) = build_generator_no_client().await;
        let latency = generator.estimate_latency(&pos);

        // No client -> base_latency_ms = 5
        assert_eq!(latency, 5);
    }

    #[tokio::test]
    async fn test_latency_increases_with_distance() {
        let client = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let near = MatrixCoordinate::new(5, 0, 0).expect("test: coord");
        let far = MatrixCoordinate::new(50, 0, 0).expect("test: coord");

        let (generator, _td) = build_generator_with_client(client).await;

        let latency_near = generator.estimate_latency(&near);
        let latency_far = generator.estimate_latency(&far);

        assert!(
            latency_far > latency_near,
            "far latency ({latency_far}) should exceed near ({latency_near})",
        );
        // near: base(5) + 5 * 2 = 15
        assert_eq!(latency_near, 15);
        // far: base(5) + 50 * 2 = 105
        assert_eq!(latency_far, 105);
    }

    #[tokio::test]
    async fn test_priority_data_shard_higher_than_parity() {
        let client = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let pos = MatrixCoordinate::new(10, 0, 0).expect("test: coord");

        let (generator, _td) = build_generator_with_client(client).await;
        let health = 0.9;

        let data_priority = generator.calculate_priority(&pos, health, true);
        let parity_priority = generator.calculate_priority(&pos, health, false);

        assert!(
            data_priority >= parity_priority,
            "data shard priority ({data_priority}) should >= parity ({parity_priority})",
        );
    }

    #[tokio::test]
    async fn test_priority_closer_node_higher() {
        let client = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let near = MatrixCoordinate::new(5, 0, 0).expect("test: coord");
        let far = MatrixCoordinate::new(80, 0, 0).expect("test: coord");

        let (generator, _td) = build_generator_with_client(client).await;

        let near_pri = generator.calculate_priority(&near, 0.9, true);
        let far_pri = generator.calculate_priority(&far, 0.9, true);

        assert!(
            near_pri > far_pri,
            "near priority ({near_pri}) should exceed far ({far_pri})",
        );
    }

    #[tokio::test]
    async fn test_priority_healthier_node_higher() {
        let client = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let pos = MatrixCoordinate::new(10, 0, 0).expect("test: coord");

        let (generator, _td) = build_generator_with_client(client).await;

        let healthy = generator.calculate_priority(&pos, 0.95, true);
        let unhealthy = generator.calculate_priority(&pos, 0.5, true);

        assert!(
            healthy >= unhealthy,
            "healthy priority ({healthy}) should >= unhealthy ({unhealthy})",
        );
    }

    #[tokio::test]
    async fn test_priority_range_valid() {
        let client = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let (generator, _td) = build_generator_with_client(client).await;

        let positions = [
            MatrixCoordinate::new(0, 0, 0).expect("test: coord"),
            MatrixCoordinate::new(50, 50, 50).expect("test: coord"),
            MatrixCoordinate::new(100, 100, 100).expect("test: coord"),
        ];

        for pos in &positions {
            let p = generator.calculate_priority(pos, 0.8, true);
            assert!(p <= 100, "priority {p} exceeds 100 for position {pos}");
        }
    }

    #[tokio::test]
    async fn test_priority_no_client_full_score() {
        let (generator, _td) = build_generator_no_client().await;
        let pos = MatrixCoordinate::new(999, 999, 999).expect("test: coord");

        // No client: distance_factor=1.0, health=0.9, data shard=1.0
        // combined = 0.40*1.0 + 0.35*0.9 + 0.25*1.0 = 0.965 -> 97
        let priority = generator.calculate_priority(&pos, 0.9, true);
        assert_eq!(priority, 97);
    }
}
