// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Reward system type definitions - structs, enums, and core data types.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};

use crate::assets::privacy::CaesarRewardConfig;

/// CAESAR reward calculator and manager
#[allow(dead_code)] // Fields used during reward calculation
pub struct CaesarRewardCalculator {
    /// Base reward configuration
    pub(crate) base_config: CaesarRewardConfig,

    /// Reward tier configurations
    pub(crate) reward_tiers: Vec<RewardTier>,

    /// Performance bonus configurations
    pub(crate) performance_bonuses: Vec<PerformanceBonus>,

    /// Penalty configurations
    pub(crate) penalty_configs: Vec<PenaltyConfig>,

    /// Dynamic adjustment factors
    pub(crate) dynamic_factors: DynamicAdjustmentFactors,
}

/// Reward tier configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RewardTier {
    /// Tier name
    pub tier_name: String,

    /// Minimum requirements to qualify for tier
    pub requirements: TierRequirements,

    /// Base multiplier for this tier
    pub base_multiplier: f32,

    /// Additional benefits
    pub benefits: TierBenefits,

    /// Tier advancement conditions
    pub advancement_conditions: AdvancementConditions,
}

/// Requirements to qualify for reward tier
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TierRequirements {
    /// Minimum total allocation time
    pub min_allocation_time: Duration,

    /// Minimum privacy level participation
    pub min_privacy_participation: HashMap<String, f32>, // privacy_level -> percentage

    /// Minimum utilization rate
    pub min_utilization_rate: f32,

    /// Minimum consensus proof success rate
    pub min_consensus_success_rate: f32,

    /// Minimum stake amount
    pub min_stake_amount: u64,

    /// Minimum trust score
    pub min_trust_score: f32,
}

/// Benefits provided by reward tier
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TierBenefits {
    /// Enhanced reward multiplier
    pub reward_multiplier: f32,

    /// Bonus for consistent participation
    pub consistency_bonus: f32,

    /// Priority in allocation selection
    pub allocation_priority: u32,

    /// Reduced penalty rates
    pub penalty_reduction: f32,

    /// Special access privileges
    pub special_privileges: Vec<String>,
}

/// Conditions for advancing to next tier
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdvancementConditions {
    /// Performance metrics required
    pub performance_thresholds: HashMap<String, f32>,

    /// Time in current tier requirement
    pub min_time_in_tier: Duration,

    /// Community contribution requirements
    pub contribution_requirements: Vec<ContributionRequirement>,

    /// Verification requirements
    pub verification_requirements: Vec<String>,
}

/// Contribution requirements for tier advancement
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContributionRequirement {
    /// Contribution type
    pub contribution_type: ContributionType,

    /// Minimum contribution amount/count
    pub minimum_amount: f32,

    /// Time period for contribution
    pub time_period: Duration,

    /// Quality thresholds
    pub quality_thresholds: QualityThresholds,
}

/// Types of contributions
///
/// NOTE: ResourceSharing only earns CAESAR when hosting paid content via NGauge
/// (advertisements, KYCML content, paid hosting). General network participation
/// does NOT earn CAESAR rewards.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ContributionType {
    /// Hosting paid content (ads, KYCML, paid hosting) - earns CAESAR via NGauge
    ResourceSharing,
    /// Consensus validation participation
    ConsensusParticipation,
    /// Network stability contributions
    NetworkStability,
    /// Community support activities
    CommunitySupport,
    /// Security vulnerability reporting
    SecurityReporting,
    /// Documentation contributions
    Documentation,
    /// Code contributions to the platform
    CodeContribution,
}

/// Quality thresholds for contributions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityThresholds {
    /// Minimum quality score
    pub min_quality_score: f32,

    /// Minimum peer ratings
    pub min_peer_ratings: u32,

    /// Minimum success rate
    pub min_success_rate: f32,

    /// Community acceptance threshold
    pub community_acceptance_threshold: f32,
}

/// Performance bonus configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceBonus {
    /// Bonus name
    pub bonus_name: String,

    /// Performance metric measured
    pub metric: PerformanceMetric,

    /// Threshold values for bonus tiers
    pub thresholds: Vec<BonusThreshold>,

    /// Maximum bonus multiplier
    pub max_multiplier: f32,

    /// Bonus calculation method
    pub calculation_method: BonusCalculationMethod,
}

