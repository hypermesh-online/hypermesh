// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Governance output parameters and network pressure classification.
//!
//! Defines the output surface of the Governor PID controller: per-tier fee
//! modifiers, demurrage overrides, pressure quadrant, and health scoring.

use hypermesh_lib::{DemurrageRate, GoldGrams, MarketTier};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// GovernanceParams
// ---------------------------------------------------------------------------

/// Output of the Governor PID controller -- modulations applied to the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceParams {
    /// Current fee modifier per tier (multiplier, 1.0 = no change).
    pub fee_modifiers: TierModifiers,
    /// Demurrage rate overrides (None = use tier defaults).
    pub demurrage_overrides: TierDemurrageOverrides,
    /// Current network pressure classification.
    pub pressure: PressureQuadrant,
    /// Overall health score (0-100).
    pub health_score: Decimal,
    /// Recommended fee adjustment as fraction (e.g., -0.008 = -0.8%).
    pub recommended_fee_adjustment: Decimal,
    /// Constitutional fee caps per tier.
    pub fee_caps: FeeCaps,
}

impl Default for GovernanceParams {
    fn default() -> Self {
        Self {
            fee_modifiers: TierModifiers::default(),
            demurrage_overrides: TierDemurrageOverrides::default(),
            pressure: PressureQuadrant::GoldenEra,
            health_score: dec!(50),
            recommended_fee_adjustment: dec!(0),
            fee_caps: FeeCaps::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// FeeCaps
// ---------------------------------------------------------------------------

/// Constitutional fee caps per tier -- the Governor CANNOT exceed these under any conditions.
/// Whitepaper section 6.5: These are baked into the protocol specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeCaps {
    /// L0 (retail) cap: 5%
    pub l0: Decimal,
    /// L1 (professional) cap: 2%
    pub l1: Decimal,
    /// L2 (institutional) cap: 0.5%
    pub l2: Decimal,
    /// L3 (sovereign) cap: 0.1%
    pub l3: Decimal,
}

impl Default for FeeCaps {
    fn default() -> Self {
        Self {
            l0: dec!(0.05),
            l1: dec!(0.02),
            l2: dec!(0.005),
            l3: dec!(0.001),
        }
    }
}

impl FeeCaps {
    /// Look up the cap percentage for a given tier.
    pub fn cap_for(&self, tier: MarketTier) -> Decimal {
        match tier {
            MarketTier::L0 => self.l0,
            MarketTier::L1 => self.l1,
            MarketTier::L2 => self.l2,
            MarketTier::L3 => self.l3,
        }
    }

    /// Clamp a fee to the constitutional cap for a given tier.
    /// fee_cap = cap_percentage * packet_value
    pub fn clamp_fee(
        &self,
        tier: MarketTier,
        fee: GoldGrams,
        packet_value: GoldGrams,
    ) -> GoldGrams {
        let max_fee = packet_value.0 * self.cap_for(tier);
        if fee.0 > max_fee {
            GoldGrams::from_decimal(max_fee)
        } else {
            fee
        }
    }
}

// ---------------------------------------------------------------------------
// TierModifiers
// ---------------------------------------------------------------------------

/// Per-tier fee modifiers (multipliers applied to the base fee).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierModifiers {
    pub l0: Decimal,
    pub l1: Decimal,
    pub l2: Decimal,
    pub l3: Decimal,
}

impl TierModifiers {
    /// Look up the modifier for a given [`MarketTier`].
    pub fn for_tier(&self, tier: MarketTier) -> Decimal {
        match tier {
            MarketTier::L0 => self.l0,
            MarketTier::L1 => self.l1,
            MarketTier::L2 => self.l2,
            MarketTier::L3 => self.l3,
        }
    }
}

impl Default for TierModifiers {
    fn default() -> Self {
        Self {
            l0: dec!(1),
            l1: dec!(1),
            l2: dec!(1),
            l3: dec!(1),
        }
    }
}

// ---------------------------------------------------------------------------
// TierDemurrageOverrides
// ---------------------------------------------------------------------------

/// Per-tier demurrage overrides. `None` means "use tier default".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierDemurrageOverrides {
    pub l0: Option<DemurrageRate>,
    pub l1: Option<DemurrageRate>,
    pub l2: Option<DemurrageRate>,
    pub l3: Option<DemurrageRate>,
}

impl TierDemurrageOverrides {
    /// Look up the override for a given [`MarketTier`].
    pub fn for_tier(&self, tier: MarketTier) -> Option<DemurrageRate> {
        match tier {
            MarketTier::L0 => self.l0,
            MarketTier::L1 => self.l1,
            MarketTier::L2 => self.l2,
            MarketTier::L3 => self.l3,
        }
    }
}

impl Default for TierDemurrageOverrides {
    fn default() -> Self {
        Self {
            l0: None,
            l1: None,
            l2: None,
            l3: None,
        }
    }
}

// ---------------------------------------------------------------------------
// PressureQuadrant
// ---------------------------------------------------------------------------

/// Network pressure classification -- six quadrants describing macro conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PressureQuadrant {
    /// High velocity + high gold deviation upward: speculative bubble.
    Bubble,
    /// High velocity + high gold deviation downward: market crash.
    Crash,
    /// Low velocity + low volume: economic stagnation.
    Stagnation,
    /// Moderate velocity + tight gold band + good liquidity: ideal state.
    GoldenEra,
    /// High volume + low liquidity: infrastructure bottleneck.
    Bottleneck,
    /// Low volume + high liquidity: excess capacity vacuum.
    Vacuum,
}

