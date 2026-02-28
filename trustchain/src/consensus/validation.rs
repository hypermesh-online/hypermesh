// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Comprehensive Proof of State Validation
//!
//! This module implements full validation of all four Proof of State proofs
//! with detailed error reporting and cryptographic verification.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

use super::proof::Proof;
use super::ConsensusProof;

// Re-export ProofType from canonical shared lib (single source of truth)
pub use hypermesh_lib::ProofType;

/// Detailed validation result for all four proofs
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
    /// Confidence score (0.0 - 1.0)
    pub confidence_score: f64,
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
    pub fn new(
        space_valid: bool,
        stake_valid: bool,
        work_valid: bool,
        time_valid: bool,
    ) -> Self {
        let all_valid = space_valid && stake_valid && work_valid && time_valid;

        // Calculate confidence score based on which proofs passed
        let mut confidence_score = 0.0;
        if space_valid { confidence_score += 0.25; }
        if stake_valid { confidence_score += 0.25; }
        if work_valid { confidence_score += 0.25; }
        if time_valid { confidence_score += 0.25; }

        Self {
            space_valid,
            stake_valid,
            work_valid,
            time_valid,
            all_valid,
            errors: Vec::new(),
            validation_timestamp: SystemTime::now(),
            confidence_score,
        }
    }

    /// Add validation error
    pub fn add_error(&mut self, proof_type: ProofType, error_message: String, error_code: ErrorCode) {
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

    /// Get human-readable error summary
    pub fn error_summary(&self) -> String {
        if self.errors.is_empty() {
            return "All proofs valid".to_string();
        }

        self.errors.iter()
            .map(|e| format!("{:?}: {}", e.proof_type, e.error_message))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl ConsensusProof {
    /// Validate all four proofs with detailed error reporting
    pub fn verify_all(&self) -> Result<ProofValidation> {
        let mut validation = ProofValidation::new(true, true, true, true);

        // Validate PoSpace (WHERE)
        if !self.space_proof.validate() {
            validation.space_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Space,
                "Storage commitment validation failed".to_string(),
                ErrorCode::StorageCommitmentInvalid,
            );
        }

        // Additional PoSpace checks
        if self.space_proof.total_storage == 0 {
            validation.space_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Space,
                "Total storage is zero".to_string(),
                ErrorCode::MissingField,
            );
        }

        if self.space_proof.total_size > self.space_proof.total_storage {
            validation.space_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Space,
                format!("Storage size {} exceeds capacity {}",
                    self.space_proof.total_size,
                    self.space_proof.total_storage),
                ErrorCode::StorageCommitmentInvalid,
            );
        }

        // Validate PoStake (WHO)
        if !self.stake_proof.validate() {
            validation.stake_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Stake,
                "Stake proof validation failed".to_string(),
                ErrorCode::InvalidSignature,
            );
        }

        // Additional PoStake checks
        if self.stake_proof.stake_amount == 0 {
            validation.stake_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Stake,
                "Stake amount is zero".to_string(),
                ErrorCode::InsufficientStake,
            );
        }

        // Check stake age (not too old)
        if let Ok(elapsed) = self.stake_proof.stake_timestamp.elapsed() {
            if elapsed > Duration::from_secs(60 * 60 * 24 * 30) { // 30 days max
                validation.stake_valid = false;
                validation.all_valid = false;
                validation.add_error(
                    ProofType::Stake,
                    format!("Stake proof expired (age: {:?})", elapsed),
                    ErrorCode::ProofExpired,
                );
            }
        }

        // Validate PoWork (WHAT/HOW)
        if !self.work_proof.validate() {
            validation.work_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Work,
                "Work proof validation failed".to_string(),
                ErrorCode::InsufficientWork,
            );
        }

        // Additional PoWork checks
        if self.work_proof.computational_power == 0 {
            validation.work_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Work,
                "Computational power is zero".to_string(),
                ErrorCode::InsufficientWork,
            );
        }

        // Validate PoTime (WHEN)
        if !self.time_proof.validate() {
            validation.time_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Time,
                "Time proof validation failed".to_string(),
                ErrorCode::HashMismatch,
            );
        }

        // Additional PoTime checks
        if self.time_proof.network_time_offset > Duration::from_secs(300) {
            validation.time_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Time,
                format!("Time offset too large: {:?} > 5 minutes",
                    self.time_proof.network_time_offset),
                ErrorCode::TimeOffsetExceeded,
            );
        }

        // Recalculate confidence score
        validation.confidence_score = 0.0;
        if validation.space_valid { validation.confidence_score += 0.25; }
        if validation.stake_valid { validation.confidence_score += 0.25; }
        if validation.work_valid { validation.confidence_score += 0.25; }
        if validation.time_valid { validation.confidence_score += 0.25; }

        Ok(validation)
    }

    /// Validate with specific minimum requirements
    pub fn verify_with_requirements(
        &self,
        min_stake: u64,
        max_time_offset: Duration,
        min_storage: u64,
        min_compute: u64,
    ) -> Result<ProofValidation> {
        let mut validation = self.verify_all()?;

        // Check minimum stake
        if self.stake_proof.stake_amount < min_stake {
            validation.stake_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Stake,
                format!("Stake {} below minimum {}",
                    self.stake_proof.stake_amount, min_stake),
                ErrorCode::InsufficientStake,
            );
        }

        // Check time offset
        if self.time_proof.network_time_offset > max_time_offset {
            validation.time_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Time,
                format!("Time offset {:?} exceeds maximum {:?}",
                    self.time_proof.network_time_offset, max_time_offset),
                ErrorCode::TimeOffsetExceeded,
            );
        }

        // Check minimum storage
        if self.space_proof.total_storage < min_storage {
            validation.space_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Space,
                format!("Storage {} below minimum {}",
                    self.space_proof.total_storage, min_storage),
                ErrorCode::StorageCommitmentInvalid,
            );
        }

        // Check minimum compute
        if self.work_proof.computational_power < min_compute {
            validation.work_valid = false;
            validation.all_valid = false;
            validation.add_error(
                ProofType::Work,
                format!("Compute power {} below minimum {}",
                    self.work_proof.computational_power, min_compute),
                ErrorCode::InsufficientWork,
            );
        }

        // Recalculate confidence score
        validation.confidence_score = 0.0;
        if validation.space_valid { validation.confidence_score += 0.25; }
        if validation.stake_valid { validation.confidence_score += 0.25; }
        if validation.work_valid { validation.confidence_score += 0.25; }
        if validation.time_valid { validation.confidence_score += 0.25; }

        Ok(validation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    

    #[test]
    fn test_proof_validation_all_valid() {
        let proof = ConsensusProof::new_for_testing();
        let validation = proof.verify_all().unwrap();

        assert!(validation.space_valid, "Space proof should be valid");
        assert!(validation.stake_valid, "Stake proof should be valid");
        assert!(validation.work_valid, "Work proof should be valid");
        assert!(validation.time_valid, "Time proof should be valid");
        assert!(validation.all_valid, "Overall validation should pass");
        assert_eq!(validation.confidence_score, 1.0);
        assert!(validation.errors.is_empty());
    }

    #[test]
    fn test_proof_validation_invalid_space() {
        let mut proof = ConsensusProof::new_for_testing();
        proof.space_proof.total_storage = 0; // Invalid

        let validation = proof.verify_all().unwrap();

        assert!(!validation.space_valid);
        assert!(!validation.all_valid);
        assert!(validation.errors.iter().any(|e| e.proof_type == ProofType::Space));
    }

    #[test]
    fn test_proof_validation_invalid_stake() {
        let mut proof = ConsensusProof::new_for_testing();
        proof.stake_proof.stake_amount = 0; // Invalid

        let validation = proof.verify_all().unwrap();

        assert!(!validation.stake_valid);
        assert!(!validation.all_valid);
        assert!(validation.errors.iter().any(|e| e.proof_type == ProofType::Stake));
    }

    #[test]
    fn test_proof_validation_with_requirements() {
        let proof = ConsensusProof::new_for_testing();

        // Should pass with reasonable requirements
        let validation = proof.verify_with_requirements(
            5000,                          // min_stake
            Duration::from_secs(60),       // max_time_offset
            10 * 1024 * 1024,             // min_storage (10MB)
            100,                           // min_compute
        ).unwrap();

        assert!(validation.all_valid);
    }

    #[test]
    fn test_proof_validation_fails_requirements() {
        let proof = ConsensusProof::new_for_testing();

        // Should fail with excessive requirements
        let validation = proof.verify_with_requirements(
            1_000_000,                     // min_stake (too high)
            Duration::from_millis(1),      // max_time_offset (too strict)
            1024 * 1024 * 1024 * 1024,    // min_storage (1TB - too high)
            1_000_000,                     // min_compute (too high)
        ).unwrap();

        assert!(!validation.all_valid);
        assert!(!validation.errors.is_empty());
    }

    #[test]
    fn test_validation_error_summary() {
        let mut proof = ConsensusProof::new_for_testing();
        proof.stake_proof.stake_amount = 0;
        proof.space_proof.total_storage = 0;

        let validation = proof.verify_all().unwrap();
        let summary = validation.error_summary();

        assert!(summary.contains("Stake") || summary.contains("Space"));
    }
}
