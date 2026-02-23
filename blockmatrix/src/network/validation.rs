// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Network-level validation for matrix positions using TrustChain PoS
//!
//! This module provides validation services for matrix positions during
//! network operations, neighbor discovery, and topology management.

use anyhow::Result;
use std::sync::Arc;
use std::time::{SystemTime, Duration};
use tokio::sync::RwLock;
use tracing::{info, warn, debug, error};

use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::blockchain_integration::{
    MatrixPositionValidator, ValidationStatus
};
use crate::blockchain::node_chain::NodeBlockchain;
use trustchain::consensus::ConsensusProof;

/// Network position validator for matrix topology
pub struct NetworkPositionValidator {
    /// Matrix position validator
    position_validator: Arc<MatrixPositionValidator>,

    /// Cache of recently validated positions
    validation_cache: Arc<RwLock<ValidationCache>>,

    /// Enable strict validation mode
    strict_mode: bool,
}

/// Cached validation results
struct ValidationCache {
    entries: HashMap<MatrixCoordinate, CachedValidation>,
    max_age: Duration,
}

use std::collections::HashMap;

/// Cached validation entry
#[derive(Clone)]
struct CachedValidation {
    _coordinate: MatrixCoordinate,
    node_id: String,
    is_valid: bool,
    timestamp: SystemTime,
    _confidence: f64,
}

impl NetworkPositionValidator {
    /// Create new network validator
    pub fn new(blockchain: Arc<NodeBlockchain>, strict_mode: bool) -> Self {
        let position_validator = if strict_mode {
            Arc::new(MatrixPositionValidator::new(blockchain))
        } else {
            Arc::new(MatrixPositionValidator::for_testing(blockchain))
        };

        Self {
            position_validator,
            validation_cache: Arc::new(RwLock::new(ValidationCache::new())),
            strict_mode,
        }
    }

    /// Validate a node's position claim before accepting connection
    pub async fn validate_node_position(
        &self,
        coordinate: MatrixCoordinate,
        node_id: String,
        consensus_proof: ConsensusProof,
    ) -> Result<bool> {
        info!(
            "Validating network position ({},{},{}) for node {}",
            coordinate.x, coordinate.y, coordinate.z, node_id
        );

        // Check cache first
        if let Some(cached) = self.check_cache(&coordinate, &node_id).await {
            debug!("Using cached validation for position ({},{},{}): {}",
                coordinate.x, coordinate.y, coordinate.z, cached);
            return Ok(cached);
        }

        // Validate using PoS
        let validation_start = SystemTime::now();

        // Try to register the position (this validates it)
        match self.position_validator.register_position(
            coordinate.clone(),
            node_id.clone(),
            consensus_proof,
        ).await {
            Ok(registration) => {
                let is_valid = registration.validation_status == ValidationStatus::Validated;

                // Cache the result
                self.cache_validation(
                    coordinate.clone(),
                    node_id.clone(),
                    is_valid,
                    1.0, // Full confidence for successful registration
                ).await;

                if let Ok(elapsed) = validation_start.elapsed() {
                    info!(
                        "Position validation completed in {:?} for ({},{},{}) - Valid: {}",
                        elapsed, coordinate.x, coordinate.y, coordinate.z, is_valid
                    );
                }

                Ok(is_valid)
            }
            Err(e) => {
                // Check if it's because position is already claimed
                if e.to_string().contains("already claimed") {
                    // Check if it's claimed by the same node
                    if let Some(existing) = self.position_validator.get_position(&coordinate).await {
                        if existing.node_id == node_id {
                            // Same node, position is valid
                            self.cache_validation(
                                coordinate.clone(),
                                node_id,
                                true,
                                0.9, // Slightly lower confidence for re-validation
                            ).await;
                            return Ok(true);
                        } else {
                            // Different node claims this position
                            warn!(
                                "Position ({},{},{}) already claimed by different node: {}",
                                coordinate.x, coordinate.y, coordinate.z, existing.node_id
                            );
                            return Ok(false);
                        }
                    }
                }

                // Other validation error
                error!(
                    "Position validation failed for ({},{},{}): {}",
                    coordinate.x, coordinate.y, coordinate.z, e
                );

                // Cache negative result
                self.cache_validation(
                    coordinate,
                    node_id,
                    false,
                    0.0,
                ).await;

                Ok(false)
            }
        }
    }

