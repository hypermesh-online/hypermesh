// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Caesar Economic System - Data Models

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ============ Wallet Models ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub wallet_id: String,
    pub user_id: String,
    pub balance: Decimal,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletResponse {
    pub wallet_id: String,
    pub balance: Decimal,
    pub pending_rewards: Decimal,
    pub total_value_usd: Decimal,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWalletRequest {
    pub user_id: String,
    pub initial_balance: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub available: Decimal,
    pub pending: Decimal,
    pub total: Decimal,
    pub updated_at: DateTime<Utc>,
}

// ============ Transaction Models ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub transaction_id: String,
    pub from_wallet: String,
    pub to_wallet: String,
    pub amount: Decimal,
    pub transaction_type: TransactionType,
    pub status: TransactionStatus,
    pub fee: Decimal,
    pub description: String,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Transfer,
    Reward,
    Staking,
    Unstaking,
    Fee,
    Exchange,
    Payment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    Pending,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub transaction_id: String,
    pub from_wallet: String,
    pub to_wallet: String,
    pub amount: Decimal,
    pub transaction_type: TransactionType,
    pub status: TransactionStatus,
    pub fee: Decimal,
    pub description: String,
    pub timestamp: DateTime<Utc>,
    pub block_height: Option<u64>,
    pub confirmation_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionsResponse {
    pub wallet_id: String,
    pub transactions: Vec<Transaction>,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTransactionRequest {
    pub from_wallet: String,
    pub to_wallet: String,
    pub amount: Decimal,
    pub description: Option<String>,
}

// ============ Reward Models ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardEntry {
    pub reward_id: String,
    pub wallet_id: String,
    pub amount: Decimal,
    pub reward_type: RewardType,
    pub source: RewardSource,
    pub timestamp: DateTime<Utc>,
    pub claimed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RewardType {
    ResourceSharing,
    NetworkValidation,
    AssetHosting,
    ReferralBonus,
    ActivityBonus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardSource {
    pub source_type: String,
    pub description: String,
    pub multiplier: Decimal,
    pub resource_metrics: Option<ResourceMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub storage_usage: f64,
    pub bandwidth_usage: f64,
    pub uptime_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardsResponse {
    pub wallet_id: String,
    pub pending_rewards: Decimal,
    pub earned_today: Decimal,
    pub earning_sources: Vec<EarningSource>,
    pub next_payout: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarningSource {
    pub source_type: String,
    pub description: String,
    pub amount_today: Decimal,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimRewardsRequest {
    pub wallet_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimRewardsResponse {
    pub wallet_id: String,
    pub claimed_amount: Decimal,
    pub transaction_id: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardHistoryResponse {
    pub wallet_id: String,
    pub history: Vec<RewardEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculateRewardsRequest {
    pub wallet_id: String,
    pub resource_metrics: ResourceMetrics,
    pub duration_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculateRewardsResponse {
    pub estimated_rewards: Decimal,
    pub breakdown: RewardBreakdown,
    pub multipliers_applied: Vec<MultiplierInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardBreakdown {
    pub cpu_rewards: Decimal,
    pub memory_rewards: Decimal,
    pub storage_rewards: Decimal,
    pub bandwidth_rewards: Decimal,
    pub base_rewards: Decimal,
    pub bonus_rewards: Decimal,
    pub total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiplierInfo {
    pub multiplier_type: String,
    pub value: Decimal,
    pub reason: String,
}

// ============ Exchange Models ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRate {
    pub from_token: String,
    pub to_token: String,
    pub rate: Decimal,
    pub inverse_rate: Decimal,
    pub timestamp: DateTime<Utc>,
    pub volume_24h: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRatesResponse {
    pub rates: Vec<ExchangeRate>,
    pub base_currency: String,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapRequest {
    pub wallet_id: String,
    pub from_token: String,
    pub to_token: String,
    pub amount: Decimal,
    pub slippage_tolerance: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapResponse {
    pub swap_id: String,
    pub wallet_id: String,
    pub from_token: String,
    pub to_token: String,
    pub from_amount: Decimal,
    pub to_amount: Decimal,
    pub rate: Decimal,
    pub fee: Decimal,
    pub slippage: Decimal,
    pub transaction_id: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityInfoResponse {
    pub total_liquidity: Decimal,
    pub csr_liquidity: Decimal,
    pub usd_liquidity: Decimal,
    pub volume_24h: Decimal,
    pub fee_24h: Decimal,
    pub apy: Decimal,
}

// ============ System Models ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub status: SystemStatus,
    pub latency_ms: f64,
    pub transactions_per_second: f64,
    pub active_connections: u64,
    pub memory_usage_mb: f64,
    pub last_block_height: u64,
    pub last_block_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemStatus {
    Healthy,
    Degraded,
    Critical,
    Maintenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error_code: String,
    pub message: String,
    pub details: Option<String>,
    pub timestamp: DateTime<Utc>,
}