// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Caesar Analytics System - Economic metrics and analytics

use anyhow::Result;
use chrono::{Utc, Duration};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;

use crate::models::*;
use crate::storage::CaesarStorage;
use serde::{Deserialize, Serialize};

/// Analytics engine for economic metrics
pub struct AnalyticsEngine {
    storage: Arc<CaesarStorage>,
}

impl AnalyticsEngine {
    pub async fn new(storage: Arc<CaesarStorage>) -> Result<Self> {
        Ok(Self { storage })
    }

    /// Get system-wide analytics overview
    pub async fn get_overview(&self, _wallet_id: Option<&String>) -> Result<AnalyticsOverviewResponse> {
        // In production, these would be calculated from actual data
        // For now, using realistic mock data

        let total_supply = dec!(1000000000); // 1 billion tokens
        let locked_in_staking = dec!(250000000); // 25% staked
        let burned_tokens = dec!(50000000); // 5% burned
        let circulating_supply = total_supply - locked_in_staking - burned_tokens;

        let csr_price = dec!(1.48);
        let market_cap = circulating_supply * csr_price;

        // Simulated 24h metrics
        let price_24h_ago = dec!(1.42);
        let price_change = ((csr_price - price_24h_ago) / price_24h_ago) * dec!(100);

        Ok(AnalyticsOverviewResponse {
            total_supply,
            circulating_supply,
            market_cap_usd: market_cap,
            total_staked: locked_in_staking,
            total_rewards_distributed: dec!(15000000), // 1.5% of supply
            active_wallets_24h: 12543,
            transactions_24h: 45821,
            volume_24h: dec!(8500000),
            price_change_24h: price_change,
        })
    }

    /// Get earnings breakdown for a wallet
    pub async fn get_earnings_breakdown(&self, wallet_id: &str) -> Result<EarningsBreakdownResponse> {
        // Calculate earnings over different time periods
        let now = Utc::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();
        let week_ago = now - Duration::days(7);
        let _month_ago = now - Duration::days(30);

        // Get historical rewards (would query database in production)
        let history = self.storage.get_wallet_rewards(wallet_id).await?;

        // Calculate totals
        let total_earned: Decimal = history.iter().map(|r| r.amount).sum();

        let earnings_today: Decimal = history
            .iter()
            .filter(|r| r.timestamp >= today_start)
            .map(|r| r.amount)
            .sum();

        let earnings_week: Decimal = history
            .iter()
            .filter(|r| r.timestamp >= week_ago)
            .map(|r| r.amount)
            .sum();

        let earnings_month = total_earned; // Since we got 30 days of history

        // Calculate breakdown by source
        let mut source_totals = std::collections::HashMap::new();
        for reward in &history {
            let source_name = match reward.reward_type {
                RewardType::ResourceSharing => "Resource Sharing",
                RewardType::NetworkValidation => "Network Validation",
                RewardType::AssetHosting => "Asset Hosting",
                RewardType::StakingReward => "Staking Rewards",
                RewardType::ReferralBonus => "Referral Bonus",
                RewardType::ActivityBonus => "Activity Bonus",
            };
            *source_totals.entry(source_name).or_insert(dec!(0)) += reward.amount;
        }

        let mut breakdown_by_source = Vec::new();
        for (source, amount) in source_totals {
            let percentage = if total_earned > dec!(0) {
                (amount / total_earned) * dec!(100)
            } else {
                dec!(0)
            };

            // Determine trend based on recent vs older earnings
            let recent_amount: Decimal = history
                .iter()
                .filter(|r| r.timestamp >= week_ago)
                .filter(|r| match r.reward_type {
                    RewardType::ResourceSharing => source == "Resource Sharing",
                    RewardType::NetworkValidation => source == "Network Validation",
                    RewardType::AssetHosting => source == "Asset Hosting",
                    RewardType::StakingReward => source == "Staking Rewards",
                    RewardType::ReferralBonus => source == "Referral Bonus",
                    RewardType::ActivityBonus => source == "Activity Bonus",
                })
                .map(|r| r.amount)
                .sum();

            let older_amount = amount - recent_amount;

            let trend = if recent_amount > older_amount {
                TrendDirection::Up
            } else if recent_amount < older_amount {
                TrendDirection::Down
            } else {
                TrendDirection::Stable
            };

            breakdown_by_source.push(EarningsBySource {
                source: source.to_string(),
                amount,
                percentage,
                trend,
            });
        }

        // Sort by amount descending
        breakdown_by_source.sort_by(|a, b| b.amount.cmp(&a.amount));

        // Calculate rates
        let hours_in_month = dec!(720); // 30 days * 24 hours
        let hourly_rate = if hours_in_month > dec!(0) {
            earnings_month / hours_in_month
        } else {
            dec!(0)
        };

        let projected_monthly = hourly_rate * hours_in_month;

        Ok(EarningsBreakdownResponse {
            wallet_id: wallet_id.to_string(),
            total_earned,
            earnings_today,
            earnings_week,
            earnings_month,
            breakdown_by_source,
            hourly_rate,
            projected_monthly,
        })
    }