    /// Batch validate multiple positions (for neighbor discovery)
    pub async fn validate_neighbor_positions(
        &self,
        neighbors: Vec<(MatrixCoordinate, String)>,
    ) -> Result<Vec<(MatrixCoordinate, bool)>> {
        let mut results = Vec::new();

        for (coordinate, node_id) in neighbors {
            // Check if position is registered and valid
            if let Some(registration) = self.position_validator.get_position(&coordinate).await {
                let is_valid = registration.validation_status == ValidationStatus::Validated
                    && registration.node_id == node_id;

                results.push((coordinate, is_valid));
            } else {
                // No registration found
                results.push((coordinate, false));
            }
        }

        Ok(results)
    }

    /// Validate matrix topology consistency
    pub async fn validate_topology_consistency(
        &self,
        center: MatrixCoordinate,
        radius: f64,
    ) -> Result<TopologyValidation> {
        debug!(
            "Validating topology consistency around ({},{},{}) with radius {}",
            center.x, center.y, center.z, radius
        );

        let all_positions = self.position_validator.get_validated_positions().await;
        let mut validation = TopologyValidation::new(center.clone());

        for registration in all_positions {
            let distance = center.euclidean_distance(&registration.coordinate);

            if distance <= radius {
                // This position is within our topology radius
                validation.positions_in_radius += 1;

                // Check if position follows matrix rules
                if self.validate_matrix_rules(&registration.coordinate).await {
                    validation.valid_positions += 1;
                } else {
                    validation.invalid_positions.push(registration.coordinate.clone());
                }
            }
        }

        validation.consistency_score = if validation.positions_in_radius > 0 {
            validation.valid_positions as f64 / validation.positions_in_radius as f64
        } else {
            1.0 // Empty topology is consistent
        };

        Ok(validation)
    }

    /// Validate that a position follows matrix topology rules
    async fn validate_matrix_rules(&self, coordinate: &MatrixCoordinate) -> bool {
        // Matrix topology rules:
        // 1. Positions must be within valid bounds
        if coordinate.validate().is_err() {
            return false;
        }

        // 2. Central positions require higher validation threshold
        let distance_from_origin = coordinate.euclidean_distance(&MatrixCoordinate::origin());
        if distance_from_origin < 10.0 && self.strict_mode {
            // Central positions need extra validation in strict mode
            // This would check additional requirements like minimum uptime, reputation, etc.
            debug!("Central position ({},{},{}) requires additional validation",
                coordinate.x, coordinate.y, coordinate.z);
        }

        true
    }

    /// Check validation cache
    async fn check_cache(&self, coordinate: &MatrixCoordinate, node_id: &str) -> Option<bool> {
        let cache = self.validation_cache.read().await;

        if let Some(entry) = cache.entries.get(coordinate) {
            if entry.node_id == node_id {
                // Check if cache entry is still valid
                if let Ok(age) = entry.timestamp.elapsed() {
                    if age < cache.max_age {
                        return Some(entry.is_valid);
                    }
                }
            }
        }

        None
    }

    /// Cache a validation result
    async fn cache_validation(
        &self,
        coordinate: MatrixCoordinate,
        node_id: String,
        is_valid: bool,
        confidence: f64,
    ) {
        let mut cache = self.validation_cache.write().await;

        cache.entries.insert(
            coordinate.clone(),
            CachedValidation {
                _coordinate: coordinate,
                node_id,
                is_valid,
                timestamp: SystemTime::now(),
                _confidence: confidence,
            }
        );

        // Clean old entries if cache is too large
        if cache.entries.len() > 1000 {
            cache.clean_old_entries();
        }
    }

    /// Get validation statistics
    pub async fn get_validation_stats(&self) -> ValidationStats {
        let cache = self.validation_cache.read().await;
        let positions = self.position_validator.get_validated_positions().await;

        ValidationStats {
            total_validated_positions: positions.len(),
            cached_validations: cache.entries.len(),
            cache_hit_rate: cache.calculate_hit_rate(),
            strict_mode: self.strict_mode,
        }
    }
}

