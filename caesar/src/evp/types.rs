// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! EVP configuration and market-tier classification.
//!
//! Economic primitives (`GoldGrams`, `MarketTier`, `PacketState`, `DemurrageRate`,
//! `PacketId`) live in `hypermesh_lib::economic`. This module defines
//! Caesar-specific configuration and tier classification logic.

use hypermesh_lib::economic::{DemurrageRate, GoldGrams, MarketTier};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

// Re-export lib types so callers can do `use crate::evp::types::*`
pub use hypermesh_lib::economic::{PacketId, PacketState};

// ---------------------------------------------------------------------------
// Per-tier rate configuration
// ---------------------------------------------------------------------------

/// Per-tier demurrage rate overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierRates {
    pub l0: DemurrageRate,
    pub l1: DemurrageRate,
    pub l2: DemurrageRate,
    pub l3: DemurrageRate,
}

impl Default for TierRates {
    fn default() -> Self {
        Self {
            l0: MarketTier::L0.default_demurrage_rate(),
            l1: MarketTier::L1.default_demurrage_rate(),
            l2: MarketTier::L2.default_demurrage_rate(),
            l3: MarketTier::L3.default_demurrage_rate(),
        }
    }
}

impl TierRates {
    /// Look up the rate for a given tier.
    pub fn rate_for(&self, tier: &MarketTier) -> &DemurrageRate {
        match tier {
            MarketTier::L0 => &self.l0,
            MarketTier::L1 => &self.l1,
            MarketTier::L2 => &self.l2,
            MarketTier::L3 => &self.l3,
        }
    }
}

// ---------------------------------------------------------------------------
// Tier thresholds
// ---------------------------------------------------------------------------

/// Value thresholds for market-tier classification (gold grams).
///
/// Classification by value amount ONLY -- no regulation at protocol level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierThresholds {
    /// Below this = L0 (retail). In gold grams.
    pub l0_max: Decimal,
    /// Below this = L1 (professional). In gold grams.
    pub l1_max: Decimal,
    /// Below this = L2 (institutional). In gold grams.
    pub l2_max: Decimal,
    // Above l2_max = L3 (sovereign)
}

impl Default for TierThresholds {
    fn default() -> Self {
        Self {
            l0_max: dec!(10),      // ~$800 at ~$80/g gold
            l1_max: dec!(1000),    // ~$80K
            l2_max: dec!(100000),  // ~$8M
        }
    }
}

// ---------------------------------------------------------------------------
// EVP configuration
// ---------------------------------------------------------------------------

/// EVP system configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvpConfig {
    /// Per-tier demurrage rates (overrides defaults if set).
    pub tier_rates: TierRates,
    /// Gold oracle polling interval in seconds.
    pub oracle_interval_secs: u64,
    /// Base transaction fee as fraction (e.g., 0.001 = 0.1%).
    pub base_fee: Decimal,
    /// Value thresholds for tier classification (in gold grams).
    pub tier_thresholds: TierThresholds,
}

impl Default for EvpConfig {
    fn default() -> Self {
        Self {
            tier_rates: TierRates::default(),
            oracle_interval_secs: 60,
            base_fee: dec!(0.001),
            tier_thresholds: TierThresholds::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tier classifier
// ---------------------------------------------------------------------------

/// Classifies packets into market tiers by value amount.
pub struct TierClassifier;

impl TierClassifier {
    /// Classify a packet's market tier based on its value in gold grams.
    pub fn classify(value: &GoldGrams, thresholds: &TierThresholds) -> MarketTier {
        let amount = value.0;
        if amount <= thresholds.l0_max {
            MarketTier::L0
        } else if amount <= thresholds.l1_max {
            MarketTier::L1
        } else if amount <= thresholds.l2_max {
            MarketTier::L2
        } else {
            MarketTier::L3
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_l0_at_boundary() {
        let thresholds = TierThresholds::default();
        let value = GoldGrams(dec!(10));
        assert_eq!(TierClassifier::classify(&value, &thresholds), MarketTier::L0);
    }

    #[test]
    fn classify_l0_below_boundary() {
        let thresholds = TierThresholds::default();
        let value = GoldGrams(dec!(5));
        assert_eq!(TierClassifier::classify(&value, &thresholds), MarketTier::L0);
    }

    #[test]
    fn classify_l1() {
        let thresholds = TierThresholds::default();
        let value = GoldGrams(dec!(500));
        assert_eq!(TierClassifier::classify(&value, &thresholds), MarketTier::L1);
    }

    #[test]
    fn classify_l1_at_boundary() {
        let thresholds = TierThresholds::default();
        let value = GoldGrams(dec!(1000));
        assert_eq!(TierClassifier::classify(&value, &thresholds), MarketTier::L1);
    }

    #[test]
    fn classify_l2() {
        let thresholds = TierThresholds::default();
        let value = GoldGrams(dec!(50000));
        assert_eq!(TierClassifier::classify(&value, &thresholds), MarketTier::L2);
    }

    #[test]
    fn classify_l3_above_l2_max() {
        let thresholds = TierThresholds::default();
        let value = GoldGrams(dec!(200000));
        assert_eq!(TierClassifier::classify(&value, &thresholds), MarketTier::L3);
    }

    #[test]
    fn classify_zero_value() {
        let thresholds = TierThresholds::default();
        let value = GoldGrams(dec!(0));
        assert_eq!(TierClassifier::classify(&value, &thresholds), MarketTier::L0);
    }

    #[test]
    fn tier_rates_lookup() {
        let rates = TierRates::default();
        let l0_rate = rates.rate_for(&MarketTier::L0);
        let l3_rate = rates.rate_for(&MarketTier::L3);
        // L0 decays faster than L3
        assert!(l0_rate.lambda > l3_rate.lambda);
    }

    #[test]
    fn default_config_is_sane() {
        let config = EvpConfig::default();
        assert_eq!(config.oracle_interval_secs, 60);
        assert_eq!(config.base_fee, dec!(0.001));
        assert!(config.tier_thresholds.l0_max < config.tier_thresholds.l1_max);
        assert!(config.tier_thresholds.l1_max < config.tier_thresholds.l2_max);
    }

    #[test]
    fn packet_state_terminal() {
        // Terminal states: Settled, Refunded, Dissolved
        assert!(PacketState::Settled.is_terminal());
        assert!(PacketState::Refunded.is_terminal());
        assert!(PacketState::Dissolved.is_terminal());
        // Non-terminal states
        assert!(!PacketState::Minted.is_terminal());
        assert!(!PacketState::InTransit.is_terminal());
        assert!(!PacketState::Delivered.is_terminal());
        assert!(!PacketState::Settling.is_terminal());
        assert!(!PacketState::Held.is_terminal());
        assert!(!PacketState::Stalled.is_terminal());
        assert!(!PacketState::Dispersed.is_terminal());
        assert!(!PacketState::Expired.is_terminal());
    }
}
