// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Canonical Ephemeral Value Protocol (EVP) types
//!
//! Gold-gram denominated value packets that decay over time via demurrage.
//! These types are shared across all HyperMesh crates (Caesar, BlockMatrix, etc.).

use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Sub};

/// Unique EVP packet identifier (32-byte hash)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PacketId(pub [u8; 32]);

impl PacketId {
    /// Create from raw bytes
    pub fn new(data: [u8; 32]) -> Self {
        Self(data)
    }

    /// Create a zeroed identifier (for defaults/tests)
    pub fn zero() -> Self {
        Self([0u8; 32])
    }
}

impl fmt::Display for PacketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0[..8] {
            write!(f, "{:02x}", byte)?;
        }
        write!(f, "...")
    }
}

/// Gold-gram denomination backed by `rust_decimal::Decimal`
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GoldGrams(pub Decimal);

impl GoldGrams {
    /// Zero value
    pub fn zero() -> Self {
        Self(Decimal::ZERO)
    }

    /// Create from a `Decimal` value
    pub fn from_decimal(d: Decimal) -> Self {
        Self(d)
    }

    /// Whether the value is exactly zero
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl Add for GoldGrams {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for GoldGrams {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl fmt::Display for GoldGrams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}g", self.0)
    }
}

/// Market tier classified by transaction value amount
///
/// L0 (retail) through L3 (sovereign), each with distinct demurrage parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketTier {
    /// Retail / Consumers
    L0,
    /// Professional / Small Institutions
    L1,
    /// Major Institutions
    L2,
    /// Sovereign / Systemic Actors
    L3,
}

impl MarketTier {
    /// Human-readable description of this tier
    pub fn description(&self) -> &'static str {
        match self {
            Self::L0 => "Retail / Consumers",
            Self::L1 => "Professional / Small Institutions",
            Self::L2 => "Major Institutions",
            Self::L3 => "Sovereign / Systemic Actors",
        }
    }

    /// Default demurrage parameters for this tier
    pub fn default_demurrage_rate(&self) -> DemurrageRate {
        match self {
            Self::L0 => DemurrageRate { lambda: 1.39e-5, max_ttl_secs: 86_400 },       // ~5%/hr, TTL 1 day
            Self::L1 => DemurrageRate { lambda: 1.157e-8, max_ttl_secs: 1_209_600 },   // ~0.1%/day, TTL 14 days
            Self::L2 => DemurrageRate { lambda: 1.157e-9, max_ttl_secs: 7_776_000 },   // ~0.01%/day, TTL 90 days
            Self::L3 => DemurrageRate { lambda: 1.157e-10, max_ttl_secs: 15_552_000 }, // ~0.001%/day, TTL 180 days
        }
    }
}

/// EVP lifecycle state
///
/// Born at `Minted`, dies at `Settled` / `Refunded` / `Dissolved`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PacketState {
    /// Just created at ingress
    Minted,
    /// Moving through mesh
    InTransit,
    /// Arrived at destination, awaiting settlement
    Delivered,
    /// External settlement in progress (egress adapter executing)
    Settling,
    /// TERMINAL: Successfully settled
    Settled,
    /// In holding pattern (orbit buffer) -- recipient offline/unavailable
    Held,
    /// Delivery failed, awaiting retry or refund
    Stalled,
    /// Egress settlement failed — shards re-dispersed for retry
    Dispersed,
    /// TTL expired — refund process initiated (non-terminal)
    Expired,
    /// TERMINAL: TTL expired, refund completed to sender
    Refunded,
    /// TERMINAL: Both parties abandoned, gravity bonus distributed
    Dissolved,
}

impl PacketState {
    /// Whether this state is terminal (no further transitions)
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Settled | Self::Refunded | Self::Dissolved)
    }

    /// Whether this state is active (packet still in flight or actionable)
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Minted
                | Self::InTransit
                | Self::Delivered
                | Self::Settling
                | Self::Held
                | Self::Stalled
                | Self::Dispersed
                | Self::Expired
        )
    }
}

