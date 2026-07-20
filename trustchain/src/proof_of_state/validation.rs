// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Comprehensive Proof of State Validation
//!
//! This module implements full validation of all four Proof of State proofs
//! with detailed error reporting and cryptographic verification.
//! Each proof is binary pass/fail. ALL four must pass.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

use super::proof::Proof;
use super::StateProof;

// Re-export ProofType from canonical shared lib (single source of truth)
pub use hypermesh_lib::ProofType;

/// Detailed validation result for all four proofs
/// Each proof is binary pass/fail. No confidence scores.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProofValidation {
    /// PoSpace validation result
    pub space_valid: bool,
    /// PoStake validation result
    pub stake_valid: bool,
    /// PoWork validation result
    pub work_valid: bool,
    /// PoTime validation result
    pub time_valid: bool,
    /// Overall validation (all four must be true)
    pub all_valid: bool,
    /// Detailed error messages per proof
    pub errors: Vec<ValidationError>,
    /// Validation timestamp
    pub validation_timestamp: SystemTime,
}

/// Validation error with specific proof information
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ValidationError {
    pub proof_type: ProofType,
    pub error_message: String,
    pub error_code: ErrorCode,
}

/// Error codes for validation failures
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorCode {
    /// Invalid cryptographic signature
    InvalidSignature,
    /// Insufficient stake amount
    InsufficientStake,
    /// Storage commitment mismatch
    StorageCommitmentInvalid,
    /// Time offset too large
    TimeOffsetExceeded,
    /// Computational work insufficient
    InsufficientWork,
    /// Proof expired or too old
    ProofExpired,
    /// Missing required field
    MissingField,
    /// Hash mismatch
    HashMismatch,
}

impl ProofValidation {
    /// Create a new validation result
    pub fn new(space_valid: bool, stake_valid: bool, work_valid: bool, time_valid: bool) -> Self {
        let all_valid = space_valid && stake_valid && work_valid && time_valid;

        Self {
            space_valid,
            stake_valid,
            work_valid,
            time_valid,
            all_valid,
            errors: Vec::new(),
            validation_timestamp: SystemTime::now(),
        }
    }

    /// Add validation error
    pub fn add_error(
        &mut self,
        proof_type: ProofType,
        error_message: String,
        error_code: ErrorCode,
    ) {
        self.errors.push(ValidationError {
            proof_type,
            error_message,
            error_code,
        });
    }

    /// Check if validation passed
    pub fn is_valid(&self) -> bool {
        self.all_valid
    }

    /// Validate a state proof and return the validation result.
    /// Convenience wrapper around [`verify_all_proofs`].
    pub fn validate_proof(proof: &StateProof) -> ProofValidation {
        match verify_all_proofs(proof) {
            Ok(validation) => validation,
            Err(_) => ProofValidation::new(false, false, false, false),
        }
    }

    /// Get human-readable error summary
    pub fn error_summary(&self) -> String {
        if self.errors.is_empty() {
            return "All proofs valid".to_string();
        }

        self.errors
            .iter()
            .map(|e| format!("{:?}: {}", e.proof_type, e.error_message))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Count how many of the four proofs passed
    pub fn proofs_passed(&self) -> u32 {
        let mut count = 0;
        if self.space_valid {
            count += 1;
        }
        if self.stake_valid {
            count += 1;
        }
        if self.work_valid {
            count += 1;
        }
        if self.time_valid {
            count += 1;
        }
        count
    }
}

/// Validate all four proofs of `proof` with detailed error reporting.
///
/// Exposed to callers through the `StateProofOps::verify_all` extension trait
/// (the `StateProof` type itself is owned by `hypermesh_lib`).
pub fn verify_all_proofs(proof: &StateProof) -> Result<ProofValidation> {
    {
        let this = proof;
        let mut validation = ProofValidation::new(true, true, true, true);

        // Validate PoSpace (WHERE)
        if !this.space_proof.validate() {
            validation.space_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Space,
                "Storage commitment validation failed".to_string(),
                ErrorCode::StorageCommitmentInvalid,
            );
        }

        // CANONICAL MODEL: PoSpace answers WHERE (location). Storage capacity is
        // a descriptive asset attribute — it is NEVER a proof field and NEVER
        // gates admission, so there is no `total_storage` minimum here. The
        // self-consistency check below (stored <= advertised) is an internal
        // coherence check on the proof, not a capacity threshold.

        if this.space_proof.total_size > this.space_proof.total_storage {
            validation.space_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Space,
                format!(
                    "Storage size {} exceeds capacity {}",
                    this.space_proof.total_size, this.space_proof.total_storage
                ),
                ErrorCode::StorageCommitmentInvalid,
            );
        }

