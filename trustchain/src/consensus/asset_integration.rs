// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! BlockMatrix Asset Integration
//!
//! This module provides integration between TrustChain's Proof of State validation
//! and BlockMatrix's AssetId system with proof requirements.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use super::{ConsensusProof, ProofValidation};

/// Asset proof requirements (mirrors BlockMatrix AssetId proof_scope)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetProofRequirements {
    pub require_space: bool,
    pub require_stake: bool,
    pub require_work: bool,
    pub require_time: bool,
}

impl AssetProofRequirements {
    /// All four proofs required (default for assets)
    pub fn all() -> Self {
        Self {
            require_space: true,
            require_stake: true,
            require_work: true,
            require_time: true,
        }
    }

    /// Minimal requirements (testing only)
    pub fn minimal() -> Self {
        Self {
            require_space: true,
            require_stake: true,
            require_work: false,
            require_time: false,
        }
    }

    /// Custom requirements
    pub fn custom(space: bool, stake: bool, work: bool, time: bool) -> Self {
        Self {
            require_space: space,
            require_stake: stake,
            require_work: work,
            require_time: time,
        }
    }
}

/// Asset validation context
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetValidationContext {
    /// Asset identifier (content hash)
    pub asset_id: String,
    /// Required proof types for this asset
    pub proof_requirements: AssetProofRequirements,
    /// Minimum stake amount (if stake required)
    pub min_stake: Option<u64>,
    /// Minimum storage (if space required)
    pub min_storage: Option<u64>,
    /// Minimum compute power (if work required)
    pub min_compute: Option<u64>,
}

impl AssetValidationContext {
    /// Create validation context for an asset
    pub fn new(asset_id: String, proof_requirements: AssetProofRequirements) -> Self {
        Self {
            asset_id,
            proof_requirements,
            min_stake: None,
            min_storage: None,
            min_compute: None,
        }
    }

    /// Set minimum stake requirement
    pub fn with_min_stake(mut self, min_stake: u64) -> Self {
        self.min_stake = Some(min_stake);
        self
    }

    /// Set minimum storage requirement
    pub fn with_min_storage(mut self, min_storage: u64) -> Self {
        self.min_storage = Some(min_storage);
        self
    }

    /// Set minimum compute requirement
    pub fn with_min_compute(mut self, min_compute: u64) -> Self {
        self.min_compute = Some(min_compute);
        self
    }
}

