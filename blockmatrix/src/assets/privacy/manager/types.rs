// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Privacy manager type definitions.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use super::super::{
    PrivacyAllocationResult, ResourceAllocationConfig, ConsensusRequirementConfig,
    CaesarRewardConfig,
};
use crate::assets::core::PrivacyMode;
use crate::assets::proxy::RemoteProxyManager;

/// Main privacy manager configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacyManagerConfig {
    /// Default privacy level for new users
    pub default_privacy_level: PrivacyMode,

    /// Default resource allocation percentages
    pub default_resource_allocation: ResourceAllocationConfig,

    /// Global consensus requirements
    pub global_consensus_requirements: ConsensusRequirementConfig,

    /// CAESAR reward base configuration
    pub base_reward_config: CaesarRewardConfig,

    /// Proxy system integration
    pub proxy_integration_enabled: bool,

    /// Privacy enforcement strictness
    pub enforcement_strictness: EnforcementStrictness,

    /// Audit logging configuration
    pub audit_logging: AuditLoggingConfig,
}

/// Privacy enforcement strictness levels
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EnforcementStrictness {
    /// Warnings only, allow violations
    Permissive,
    /// Block violations but allow overrides
    Moderate,
    /// Strict enforcement, no overrides
    Strict,
}

/// Audit logging configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditLoggingConfig {
    /// Enable privacy audit logging
    pub enabled: bool,

    /// Log all privacy events
    pub log_all_events: bool,

    /// Log only violations and changes
    pub log_violations_only: bool,

    /// Retention period for audit logs
    pub retention_period: Duration,

    /// Anonymize logged data
    pub anonymize_logs: bool,
}

impl Default for PrivacyManagerConfig {
    fn default() -> Self {
        Self {
            default_privacy_level: PrivacyMode::PRIVATE,
            default_resource_allocation: ResourceAllocationConfig::default(),
            global_consensus_requirements: ConsensusRequirementConfig::default(),
            base_reward_config: CaesarRewardConfig::default(),
            proxy_integration_enabled: true,
            enforcement_strictness: EnforcementStrictness::Moderate,
            audit_logging: AuditLoggingConfig::default(),
        }
    }
}

impl Default for AuditLoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_all_events: false,
            log_violations_only: true,
            retention_period: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            anonymize_logs: false,
        }
    }
}

impl Default for CaesarRewardConfig {
    fn default() -> Self {
        Self {
            base_reward_rate: 1.0,
            privacy_multiplier: 1.0,
            utilization_multiplier: 1.0,
            consensus_bonus: 0.1,
            max_reward_cap: 1000.0,
            distribution_config: super::super::RewardDistributionConfig::default(),
        }
    }
}

impl Default for super::super::RewardDistributionConfig {
    fn default() -> Self {
        Self {
            immediate_payout: false,
            immediate_percentage: 0.5,
            auto_stake_remainder: true,
            minimum_payout_threshold: 10.0,
            payout_frequency: super::super::PayoutFrequency::Daily,
        }
    }
}

/// Core privacy manager
pub struct PrivacyManager {
    /// Manager configuration
    pub(crate) config: PrivacyManagerConfig,

    /// User privacy configurations
    pub(crate) user_configs: Arc<RwLock<HashMap<String, UserPrivacyConfiguration>>>,

    /// Active privacy allocations
    pub(crate) active_allocations: Arc<RwLock<HashMap<String, PrivacyAllocationResult>>>,

    /// Remote proxy manager reference
    pub(crate) _proxy_manager: Option<Arc<RemoteProxyManager>>,

    /// Privacy enforcement engine
    pub(crate) enforcer: Arc<super::super::PrivacyEnforcer>,

    /// CAESAR reward calculator
    pub(crate) reward_calculator: Arc<super::super::CaesarRewardCalculator>,

    /// Privacy audit logger
    pub(crate) audit_logger: Arc<RwLock<Vec<PrivacyAuditEntry>>>,
}