/// Per-tier demurrage (decay) parameters
///
/// Value decays exponentially: `V_t = V_0 * e^(-lambda * t)`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DemurrageRate {
    /// Decay rate per second (lambda)
    pub lambda: f64,
    /// Maximum time-to-live in seconds before forced expiry
    pub max_ttl_secs: u64,
}

impl DemurrageRate {
    /// Calculate remaining value after `elapsed_secs` of decay.
    ///
    /// Uses `V_t = V_0 * e^(-lambda * t)`. Returns zero if elapsed exceeds max TTL.
    pub fn calculate_remaining(&self, initial: GoldGrams, elapsed_secs: u64) -> GoldGrams {
        if elapsed_secs >= self.max_ttl_secs {
            return GoldGrams::zero();
        }
        let factor = (-self.lambda * elapsed_secs as f64).exp();
        let factor_dec = match Decimal::from_f64(factor) {
            Some(d) => d,
            None => return GoldGrams::zero(),
        };
        GoldGrams(initial.0 * factor_dec)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_state_terminal_classification() {
        assert!(PacketState::Settled.is_terminal());
        assert!(PacketState::Refunded.is_terminal());
        assert!(PacketState::Dissolved.is_terminal());

        assert!(!PacketState::Minted.is_terminal());
        assert!(!PacketState::InTransit.is_terminal());
        assert!(!PacketState::Delivered.is_terminal());
        assert!(!PacketState::Settling.is_terminal());
        assert!(!PacketState::Held.is_terminal());
        assert!(!PacketState::Stalled.is_terminal());
        assert!(!PacketState::Dispersed.is_terminal());
        assert!(!PacketState::Expired.is_terminal());
    }

    #[test]
    fn packet_state_active_classification() {
        assert!(PacketState::Minted.is_active());
        assert!(PacketState::InTransit.is_active());
        assert!(PacketState::Delivered.is_active());
        assert!(PacketState::Settling.is_active());
        assert!(PacketState::Held.is_active());
        assert!(PacketState::Stalled.is_active());
        assert!(PacketState::Dispersed.is_active());
        assert!(PacketState::Expired.is_active());

        assert!(!PacketState::Settled.is_active());
        assert!(!PacketState::Refunded.is_active());
        assert!(!PacketState::Dissolved.is_active());
    }

    #[test]
    fn packet_state_terminal_and_active_are_disjoint() {
        let all = [
            PacketState::Minted, PacketState::InTransit, PacketState::Delivered,
            PacketState::Settling, PacketState::Settled, PacketState::Held,
            PacketState::Stalled, PacketState::Dispersed, PacketState::Expired,
            PacketState::Refunded, PacketState::Dissolved,
        ];
        for state in &all {
            assert_ne!(
                state.is_terminal(), state.is_active(),
                "{:?} must be exactly one of terminal or active", state
            );
        }
    }

    #[test]
    fn gold_grams_addition() {
        let a = GoldGrams::from_decimal(Decimal::new(100, 0));
        let b = GoldGrams::from_decimal(Decimal::new(250, 0));
        let sum = a + b;
        assert_eq!(sum.0, Decimal::new(350, 0));
    }

    #[test]
    fn gold_grams_subtraction() {
        let a = GoldGrams::from_decimal(Decimal::new(500, 0));
        let b = GoldGrams::from_decimal(Decimal::new(200, 0));
        let diff = a - b;
        assert_eq!(diff.0, Decimal::new(300, 0));
    }

    #[test]
    fn gold_grams_zero() {
        let z = GoldGrams::zero();
        assert!(z.is_zero());
        assert_eq!(z.0, Decimal::ZERO);
    }

    #[test]
    fn gold_grams_display() {
        let g = GoldGrams::from_decimal(Decimal::new(12345, 2)); // 123.45
        let s = format!("{}", g);
        assert_eq!(s, "123.45g");
    }

    #[test]
    fn demurrage_rate_l0_decay() {
        // L0: lambda = 1.39e-5, max_ttl = 86400s
        // Test at half the TTL (43200s) to stay within valid range
        let rate = MarketTier::L0.default_demurrage_rate();
        let initial = GoldGrams::from_decimal(Decimal::new(1000, 0));
        let remaining = rate.calculate_remaining(initial, 43_200);

        // e^(-1.39e-5 * 43200) ~ e^(-0.60048) ~ 0.5486
        // Remaining should be ~549g — verify it decayed but not to zero
        let ratio = remaining.0 / initial.0;
        assert!(
            ratio > Decimal::new(40, 2) && ratio < Decimal::new(70, 2),
            "L0 decay ratio after half-day: {}", ratio
        );
    }

    #[test]
    fn demurrage_rate_at_max_ttl_returns_zero() {
        // At exactly max TTL, value should be zero (expired)
        let rate = MarketTier::L0.default_demurrage_rate();
        let initial = GoldGrams::from_decimal(Decimal::new(1000, 0));
        let remaining = rate.calculate_remaining(initial, rate.max_ttl_secs);
        assert!(remaining.is_zero());
    }

    #[test]
    fn demurrage_rate_zero_elapsed() {
        let rate = MarketTier::L1.default_demurrage_rate();
        let initial = GoldGrams::from_decimal(Decimal::new(1000, 0));
        let remaining = rate.calculate_remaining(initial, 0);
        assert_eq!(remaining.0, initial.0);
    }

    #[test]
    fn demurrage_rate_past_ttl_returns_zero() {
        let rate = MarketTier::L0.default_demurrage_rate();
        let initial = GoldGrams::from_decimal(Decimal::new(1000, 0));
        let remaining = rate.calculate_remaining(initial, rate.max_ttl_secs + 1);
        assert!(remaining.is_zero());
    }

    #[test]
    fn market_tier_default_rates() {
        let l0 = MarketTier::L0.default_demurrage_rate();
        let l1 = MarketTier::L1.default_demurrage_rate();
        let l2 = MarketTier::L2.default_demurrage_rate();
        let l3 = MarketTier::L3.default_demurrage_rate();

        // Higher tiers have lower decay rates (longer half-lives)
        assert!(l0.lambda > l1.lambda);
        assert!(l1.lambda > l2.lambda);
        assert!(l2.lambda > l3.lambda);

        // Higher tiers have longer max TTLs
        assert!(l0.max_ttl_secs < l1.max_ttl_secs);
        assert!(l1.max_ttl_secs < l2.max_ttl_secs);
        assert!(l2.max_ttl_secs < l3.max_ttl_secs);
    }

    #[test]
    fn market_tier_descriptions() {
        assert!(!MarketTier::L0.description().is_empty());
        assert!(!MarketTier::L1.description().is_empty());
        assert!(!MarketTier::L2.description().is_empty());
        assert!(!MarketTier::L3.description().is_empty());
    }

    #[test]
    fn packet_id_display_truncated_hex() {
        let mut data = [0u8; 32];
        data[0] = 0xAB;
        data[1] = 0xCD;
        let id = PacketId::new(data);
        let s = format!("{}", id);
        assert!(s.starts_with("abcd"), "got: {}", s);
        assert!(s.ends_with("..."));
    }

    #[test]
    fn packet_id_zero() {
        let id = PacketId::zero();
        assert_eq!(id.0, [0u8; 32]);
    }

    #[test]
    fn demurrage_rate_l1_one_day_retention() {
        // L1: lambda = 1.157e-8, ~0.1%/day decay
        // After 1 day (86400s): e^(-1.157e-8 * 86400) = e^(-0.001) ~ 0.999
        // Should retain ~99.9% of value
        let rate = MarketTier::L1.default_demurrage_rate();
        let initial = GoldGrams::from_decimal(Decimal::new(1000, 0));
        let remaining = rate.calculate_remaining(initial, 86_400);
        let ratio = remaining.0 / initial.0;
        assert!(
            ratio > Decimal::new(999, 3),
            "L1 should retain >99.9% after 1 day, got ratio: {}", ratio
        );
    }
}
