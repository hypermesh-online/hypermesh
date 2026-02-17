// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Caesar Exchange System - Token exchange and market operations

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;
use tracing::{info, debug};
use uuid::Uuid;
use tokio::sync::RwLock;

use crate::models::*;
use crate::ExchangeConfig;

/// Exchange engine for token swaps and market operations
pub struct ExchangeEngine {
    config: ExchangeConfig,

    // Market state
    market_state: Arc<RwLock<MarketState>>,
}

#[derive(Debug, Clone)]
struct MarketState {
    csr_usd_rate: Decimal,
    liquidity_pool: Decimal,
    volume_24h: Decimal,
    fees_24h: Decimal,
    last_update: DateTime<Utc>,
}

impl ExchangeEngine {
    pub fn new(config: ExchangeConfig) -> Self {
        let market_state = Arc::new(RwLock::new(MarketState {
            csr_usd_rate: config.csr_usd_rate,
            liquidity_pool: config.liquidity_pool,
            volume_24h: dec!(0),
            fees_24h: dec!(0),
            last_update: Utc::now(),
        }));

        Self {
            config,
            market_state,
        }
    }

    /// Calculate USD value of CSR tokens
    pub fn calculate_usd_value(&self, csr_amount: Decimal) -> Result<Decimal> {
        Ok(csr_amount * self.config.csr_usd_rate)
    }

    /// Get current exchange rates
    pub async fn get_rates(&self) -> Result<ExchangeRatesResponse> {
        let state = self.market_state.read().await;

        // Simulate market volatility
        let volatility_factor = dec!(1) + (
            Decimal::from_f64_retain(rand::random::<f64>()).unwrap_or(dec!(0)) - dec!(0.5)
        ) * self.config.volatility * dec!(2);

        let adjusted_rate = state.csr_usd_rate * volatility_factor;

        let rates = vec![
            ExchangeRate {
                from_token: "CSR".to_string(),
                to_token: "USD".to_string(),
                rate: adjusted_rate,
                inverse_rate: dec!(1) / adjusted_rate,
                timestamp: Utc::now(),
                volume_24h: state.volume_24h,
            },
            ExchangeRate {
                from_token: "CSR".to_string(),
                to_token: "ETH".to_string(),
                rate: adjusted_rate / dec!(3000), // Assuming ETH = $3000
                inverse_rate: dec!(3000) / adjusted_rate,
                timestamp: Utc::now(),
                volume_24h: state.volume_24h / dec!(10),
            },
            ExchangeRate {
                from_token: "CSR".to_string(),
                to_token: "BTC".to_string(),
                rate: adjusted_rate / dec!(60000), // Assuming BTC = $60000
                inverse_rate: dec!(60000) / adjusted_rate,
                timestamp: Utc::now(),
                volume_24h: state.volume_24h / dec!(20),
            },
        ];

        Ok(ExchangeRatesResponse {
            rates,
            base_currency: "CSR".to_string(),
            last_updated: Utc::now(),
        })
    }

