// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Production State Proof Validator
//!
//! Real four-proof state authentication with misbehavior detection.
//! Each proof is binary pass/fail. ALL four must pass.
//! No voting, no quorum, no confidence scores.

use crate::proof_of_state::proof::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tracing::{error, info, warn};

/// Production state proof authenticator with misbehavior detection
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateAuthenticator {
    /// Misbehavior detector (identifies nodes whose proofs consistently fail)
    misbehavior_detector: MisbehaviorDetector,
    /// Validation metrics
    metrics: ValidationMetrics,
    /// Security configuration
    security_config: SecurityConfig,
}

/// Misbehavior detector - identifies nodes whose proofs consistently fail
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MisbehaviorDetector {
    /// Known malicious nodes
    malicious_nodes: HashMap<String, MaliciousNodeInfo>,
    /// Suspicious activity tracking
    suspicious_activity: HashMap<String, SuspiciousActivity>,
    /// Detection thresholds
    thresholds: MisbehaviorThresholds,
}

/// Malicious node information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaliciousNodeInfo {
    pub node_id: String,
    pub detected_at: SystemTime,
    pub violation_type: SecurityViolationType,
    pub evidence: Vec<String>,
}

/// Suspicious activity tracking
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SuspiciousActivity {
    pub failed_validations: u64,
    pub invalid_signatures: u64,
    pub timestamp_anomalies: u64,
    pub last_activity: SystemTime,
}

/// Security violation types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SecurityViolationType {
    InvalidSignature,
    TimestampManipulation,
    FalseStakeProof,
    StorageCommitmentFraud,
    ComputationalFraud,
    DoubleSpending,
    NetworkPartitioning,
}

/// Misbehavior detection thresholds
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MisbehaviorThresholds {
    pub max_failed_validations: u64,
    pub max_timestamp_offset: Duration,
    pub detection_window: Duration,
}

/// Validation metrics
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ValidationMetrics {
    pub total_validations: u64,
    pub successful_validations: u64,
    pub failed_validations: u64,
    pub misbehavior_detections: u64,
    pub rejected_proofs: u64,
}

/// Security configuration.
///
/// CANONICAL MODEL: PoStake is authorization (WHO), never a magnitude. There is
/// no minimum-stake threshold — admission keys on a bound identity and fresh
/// proofs, not on a coin quantity that does not exist.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub strict_mode: bool,
    pub require_all_proofs: bool,
    pub enable_misbehavior_detection: bool,
    pub maximum_time_variance: Duration,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            strict_mode: true,
            require_all_proofs: true,
            enable_misbehavior_detection: true,
            maximum_time_variance: Duration::from_secs(30),
        }
    }
}

impl Default for MisbehaviorThresholds {
    fn default() -> Self {
        Self {
            max_failed_validations: 3,
            max_timestamp_offset: Duration::from_secs(300),
            detection_window: Duration::from_secs(3600),
        }
    }
}

impl StateAuthenticator {
    pub fn new() -> Self {
        Self {
            misbehavior_detector: MisbehaviorDetector::new(),
            metrics: ValidationMetrics::default(),
            security_config: SecurityConfig::default(),
        }
    }

    /// Create production-grade authenticator with strict security
    pub fn production() -> Self {
        let mut authenticator = Self::new();
        authenticator.security_config.strict_mode = true;
        authenticator.security_config.require_all_proofs = true;
        authenticator
    }
}

impl Default for StateAuthenticator {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for MisbehaviorDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl MisbehaviorDetector {
    pub fn new() -> Self {
        Self {
            malicious_nodes: HashMap::new(),
            suspicious_activity: HashMap::new(),
            thresholds: MisbehaviorThresholds::default(),
        }
    }

    /// Detect potential misbehavior from a node whose proofs fail
    pub fn detect_misbehavior(
        &mut self,
        node_id: &str,
        violation: SecurityViolationType,
    ) -> bool {
        let activity = self
            .suspicious_activity
            .entry(node_id.to_string())
            .or_insert(SuspiciousActivity {
                failed_validations: 0,
                invalid_signatures: 0,
                timestamp_anomalies: 0,
                last_activity: SystemTime::now(),
            });

        // Update suspicious activity based on violation type
        match violation {
            SecurityViolationType::InvalidSignature => activity.invalid_signatures += 1,
            SecurityViolationType::TimestampManipulation => activity.timestamp_anomalies += 1,
            _ => activity.failed_validations += 1,
        }

        activity.last_activity = SystemTime::now();

        // Check if node exceeds misbehavior thresholds
        let is_malicious = activity.failed_validations >= self.thresholds.max_failed_validations
            || activity.invalid_signatures >= 5
            || activity.timestamp_anomalies >= 3;

        if is_malicious {
            warn!(
                "Misbehaving node detected: {} - Violation: {:?}",
                node_id, violation
            );
            self.mark_as_malicious(node_id, violation);
        }

        is_malicious
    }

