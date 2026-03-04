// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Privacy transition validation and impact assessment types

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

use super::PrivacyAllocationType;
use crate::assets::core::AssetResult;

/// Privacy transition validation and management
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacyTransition {
    /// Current allocation type
    pub from_type: PrivacyAllocationType,

    /// Target allocation type
    pub to_type: PrivacyAllocationType,

    /// Transition timestamp
    pub transition_time: SystemTime,

    /// Transition reason
    pub reason: String,

    /// Validation requirements
    pub validation_requirements: TransitionValidation,

    /// Transition impact
    pub impact_assessment: TransitionImpact,
}

/// Validation requirements for privacy transitions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransitionValidation {
    /// Require user consent
    pub require_user_consent: bool,

    /// Require state proof
    pub require_state_proof: bool,

    /// Require administrator approval
    pub require_admin_approval: bool,

    /// Cooling off period
    pub cooling_off_period: Duration,

    /// Validation criteria
    pub validation_criteria: Vec<ValidationCriterion>,
}

/// Validation criteria for transitions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationCriterion {
    /// Criterion name
    pub name: String,

    /// Criterion type
    pub criterion_type: ValidationCriterionType,

    /// Required value or threshold
    pub required_value: String,

    /// Validation method
    pub validation_method: String,
}

/// Types of validation criteria
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ValidationCriterionType {
    AuthenticationCheck,
    StakeAmount,
    HistoryCheck,
    PerformanceMetric,
    SecurityCheck,
    ComplianceCheck,
}

/// Impact assessment for privacy transitions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransitionImpact {
    /// Privacy level change impact
    pub privacy_impact: PrivacyImpact,

    /// Performance impact
    pub performance_impact: PerformanceImpact,

    /// Security impact
    pub security_impact: SecurityImpact,

    /// Economic impact
    pub economic_impact: EconomicImpact,
}

/// Privacy impact assessment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacyImpact {
    /// Privacy increase/decrease
    pub privacy_delta: i8,

    /// Anonymity change
    pub anonymity_change: AnonymityChange,

    /// Data exposure change
    pub exposure_change: ExposureChange,
}

/// Anonymity change assessment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AnonymityChange {
    Increased,
    Decreased,
    NoChange,
}

/// Data exposure change assessment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExposureChange {
    Increased,
    Decreased,
    NoChange,
}

/// Performance impact assessment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceImpact {
    /// Latency change (ms)
    pub latency_delta: i32,

    /// Throughput change (%)
    pub throughput_delta: f32,

    /// Reliability change
    pub reliability_delta: f32,
}

/// Security impact assessment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityImpact {
    /// Security level change
    pub security_level_delta: i8,

    /// Attack surface change
    pub attack_surface_change: AttackSurfaceChange,

    /// Compliance impact
    pub compliance_impact: Vec<String>,
}

/// Attack surface change assessment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AttackSurfaceChange {
    Increased,
    Decreased,
    NoChange,
}

/// Economic impact assessment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EconomicImpact {
    /// Reward rate change
    pub reward_rate_delta: f32,

    /// Cost change
    pub cost_delta: f32,

    /// Stake requirement change
    pub stake_requirement_delta: i64,
}

impl PrivacyTransition {
    /// Validate if transition is allowed and safe
    pub async fn validate_transition(&self) -> AssetResult<bool> {
        // Check if base transition is allowed
        if !self.from_type.can_transition_to(&self.to_type) {
            return Ok(false);
        }

        // Validate specific requirements
        if self.validation_requirements.require_state_proof {
            // Check if state proof is available and valid
            // This would integrate with the state proof system
        }

        if self.validation_requirements.require_user_consent {
            // Check if user consent has been obtained
            // This would integrate with user consent management
        }

        // Validate each criterion
        for criterion in &self.validation_requirements.validation_criteria {
            if !self.validate_criterion(criterion).await? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn validate_criterion(&self, _criterion: &ValidationCriterion) -> AssetResult<bool> {
        // Implementation would validate specific criteria
        // For now, return true as placeholder
        Ok(true)
    }
}
