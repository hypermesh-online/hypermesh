// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Gold oracle -- spot price feed and effective rate composite.
//!
//! No token swaps. Replaces the old `exchange.rs`. Provides the gold price
//! anchor and the emergent CAES effective rate (whitepaper S5.1).

use chrono::{DateTime, Utc};
use hypermesh_lib::economic::GoldGrams;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Grams per troy ounce.
const GRAMS_PER_TROY_OZ: Decimal = dec!(31.1035);

/// Maximum allowed price change per update (10%).
const MAX_PRICE_CHANGE_PCT: Decimal = dec!(10);

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors from oracle operations.
#[derive(Debug, thiserror::Error)]
pub enum OracleError {
    #[error("price change {change_pct}% exceeds 10% limit")]
    ExcessivePriceChange { change_pct: Decimal },
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single historical price observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePoint {
    pub price_usd: Decimal,
    pub timestamp: DateTime<Utc>,
}

/// Effective price composite (whitepaper S5.1).
///
/// The "price" of CAES is emergent from three independent forces:
/// network fee rates, speculative pressure, and in-transit float.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectivePriceComposite {
    pub network_fees_component: Decimal,
    pub speculation_pressure: Decimal,
    pub liquidity_shadow: Decimal,
    /// Emergent composite rate (gold grams -> USD adjustment).
    pub effective_rate: Decimal,
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Internal state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct OracleState {
    /// Spot gold price per troy ounce in USD.
    gold_price_usd: Decimal,
    /// When the price was last updated.
    last_update: DateTime<Utc>,
    /// Rolling price history.
    history: Vec<PricePoint>,
}

// ---------------------------------------------------------------------------
// GoldOracle
// ---------------------------------------------------------------------------

/// Gold price feed with band-limited updates and composite rate calculation.
#[derive(Debug, Clone)]
pub struct GoldOracle {
    state: Arc<RwLock<OracleState>>,
}

impl GoldOracle {
    /// Create a new oracle seeded with an initial gold price (USD per troy oz).
    pub fn new(initial_price_usd: Decimal) -> Self {
        let now = Utc::now();
        Self {
            state: Arc::new(RwLock::new(OracleState {
                gold_price_usd: initial_price_usd,
                last_update: now,
                history: vec![PricePoint {
                    price_usd: initial_price_usd,
                    timestamp: now,
                }],
            })),
        }
    }

    /// Current gold price in USD per troy ounce.
    pub async fn current_gold_price_usd(&self) -> Decimal {
        self.state.read().await.gold_price_usd
    }

    /// Last N historical price points (most recent last).
    pub async fn gold_price_history(&self, limit: usize) -> Vec<PricePoint> {
        let state = self.state.read().await;
        let len = state.history.len();
        let start = len.saturating_sub(limit);
        state.history[start..].to_vec()
    }

    /// Update the gold price. Rejects changes exceeding a 10% band from
    /// the current price to prevent oracle manipulation.
    pub async fn update_price(&self, new_price: Decimal) -> Result<(), OracleError> {
        let mut state = self.state.write().await;
        let current = state.gold_price_usd;

        if !current.is_zero() {
            let change_pct = ((new_price - current) / current * dec!(100)).abs();
            if change_pct > MAX_PRICE_CHANGE_PCT {
                return Err(OracleError::ExcessivePriceChange { change_pct });
            }
        }

        let now = Utc::now();
        state.gold_price_usd = new_price;
        state.last_update = now;
        state.history.push(PricePoint {
            price_usd: new_price,
            timestamp: now,
        });
        Ok(())
    }

    /// Convert a gold-gram amount to USD using the current spot price.
    ///
    /// 1 troy ounce = 31.1035 grams, so:
    ///   USD = grams * (price_per_oz / 31.1035)
    pub async fn grams_to_usd(&self, grams: GoldGrams) -> Decimal {
        let price_per_oz = self.state.read().await.gold_price_usd;
        let price_per_gram = price_per_oz / GRAMS_PER_TROY_OZ;
        grams.0 * price_per_gram
    }

    /// Calculate the effective CAES rate from network observables.
    ///
    /// effective_rate = gold_price_per_gram * (1 + fees + speculation - liquidity_shadow)
    pub async fn calculate_effective_rate(
        &self,
        avg_fee_rate: Decimal,
        speculation_index: Decimal,
        in_transit_float: Decimal,
        total_capacity: Decimal,
    ) -> EffectivePriceComposite {
        let price_per_gram = self.state.read().await.gold_price_usd / GRAMS_PER_TROY_OZ;

        let liquidity_shadow = if total_capacity > Decimal::ZERO {
            in_transit_float / total_capacity
        } else {
            Decimal::ZERO
        };

        let effective_rate = price_per_gram
            * (Decimal::ONE + avg_fee_rate + speculation_index - liquidity_shadow);

        EffectivePriceComposite {
            network_fees_component: avg_fee_rate,
            speculation_pressure: speculation_index,
            liquidity_shadow,
            effective_rate,
            timestamp: Utc::now(),
        }
    }

