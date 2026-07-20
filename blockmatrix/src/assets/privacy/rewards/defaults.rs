// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Default implementations for reward system types.

use std::collections::HashMap;
use std::time::Duration;

use crate::assets::privacy::PayoutFrequency;

use super::config::*;
use super::types::*;

impl Default for RewardConfiguration {
    fn default() -> Self {
        Self {
            current_tier: "basic".to_string(),
            calculation_preferences: RewardCalculationPreferences::default(),
            payout_preferences: PayoutPreferences::default(),
            tax_settings: TaxSettings::default(),
            optimization_settings: RewardOptimizationSettings::default(),
        }
    }
}

impl Default for TierRequirements {
    fn default() -> Self {
        Self {
            min_allocation_time: Duration::from_secs(0),
            min_privacy_participation: HashMap::new(),
            min_utilization_rate: 0.0,
            min_state_proof_success_rate: 0.0,
            require_authentication: false,
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
                utilization_multipliers: vec![(0.8, 1.1), (0.9, 1.2), (0.95, 1.3)],
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

impl Default for RewardCalculationMethod {
    fn default() -> Self {
        Self::Balanced
    }
}

impl Default for RiskToleranceLevel {
    fn default() -> Self {
        Self::Medium
    }
}

impl Default for RewardCalculationPreferences {
    fn default() -> Self {
        Self {
            calculation_method: RewardCalculationMethod::default(),
            risk_tolerance: RiskToleranceLevel::default(),
            reward_privacy_balance: 0.5,
            accept_dynamic_adjustments: true,
        }
    }
}

impl Default for PayoutPreferences {
    fn default() -> Self {
        Self {
            frequency: PayoutFrequency::Daily,
            minimum_threshold: 10.0,
            auto_compound_percentage: 0.0,
            preferred_token: "CAESAR".to_string(),
            staking_preferences: StakingPreferences::default(),
        }
    }
}

impl Default for StakingPreferences {
    fn default() -> Self {
        Self {
            auto_stake_percentage: 0.0,
            preferred_duration: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            risk_tolerance: RiskToleranceLevel::default(),
            liquid_reserve_percentage: 20.0,
        }
    }
}

impl Default for TaxSettings {
    fn default() -> Self {
        Self {
            jurisdiction: "US".to_string(),
            reporting_requirements: vec![],
            withholding_preferences: WithholdingPreferences::default(),
            cost_basis_tracking: true,
        }
    }
}

impl Default for WithholdingPreferences {
    fn default() -> Self {
        Self {
            auto_withholding: false,
            withholding_percentage: 0.0,
            withholding_account: None,
            quarterly_payments: false,
        }
    }
}

impl Default for RewardOptimizationSettings {
    fn default() -> Self {
        Self {
            auto_optimization: true,
            objectives: vec![OptimizationObjective::MaximizeRewards],
            rebalancing: RebalancingPreferences::default(),
            performance_tracking: PerformanceTracking::default(),
        }
    }
}

impl Default for RebalancingPreferences {
    fn default() -> Self {
        Self {
            frequency: RebalancingFrequency::Weekly,
            thresholds: HashMap::new(),
            automatic: true,
            consider_costs: true,
        }
    }
}

impl Default for RebalancingFrequency {
    fn default() -> Self {
        Self::Weekly
    }
}

impl Default for PerformanceTracking {
    fn default() -> Self {
        Self {
            track_rewards: true,
            track_privacy: true,
            benchmarks: vec![],
            reporting: PerformanceReporting::default(),
        }
    }
}

impl Default for PerformanceReporting {
    fn default() -> Self {
        Self {
            frequency: ReportingFrequency::Monthly,
            formats: vec!["json".to_string()],
            detailed_breakdowns: true,
            privacy_aware: true,
        }
    }
}

impl Default for ReportingFrequency {
    fn default() -> Self {
        Self::Monthly
    }
}
