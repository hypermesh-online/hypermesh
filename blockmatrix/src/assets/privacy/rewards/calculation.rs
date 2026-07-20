// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CAESAR reward calculation logic - core computation and tier management.

use std::collections::HashMap;
use std::time::Duration;

use crate::assets::core::{AssetResult, PrivacyMode};
use crate::assets::privacy::{CaesarRewardConfig, PayoutFrequency, ResourceAllocationConfig};

use super::config::*;
use super::types::*;

impl CaesarRewardCalculator {
    /// Create new reward calculator
    pub async fn new(base_config: &CaesarRewardConfig) -> AssetResult<Self> {
        Ok(Self {
            base_config: base_config.clone(),
            reward_tiers: Self::create_default_tiers(),
            performance_bonuses: Self::create_default_bonuses(),
            _penalty_configs: Self::create_default_penalties(),
            dynamic_factors: DynamicAdjustmentFactors::default(),
        })
    }

    /// Calculate reward configuration for allocation
    pub async fn calculate_reward_config(
        &self,
        privacy_level: &PrivacyMode,
        resource_config: &ResourceAllocationConfig,
        user_preferences: &super::super::manager::CaesarRewardPreferences,
    ) -> AssetResult<CaesarRewardConfig> {
        // Base reward rate calculation
        let base_rate = self
            .calculate_base_reward_rate(privacy_level, resource_config)
            .await?;

        // Privacy multiplier
        let privacy_multiplier = self.calculate_privacy_multiplier(privacy_level).await?;

        // Utilization multiplier
        let utilization_multiplier = self
            .calculate_utilization_multiplier(resource_config)
            .await?;

        // Performance bonuses
        let verification_bonus = self.calculate_verification_bonus().await?;

        // Apply dynamic adjustments
        let final_rate = self
            .apply_dynamic_adjustments(base_rate * privacy_multiplier * utilization_multiplier)
            .await?;

        // Create reward configuration
        Ok(CaesarRewardConfig {
            base_reward_rate: final_rate,
            privacy_multiplier,
            utilization_multiplier,
            verification_bonus,
            max_reward_cap: self.base_config.max_reward_cap,
            distribution_config: self.create_distribution_config(user_preferences).await?,
        })
    }

    /// Calculate actual rewards for completed allocation
    pub async fn calculate_actual_rewards(
        &self,
        allocation_duration: Duration,
        resource_utilization: &HashMap<String, f32>,
        privacy_level: &PrivacyMode,
        performance_metrics: &HashMap<String, f32>,
        user_tier: &str,
    ) -> AssetResult<RewardCalculationResult> {
        // Get base reward rate
        let base_rate = self.base_config.base_reward_rate;

        // Calculate time-based reward
        let hours = allocation_duration.as_secs_f32() / 3600.0;
        let base_reward = base_rate * hours;

        // Apply privacy multiplier
        let privacy_multiplier =
            crate::assets::core::privacy::caesar_reward_multiplier(privacy_level);
        let privacy_adjusted_reward = base_reward * privacy_multiplier;

        // Apply utilization multipliers
        let utilization_bonus = self
            .calculate_utilization_bonus(resource_utilization)
            .await?;
        let utilization_adjusted_reward = privacy_adjusted_reward * (1.0 + utilization_bonus);

        // Apply performance bonuses
        let performance_bonus = self
            .calculate_performance_bonuses(performance_metrics)
            .await?;
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
        _privacy_level: &PrivacyMode,
        resource_config: &ResourceAllocationConfig,
    ) -> AssetResult<f32> {
        let base = self.base_config.base_reward_rate;

        // Adjust based on resource allocation
        let resource_factor = (resource_config.cpu_percentage
            + resource_config.gpu_percentage
            + resource_config.memory_percentage
            + resource_config.storage_percentage
            + resource_config.network_percentage)
            / 5.0;

        Ok(base * resource_factor)
    }

    async fn calculate_privacy_multiplier(&self, privacy_level: &PrivacyMode) -> AssetResult<f32> {
        Ok(crate::assets::core::privacy::caesar_reward_multiplier(
            privacy_level,
        ))
    }

    async fn calculate_utilization_multiplier(
        &self,
        _resource_config: &ResourceAllocationConfig,
    ) -> AssetResult<f32> {
        Ok(1.0)
    }

    async fn calculate_verification_bonus(&self) -> AssetResult<f32> {
        Ok(self.base_config.verification_bonus)
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
        user_preferences: &super::super::manager::CaesarRewardPreferences,
    ) -> AssetResult<super::super::RewardDistributionConfig> {
        Ok(super::super::RewardDistributionConfig {
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
        let avg_utilization =
            resource_utilization.values().sum::<f32>() / resource_utilization.len() as f32;

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
            if let Some(metric_value) =
                performance_metrics.get(&format!("{:?}", bonus_config.metric))
            {
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
                    min_state_proof_success_rate: 0.9,
                    require_authentication: true,
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
                    min_state_proof_success_rate: 0.95,
                    require_authentication: true,
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
        vec![PerformanceBonus {
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
        }]
    }

    fn create_default_penalties() -> Vec<PenaltyConfig> {
        vec![PenaltyConfig {
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
        }]
    }
}
