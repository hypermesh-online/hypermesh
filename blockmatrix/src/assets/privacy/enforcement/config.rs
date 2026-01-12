//! Privacy Enforcement Configuration
//!
//! Main configuration types for privacy enforcement.

use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::assets::core::PrivacyLevel;
use super::types::{
    DataCollectionSettings, NotificationThrottling, AutoRecoveryConfig,
    RecoveryProcedures, PatternAnalysisAlgorithm, RiskLevelThresholds,
    AnomalyCategory, NotificationChannel, EscalationRule, AutoResponseTrigger,
    EnforcementAction, FalsePositiveReduction,
};

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

    /// Alert thresholds (keyed by alert type)
    pub alert_thresholds: std::collections::HashMap<String, f32>,

    /// Automated response triggers
    pub auto_response_triggers: Vec<AutoResponseTrigger>,

    /// Data collection settings
    pub data_collection: DataCollectionSettings,
}

impl Default for RealtimeMonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            monitoring_frequency: Duration::from_secs(60),
            alert_thresholds: std::collections::HashMap::new(),
            auto_response_triggers: Vec::new(),
            data_collection: DataCollectionSettings::default(),
        }
    }
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
