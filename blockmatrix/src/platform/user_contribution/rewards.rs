//! Reward calculation engine and reputation system

use std::collections::HashMap;
use crate::assets::core::AssetType;

/// Reward calculation engine
pub struct RewardEngine {
    pub base_rates: HashMap<AssetType, f64>,
    pub performance_multipliers: PerformanceMultipliers,
    pub reputation_system: ReputationSystem,
}

/// Performance multipliers for reward calculation
#[derive(Debug, Clone)]
pub struct PerformanceMultipliers {
    pub uptime_multiplier: f64,
    pub efficiency_multiplier: f64,
    pub user_satisfaction_multiplier: f64,
    pub response_time_multiplier: f64,
}

/// Reputation system for users
#[derive(Debug, Clone)]
pub struct ReputationSystem {
    pub base_reputation: f64,
    pub uptime_weight: f64,
    pub performance_weight: f64,
    pub user_feedback_weight: f64,
    pub tenure_weight: f64,
}

impl RewardEngine {
    pub fn new() -> Self {
        let mut base_rates = HashMap::new();
        base_rates.insert(AssetType::Cpu, 0.10);
        base_rates.insert(AssetType::Memory, 0.01);
        base_rates.insert(AssetType::Gpu, 1.00);
        base_rates.insert(AssetType::Storage, 0.001);
        base_rates.insert(AssetType::Network, 0.05);

        Self {
            base_rates,
            performance_multipliers: PerformanceMultipliers {
                uptime_multiplier: 1.2,
                efficiency_multiplier: 1.1,
                user_satisfaction_multiplier: 1.15,
                response_time_multiplier: 1.05,
            },
            reputation_system: ReputationSystem {
                base_reputation: 1.0,
                uptime_weight: 0.3,
                performance_weight: 0.3,
                user_feedback_weight: 0.25,
                tenure_weight: 0.15,
            },
        }
    }
}

/// Platform metrics
#[derive(Debug, Default)]
pub struct PlatformMetrics {
    pub total_users: u64,
    pub active_users: u64,
    pub total_resources_shared: HashMap<AssetType, u64>,
    pub total_earnings_distributed: f64,
    pub average_user_rating: f64,
    pub platform_efficiency: f64,
    pub network_utilization: f64,
}

impl Clone for PlatformMetrics {
    fn clone(&self) -> Self {
        Self {
            total_users: self.total_users,
            active_users: self.active_users,
            total_resources_shared: self.total_resources_shared.clone(),
            total_earnings_distributed: self.total_earnings_distributed,
            average_user_rating: self.average_user_rating,
            platform_efficiency: self.platform_efficiency,
            network_utilization: self.network_utilization,
        }
    }
}