impl ConsensusProof {
    /// Validate consensus proof against asset requirements
    /// This method integrates with BlockMatrix AssetId proof_scope validation
    pub fn validate_for_asset(&self, context: &AssetValidationContext) -> Result<ProofValidation> {
        // First, do standard validation
        let mut validation = self.verify_all()?;

        // Check required proofs based on asset requirements
        let reqs = &context.proof_requirements;

        // Validate Space proof if required
        if reqs.require_space {
            if !validation.space_valid {
                return Ok(validation); // Already marked invalid
            }

            // Check minimum storage if specified
            if let Some(min_storage) = context.min_storage {
                if self.space_proof.total_storage < min_storage {
                    validation.space_valid = false;
                    validation.all_valid = false;
                    validation.add_error(
                        super::validation::ProofType::Space,
                        format!("Asset {} requires minimum storage {} bytes",
                            context.asset_id, min_storage),
                        super::validation::ErrorCode::StorageCommitmentInvalid,
                    );
                }
            }
        }

        // Validate Stake proof if required
        if reqs.require_stake {
            if !validation.stake_valid {
                return Ok(validation); // Already marked invalid
            }

            // Check minimum stake if specified
            if let Some(min_stake) = context.min_stake {
                if self.stake_proof.stake_amount < min_stake {
                    validation.stake_valid = false;
                    validation.all_valid = false;
                    validation.add_error(
                        super::validation::ProofType::Stake,
                        format!("Asset {} requires minimum stake {} tokens",
                            context.asset_id, min_stake),
                        super::validation::ErrorCode::InsufficientStake,
                    );
                }
            }
        }

        // Validate Work proof if required
        if reqs.require_work {
            if !validation.work_valid {
                return Ok(validation); // Already marked invalid
            }

            // Check minimum compute if specified
            if let Some(min_compute) = context.min_compute {
                if self.work_proof.computational_power < min_compute {
                    validation.work_valid = false;
                    validation.all_valid = false;
                    validation.add_error(
                        super::validation::ProofType::Work,
                        format!("Asset {} requires minimum compute power {}",
                            context.asset_id, min_compute),
                        super::validation::ErrorCode::InsufficientWork,
                    );
                }
            }
        }

        // Validate Time proof if required
        if reqs.require_time && !validation.time_valid {
            return Ok(validation); // Already marked invalid
        }

        // If asset doesn't require certain proofs, we don't fail on them
        // but we keep track of what was validated
        if !reqs.require_space {
            validation.space_valid = true; // Not required, so passes
        }
        if !reqs.require_stake {
            validation.stake_valid = true; // Not required, so passes
        }
        if !reqs.require_work {
            validation.work_valid = true; // Not required, so passes
        }
        if !reqs.require_time {
            validation.time_valid = true; // Not required, so passes
        }

        // Recalculate all_valid based on required proofs
        validation.all_valid =
            (!reqs.require_space || validation.space_valid) &&
            (!reqs.require_stake || validation.stake_valid) &&
            (!reqs.require_work || validation.work_valid) &&
            (!reqs.require_time || validation.time_valid);

        // Recalculate confidence score based on what was actually validated
        let mut required_count = 0;
        let mut passed_count = 0;

        if reqs.require_space {
            required_count += 1;
            if validation.space_valid { passed_count += 1; }
        }
        if reqs.require_stake {
            required_count += 1;
            if validation.stake_valid { passed_count += 1; }
        }
        if reqs.require_work {
            required_count += 1;
            if validation.work_valid { passed_count += 1; }
        }
        if reqs.require_time {
            required_count += 1;
            if validation.time_valid { passed_count += 1; }
        }

        validation.confidence_score = if required_count > 0 {
            passed_count as f64 / required_count as f64
        } else {
            0.0
        };

        Ok(validation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::proof::{WorkloadType, WorkState};

    #[test]
    fn test_asset_validation_all_proofs_required() {
        let proof = ConsensusProof::new_for_testing();
        let context = AssetValidationContext::new(
            "test_asset_123".to_string(),
            AssetProofRequirements::all(),
        );

        let validation = proof.validate_for_asset(&context).unwrap();
        assert!(validation.all_valid);
        assert_eq!(validation.confidence_score, 1.0);
    }

    #[test]
    fn test_asset_validation_partial_requirements() {
        let proof = ConsensusProof::new_for_testing();

        // Only require stake and space
        let context = AssetValidationContext::new(
            "test_asset_456".to_string(),
            AssetProofRequirements::custom(true, true, false, false),
        );

        let validation = proof.validate_for_asset(&context).unwrap();
        assert!(validation.all_valid);
        // Confidence is 100% because we passed the 2 required proofs
        assert_eq!(validation.confidence_score, 1.0);
    }

    #[test]
    fn test_asset_validation_min_stake() {
        let proof = ConsensusProof::new_for_testing();

        let context = AssetValidationContext::new(
            "test_asset_789".to_string(),
            AssetProofRequirements::all(),
        ).with_min_stake(5000); // Require 5000 tokens

        let validation = proof.validate_for_asset(&context).unwrap();
        assert!(validation.all_valid); // new_for_testing has 10000 stake
    }

    #[test]
    fn test_asset_validation_insufficient_stake() {
        let proof = ConsensusProof::new_for_testing();

        let context = AssetValidationContext::new(
            "test_asset_999".to_string(),
            AssetProofRequirements::all(),
        ).with_min_stake(1_000_000); // Require 1M tokens (too high)

        let validation = proof.validate_for_asset(&context).unwrap();
        assert!(!validation.all_valid);
        assert!(!validation.stake_valid);
    }

    #[test]
    fn test_asset_validation_minimal_requirements() {
        let mut proof = ConsensusProof::new_for_testing();
        // Intentionally break work and time proofs
        proof.work_proof.computational_power = 0;
        proof.time_proof.nonce = 0;

        // Only require space and stake (minimal)
        let context = AssetValidationContext::new(
            "test_asset_minimal".to_string(),
            AssetProofRequirements::minimal(),
        );

        let validation = proof.validate_for_asset(&context).unwrap();
        // Should still pass because work and time aren't required
        assert!(validation.all_valid);
    }

    #[test]
    fn test_asset_validation_with_all_minimums() {
        let proof = ConsensusProof::new_for_testing();

        let context = AssetValidationContext::new(
            "test_asset_full".to_string(),
            AssetProofRequirements::all(),
        )
        .with_min_stake(5000)
        .with_min_storage(10 * 1024 * 1024) // 10MB
        .with_min_compute(500);

        let validation = proof.validate_for_asset(&context).unwrap();
        assert!(validation.all_valid);
    }
}
