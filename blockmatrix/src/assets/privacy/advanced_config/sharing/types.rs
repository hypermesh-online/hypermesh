// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Core sharing types - policies, workflows, anonymization, and risk management.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use super::reporting::{ReportDetailLevel, ReportingFrequency, RiskReportingRequirements};

/// Data sharing minimization settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharingMinimizationSettings {
    /// Default sharing policy
    pub default_policy: SharingPolicy,
    /// Per-recipient sharing rules
    pub per_recipient_rules: HashMap<String, SharingPolicy>,
    /// Data category sharing preferences
    pub category_preferences: HashMap<String, SharingPreference>,
    /// Sharing approval workflows
    pub approval_workflows: Vec<SharingApprovalWorkflow>,
}

/// Data sharing policies
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SharingPolicy {
    NoSharing,
    MinimalSharing,
    ContextualSharing,
    StandardSharing,
    MaximalSharing,
}

/// Sharing preference for data categories
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharingPreference {
    pub allow_sharing: bool,
    pub anonymization_required: bool,
    pub purpose_limitations: Vec<String>,
    pub retention_limitations: Duration,
    pub geographic_limitations: Vec<String>,
}

/// Sharing approval workflow
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharingApprovalWorkflow {
    pub name: String,
    pub triggers: Vec<SharingTrigger>,
    pub approval_steps: Vec<ApprovalStep>,
    pub default_action: SharingAction,
    pub timeout: Duration,
}

/// Sharing approval triggers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharingTrigger {
    pub trigger_type: SharingTriggerType,
    pub conditions: HashMap<String, String>,
    pub priority: u32,
}

/// Types of sharing triggers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SharingTriggerType {
    RecipientType,
    DataSensitivity,
    Purpose,
    Geographic,
    Temporal,
}

/// Approval step in workflow
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApprovalStep {
    pub name: String,
    pub approvers: Vec<ApproverRequirement>,
    pub timeout: Duration,
    pub consensus_level: ConsensusLevel,
    pub conditions: Vec<String>,
}

/// Approver requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApproverRequirement {
    pub approver_type: ApproverType,
    pub qualifications: Vec<String>,
    pub alternatives: Vec<String>,
    pub escalation: Vec<String>,
}

/// Types of approvers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ApproverType {
    DataOwner,
    PrivacyOfficer,
    Administrator,
    LegalCounsel,
    ComplianceOfficer,
    ExternalAuditor,
}

/// Consensus levels for approval
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsensusLevel {
    Unanimous,
    Majority,
    Plurality,
    SingleApprover,
}

/// Sharing actions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SharingAction {
    Allow,
    Deny,
    Conditional,
    Escalate,
    Defer,
}

/// Anonymization preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnonymizationPreferences {
    pub preferred_techniques: Vec<AnonymizationTechnique>,
    pub strength_preferences: AnonymizationStrengthPreferences,
    pub risk_tolerance: ReidentificationRiskTolerance,
    pub utility_requirements: UtilityPreservationRequirements,
}

/// Anonymization techniques
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AnonymizationTechnique {
    KAnonymity,
    LDiversity,
    TCloseness,
    DifferentialPrivacy,
    DataMasking,
    Pseudonymization,
    Generalization,
    Suppression,
}

/// Anonymization strength preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnonymizationStrengthPreferences {
    pub k_anonymity_level: Option<u32>,
    pub l_diversity_requirements: Option<u32>,
    pub t_closeness_requirements: Option<f32>,
    pub differential_privacy: Option<DifferentialPrivacyPreferences>,
}

/// Differential privacy preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DifferentialPrivacyPreferences {
    pub epsilon: f32,
    pub delta: f32,
    pub sensitivity_tolerance: f32,
    pub noise_distribution: NoiseDistribution,
}

/// Noise distribution options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NoiseDistribution {
    Laplacian,
    Gaussian,
    Exponential,
    Custom { parameters: HashMap<String, f32> },
}

/// Re-identification risk tolerance
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReidentificationRiskTolerance {
    pub max_risk_level: f32,
    pub assessment_frequency: Duration,
    pub mitigation_preferences: Vec<RiskMitigationStrategy>,
    pub monitoring_requirements: RiskMonitoringRequirements,
}