        // Validate PoStake (WHO)
        if !this.stake_proof.validate() {
            validation.stake_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Stake,
                "Stake proof validation failed".to_string(),
                ErrorCode::InvalidSignature,
            );
        }

        // Additional PoStake checks: authorization requires a bound identity
        // (WHO). No magnitude — PoStake is never an amount.
        if this.stake_proof.stake_holder_id.is_empty() {
            validation.stake_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Stake,
                "Stake authorization has no bound identity".to_string(),
                ErrorCode::InsufficientStake,
            );
        }

        // Check stake age (not too old)
        if let Ok(elapsed) = this.stake_proof.stake_timestamp.elapsed() {
            if elapsed > Duration::from_secs(60 * 60 * 24 * 30) {
                // 30 days max
                validation.stake_valid = false;
                validation.all_valid = false;
                validation.add_error(
                    ProofType::Stake,
                    format!("Stake proof expired (age: {elapsed:?})"),
                    ErrorCode::ProofExpired,
                );
            }
        }

        // Validate PoWork (WHAT/HOW)
        if !this.work_proof.validate() {
            validation.work_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Work,
                "Work proof validation failed".to_string(),
                ErrorCode::InsufficientWork,
            );
        }

        // Additional PoWork checks: WHAT requires a real (non-zero) work hash.
        // No compute magnitude — capacity is a descriptive attribute.
        if this.work_proof.work_hash == [0u8; 32] {
            validation.work_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Work,
                "Work proof has zero work hash (no work performed)".to_string(),
                ErrorCode::InsufficientWork,
            );
        }

        // Validate PoTime (WHEN)
        if !this.time_proof.validate() {
            validation.time_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Time,
                "Time proof validation failed".to_string(),
                ErrorCode::HashMismatch,
            );
        }

        // Additional PoTime checks
        if this.time_proof.network_time_offset > Duration::from_secs(300) {
            validation.time_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Time,
                format!(
                    "Time offset too large: {:?} > 5 minutes",
                    this.time_proof.network_time_offset
                ),
                ErrorCode::TimeOffsetExceeded,
            );
        }

        // Recalculate all_valid from individual results
        validation.all_valid = validation.space_valid
            && validation.stake_valid
            && validation.work_valid
            && validation.time_valid;

        Ok(validation)
    }
}

