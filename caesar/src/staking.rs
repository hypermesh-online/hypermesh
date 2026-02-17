// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Caesar Staking System - Token staking with APY rewards

use anyhow::{Result, anyhow};
use chrono::{Utc, Duration};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use rust_decimal::prelude::ToPrimitive;
use std::sync::Arc;
use tracing::{info, debug};
use uuid::Uuid;

use crate::models::*;
use crate::storage::CaesarStorage;
use crate::StakingConfig;

/// Staking manager for token staking and rewards
pub struct StakingManager {
    config: StakingConfig,
    storage: Arc<CaesarStorage>,
}

impl StakingManager {
    pub async fn new(config: StakingConfig, storage: Arc<CaesarStorage>) -> Result<Self> {
        Ok(Self { config, storage })
    }

    /// Get all active stakes for a wallet
    pub async fn get_stakes(&self, wallet_id: &str) -> Result<Vec<StakeInfo>> {
        self.storage.get_stakes(wallet_id).await
    }

    /// Get total staked amount for a wallet
    pub async fn get_staked_amount(&self, wallet_id: &str) -> Result<Decimal> {
        self.storage.get_total_staked(wallet_id).await
    }

    /// Get current APY rate
    pub fn get_current_apy(&self) -> Decimal {
        self.config.base_apy
    }

    /// Calculate accumulated staking rewards
    pub async fn calculate_rewards(&self, wallet_id: &str) -> Result<Decimal> {
        let stakes = self.get_stakes(wallet_id).await?;
        let mut total_rewards = dec!(0);

        for stake in stakes {
            let days_staked = (Utc::now() - stake.start_date).num_days() as i32;
            if days_staked <= 0 {
                continue;
            }

            // Calculate compound interest
            // A = P(1 + r/n)^(nt)
            // where P = principal, r = annual rate, n = compound frequency, t = time in years
            let principal = stake.amount;
            let rate = stake.apy / dec!(100); // Convert percentage to decimal
            let compounds_per_year = dec!(365) / Decimal::from(self.config.compound_frequency_hours) * dec!(24);
            let time_in_years = Decimal::from(days_staked) / dec!(365);

            let compound_factor = dec!(1) + (rate / compounds_per_year);
            let exponent = compounds_per_year * time_in_years;

            // Approximate compound interest calculation
            let final_amount = principal * Self::power_approximation(compound_factor, exponent);
            let rewards = final_amount - principal;

            total_rewards += rewards;
        }

        Ok(total_rewards)
    }

    /// Power approximation for compound interest
    fn power_approximation(base: Decimal, exponent: Decimal) -> Decimal {
        // Simple approximation using Taylor series
        // For more accuracy, use proper math library
        let exp_f64 = exponent.to_f64().unwrap_or(1.0);
        let base_f64 = base.to_f64().unwrap_or(1.0);
        let result = base_f64.powf(exp_f64);
        Decimal::from_f64_retain(result).unwrap_or(base)
    }

    /// Stake tokens
    pub async fn stake(&self, request: StakeRequest) -> Result<StakeResponse> {
        // Validate amount
        if request.amount < self.config.min_stake {
            return Err(anyhow!("Amount below minimum stake of {}", self.config.min_stake));
        }

        let current_staked = self.get_staked_amount(&request.wallet_id).await?;
        if current_staked + request.amount > self.config.max_stake {
            return Err(anyhow!("Would exceed maximum stake of {}", self.config.max_stake));
        }

        // Check wallet balance
        let wallet = self.storage.get_wallet(&request.wallet_id).await?
            .ok_or_else(|| anyhow!("Wallet not found"))?;
        let balance = wallet.balance;
        if balance < request.amount {
            return Err(anyhow!("Insufficient balance"));
        }

        // Calculate APY based on lock period
        let apy = if let Some(lock_days) = request.lock_period_days {
            // Higher APY for longer lock periods
            let bonus = Decimal::from(lock_days) / dec!(365) * dec!(2); // Up to 2% bonus for 1 year lock
            self.config.base_apy + bonus
        } else {
            self.config.base_apy
        };

        // Create stake
        let stake_id = format!("STK_{}", Uuid::new_v4());
        let stake = StakeInfo {
            stake_id: stake_id.clone(),
            wallet_id: request.wallet_id.clone(),
            amount: request.amount,
            start_date: Utc::now(),
            lock_period_days: request.lock_period_days,
            apy,
            accumulated_rewards: dec!(0),
            is_active: true,
        };

        self.storage.create_stake(stake).await?;

        // Update wallet balance
        self.storage.update_balance(&request.wallet_id, balance - request.amount).await?;

        // Calculate estimated rewards
        let days = request.lock_period_days.unwrap_or(365);
        let time_in_years = Decimal::from(days) / dec!(365);
        let estimated_rewards = request.amount * (apy / dec!(100)) * time_in_years;

        // Create transaction record
        let transaction = Transaction {
            transaction_id: format!("TX_{}", Uuid::new_v4()),
            from_wallet: request.wallet_id.clone(),
            to_wallet: "STAKING_POOL".to_string(),
            amount: request.amount,
            transaction_type: TransactionType::Staking,
            status: TransactionStatus::Completed,
            fee: dec!(0),
            description: format!("Staked {} CSR for {} days", request.amount, days),
            timestamp: Utc::now(),
            created_at: Utc::now(),
        };

        self.storage.create_transaction(transaction.clone()).await?;

        let lock_until = request.lock_period_days
            .map(|days| Utc::now() + Duration::days(days as i64));

        info!("Wallet {} staked {} CSR at {}% APY", request.wallet_id, request.amount, apy);

        Ok(StakeResponse {
            stake_id,
            wallet_id: request.wallet_id,
            amount: request.amount,
            apy,
            estimated_rewards,
            lock_until,
            transaction_id: transaction.transaction_id,
        })
    }

