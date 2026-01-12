//! Privacy Enforcement Configuration
//!
//! Configuration types, enums, and constants for privacy enforcement.

use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::assets::core::PrivacyLevel;

/// Privacy enforcement configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacyEnforcementConfig {
    /// Enforcement strictness level
    pub strictness: super::super::manager::EnforcementStrictness,

    /// Real-time monitoring settings
    pub realtime_monitoring: RealtimeMonitoringConfig,

    /// Violation response settings
    pub violation_response: ViolationResponseConfig,

    /// Access pattern analysis settings
    pub pattern_analysis: PatternAnalysisConfig,

    /// Risk assessment thresholds
    pub risk_thresholds: RiskThresholdConfig,
}

/// Real-time monitoring configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RealtimeMonitoringConfig {
    /// Enable real-time privacy monitoring
    pub enabled: bool,

    /// Monitoring frequency
    pub monitoring_frequency: Duration,

    /// Alert thresholds
    pub alert_thresholds: HashMap<String, f32>,

    /// Automated response triggers
    pub auto_response_triggers: Vec<AutoResponseTrigger>,

    /// Data collection settings
    pub data_collection: DataCollectionSettings,
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

/// Violation response configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViolationResponseConfig {
    /// Immediate response actions
    pub immediate_responses: Vec<EnforcementAction>,

    /// Progressive response escalation
    pub escalation_rules: Vec<EscalationRule>,

    /// Notification settings
    pub notifications: ViolationNotificationConfig,

    /// Recovery procedures
    pub recovery_procedures: RecoveryProcedures,
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

/// Violation notification configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViolationNotificationConfig {
    /// Notify user of violations
    pub notify_user: bool,

    /// Notify administrators
    pub notify_admin: bool,

    /// Real-time notifications
    pub realtime_notifications: bool,

    /// Notification channels
    pub channels: Vec<NotificationChannel>,

    /// Notification throttling
    pub throttling: NotificationThrottling,
}

/// Notification channels
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationChannel {
    /// Channel type
    pub channel_type: NotificationChannelType,

    /// Channel configuration
    pub config: HashMap<String, String>,

    /// Priority threshold for this channel
    pub priority_threshold: super::super::manager::NotificationPriority,
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

/// Recovery strategies
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    RestartService,
    ResetPermissions,
    RefreshCredentials,
    IsolateResource,
    EscalateToAdmin,
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

/// Types of recovery steps
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RecoveryStepType {
    Command,
    API,
    Manual,
    Validation,
    Notification,
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

/// Types of validation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ValidationType {
    HealthCheck,
    PermissionCheck,
    AccessTest,
    DataIntegrityCheck,
    PerformanceTest,
}

/// Access pattern analysis configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatternAnalysisConfig {
    /// Enable pattern analysis
    pub enabled: bool,

    /// Analysis algorithms
    pub algorithms: Vec<PatternAnalysisAlgorithm>,

    /// Learning period
    pub learning_period: Duration,

    /// Anomaly detection settings
    pub anomaly_detection: AnomalyDetectionConfig,

    /// Baseline update frequency
    pub baseline_update_frequency: Duration,
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

/// Types of pattern analysis algorithms
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AlgorithmType {
    StatisticalBaseline,
    MachineLearning,
    RuleBased,
    HeuristicAnalysis,
    BehavioralAnalysis,
}

/// Anomaly detection configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnomalyDetectionConfig {
    /// Detection sensitivity
    pub sensitivity: f32,

    /// Minimum confidence threshold
    pub min_confidence: f32,

    /// False positive reduction settings
    pub false_positive_reduction: FalsePositiveReduction,

    /// Anomaly categories
    pub categories: Vec<AnomalyCategory>,
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

/// Risk assessment threshold configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskThresholdConfig {
    /// Privacy risk thresholds
    pub privacy_risk: RiskLevelThresholds,

    /// Security risk thresholds
    pub security_risk: RiskLevelThresholds,

    /// Compliance risk thresholds
    pub compliance_risk: RiskLevelThresholds,

    /// Operational risk thresholds
    pub operational_risk: RiskLevelThresholds,
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
    ReducePrivacyLevel {
        new_level: PrivacyLevel,
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

// Default implementations
impl Default for RealtimeMonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            monitoring_frequency: Duration::from_secs(60),
            alert_thresholds: HashMap::new(),
            auto_response_triggers: Vec::new(),
            data_collection: DataCollectionSettings::default(),
        }
    }
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

impl Default for ViolationResponseConfig {
    fn default() -> Self {
        Self {
            immediate_responses: vec![EnforcementAction::LogViolation],
            escalation_rules: Vec::new(),
            notifications: ViolationNotificationConfig::default(),
            recovery_procedures: RecoveryProcedures::default(),
        }
    }
}

impl Default for ViolationNotificationConfig {
    fn default() -> Self {
        Self {
            notify_user: true,
            notify_admin: true,
            realtime_notifications: true,
            channels: Vec::new(),
            throttling: NotificationThrottling::default(),
        }
    }
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

impl Default for RecoveryProcedures {
    fn default() -> Self {
        Self {
            auto_recovery: AutoRecoveryConfig::default(),
            manual_procedures: Vec::new(),
            validation_steps: Vec::new(),
        }
    }
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

impl Default for PatternAnalysisConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithms: Vec::new(),
            learning_period: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
            anomaly_detection: AnomalyDetectionConfig::default(),
            baseline_update_frequency: Duration::from_secs(24 * 60 * 60), // 24 hours
        }
    }
}

impl Default for AnomalyDetectionConfig {
    fn default() -> Self {
        Self {
            sensitivity: 0.8,
            min_confidence: 0.7,
            false_positive_reduction: FalsePositiveReduction::default(),
            categories: Vec::new(),
        }
    }
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

impl Default for RiskThresholdConfig {
    fn default() -> Self {
        Self {
            privacy_risk: RiskLevelThresholds {
                low: 0.3,
                medium: 0.6,
                high: 0.8,
                critical: 0.95,
            },
            security_risk: RiskLevelThresholds {
                low: 0.2,
                medium: 0.5,
                high: 0.75,
                critical: 0.9,
            },
            compliance_risk: RiskLevelThresholds {
                low: 0.25,
                medium: 0.55,
                high: 0.8,
                critical: 0.95,
            },
            operational_risk: RiskLevelThresholds {
                low: 0.4,
                medium: 0.7,
                high: 0.85,
                critical: 0.95,
            },
        }
    }
}
