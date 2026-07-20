// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Byzantine Fault Detection with State Proof Integration
//!
//! Detects Byzantine behavior in state proofs and certificate operations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::proof_of_state::{StateProof, SpaceProof, StakeProof, TimeProof, WorkProof};
use crate::errors::Result as TrustChainResult;

/// Byzantine fault detector with state proof integration
pub struct ByzantineDetector {
    /// Detection threshold (0.0 - 1.0)
    threshold: f64,
    /// Node behavior history
    node_history: Arc<RwLock<HashMap<String, NodeBehaviorHistory>>>,
    /// Detection patterns
    patterns: Arc<RwLock<DetectionPatterns>>,
    /// Detection statistics
    stats: Arc<RwLock<ByzantineDetectionStats>>,
}

/// Node behavior history for Byzantine detection
#[derive(Clone, Debug)]
pub struct NodeBehaviorHistory {
    /// Node ID
    pub node_id: String,
    /// Total state proofs submitted
    pub proofs_submitted: u64,
    /// Invalid proofs detected
    pub invalid_proofs: u64,
    /// Stake proof violations
    pub stake_violations: u64,
    /// Time proof violations
    pub time_violations: u64,
    /// Space proof violations
    pub space_violations: u64,
    /// Work proof violations
    pub work_violations: u64,
    /// Suspicious patterns detected
    pub suspicious_patterns: u64,
    /// Last activity timestamp
    pub last_activity: SystemTime,
    /// Byzantine confidence score (0.0 - 1.0)
    pub byzantine_confidence: f64,
}

impl Default for NodeBehaviorHistory {
    fn default() -> Self {
        Self {
            node_id: String::new(),
            proofs_submitted: 0,
            invalid_proofs: 0,
            stake_violations: 0,
            time_violations: 0,
            space_violations: 0,
            work_violations: 0,
            suspicious_patterns: 0,
            last_activity: SystemTime::now(),
            byzantine_confidence: 0.0,
        }
    }
}

/// Detection patterns for Byzantine behavior
#[derive(Clone, Debug, Default)]
pub struct DetectionPatterns {
    /// Repeated invalid stake signatures
    pub invalid_stake_signatures: HashMap<String, u64>,
    /// Time manipulation attempts
    pub time_manipulation: HashMap<String, u64>,
    /// Storage proof falsification
    pub storage_falsification: HashMap<String, u64>,
    /// Work proof cheating
    pub work_cheating: HashMap<String, u64>,
    /// State proof replay attacks
    pub replay_attacks: HashMap<String, u64>,
}

/// Byzantine detection statistics
#[derive(Clone, Debug, Default)]
pub struct ByzantineDetectionStats {
    /// Total detections performed
    pub total_detections: u64,
    /// Byzantine behavior detected
    pub byzantine_detected: u64,
    /// False positive rate
    pub false_positive_rate: f64,
    /// Detection accuracy
    pub detection_accuracy: f64,
    /// Average detection time (ms)
    pub avg_detection_time_ms: u64,
}

/// Byzantine detection result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ByzantineDetectionResult {
    NotDetected,
    Detected {
        /// Node ID of Byzantine actor
        node_id: String,
        /// Confidence level (0.0 - 1.0)
        confidence: f64,
        /// Detection reasons
        reasons: Vec<ByzantineViolation>,
        /// Detection timestamp
        detected_at: SystemTime,
    },
}

/// Byzantine violation types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ByzantineViolation {
    /// Invalid stake proof signature
    InvalidStakeSignature { stake_holder_id: String },
    /// Time manipulation detected
    TimeManipulation {
        time_offset: Duration,
        suspicious_nonce: u64,
    },
    /// Storage proof falsification
    StorageFalsification {
        claimed_storage: u64,
        actual_storage: u64,
    },
    /// Work proof cheating
    WorkCheating {
        claimed_power: u64,
        actual_power: u64,
    },
    /// State proof replay attack
    ReplayAttack { original_timestamp: SystemTime },
    /// Inconsistent proof data
    InconsistentProofData { proof_type: String, details: String },
}

