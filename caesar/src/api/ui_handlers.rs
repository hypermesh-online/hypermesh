// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! UI-facing Caesar STOQ API handlers.
//!
//! Adapts the packet-centric EVP protocol into the wallet/balance/transaction
//! response shapes expected by the UI TypeScript interfaces in
//! `ui/frontend/lib/api/services/CaesarAPI.ts`.
//!
//! Caesar has no persistent wallets or balances -- value exists only in-flight
//! as ephemeral packets. These handlers expose the EVP state through a
//! UI-compatible lens.

use anyhow::Result;
use async_trait::async_trait;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

use stoq::api::{ApiError, ApiHandler, ApiRequest, ApiResponse};

use super::stoq_api::CaesarAppState;

// ---------------------------------------------------------------------------
// UI response types (aligned with CaesarAPI.ts interfaces)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletResponse {
    pub success: bool,
    pub wallet: Option<WalletInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    pub id: String,
    pub address: String,
    pub balance: f64,
    pub locked_balance: f64,
    pub pending_rewards: f64,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub total: f64,
    pub available: f64,
    pub locked: f64,
    pub pending: f64,
    pub staked: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionsResponse {
    pub transactions: Vec<TransactionEntry>,
    pub total: u64,
    pub page: u64,
    pub limit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub tx_type: String,
    pub from_wallet: String,
    pub to_wallet: String,
    pub amount: f64,
    pub fee: f64,
    pub status: String,
    pub timestamp: u64,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardsInfo {
    pub total_earned: f64,
    pub pending_rewards: f64,
    pub claimed_rewards: f64,
    pub last_claim: u64,
    pub entries: Vec<RewardEntry>,
    pub daily_rate: f64,
    pub multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub reward_type: String,
    pub amount: f64,
    pub timestamp: u64,
    pub status: String,
    pub source: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingInfo {
    pub total_staked: f64,
    pub active_stakes: Vec<StakePosition>,
    pub available_to_stake: f64,
    pub total_rewards: f64,
    pub apy: f64,
    pub min_stake_amount: f64,
    pub lock_periods: Vec<LockPeriod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakePosition {
    pub id: String,
    pub amount: f64,
    pub lock_period_days: u64,
    pub started_at: u64,
    pub unlock_at: u64,
    pub apy: f64,
    pub rewards_earned: f64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockPeriod {
    pub days: u64,
    pub apy: f64,
    pub min_amount: f64,
    pub max_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRatesResponse {
    pub csr_to_usd: f64,
    pub csr_to_btc: f64,
    pub csr_to_eth: f64,
    pub last_updated: u64,
    pub volume_24h: f64,
    pub change_24h: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsResponse {
    pub total_supply: f64,
    pub circulating_supply: f64,
    pub market_cap: f64,
    pub holders: u64,
    pub transactions_24h: u64,
    pub volume_24h: f64,
    pub average_transaction: f64,
    pub network_activity: NetworkActivity,
    pub staking_metrics: StakingMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkActivity {
    pub active_nodes: u64,
    pub total_resources_shared: u64,
    pub rewards_distributed_24h: f64,
    pub new_users_24h: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingMetrics {
    pub total_staked: f64,
    pub staking_ratio: f64,
    pub average_lock_period: f64,
    pub total_stakers: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarningsResponse {
    pub total_earnings: f64,
    pub earnings_24h: f64,
    pub earnings_7d: f64,
    pub earnings_30d: f64,
    pub breakdown: Vec<EarningsBreakdown>,
    pub projection_daily: f64,
    pub projection_monthly: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarningsBreakdown {
    pub source: String,
    pub amount: f64,
    pub percentage: f64,
    pub trend: String,
}

// ---------------------------------------------------------------------------
// Helper: current unix timestamp
// ---------------------------------------------------------------------------

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ---------------------------------------------------------------------------
// Wallet handler
// ---------------------------------------------------------------------------

pub struct WalletHandler {
    pub state: Arc<CaesarAppState>,
}

#[async_trait]
impl ApiHandler for WalletHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling caesar/wallet: {}", request.id);

        let protocol = self.state.protocol.read().await;
        let in_transit = protocol
            .in_transit_value()
            .await
            .map(|g| g.0.to_f64().unwrap_or(0.0))
            .unwrap_or(0.0);

        let now = now_unix();
        let response = WalletResponse {
            success: true,
            wallet: Some(WalletInfo {
                id: "evp-node-wallet".to_string(),
                address: "evp://local".to_string(),
                balance: in_transit,
                locked_balance: 0.0,
                pending_rewards: 0.0,
                created_at: now.saturating_sub(86400),
                updated_at: now,
            }),
            error: None,
        };

        serialize_response(&request.id, &response)
    }

    fn path(&self) -> &str {
        "caesar/wallet"
    }
}

// ---------------------------------------------------------------------------
// Balance handler
// ---------------------------------------------------------------------------

pub struct BalanceHandler {
    pub state: Arc<CaesarAppState>,
}

#[async_trait]
impl ApiHandler for BalanceHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling caesar/balance: {}", request.id);

        let protocol = self.state.protocol.read().await;
        let in_transit = protocol
            .in_transit_value()
            .await
            .map(|g| g.0.to_f64().unwrap_or(0.0))
            .unwrap_or(0.0);

        let response = BalanceResponse {
            total: in_transit,
            available: in_transit,
            locked: 0.0,
            pending: 0.0,
            staked: 0.0,
        };

        serialize_response(&request.id, &response)
    }

    fn path(&self) -> &str {
        "caesar/balance"
    }
}

// ---------------------------------------------------------------------------
// Transactions handler
// ---------------------------------------------------------------------------

pub struct TransactionsHandler {
    pub state: Arc<CaesarAppState>,
}

#[async_trait]
impl ApiHandler for TransactionsHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling caesar/transactions: {}", request.id);

        // Caesar EVP packets are ephemeral -- no persistent transaction history.
        // Return empty list for now; future: stream settled packets as tx history.
        let response = TransactionsResponse {
            transactions: Vec::new(),
            total: 0,
            page: 1,
            limit: 50,
        };

        serialize_response(&request.id, &response)
    }

    fn path(&self) -> &str {
        "caesar/transactions"
    }
}

// ---------------------------------------------------------------------------
// Rewards handler
// ---------------------------------------------------------------------------

pub struct RewardsHandler {
    pub state: Arc<CaesarAppState>,
}

#[async_trait]
impl ApiHandler for RewardsHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling caesar/rewards: {}", request.id);

        let response = RewardsInfo {
            total_earned: 0.0,
            pending_rewards: 0.0,
            claimed_rewards: 0.0,
            last_claim: 0,
            entries: Vec::new(),
            daily_rate: 0.0,
            multiplier: 1.0,
        };

        serialize_response(&request.id, &response)
    }

    fn path(&self) -> &str {
        "caesar/rewards"
    }
}

// ---------------------------------------------------------------------------
// Staking handler
// ---------------------------------------------------------------------------

pub struct StakingHandler {
    pub state: Arc<CaesarAppState>,
}

#[async_trait]
impl ApiHandler for StakingHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling caesar/staking: {}", request.id);

        let response = StakingInfo {
            total_staked: 0.0,
            active_stakes: Vec::new(),
            available_to_stake: 0.0,
            total_rewards: 0.0,
            apy: 0.0,
            min_stake_amount: 0.001,
            lock_periods: vec![
                LockPeriod {
                    days: 30,
                    apy: 3.0,
                    min_amount: 0.001,
                    max_amount: 1000.0,
                },
                LockPeriod {
                    days: 90,
                    apy: 5.0,
                    min_amount: 0.001,
                    max_amount: 1000.0,
                },
                LockPeriod {
                    days: 365,
                    apy: 8.0,
                    min_amount: 0.001,
                    max_amount: 1000.0,
                },
            ],
        };

        serialize_response(&request.id, &response)
    }

    fn path(&self) -> &str {
        "caesar/staking"
    }
}

