//! Consensus Validation Logic
//!
//! Implements validation for the Four-Proof Consensus system

use anyhow::{Result, anyhow};
use std::time::{Duration, SystemTime};
use tracing::{info, debug, warn};

use super::types::*;

/// Consensus validator
pub struct ConsensusValidator {
    /// Minimum stake required
    min_stake: u64,

    /// Maximum time drift allowed
    max_time_drift: Duration,

    /// Minimum difficulty
    min_difficulty: u64,
}

impl ConsensusValidator {
    /// Create new validator
    pub fn new(min_stake: u64, max_time_drift: Duration, min_difficulty: u64) -> Self {
        Self {
            min_stake,
            max_time_drift,
            min_difficulty,
        }
    }

    /// Validate a consensus proof
    pub fn validate(&self, proof: &ConsensusProof) -> ValidationResults {
        debug!("Validating consensus proof for round {}", proof.round);

        let space_valid = self.validate_space(&proof.space_proof);
        let stake_valid = self.validate_stake(&proof.stake_proof);
        let work_valid = self.validate_work(&proof.work_proof);
        let time_valid = self.validate_time(&proof.time_proof);

        let space_score = if space_valid { 1.0 } else { 0.0 };
        let stake_score = self.calculate_stake_score(&proof.stake_proof);
        let work_score = self.calculate_work_score(&proof.work_proof);
        let time_score = self.calculate_time_score(&proof.time_proof);

        let consensus_score = (space_score + stake_score + work_score + time_score) / 4.0;
        let is_valid = space_valid && stake_valid && work_valid && time_valid;

        let mut errors = Vec::new();
        if !space_valid {
            errors.push("Invalid space proof".to_string());
        }
        if !stake_valid {
            errors.push("Invalid stake proof".to_string());
        }
        if !work_valid {
            errors.push("Invalid work proof".to_string());
        }
        if !time_valid {
            errors.push("Invalid time proof".to_string());
        }

        ValidationResults {
            is_valid,
            space_valid,
            stake_valid,
            work_valid,
            time_valid,
            space_score,
            stake_score,
            work_score,
            time_score,
            consensus_score,
            errors,
        }
    }

    /// Validate space proof
    fn validate_space(&self, proof: &SpaceProof) -> bool {
        // Check storage commitment
        if proof.storage_commitment.is_empty() {
            return false;
        }

        // Check replication factor
        if proof.replication_factor < 1 {
            return false;
        }

        // Check availability proof
        if proof.availability_proof.is_empty() {
            return false;
        }

        true
    }

    /// Validate stake proof
    fn validate_stake(&self, proof: &StakeProof) -> bool {
        // Check minimum stake
        if proof.stake_amount < self.min_stake {
            return false;
        }

        // Check stake lock proof
        if proof.lock_proof.is_empty() {
            return false;
        }

        // Validate reputation score
        if proof.reputation_score < 0.0 || proof.reputation_score > 1.0 {
            return false;
        }

        true
    }

    /// Validate work proof
    fn validate_work(&self, proof: &WorkProof) -> bool {
        // Check difficulty
        if proof.difficulty < self.min_difficulty {
            return false;
        }

        // Check result hash
        if proof.result_hash.is_empty() {
            return false;
        }

        // Verify checkpoints exist
        if proof.checkpoints.is_empty() {
            return false;
        }

        true
    }

    /// Validate time proof
    fn validate_time(&self, proof: &TimeProof) -> bool {
        // Check time drift
        let now = SystemTime::now();
        let drift = now.duration_since(proof.timestamp)
            .or_else(|_| proof.timestamp.duration_since(now));

        if let Ok(drift) = drift {
            if drift > self.max_time_drift {
                return false;
            }
        } else {
            return false;
        }

        // Check time beacon
        if proof.time_beacon.is_empty() {
            return false;
        }

        // Check previous hash
        if proof.previous_hash.is_empty() && proof.block_height > 0 {
            return false;
        }

        true
    }

    /// Calculate stake score
    fn calculate_stake_score(&self, proof: &StakeProof) -> f64 {
        let stake_ratio = (proof.stake_amount as f64) / (self.min_stake as f64);
        let stake_score = stake_ratio.min(10.0) / 10.0; // Cap at 10x min stake

        let reputation_weight = 0.3;
        let stake_weight = 0.7;

        stake_score * stake_weight + proof.reputation_score * reputation_weight
    }

