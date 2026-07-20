// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! BlockMatrix Asset Integration
//!
//! This module provides integration between TrustChain's Proof of State validation
//! and BlockMatrix's AssetId system with proof requirements.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::{StateProof, ProofValidation};

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

/// Asset validation context.
///
/// CANONICAL MODEL: proofs answer WHO (authorization) / WHAT (work hash) /
/// WHERE (location) / WHEN (time) — never a magnitude. There are no minimum
/// stake / storage / compute thresholds; admission requires the proofs to be
/// present and self-consistent, not to clear a numeric gate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetValidationContext {
    /// Asset identifier (content hash)
    pub asset_id: String,
    /// Required proof types for this asset
    pub proof_requirements: AssetProofRequirements,
}

impl AssetValidationContext {
    /// Create validation context for an asset
    pub fn new(asset_id: String, proof_requirements: AssetProofRequirements) -> Self {
        Self {
            asset_id,
            proof_requirements,
        }
    }
}

/// Validate `proof` against an asset's proof requirements.
///
/// Integrates with BlockMatrix `AssetId` proof_scope validation. Exposed to
/// callers through the `StateProofOps::validate_for_asset` extension trait
/// (the `StateProof` type itself is owned by `hypermesh_lib`).
pub fn validate_proof_for_asset(
    proof: &StateProof,
    context: &AssetValidationContext,
) -> Result<ProofValidation> {
    {
        let this = proof;
        // First, do standard validation
        let mut validation = super::validation::verify_all_proofs(this)?;

        // Check required proofs based on asset requirements
        let reqs = &context.proof_requirements;

        // Validate Space proof if required.
        //
        // CANONICAL MODEL: PoSpace is WHERE (location). Storage capacity is a
        // descriptive attribute, never a minimum-storage admission gate. When
        // space is required, require a bound location commitment (present WHERE).
        if reqs.require_space {
            if !validation.space_valid {
                return Ok(validation); // Already marked invalid
            }

            if this.space_proof.file_hash.is_empty() && this.space_proof.storage_path.is_empty() {
                validation.space_valid = false;
                validation.all_valid = false;
                validation.add_error(
                    super::validation::ProofType::Space,
                    format!(
                        "Asset {} requires a bound storage location (WHERE)",
                        context.asset_id
                    ),
                    super::validation::ErrorCode::StorageCommitmentInvalid,
                );
            }
        }

        // Validate Stake proof if required
        if reqs.require_stake {
            if !validation.stake_valid {
                return Ok(validation); // Already marked invalid
            }

            // CANONICAL MODEL: PoStake is authorization (WHO), not a magnitude.
            // Instead of a minimum-stake gate, require the identity binding to
            // be present when authorization is required.
            if this.stake_proof.stake_holder_id.is_empty() {
                validation.stake_valid = false;
                validation.all_valid = false;
                validation.add_error(
                    super::validation::ProofType::Stake,
                    format!(
                        "Asset {} requires a bound authorization identity (WHO)",
                        context.asset_id
                    ),
                    super::validation::ErrorCode::InsufficientStake,
                );
            }
        }

        // Validate Work proof if required
        if reqs.require_work {
            if !validation.work_valid {
                return Ok(validation); // Already marked invalid
            }

            // CANONICAL MODEL: PoWork is the HASH of work done, never a compute
            // magnitude (capacity is a descriptive adapter attribute). When
            // work is required, require a real (non-zero) work hash.
            if this.work_proof.work_hash == [0u8; 32] {
                validation.work_valid = false;
                validation.all_valid = false;
                validation.add_error(
                    super::validation::ProofType::Work,
                    format!(
                        "Asset {} requires a real (non-zero) work hash",
                        context.asset_id
                    ),
                    super::validation::ErrorCode::InsufficientWork,
                );
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
        validation.all_valid = (!reqs.require_space || validation.space_valid)
            && (!reqs.require_stake || validation.stake_valid)
            && (!reqs.require_work || validation.work_valid)
            && (!reqs.require_time || validation.time_valid);

        Ok(validation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof_of_state::StateProofOps;

    #[test]
    fn test_asset_validation_all_proofs_required() {
        let proof = StateProof::new_for_testing();
        let context = AssetValidationContext::new(
            "test_asset_123".to_string(),
            AssetProofRequirements::all(),
        );

        let validation = proof.validate_for_asset(&context).expect("test: validation");
        assert!(validation.all_valid);
        assert_eq!(validation.proofs_passed(), 4);
    }

    #[test]
    fn test_asset_validation_partial_requirements() {
        let proof = StateProof::new_for_testing();

        // Only require stake and space
        let context = AssetValidationContext::new(
            "test_asset_456".to_string(),
            AssetProofRequirements::custom(true, true, false, false),
        );

        let validation = proof.validate_for_asset(&context).expect("test: validation");
        assert!(validation.all_valid);
    }

    #[test]
    fn test_asset_validation_authorized_identity_passes() {
        // PoStake is authorization: a bound identity (WHO) passes, regardless
        // of any magnitude (there is none).
        let proof = StateProof::new_for_testing();

        let context = AssetValidationContext::new(
            "test_asset_789".to_string(),
            AssetProofRequirements::all(),
        );

        let validation = proof.validate_for_asset(&context).expect("test: validation");
        assert!(validation.all_valid);
        assert!(validation.stake_valid);
    }

    #[test]
    fn test_asset_validation_unauthorized_identity_fails() {
        // No bound identity => authorization is malformed => stake invalid.
        let mut proof = StateProof::new_for_testing();
        proof.stake_proof.stake_holder_id = String::new();

        let context = AssetValidationContext::new(
            "test_asset_999".to_string(),
            AssetProofRequirements::all(),
        );

        let validation = proof.validate_for_asset(&context).expect("test: validation");
        assert!(!validation.all_valid);
        assert!(!validation.stake_valid);
    }

    #[test]
    fn test_asset_validation_minimal_requirements() {
        let mut proof = StateProof::new_for_testing();
        // Intentionally break work (zero hash) and time proofs.
        proof.work_proof.work_hash = [0u8; 32];
        proof.time_proof.nonce = 0;

        // Only require space and stake (minimal)
        let context = AssetValidationContext::new(
            "test_asset_minimal".to_string(),
            AssetProofRequirements::minimal(),
        );

        let validation = proof.validate_for_asset(&context).expect("test: validation");
        // Should still pass because work and time aren't required
        assert!(validation.all_valid);
    }

    #[test]
    fn test_asset_validation_requires_bound_location_not_capacity() {
        // PoSpace answers WHERE via a bound location — never a storage-capacity
        // minimum. A proof with a bound location passes.
        let mut proof = StateProof::new_for_testing();
        proof.space_proof.storage_path = "/hypermesh/storage/node".to_string();
        proof.space_proof.file_hash = "location-commitment".to_string();

        let context = AssetValidationContext::new(
            "test_asset_full".to_string(),
            AssetProofRequirements::all(),
        );

        let validation = proof.validate_for_asset(&context).expect("test: validation");
        assert!(validation.all_valid);
    }

    #[test]
    fn test_asset_validation_missing_location_fails() {
        // A required PoSpace with no bound location (WHERE) is rejected — on
        // location absence, never on a capacity threshold.
        let mut proof = StateProof::new_for_testing();
        proof.space_proof.storage_path = String::new();
        proof.space_proof.file_hash = String::new();

        let context = AssetValidationContext::new(
            "test_asset_no_location".to_string(),
            AssetProofRequirements::custom(true, false, false, false),
        );

        let validation = proof.validate_for_asset(&context).expect("test: validation");
        assert!(!validation.space_valid);
        assert!(!validation.all_valid);
    }
}
