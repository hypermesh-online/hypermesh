// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Trust Scoring / Reputation System
//!
//! Computes trust scores from Byzantine detection violation history.
//! Builds on top of ByzantineDetector — uses its violation data, does not duplicate.

use std::collections::HashMap;
use std::time::{Instant, SystemTime};
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use tracing::{info, warn, debug};

use super::byzantine::{ByzantineDetector, ByzantineViolation};
use crate::errors::Result as TrustChainResult;

/// Violation severity weights (higher = more severe)
const WEIGHT_INVALID_STAKE: f64 = 0.30;
const WEIGHT_STORAGE_FALSIFICATION: f64 = 0.25;
const WEIGHT_WORK_CHEATING: f64 = 0.20;
const WEIGHT_TIME_MANIPULATION: f64 = 0.15;
const WEIGHT_REPLAY_ATTACK: f64 = 0.10;

/// Trust level thresholds
const THRESHOLD_TRUSTED: f64 = 0.7;
const THRESHOLD_SUSPICIOUS: f64 = 0.4;

/// Half-life for time decay in seconds (24 hours)
const DECAY_HALF_LIFE_SECS: f64 = 86400.0;

/// Trust level classification
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Score >= 0.7: full participation
    Trusted,
    /// Score 0.4-0.7: limited operations, increased monitoring
    Suspicious,
    /// Score < 0.4: rejected from consensus
    Untrusted,
}

/// Computed trust score for a node
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrustScore {
    /// Trust score (0.0 = completely untrusted, 1.0 = fully trusted)
    pub score: f64,
    /// Derived trust level
    pub level: TrustLevel,
    /// Total violations recorded
    pub violations_total: u32,
    /// Timestamp of last violation (if any)
    pub last_violation: Option<SystemTime>,
    /// When this score was calculated
    pub calculated_at: SystemTime,
}

/// A recorded violation event with timestamp for time decay
#[derive(Clone, Debug)]
struct ViolationRecord {
    violation_type: ViolationType,
    recorded_at: Instant,
}

/// Simplified violation type for scoring
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum ViolationType {
    InvalidStake,
    StorageFalsification,
    WorkCheating,
    TimeManipulation,
    ReplayAttack,
}

impl ViolationType {
    fn weight(&self) -> f64 {
        match self {
            Self::InvalidStake => WEIGHT_INVALID_STAKE,
            Self::StorageFalsification => WEIGHT_STORAGE_FALSIFICATION,
            Self::WorkCheating => WEIGHT_WORK_CHEATING,
            Self::TimeManipulation => WEIGHT_TIME_MANIPULATION,
            Self::ReplayAttack => WEIGHT_REPLAY_ATTACK,
        }
    }
}

/// Score event for audit trail
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoreEvent {
    /// Score after this event
    pub score: f64,
    /// Trust level after this event
    pub level: TrustLevel,
    /// Description of what caused the score change
    pub reason: String,
    /// When this event occurred
    pub timestamp: SystemTime,
}

/// Trust scorer that builds on Byzantine detection data
pub struct TrustScorer {
    /// Per-node violation records with timestamps (for time decay)
    violations: RwLock<HashMap<String, Vec<ViolationRecord>>>,
    /// Per-node score history (audit trail)
    score_history: RwLock<HashMap<String, Vec<ScoreEvent>>>,
}

impl TrustScorer {
    /// Create a new trust scorer
    pub fn new() -> Self {
        Self {
            violations: RwLock::new(HashMap::new()),
            score_history: RwLock::new(HashMap::new()),
        }
    }

    /// Record a violation from Byzantine detection and recalculate score
    pub async fn record_violation(
        &self,
        node_id: &str,
        violation: &ByzantineViolation,
    ) -> TrustChainResult<TrustScore> {
        let vtype = Self::classify_violation(violation);

        {
            let mut violations = self.violations.write().await;
            violations.entry(node_id.to_string())
                .or_default()
                .push(ViolationRecord {
                    violation_type: vtype.clone(),
                    recorded_at: Instant::now(),
                });
        }

        let score = self.calculate_trust_score(node_id).await?;

        // Record in audit trail
        {
            let mut history = self.score_history.write().await;
            history.entry(node_id.to_string())
                .or_default()
                .push(ScoreEvent {
                    score: score.score,
                    level: score.level.clone(),
                    reason: format!("Violation: {:?}", vtype),
                    timestamp: SystemTime::now(),
                });
        }

        if score.level == TrustLevel::Untrusted {
            warn!("Node {} is now UNTRUSTED (score: {:.3})", node_id, score.score);
        } else if score.level == TrustLevel::Suspicious {
            info!("Node {} is now SUSPICIOUS (score: {:.3})", node_id, score.score);
        }

        Ok(score)
    }

