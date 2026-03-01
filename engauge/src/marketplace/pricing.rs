// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Governor-adjusted pricing engine for the capacity marketplace.
//!
//! Calculates lease prices based on resource type, allocation percentage,
//! and market tier. Tier multipliers ensure sovereign/institutional actors
//! pay proportionally less per unit (volume discount).

use std::collections::HashMap;

use hypermesh_lib::economic::{GoldGrams, MarketTier};
use rust_decimal::Decimal;

use super::resource_pool::LeaseableResource;

/// Pricing engine that calculates lease costs per epoch.
pub struct PricingEngine {
    /// Base price per epoch per resource type (in gold grams).
    base_prices: HashMap<LeaseableResource, GoldGrams>,
}

impl PricingEngine {
    /// Create a pricing engine with sensible defaults.
    pub fn new() -> Self {
        let mut base_prices = HashMap::new();
        base_prices.insert(
            LeaseableResource::Cpu,
            GoldGrams::from_decimal(Decimal::new(1, 3)), // 0.001g
        );
        base_prices.insert(
            LeaseableResource::Gpu,
            GoldGrams::from_decimal(Decimal::new(1, 2)), // 0.01g
        );
        base_prices.insert(
            LeaseableResource::Memory,
            GoldGrams::from_decimal(Decimal::new(5, 4)), // 0.0005g
        );
        base_prices.insert(
            LeaseableResource::Storage,
            GoldGrams::from_decimal(Decimal::new(1, 4)), // 0.0001g
        );
        base_prices.insert(
            LeaseableResource::Bandwidth,
            GoldGrams::from_decimal(Decimal::new(2, 4)), // 0.0002g
        );
        Self { base_prices }
    }

    /// Set or update base price for a resource.
    pub fn set_base_price(&mut self, resource: LeaseableResource, price: GoldGrams) {
        self.base_prices.insert(resource, price);
    }

    /// Calculate the price per epoch for a lease, adjusted by tier.
    ///
    /// Formula: `base_price * (allocation_pct / 100) * tier_multiplier`
    ///
    /// Tier multipliers: L0 = 1.0, L1 = 0.8, L2 = 0.5, L3 = 0.2
    pub fn calculate_price(
        &self,
        resource: LeaseableResource,
        allocation_pct: u8,
        tier: MarketTier,
    ) -> GoldGrams {
        let base = match self.base_prices.get(&resource) {
            Some(p) => p.0,
            None => return GoldGrams::zero(),
        };

        let pct = Decimal::new(allocation_pct.into(), 2); // e.g. 50 -> 0.50
        let multiplier = tier_multiplier(tier);
        let price = base * pct * multiplier;

        GoldGrams::from_decimal(price)
    }

    /// Get base price for a resource.
    pub fn base_price(&self, resource: &LeaseableResource) -> Option<GoldGrams> {
        self.base_prices.get(resource).copied()
    }
}

impl Default for PricingEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Tier multiplier for pricing.
///
/// Higher tiers (institutional/sovereign) get volume discounts.
fn tier_multiplier(tier: MarketTier) -> Decimal {
    match tier {
        MarketTier::L0 => Decimal::ONE,       // 1.0
        MarketTier::L1 => Decimal::new(8, 1), // 0.8
        MarketTier::L2 => Decimal::new(5, 1), // 0.5
        MarketTier::L3 => Decimal::new(2, 1), // 0.2
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_price_with_default_base_prices() {
        let engine = PricingEngine::new();

        // CPU: base 0.001g, 100% allocation, L0 (mult 1.0)
        // Expected: 0.001 * 1.0 * 1.0 = 0.001g
        let price = engine.calculate_price(LeaseableResource::Cpu, 100, MarketTier::L0);
        assert_eq!(price.0, Decimal::new(1, 3));
    }

    #[test]
    fn tier_multiplier_affects_price() {
        let engine = PricingEngine::new();

        let l0_price = engine.calculate_price(LeaseableResource::Gpu, 100, MarketTier::L0);
        let l3_price = engine.calculate_price(LeaseableResource::Gpu, 100, MarketTier::L3);

        // L0 price should be greater than L3 price (L3 has 0.2 multiplier).
        assert!(
            l0_price.0 > l3_price.0,
            "L0={l0_price} should > L3={l3_price}"
        );

        // L3 should be 20% of L0.
        let expected_l3 = l0_price.0 * Decimal::new(2, 1);
        assert_eq!(l3_price.0, expected_l3);
    }

    #[test]
    fn custom_base_price() {
        let mut engine = PricingEngine::new();
        let custom_price = GoldGrams::from_decimal(Decimal::new(5, 2)); // 0.05g
        engine.set_base_price(LeaseableResource::Cpu, custom_price);

        // 50% allocation, L0 tier: 0.05 * 0.50 * 1.0 = 0.025g
        let price = engine.calculate_price(LeaseableResource::Cpu, 50, MarketTier::L0);
        assert_eq!(price.0, Decimal::new(25, 3));

        let retrieved = engine
            .base_price(&LeaseableResource::Cpu)
            .expect("test: base price exists");
        assert_eq!(retrieved.0, Decimal::new(5, 2));
    }
}
