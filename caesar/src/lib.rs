// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Caesar Economic System
//!
//! Complete economic backend for HyperMesh providing:
//! - Real-time token balance tracking
//! - Transaction processing and validation
//! - Reward calculation based on resource sharing
//! - Exchange operations and market rates
//! - EVP (Economic Value Packet) protocol
//! - Governor PID controller for monetary policy
//! - UPI (Universal Payment Interface) ingress/egress
//! - Settlement protocol with gravity-based clearing
//!
//! **API**: STOQ protocol (HTTP REMOVED)

use anyhow::Result;
// REMOVED: HTTP dependencies (axum)
// Migrated to STOQ protocol - see api/stoq_api.rs
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;
use std::collections::HashMap;

// HyperMesh Asset System integration
#[cfg(feature = "hypermesh")]
use blockmatrix::assets::core::{AssetManager, AssetId, AssetType, ConsensusProof, AssetAllocationRequest, ResourceRequirements};

pub mod models;
pub mod storage;
pub mod rewards;
pub mod exchange;
pub mod transactions;
pub mod banking_interop_bridge;
pub mod banking_providers;
pub mod crypto_exchange_providers;
pub mod cross_chain_bridge;
pub mod api;
pub mod evp;
pub mod governor;
pub mod upi;
pub mod settlement;

use models::*;
use storage::CaesarStorage;
use rewards::RewardCalculator;
use exchange::ExchangeEngine;
use transactions::TransactionProcessor;
use cross_chain_bridge::CrossChainBridge;

