// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Blockchain Integration for Matrix Position Registration
//!
//! This module implements the integration between Matrix Foundation and TrustChain
//! for validating matrix position claims using Proof of State validation.
//!
//! Core Functionality:
//! - Register matrix positions on blockchain as assets
//! - Validate position claims using all 4 PoS proofs
//! - Integrate with TrustChain certificate hierarchy for neighbor trust
//! - Ensure Proof of State validation before accepting positions

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::blockchain::node_chain::NodeBlockchain;
use crate::matrix::coordinate::MatrixCoordinate;
use trustchain::proof_of_state::validation::{ErrorCode, ProofType, ProofValidation};
use trustchain::proof_of_state::{
    StateProof, StateRequirements, SpaceProof, StakeProof, TimeProof, WorkProof,
};

/// Matrix position registration on blockchain
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatrixPositionRegistration {
    /// The matrix coordinate being claimed
    pub coordinate: MatrixCoordinate,

    /// Node ID claiming this position
    pub node_id: String,

    /// State proof for this position claim
    pub state_proof: Vec<u8>,

    /// Registration timestamp
    pub timestamp: SystemTime,

    /// Block hash where this was registered
    pub block_hash: Option<String>,

    /// Validation status
    pub validation_status: ValidationStatus,
}

/// Validation status for position claims
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ValidationStatus {
    /// Pending validation
    Pending,
    /// Validated with all 4 proofs
    Validated,
    /// Rejected due to invalid proofs
    Rejected(String),
    /// Conflicted - another node claims this position
    Conflicted(String),
}

/// Matrix position validator using TrustChain PoS
pub struct MatrixPositionValidator {
    /// Blockchain instance for registration
    blockchain: Arc<NodeBlockchain>,

    /// Registered positions (coordinate -> registration)
    positions: Arc<RwLock<HashMap<MatrixCoordinate, MatrixPositionRegistration>>>,

    /// State proof requirements for matrix positions
    requirements: StateRequirements,

    /// Production mode: require the storage commitment to reference the claimed
    /// matrix position (a WHERE/location binding, not a capacity magnitude).
    production_mode: bool,

    /// Enable verbose logging
    verbose: bool,
}

use std::collections::HashMap;
use trustchain::proof_of_state::StateProofOps;

impl MatrixPositionValidator {
    /// Create new validator with production requirements
    pub fn new(blockchain: Arc<NodeBlockchain>) -> Self {
        Self {
            blockchain,
            positions: Arc::new(RwLock::new(HashMap::new())),
            requirements: production_matrix_requirements(),
            production_mode: true,
            verbose: false,
        }
    }

    /// Create validator for testing with relaxed requirements
    pub fn for_testing(blockchain: Arc<NodeBlockchain>) -> Self {
        Self {
            blockchain,
            positions: Arc::new(RwLock::new(HashMap::new())),
            requirements: StateRequirements::localhost_testing(),
            production_mode: false,
            verbose: true,
        }
    }

    /// Register a matrix position claim on blockchain
    pub async fn register_position(
        &self,
        coordinate: MatrixCoordinate,
        node_id: String,
        state_proof: StateProof,
    ) -> Result<MatrixPositionRegistration> {
        info!(
            "Registering matrix position ({},{},{}) for node {}",
            coordinate.x, coordinate.y, coordinate.z, node_id
        );

        // Check if position is already claimed
        let positions = self.positions.read().await;
        if let Some(existing) = positions.get(&coordinate) {
            if existing.validation_status == ValidationStatus::Validated {
                return Err(anyhow!(
                    "Position ({},{},{}) already claimed by node {}",
                    coordinate.x,
                    coordinate.y,
                    coordinate.z,
                    existing.node_id
                ));
            }
        }
        drop(positions);

        // Validate the state proof
        let validation_result = self
            .validate_position_claim(&coordinate, &node_id, &state_proof)
            .await?;

        if !validation_result.is_valid() {
            return Err(anyhow!(
                "Position claim validation failed: {}",
                validation_result.error_summary()
            ));
        }

        // Serialize state proof
        let proof_bytes = state_proof.to_bytes()?;

        // Create registration
        let mut registration = MatrixPositionRegistration {
            coordinate,
            node_id: node_id.clone(),
            state_proof: proof_bytes.clone(),
            timestamp: SystemTime::now(),
            block_hash: None,
            validation_status: ValidationStatus::Pending,
        };

        // Register on blockchain
        let registration_data = serde_json::json!({
            "type": "matrix_position_registration",
            "coordinate": {
                "x": coordinate.x,
                "y": coordinate.y,
                "z": coordinate.z,
            },
            "node_id": node_id,
            "state_proof": hex::encode(&proof_bytes),
            "timestamp": registration.timestamp.duration_since(SystemTime::UNIX_EPOCH)?.as_secs(),
        });

        let block = self
            .blockchain
            .add_block_with_data(serde_json::to_vec(&registration_data)?, &state_proof)
            .await
            .map_err(|e| anyhow!("Failed to add block: {e}"))?;

        registration.block_hash = Some(block.hash.clone());
        registration.validation_status = ValidationStatus::Validated;

        // Store validated registration
        let mut positions = self.positions.write().await;
        positions.insert(coordinate, registration.clone());

        info!(
            "Successfully registered position ({},{},{}) for node {} in block {}",
            coordinate.x, coordinate.y, coordinate.z, node_id, block.hash
        );

        Ok(registration)
    }

