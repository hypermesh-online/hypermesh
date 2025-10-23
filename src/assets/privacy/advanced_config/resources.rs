//! Resource Privacy Configuration
//!
//! Configuration for resource-specific privacy settings and allocation optimization.

use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::assets::core::{AssetResult, AssetError};

/// Resource privacy settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourcePrivacySettings {
    /// Default resource privacy level
    pub default_privacy_level: String,
    
    /// Per-resource-type settings
    pub per_type_settings: HashMap<String, ResourceTypePrivacySettings>,
    
    /// Resource grouping settings
    pub grouping_settings: ResourceGroupingSettings,
    
    /// Resource allocation optimization
    pub allocation_optimization: ResourceAllocationOptimization,
}

/// Privacy settings for specific resource types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceTypePrivacySettings {
    /// Privacy level for this resource type
    pub privacy_level: String,
    
    /// Privacy rules
    pub privacy_rules: Vec<ResourcePrivacyRule>,
    
    /// Performance settings
    pub performance_settings: ResourcePerformanceSettings,
    
    /// Quality of service settings
    pub qos_settings: QualityOfServiceSettings,
}

/// Resource privacy rule
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourcePrivacyRule {
    /// Rule name
    pub name: String,
    
    /// Rule condition
    pub condition: ResourceRuleCondition,
    
    /// Rule action
    pub action: ResourceRuleAction,
    
    /// Rule priority
    pub priority: u32,
    
    /// Rule enabled
    pub enabled: bool,
}

/// Resource rule conditions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ResourceRuleCondition {
    UsageThreshold(f32),
    TimeWindow(Duration),
    UserType(String),
    GeographicLocation(String),
    NetworkCondition(String),
}

/// Resource rule actions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ResourceRuleAction {
    AllowAccess,
    DenyAccess,
    RequireApproval,
    ApplyLimitations,
    LogActivity,
}

/// Resource performance settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourcePerformanceSettings {
    /// Performance priority
    pub priority: PerformancePriority,
    
    /// Resource limits
    pub limits: HashMap<String, String>,
    
    /// Monitoring settings
    pub monitoring: PerformanceMonitoringSettings,
    
    /// Optimization preferences
    pub optimization_preferences: Vec<String>,
}

/// Performance priority levels
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PerformancePriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Quality of service settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityOfServiceSettings {
    /// Minimum QoS guarantees
    pub min_guarantees: HashMap<String, f32>,
    
    /// Maximum allowed latency
    pub max_latency: Duration,
    
    /// Bandwidth requirements
    pub bandwidth_requirements: HashMap<String, u64>,
    
    /// Reliability requirements
    pub reliability_requirements: f32,
}

/// Performance monitoring settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceMonitoringSettings {
    /// Enable monitoring
    pub enabled: bool,
    
    /// Monitoring frequency
    pub frequency: Duration,
    
    /// Metrics to collect
    pub metrics: Vec<PerformanceMetric>,
    
    /// Alert thresholds
    pub alert_thresholds: HashMap<String, f32>,
}

/// Performance metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PerformanceMetric {
    Latency,
    Throughput,
    ErrorRate,
    Availability,
    ResourceUtilization,
}

/// Resource grouping settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceGroupingSettings {
    /// Grouping strategy
    pub strategy: ResourceGroupingStrategy,
    
    /// Group policies
    pub group_policies: Vec<ResourceGroupPolicy>,
    
    /// Cross-resource policies
    pub cross_resource_policies: Vec<CrossResourcePolicy>,
}

/// Resource grouping strategies
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ResourceGroupingStrategy {
    ByType,
    ByLocation,
    ByUser,
    ByPerformance,
    Custom,
}

/// Resource group policy
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceGroupPolicy {
    /// Policy name
    pub name: String,
    
    /// Group criteria
    pub criteria: HashMap<String, String>,
    
    /// Policy inheritance
    pub inheritance: PolicyInheritance,
    
    /// Policy priority
    pub priority: u32,
}

/// Policy inheritance options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PolicyInheritance {
    Inherit,
    Override,
    Merge,
    Isolate,
}

/// Cross-resource policy
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrossResourcePolicy {
    /// Policy name
    pub name: String,
    
    /// Resource types affected
    pub resource_types: Vec<String>,
    
    /// Policy conditions
    pub conditions: Vec<CrossResourceCondition>,
    
    /// Policy actions
    pub actions: Vec<CrossResourceAction>,
}

/// Cross-resource conditions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CrossResourceCondition {
    CombinedUsage(f32),
    ConflictDetection,
    DependencyViolation,
    PolicyConflict,
}

/// Cross-resource actions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CrossResourceAction {
    CoordinateAccess,
    PrioritizeAccess,
    LoadBalance,
    IsolateResources,
}

/// Resource allocation optimization
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceAllocationOptimization {
    /// Enable optimization
    pub enabled: bool,
    
    /// Optimization objectives
    pub objectives: Vec<OptimizationObjective>,
    
    /// Optimization constraints
    pub constraints: Vec<OptimizationConstraint>,
    
    /// Optimization frequency
    pub frequency: Duration,
}

/// Optimization objectives
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OptimizationObjective {
    MinimizeLatency,
    MaximizeThroughput,
    MinimizeCost,
    MaximizePrivacy,
    BalancePerformance,
}

/// Optimization constraints
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptimizationConstraint {
    /// Constraint type
    pub constraint_type: String,
    
    /// Constraint value
    pub value: f32,
    
    /// Constraint priority
    pub priority: u32,
    
    /// Soft vs hard constraint
    pub hard_constraint: bool,
}

impl Default for ResourcePrivacySettings {
    fn default() -> Self {
        Self {
            default_privacy_level: "private".to_string(),
            per_type_settings: HashMap::new(),
            grouping_settings: ResourceGroupingSettings::default(),
            allocation_optimization: ResourceAllocationOptimization::default(),
        }
    }
}

impl Default for ResourceGroupingSettings {
    fn default() -> Self {
        Self {
            strategy: ResourceGroupingStrategy::ByType,
            group_policies: Vec::new(),
            cross_resource_policies: Vec::new(),
        }
    }
}

impl Default for ResourceAllocationOptimization {
    fn default() -> Self {
        Self {
            enabled: false,
            objectives: vec![OptimizationObjective::BalancePerformance],
            constraints: Vec::new(),
            frequency: Duration::from_secs(300), // 5 minutes
        }
    }
}