/// Performance metrics for bonuses
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PerformanceMetric {
    Uptime,
    ResponseTime,
    Throughput,
    ResourceUtilization,
    SecurityScore,
    TrustScore,
    ConsensusParticipation,
    PeerRating,
}

/// Bonus threshold configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BonusThreshold {
    /// Threshold value
    pub threshold_value: f32,

    /// Bonus multiplier at this threshold
    pub multiplier: f32,

    /// Duration requirement at threshold
    pub duration_requirement: Option<Duration>,
}

/// Bonus calculation methods
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BonusCalculationMethod {
    Linear,
    Exponential,
    Stepped,
    Logarithmic,
}

/// Penalty configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PenaltyConfig {
    /// Penalty name
    pub penalty_name: String,

    /// Violation type that triggers penalty
    pub violation_type: ViolationType,

    /// Penalty severity levels
    pub severity_levels: Vec<PenaltySeverityLevel>,

    /// Recovery conditions
    pub recovery_conditions: RecoveryConditions,
}

/// Types of violations that incur penalties
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ViolationType {
    PrivacyViolation,
    ServiceUnavailability,
    ConsensusFailure,
    SecurityBreach,
    ResourceMisuse,
    ContractViolation,
}

/// Penalty severity levels
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PenaltySeverityLevel {
    /// Severity name
    pub severity: String,

    /// Penalty multiplier (reduction factor)
    pub penalty_multiplier: f32,

    /// Duration of penalty
    pub penalty_duration: Duration,

    /// Additional restrictions
    pub restrictions: Vec<String>,
}

/// Conditions for recovering from penalties
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryConditions {
    /// Time-based recovery
    pub time_based_recovery: bool,

    /// Performance-based recovery
    pub performance_recovery: Option<PerformanceRecovery>,

    /// Community-based recovery
    pub community_recovery: Option<CommunityRecovery>,

    /// Administrative recovery
    pub admin_recovery: bool,
}

/// Performance-based penalty recovery
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceRecovery {
    /// Required performance metrics
    pub required_metrics: HashMap<String, f32>,

    /// Performance duration requirement
    pub performance_duration: Duration,

    /// Progressive recovery
    pub progressive_recovery: bool,
}

/// Community-based penalty recovery
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommunityRecovery {
    /// Required community votes
    pub required_votes: u32,

    /// Vote threshold percentage
    pub vote_threshold: f32,

    /// Community service requirements
    pub service_requirements: Vec<String>,
}

/// Dynamic adjustment factors for rewards
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DynamicAdjustmentFactors {
    /// Network load adjustment
    pub network_load_factor: NetworkLoadFactor,

    /// Economic adjustment
    pub economic_factor: EconomicFactor,

    /// Supply and demand adjustment
    pub supply_demand_factor: SupplyDemandFactor,

    /// Seasonal adjustments
    pub seasonal_adjustments: Vec<SeasonalAdjustment>,
}

/// Network load factor for dynamic adjustments
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkLoadFactor {
    /// Current network utilization
    pub current_utilization: f32,

    /// Utilization thresholds and multipliers
    pub utilization_multipliers: Vec<(f32, f32)>, // (threshold, multiplier)

    /// Load balancing incentives
    pub load_balancing_incentives: bool,
}

/// Economic factor for dynamic adjustments
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EconomicFactor {
    /// Current token price
    pub current_token_price: f32,

    /// Price stability adjustment
    pub price_stability_adjustment: f32,

    /// Inflation/deflation adjustment
    pub inflation_adjustment: f32,

    /// Economic cycle adjustment
    pub cycle_adjustment: f32,
}

/// Supply and demand factor
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SupplyDemandFactor {
    /// Resource supply levels
    pub resource_supply: HashMap<String, f32>,

    /// Resource demand levels
    pub resource_demand: HashMap<String, f32>,

    /// Supply/demand multipliers
    pub supply_demand_multipliers: HashMap<String, f32>,

    /// Scarcity bonuses
    pub scarcity_bonuses: HashMap<String, f32>,
}

/// Seasonal adjustment configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeasonalAdjustment {
    /// Season identifier
    pub season_id: String,

    /// Start and end dates
    pub date_range: (SystemTime, SystemTime),

    /// Adjustment multiplier
    pub multiplier: f32,

    /// Affected resource types
    pub affected_resources: Vec<String>,
}