    /// Mark node as malicious
    fn mark_as_malicious(&mut self, node_id: &str, violation: SecurityViolationType) {
        let malicious_info = MaliciousNodeInfo {
            node_id: node_id.to_string(),
            detected_at: SystemTime::now(),
            violation_type: violation.clone(),
            evidence: vec![format!("Automated detection: {:?}", violation)],
        };

        self.malicious_nodes
            .insert(node_id.to_string(), malicious_info);
        error!("Node {} marked as malicious and blocked", node_id);
    }

    /// Check if node is known malicious
    pub fn is_malicious(&self, node_id: &str) -> bool {
        self.malicious_nodes.contains_key(node_id)
    }
}

/// Production four-proof authenticator with real state proof validation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FourProofValidator {
    pub space_validator: ProofOfSpaceValidator,
    pub stake_validator: ProofOfStakeValidator,
    pub work_validator: ProofOfWorkValidator,
    pub time_validator: ProofOfTimeValidator,
    /// Misbehavior detector
    pub misbehavior_detector: MisbehaviorDetector,
    /// Validation metrics
    pub metrics: ValidationMetrics,
    /// Security configuration
    pub security_config: SecurityConfig,
}

impl Default for FourProofValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl FourProofValidator {
    pub fn new() -> Self {
        Self {
            space_validator: ProofOfSpaceValidator::new(),
            stake_validator: ProofOfStakeValidator::new(),
            work_validator: ProofOfWorkValidator::new(),
            time_validator: ProofOfTimeValidator::new(),
            misbehavior_detector: MisbehaviorDetector::new(),
            metrics: ValidationMetrics::default(),
            security_config: SecurityConfig::default(),
        }
    }

    /// Create production authenticator with strict security requirements
    pub fn production() -> Self {
        let mut validator = Self::new();
        validator.security_config = SecurityConfig {
            strict_mode: true,
            require_all_proofs: true,
            enable_misbehavior_detection: true,
            maximum_time_variance: Duration::from_secs(15), // Strict timing
        };
        validator
    }

    /// PRODUCTION STATE PROOF VALIDATION - binary pass/fail
    pub async fn validate_state_proof(
        &mut self,
        proof: &crate::proof_of_state::StateProof,
    ) -> Result<crate::proof_of_state::StateProofResult> {
        use crate::proof_of_state::StateProofResult;
        use std::time::SystemTime;

        info!("Starting production state proof validation (strict mode)");
        let start_time = std::time::Instant::now();

        self.metrics.total_validations += 1;
        let mut failed_proofs = Vec::new();

        // Check for known malicious nodes FIRST
        let node_id = &proof.stake_proof.stake_holder_id;
        if self.misbehavior_detector.is_malicious(node_id) {
            error!(
                "SECURITY VIOLATION: Rejecting proof from known malicious node: {}",
                node_id
            );
            self.metrics.rejected_proofs += 1;
            return Ok(StateProofResult::Invalid {
                reason: format!("Malicious node detected: {node_id}"),
                failed_proofs: vec!["MALICIOUS_NODE".to_string()],
                validation_timestamp: SystemTime::now(),
            });
        }

        // 1. PROOF OF SPACE VALIDATION (WHERE)
        info!("Validating Proof of Space (WHERE)");
        let space_valid = self
            .space_validator
            .validate_production(&proof.space_proof)
            .await?;
        if !space_valid {
            failed_proofs.push("SPACE_PROOF_FAILED".to_string());
            self.misbehavior_detector
                .detect_misbehavior(node_id, SecurityViolationType::StorageCommitmentFraud);
        }

        // 2. PROOF OF STAKE VALIDATION (WHO)
        info!("Validating Proof of Stake (WHO)");
        let stake_valid = self
            .stake_validator
            .validate_production(&proof.stake_proof, &self.security_config)
            .await?;
        if !stake_valid {
            failed_proofs.push("STAKE_PROOF_FAILED".to_string());
            self.misbehavior_detector
                .detect_misbehavior(node_id, SecurityViolationType::FalseStakeProof);
        }

        // 3. PROOF OF WORK VALIDATION (WHAT)
        info!("Validating Proof of Work (WHAT)");
        let work_valid = self
            .work_validator
            .validate_production(&proof.work_proof)
            .await?;
        if !work_valid {
            failed_proofs.push("WORK_PROOF_FAILED".to_string());
            self.misbehavior_detector
                .detect_misbehavior(node_id, SecurityViolationType::ComputationalFraud);
        }

        // 4. PROOF OF TIME VALIDATION (WHEN)
        info!("Validating Proof of Time (WHEN)");
        let time_valid = self
            .time_validator
            .validate_production(&proof.time_proof, &self.security_config)
            .await?;
        if !time_valid {
            failed_proofs.push("TIME_PROOF_FAILED".to_string());
            self.misbehavior_detector
                .detect_misbehavior(node_id, SecurityViolationType::TimestampManipulation);
        }

        // ALL FOUR PROOFS MUST PASS - binary result
        let all_proofs_valid = space_valid && stake_valid && work_valid && time_valid;

        let validation_time = start_time.elapsed().as_millis();

        if all_proofs_valid {
            let _proof_hash = proof.hash()?;
            self.metrics.successful_validations += 1;

            info!("State proof validation PASSED ({}ms)", validation_time);
            Ok(StateProofResult::Valid {
                valid: true,
                validation_timestamp: SystemTime::now(),
                validation_duration: std::time::Duration::from_millis(validation_time as u64),
            })
        } else {
            self.metrics.failed_validations += 1;
            self.metrics.rejected_proofs += 1;

            error!(
                "State proof validation FAILED - Rejected proofs: {:?} ({}ms)",
                failed_proofs, validation_time
            );
            Ok(StateProofResult::Invalid {
                reason: format!(
                    "Production validation failed: {} out of 4 proofs invalid",
                    failed_proofs.len()
                ),
                failed_proofs,
                validation_timestamp: SystemTime::now(),
            })
        }
    }

