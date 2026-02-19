// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Privacy Enforcement Types, Enums, and Constants
//!
//! Shared types, enums, and constants used throughout privacy enforcement.

use std::time::Duration;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::assets::core::PrivacyMode;

/// Trigger conditions for automated responses
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TriggerCondition {
    ViolationRateExceeded {
        rate_per_hour: f32,
    },
    SuspiciousAccessPattern {
        pattern_type: String,
        confidence_threshold: f32,
    },
    RiskScoreExceeded {
        threshold: f32,
    },
    UnauthorizedAccess {
        consecutive_failures: u32,
    },
    DataExposureRisk {
        risk_level: DataExposureRiskLevel,
    },
}

/// Data exposure risk levels
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DataExposureRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Enforcement actions that can be taken
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EnforcementAction {
    /// Log the violation
    LogViolation,
    /// Send warning to user
    SendWarning,
    /// Restrict resource access
    RestrictAccess {
        restriction_type: AccessRestriction,
        duration: Duration,
    },
    /// Suspend user account
    SuspendUser {
        duration: Duration,
    },
    /// Revoke resource allocation
    RevokeAllocation {
        allocation_id: String,
    },
    /// Reduce privacy level
    ReducePrivacyMode {
        new_level: PrivacyMode,
    },
    /// Require reauth
    RequireReauthentication,
    /// Escalate to administrator
    EscalateToAdmin,
    /// Emergency shutdown
    EmergencyShutdown,
}

/// Types of access restrictions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AccessRestriction {
    ReadOnly,
    NoNewAllocations,
    ReducedBandwidth,
    LimitedConcurrency,
    GeographicRestriction,
    TimeRestriction,
}

/// Recovery strategies
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    RestartService,
    ResetPermissions,
    RefreshCredentials,
    IsolateResource,
    EscalateToAdmin,
}

/// Types of recovery steps
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RecoveryStepType {
    Command,
    API,
    Manual,
    Validation,
    Notification,
}

/// Types of validation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ValidationType {
    HealthCheck,
    PermissionCheck,
    AccessTest,
    DataIntegrityCheck,
    PerformanceTest,
}

/// Types of pattern analysis algorithms
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AlgorithmType {
    StatisticalBaseline,
    MachineLearning,
    RuleBased,
    HeuristicAnalysis,
    BehavioralAnalysis,
}

/// Types of notification channels
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NotificationChannelType {
    Email,
    SMS,
    Webhook,
    Discord,
    Slack,
    InApp,
}

/// Data collection settings for monitoring
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataCollectionSettings {
    /// Collect access logs
    pub collect_access_logs: bool,

    /// Collect performance metrics
    pub collect_performance_metrics: bool,

    /// Collect network traffic patterns
    pub collect_traffic_patterns: bool,

    /// Data retention period
    pub retention_period: Duration,

    /// Anonymize collected data
    pub anonymize_data: bool,

    /// Encryption for stored data
    pub encrypt_stored_data: bool,
}

impl Default for DataCollectionSettings {
    fn default() -> Self {
        Self {
            collect_access_logs: true,
            collect_performance_metrics: true,
            collect_traffic_patterns: false,
            retention_period: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            anonymize_data: true,
            encrypt_stored_data: true,
        }
    }
}

/// Notification throttling settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationThrottling {
    /// Maximum notifications per time period
    pub max_notifications_per_period: u32,

    /// Time period for throttling
    pub throttling_period: Duration,

    /// Burst allowance
    pub burst_allowance: u32,

    /// Priority bypass threshold
    pub priority_bypass_threshold: super::super::manager::NotificationPriority,
}

impl Default for NotificationThrottling {
    fn default() -> Self {
        Self {
            max_notifications_per_period: 10,
            throttling_period: Duration::from_secs(60 * 60), // 1 hour
            burst_allowance: 3,
            priority_bypass_threshold: super::super::manager::NotificationPriority::High,
        }
    }
}

/// False positive reduction settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FalsePositiveReduction {
    /// Enable correlation analysis
    pub correlation_analysis: bool,

    /// Historical context window
    pub context_window: Duration,

    /// Whitelist known patterns
    pub whitelist_known_patterns: bool,

    /// User feedback integration
    pub user_feedback_integration: bool,
}

impl Default for FalsePositiveReduction {
    fn default() -> Self {
        Self {
            correlation_analysis: true,
            context_window: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
            whitelist_known_patterns: true,
            user_feedback_integration: true,
        }
    }
}