/// Validate `proof` against a WHEN-freshness bound.
///
/// CANONICAL MODEL: proofs answer WHO (authorization) / WHAT (work hash) /
/// WHERE (location) / WHEN (time), never a magnitude. There is no minimum
/// storage / stake / compute gate — the only bound here is the temporal
/// freshness of the WHEN proof.
///
/// Exposed through the `StateProofOps::verify_with_requirements` extension
/// trait (the `StateProof` type is owned by `hypermesh_lib`).
pub fn verify_proof_with_requirements(
    proof: &StateProof,
    max_time_offset: Duration,
) -> Result<ProofValidation> {
    {
        let this = proof;
        let mut validation = verify_all_proofs(this)?;

        // WHO must be authorized (identity bound). No amount check.
        if this.stake_proof.stake_holder_id.is_empty() {
            validation.stake_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Stake,
                "Stake authorization has no bound identity".to_string(),
                ErrorCode::InsufficientStake,
            );
        }

        // Check time offset (WHEN freshness — a temporal bound, not a magnitude).
        if this.time_proof.network_time_offset > max_time_offset {
            validation.time_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Time,
                format!(
                    "Time offset {:?} exceeds maximum {:?}",
                    this.time_proof.network_time_offset, max_time_offset
                ),
                ErrorCode::TimeOffsetExceeded,
            );
        }

        // Recalculate all_valid
        validation.all_valid = validation.space_valid
            && validation.stake_valid
            && validation.work_valid
            && validation.time_valid;

        Ok(validation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof_of_state::StateProofOps;

    #[test]
    fn test_proof_validation_all_valid() {
        let proof = StateProof::new_for_testing();
        let validation = proof.verify_all().expect("test: expected success");

        assert!(validation.space_valid, "Space proof should be valid");
        assert!(validation.stake_valid, "Stake proof should be valid");
        assert!(validation.work_valid, "Work proof should be valid");
        assert!(validation.time_valid, "Time proof should be valid");
        assert!(validation.all_valid, "Overall validation should pass");
        assert_eq!(validation.proofs_passed(), 4);
        assert!(validation.errors.is_empty());
    }

    #[test]
    fn test_proof_validation_invalid_space() {
        // CANONICAL MODEL: PoSpace answers WHERE. A space proof is invalid when
        // it has no bound LOCATION — never because of a capacity magnitude.
        let mut proof = StateProof::new_for_testing();
        proof.space_proof.node_id = String::new();

        let validation = proof.verify_all().expect("test: expected success");

        assert!(!validation.space_valid);
        assert!(!validation.all_valid);
        assert!(validation
            .errors
            .iter()
            .any(|e| e.proof_type == ProofType::Space));
    }

    #[test]
    fn test_proof_validation_invalid_stake() {
        // PoStake is authorization: a malformed proof has NO bound identity.
        let mut proof = StateProof::new_for_testing();
        proof.stake_proof.stake_holder_id = String::new();

        let validation = proof.verify_all().expect("test: expected success");

        assert!(!validation.stake_valid);
        assert!(!validation.all_valid);
        assert!(validation
            .errors
            .iter()
            .any(|e| e.proof_type == ProofType::Stake));
    }

    #[test]
    fn test_proof_validation_with_requirements() {
        let proof = StateProof::new_for_testing();

        // Should pass with a reasonable time-freshness bound (no magnitude gate).
        let validation = proof
            .verify_with_requirements(Duration::from_secs(60)) // max_time_offset
            .expect("test: expected success");

        assert!(validation.all_valid);
    }

    #[test]
    fn test_proof_validation_fails_requirements() {
        let mut proof = StateProof::new_for_testing();
        // Push the WHEN proof out of the freshness window so the temporal bound
        // rejects it — rejection is on time freshness, never on a magnitude.
        proof.time_proof.network_time_offset = Duration::from_secs(600);

        let validation = proof
            .verify_with_requirements(Duration::from_millis(1)) // max_time_offset (too strict)
            .expect("test: expected success");

        assert!(!validation.all_valid);
        assert!(!validation.errors.is_empty());
    }

    #[test]
    fn test_zero_capacity_space_proof_is_admitted() {
        // CANONICAL MODEL: capacity is a DESCRIPTIVE asset attribute — it is
        // never a proof field that gates admission. A node advertising zero
        // spare capacity still answers WHERE, so its proof must validate.
        let mut proof = StateProof::new_for_testing();
        proof.space_proof.total_storage = 0;
        proof.space_proof.total_size = 0;

        let validation = proof.verify_all().expect("test: expected success");

        assert!(
            validation.space_valid,
            "zero capacity must not fail PoSpace — capacity is never a gate"
        );
        assert!(validation.all_valid);
    }

    #[test]
    fn test_validation_error_summary() {
        let mut proof = StateProof::new_for_testing();
        // WHO unbound (authorization missing) and WHERE unbound (no location).
        proof.stake_proof.stake_holder_id = String::new();
        proof.space_proof.node_id = String::new();

        let validation = proof.verify_all().expect("test: expected success");
        let summary = validation.error_summary();

        assert!(summary.contains("Stake") || summary.contains("Space"));
    }
}