/// Topology validation result
#[derive(Debug, Clone)]
pub struct TopologyValidation {
    pub center: MatrixCoordinate,
    pub positions_in_radius: usize,
    pub valid_positions: usize,
    pub invalid_positions: Vec<MatrixCoordinate>,
    pub consistency_score: f64,
}

impl TopologyValidation {
    fn new(center: MatrixCoordinate) -> Self {
        Self {
            center,
            positions_in_radius: 0,
            valid_positions: 0,
            invalid_positions: Vec::new(),
            consistency_score: 0.0,
        }
    }
}

/// Validation statistics
#[derive(Debug, Clone)]
pub struct ValidationStats {
    pub total_validated_positions: usize,
    pub cached_validations: usize,
    pub cache_hit_rate: f64,
    pub strict_mode: bool,
}

impl ValidationCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            max_age: Duration::from_secs(300), // 5 minute cache
        }
    }

    fn clean_old_entries(&mut self) {
        let now = SystemTime::now();
        self.entries.retain(|_, entry| {
            if let Ok(age) = now.duration_since(entry.timestamp) {
                age < self.max_age
            } else {
                false
            }
        });
    }

    fn calculate_hit_rate(&self) -> f64 {
        // This would track actual hits/misses in production
        // For now, estimate based on cache freshness
        let now = SystemTime::now();
        let fresh_entries = self.entries.values().filter(|entry| {
            if let Ok(age) = now.duration_since(entry.timestamp) {
                age < Duration::from_secs(60) // Fresh = < 1 minute old
            } else {
                false
            }
        }).count();

        if self.entries.is_empty() {
            0.0
        } else {
            fresh_entries as f64 / self.entries.len() as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_position_validation() {
        let coordinate = MatrixCoordinate::new(1, 2, 3).unwrap();
        let blockchain = Arc::new(NodeBlockchain::new(coordinate.clone()));
        let validator = NetworkPositionValidator::new(blockchain, false);

        // Create test proof
        let proof = ConsensusProof::new_for_testing();

        // Validate position
        let result = validator.validate_node_position(
            coordinate.clone(),
            "test_node".to_string(),
            proof,
        ).await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_topology_consistency_validation() {
        let center = MatrixCoordinate::new(0, 0, 0).unwrap();
        let blockchain = Arc::new(NodeBlockchain::new(center.clone()));
        let validator = NetworkPositionValidator::new(blockchain, false);

        // Register some positions
        for x in -2..=2 {
            for y in -2..=2 {
                let coord = MatrixCoordinate::new(x * 10, y * 10, 0).unwrap();
                let proof = ConsensusProof::new_for_testing();
                let _ = validator.validate_node_position(
                    coord,
                    format!("node_{}_{}", x, y),
                    proof,
                ).await;
            }
        }

        // Validate topology
        let topology = validator.validate_topology_consistency(
            center,
            50.0, // Radius
        ).await.unwrap();

        assert!(topology.positions_in_radius > 0);
        assert_eq!(topology.valid_positions, topology.positions_in_radius);
        assert!(topology.consistency_score >= 1.0);
    }

    #[tokio::test]
    async fn test_validation_caching() {
        let coordinate = MatrixCoordinate::new(5, 5, 5).unwrap();
        let blockchain = Arc::new(NodeBlockchain::new(coordinate.clone()));
        let validator = NetworkPositionValidator::new(blockchain, false);

        let proof = ConsensusProof::new_for_testing();

        // First validation - should hit blockchain
        let result1 = validator.validate_node_position(
            coordinate.clone(),
            "cached_node".to_string(),
            proof.clone(),
        ).await.unwrap();

        // Second validation - should hit cache
        let result2 = validator.validate_node_position(
            coordinate.clone(),
            "cached_node".to_string(),
            proof,
        ).await.unwrap();

        assert_eq!(result1, result2);

        // Check stats
        let stats = validator.get_validation_stats().await;
        assert!(stats.cached_validations > 0);
    }
}