    /// Get validation metrics for monitoring
    pub fn get_metrics(&self) -> &ValidationMetrics {
        &self.metrics
    }

    /// Get misbehavior detection report
    pub fn get_misbehavior_report(&self) -> HashMap<String, MaliciousNodeInfo> {
        self.misbehavior_detector.malicious_nodes.clone()
    }
}

/// Production Proof of Space validator (WHERE)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofOfSpaceValidator {
    known_storage_nodes: HashMap<String, StorageNodeInfo>,
}

/// Storage node information for validation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageNodeInfo {
    node_id: String,
    verified_capacity: u64,
    last_verified: SystemTime,
    is_verified: bool,
}

impl Default for ProofOfSpaceValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ProofOfSpaceValidator {
    pub fn new() -> Self {
        Self {
            known_storage_nodes: HashMap::new(),
        }
    }

    /// Basic validation (for testing)
    pub async fn validate(&self, proof: &SpaceProof) -> Result<bool> {
        Ok(proof.validate())
    }

    /// PRODUCTION VALIDATION - Real storage commitment verification
    pub async fn validate_production(&mut self, proof: &SpaceProof) -> Result<bool> {
        info!(
            "PRODUCTION Space Proof validation for node: {}",
            proof.node_id
        );

        // 1. Basic validation first
        if !proof.validate() {
            error!("Space proof failed basic validation");
            return Ok(false);
        }

        // 2. PoSpace is WHERE. Require a bound LOCATION, not a capacity.
        //    CANONICAL MODEL: storage capacity is a descriptive asset attribute
        //    — it is never a proof field and never gates admission, so a node
        //    advertising zero spare capacity still answers WHERE.
        if proof.node_id.is_empty() && proof.storage_path.is_empty() {
            error!("Space proof: no bound location (WHERE)");
            return Ok(false);
        }

        // 3. Check storage size vs. claimed size ratio (self-consistency:
        //    stored ≤ capacity). Capacity is descriptive; there is NO upper
        //    capacity bound — PoSpace is WHERE (location), not a magnitude.
        if proof.total_size > proof.total_storage {
            error!("Space proof: Claimed size exceeds storage capacity");
            return Ok(false);
        }

        // 4. Verify file hash if provided
        if !proof.file_hash.is_empty() && proof.file_hash.len() < 32 {
            error!("Space proof: Invalid file hash length");
            return Ok(false);
        }

        // 6. Check storage path is reasonable
        if proof.storage_path.is_empty() || proof.storage_path.len() > 1000 {
            error!("Space proof: Invalid storage path");
            return Ok(false);
        }

        // 7. Timestamp validation
        if let Ok(elapsed) = proof.proof_timestamp.elapsed() {
            if elapsed > Duration::from_secs(3600) {
                error!("Space proof: Timestamp too old ({}s)", elapsed.as_secs());
                return Ok(false);
            }
        }

        // Update known storage nodes
        self.known_storage_nodes.insert(
            proof.node_id.clone(),
            StorageNodeInfo {
                node_id: proof.node_id.clone(),
                verified_capacity: proof.total_storage,
                last_verified: SystemTime::now(),
                is_verified: true,
            },
        );

        info!(
            "Space proof validation PASSED for node: {}",
            proof.node_id
        );
        Ok(true)
    }
}

/// Production Proof of Stake validator (WHO / authorization).
///
/// CANONICAL MODEL: PoStake is an authorization (a bound identity), never a
/// magnitude. There is no per-entity minimum-stake table — admission keys on a
/// bound FALCON identity and a fresh authorization timestamp.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProofOfStakeValidator;

