//! CAESAR Reward Calculation System
//!
//! Calculates and manages CAESAR token rewards based on privacy levels,
//! resource allocation, utilization, and consensus proof validation.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};

use super::{
    CaesarRewardConfig, ResourceAllocationConfig, PrivacyAllocationType,
    PayoutFrequency
};
use crate::assets::core::{AssetResult, AssetError, PrivacyLevel};

/// CAESAR reward calculator and manager
pub struct CaesarRewardCalculator {
    /// Base reward configuration
    base_config: CaesarRewardConfig,
    
    /// Reward tier configurations
    reward_tiers: Vec<RewardTier>,
    
    /// Performance bonus configurations
    performance_bonuses: Vec<PerformanceBonus>,
    
    /// Penalty configurations
    penalty_configs: Vec<PenaltyConfig>,
    
    /// Dynamic adjustment factors
    dynamic_factors: DynamicAdjustmentFactors,
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ContributionType {
    ResourceSharing,
    ConsensusParticipation,
    NetworkStability,
    CommunitySupport,
    SecurityReporting,
    Documentation,
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

impl CaesarRewardCalculator {
    /// Create new reward calculator
    pub async fn new(base_config: &CaesarRewardConfig) -> AssetResult<Self> {
        Ok(Self {
            base_config: base_config.clone(),
            reward_tiers: Self::create_default_tiers(),
            performance_bonuses: Self::create_default_bonuses(),
            penalty_configs: Self::create_default_penalties(),
            dynamic_factors: DynamicAdjustmentFactors::default(),
        })
    }
    
    /// Calculate reward configuration for allocation
    pub async fn calculate_reward_config(
        &self,
        privacy_level: &PrivacyLevel,
        resource_config: &ResourceAllocationConfig,
        user_preferences: &super::manager::CaesarRewardPreferences,
    ) -> AssetResult<CaesarRewardConfig> {
        // Base reward rate calculation
        let base_rate = self.calculate_base_reward_rate(privacy_level, resource_config).await?;
        
        // Privacy multiplier
        let privacy_multiplier = self.calculate_privacy_multiplier(privacy_level).await?;
        
        // Utilization multiplier
        let utilization_multiplier = self.calculate_utilization_multiplier(resource_config).await?;
        
        // Performance bonuses
        let consensus_bonus = self.calculate_consensus_bonus().await?;
        
        // Apply dynamic adjustments
        let final_rate = self.apply_dynamic_adjustments(
            base_rate * privacy_multiplier * utilization_multiplier,
        ).await?;
        
        // Create reward configuration
        Ok(CaesarRewardConfig {
            base_reward_rate: final_rate,
            privacy_multiplier,
            utilization_multiplier,
            consensus_bonus,
            max_reward_cap: self.base_config.max_reward_cap,
            distribution_config: self.create_distribution_config(user_preferences).await?,
        })
    }
    
    /// Calculate actual rewards for completed allocation
    pub async fn calculate_actual_rewards(
        &self,
        allocation_duration: Duration,
        resource_utilization: &HashMap<String, f32>,
        privacy_level: &PrivacyLevel,
        performance_metrics: &HashMap<String, f32>,
        user_tier: &str,
    ) -> AssetResult<RewardCalculationResult> {
        
        // Get base reward rate
        let base_rate = self.base_config.base_reward_rate;
        
        // Calculate time-based reward
        let hours = allocation_duration.as_secs_f32() / 3600.0;
        let base_reward = base_rate * hours;
        
        // Apply privacy multiplier
        let privacy_multiplier = privacy_level.caesar_reward_multiplier();
        let privacy_adjusted_reward = base_reward * privacy_multiplier;
        
        // Apply utilization multipliers
        let utilization_bonus = self.calculate_utilization_bonus(resource_utilization).await?;
        let utilization_adjusted_reward = privacy_adjusted_reward * (1.0 + utilization_bonus);
        
        // Apply performance bonuses
        let performance_bonus = self.calculate_performance_bonuses(performance_metrics).await?;
        let performance_adjusted_reward = utilization_adjusted_reward * (1.0 + performance_bonus);
        
        // Apply tier multipliers
        let tier_multiplier = self.get_tier_multiplier(user_tier).await?;
        let tier_adjusted_reward = performance_adjusted_reward * tier_multiplier;
        
        // Apply penalties if any
        let penalty_factor = self.calculate_penalty_factor(performance_metrics).await?;
        let final_reward = tier_adjusted_reward * penalty_factor;
        
        // Apply reward cap
        let capped_reward = final_reward.min(self.base_config.max_reward_cap);
        
        Ok(RewardCalculationResult {
            base_reward,
            privacy_adjusted_reward,
            utilization_adjusted_reward,
            performance_adjusted_reward,
            tier_adjusted_reward,
            final_reward: capped_reward,
            breakdown: RewardBreakdown {
                base_rate,
                hours,
                privacy_multiplier,
                utilization_bonus,
                performance_bonus,
                tier_multiplier,
                penalty_factor,
            },
        })
    }
    
    // Helper methods (implementation details)
    async fn calculate_base_reward_rate(
        &self,
        privacy_level: &PrivacyLevel,
        resource_config: &ResourceAllocationConfig,
    ) -> AssetResult<f32> {
        let base = self.base_config.base_reward_rate;
        
        // Adjust based on resource allocation
        let resource_factor = (
            resource_config.cpu_percentage +
            resource_config.gpu_percentage +
            resource_config.memory_percentage +
            resource_config.storage_percentage +
            resource_config.network_percentage
        ) / 5.0;
        
        Ok(base * resource_factor)
    }
    
    async fn calculate_privacy_multiplier(&self, privacy_level: &PrivacyLevel) -> AssetResult<f32> {
        Ok(privacy_level.caesar_reward_multiplier())
    }
    
    async fn calculate_utilization_multiplier(
        &self,
        _resource_config: &ResourceAllocationConfig,
    ) -> AssetResult<f32> {
        // Placeholder implementation
        Ok(1.0)
    }
    
    async fn calculate_consensus_bonus(&self) -> AssetResult<f32> {
        // Placeholder implementation
        Ok(self.base_config.consensus_bonus)
    }
    
    async fn apply_dynamic_adjustments(&self, base_rate: f32) -> AssetResult<f32> {
        let mut adjusted_rate = base_rate;
        
        // Apply network load factor
        let network_factor = self.dynamic_factors.network_load_factor.current_utilization;
        adjusted_rate *= 1.0 + (network_factor * 0.1); // 10% max adjustment
        
        // Apply economic factors
        adjusted_rate *= 1.0 + self.dynamic_factors.economic_factor.inflation_adjustment;
        
        Ok(adjusted_rate)
    }
    
    async fn create_distribution_config(
        &self,
        user_preferences: &super::manager::CaesarRewardPreferences,
    ) -> AssetResult<super::RewardDistributionConfig> {
        Ok(super::RewardDistributionConfig {
            immediate_payout: user_preferences.payout_frequency == PayoutFrequency::Immediate,
            immediate_percentage: 1.0 - user_preferences.auto_stake_percentage,
            auto_stake_remainder: user_preferences.auto_stake_percentage > 0.0,
            minimum_payout_threshold: user_preferences.minimum_reward_rate,
            payout_frequency: user_preferences.payout_frequency.clone(),
        })
    }
    
    async fn calculate_utilization_bonus(
        &self,
        resource_utilization: &HashMap<String, f32>,
    ) -> AssetResult<f32> {
        let avg_utilization = resource_utilization.values().sum::<f32>() / 
                             resource_utilization.len() as f32;
        
        // Bonus for high utilization
        if avg_utilization > 0.8 {
            Ok(0.2) // 20% bonus
        } else if avg_utilization > 0.6 {
            Ok(0.1) // 10% bonus
        } else {
            Ok(0.0) // No bonus
        }
    }
    
    async fn calculate_performance_bonuses(
        &self,
        performance_metrics: &HashMap<String, f32>,
    ) -> AssetResult<f32> {
        let mut total_bonus = 0.0;
        
        for bonus_config in &self.performance_bonuses {
            if let Some(metric_value) = performance_metrics.get(&format!("{:?}", bonus_config.metric)) {
                for threshold in &bonus_config.thresholds {
                    if *metric_value >= threshold.threshold_value {
                        total_bonus += threshold.multiplier - 1.0; // Convert to bonus factor
                        break;
                    }
                }
            }
        }
        
        Ok(total_bonus.min(0.5)) // Cap at 50% bonus
    }
    
    async fn get_tier_multiplier(&self, user_tier: &str) -> AssetResult<f32> {
        for tier in &self.reward_tiers {
            if tier.tier_name == user_tier {
                return Ok(tier.benefits.reward_multiplier);
            }
        }
        
        Ok(1.0) // Default multiplier
    }
    
    async fn calculate_penalty_factor(
        &self,
        _performance_metrics: &HashMap<String, f32>,
    ) -> AssetResult<f32> {
        // Placeholder - would check for violations and apply penalties
        Ok(1.0) // No penalties for now
    }
    
    // Create default configurations
    fn create_default_tiers() -> Vec<RewardTier> {
        vec![
            RewardTier {
                tier_name: "Bronze".to_string(),
                requirements: TierRequirements::default(),
                base_multiplier: 1.0,
                benefits: TierBenefits {
                    reward_multiplier: 1.0,
                    consistency_bonus: 0.0,
                    allocation_priority: 1,
                    penalty_reduction: 0.0,
                    special_privileges: vec![],
                },
                advancement_conditions: AdvancementConditions::default(),
            },
            RewardTier {
                tier_name: "Silver".to_string(),
                requirements: TierRequirements {
                    min_allocation_time: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
                    min_privacy_participation: HashMap::new(),
                    min_utilization_rate: 0.5,
                    min_consensus_success_rate: 0.9,
                    min_stake_amount: 1000,
                    min_trust_score: 0.7,
                },
                base_multiplier: 1.2,
                benefits: TierBenefits {
                    reward_multiplier: 1.2,
                    consistency_bonus: 0.05,
                    allocation_priority: 2,
                    penalty_reduction: 0.1,
                    special_privileges: vec!["priority_support".to_string()],
                },
                advancement_conditions: AdvancementConditions::default(),
            },
            RewardTier {
                tier_name: "Gold".to_string(),
                requirements: TierRequirements {
                    min_allocation_time: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
                    min_privacy_participation: HashMap::new(),
                    min_utilization_rate: 0.7,
                    min_consensus_success_rate: 0.95,
                    min_stake_amount: 5000,
                    min_trust_score: 0.85,
                },
                base_multiplier: 1.5,
                benefits: TierBenefits {
                    reward_multiplier: 1.5,
                    consistency_bonus: 0.1,
                    allocation_priority: 3,
                    penalty_reduction: 0.2,
                    special_privileges: vec![
                        "priority_support".to_string(),
                        "beta_access".to_string(),
                    ],
                },
                advancement_conditions: AdvancementConditions::default(),
            },
        ]
    }
    
    fn create_default_bonuses() -> Vec<PerformanceBonus> {
        vec![
            PerformanceBonus {
                bonus_name: "High Uptime".to_string(),
                metric: PerformanceMetric::Uptime,
                thresholds: vec![
                    BonusThreshold {
                        threshold_value: 0.95,
                        multiplier: 1.05,
                        duration_requirement: Some(Duration::from_secs(24 * 60 * 60)),
                    },
                    BonusThreshold {
                        threshold_value: 0.99,
                        multiplier: 1.1,
                        duration_requirement: Some(Duration::from_secs(7 * 24 * 60 * 60)),
                    },
                ],
                max_multiplier: 1.2,
                calculation_method: BonusCalculationMethod::Stepped,
            },
        ]
    }
    
    fn create_default_penalties() -> Vec<PenaltyConfig> {
        vec![
            PenaltyConfig {
                penalty_name: "Service Unavailability".to_string(),
                violation_type: ViolationType::ServiceUnavailability,
                severity_levels: vec![
                    PenaltySeverityLevel {
                        severity: "Minor".to_string(),
                        penalty_multiplier: 0.9,
                        penalty_duration: Duration::from_secs(60 * 60), // 1 hour
                        restrictions: vec![],
                    },
                    PenaltySeverityLevel {
                        severity: "Major".to_string(),
                        penalty_multiplier: 0.5,
                        penalty_duration: Duration::from_secs(24 * 60 * 60), // 1 day
                        restrictions: vec!["reduced_allocation_limit".to_string()],
                    },
                ],
                recovery_conditions: RecoveryConditions {
                    time_based_recovery: true,
                    performance_recovery: None,
                    community_recovery: None,
                    admin_recovery: false,
                },
            },
        ]
    }
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

// Default implementations
impl Default for TierRequirements {
    fn default() -> Self {
        Self {
            min_allocation_time: Duration::from_secs(0),
            min_privacy_participation: HashMap::new(),
            min_utilization_rate: 0.0,
            min_consensus_success_rate: 0.0,
            min_stake_amount: 0,
            min_trust_score: 0.0,
        }
    }
}

impl Default for AdvancementConditions {
    fn default() -> Self {
        Self {
            performance_thresholds: HashMap::new(),
            min_time_in_tier: Duration::from_secs(24 * 60 * 60), // 1 day
            contribution_requirements: vec![],
            verification_requirements: vec![],
        }
    }
}

impl Default for DynamicAdjustmentFactors {
    fn default() -> Self {
        Self {
            network_load_factor: NetworkLoadFactor {
                current_utilization: 0.5,
                utilization_multipliers: vec![
                    (0.8, 1.1),
                    (0.9, 1.2),
                    (0.95, 1.3),
                ],
                load_balancing_incentives: true,
            },
            economic_factor: EconomicFactor {
                current_token_price: 1.0,
                price_stability_adjustment: 0.0,
                inflation_adjustment: 0.0,
                cycle_adjustment: 0.0,
            },
            supply_demand_factor: SupplyDemandFactor {
                resource_supply: HashMap::new(),
                resource_demand: HashMap::new(),
                supply_demand_multipliers: HashMap::new(),
                scarcity_bonuses: HashMap::new(),
            },
            seasonal_adjustments: vec![],
        }
    }
}