/// Validation criteria for recovery
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationCriteria {
    /// Validation type
    pub validation_type: ValidationType,

    /// Expected values
    pub expected_values: HashMap<String, String>,

    /// Tolerance thresholds
    pub tolerance: HashMap<String, f32>,
}

/// Recovery step details
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryStep {
    /// Step description
    pub description: String,

    /// Step type
    pub step_type: RecoveryStepType,

    /// Required parameters
    pub parameters: HashMap<String, String>,

    /// Expected outcome
    pub expected_outcome: String,
}

/// Pattern analysis algorithms
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatternAnalysisAlgorithm {
    /// Algorithm name
    pub name: String,

    /// Algorithm type
    pub algorithm_type: AlgorithmType,

    /// Configuration parameters
    pub parameters: HashMap<String, f32>,

    /// Detection thresholds
    pub thresholds: HashMap<String, f32>,
}

/// Risk level thresholds
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskLevelThresholds {
    /// Low risk threshold
    pub low: f32,

    /// Medium risk threshold
    pub medium: f32,

    /// High risk threshold
    pub high: f32,

    /// Critical risk threshold
    pub critical: f32,
}

impl Default for RiskLevelThresholds {
    fn default() -> Self {
        Self {
            low: 0.3,
            medium: 0.6,
            high: 0.8,
            critical: 0.95,
        }
    }
}

/// Anomaly categories for detection
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnomalyCategory {
    /// Category name
    pub name: String,

    /// Category description
    pub description: String,

    /// Detection patterns
    pub patterns: Vec<String>,

    /// Risk level
    pub risk_level: super::analysis::RiskLevel,
}

/// Notification channel configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationChannel {
    /// Channel type
    pub channel_type: NotificationChannelType,

    /// Channel configuration
    pub config: HashMap<String, String>,

    /// Priority threshold for this channel
    pub priority_threshold: super::super::manager::NotificationPriority,
}

/// Progressive escalation rules
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EscalationRule {
    /// Violation count threshold
    pub violation_threshold: u32,

    /// Time window for violation count
    pub time_window: Duration,

    /// Escalation action
    pub action: EnforcementAction,

    /// De-escalation conditions
    pub deescalation_conditions: Vec<String>,
}

/// Automated response triggers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutoResponseTrigger {
    /// Trigger condition
    pub condition: TriggerCondition,

    /// Response action
    pub response: EnforcementAction,

    /// Cooldown period
    pub cooldown: Duration,

    /// Maximum triggers per period
    pub max_triggers_per_period: u32,
}

/// Manual recovery procedure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManualRecoveryProcedure {
    /// Procedure name
    pub name: String,

    /// Violation types this applies to
    pub applicable_violations: Vec<super::violations::PrivacyViolationType>,

    /// Step-by-step instructions
    pub steps: Vec<RecoveryStep>,

    /// Required permissions
    pub required_permissions: Vec<String>,
}

/// Recovery validation step
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryValidationStep {
    /// Validation name
    pub name: String,

    /// Validation criteria
    pub criteria: ValidationCriteria,

    /// Required for recovery completion
    pub required: bool,

    /// Validation timeout
    pub timeout: Duration,
}

/// Automatic recovery configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutoRecoveryConfig {
    /// Enable automatic recovery
    pub enabled: bool,

    /// Recovery attempt limit
    pub max_attempts: u32,

    /// Recovery attempt interval
    pub attempt_interval: Duration,

    /// Recovery strategies
    pub strategies: Vec<RecoveryStrategy>,
}

impl Default for AutoRecoveryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 3,
            attempt_interval: Duration::from_secs(5 * 60), // 5 minutes
            strategies: vec![RecoveryStrategy::RestartService],
        }
    }
}

/// Recovery procedures after violations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryProcedures {
    /// Automatic recovery attempts
    pub auto_recovery: AutoRecoveryConfig,

    /// Manual recovery procedures
    pub manual_procedures: Vec<ManualRecoveryProcedure>,

    /// Recovery validation steps
    pub validation_steps: Vec<RecoveryValidationStep>,
}

impl Default for RecoveryProcedures {
    fn default() -> Self {
        Self {
            auto_recovery: AutoRecoveryConfig::default(),
            manual_procedures: Vec::new(),
            validation_steps: Vec::new(),
        }
    }
}