/// Caesar Economic System Configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CaesarConfig {
    /// Token economics configuration
    pub economics: EconomicsConfig,

    /// Reward calculation settings
    pub rewards: RewardConfig,

    /// Exchange settings
    pub exchange: ExchangeConfig,

    /// Database configuration
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EconomicsConfig {
    /// Total token supply
    pub total_supply: Decimal,

    /// Initial distribution percentage
    pub initial_distribution: Decimal,

    /// Minimum transaction amount
    pub min_transaction: Decimal,

    /// Maximum transaction amount
    pub max_transaction: Decimal,

    /// Transaction fee percentage
    pub transaction_fee: Decimal,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RewardConfig {
    /// Base reward rate per hour for resource sharing
    pub base_rate_per_hour: Decimal,

    /// CPU sharing multiplier
    pub cpu_multiplier: Decimal,

    /// Memory sharing multiplier
    pub memory_multiplier: Decimal,

    /// Storage sharing multiplier
    pub storage_multiplier: Decimal,

    /// Network validation multiplier
    pub validation_multiplier: Decimal,

    /// Asset hosting multiplier
    pub hosting_multiplier: Decimal,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExchangeConfig {
    /// CSR to USD exchange rate
    pub csr_usd_rate: Decimal,

    /// Market volatility percentage
    pub volatility: Decimal,

    /// Liquidity pool size
    pub liquidity_pool: Decimal,

    /// Slippage tolerance percentage
    pub slippage_tolerance: Decimal,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    /// Database URL
    pub url: String,

    /// Redis cache URL (optional)
    pub redis_url: Option<String>,

    /// Connection pool size
    pub pool_size: u32,
}

/// Main Caesar Economic System
#[allow(dead_code)] // System fields used across economic operations
pub struct CaesarEconomicSystem {
    /// Configuration
    config: Arc<CaesarConfig>,

    /// Storage layer
    storage: Arc<CaesarStorage>,

    /// Reward calculator
    rewards: Arc<RewardCalculator>,

    /// Exchange engine
    exchange: Arc<ExchangeEngine>,

    /// Transaction processor
    transactions: Arc<TransactionProcessor>,

    /// Cross-chain bridge for "mostly-stable" token
    bridge: Arc<CrossChainBridge>,

    /// Active sessions cache
    sessions: Arc<RwLock<HashMap<String, UserSession>>>,

    /// HyperMesh Asset Manager integration
    #[cfg(feature = "hypermesh")]
    asset_manager: Option<Arc<AssetManager>>,
}

/// User session for real-time data
#[derive(Debug, Clone)]
pub struct UserSession {
    pub wallet_id: String,
    pub last_update: DateTime<Utc>,
    pub cached_balance: Decimal,
    pub cached_rewards: Decimal,
}

impl CaesarEconomicSystem {
    /// Create new Caesar economic system
    pub async fn new(config: CaesarConfig) -> Result<Self> {
        info!("💰 Initializing Caesar Economic System");

        let config = Arc::new(config);

        // Initialize storage
        let storage = Arc::new(CaesarStorage::new(config.database.clone()).await?);

        // Initialize components
        let rewards = Arc::new(RewardCalculator::new(config.rewards.clone(), storage.clone()));
        let exchange = Arc::new(ExchangeEngine::new(config.exchange.clone()));
        let transactions = Arc::new(TransactionProcessor::new(config.economics.clone(), storage.clone()).await?);
        let bridge = Arc::new(CrossChainBridge::new().await?);

        let sessions = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            config,
            storage,
            rewards,
            exchange,
            transactions,
            bridge,
            sessions,
            #[cfg(feature = "hypermesh")]
            asset_manager: None, // Will be set when integrated with HyperMesh
        })
    }

    // HTTP REMOVED: Migrated to STOQ protocol
    // All API endpoints now available through api::stoq_api::CaesarStoqApi
    // See /src/api/stoq_api.rs for STOQ-based API implementation

    // Public implementation methods for integration

    pub async fn get_wallet_info(&self, wallet_id: &str) -> Result<WalletResponse> {
        let wallet = self.storage.get_wallet(wallet_id).await?
            .ok_or_else(|| anyhow::anyhow!("Wallet not found"))?;
        let balance = wallet.balance;
        let pending_rewards = self.rewards.get_pending_rewards(wallet_id).await?;

        // Calculate USD value
        let total_csr = balance + pending_rewards;
        let usd_value = self.exchange.calculate_usd_value(total_csr)?;

        Ok(WalletResponse {
            wallet_id: wallet_id.to_string(),
            balance,
            pending_rewards,
            total_value_usd: usd_value,
            created_at: wallet.created_at,
            last_activity: wallet.last_activity,
        })
    }

    pub async fn get_wallet_balance(&self, wallet_id: &str) -> Result<BalanceResponse> {
        let wallet = self.storage.get_wallet(wallet_id).await?
            .ok_or_else(|| anyhow::anyhow!("Wallet not found"))?;
        let balance = wallet.balance;
        let pending = self.rewards.get_pending_rewards(wallet_id).await?;

        Ok(BalanceResponse {
            available: balance,
            pending,
            total: balance + pending,
            updated_at: Utc::now(),
        })
    }

    pub async fn create_new_wallet(&self, request: CreateWalletRequest) -> Result<WalletResponse> {
        let wallet = self.storage.create_wallet(request.user_id).await?;
        self.get_wallet_info(&wallet.wallet_id).await
    }

    pub async fn get_wallet_transactions(&self, wallet_id: &str) -> Result<TransactionsResponse> {
        let transactions = self.storage.get_wallet_transactions(wallet_id).await?;
        let total_count = transactions.len();

        Ok(TransactionsResponse {
            wallet_id: wallet_id.to_string(),
            transactions,
            total_count,
        })
    }

    pub async fn get_transaction_details(&self, tx_id: &str) -> Result<TransactionResponse> {
        let tx = self.storage.get_transaction(tx_id).await?
            .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;

        // Convert Transaction to TransactionResponse
        Ok(TransactionResponse {
            transaction_id: tx.transaction_id,
            from_wallet: tx.from_wallet,
            to_wallet: tx.to_wallet,
            amount: tx.amount,
            transaction_type: tx.transaction_type,
            status: tx.status,
            fee: tx.fee,
            description: tx.description,
            timestamp: tx.timestamp,
            block_height: None, // Not tracked in current storage
            confirmation_count: 0, // Not tracked in current storage
        })
    }

    pub async fn process_transaction(&self, request: SendTransactionRequest) -> Result<TransactionResponse> {
        self.transactions.process(request).await
    }

    pub async fn get_rewards_info(&self, wallet_id: &str) -> Result<RewardsResponse> {
        let pending = self.rewards.get_pending_rewards(wallet_id).await?;
        let earned_today = self.rewards.get_today_earnings(wallet_id).await?;
        let sources = self.rewards.get_earning_sources(wallet_id).await?;

        Ok(RewardsResponse {
            wallet_id: wallet_id.to_string(),
            pending_rewards: pending,
            earned_today,
            earning_sources: sources,
            next_payout: self.rewards.get_next_payout_time(),
        })
    }

    pub async fn claim_pending_rewards(&self, request: ClaimRewardsRequest) -> Result<ClaimRewardsResponse> {
        let claimed = self.rewards.claim_rewards(&request.wallet_id).await?;

        Ok(ClaimRewardsResponse {
            wallet_id: request.wallet_id,
            claimed_amount: claimed,
            transaction_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
        })
    }

    pub async fn get_reward_history_for_wallet(&self, wallet_id: &str) -> Result<RewardHistoryResponse> {
        let history = self.storage.get_wallet_rewards(wallet_id).await?;

        Ok(RewardHistoryResponse {
            wallet_id: wallet_id.to_string(),
            history,
        })
    }

    pub async fn calculate_resource_rewards(&self, request: CalculateRewardsRequest) -> Result<CalculateRewardsResponse> {
        let rewards = self.rewards.calculate(request).await?;

        Ok(rewards)
    }

    pub async fn get_current_exchange_rates(&self) -> Result<ExchangeRatesResponse> {
        self.exchange.get_rates().await
    }

    pub async fn execute_token_swap(&self, request: SwapRequest) -> Result<SwapResponse> {
        self.exchange.swap(request).await
    }

    pub async fn get_liquidity_pool_info(&self) -> Result<LiquidityInfoResponse> {
        self.exchange.get_liquidity_info().await
    }

}

