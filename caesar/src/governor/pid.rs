// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Governor PID Controller -- core feedback loop for Caesar EVP.
//!
//! Consumes [`NetworkMetrics`] and produces [`GovernanceParams`] that modulate
//! fees, demurrage overrides, and routing incentives across L0-L3 market tiers.

use hypermesh_lib::{GoldGrams, MarketTier};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use super::params::*;

/// Network metrics fed into the Governor each control cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    /// Current gold price in USD per gram (from oracle).
    pub current_gold_price_usd: Decimal,
    /// Target gold price (reference spot price from trusted oracle).
    pub target_gold_price_usd: Decimal,
    /// Market volatility on a 0.0-1.0 scale.
    pub market_volatility: Decimal,
    /// Transaction volume in gold grams per period.
    pub transaction_volume: Decimal,
    /// Network liquidity depth in gold grams available.
    pub liquidity_depth: Decimal,
    /// Network velocity (transactions per second).
    pub network_velocity: Decimal,
    /// Per-tier active packet counts.
    pub active_packets_by_tier: TierCounts,
    /// In-transit float: total value currently in-flight (liquidity shadow).
    pub in_transit_float: Decimal,
}

/// Per-tier active packet counts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TierCounts {
    pub l0: u64,
    pub l1: u64,
    pub l2: u64,
    pub l3: u64,
}

/// Fee reward distribution between egress and transit nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardSplit {
    /// 80% of total fee goes to the egress (settlement) node.
    pub egress_share: GoldGrams,
    /// 20% of total fee goes to transit nodes (weighted by trust).
    pub transit_share: GoldGrams,
}

// -- Thresholds & constants ------------------------------------------------

const MAX_FEE_ADJ: Decimal = dec!(0.02);
const MIN_FEE_ADJ: Decimal = dec!(-0.02);
const GOLD_DEV_EMERGENCY: Decimal = dec!(0.18);
const HIGH_VELOCITY: Decimal = dec!(1.5);
const LOW_VELOCITY: Decimal = dec!(0.3);
const LOW_VOLUME: Decimal = dec!(100000);
const HIGH_VOLUME: Decimal = dec!(1000000);
const LOW_LIQUIDITY: Decimal = dec!(100000);
const HIGH_LIQUIDITY: Decimal = dec!(1000000);
const EGRESS_SHARE: Decimal = dec!(0.8);
const TRANSIT_SHARE: Decimal = dec!(0.2);

// -- GovernorPid -----------------------------------------------------------

/// PID controller producing [`GovernanceParams`] from [`NetworkMetrics`].
pub struct GovernorPid {
    last_params: GovernanceParams,
    integral_error: Decimal,
    kp: Decimal,
    ki: Decimal,
    kd: Decimal,
}

impl GovernorPid {
    /// Create with default gains: Kp=0.5, Ki=0.1, Kd=0.05.
    pub fn new() -> Self {
        Self {
            last_params: GovernanceParams::default(),
            integral_error: dec!(0),
            kp: dec!(0.5),
            ki: dec!(0.1),
            kd: dec!(0.05),
        }
    }

    /// Create with custom PID gains.
    pub fn with_gains(kp: Decimal, ki: Decimal, kd: Decimal) -> Self {
        Self { kp, ki, kd, ..Self::new() }
    }

    /// Run one PID control cycle, producing updated [`GovernanceParams`].
    pub fn recalculate(&mut self, metrics: &NetworkMetrics) -> GovernanceParams {
        let error = self.gold_deviation(metrics);
        let health = self.calculate_economic_health_score(metrics);
        let base_adj = self.score_to_fee_adjustment(health);

        self.integral_error += error;
        let derivative = error - self.last_params.recommended_fee_adjustment;
        let pid = self.kp * error + self.ki * self.integral_error + self.kd * derivative;
        let clamped = (base_adj + pid).clamp(MIN_FEE_ADJ, MAX_FEE_ADJ);

        let params = GovernanceParams {
            fee_modifiers: self.compute_tier_modifiers(clamped),
            demurrage_overrides: TierDemurrageOverrides::default(),
            pressure: self.classify_pressure(metrics),
            health_score: health,
            recommended_fee_adjustment: clamped,
            fee_caps: FeeCaps::default(),
        };
        self.last_params = params.clone();
        params
    }

