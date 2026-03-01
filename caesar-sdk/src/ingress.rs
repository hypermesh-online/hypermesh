// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Ingress adapter trait -- external value entering the Caesar network.
//!
//! Implementors include fiat gateways (Stripe, Plaid, OpenBanking) and
//! crypto bridges (Uniswap, LayerZero, on-chain verification).
//!
//! The protocol is regulation-agnostic: adapters self-comply with their
//! jurisdiction's requirements.

use async_trait::async_trait;
use hypermesh_lib::GoldGrams;
use rust_decimal::Decimal;

use crate::types::{IngressLockProof, LiquidityPressure, UpiError};

/// Adapter for bringing external value into the Caesar network.
///
/// Each implementation wraps a single external system (one payment rail,
/// one chain, one exchange). The adapter is **stateless** -- all persistent
/// state lives in the external system or in the Caesar ledger.
#[async_trait]
pub trait IngressAdapter: Send + Sync {
    /// Unique identifier for this adapter (e.g. "stripe_us", "uniswap_v3_eth").
    fn adapter_id(&self) -> &str;

    /// Supported source denominations (e.g. `["USD", "EUR"]` or `["ETH", "BTC"]`).
    fn supported_denominations(&self) -> Vec<String>;

    /// Lock external value, preparing it for CAES minting.
    ///
    /// Returns proof of lock that can be verified independently.
    async fn lock_external_value(
        &self,
        amount: Decimal,
        denomination: &str,
        gold_price_usd: Decimal,
    ) -> Result<IngressLockProof, UpiError>;

    /// Verify that a lock proof is still valid and funds are held.
    ///
    /// Crypto adapters verify on-chain; fiat adapters check hold status.
    async fn verify_lock(&self, proof: &IngressLockProof) -> Result<bool, UpiError>;

    /// Release a lock (cancel ingress before minting).
    ///
    /// Returns the external value to the original owner.
    async fn release_lock(&self, proof: &IngressLockProof) -> Result<(), UpiError>;

    /// Report current liquidity pressure for Governor feedback.
    async fn liquidity_pressure(&self) -> Result<LiquidityPressure, UpiError>;

    /// Convert an external amount to its gold-gram equivalent.
    fn to_gold_grams(
        &self,
        amount: Decimal,
        denomination: &str,
        gold_price_usd: Decimal,
    ) -> Result<GoldGrams, UpiError>;
}

// ===========================================================================
// Mock adapter for testing
// ===========================================================================

/// Public mock module for SDK consumers to test their adapter integrations.
pub mod testing {
    use super::*;
    use chrono::Utc;
    use hypermesh_lib::NodeId;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Configurable mock ingress adapter for unit tests.
    pub struct MockIngressAdapter {
        /// Whether `verify_lock` returns true.
        pub lock_valid: AtomicBool,
        /// Whether `release_lock` has been called.
        pub released: AtomicBool,
    }

    impl MockIngressAdapter {
        pub fn new() -> Self {
            Self {
                lock_valid: AtomicBool::new(true),
                released: AtomicBool::new(false),
            }
        }
    }

    impl Default for MockIngressAdapter {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl IngressAdapter for MockIngressAdapter {
        fn adapter_id(&self) -> &str {
            "mock_ingress"
        }

        fn supported_denominations(&self) -> Vec<String> {
            vec!["USD".into(), "EUR".into()]
        }

        async fn lock_external_value(
            &self,
            amount: Decimal,
            denomination: &str,
            gold_price_usd: Decimal,
        ) -> Result<IngressLockProof, UpiError> {
            let gold = self.to_gold_grams(amount, denomination, gold_price_usd)?;
            let now = Utc::now();
            Ok(IngressLockProof {
                lock_id: "mock-lock-001".into(),
                adapter_id: self.adapter_id().into(),
                value: gold,
                source_denomination: denomination.into(),
                source_amount: amount,
                gold_price_at_lock: gold_price_usd,
                locking_node: NodeId::from_public_key(b"mock-node"),
                locked_at: now,
                expires_at: now + chrono::Duration::hours(1),
                external_reference: "mock-tx-ref".into(),
            })
        }

        async fn verify_lock(&self, _proof: &IngressLockProof) -> Result<bool, UpiError> {
            Ok(self.lock_valid.load(Ordering::Relaxed))
        }

        async fn release_lock(&self, _proof: &IngressLockProof) -> Result<(), UpiError> {
            self.released.store(true, Ordering::Relaxed);
            Ok(())
        }