/// Default configuration for development
impl Default for CaesarConfig {
    fn default() -> Self {
        Self {
            economics: EconomicsConfig {
                total_supply: dec!(1000000000), // 1 billion tokens
                initial_distribution: dec!(0.1), // 10% initial distribution
                min_transaction: dec!(0.01),
                max_transaction: dec!(1000000),
                transaction_fee: dec!(0.001), // 0.1% fee
            },
            rewards: RewardConfig {
                base_rate_per_hour: dec!(1.0),
                cpu_multiplier: dec!(2.0),
                memory_multiplier: dec!(1.5),
                storage_multiplier: dec!(1.2),
                validation_multiplier: dec!(3.0),
                hosting_multiplier: dec!(1.8),
            },
            exchange: ExchangeConfig {
                csr_usd_rate: dec!(1.48),
                volatility: dec!(0.05),
                liquidity_pool: dec!(10000000),
                slippage_tolerance: dec!(0.02),
            },
            database: DatabaseConfig {
                url: "sqlite::memory:".to_string(),
                redis_url: None,
                pool_size: 10,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_caesar_initialization() {
        let config = CaesarConfig::default();
        let caesar = CaesarEconomicSystem::new(config).await;
        assert!(caesar.is_ok());
    }

    #[tokio::test]
    async fn test_wallet_creation() {
        let config = CaesarConfig::default();
        let caesar = Arc::new(CaesarEconomicSystem::new(config).await.unwrap());

        let request = CreateWalletRequest {
            user_id: "test_user".to_string(),
            initial_balance: Some(dec!(100)),
        };

        let wallet = caesar.create_new_wallet(request).await;
        assert!(wallet.is_ok());
    }
}
// Type aliases for ethers compatibility
pub type BalanceAmount = rust_decimal::Decimal;
pub type SignerMiddleware = ();
pub type Address = String;
pub type Abi = String;
pub type U256 = u256;

#[allow(non_camel_case_types)] // EVM-compatible type naming convention
#[derive(Clone, Copy, Debug)]
pub struct u256(pub u128, pub u128);