    /// Validate a matrix position claim using PoS
    async fn validate_position_claim(
        &self,
        coordinate: &MatrixCoordinate,
        node_id: &str,
        state_proof: &StateProof,
    ) -> Result<ProofValidation> {
        debug!(
            "Validating position claim for ({},{},{}) by node {}",
            coordinate.x, coordinate.y, coordinate.z, node_id
        );

        // Validate all 4 proofs
        let mut validation = state_proof.verify_all()?;

        // Additional matrix-specific validations

        // 1. PoSpace (WHERE) - Must have storage at claimed position
        if !self
            .validate_space_for_position(coordinate, &state_proof.space_proof)
            .await
        {
            validation.space_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Space,
                format!(
                    "Storage proof invalid for position ({},{},{})",
                    coordinate.x, coordinate.y, coordinate.z
                ),
                ErrorCode::StorageCommitmentInvalid,
            );
        }

        // 2. PoStake (WHO) - Must have sufficient stake for position
        if !self
            .validate_stake_for_position(coordinate, &state_proof.stake_proof)
            .await
        {
            validation.stake_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Stake,
                format!(
                    "Insufficient stake for position ({},{},{})",
                    coordinate.x, coordinate.y, coordinate.z
                ),
                ErrorCode::InsufficientStake,
            );
        }

        // 3. PoWork (WHAT) - Must prove computational work for position
        if !self
            .validate_work_for_position(coordinate, &state_proof.work_proof)
            .await
        {
            validation.work_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Work,
                format!(
                    "Work proof invalid for position ({},{},{})",
                    coordinate.x, coordinate.y, coordinate.z
                ),
                ErrorCode::InsufficientWork,
            );
        }

        // 4. PoTime (WHEN) - Must be temporally valid
        if !self
            .validate_time_for_position(coordinate, &state_proof.time_proof)
            .await
        {
            validation.time_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Time,
                format!(
                    "Time proof invalid for position ({},{},{})",
                    coordinate.x, coordinate.y, coordinate.z
                ),
                ErrorCode::TimeOffsetExceeded,
            );
        }

        // Check against requirements
        let requirements_met = state_proof.validate_with_requirements(&self.requirements);
        if !requirements_met {
            validation.all_valid = false;
            if self.verbose {
                warn!(
                    "Position claim failed requirements check for ({},{},{})",
                    coordinate.x, coordinate.y, coordinate.z
                );
            }
        }

        // Recalculate all_valid (binary: all four must pass)
        validation.all_valid = validation.space_valid
            && validation.stake_valid
            && validation.work_valid
            && validation.time_valid;

        Ok(validation)
    }

    /// Validate space proof for matrix position (PoSpace = WHERE / location).
    ///
    /// CANONICAL MODEL: PoSpace answers WHERE via a location commitment bound to
    /// the claimed matrix position — never a storage-capacity magnitude. A claim
    /// requires a present commitment that references the position, not a minimum
    /// byte count.
    async fn validate_space_for_position(
        &self,
        coordinate: &MatrixCoordinate,
        space_proof: &SpaceProof,
    ) -> bool {
        // WHERE: a location commitment must be present (bound node + hash).
        if space_proof.node_id.is_empty() || space_proof.file_hash.is_empty() {
            if self.verbose {
                warn!(
                    "No storage location commitment for position ({},{},{})",
                    coordinate.x, coordinate.y, coordinate.z
                );
            }
            return false;
        }

        // In production, require the commitment to reference the claimed
        // position (a location binding, not a capacity threshold).
        if self.production_mode {
            let position_hash = self.hash_position(coordinate);
            if !space_proof.file_hash.contains(&position_hash[0..8]) {
                if self.verbose {
                    warn!(
                        "Storage commitment doesn't include position hash for ({},{},{})",
                        coordinate.x, coordinate.y, coordinate.z
                    );
                }
                return false;
            }
        }

        true
    }

    /// Validate stake proof for matrix position
    async fn validate_stake_for_position(
        &self,
        coordinate: &MatrixCoordinate,
        stake_proof: &StakeProof,
    ) -> bool {
        // CANONICAL MODEL: PoStake is authorization (WHO) — a position claim
        // requires a bound identity, never a stake magnitude. Position
        // desirability does not raise a stake "price".
        if stake_proof.stake_holder_id.is_empty() {
            if self.verbose {
                warn!(
                    "No authorizing identity for position claim ({},{},{})",
                    coordinate.x, coordinate.y, coordinate.z,
                );
            }
            return false;
        }

        // Validate stake age (not too old for position claim)
        if let Ok(elapsed) = stake_proof.stake_timestamp.elapsed() {
            if elapsed > Duration::from_secs(60 * 60 * 24) {
                // 24 hours max for position claims
                if self.verbose {
                    warn!(
                        "Stake too old for position claim at ({},{},{}): {:?}",
                        coordinate.x, coordinate.y, coordinate.z, elapsed
                    );
                }
                return false;
            }
        }

        true
    }

    /// Validate work proof for matrix position
    async fn validate_work_for_position(
        &self,
        coordinate: &MatrixCoordinate,
        work_proof: &WorkProof,
    ) -> bool {
        // CANONICAL MODEL: PoWork is the HASH of work done (WHAT), never a
        // capacity magnitude. A position claim requires that work was actually
        // hashed (non-zero work hash) — capacity is descriptive, never gated.
        if work_proof.work_hash == [0u8; 32] {
            if self.verbose {
                warn!(
                    "No work hash for position claim ({},{},{})",
                    coordinate.x, coordinate.y, coordinate.z,
                );
            }
            return false;
        }

        // For testing, allow workload_id without position hash
        // In production, require workload_id to reference the claimed position
        // (a location binding on the work, not a compute magnitude).
        if self.production_mode {
            let position_hash = self.hash_position(coordinate);
            if !work_proof.workload_id.contains(&position_hash[0..8]) {
                if self.verbose {
                    warn!(
                        "Work proof doesn't include position reference for ({},{},{})",
                        coordinate.x, coordinate.y, coordinate.z
                    );
                }
                return false;
            }
        }

        true
    }

    /// Validate time proof for matrix position
    async fn validate_time_for_position(
        &self,
        coordinate: &MatrixCoordinate,
        time_proof: &TimeProof,
    ) -> bool {
        // Time offset must be reasonable for position claims
        if time_proof.network_time_offset > Duration::from_secs(60) {
            // 1 minute max for positions
            if self.verbose {
                warn!(
                    "Time offset too large for position ({},{},{}): {:?}",
                    coordinate.x, coordinate.y, coordinate.z, time_proof.network_time_offset
                );
            }
            return false;
        }

        // Validate nonce is sufficiently large for position (simulating VDF)
        let expected_nonce_min = self.calculate_vdf_delay(coordinate);
        if time_proof.nonce < expected_nonce_min {
            if self.verbose {
                warn!(
                    "Time proof nonce insufficient for position ({},{},{}): {} < {}",
                    coordinate.x, coordinate.y, coordinate.z, time_proof.nonce, expected_nonce_min
                );
            }
            return false;
        }

        true
    }

    /// Calculate VDF delay for a matrix position

    fn calculate_vdf_delay(&self, coordinate: &MatrixCoordinate) -> u64 {
        // Central positions require longer VDF
        let distance_from_origin = coordinate.euclidean_distance(&MatrixCoordinate::origin());

        if distance_from_origin < 10.0 {
            1000000 // Long VDF for central positions
        } else if distance_from_origin < 100.0 {
            100000 // Medium VDF for mid-range
        } else {
            10000 // Short VDF for edge positions
        }
    }

    /// Hash a matrix position for inclusion in proofs
    fn hash_position(&self, coordinate: &MatrixCoordinate) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&coordinate.x.to_le_bytes());
        hasher.update(&coordinate.y.to_le_bytes());
        hasher.update(&coordinate.z.to_le_bytes());
        hex::encode(hasher.finalize().as_bytes())
    }

    /// Get a registered position
    pub async fn get_position(
        &self,
        coordinate: &MatrixCoordinate,
    ) -> Option<MatrixPositionRegistration> {
        self.positions.read().await.get(coordinate).cloned()
    }

    /// Get all validated positions
    pub async fn get_validated_positions(&self) -> Vec<MatrixPositionRegistration> {
        self.positions
            .read()
            .await
            .values()
            .filter(|r| r.validation_status == ValidationStatus::Validated)
            .cloned()
            .collect()
    }

    /// Verify neighbor positions using TrustChain certificates
    pub async fn verify_neighbor_positions(
        &self,
        _center: &MatrixCoordinate,
        neighbors: Vec<MatrixCoordinate>,
    ) -> Result<Vec<(MatrixCoordinate, bool)>> {
        let mut results = Vec::new();
        let positions = self.positions.read().await;

        for neighbor in neighbors {
            // Check if neighbor has registered position
            if let Some(registration) = positions.get(&neighbor) {
                // Verify the registration is still valid
                let valid = registration.validation_status == ValidationStatus::Validated;
                results.push((neighbor, valid));

                if self.verbose {
                    debug!(
                        "Neighbor at ({},{},{}) validation: {}",
                        neighbor.x, neighbor.y, neighbor.z, valid
                    );
                }
            } else {
                // No registration found
                results.push((neighbor, false));
                if self.verbose {
                    debug!(
                        "Neighbor at ({},{},{}) has no registration",
                        neighbor.x, neighbor.y, neighbor.z
                    );
                }
            }
        }

        Ok(results)
    }
}