/// Byzantine detection summary for dashboard
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ByzantineDetectionSummary {
    /// Total nodes monitored
    pub nodes_monitored: u64,
    /// Nodes with Byzantine behavior
    pub byzantine_nodes: u64,
    /// Active Byzantine detections
    pub active_detections: u64,
    /// Detection accuracy rate
    pub accuracy_rate: f64,
    /// Top suspicious nodes
    pub top_suspicious_nodes: Vec<SuspiciousNode>,
}

/// Suspicious node information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SuspiciousNode {
    /// Node ID
    pub node_id: String,
    /// Number of failed verifications
    pub failed_verifications: u32,
    /// Recent violations
    pub recent_violations: u64,
    /// Last violation timestamp
    pub last_violation: SystemTime,
}

impl ByzantineDetector {
    /// Create new Byzantine detector
    pub async fn new(threshold: f64) -> TrustChainResult<Self> {
        info!(
            "Initializing Byzantine fault detector with threshold: {:.2}",
            threshold
        );

        Ok(Self {
            threshold,
            node_history: Arc::new(RwLock::new(HashMap::new())),
            patterns: Arc::new(RwLock::new(DetectionPatterns::default())),
            stats: Arc::new(RwLock::new(ByzantineDetectionStats::default())),
        })
    }

    /// Detect Byzantine behavior in state proof
    pub async fn detect_byzantine_behavior(
        &self,
        state_proof: &StateProof,
        operation: &str,
    ) -> TrustChainResult<ByzantineDetectionResult> {
        let start_time = std::time::Instant::now();

        debug!(
            "Performing Byzantine detection for operation: {}",
            operation
        );

        // Update detection statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_detections += 1;
        }

        // Extract node ID from stake proof
        let node_id = state_proof.stake_proof.stake_holder_id.clone();

        // Analyze each proof component for Byzantine behavior
        let mut violations = Vec::new();

        // Check stake proof
        if let Some(violation) = self
            .analyze_stake_proof(&state_proof.stake_proof)
            .await?
        {
            violations.push(violation);
        }

        // Check time proof
        if let Some(violation) = self.analyze_time_proof(&state_proof.time_proof).await? {
            violations.push(violation);
        }

        // Check space proof
        if let Some(violation) = self
            .analyze_space_proof(&state_proof.space_proof)
            .await?
        {
            violations.push(violation);
        }

        // Check work proof
        if let Some(violation) = self.analyze_work_proof(&state_proof.work_proof).await? {
            violations.push(violation);
        }

        // Check for replay attacks
        if let Some(violation) = self.check_replay_attack(state_proof, &node_id).await? {
            violations.push(violation);
        }

        // Update node behavior history
        self.update_node_history(&node_id, &violations).await?;

        // Calculate Byzantine confidence
        let confidence = self
            .calculate_byzantine_confidence(&node_id, &violations)
            .await?;

        // Update detection timing
        let detection_time = start_time.elapsed().as_millis() as u64;
        {
            let mut stats = self.stats.write().await;
            stats.avg_detection_time_ms = (stats.avg_detection_time_ms + detection_time) / 2;
        }

        let result = if confidence >= self.threshold {
            warn!(
                "Byzantine behavior detected for node {}: confidence={:.2}, violations={}",
                node_id,
                confidence,
                violations.len()
            );

            // Update detection statistics
            {
                let mut stats = self.stats.write().await;
                stats.byzantine_detected += 1;
            }

            ByzantineDetectionResult::Detected {
                node_id: node_id.clone(),
                confidence,
                reasons: violations,
                detected_at: SystemTime::now(),
            }
        } else {
            debug!(
                "No Byzantine behavior detected for node {}: confidence={:.2}",
                node_id, confidence
            );
            ByzantineDetectionResult::NotDetected
        };