    /// Execute token swap
    pub async fn swap(&self, request: SwapRequest) -> Result<SwapResponse> {
        let mut state = self.market_state.write().await;

        // Get current rate
        let rate = if request.from_token == "CSR" && request.to_token == "USD" {
            state.csr_usd_rate
        } else if request.from_token == "USD" && request.to_token == "CSR" {
            dec!(1) / state.csr_usd_rate
        } else {
            return Err(anyhow!("Unsupported token pair"));
        };

        // Calculate slippage based on amount relative to liquidity
        let impact = request.amount / state.liquidity_pool;
        let slippage = impact * self.config.volatility * dec!(10); // Amplify for larger trades

        // Check slippage tolerance
        let tolerance = request.slippage_tolerance.unwrap_or(self.config.slippage_tolerance);
        if slippage > tolerance {
            return Err(anyhow!("Slippage {} exceeds tolerance {}", slippage, tolerance));
        }

        // Adjust rate for slippage
        let effective_rate = rate * (dec!(1) - slippage);

        // Calculate output amount
        let to_amount = request.amount * effective_rate;

        // Calculate fee (0.3% standard)
        let fee = request.amount * dec!(0.003);

        // Update market state
        state.volume_24h += request.amount;
        state.fees_24h += fee;

        // Simulate price impact
        if request.from_token == "CSR" {
            // Selling CSR decreases price
            state.csr_usd_rate *= dec!(1) - (impact * dec!(0.01));
        } else {
            // Buying CSR increases price
            state.csr_usd_rate *= dec!(1) + (impact * dec!(0.01));
        }

        state.last_update = Utc::now();

        let swap_id = format!("SWAP_{}", Uuid::new_v4());

        info!(
            "Swap executed: {} {} -> {} {} at rate {}",
            request.amount, request.from_token, to_amount, request.to_token, effective_rate
        );

        Ok(SwapResponse {
            swap_id,
            wallet_id: request.wallet_id,
            from_token: request.from_token,
            to_token: request.to_token,
            from_amount: request.amount,
            to_amount,
            rate: effective_rate,
            fee,
            slippage,
            transaction_id: format!("TX_{}", Uuid::new_v4()),
            timestamp: Utc::now(),
        })
    }

    /// Get liquidity pool information
    pub async fn get_liquidity_info(&self) -> Result<LiquidityInfoResponse> {
        let state = self.market_state.read().await;

        // Calculate APY from fees
        let annual_fees = state.fees_24h * dec!(365);
        let apy = (annual_fees / state.liquidity_pool) * dec!(100);

        Ok(LiquidityInfoResponse {
            total_liquidity: state.liquidity_pool,
            csr_liquidity: state.liquidity_pool / dec!(2), // 50/50 split
            usd_liquidity: state.liquidity_pool / dec!(2),
            volume_24h: state.volume_24h,
            fee_24h: state.fees_24h,
            apy,
        })
    }

    /// Add liquidity to pool
    pub async fn add_liquidity(&self, amount_csr: Decimal, amount_usd: Decimal) -> Result<()> {
        let mut state = self.market_state.write().await;

        // Validate ratio matches current rate
        let expected_usd = amount_csr * state.csr_usd_rate;
        if (expected_usd - amount_usd).abs() > amount_usd * dec!(0.01) {
            return Err(anyhow!("Liquidity amounts don't match current rate"));
        }

        state.liquidity_pool += amount_csr + amount_usd;
        state.last_update = Utc::now();

        debug!("Added liquidity: {} CSR, {} USD", amount_csr, amount_usd);
        Ok(())
    }

    /// Remove liquidity from pool
    pub async fn remove_liquidity(&self, lp_tokens: Decimal) -> Result<(Decimal, Decimal)> {
        let mut state = self.market_state.write().await;

        if lp_tokens > state.liquidity_pool {
            return Err(anyhow!("Insufficient liquidity"));
        }

        let percentage = lp_tokens / state.liquidity_pool;
        let csr_amount = (state.liquidity_pool / dec!(2)) * percentage;
        let usd_amount = (state.liquidity_pool / dec!(2)) * percentage;

        state.liquidity_pool -= lp_tokens;
        state.last_update = Utc::now();

        debug!("Removed liquidity: {} CSR, {} USD", csr_amount, usd_amount);
        Ok((csr_amount, usd_amount))
    }

    /// Update market price (oracle or external feed)
    pub async fn update_price(&self, new_rate: Decimal) -> Result<()> {
        let mut state = self.market_state.write().await;

        // Validate reasonable price change (max 10% per update)
        let change = ((new_rate - state.csr_usd_rate) / state.csr_usd_rate).abs();
        if change > dec!(0.1) {
            return Err(anyhow!("Price change {} exceeds 10% limit", change));
        }

        state.csr_usd_rate = new_rate;
        state.last_update = Utc::now();

        info!("Market price updated to {} USD/CSR", new_rate);
        Ok(())
    }
}