    /// Classify the current network pressure quadrant.
    pub fn classify_pressure(&self, m: &NetworkMetrics) -> PressureQuadrant {
        let dev = self.gold_deviation(m);
        if dev > GOLD_DEV_EMERGENCY {
            return if m.network_velocity > HIGH_VELOCITY {
                PressureQuadrant::Bubble
            } else {
                PressureQuadrant::Bottleneck
            };
        }
        if dev < -GOLD_DEV_EMERGENCY { return PressureQuadrant::Crash; }
        if m.network_velocity < LOW_VELOCITY && m.transaction_volume < LOW_VOLUME {
            return PressureQuadrant::Stagnation;
        }
        if m.liquidity_depth > HIGH_LIQUIDITY && m.transaction_volume < LOW_VOLUME {
            return PressureQuadrant::Vacuum;
        }
        PressureQuadrant::GoldenEra
    }

    /// Economic health score (0-10). Weights: 40% gold, 30% vol, 20% txn, 10% liq.
    pub fn calculate_economic_health_score(&self, m: &NetworkMetrics) -> Decimal {
        let gold = (dec!(1) - self.gold_deviation(m).abs()).max(dec!(0)) * dec!(10);
        let vol = (dec!(1) - m.market_volatility).max(dec!(0)) * dec!(10);
        let txn = (m.transaction_volume / HIGH_VOLUME).min(dec!(10));
        let liq = (m.liquidity_depth / LOW_LIQUIDITY).min(dec!(10));
        gold * dec!(0.4) + vol * dec!(0.3) + txn * dec!(0.2) + liq * dec!(0.1)
    }

    /// Map health score to fee adjustment fraction.
    pub fn score_to_fee_adjustment(&self, score: Decimal) -> Decimal {
        if score >= dec!(85) { dec!(-0.008) }
        else if score >= dec!(75) { dec!(-0.006) }
        else if score >= dec!(65) { dec!(-0.004) }
        else if score >= dec!(55) { dec!(-0.002) }
        else if score >= dec!(50) { dec!(0) }
        else if score >= dec!(40) { dec!(0.002) }
        else { dec!(0.005) }
    }

    /// Calculate effective fee for a tier, clamped to the constitutional cap.
    ///
    /// The raw fee is `base * tier_modifier`, floored at zero, then capped
    /// at `packet_value * fee_cap_percentage` for the tier.
    pub fn calculate_fee(
        &self,
        p: &GovernanceParams,
        tier: MarketTier,
        base: Decimal,
        packet_value: Decimal,
    ) -> Decimal {
        let raw = (base * p.fee_modifiers.for_tier(tier)).max(dec!(0));
        let max_fee = packet_value * p.fee_caps.cap_for(tier);
        raw.min(max_fee)
    }

    /// Split total fee: 80% egress, 20% transit.
    pub fn split_rewards(&self, total: GoldGrams) -> RewardSplit {
        RewardSplit {
            egress_share: GoldGrams::from_decimal(total.0 * EGRESS_SHARE),
            transit_share: GoldGrams::from_decimal(total.0 * TRANSIT_SHARE),
        }
    }

    fn gold_deviation(&self, m: &NetworkMetrics) -> Decimal {
        if m.target_gold_price_usd.is_zero() { return dec!(0); }
        (m.current_gold_price_usd - m.target_gold_price_usd) / m.target_gold_price_usd
    }

    fn compute_tier_modifiers(&self, adj: Decimal) -> TierModifiers {
        TierModifiers {
            l0: dec!(1) + adj * dec!(1.5),
            l1: dec!(1) + adj * dec!(1.2),
            l2: dec!(1) + adj * dec!(0.8),
            l3: dec!(1) + adj * dec!(0.5),
        }
    }
}

