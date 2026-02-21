// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Demurrage (decay) engine for EVP packets.
//!
//! Implements exponential decay: V_t = V_0 * e^(-lambda * t)
//! where lambda is the per-second decay constant.
//!
//! Core calculation lives in `DemurrageRate::calculate_remaining()` (lib).
//! This module provides the Caesar-specific engine with convenience methods.

use hypermesh_lib::economic::{DemurrageRate, GoldGrams};

/// Demurrage calculation engine.
///
/// Stateless -- all methods are pure functions of their inputs.
/// Wraps `DemurrageRate::calculate_remaining()` from lib and adds
/// Caesar-specific convenience helpers.
pub struct DemurrageEngine;

impl DemurrageEngine {
    /// Calculate remaining value after elapsed time.
    ///
    /// Delegates to `DemurrageRate::calculate_remaining()`.
    pub fn calculate_remaining(
        initial: GoldGrams,
        elapsed_secs: u64,
        rate: &DemurrageRate,
    ) -> GoldGrams {
        rate.calculate_remaining(initial, elapsed_secs)
    }

    /// Calculate the cost of demurrage (value lost to decay).
    pub fn demurrage_cost(
        initial: GoldGrams,
        elapsed_secs: u64,
        rate: &DemurrageRate,
    ) -> GoldGrams {
        let remaining = Self::calculate_remaining(initial, elapsed_secs, rate);
        GoldGrams(initial.0 - remaining.0)
    }

    /// Check if a packet has exceeded its TTL.
    pub fn is_expired(elapsed_secs: u64, rate: &DemurrageRate) -> bool {
        elapsed_secs >= rate.max_ttl_secs
    }

    /// Calculate percentage of value remaining (0.0 -- 100.0).
    pub fn remaining_percentage(elapsed_secs: u64, rate: &DemurrageRate) -> f64 {
        (-rate.lambda * elapsed_secs as f64).exp() * 100.0
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::economic::MarketTier;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    #[test]
    fn zero_elapsed_returns_full_value() {
        let initial = GoldGrams(dec!(100));
        let rate = MarketTier::L0.default_demurrage_rate();
        let remaining = DemurrageEngine::calculate_remaining(initial, 0, &rate);
        assert_eq!(remaining.0, dec!(100));
    }

    #[test]
    fn l0_decays_significantly_after_half_day() {
        // L0 max_ttl is 86400s (1 day); test at half-day to stay within TTL
        let initial = GoldGrams(dec!(100));
        let rate = MarketTier::L0.default_demurrage_rate();
        let half_day = 43_200;
        let remaining = DemurrageEngine::calculate_remaining(initial, half_day, &rate);

        // Value should have decayed -- remaining < initial
        assert!(
            remaining.0 < initial.0,
            "L0 should decay after half day: remaining={}, initial={}",
            remaining.0,
            initial.0
        );
        // Should still be positive (well within TTL)
        assert!(
            remaining.0 > Decimal::ZERO,
            "L0 remaining should be positive: {}",
            remaining.0
        );
    }

    #[test]
    fn l0_at_max_ttl_returns_zero() {
        // L0 max_ttl = 86400s; at or past TTL, calculate_remaining returns zero
        let initial = GoldGrams(dec!(100));
        let rate = MarketTier::L0.default_demurrage_rate();
        let remaining = DemurrageEngine::calculate_remaining(initial, rate.max_ttl_secs, &rate);
        assert_eq!(remaining.0, Decimal::ZERO);
    }

    #[test]
    fn l3_after_one_day_nearly_full() {
        // L3 has very slow decay -- nearly all value preserved after 1 day
        let initial = GoldGrams(dec!(1000));
        let rate = MarketTier::L3.default_demurrage_rate();
        let remaining = DemurrageEngine::calculate_remaining(initial, 86_400, &rate);

        assert!(
            remaining.0 > dec!(990),
            "L3 after 1 day: expected >990, got {}",
            remaining.0
        );
    }

    #[test]
    fn demurrage_cost_plus_remaining_equals_initial() {
        let initial = GoldGrams(dec!(500));
        let rate = MarketTier::L1.default_demurrage_rate();
        let elapsed = 3600; // 1 hour

        let remaining = DemurrageEngine::calculate_remaining(initial, elapsed, &rate);
        let cost = DemurrageEngine::demurrage_cost(initial, elapsed, &rate);

        // remaining + cost should equal initial (within decimal precision)
        let sum = remaining.0 + cost.0;
        let diff = (sum - initial.0).abs();
        assert!(
            diff < dec!(0.000001),
            "remaining({}) + cost({}) = {}, expected {}",
            remaining.0,
            cost.0,
            sum,
            initial.0
        );
    }

    #[test]
    fn expired_at_max_ttl() {
        let rate = MarketTier::L0.default_demurrage_rate();
        assert!(!DemurrageEngine::is_expired(rate.max_ttl_secs - 1, &rate));
        assert!(DemurrageEngine::is_expired(rate.max_ttl_secs, &rate));
        assert!(DemurrageEngine::is_expired(rate.max_ttl_secs + 1, &rate));
    }

    #[test]
    fn remaining_percentage_at_zero() {
        let rate = MarketTier::L2.default_demurrage_rate();
        let pct = DemurrageEngine::remaining_percentage(0, &rate);
        assert!((pct - 100.0).abs() < 0.001);
    }

    #[test]
    fn remaining_percentage_decreases_over_time() {
        let rate = MarketTier::L1.default_demurrage_rate();
        let pct_1h = DemurrageEngine::remaining_percentage(3600, &rate);
        let pct_1d = DemurrageEngine::remaining_percentage(86_400, &rate);
        let pct_7d = DemurrageEngine::remaining_percentage(7 * 86_400, &rate);

        assert!(pct_1h > pct_1d, "1h={pct_1h} should > 1d={pct_1d}");
        assert!(pct_1d > pct_7d, "1d={pct_1d} should > 7d={pct_7d}");
    }

    #[test]
    fn zero_initial_value() {
        let initial = GoldGrams(dec!(0));
        let rate = MarketTier::L0.default_demurrage_rate();
        let remaining = DemurrageEngine::calculate_remaining(initial, 86_400, &rate);
        assert_eq!(remaining.0, dec!(0));
    }
}