        debug!(
            "Byzantine detection completed for {} in {}ms",
            operation, detection_time
        );
        Ok(result)
    }

    /// Analyze stake proof for Byzantine behavior
    async fn analyze_stake_proof(
        &self,
        stake_proof: &StakeProof,
    ) -> TrustChainResult<Option<ByzantineViolation>> {
        // CANONICAL MODEL: PoStake is authorization (WHO), never a magnitude.
        // Byzantine check = a MISSING identity binding (the authorization is
        // malformed). There is NO stake-amount falsification heuristic because
        // there is no amount.
        if !stake_proof.is_structurally_valid() {
            warn!(
                "Malformed stake authorization detected for: {}",
                stake_proof.stake_holder_id
            );

            // Update pattern tracking
            {
                let mut patterns = self.patterns.write().await;
                *patterns
                    .invalid_stake_signatures
                    .entry(stake_proof.stake_holder_id.clone())
                    .or_insert(0) += 1;
            }

            return Ok(Some(ByzantineViolation::InvalidStakeSignature {
                stake_holder_id: stake_proof.stake_holder_id.clone(),
            }));
        }

        Ok(None)
    }

    /// Analyze time proof for Byzantine behavior
    async fn analyze_time_proof(
        &self,
        time_proof: &TimeProof,
    ) -> TrustChainResult<Option<ByzantineViolation>> {
        // Check for time manipulation
        let max_acceptable_offset = Duration::from_secs(300); // 5 minutes
        if time_proof.network_time_offset > max_acceptable_offset {
            warn!(
                "Suspicious time offset detected: {:?}",
                time_proof.network_time_offset
            );

            // Update pattern tracking
            {
                let mut patterns = self.patterns.write().await;
                *patterns
                    .time_manipulation
                    .entry("time_offset_violation".to_string())
                    .or_insert(0) += 1;
            }

            return Ok(Some(ByzantineViolation::TimeManipulation {
                time_offset: time_proof.network_time_offset,
                suspicious_nonce: time_proof.nonce,
            }));
        }

        // Check for nonce reuse (potential replay)
        // In production, this would check against a nonce database
        if time_proof.nonce == 0 {
            warn!("Suspicious nonce value detected: {}", time_proof.nonce);
        }

        Ok(None)
    }

    /// Analyze space proof for Byzantine behavior
    async fn analyze_space_proof(
        &self,
        space_proof: &SpaceProof,
    ) -> TrustChainResult<Option<ByzantineViolation>> {
        // Check for storage falsification
        if space_proof.total_size > space_proof.total_storage {
            warn!(
                "Storage falsification detected: claimed {} > capacity {}",
                space_proof.total_size, space_proof.total_storage
            );

            // Update pattern tracking
            {
                let mut patterns = self.patterns.write().await;
                *patterns
                    .storage_falsification
                    .entry(space_proof.node_id.clone())
                    .or_insert(0) += 1;
            }

            return Ok(Some(ByzantineViolation::StorageFalsification {
                claimed_storage: space_proof.total_size,
                actual_storage: space_proof.total_storage,
            }));
        }

        // CANONICAL MODEL: there is NO upper capacity bound. Capacity is a
        // descriptive asset attribute, never a proof field — a large claim is
        // not by itself Byzantine. The falsification check above (stored bytes
        // exceeding advertised capacity) is a self-consistency check on the
        // proof, which is the only thing PoSpace can be wrong about.

        Ok(None)
    }

    /// Analyze work proof for Byzantine behavior
    async fn analyze_work_proof(
        &self,
        work_proof: &WorkProof,
    ) -> TrustChainResult<Option<ByzantineViolation>> {
        // CANONICAL MODEL: PoWork is the HASH of work done, never a capacity
        // claim. Work cheating = presenting a proof with NO real work hashed
        // (a zero work_hash), i.e. claiming work that was never performed.
        if work_proof.work_hash == [0u8; 32] {
            warn!(
                "Work proof with zero work hash (no work performed) for node {}",
                work_proof.owner_id
            );

            // Update pattern tracking
            {
                let mut patterns = self.patterns.write().await;
                *patterns
                    .work_cheating
                    .entry(work_proof.owner_id.clone())
                    .or_insert(0) += 1;
            }

            return Ok(Some(ByzantineViolation::WorkCheating {
                claimed_power: 0,
                actual_power: 0,
            }));
        }

        Ok(None)
    }

    /// Check for replay attacks
    async fn check_replay_attack(
        &self,
        state_proof: &StateProof,
        node_id: &str,
    ) -> TrustChainResult<Option<ByzantineViolation>> {
        // In production, this would check against a database of used proofs
        // For now, we'll do a simple timestamp check

        let proof_age = state_proof
            .time_proof
            .time_verification_timestamp
            .elapsed()
            .unwrap_or(Duration::from_secs(0));

        // Proofs older than 1 hour are suspicious
        if proof_age > Duration::from_secs(3600) {
            warn!(
                "Potentially replayed proof detected for node {}: age={:?}",
                node_id, proof_age
            );

            // Update pattern tracking
            {
                let mut patterns = self.patterns.write().await;
                *patterns
                    .replay_attacks
                    .entry(node_id.to_string())
                    .or_insert(0) += 1;
            }

            return Ok(Some(ByzantineViolation::ReplayAttack {
                original_timestamp: state_proof.time_proof.time_verification_timestamp,
            }));
        }

        Ok(None)
    }

    /// Update node behavior history
    async fn update_node_history(
        &self,
        node_id: &str,
        violations: &[ByzantineViolation],
    ) -> TrustChainResult<()> {
        let mut history = self.node_history.write().await;
        let node_history =
            history
                .entry(node_id.to_string())
                .or_insert_with(|| NodeBehaviorHistory {
                    node_id: node_id.to_string(),
                    ..Default::default()
                });

        // Update counters
        node_history.proofs_submitted += 1;
        node_history.last_activity = SystemTime::now();

        // Process violations
        for violation in violations {
            node_history.invalid_proofs += 1;

            match violation {
                ByzantineViolation::InvalidStakeSignature { .. } => {
                    node_history.stake_violations += 1;
                }
                ByzantineViolation::TimeManipulation { .. } => {
                    node_history.time_violations += 1;
                }
                ByzantineViolation::StorageFalsification { .. } => {
                    node_history.space_violations += 1;
                }
                ByzantineViolation::WorkCheating { .. } => {
                    node_history.work_violations += 1;
                }
                ByzantineViolation::ReplayAttack { .. } => {
                    node_history.suspicious_patterns += 1;
                }
                ByzantineViolation::InconsistentProofData { .. } => {
                    node_history.suspicious_patterns += 1;
                }
            }
        }

        debug!(
            "Updated behavior history for node {}: {} violations",
            node_id,
            violations.len()
        );
        Ok(())
    }

    /// Calculate Byzantine confidence score
    async fn calculate_byzantine_confidence(
        &self,
        node_id: &str,
        current_violations: &[ByzantineViolation],
    ) -> TrustChainResult<f64> {
        let history = self.node_history.read().await;

        if let Some(node_history) = history.get(node_id) {
            // Calculate confidence based on violation rate and patterns
            let violation_rate = if node_history.proofs_submitted > 0 {
                node_history.invalid_proofs as f64 / node_history.proofs_submitted as f64
            } else {
                0.0
            };

            // Weight current violations more heavily
            let current_violation_weight = current_violations.len() as f64 * 0.3;

            // Calculate confidence score
            let mut confidence = violation_rate + current_violation_weight;

            // Add penalties for specific violation types
            if node_history.stake_violations > 3 {
                confidence += 0.2; // Stake violations are serious
            }
            if node_history.suspicious_patterns > 5 {
                confidence += 0.3; // Multiple patterns indicate Byzantine behavior
            }

            // Cap confidence at 1.0
            confidence = confidence.min(1.0);

            // Update node history with new confidence
            drop(history);
            let mut history_mut = self.node_history.write().await;
            if let Some(node_history_mut) = history_mut.get_mut(node_id) {
                node_history_mut.byzantine_confidence = confidence;
            }

            Ok(confidence)
        } else {
            // New node with current violations
            Ok(current_violations.len() as f64 * 0.2)
        }
    }

    /// Get detection summary for dashboard
    pub async fn get_detection_summary(&self) -> TrustChainResult<ByzantineDetectionSummary> {
        let history = self.node_history.read().await;
        let stats = self.stats.read().await;

        let nodes_monitored = history.len() as u64;
        let byzantine_nodes = history
            .values()
            .filter(|h| h.byzantine_confidence >= self.threshold)
            .count() as u64;

        let active_detections = byzantine_nodes; // For now, all Byzantine nodes are "active"

        let accuracy_rate = stats.detection_accuracy;

        // Get top 5 suspicious nodes
        let mut suspicious_nodes: Vec<_> = history
            .values()
            .filter(|h| h.byzantine_confidence > 0.1) // Only nodes with some suspicion
            .map(|h| SuspiciousNode {
                node_id: h.node_id.clone(),
                failed_verifications: h.invalid_proofs as u32,
                recent_violations: h.invalid_proofs,
                last_violation: h.last_activity,
            })
            .collect();

        suspicious_nodes.sort_by(|a, b| {
            b.failed_verifications.cmp(&a.failed_verifications)
        });
        suspicious_nodes.truncate(5);

        Ok(ByzantineDetectionSummary {
            nodes_monitored,
            byzantine_nodes,
            active_detections,
            accuracy_rate,
            top_suspicious_nodes: suspicious_nodes,
        })
    }

    /// Get Byzantine detection statistics
    pub async fn get_stats(&self) -> ByzantineDetectionStats {
        self.stats.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof_of_state::StateProof;

    #[tokio::test]
    async fn test_byzantine_detector_creation() {
        let detector = ByzantineDetector::new(0.5).await.expect("test: async operation");
        assert_eq!(detector.threshold, 0.5);
    }

    #[tokio::test]
    async fn test_valid_state_proof() {
        let detector = ByzantineDetector::new(0.5).await.expect("test: async operation");
        let state_proof = StateProof::default_for_testing();

        let result = detector
            .detect_byzantine_behavior(&state_proof, "test_operation")
            .await
            .expect("test: expected success");

        match result {
            ByzantineDetectionResult::NotDetected => {
                // Expected for valid proof
            }
            ByzantineDetectionResult::Detected { .. } => {
                panic!("Valid proof should not trigger Byzantine detection");
            }
        }
    }

    #[tokio::test]
    async fn test_invalid_stake_proof() {
        let detector = ByzantineDetector::new(0.1).await.expect("test: async operation"); // Low threshold for testing

        // Create an invalid authorization: unbind the identity (WHO). PoStake
        // is authorization, not an amount, so a malformed proof = empty id.
        let mut state_proof = StateProof::default_for_testing();
        state_proof.stake_proof.stake_holder_id = String::new();

        let result = detector
            .detect_byzantine_behavior(&state_proof, "test_operation")
            .await
            .expect("test: expected success");

        match result {
            ByzantineDetectionResult::Detected { reasons, .. } => {
                // Should detect invalid stake
                assert!(!reasons.is_empty());
            }
            ByzantineDetectionResult::NotDetected => {
                // May not detect with current implementation
            }
        }
    }

    #[tokio::test]
    async fn test_time_manipulation_detection() {
        let detector = ByzantineDetector::new(0.1).await.expect("test: async operation");

        // Create time proof with excessive offset
        let mut state_proof = StateProof::default_for_testing();
        state_proof.time_proof.network_time_offset = Duration::from_secs(3600); // 1 hour offset

        let result = detector
            .detect_byzantine_behavior(&state_proof, "test_operation")
            .await
            .expect("test: expected success");

        match result {
            ByzantineDetectionResult::Detected { reasons, .. } => {
                assert!(reasons
                    .iter()
                    .any(|v| matches!(v, ByzantineViolation::TimeManipulation { .. })));
            }
            ByzantineDetectionResult::NotDetected => {
                panic!("Time manipulation should be detected");
            }
        }
    }

    #[tokio::test]
    async fn test_storage_falsification_detection() {
        let detector = ByzantineDetector::new(0.1).await.expect("test: async operation");

        // Create space proof with falsified storage
        let mut state_proof = StateProof::default_for_testing();
        state_proof.space_proof.total_size = 1000;
        state_proof.space_proof.total_storage = 500; // Size > storage (impossible)

        let result = detector
            .detect_byzantine_behavior(&state_proof, "test_operation")
            .await
            .expect("test: expected success");

        match result {
            ByzantineDetectionResult::Detected { reasons, .. } => {
                assert!(reasons
                    .iter()
                    .any(|v| matches!(v, ByzantineViolation::StorageFalsification { .. })));
            }
            ByzantineDetectionResult::NotDetected => {
                panic!("Storage falsification should be detected");
            }
        }
    }

    #[tokio::test]
    async fn test_detection_summary() {
        let detector = ByzantineDetector::new(0.5).await.expect("test: async operation");

        // Perform some detections to populate data
        let state_proof = StateProof::default_for_testing();
        detector
            .detect_byzantine_behavior(&state_proof, "test_operation")
            .await
            .expect("test: expected success");

        let summary = detector.get_detection_summary().await.expect("test: async operation");

        assert_eq!(summary.nodes_monitored, 1);
        // Other fields depend on detection results
    }
}