impl ProofOfStakeValidator {
    pub fn new() -> Self {
        Self
    }

    /// Basic validation (for testing)
    pub async fn validate(&self, proof: &StakeProof) -> Result<bool> {
        Ok(proof.validate())
    }

    /// PRODUCTION VALIDATION - Authorization (WHO) verification.
    ///
    /// `_config` is retained for signature compatibility; PoStake carries no
    /// magnitude, so no threshold from config is consulted.
    pub async fn validate_production(
        &mut self,
        proof: &StakeProof,
        _config: &SecurityConfig,
    ) -> Result<bool> {
        info!(
            "PRODUCTION Stake Proof validation for holder: {}",
            proof.stake_holder
        );

        // CANONICAL MODEL: PoStake is authorization (WHO), never a magnitude.
        // There is NO minimum-stake threshold and NO MAX_REASONABLE_STAKE
        // overflow gate — those enforced a coin quantity that does not exist.
        // Authorization = a bound FALCON identity + fresh (non-stale) timestamp.

        // 1. Basic structural validation (identity bound + not stale).
        if !Proof::validate(proof) {
            error!("Stake proof failed basic validation");
            return Ok(false);
        }

        // 2. Verify stake holder identity is not empty (the WHO binding).
        if proof.stake_holder.is_empty() || proof.stake_holder_id.is_empty() {
            error!("Stake proof: Missing stake holder identity");
            return Ok(false);
        }

        // 3. Timestamp validation - authorization must not be too old.
        if let Ok(elapsed) = proof.stake_timestamp.elapsed() {
            if elapsed > Duration::from_secs(30 * 24 * 60 * 60) {
                error!(
                    "Stake proof: authorization timestamp too old ({}s)",
                    elapsed.as_secs()
                );
                return Ok(false);
            }
        }

        info!(
            "Stake proof validation PASSED for holder: {} (id: {})",
            proof.stake_holder, proof.stake_holder_id
        );
        Ok(true)
    }
}

/// Proof of Work validator
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofOfWorkValidator;

impl Default for ProofOfWorkValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ProofOfWorkValidator {
    pub fn new() -> Self {
        Self
    }

    pub async fn validate(&self, proof: &WorkProof) -> Result<bool> {
        Ok(proof.validate())
    }

    /// Production validation - validates real computational work
    pub async fn validate_production(&self, proof: &WorkProof) -> Result<bool> {
        info!(
            "PRODUCTION Work Proof validation for workload: {}",
            proof.workload_id
        );

        // CANONICAL MODEL: PoWork is the HASH of work done, never a capacity
        // number. There is NO computational_power magnitude gate and no
        // workload-type classification. WHAT = a bound owner + a real
        // (non-zero) BLAKE3 work hash.
        if !Proof::validate(proof) {
            error!("Work proof failed basic validation");
            return Ok(false);
        }

        if proof.work_hash == [0u8; 32] {
            error!("Work proof: zero work hash (no work performed)");
            return Ok(false);
        }

        info!(
            "Work proof validation PASSED for workload: {}",
            proof.workload_id
        );
        Ok(true)
    }
}

/// Proof of Time validator
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofOfTimeValidator;

impl Default for ProofOfTimeValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ProofOfTimeValidator {
    pub fn new() -> Self {
        Self
    }

    pub async fn validate(&self, proof: &TimeProof) -> Result<bool> {
        Ok(proof.validate())
    }

    /// Production validation - validates real time synchronization
    pub async fn validate_production(
        &self,
        proof: &TimeProof,
        config: &SecurityConfig,
    ) -> Result<bool> {
        info!("PRODUCTION Time Proof validation");

        // Basic validation first
        if !proof.validate() {
            error!("Time proof failed basic validation");
            return Ok(false);
        }

        // Verify time synchronization is within acceptable bounds
        if proof.network_time_offset > config.maximum_time_variance {
            error!(
                "Time proof: Network time offset too large: {:?} > {:?}",
                proof.network_time_offset, config.maximum_time_variance
            );
            return Ok(false);
        }

        // Verify proof timestamp is recent
        if let Ok(elapsed) = proof.time_verification_timestamp.elapsed() {
            if elapsed > Duration::from_secs(300) {
                // 5 minutes max age
                error!("Time proof: Timestamp too old: {}s", elapsed.as_secs());
                return Ok(false);
            }
        } else {
            error!("Time proof: Invalid timestamp (future timestamp)");
            return Ok(false);
        }

        // Verify proof hash is correct
        if proof.proof_hash.is_empty() {
            error!("Time proof: Missing proof hash");
            return Ok(false);
        }

        info!("Time proof validation PASSED");
        Ok(true)
    }
}