/// Risk mitigation strategies
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RiskMitigationStrategy {
    IncreaseAnonymization,
    ReduceDataSharing,
    ImproveAccessControls,
    EnhanceMonitoring,
    SeekExpertReview,
}

/// Risk monitoring requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskMonitoringRequirements {
    pub continuous_monitoring: bool,
    pub alert_thresholds: HashMap<String, f32>,
    pub automated_responses: Vec<AutomatedRiskResponse>,
    pub reporting_requirements: RiskReportingRequirements,
}

/// Automated risk responses
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutomatedRiskResponse {
    pub trigger: RiskResponseTrigger,
    pub action: RiskResponseAction,
    pub delay: Duration,
    pub confirmation_required: bool,
}

/// Risk response triggers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RiskResponseTrigger {
    ThresholdExceeded { metric: String, threshold: f32 },
    TrendDetected { trend_type: String },
    AnomalyDetected { confidence: f32 },
    ExternalThreat { threat_level: String },
}

/// Risk response actions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RiskResponseAction {
    IncreasePrivacy,
    RestrictAccess,
    NotifyUser,
    EscalateToAdmin,
    ActivateContingency,
}

/// Utility preservation requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UtilityPreservationRequirements {
    pub min_utility_threshold: f32,
    pub utility_metrics: Vec<UtilityMetric>,
    pub tradeoff_preferences: UtilityTradeoffPreferences,
    pub quality_assessment: QualityAssessmentRequirements,
}

/// Utility metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UtilityMetric {
    pub name: String,
    pub metric_type: UtilityMetricType,
    pub min_value: f32,
    pub target_value: f32,
    pub measurement_frequency: Duration,
}

/// Types of utility metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UtilityMetricType {
    Accuracy,
    Precision,
    Recall,
    F1Score,
    AUC,
    InformationLoss,
    CustomMetric { definition: String },
}

/// Utility trade-off preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UtilityTradeoffPreferences {
    pub privacy_utility_weight: f32,
    pub acceptable_utility_loss: f32,
    pub adaptive_adjustment: AdaptiveAdjustmentSettings,
}

/// Adaptive adjustment settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptiveAdjustmentSettings {
    pub enabled: bool,
    pub triggers: Vec<AdjustmentTrigger>,
    pub limits: AdjustmentLimits,
    pub learning_parameters: HashMap<String, f32>,
}

/// Adjustment triggers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdjustmentTrigger {
    pub condition: AdjustmentCondition,
    pub threshold: f32,
    pub direction: AdjustmentDirection,
    pub magnitude: f32,
}

/// Adjustment conditions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AdjustmentCondition {
    UtilityBelowThreshold,
    PrivacyAboveThreshold,
    RiskExceedsLimit,
    UserFeedback,
    PerformanceMetric,
}

/// Adjustment directions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AdjustmentDirection {
    IncreasePrivacy,
    DecreasePrivacy,
    IncreaseUtility,
    DecreaseUtility,
    Balanced,
}

/// Adjustment limits
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdjustmentLimits {
    pub max_privacy_adjustment: f32,
    pub max_utility_adjustment: f32,
    pub frequency_limits: HashMap<String, Duration>,
}

/// Quality assessment requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityAssessmentRequirements {
    pub metrics: Vec<QualityMetric>,
    pub assessment_frequency: Duration,
    pub thresholds: QualityThresholds,
    pub reporting: QualityReportingRequirements,
}

/// Quality metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityMetric {
    pub name: String,
    pub measurement_method: QualityMeasurementMethod,
    pub expected_range: (f32, f32),
    pub critical_threshold: f32,
}

/// Quality measurement methods
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QualityMeasurementMethod {
    Automated,
    ManualReview,
    UserFeedback,
    ExpertEvaluation,
    BenchmarkComparison,
}

/// Quality thresholds
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityThresholds {
    pub minimum_quality: f32,
    pub target_quality: f32,
    pub degradation_tolerance: f32,
    pub assessment_methods: Vec<QualityAssessmentMethod>,
}

/// Quality assessment methods
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QualityAssessmentMethod {
    Statistical,
    MachineLearning,
    UserStudy,
    ExpertReview,
    BenchmarkTesting,
}

/// Quality reporting requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityReportingRequirements {
    pub frequency: ReportingFrequency,
    pub recipients: Vec<String>,
    pub detail_level: ReportDetailLevel,
    pub alert_conditions: Vec<String>,
}