    /// Unstake tokens
    pub async fn unstake(&self, request: UnstakeRequest) -> Result<UnstakeResponse> {
        let stakes = self.get_stakes(&request.wallet_id).await?;

        let stake = stakes
            .iter()
            .find(|s| s.stake_id == request.stake_id)
            .ok_or_else(|| anyhow!("Stake not found"))?;

        // Check lock period
        if let Some(lock_days) = stake.lock_period_days {
            let unlock_date = stake.start_date + Duration::days(lock_days as i64);
            if Utc::now() < unlock_date {
                return Err(anyhow!("Stake is locked until {}", unlock_date));
            }
        }

        let amount_to_unstake = request.amount.unwrap_or(stake.amount);
        if amount_to_unstake > stake.amount {
            return Err(anyhow!("Cannot unstake more than staked amount"));
        }

        // Calculate rewards
        let days_staked = (Utc::now() - stake.start_date).num_days() as i32;
        let time_in_years = Decimal::from(days_staked.max(1)) / dec!(365);
        let rewards = amount_to_unstake * (stake.apy / dec!(100)) * time_in_years;

        // Update or deactivate stake
        if amount_to_unstake >= stake.amount {
            self.storage.deactivate_stake(&stake.stake_id).await?;
        } else {
            // Partial unstake not implemented yet, would update stake amount
            return Err(anyhow!("Partial unstaking not yet supported"));
        }

        // Return funds to wallet
        let wallet = self.storage.get_wallet(&request.wallet_id).await?
            .ok_or_else(|| anyhow!("Wallet not found"))?;
        let balance = wallet.balance;
        let total_return = amount_to_unstake + rewards;
        self.storage.update_balance(&request.wallet_id, balance + total_return).await?;

        // Create transaction record
        let transaction = Transaction {
            transaction_id: format!("TX_{}", Uuid::new_v4()),
            from_wallet: "STAKING_POOL".to_string(),
            to_wallet: request.wallet_id.clone(),
            amount: total_return,
            transaction_type: TransactionType::Unstaking,
            status: TransactionStatus::Completed,
            fee: dec!(0),
            description: format!("Unstaked {} CSR with {} CSR rewards", amount_to_unstake, rewards),
            timestamp: Utc::now(),
            created_at: Utc::now(),
        };

        self.storage.create_transaction(transaction.clone()).await?;

        let cooldown_ends = Utc::now() + Duration::hours(self.config.unstaking_cooldown_hours as i64);

        info!("Wallet {} unstaked {} CSR with {} CSR rewards", request.wallet_id, amount_to_unstake, rewards);

        Ok(UnstakeResponse {
            wallet_id: request.wallet_id,
            unstaked_amount: amount_to_unstake,
            rewards_claimed: rewards,
            transaction_id: transaction.transaction_id,
            cooldown_ends,
        })
    }

    /// Get rewards breakdown for all stakes
    pub async fn get_rewards_breakdown(&self, wallet_id: &str) -> Result<Vec<StakeRewardBreakdown>> {
        let stakes = self.get_stakes(wallet_id).await?;
        let mut breakdown = Vec::new();

        for stake in stakes {
            let days_staked = (Utc::now() - stake.start_date).num_days() as u32;
            let time_in_years = Decimal::from(days_staked) / dec!(365);
            let rewards = stake.amount * (stake.apy / dec!(100)) * time_in_years;

            breakdown.push(StakeRewardBreakdown {
                stake_id: stake.stake_id,
                principal: stake.amount,
                rewards,
                apy: stake.apy,
                days_staked,
            });
        }

        Ok(breakdown)
    }

    /// Process automated staking rewards distribution
    pub async fn distribute_staking_rewards(&self) -> Result<()> {
        // This would be called periodically to compound rewards
        // For now, rewards are calculated on-demand
        debug!("Processing staking rewards distribution");
        Ok(())
    }
}