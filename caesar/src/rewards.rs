// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Caesar Rewards System - Calculate and distribute rewards

use anyhow::Result;
use chrono::{DateTime, Utc, Duration, Timelike};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;
use tracing::{info, debug};
use uuid::Uuid;
use std::collections::HashMap;

use crate::models::*;
use crate::storage::CaesarStorage;
use crate::RewardConfig;

/// Reward calculator for resource sharing and network participation
pub struct RewardCalculator {
    config: RewardConfig,
    storage: Arc<CaesarStorage>,

    // Cache for performance
    pending_cache: Arc<tokio::sync::RwLock<HashMap<String, Decimal>>>,
    earnings_cache: Arc<tokio::sync::RwLock<HashMap<String, DailyEarnings>>>,
}

#[allow(dead_code)] // Earnings tracking fields
#[derive(Debug, Clone)]
struct DailyEarnings {
    amount: Decimal,
    last_update: DateTime<Utc>,
    sources: Vec<EarningSource>,
}

impl RewardCalculator {
    /// Create a new RewardCalculator with proper storage initialization
    /// This constructor requires storage to be provided to ensure safety
    pub fn new(config: RewardConfig, storage: Arc<CaesarStorage>) -> Self {
        Self {
            config,
            storage,
            pending_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            earnings_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Create a new RewardCalculator for testing purposes with default storage
    #[cfg(test)]
    #[allow(dead_code)]
    pub fn new_for_testing(config: RewardConfig, storage: Arc<CaesarStorage>) -> Self {
        // For testing with provided storage instance
        Self::new(config, storage)
    }

    /// Get pending rewards for a wallet
    pub async fn get_pending_rewards(&self, wallet_id: &str) -> Result<Decimal> {
        // Check cache first
        {
            let cache = self.pending_cache.read().await;
            if let Some(&amount) = cache.get(wallet_id) {
                return Ok(amount);
            }
        }

        // Calculate pending from unclaimed rewards
        let rewards = self.storage.get_wallet_rewards(wallet_id).await?;
        let pending = rewards
            .iter()
            .filter(|r| !r.claimed)
            .map(|r| r.amount)
            .sum();

        // Update cache
        {
            let mut cache = self.pending_cache.write().await;
            cache.insert(wallet_id.to_string(), pending);
        }

        Ok(pending)
    }

    /// Get today's earnings for a wallet
    pub async fn get_today_earnings(&self, wallet_id: &str) -> Result<Decimal> {
        let cache = self.earnings_cache.read().await;

        if let Some(earnings) = cache.get(wallet_id) {
            // Check if cache is still valid (within last hour)
            if earnings.last_update > Utc::now() - Duration::hours(1) {
                return Ok(earnings.amount);
            }
        }

        // Calculate fresh earnings
        let today_start = Utc::now().date_naive().and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();

        let history = self.storage.get_wallet_rewards(wallet_id).await?;

        let today_earnings = history
            .iter()
            .filter(|r| r.timestamp >= today_start)
            .map(|r| r.amount)
            .sum();

        // Update cache
        drop(cache);
        let mut cache = self.earnings_cache.write().await;
        cache.insert(wallet_id.to_string(), DailyEarnings {
            amount: today_earnings,
            last_update: Utc::now(),
            sources: vec![],
        });

        Ok(today_earnings)
    }

    /// Get earning sources for a wallet
    pub async fn get_earning_sources(&self, _wallet_id: &str) -> Result<Vec<EarningSource>> {
        // These are the active earning sources
        // In production, this would check actual resource sharing status
        Ok(vec![
            EarningSource {
                source_type: "resource_sharing".to_string(),
                description: "CPU, memory, and storage sharing".to_string(),
                amount_today: dec!(15.2),
                is_active: true,
            },
            EarningSource {
                source_type: "network_validation".to_string(),
                description: "Consensus participation".to_string(),
                amount_today: dec!(6.8),
                is_active: true,
            },
            EarningSource {
                source_type: "asset_hosting".to_string(),
                description: "Catalog asset hosting".to_string(),
                amount_today: dec!(2.5),
                is_active: true,
            },
            EarningSource {
                source_type: "ngauge_ads".to_string(),
                description: "Optional advertising integration".to_string(),
                amount_today: dec!(0),
                is_active: false,
            },
        ])
    }

    /// Calculate rewards based on resource metrics with overflow protection
    pub async fn calculate(&self, request: CalculateRewardsRequest) -> Result<CalculateRewardsResponse> {
        // Validate inputs to prevent overflow
        if request.duration_hours < 0.0 || request.duration_hours > 24.0 {
            return Err(anyhow::anyhow!("Invalid duration_hours: must be between 0 and 24"));
        }

        let base_rate = self.config.base_rate_per_hour;
        let duration = Decimal::from_f64_retain(request.duration_hours).unwrap_or(dec!(1));

        // Safe decimal conversions with validation
        let cpu_usage = Decimal::from_f64_retain(request.resource_metrics.cpu_usage.clamp(0.0, 1.0)).unwrap_or(dec!(0));
        let memory_usage = Decimal::from_f64_retain(request.resource_metrics.memory_usage.clamp(0.0, 1.0)).unwrap_or(dec!(0));
        let storage_usage = Decimal::from_f64_retain(request.resource_metrics.storage_usage.clamp(0.0, 1.0)).unwrap_or(dec!(0));
        let bandwidth_usage = Decimal::from_f64_retain(request.resource_metrics.bandwidth_usage.clamp(0.0, 10.0)).unwrap_or(dec!(0)); // Allow up to 10x normal bandwidth

        // Calculate rewards for each resource type with overflow protection
        let cpu_rewards = base_rate
            .checked_mul(self.config.cpu_multiplier)
            .and_then(|x| x.checked_mul(cpu_usage))
            .and_then(|x| x.checked_mul(duration))
            .unwrap_or(dec!(0));

        let memory_rewards = base_rate
            .checked_mul(self.config.memory_multiplier)
            .and_then(|x| x.checked_mul(memory_usage))
            .and_then(|x| x.checked_mul(duration))
            .unwrap_or(dec!(0));

        let storage_rewards = base_rate
            .checked_mul(self.config.storage_multiplier)
            .and_then(|x| x.checked_mul(storage_usage))
            .and_then(|x| x.checked_mul(duration))
            .unwrap_or(dec!(0));

        let bandwidth_rewards = base_rate
            .checked_mul(bandwidth_usage)
            .and_then(|x| x.checked_mul(duration))
            .unwrap_or(dec!(0));

        // Calculate base rewards with overflow protection
        let base_rewards = base_rate.checked_mul(duration).unwrap_or(dec!(0));

        // Calculate bonus rewards based on uptime with overflow protection
        let uptime_bonus = if request.resource_metrics.uptime_hours > 23.0 {
            base_rewards.checked_mul(dec!(0.1)).unwrap_or(dec!(0)) // 10% bonus for high uptime
        } else {
            dec!(0)
        };

        // Sum with overflow protection
        let total = cpu_rewards
            .checked_add(memory_rewards).unwrap_or(dec!(0))
            .checked_add(storage_rewards).unwrap_or(dec!(0))
            .checked_add(bandwidth_rewards).unwrap_or(dec!(0))
            .checked_add(base_rewards).unwrap_or(dec!(0))
            .checked_add(uptime_bonus).unwrap_or(dec!(0));

        // Create multiplier info
        let multipliers = vec![
            MultiplierInfo {
                multiplier_type: "cpu".to_string(),
                value: self.config.cpu_multiplier,
                reason: "CPU sharing multiplier".to_string(),
            },
            MultiplierInfo {
                multiplier_type: "memory".to_string(),
                value: self.config.memory_multiplier,
                reason: "Memory sharing multiplier".to_string(),
            },
            MultiplierInfo {
                multiplier_type: "storage".to_string(),
                value: self.config.storage_multiplier,
                reason: "Storage sharing multiplier".to_string(),
            },
            MultiplierInfo {
                multiplier_type: "uptime".to_string(),
                value: if uptime_bonus > dec!(0) { dec!(1.1) } else { dec!(1.0) },
                reason: "Uptime bonus multiplier".to_string(),
            },
        ];

        Ok(CalculateRewardsResponse {
            estimated_rewards: total,
            breakdown: RewardBreakdown {
                cpu_rewards,
                memory_rewards,
                storage_rewards,
                bandwidth_rewards,
                base_rewards,
                bonus_rewards: uptime_bonus,
                total,
            },
            multipliers_applied: multipliers,
        })
    }

    /// Claim rewards for a wallet
    pub async fn claim_rewards(&self, wallet_id: &str) -> Result<Decimal> {
        // Get unclaimed rewards
        let rewards = self.storage.get_wallet_rewards(wallet_id).await?;
        let claimed: Decimal = rewards
            .iter()
            .filter(|r| !r.claimed)
            .map(|r| r.amount)
            .sum();

        if claimed > Decimal::ZERO {
            // Update wallet balance
            let wallet = self.storage.get_wallet(wallet_id).await?
                .ok_or_else(|| anyhow::anyhow!("Wallet not found"))?;
            let new_balance = wallet.balance + claimed;
            self.storage.update_balance(wallet_id, new_balance).await?;

            // Note: In production, we would mark rewards as claimed
            // For now, we just create the transaction record
        }

        // Clear cache
        {
            let mut cache = self.pending_cache.write().await;
            cache.remove(wallet_id);
        }

        // Create transaction record
        let transaction = Transaction {
            transaction_id: format!("TX_{}", Uuid::new_v4()),
            from_wallet: "SYSTEM_REWARDS".to_string(),
            to_wallet: wallet_id.to_string(),
            amount: claimed,
            transaction_type: TransactionType::Reward,
            status: TransactionStatus::Completed,
            fee: dec!(0),
            description: "Claimed pending rewards".to_string(),
            timestamp: Utc::now(),
            created_at: Utc::now(),
        };

        self.storage.create_transaction(transaction).await?;

        info!("Claimed {} CSR rewards for wallet {}", claimed, wallet_id);
        Ok(claimed)
    }

    /// Get next payout time
    pub fn get_next_payout_time(&self) -> DateTime<Utc> {
        // Payouts every 4 hours
        let now = Utc::now();
        let next_hour = ((now.time().hour() / 4) + 1) * 4;

        if next_hour >= 24 {
            // Next day
            (now + Duration::days(1))
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(Utc)
                .unwrap()
        } else {
            now.date_naive()
                .and_hms_opt(next_hour, 0, 0)
                .unwrap()
                .and_local_timezone(Utc)
                .unwrap()
        }
    }

    /// Process automated reward distribution
    pub async fn distribute_rewards(&self, wallet_id: &str, resource_metrics: ResourceMetrics) -> Result<()> {
        // Calculate rewards
        let request = CalculateRewardsRequest {
            wallet_id: wallet_id.to_string(),
            resource_metrics: resource_metrics.clone(),
            duration_hours: 1.0,
        };

        let response = self.calculate(request).await?;

        // Create reward entry
        let reward = RewardEntry {
            reward_id: format!("RWD_{}", Uuid::new_v4()),
            wallet_id: wallet_id.to_string(),
            amount: response.estimated_rewards,
            reward_type: RewardType::ResourceSharing,
            source: RewardSource {
                source_type: "automated".to_string(),
                description: "Hourly resource sharing rewards".to_string(),
                multiplier: dec!(1),
                resource_metrics: Some(resource_metrics),
            },
            timestamp: Utc::now(),
            claimed: false,
        };

        self.storage.create_reward(reward).await?;

        // Invalidate cache
        {
            let mut cache = self.pending_cache.write().await;
            cache.remove(wallet_id);
        }

        debug!("Distributed {} CSR rewards to wallet {}", response.estimated_rewards, wallet_id);
        Ok(())
    }
}