    /// Calculate work score
    fn calculate_work_score(&self, proof: &WorkProof) -> f64 {
        let difficulty_ratio = (proof.difficulty as f64) / (self.min_difficulty as f64);
        let difficulty_score = difficulty_ratio.min(10.0) / 10.0;

        // Factor in resource usage
        let resource_score = self.calculate_resource_score(&proof.resource_metrics);

        difficulty_score * 0.6 + resource_score * 0.4
    }

    /// Calculate resource score
    fn calculate_resource_score(&self, metrics: &ResourceMetrics) -> f64 {
        // Simple scoring based on resource usage
        let cpu_score = (metrics.cpu_cycles as f64 / 1_000_000_000.0).min(1.0);
        let memory_score = (metrics.memory_usage_mb as f64 / 1024.0).min(1.0);
        let io_score = (metrics.io_operations as f64 / 10_000.0).min(1.0);
        let network_score = (metrics.network_bytes as f64 / 1_000_000_000.0).min(1.0);

        (cpu_score + memory_score + io_score + network_score) / 4.0
    }

    /// Calculate time score
    fn calculate_time_score(&self, proof: &TimeProof) -> f64 {
        // Score based on time weight and network consensus
        let time_weight_score = proof.time_weight.min(1.0);

        // Check network time deviation
        let deviation_score = if proof.network_time.deviation_ms < 100.0 {
            1.0
        } else if proof.network_time.deviation_ms < 1000.0 {
            0.8
        } else {
            0.5
        };

        time_weight_score * 0.5 + deviation_score * 0.5
    }
}

/// Batch validator for multiple proofs
pub struct BatchValidator {
    validator: ConsensusValidator,
}

impl BatchValidator {
    /// Create new batch validator
    pub fn new(validator: ConsensusValidator) -> Self {
        Self { validator }
    }

    /// Validate batch of proofs
    pub fn validate_batch(&self, proofs: &[ConsensusProof]) -> Vec<ValidationResults> {
        proofs.iter()
            .map(|proof| self.validator.validate(proof))
            .collect()
    }

    /// Get batch validation summary
    pub fn get_summary(&self, results: &[ValidationResults]) -> BatchSummary {
        let total = results.len();
        let valid = results.iter().filter(|r| r.is_valid).count();
        let invalid = total - valid;

        let avg_score = if total > 0 {
            results.iter().map(|r| r.consensus_score).sum::<f64>() / total as f64
        } else {
            0.0
        };

        BatchSummary {
            total_proofs: total,
            valid_proofs: valid,
            invalid_proofs: invalid,
            average_score: avg_score,
        }
    }
}

/// Batch validation summary
#[derive(Debug, Clone)]
pub struct BatchSummary {
    pub total_proofs: usize,
    pub valid_proofs: usize,
    pub invalid_proofs: usize,
    pub average_score: f64,
}

/// Challenge validator for disputed proofs
pub struct ChallengeValidator {
    validator: ConsensusValidator,
}

impl ChallengeValidator {
    /// Create new challenge validator
    pub fn new(validator: ConsensusValidator) -> Self {
        Self { validator }
    }

    /// Validate a challenge against a proof
    pub fn validate_challenge(
        &self,
        original: &ConsensusProof,
        challenge: &ConsensusProof,
    ) -> ChallengeResult {
        let original_result = self.validator.validate(original);
        let challenge_result = self.validator.validate(challenge);

        let winner = if challenge_result.consensus_score > original_result.consensus_score {
            ChallengeWinner::Challenger
        } else {
            ChallengeWinner::Original
        };

        ChallengeResult {
            original_score: original_result.consensus_score,
            challenge_score: challenge_result.consensus_score,
            winner,
            reason: self.determine_reason(&original_result, &challenge_result),
        }
    }

    /// Determine challenge reason
    fn determine_reason(
        &self,
        original: &ValidationResults,
        challenge: &ValidationResults,
    ) -> String {
        if !original.space_valid && challenge.space_valid {
            "Better space proof".to_string()
        } else if !original.stake_valid && challenge.stake_valid {
            "Better stake proof".to_string()
        } else if !original.work_valid && challenge.work_valid {
            "Better work proof".to_string()
        } else if !original.time_valid && challenge.time_valid {
            "Better time proof".to_string()
        } else {
            "Higher overall consensus score".to_string()
        }
    }
}

/// Challenge result
#[derive(Debug, Clone)]
pub struct ChallengeResult {
    pub original_score: f64,
    pub challenge_score: f64,
    pub winner: ChallengeWinner,
    pub reason: String,
}

/// Challenge winner
#[derive(Debug, Clone)]
pub enum ChallengeWinner {
    Original,
    Challenger,
}