    /// Get network statistics
    pub async fn get_network_stats(&self) -> Result<NetworkStatistics> {
        Ok(NetworkStatistics {
            total_nodes: 1523,
            active_nodes: 1489,
            total_resources_shared: ResourceMetrics {
                cpu_usage: 45.2,
                memory_usage: 62.8,
                storage_usage: 78.4,
                bandwidth_usage: 34.6,
                uptime_hours: 23.7,
            },
            average_node_earnings: dec!(24.5),
            network_utilization: 67.3,
            consensus_participation: 89.2,
        })
    }

    /// Get market depth and liquidity metrics
    pub async fn get_market_depth(&self) -> Result<MarketDepth> {
        Ok(MarketDepth {
            bid_liquidity: dec!(5000000),
            ask_liquidity: dec!(4800000),
            spread: dec!(0.002), // 0.2%
            depth_10_percent: dec!(2500000),
            slippage_100k: dec!(0.015), // 1.5% slippage for 100k trade
        })
    }

    /// Get staking analytics
    pub async fn get_staking_analytics(&self) -> Result<StakingAnalytics> {
        Ok(StakingAnalytics {
            total_staked: dec!(250000000),
            average_apy: dec!(4.2),
            total_stakers: 45821,
            average_stake_size: dec!(5456),
            lock_distribution: vec![
                LockPeriodDistribution {
                    days: 0,
                    amount: dec!(50000000),
                    percentage: dec!(20),
                },
                LockPeriodDistribution {
                    days: 30,
                    amount: dec!(75000000),
                    percentage: dec!(30),
                },
                LockPeriodDistribution {
                    days: 90,
                    amount: dec!(62500000),
                    percentage: dec!(25),
                },
                LockPeriodDistribution {
                    days: 365,
                    amount: dec!(62500000),
                    percentage: dec!(25),
                },
            ],
        })
    }
}

// Additional analytics models

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatistics {
    pub total_nodes: u64,
    pub active_nodes: u64,
    pub total_resources_shared: ResourceMetrics,
    pub average_node_earnings: Decimal,
    pub network_utilization: f64,
    pub consensus_participation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDepth {
    pub bid_liquidity: Decimal,
    pub ask_liquidity: Decimal,
    pub spread: Decimal,
    pub depth_10_percent: Decimal,
    pub slippage_100k: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingAnalytics {
    pub total_staked: Decimal,
    pub average_apy: Decimal,
    pub total_stakers: u64,
    pub average_stake_size: Decimal,
    pub lock_distribution: Vec<LockPeriodDistribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockPeriodDistribution {
    pub days: u32,
    pub amount: Decimal,
    pub percentage: Decimal,
}