/// Production requirements for matrix position claims.
///
/// CANONICAL MODEL: proofs answer WHO/WHAT/WHERE/WHEN, never a magnitude. The
/// only quantitative bound is the WHEN-freshness (`max_time_offset`); position
/// binding (WHERE) is enforced by the validator's `production_mode` flag.
fn production_matrix_requirements() -> StateRequirements {
    StateRequirements {
        max_time_offset: Duration::from_secs(60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_matrix_position_registration() {
        // Create blockchain for testing
        let coordinate = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
        let blockchain = Arc::new(NodeBlockchain::new(coordinate));

        // Create validator
        let validator = MatrixPositionValidator::for_testing(blockchain);

        // Create test state proof
        let state_proof = StateProof::new_for_testing();

        // Register position
        let registration = validator
            .register_position(coordinate, "test_node_001".to_string(), state_proof)
            .await;

        assert!(registration.is_ok());
        let reg = registration.expect("test: expected success");
        assert_eq!(reg.coordinate, coordinate);
        assert_eq!(reg.validation_status, ValidationStatus::Validated);
        assert!(reg.block_hash.is_some());
    }

    #[tokio::test]
    async fn test_duplicate_position_rejection() {
        let coordinate = MatrixCoordinate::new(5, 5, 5).expect("test: valid coordinate");
        let blockchain = Arc::new(NodeBlockchain::new(coordinate));
        let validator = MatrixPositionValidator::for_testing(blockchain);

        // Register first claim
        let proof1 = StateProof::new_for_testing();
        let result1 = validator
            .register_position(coordinate, "node_001".to_string(), proof1)
            .await;
        assert!(result1.is_ok());

        // Try to register same position
        let proof2 = StateProof::new_for_testing();
        let result2 = validator
            .register_position(coordinate, "node_002".to_string(), proof2)
            .await;

        assert!(result2.is_err());
        assert!(result2.unwrap_err().to_string().contains("already claimed"));
    }

    #[tokio::test]
    async fn test_neighbor_verification() {
        let center = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let blockchain = Arc::new(NodeBlockchain::new(center));
        let validator = MatrixPositionValidator::for_testing(blockchain);

        // Register some neighbors
        let neighbors = vec![
            MatrixCoordinate::new(1, 0, 0).expect("test: valid coordinate"),
            MatrixCoordinate::new(0, 1, 0).expect("test: valid coordinate"),
            MatrixCoordinate::new(0, 0, 1).expect("test: valid coordinate"),
        ];

        for (i, neighbor) in neighbors.iter().enumerate() {
            let proof = StateProof::new_for_testing();
            let _ = validator
                .register_position(*neighbor, format!("neighbor_{i}"), proof)
                .await;
        }

        // Verify neighbors
        let verification = validator
            .verify_neighbor_positions(&center, neighbors.clone())
            .await
            .expect("test: expected success");

        assert_eq!(verification.len(), 3);
        for (coord, valid) in verification {
            assert!(
                valid,
                "Neighbor at ({},{},{}) should be valid",
                coord.x, coord.y, coord.z
            );
        }
    }
}