// ---------------------------------------------------------------------------
// Exchange rates handler
// ---------------------------------------------------------------------------

pub struct ExchangeRatesHandler {
    pub state: Arc<CaesarAppState>,
}

#[async_trait]
impl ApiHandler for ExchangeRatesHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling caesar/exchange_rates: {}", request.id);

        let protocol = self.state.protocol.read().await;
        let oracle = protocol.oracle();
        // Oracle stores price per troy ounce; convert to per gram
        let price_oz = oracle.current_gold_price_usd().await;
        let grams_per_oz = rust_decimal::Decimal::new(311035, 4); // 31.1035
        let price_per_gram = price_oz / grams_per_oz;
        let price_f64 = price_per_gram.to_f64().unwrap_or(75.56);

        let response = ExchangeRatesResponse {
            csr_to_usd: price_f64,
            csr_to_btc: 0.0,
            csr_to_eth: 0.0,
            last_updated: now_unix(),
            volume_24h: 0.0,
            change_24h: 0.0,
        };

        serialize_response(&request.id, &response)
    }

    fn path(&self) -> &str {
        "caesar/exchange_rates"
    }
}

// ---------------------------------------------------------------------------
// Analytics handler
// ---------------------------------------------------------------------------

pub struct AnalyticsHandler {
    pub state: Arc<CaesarAppState>,
}