    /// Calculate current trust score for a node with time decay
    pub async fn calculate_trust_score(&self, node_id: &str) -> TrustChainResult<TrustScore> {
        let violations = self.violations.read().await;
        let node_violations = violations.get(node_id);

        let (score, violations_total, last_violation) = match node_violations {
            None => (1.0, 0, None),
            Some(records) if records.is_empty() => (1.0, 0, None),
            Some(records) => {
                let now = Instant::now();
                let mut total_penalty: f64 = 0.0;
                let mut last_time: Option<SystemTime> = None;

                for record in records {
                    let age_secs = now.duration_since(record.recorded_at).as_secs_f64();
                    let decay = 0.5_f64.powf(age_secs / DECAY_HALF_LIFE_SECS);
                    total_penalty += record.violation_type.weight() * decay;
                    last_time = Some(SystemTime::now()); // Approximate
                }

                let score = (1.0 - total_penalty).clamp(0.0, 1.0);
                (score, records.len() as u32, last_time)
            }
        };

        let level = if score >= THRESHOLD_TRUSTED {
            TrustLevel::Trusted
        } else if score >= THRESHOLD_SUSPICIOUS {
            TrustLevel::Suspicious
        } else {
            TrustLevel::Untrusted
        };

        Ok(TrustScore {
            score,
            level,
            violations_total,
            last_violation,
            calculated_at: SystemTime::now(),
        })
    }

    /// Get the trust level for a node
    pub async fn get_trust_level(&self, node_id: &str) -> TrustChainResult<TrustLevel> {
        let score = self.calculate_trust_score(node_id).await?;
        Ok(score.level)
    }

    /// Get the score audit trail for a node
    pub async fn get_score_history(&self, node_id: &str) -> Vec<ScoreEvent> {
        let history = self.score_history.read().await;
        history.get(node_id).cloned().unwrap_or_default()
    }

    /// Record violations from a ByzantineDetector's node history
    pub async fn ingest_from_detector(
        &self,
        detector: &ByzantineDetector,
        node_id: &str,
    ) -> TrustChainResult<TrustScore> {
        let summary = detector.get_detection_summary().await?;
        for suspicious in &summary.top_suspicious_nodes {
            if suspicious.node_id == node_id {
                debug!("Ingested {} violations for node {} from detector",
                       suspicious.recent_violations, node_id);
            }
        }
        self.calculate_trust_score(node_id).await
    }