impl PressureQuadrant {
    /// Human-readable description of this quadrant.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Bubble => "Speculative bubble: high velocity with gold deviation upward",
            Self::Crash => "Market crash: high velocity with gold deviation downward",
            Self::Stagnation => "Economic stagnation: low velocity and low volume",
            Self::GoldenEra => "Golden era: moderate velocity, tight gold band, good liquidity",
            Self::Bottleneck => "Infrastructure bottleneck: high volume but low liquidity",
            Self::Vacuum => "Excess capacity vacuum: low volume with high liquidity",
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_governance_params() {
        let params = GovernanceParams::default();
        assert_eq!(params.pressure, PressureQuadrant::GoldenEra);
        assert_eq!(params.health_score, dec!(50));
        assert_eq!(params.recommended_fee_adjustment, dec!(0));
    }

    #[test]
    fn default_tier_modifiers_are_neutral() {
        let mods = TierModifiers::default();
        assert_eq!(mods.l0, dec!(1));
        assert_eq!(mods.l1, dec!(1));
        assert_eq!(mods.l2, dec!(1));
        assert_eq!(mods.l3, dec!(1));
    }

    #[test]
    fn tier_modifier_lookup() {
        let mods = TierModifiers {
            l0: dec!(1.5),
            l1: dec!(1.2),
            l2: dec!(0.9),
            l3: dec!(0.5),
        };
        assert_eq!(mods.for_tier(MarketTier::L0), dec!(1.5));
        assert_eq!(mods.for_tier(MarketTier::L1), dec!(1.2));
        assert_eq!(mods.for_tier(MarketTier::L2), dec!(0.9));
        assert_eq!(mods.for_tier(MarketTier::L3), dec!(0.5));
    }

    #[test]
    fn default_demurrage_overrides_are_none() {
        let overrides = TierDemurrageOverrides::default();
        assert!(overrides.l0.is_none());
        assert!(overrides.l1.is_none());
        assert!(overrides.l2.is_none());
        assert!(overrides.l3.is_none());
    }

    #[test]
    fn demurrage_override_lookup() {
        let custom = DemurrageRate {
            lambda: 1.0e-6,
            max_ttl_secs: 3600,
        };
        let overrides = TierDemurrageOverrides {
            l0: Some(custom),
            l1: None,
            l2: None,
            l3: None,
        };
        let l0 = overrides
            .for_tier(MarketTier::L0)
            .expect("test: L0 override should be Some");
        assert_eq!(l0.lambda, custom.lambda);
        assert_eq!(l0.max_ttl_secs, custom.max_ttl_secs);
        assert!(overrides.for_tier(MarketTier::L1).is_none());
    }

    #[test]
    fn pressure_quadrant_descriptions_non_empty() {
        let quadrants = [
            PressureQuadrant::Bubble,
            PressureQuadrant::Crash,
            PressureQuadrant::Stagnation,
            PressureQuadrant::GoldenEra,
            PressureQuadrant::Bottleneck,
            PressureQuadrant::Vacuum,
        ];
        for q in &quadrants {
            assert!(
                !q.description().is_empty(),
                "{:?} description must not be empty",
                q
            );
        }
    }

    #[test]
    fn pressure_quadrant_equality() {
        assert_eq!(PressureQuadrant::Bubble, PressureQuadrant::Bubble);
        assert_ne!(PressureQuadrant::Bubble, PressureQuadrant::Crash);
    }

    // -- FeeCaps tests (18D) ------------------------------------------------

    #[test]
    fn fee_caps_default_values() {
        let caps = FeeCaps::default();
        assert_eq!(caps.l0, dec!(0.05), "L0 cap should be 5%");
        assert_eq!(caps.l1, dec!(0.02), "L1 cap should be 2%");
        assert_eq!(caps.l2, dec!(0.005), "L2 cap should be 0.5%");
        assert_eq!(caps.l3, dec!(0.001), "L3 cap should be 0.1%");
    }

    #[test]
    fn fee_cap_clamps_excessive_fee() {
        let caps = FeeCaps::default();
        let packet_value = GoldGrams(dec!(100));
        let excessive_fee = GoldGrams(dec!(10));
        // L0 cap = 5% of 100 = 5g, so 10g should be clamped to 5g
        let clamped = caps.clamp_fee(MarketTier::L0, excessive_fee, packet_value);
        assert_eq!(clamped, GoldGrams(dec!(5)));
    }

    #[test]
    fn fee_cap_allows_under_cap() {
        let caps = FeeCaps::default();
        let packet_value = GoldGrams(dec!(100));
        let reasonable_fee = GoldGrams(dec!(3));
        // L0 cap = 5% of 100 = 5g, so 3g should pass through
        let result = caps.clamp_fee(MarketTier::L0, reasonable_fee, packet_value);
        assert_eq!(result, GoldGrams(dec!(3)));
    }

    #[test]
    fn fee_cap_l3_very_tight() {
        let caps = FeeCaps::default();
        let packet_value = GoldGrams(dec!(1000));
        // L3 cap = 0.1% of 1000 = 1g
        let excessive = GoldGrams(dec!(5));
        let clamped = caps.clamp_fee(MarketTier::L3, excessive, packet_value);
        assert_eq!(clamped, GoldGrams(dec!(1)));

        let under = GoldGrams(dec!(0.5));
        let result = caps.clamp_fee(MarketTier::L3, under, packet_value);
        assert_eq!(result, GoldGrams(dec!(0.5)));
    }

    #[test]
    fn fee_cap_lookup_all_tiers() {
        let caps = FeeCaps::default();
        assert_eq!(caps.cap_for(MarketTier::L0), dec!(0.05));
        assert_eq!(caps.cap_for(MarketTier::L1), dec!(0.02));
        assert_eq!(caps.cap_for(MarketTier::L2), dec!(0.005));
        assert_eq!(caps.cap_for(MarketTier::L3), dec!(0.001));
    }
}