        async fn liquidity_pressure(&self) -> Result<LiquidityPressure, UpiError> {
            Ok(LiquidityPressure {
                adapter_id: self.adapter_id().into(),
                ingress_capacity: GoldGrams::from_decimal(Decimal::new(10_000, 0)),
                egress_capacity: GoldGrams::from_decimal(Decimal::new(5_000, 0)),
                utilization: Decimal::new(35, 2), // 0.35
                estimated_latency_secs: 2,
            })
        }

        fn to_gold_grams(
            &self,
            amount: Decimal,
            denomination: &str,
            gold_price_usd: Decimal,
        ) -> Result<GoldGrams, UpiError> {
            if denomination != "USD" && denomination != "EUR" {
                return Err(UpiError::UnsupportedDenomination {
                    denomination: denomination.into(),
                });
            }
            // Simple conversion: amount_usd / gold_price_usd = grams
            let usd_amount = if denomination == "EUR" {
                amount * Decimal::new(110, 2) // rough EUR->USD
            } else {
                amount
            };
            Ok(GoldGrams::from_decimal(usd_amount / gold_price_usd))
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::testing::MockIngressAdapter;
    use super::*;
    use rust_decimal::Decimal;
    use std::sync::atomic::Ordering;

    fn gold_price() -> Decimal {
        Decimal::new(75, 0) // $75 per gram
    }

    #[tokio::test]
    async fn mock_adapter_returns_expected_lock_proof() {
        let adapter = MockIngressAdapter::new();
        let proof = adapter
            .lock_external_value(Decimal::new(150, 0), "USD", gold_price())
            .await
            .expect("test: lock should succeed");

        assert_eq!(proof.adapter_id, "mock_ingress");
        assert_eq!(proof.source_denomination, "USD");
        assert_eq!(proof.source_amount, Decimal::new(150, 0));
        // 150 USD / 75 USD-per-gram = 2.0 grams
        assert_eq!(proof.value, GoldGrams::from_decimal(Decimal::new(2, 0)));
    }

    #[tokio::test]
    async fn lock_verification_succeeds_for_valid_proof() {
        let adapter = MockIngressAdapter::new();
        let proof = adapter
            .lock_external_value(Decimal::new(100, 0), "USD", gold_price())
            .await
            .expect("test: lock should succeed");

        let valid = adapter
            .verify_lock(&proof)
            .await
            .expect("test: verify should succeed");
        assert!(valid, "proof should be valid");
    }

    #[tokio::test]
    async fn lock_verification_fails_when_invalidated() {
        let adapter = MockIngressAdapter::new();
        let proof = adapter
            .lock_external_value(Decimal::new(100, 0), "USD", gold_price())
            .await
            .expect("test: lock should succeed");

        adapter.lock_valid.store(false, Ordering::Relaxed);
        let valid = adapter
            .verify_lock(&proof)
            .await
            .expect("test: verify should succeed");
        assert!(!valid, "proof should be invalid after flag flip");
    }

    #[tokio::test]
    async fn release_lock_clears_the_hold() {
        let adapter = MockIngressAdapter::new();
        let proof = adapter
            .lock_external_value(Decimal::new(100, 0), "USD", gold_price())
            .await
            .expect("test: lock should succeed");

        assert!(!adapter.released.load(Ordering::Relaxed));
        adapter
            .release_lock(&proof)
            .await
            .expect("test: release should succeed");
        assert!(adapter.released.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn unsupported_denomination_returns_error() {
        let adapter = MockIngressAdapter::new();
        let result = adapter
            .lock_external_value(Decimal::new(100, 0), "JPY", gold_price())
            .await;

        assert!(result.is_err());
        let err = result.expect_err("test: should be UnsupportedDenomination");
        assert!(
            matches!(err, UpiError::UnsupportedDenomination { .. }),
            "expected UnsupportedDenomination, got: {err}",
        );
    }

    #[tokio::test]
    async fn gold_gram_conversion_is_correct() {
        let adapter = MockIngressAdapter::new();
        // $300 at $75/gram = 4 grams
        let gold = adapter
            .to_gold_grams(Decimal::new(300, 0), "USD", gold_price())
            .expect("test: conversion should succeed");
        assert_eq!(gold, GoldGrams::from_decimal(Decimal::new(4, 0)));
    }

    #[test]
    fn supported_denominations_are_non_empty() {
        let adapter = MockIngressAdapter::new();
        let denoms = adapter.supported_denominations();
        assert!(!denoms.is_empty());
        assert!(denoms.contains(&"USD".to_string()));
    }
}