#[async_trait]
impl ApiHandler for AnalyticsHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling caesar/analytics: {}", request.id);

        let protocol = self.state.protocol.read().await;
        let active = protocol.active_packet_count().await.unwrap_or(0) as u64;

        let response = AnalyticsResponse {
            total_supply: 0.0,
            circulating_supply: 0.0,
            market_cap: 0.0,
            holders: 0,
            transactions_24h: active,
            volume_24h: 0.0,
            average_transaction: 0.0,
            network_activity: NetworkActivity {
                active_nodes: 1,
                total_resources_shared: 0,
                rewards_distributed_24h: 0.0,
                new_users_24h: 0,
            },
            staking_metrics: StakingMetrics {
                total_staked: 0.0,
                staking_ratio: 0.0,
                average_lock_period: 0.0,
                total_stakers: 0,
            },
        };

        serialize_response(&request.id, &response)
    }

    fn path(&self) -> &str {
        "caesar/analytics"
    }
}

// ---------------------------------------------------------------------------
// Earnings handler
// ---------------------------------------------------------------------------

pub struct EarningsHandler {
    pub state: Arc<CaesarAppState>,
}

#[async_trait]
impl ApiHandler for EarningsHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling caesar/earnings: {}", request.id);

        let response = EarningsResponse {
            total_earnings: 0.0,
            earnings_24h: 0.0,
            earnings_7d: 0.0,
            earnings_30d: 0.0,
            breakdown: Vec::new(),
            projection_daily: 0.0,
            projection_monthly: 0.0,
        };

        serialize_response(&request.id, &response)
    }

    fn path(&self) -> &str {
        "caesar/earnings"
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn serialize_response<T: Serialize>(
    request_id: &str,
    body: &T,
) -> Result<ApiResponse, ApiError> {
    let payload = serde_json::to_vec(body)
        .map_err(|e| ApiError::SerializationError(e.to_string()))?;

    Ok(ApiResponse {
        request_id: request_id.to_string(),
        success: true,
        payload: payload.into(),
        error: None,
        metadata: std::collections::HashMap::new(),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use tempfile::TempDir;
    use tokio::sync::RwLock;

    use crate::CaesarProtocol;

    async fn make_app_state(dir: &TempDir) -> Arc<CaesarAppState> {
        let config = crate::CaesarConfig {
            storage: crate::storage::StorageConfig {
                path: dir.path().to_str().expect("test: tempdir path").to_string(),
            },
            ..crate::CaesarConfig::default()
        };
        let protocol = CaesarProtocol::new(config)
            .await
            .expect("test: protocol init");
        Arc::new(CaesarAppState {
            protocol: Arc::new(RwLock::new(protocol)),
        })
    }

    fn empty_request(id: &str) -> ApiRequest {
        ApiRequest {
            id: id.to_string(),
            service: "caesar".to_string(),
            method: String::new(),
            payload: Bytes::from("{}"),
            metadata: std::collections::HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_wallet_handler() {
        let dir = TempDir::new().expect("test: tempdir");
        let app = make_app_state(&dir).await;
        let handler = WalletHandler { state: app };

        let resp = handler
            .handle(empty_request("test-wallet-1"))
            .await
            .expect("test: wallet handler");
        assert!(resp.success);

        let body: WalletResponse =
            serde_json::from_slice(&resp.payload).expect("test: deserialize");
        assert!(body.success);
        assert!(body.wallet.is_some());
        assert_eq!(body.wallet.as_ref().map(|w| &w.id).expect("test: wallet should exist"), "evp-node-wallet");
    }

    #[tokio::test]
    async fn test_balance_handler() {
        let dir = TempDir::new().expect("test: tempdir");
        let app = make_app_state(&dir).await;
        let handler = BalanceHandler { state: app };

        let resp = handler
            .handle(empty_request("test-balance-1"))
            .await
            .expect("test: balance handler");
        assert!(resp.success);

        let body: BalanceResponse =
            serde_json::from_slice(&resp.payload).expect("test: deserialize");
        assert_eq!(body.total, 0.0);
        assert_eq!(body.staked, 0.0);
    }

    #[tokio::test]
    async fn test_transactions_handler() {
        let dir = TempDir::new().expect("test: tempdir");
        let app = make_app_state(&dir).await;
        let handler = TransactionsHandler { state: app };

        let resp = handler
            .handle(empty_request("test-tx-1"))
            .await
            .expect("test: transactions handler");
        assert!(resp.success);

        let body: TransactionsResponse =
            serde_json::from_slice(&resp.payload).expect("test: deserialize");
        assert_eq!(body.total, 0);
        assert_eq!(body.page, 1);
    }

    #[tokio::test]
    async fn test_exchange_rates_handler() {
        let dir = TempDir::new().expect("test: tempdir");
        let app = make_app_state(&dir).await;
        let handler = ExchangeRatesHandler { state: app };

        let resp = handler
            .handle(empty_request("test-rates-1"))
            .await
            .expect("test: exchange rates handler");
        assert!(resp.success);

        let body: ExchangeRatesResponse =
            serde_json::from_slice(&resp.payload).expect("test: deserialize");
        assert!(body.csr_to_usd > 0.0);
        assert!(body.last_updated > 0);
    }

    #[tokio::test]
    async fn test_analytics_handler() {
        let dir = TempDir::new().expect("test: tempdir");
        let app = make_app_state(&dir).await;
        let handler = AnalyticsHandler { state: app };

        let resp = handler
            .handle(empty_request("test-analytics-1"))
            .await
            .expect("test: analytics handler");
        assert!(resp.success);

        let body: AnalyticsResponse =
            serde_json::from_slice(&resp.payload).expect("test: deserialize");
        assert_eq!(body.network_activity.active_nodes, 1);
    }

    #[tokio::test]
    async fn test_staking_handler() {
        let dir = TempDir::new().expect("test: tempdir");
        let app = make_app_state(&dir).await;
        let handler = StakingHandler { state: app };

        let resp = handler
            .handle(empty_request("test-staking-1"))
            .await
            .expect("test: staking handler");
        assert!(resp.success);

        let body: StakingInfo =
            serde_json::from_slice(&resp.payload).expect("test: deserialize");
        assert_eq!(body.lock_periods.len(), 3);
        assert_eq!(body.lock_periods[0].days, 30);
    }

    #[tokio::test]
    async fn test_rewards_handler() {
        let dir = TempDir::new().expect("test: tempdir");
        let app = make_app_state(&dir).await;
        let handler = RewardsHandler { state: app };

        let resp = handler
            .handle(empty_request("test-rewards-1"))
            .await
            .expect("test: rewards handler");
        assert!(resp.success);

        let body: RewardsInfo =
            serde_json::from_slice(&resp.payload).expect("test: deserialize");
        assert_eq!(body.multiplier, 1.0);
    }

    #[tokio::test]
    async fn test_earnings_handler() {
        let dir = TempDir::new().expect("test: tempdir");
        let app = make_app_state(&dir).await;
        let handler = EarningsHandler { state: app };

        let resp = handler
            .handle(empty_request("test-earnings-1"))
            .await
            .expect("test: earnings handler");
        assert!(resp.success);

        let body: EarningsResponse =
            serde_json::from_slice(&resp.payload).expect("test: deserialize");
        assert_eq!(body.total_earnings, 0.0);
    }
}
