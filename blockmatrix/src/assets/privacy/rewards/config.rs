// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Reward configuration types - user preferences, payout, tax, and optimization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::assets::privacy::PayoutFrequency;

/// Complete reward configuration for user preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RewardConfiguration {
    /// User's reward tier
    pub current_tier: String,

    /// Reward calculation preferences
    pub calculation_preferences: RewardCalculationPreferences,

    /// Payout preferences
    pub payout_preferences: PayoutPreferences,

    /// Tax and compliance settings
    pub tax_settings: TaxSettings,

    /// Reward optimization settings
    pub optimization_settings: RewardOptimizationSettings,
}

/// Reward calculation preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RewardCalculationPreferences {
    /// Preferred calculation method
    pub calculation_method: RewardCalculationMethod,

    /// Risk tolerance level
    pub risk_tolerance: RiskToleranceLevel,

    /// Preferred reward/privacy balance
    pub reward_privacy_balance: f32, // 0.0 = max privacy, 1.0 = max rewards

    /// Dynamic adjustment acceptance
    pub accept_dynamic_adjustments: bool,
}

/// Reward calculation methods
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RewardCalculationMethod {
    Conservative, // Lower rewards, higher stability
    Balanced,     // Balanced approach
    Aggressive,   // Higher rewards, more volatility
    Custom,       // User-defined parameters
}

/// Risk tolerance levels
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RiskToleranceLevel {
    Low,
    Medium,
    High,
}

/// Payout preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PayoutPreferences {
    /// Payout frequency
    pub frequency: PayoutFrequency,

    /// Minimum payout threshold
    pub minimum_threshold: f32,

    /// Auto-compound percentage
    pub auto_compound_percentage: f32,

    /// Preferred payout token
    pub preferred_token: String,

    /// Staking preferences
    pub staking_preferences: StakingPreferences,
}

/// Staking preferences for rewards
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StakingPreferences {
    /// Auto-stake percentage
    pub auto_stake_percentage: f32,

    /// Preferred staking duration
    pub preferred_duration: Duration,

    /// Staking risk tolerance
    pub risk_tolerance: RiskToleranceLevel,

    /// Liquid reserve percentage
    pub liquid_reserve_percentage: f32,
}

/// Tax and compliance settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaxSettings {
    /// Tax jurisdiction
    pub jurisdiction: String,

    /// Tax reporting requirements
    pub reporting_requirements: Vec<String>,

    /// Tax withholding preferences
    pub withholding_preferences: WithholdingPreferences,

    /// Cost basis tracking
    pub cost_basis_tracking: bool,
}

/// Tax withholding preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WithholdingPreferences {
    /// Enable automatic withholding
    pub auto_withholding: bool,

    /// Withholding percentage
    pub withholding_percentage: f32,

    /// Withholding account
    pub withholding_account: Option<String>,

    /// Quarterly payment scheduling
    pub quarterly_payments: bool,
}

/// Reward optimization settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RewardOptimizationSettings {
    /// Enable automatic optimization
    pub auto_optimization: bool,

    /// Optimization objectives
    pub objectives: Vec<OptimizationObjective>,

    /// Rebalancing preferences
    pub rebalancing: RebalancingPreferences,

    /// Performance tracking
    pub performance_tracking: PerformanceTracking,
}

/// Optimization objectives
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OptimizationObjective {
    MaximizeRewards,
    MinimizeRisk,
    BalanceRewardRisk,
    MaximizePrivacy,
    OptimizeForTaxes,
}

/// Rebalancing preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RebalancingPreferences {
    /// Rebalancing frequency
    pub frequency: RebalancingFrequency,

    /// Rebalancing thresholds
    pub thresholds: HashMap<String, f32>,

    /// Automatic rebalancing
    pub automatic: bool,

    /// Rebalancing costs consideration
    pub consider_costs: bool,
}

/// Rebalancing frequency options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RebalancingFrequency {
    Never,
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Threshold, // Based on deviation thresholds
}

/// Performance tracking configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceTracking {
    /// Track reward performance
    pub track_rewards: bool,

    /// Track privacy performance
    pub track_privacy: bool,

    /// Benchmark comparisons
    pub benchmarks: Vec<String>,

    /// Performance reporting
    pub reporting: PerformanceReporting,
}

/// Performance reporting settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceReporting {
    /// Reporting frequency
    pub frequency: ReportingFrequency,

    /// Report formats
    pub formats: Vec<String>,

    /// Include detailed breakdowns
    pub detailed_breakdowns: bool,

    /// Privacy-aware reporting
    pub privacy_aware: bool,
}

/// Reporting frequency options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReportingFrequency {
    RealTime,
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Annual,
}

/// Result of reward calculation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RewardCalculationResult {
    pub base_reward: f32,
    pub privacy_adjusted_reward: f32,
    pub utilization_adjusted_reward: f32,
    pub performance_adjusted_reward: f32,
    pub tier_adjusted_reward: f32,
    pub final_reward: f32,
    pub breakdown: RewardBreakdown,
}

/// Detailed breakdown of reward calculation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RewardBreakdown {
    pub base_rate: f32,
    pub hours: f32,
    pub privacy_multiplier: f32,
    pub utilization_bonus: f32,
    pub performance_bonus: f32,
    pub tier_multiplier: f32,
    pub penalty_factor: f32,
}