/// User-specific privacy configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserPrivacyConfiguration {
    /// User identifier
    pub user_id: String,

    /// User's preferred privacy level
    pub preferred_privacy_level: PrivacyMode,

    /// Per-resource privacy settings
    pub resource_privacy_settings: HashMap<String, ResourcePrivacyConfig>,

    /// Consensus proof requirements
    pub consensus_requirements: ConsensusRequirementConfig,

    /// CAESAR reward preferences
    pub reward_preferences: CaesarRewardPreferences,

    /// Proxy addressing preferences
    pub proxy_preferences: ProxyPreferences,

    /// Allocation constraints
    pub allocation_constraints: AllocationConstraints,

    /// Privacy history and learning
    pub privacy_history: PrivacyHistory,
}

/// Resource-specific privacy configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourcePrivacyConfig {
    /// Resource type (cpu, gpu, memory, storage, network)
    pub resource_type: String,

    /// Privacy level for this resource
    pub privacy_level: PrivacyMode,

    /// Allocation percentage (0.0 - 1.0)
    pub allocation_percentage: f32,

    /// Maximum concurrent access
    pub max_concurrent_access: u32,

    /// Duration limits
    pub duration_limits: super::super::DurationLimits,

    /// Special access rules
    pub access_rules: Vec<AccessRule>,
}

/// Access rule for resources
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessRule {
    /// Rule identifier
    pub rule_id: String,
    /// Rule type
    pub rule_type: AccessRuleType,
    /// Condition for rule activation
    pub condition: AccessCondition,
    /// Action to take when rule matches
    pub action: AccessAction,
    /// Priority level (higher = more important)
    pub priority: u32,
}

/// Types of access rules
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AccessRuleType {
    Allow,
    Deny,
    Restrict,
    Redirect,
    Monitor,
}

/// Access condition specification
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessCondition {
    pub time_conditions: Option<TimeConditions>,
    pub network_conditions: Option<NetworkConditions>,
    pub user_conditions: Option<UserConditions>,
    pub resource_conditions: Option<ResourceConditions>,
}

/// Time-based access conditions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeConditions {
    pub allowed_hours: Vec<(u8, u8)>,
    pub allowed_days: Vec<u8>,
    pub usage_limits: Vec<UsageLimit>,
    pub timezone: String,
}

/// Usage limit specification
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UsageLimit {
    pub period: Duration,
    pub max_usage: Duration,
    pub reset_behavior: ResetBehavior,
}

/// Reset behavior for usage limits
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ResetBehavior {
    Rolling,
    Fixed,
    Manual,
}