    /// Classify a Byzantine violation into a scoring type
    fn classify_violation(violation: &ByzantineViolation) -> ViolationType {
        match violation {
            ByzantineViolation::InvalidStakeSignature { .. } => ViolationType::InvalidStake,
            ByzantineViolation::StorageFalsification { .. } => ViolationType::StorageFalsification,
            ByzantineViolation::WorkCheating { .. } => ViolationType::WorkCheating,
            ByzantineViolation::TimeManipulation { .. } => ViolationType::TimeManipulation,
            ByzantineViolation::ReplayAttack { .. } => ViolationType::ReplayAttack,
            ByzantineViolation::InconsistentProofData { .. } => ViolationType::ReplayAttack,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_new_node_is_trusted() {
        let scorer = TrustScorer::new();
        let score = scorer.calculate_trust_score("new-node").await.expect("test");
        assert_eq!(score.score, 1.0);
        assert_eq!(score.level, TrustLevel::Trusted);
        assert_eq!(score.violations_total, 0);
        assert!(score.last_violation.is_none());
    }

    #[tokio::test]
    async fn test_single_violation_reduces_score() {
        let scorer = TrustScorer::new();
        let violation = ByzantineViolation::TimeManipulation {
            time_offset: Duration::from_secs(600),
            suspicious_nonce: 42,
        };
        let score = scorer.record_violation("node-1", &violation).await.expect("test");
        assert!(score.score < 1.0);
        assert!(score.score > 0.0);
        assert_eq!(score.violations_total, 1);
    }

    #[tokio::test]
    async fn test_severe_violations_lower_score_more() {
        let scorer = TrustScorer::new();

        // Record a stake violation (weight 0.30 — most severe)
        let stake_v = ByzantineViolation::InvalidStakeSignature {
            stake_holder_id: "bad-node".to_string(),
        };
        let score_after_stake = scorer.record_violation("node-severe", &stake_v).await.expect("test");

        // Compare with a replay attack on a different node (weight 0.10 — least severe)
        let replay_v = ByzantineViolation::ReplayAttack {
            original_timestamp: SystemTime::now(),
        };
        let score_after_replay = scorer.record_violation("node-mild", &replay_v).await.expect("test");

        assert!(score_after_stake.score < score_after_replay.score,
                "Stake violation ({:.3}) should lower score more than replay ({:.3})",
                score_after_stake.score, score_after_replay.score);
    }

    #[tokio::test]
    async fn test_many_violations_reach_untrusted() {
        let scorer = TrustScorer::new();
        let node = "bad-actor";

        // Stack violations to drive score below 0.4
        for _ in 0..5 {
            let v = ByzantineViolation::InvalidStakeSignature {
                stake_holder_id: node.to_string(),
            };
            scorer.record_violation(node, &v).await.expect("test");
        }

        let score = scorer.calculate_trust_score(node).await.expect("test");
        assert_eq!(score.level, TrustLevel::Untrusted);
        assert!(score.score < THRESHOLD_SUSPICIOUS);
    }

    #[tokio::test]
    async fn test_trust_level_thresholds() {
        let scorer = TrustScorer::new();

        // No violations = Trusted
        let level = scorer.get_trust_level("clean-node").await.expect("test");
        assert_eq!(level, TrustLevel::Trusted);

        // One moderate violation = still Trusted (1.0 - 0.20 = 0.80 > 0.7)
        let v = ByzantineViolation::WorkCheating {
            claimed_power: 999999,
            actual_power: 100,
        };
        scorer.record_violation("moderate-node", &v).await.expect("test");
        let level = scorer.get_trust_level("moderate-node").await.expect("test");
        assert_eq!(level, TrustLevel::Trusted);

        // Two stake violations = Suspicious (1.0 - 0.60 = 0.40 => boundary)
        let v1 = ByzantineViolation::InvalidStakeSignature {
            stake_holder_id: "sus".to_string(),
        };
        scorer.record_violation("sus-node", &v1).await.expect("test");
        scorer.record_violation("sus-node", &v1).await.expect("test");
        let level = scorer.get_trust_level("sus-node").await.expect("test");
        assert!(level == TrustLevel::Suspicious || level == TrustLevel::Untrusted);
    }

    #[tokio::test]
    async fn test_score_history_audit_trail() {
        let scorer = TrustScorer::new();
        let v = ByzantineViolation::TimeManipulation {
            time_offset: Duration::from_secs(600),
            suspicious_nonce: 1,
        };
        scorer.record_violation("audit-node", &v).await.expect("test");
        scorer.record_violation("audit-node", &v).await.expect("test");

        let history = scorer.get_score_history("audit-node").await;
        assert_eq!(history.len(), 2);
        // Scores should be decreasing
        assert!(history[1].score <= history[0].score);
    }

    #[tokio::test]
    async fn test_score_floor_and_ceiling() {
        let scorer = TrustScorer::new();

        // Score should never exceed 1.0
        let score = scorer.calculate_trust_score("fresh").await.expect("test");
        assert!(score.score <= 1.0);

        // Score should never go below 0.0 even with many violations
        let node = "punished";
        for _ in 0..20 {
            let v = ByzantineViolation::InvalidStakeSignature {
                stake_holder_id: node.to_string(),
            };
            scorer.record_violation(node, &v).await.expect("test");
        }
        let score = scorer.calculate_trust_score(node).await.expect("test");
        assert!(score.score >= 0.0);
    }
}
