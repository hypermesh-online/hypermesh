// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Banking interoperability bridge types and data structures

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Banking API Provider Types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BankingProvider {
    OpenBanking,
    Stripe,
    Plaid,
    Link,
    Square,
    Mock, // For testing
}

/// Crypto Exchange Types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CryptoExchange {
    Uniswap,
    LayerZero,
    Axelar,
    Chainlink,
    Internal, // Caesar's internal exchange
}

/// Economic Stability Zone for global market stabilization
#[derive(Debug, Clone)]
pub struct VelocityZone {
    pub zone_id: String,
    pub name: String,
    pub market_velocity: Decimal,
    pub stability_deviation: Decimal,
    pub throttle_factor: Decimal,
    pub target_stability_range: (Decimal, Decimal),
    pub location_data: LocationData,
}

#[derive(Debug, Clone)]
pub struct LocationData {
    pub country: String,
    pub region: String,
    pub city: Option<String>,
    pub economic_indicators: EconomicIndicators,
}

#[derive(Debug, Clone)]
pub struct EconomicIndicators {
    pub current_gold_price_usd: Decimal,
    pub target_gold_price_usd: Decimal,
    pub market_volatility: Decimal,
    pub transaction_volume: Decimal,
    pub liquidity_depth: Decimal,
}

/// Interop Bridge Transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteropTransaction {
    pub transaction_id: String,
    pub bridge_type: BridgeType,
    pub source_asset: AssetType,
    pub destination_asset: AssetType,
    pub amount: Decimal,
    pub source_provider: String,
    pub destination_provider: String,
    pub exchange_rate: Decimal,
    pub fees: BridgeFees,
    pub status: InteropStatus,
    pub velocity_zone: Option<String>,
    pub contract_reference: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub completion_time: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BridgeType {
    FiatToCrypto,
    CryptoToFiat,
    CryptoToCrypto,
    FiatToFiat,
    ContractExecution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetType {
    Fiat { currency: String },
    Crypto { symbol: String, chain: String },
    Caesar { version: String },
    HyperMeshAsset { asset_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeFees {
    pub network_fee: Decimal,
    pub provider_fee: Decimal,
    pub bridge_fee: Decimal,
    pub velocity_adjustment: Decimal,
    pub total_fee: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteropStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    RequiresApproval,
}

/// Velocity Economics Score for comprehensive zone evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VelocityScore {
    pub zone_id: String,
    pub total_score: Decimal,
    pub base_velocity_component: Decimal,
    pub economic_component: Decimal,
    pub activity_component: Decimal,
    pub decay_component: Decimal,
    pub grade: String,
    pub recommended_fee_adjustment: Decimal,
}

/// Supporting Types
#[derive(Debug, Clone)]
pub struct BankingCredentials {
    pub provider: BankingProvider,
    pub api_key: String,
    pub api_secret: Option<String>,
    pub client_id: Option<String>,
    pub environment: String,
}

#[derive(Debug, Clone)]
pub struct AuthToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub refresh_token: Option<String>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AccountBalance {
    pub account_id: String,
    pub available: Decimal,
    pub current: Decimal,
    pub pending: Decimal,
    pub currency: String,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PaymentRequest {
    pub from_account: String,
    pub to_account: String,
    pub amount: Decimal,
    pub currency: String,
    pub reference: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct PaymentResponse {
    pub payment_id: String,
    pub status: String,
    pub estimated_completion: DateTime<Utc>,
    pub fees: Decimal,
}

#[derive(Debug, Clone)]
pub struct BankTransaction {
    pub transaction_id: String,
    pub amount: Decimal,
    pub currency: String,
    pub transaction_type: String,
    pub description: String,
    pub timestamp: DateTime<Utc>,
    pub balance_after: Decimal,
}

#[derive(Debug, Clone)]
pub struct HistoryParams {
    pub from_date: DateTime<Utc>,
    pub to_date: DateTime<Utc>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct AccountDetails {
    pub account_number: String,
    pub routing_number: Option<String>,
    pub bank_name: String,
    pub account_type: String,
}

#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub is_valid: bool,
    pub verification_id: String,
    pub confidence_score: Decimal,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TradingPair {
    pub base: String,
    pub quote: String,
    pub exchange: String,
    pub min_amount: Decimal,
    pub max_amount: Decimal,
}

#[derive(Debug, Clone)]
pub struct ExchangeQuote {
    pub from_amount: Decimal,
    pub to_amount: Decimal,
    pub exchange_rate: Decimal,
    pub fees: Decimal,
    pub estimated_gas: Option<Decimal>,
    pub valid_until: DateTime<Utc>,
    pub slippage_tolerance: Decimal,
}

#[derive(Debug, Clone)]
pub struct CryptoCredentials {
    pub private_key: String,
    pub address: String,
    pub chain_id: u64,
}

#[derive(Debug, Clone)]
pub struct SwapRequest {
    pub from_token: String,
    pub to_token: String,
    pub amount: Decimal,
    pub slippage_tolerance: Decimal,
    pub recipient: String,
}

#[derive(Debug, Clone)]
pub struct SwapResult {
    pub transaction_hash: String,
    pub from_amount: Decimal,
    pub to_amount: Decimal,
    pub gas_used: Decimal,
    pub gas_price: Decimal,
}

#[derive(Debug, Clone)]
pub struct LiquidityInfo {
    pub reserve_a: Decimal,
    pub reserve_b: Decimal,
    pub total_supply: Decimal,
    pub apr: Decimal,
}

#[derive(Debug, Clone)]
pub struct GasEstimate {
    pub estimated_gas: Decimal,
    pub gas_price: Decimal,
    pub total_cost: Decimal,
}

/// Banking API Interface
#[async_trait::async_trait]
pub trait BankingApiProvider: Send + Sync {
    async fn authenticate(&self, credentials: &BankingCredentials) -> anyhow::Result<AuthToken>;
    async fn get_account_balance(
        &self,
        auth: &AuthToken,
        account_id: &str,
    ) -> anyhow::Result<AccountBalance>;
    async fn initiate_payment(
        &self,
        auth: &AuthToken,
        payment: &PaymentRequest,
    ) -> anyhow::Result<PaymentResponse>;
    async fn get_transaction_history(
        &self,
        auth: &AuthToken,
        account_id: &str,
        params: &HistoryParams,
    ) -> anyhow::Result<Vec<BankTransaction>>;
    async fn verify_account(
        &self,
        auth: &AuthToken,
        account_details: &AccountDetails,
    ) -> anyhow::Result<VerificationResult>;
    async fn get_supported_currencies(&self) -> anyhow::Result<Vec<String>>;
    async fn get_exchange_rates(
        &self,
        base: &str,
        targets: &[String],
    ) -> anyhow::Result<HashMap<String, Decimal>>;
}

/// Crypto Exchange Interface
#[async_trait::async_trait]
pub trait CryptoExchangeProvider: Send + Sync {
    async fn get_supported_pairs(&self) -> anyhow::Result<Vec<TradingPair>>;
    async fn get_quote(
        &self,
        from: &str,
        to: &str,
        amount: Decimal,
    ) -> anyhow::Result<ExchangeQuote>;
    async fn execute_swap(
        &self,
        auth: &CryptoCredentials,
        swap: &SwapRequest,
    ) -> anyhow::Result<SwapResult>;
    async fn get_liquidity_info(&self, pair: &TradingPair) -> anyhow::Result<LiquidityInfo>;
    async fn estimate_gas(&self, swap: &SwapRequest) -> anyhow::Result<GasEstimate>;
}