/// Network-based access conditions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkConditions {
    pub allowed_networks: Vec<String>,
    pub denied_networks: Vec<String>,
    pub geographic_restrictions: Vec<String>,
    pub vpn_restrictions: VpnRestrictions,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VpnRestrictions {
    pub allow_vpn: bool,
    pub allow_proxy: bool,
    pub allow_tor: bool,
    pub required_vpn_providers: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserConditions {
    pub required_groups: Vec<String>,
    pub require_authentication: bool,
    pub required_certificates: Vec<String>,
    pub mfa_requirements: MfaRequirements,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MfaRequirements {
    pub required: bool,
    pub accepted_methods: Vec<String>,
    pub validity_period: Duration,
    pub allow_trusted_devices: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceConditions {
    pub min_available_resources: HashMap<String, f32>,
    pub max_utilization: f32,
    pub required_capabilities: Vec<String>,
    pub performance_thresholds: PerformanceThresholds,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    pub max_latency_ms: u32,
    pub min_bandwidth_mbps: u32,
    pub min_success_rate: f32,
    pub max_error_rate: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessAction {
    pub action_type: AccessActionType,
    pub parameters: HashMap<String, String>,
    pub notifications: NotificationSettings,
    pub logging: ActionLoggingConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AccessActionType {
    Allow, Deny, Redirect, Throttle, Queue, Authenticate, Log, Alert,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub notify_user: bool,
    pub notify_admin: bool,
    pub channels: Vec<String>,
    pub priority: NotificationPriority,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NotificationPriority { Low, Medium, High, Critical }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionLoggingConfig {
    pub enabled: bool,
    pub log_level: LogLevel,
    pub include_sensitive_data: bool,
    pub message_template: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LogLevel { Debug, Info, Warn, Error }

/// CAESAR reward preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaesarRewardPreferences {
    pub enabled: bool,
    pub minimum_reward_rate: f32,
    pub payout_frequency: super::super::PayoutFrequency,
    pub auto_stake_percentage: f32,
    pub optimization_preferences: RewardOptimizationPreferences,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RewardOptimizationPreferences {
    pub optimize_for_maximum_rewards: bool,
    pub balance_rewards_privacy: bool,
    pub reward_privacy_ratio: f32,
    pub accept_dynamic_adjustments: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxyPreferences {
    pub enabled: bool,
    pub preferred_proxy_types: Vec<String>,
    pub geographic_preferences: Vec<String>,
    pub trust_requirements: super::super::TrustRequirements,
    pub performance_requirements: ProxyPerformanceRequirements,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxyPerformanceRequirements {
    pub max_latency_ms: u32,
    pub min_bandwidth_mbps: u32,
    pub min_uptime_percentage: f32,
    pub max_connection_time_ms: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AllocationConstraints {
    pub max_total_allocations: u32,
    pub max_per_resource_type: HashMap<String, u32>,
    pub max_allocation_duration: Duration,
    pub allocation_cooldown: Duration,
    pub budget_constraints: BudgetConstraints,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BudgetConstraints {
    pub max_tokens_per_allocation: f32,
    pub max_tokens_per_period: f32,
    pub budget_period: Duration,
    pub auto_renewal_budget: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacyHistory {
    pub total_allocations: u64,
    pub privacy_level_usage: HashMap<String, u64>,
    pub resource_usage: HashMap<String, ResourceUsageStats>,
    pub violations: Vec<PrivacyViolationRecord>,
    pub preference_evolution: Vec<PrivacyPreferenceChange>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceUsageStats {
    pub total_usage_time: Duration,
    pub total_data_transferred: u64,
    pub average_utilization: f32,
    pub peak_utilization: f32,
    pub usage_patterns: Vec<UsagePattern>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UsagePattern {
    pub pattern_name: String,
    pub frequency: f32,
    pub time_pattern: TimePattern,
    pub allocation_pattern: HashMap<String, f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimePattern {
    pub preferred_hours: Vec<u8>,
    pub preferred_days: Vec<u8>,
    pub seasonal_preferences: Vec<String>,
    pub typical_duration: Duration,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacyViolationRecord {
    pub timestamp: SystemTime,
    pub violation_type: String,
    pub severity: ViolationSeverity,
    pub description: String,
    pub resolution: Option<String>,
    pub impact_assessment: ImpactAssessment,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ViolationSeverity { Low, Medium, High, Critical }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImpactAssessment {
    pub data_exposure: DataExposureLevel,
    pub affected_users: u32,
    pub exposure_duration: Duration,
    pub potential_consequences: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DataExposureLevel { None, Minimal, Moderate, Significant, Severe }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacyPreferenceChange {
    pub timestamp: SystemTime,
    pub changed_setting: String,
    pub old_value: String,
    pub new_value: String,
    pub reason: Option<String>,
    pub impact: ChangeImpact,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChangeImpact {
    pub reward_rate_delta: f32,
    pub privacy_level_change: i8,
    pub access_restrictions_change: i8,
    pub performance_impact: f32,
}

/// Privacy audit log entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacyAuditEntry {
    pub timestamp: SystemTime,
    pub user_id: Option<String>,
    pub event_type: PrivacyEventType,
    pub details: HashMap<String, String>,
    pub severity: LogLevel,
    pub allocation_id: Option<String>,
}

/// Privacy event types for audit logging
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PrivacyEventType {
    AllocationCreated,
    AllocationModified,
    AllocationExpired,
    AccessGranted,
    AccessDenied,
    PrivacyViolation,
    ConsentUpdated,
    DataAccessed,
    DataShared,
    ConfigurationChanged,
}