    /// Update price from an oracle feed source.
    pub async fn update_from_feed(&self, feed: &dyn OracleFeed) -> Result<(), OracleError> {
        let price = feed.fetch_gold_price_usd().await?;
        self.update_price(price).await
    }
}

// ---------------------------------------------------------------------------
// OracleFeed trait
// ---------------------------------------------------------------------------

/// Trait for gold price feed sources.
///
/// Implementations can range from manual/hardcoded feeds (for testing and
/// alpha) to real-time API integrations with multiple providers.
#[async_trait::async_trait]
pub trait OracleFeed: Send + Sync {
    /// Fetch the current gold price in USD per troy ounce.
    async fn fetch_gold_price_usd(&self) -> Result<Decimal, OracleError>;
    /// Human-readable name for this feed source.
    fn feed_name(&self) -> &str;
}

/// Manual feed -- returns a static price. Suitable for testing and alpha.
pub struct ManualFeed {
    price_usd: Decimal,
}

impl ManualFeed {
    /// Create a manual feed with a fixed price.
    pub fn new(price_usd: Decimal) -> Self {
        Self { price_usd }
    }
}

#[async_trait::async_trait]
impl OracleFeed for ManualFeed {
    async fn fetch_gold_price_usd(&self) -> Result<Decimal, OracleError> {
        Ok(self.price_usd)
    }
    fn feed_name(&self) -> &str {
        "manual"
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Approximate gold price for tests (USD per troy oz).
    const TEST_GOLD_PRICE: Decimal = dec!(2000);

    #[tokio::test]
    async fn oracle_creation() {
        let oracle = GoldOracle::new(TEST_GOLD_PRICE);
        let price = oracle.current_gold_price_usd().await;
        assert_eq!(price, TEST_GOLD_PRICE);
    }

    #[tokio::test]
    async fn price_update_within_band() {
        let oracle = GoldOracle::new(TEST_GOLD_PRICE);
        // 5% increase -- well within the 10% band
        let new_price = TEST_GOLD_PRICE * dec!(1.05);
        oracle
            .update_price(new_price)
            .await
            .expect("test: 5% change should succeed");
        assert_eq!(oracle.current_gold_price_usd().await, new_price);
    }

    #[tokio::test]
    async fn price_update_exceeds_band() {
        let oracle = GoldOracle::new(TEST_GOLD_PRICE);
        // 15% increase -- exceeds the 10% band
        let new_price = TEST_GOLD_PRICE * dec!(1.15);
        let err = oracle.update_price(new_price).await;
        assert!(
            matches!(err, Err(OracleError::ExcessivePriceChange { .. })),
            "expected ExcessivePriceChange, got {err:?}"
        );
    }

    #[tokio::test]
    async fn price_history() {
        let oracle = GoldOracle::new(TEST_GOLD_PRICE);
        oracle
            .update_price(TEST_GOLD_PRICE * dec!(1.01))
            .await
            .expect("test: update 1");
        oracle
            .update_price(TEST_GOLD_PRICE * dec!(1.02))
            .await
            .expect("test: update 2");

        let history = oracle.gold_price_history(10).await;
        // Initial + 2 updates = 3 entries
        assert_eq!(history.len(), 3);
    }

    #[tokio::test]
    async fn grams_to_usd_conversion() {
        let oracle = GoldOracle::new(TEST_GOLD_PRICE);
        // 31.1035g = 1 troy oz = $2000
        let usd = oracle.grams_to_usd(GoldGrams(GRAMS_PER_TROY_OZ)).await;
        let diff = (usd - TEST_GOLD_PRICE).abs();
        assert!(
            diff < dec!(0.01),
            "31.1035g should equal 1 troy oz: got ${usd}, expected ${TEST_GOLD_PRICE}"
        );

        // 1g should be ~$64.30 (2000 / 31.1035)
        let usd_1g = oracle.grams_to_usd(GoldGrams(dec!(1))).await;
        let expected = TEST_GOLD_PRICE / GRAMS_PER_TROY_OZ;
        let diff = (usd_1g - expected).abs();
        assert!(
            diff < dec!(0.01),
            "1g conversion: got ${usd_1g}, expected ${expected}"
        );
    }

    #[tokio::test]
    async fn effective_rate_calculation() {
        let oracle = GoldOracle::new(TEST_GOLD_PRICE);
        let composite = oracle
            .calculate_effective_rate(
                dec!(0.02),  // 2% avg fee
                dec!(0.01),  // 1% speculation
                dec!(500),   // 500g in transit
                dec!(10000), // 10000g capacity
            )
            .await;

        assert_eq!(composite.network_fees_component, dec!(0.02));
        assert_eq!(composite.speculation_pressure, dec!(0.01));
        // liquidity_shadow = 500 / 10000 = 0.05
        assert_eq!(composite.liquidity_shadow, dec!(0.05));

        // effective_rate = price_per_gram * (1 + 0.02 + 0.01 - 0.05)
        //                = price_per_gram * 0.98
        let price_per_gram = TEST_GOLD_PRICE / GRAMS_PER_TROY_OZ;
        let expected = price_per_gram * dec!(0.98);
        let diff = (composite.effective_rate - expected).abs();
        assert!(
            diff < dec!(0.01),
            "effective_rate: got {}, expected {expected}",
            composite.effective_rate
        );
    }

    #[tokio::test]
    async fn effective_rate_zero_capacity() {
        let oracle = GoldOracle::new(TEST_GOLD_PRICE);
        let composite = oracle
            .calculate_effective_rate(
                dec!(0.01),
                dec!(0.0),
                dec!(100), // non-zero float but zero capacity
                dec!(0),
            )
            .await;

        // liquidity_shadow should be 0 (no division by zero)
        assert_eq!(composite.liquidity_shadow, Decimal::ZERO);
    }

    #[tokio::test]
    async fn manual_feed_returns_price() {
        let feed = ManualFeed::new(dec!(2000));
        let price = feed.fetch_gold_price_usd().await.expect("test: manual feed");
        assert_eq!(price, dec!(2000));
        assert_eq!(feed.feed_name(), "manual");
    }

    #[tokio::test]
    async fn oracle_update_from_feed() {
        let oracle = GoldOracle::new(dec!(2000));
        let feed = ManualFeed::new(dec!(2050));
        oracle.update_from_feed(&feed).await.expect("test: update from feed");
        assert_eq!(oracle.current_gold_price_usd().await, dec!(2050));
    }
}