impl Default for GovernorPid { fn default() -> Self { Self::new() } }

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics(
        gold: Decimal, target: Decimal, volatility: Decimal,
        volume: Decimal, liquidity: Decimal, velocity: Decimal,
    ) -> NetworkMetrics {
        NetworkMetrics {
            current_gold_price_usd: gold,
            target_gold_price_usd: target,
            market_volatility: volatility,
            transaction_volume: volume,
            liquidity_depth: liquidity,
            network_velocity: velocity,
            active_packets_by_tier: TierCounts::default(),
            in_transit_float: dec!(0),
        }
    }

    fn golden_era() -> NetworkMetrics {
        metrics(dec!(84.5), dec!(84), dec!(0.10), dec!(500000), dec!(1500000), dec!(1.0))
    }
    fn bubble() -> NetworkMetrics {
        metrics(dec!(110), dec!(84), dec!(0.45), dec!(2000000), dec!(5000000), dec!(2.0))
    }
    fn crash() -> NetworkMetrics {
        metrics(dec!(64), dec!(84), dec!(0.80), dec!(5000000), dec!(100000), dec!(0.5))
    }
    fn stagnation() -> NetworkMetrics {
        metrics(dec!(83), dec!(84), dec!(0.05), dec!(50000), dec!(200000), dec!(0.1))
    }
    fn vacuum() -> NetworkMetrics {
        metrics(dec!(84.2), dec!(84), dec!(0.05), dec!(50000), dec!(2000000), dec!(0.8))
    }

    #[test]
    fn pressure_classification() {
        let g = GovernorPid::new();
        assert_eq!(g.classify_pressure(&golden_era()), PressureQuadrant::GoldenEra);
        assert_eq!(g.classify_pressure(&bubble()), PressureQuadrant::Bubble);
        assert_eq!(g.classify_pressure(&crash()), PressureQuadrant::Crash);
        assert_eq!(g.classify_pressure(&stagnation()), PressureQuadrant::Stagnation);
        assert_eq!(g.classify_pressure(&vacuum()), PressureQuadrant::Vacuum);
    }

    #[test]
    fn health_scoring() {
        let g = GovernorPid::new();
        let era_score = g.calculate_economic_health_score(&golden_era());
        let crash_score = g.calculate_economic_health_score(&crash());
        assert!(era_score > dec!(4), "golden era score {} > 4", era_score);
        assert!(crash_score < dec!(5), "crash score {} < 5", crash_score);
        assert!(era_score > crash_score, "golden era {} > crash {}", era_score, crash_score);
    }

    #[test]
    fn score_to_fee_boundaries() {
        let g = GovernorPid::new();
        assert_eq!(g.score_to_fee_adjustment(dec!(90)), dec!(-0.008));
        assert_eq!(g.score_to_fee_adjustment(dec!(85)), dec!(-0.008));
        assert_eq!(g.score_to_fee_adjustment(dec!(75)), dec!(-0.006));
        assert_eq!(g.score_to_fee_adjustment(dec!(70)), dec!(-0.004));
        assert_eq!(g.score_to_fee_adjustment(dec!(55)), dec!(-0.002));
        assert_eq!(g.score_to_fee_adjustment(dec!(50)), dec!(0));
        assert_eq!(g.score_to_fee_adjustment(dec!(40)), dec!(0.002));
        assert_eq!(g.score_to_fee_adjustment(dec!(35)), dec!(0.005));
    }

    #[test]
    fn fee_calculation() {
        let g = GovernorPid::new();
        let packet_val = dec!(1000);
        let neutral = GovernanceParams::default();
        // With neutral modifiers (1.0x), fee of 10 on 1000g packet:
        // raw = 10, cap = 1000 * 0.05 = 50, so result = 10
        assert_eq!(
            g.calculate_fee(&neutral, MarketTier::L0, dec!(10), packet_val),
            dec!(10)
        );

        let discounted = GovernanceParams {
            fee_modifiers: TierModifiers { l0: dec!(0.5), ..TierModifiers::default() },
            ..GovernanceParams::default()
        };
        assert_eq!(
            g.calculate_fee(&discounted, MarketTier::L0, dec!(10), packet_val),
            dec!(5)
        );

        let negative = GovernanceParams {
            fee_modifiers: TierModifiers { l0: dec!(-1), ..TierModifiers::default() },
            ..GovernanceParams::default()
        };
        assert_eq!(
            g.calculate_fee(&negative, MarketTier::L0, dec!(10), packet_val),
            dec!(0)
        );
    }

    #[test]
    fn reward_split() {
        let g = GovernorPid::new();
        let s = g.split_rewards(GoldGrams::from_decimal(dec!(100)));
        assert_eq!(s.egress_share.0, dec!(80));
        assert_eq!(s.transit_share.0, dec!(20));

        let z = g.split_rewards(GoldGrams::zero());
        assert!(z.egress_share.is_zero());
        assert!(z.transit_share.is_zero());
    }

    #[test]
    fn pid_integral_accumulates() {
        let mild = metrics(dec!(85), dec!(84), dec!(0.10), dec!(500000), dec!(1500000), dec!(1.0));
        let mut g = GovernorPid::new();
        let p1 = g.recalculate(&mild);
        let p2 = g.recalculate(&mild);
        assert_ne!(p1.recommended_fee_adjustment, p2.recommended_fee_adjustment,
            "integral should cause drift between calls");
    }

    #[test]
    fn fee_adjustment_clamped() {
        let mut g = GovernorPid::new();
        let up = g.recalculate(&bubble());
        assert!(up.recommended_fee_adjustment <= dec!(0.02),
            "upper clamp: {}", up.recommended_fee_adjustment);

        let mut g2 = GovernorPid::new();
        let healthy = metrics(dec!(84), dec!(84), dec!(0.01), dec!(10000000), dec!(10000000), dec!(1.0));
        let down = g2.recalculate(&healthy);
        assert!(down.recommended_fee_adjustment >= dec!(-0.02),
            "lower clamp: {}", down.recommended_fee_adjustment);
    }

    #[test]
    fn recalculate_integration() {
        let mut g = GovernorPid::new();
        let era = g.recalculate(&golden_era());
        assert_eq!(era.pressure, PressureQuadrant::GoldenEra);
        assert!(era.health_score > dec!(3), "health {} > 3", era.health_score);

        let mut g2 = GovernorPid::new();
        let cr = g2.recalculate(&crash());
        assert_eq!(cr.pressure, PressureQuadrant::Crash);
    }

    #[test]
    fn tier_modifier_sensitivity() {
        let g = GovernorPid::new();
        let m = g.compute_tier_modifiers(dec!(0.01));
        assert!((m.l0 - dec!(1)).abs() > (m.l3 - dec!(1)).abs(),
            "L0 more sensitive than L3");
    }

    #[test]
    fn default_and_custom_gains() {
        let a = GovernorPid::new();
        let b = GovernorPid::default();
        assert_eq!(a.kp, b.kp);
        assert_eq!(a.ki, b.ki);
        assert_eq!(a.kd, b.kd);

        let c = GovernorPid::with_gains(dec!(1), dec!(2), dec!(3));
        assert_eq!(c.kp, dec!(1));
        assert_eq!(c.ki, dec!(2));
        assert_eq!(c.kd, dec!(3));
    }

    #[test]
    fn gold_deviation_zero_target() {
        let g = GovernorPid::new();
        let m = metrics(dec!(100), dec!(0), dec!(0.1), dec!(500000), dec!(1000000), dec!(1.0));
        assert_eq!(g.gold_deviation(&m), dec!(0));
    }

    // -- Fee cap enforcement tests (18D) ------------------------------------

    #[test]
    fn calculate_fee_respects_caps() {
        let g = GovernorPid::new();
        let params = GovernanceParams {
            // High modifier to push fee above cap
            fee_modifiers: TierModifiers { l0: dec!(10), ..TierModifiers::default() },
            ..GovernanceParams::default()
        };
        // base=10, modifier=10x => raw=100, but L0 cap = 5% of 100 = 5
        let fee = g.calculate_fee(&params, MarketTier::L0, dec!(10), dec!(100));
        assert_eq!(fee, dec!(5), "fee should be capped at 5% of 100g");
    }

    #[test]
    fn calculate_fee_under_cap_passes_through() {
        let g = GovernorPid::new();
        let params = GovernanceParams::default();
        // base=1, modifier=1x => raw=1, L0 cap = 5% of 1000 = 50, so 1 < 50
        let fee = g.calculate_fee(&params, MarketTier::L0, dec!(1), dec!(1000));
        assert_eq!(fee, dec!(1), "fee under cap should pass through");
    }

    #[test]
    fn calculate_fee_l3_tight_cap() {
        let g = GovernorPid::new();
        let params = GovernanceParams {
            fee_modifiers: TierModifiers { l3: dec!(5), ..TierModifiers::default() },
            ..GovernanceParams::default()
        };
        // base=10, modifier=5x => raw=50, L3 cap = 0.1% of 1000 = 1
        let fee = g.calculate_fee(&params, MarketTier::L3, dec!(10), dec!(1000));
        assert_eq!(fee, dec!(1), "L3 cap = 0.1% of 1000 = 1g");
    }

    #[test]
    fn in_transit_float_field() {
        let m = NetworkMetrics {
            current_gold_price_usd: dec!(84),
            target_gold_price_usd: dec!(84),
            market_volatility: dec!(0.1),
            transaction_volume: dec!(500000),
            liquidity_depth: dec!(1000000),
            network_velocity: dec!(1.0),
            active_packets_by_tier: TierCounts::default(),
            in_transit_float: dec!(250000),
        };
        assert_eq!(m.in_transit_float, dec!(250000));
    }
}
