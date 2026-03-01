// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Egress adapter trait -- CAES value leaving to external systems.
//!
//! The counterpart to [`super::IngressAdapter`]. Handles the "last mile"
//! delivery of value to the recipient's external account.
//!
//! Network-as-Processor: any node running this adapter can settle
//! for any verified user with matching acceptance criteria.

use async_trait::async_trait;
use hypermesh_lib::GoldGrams;
use rust_decimal::Decimal;

use crate::types::{SettlementReceipt, UpiError};

/// Adapter for settling Caesar value into external systems.
///
/// Each implementation wraps a single external destination (one payment
/// rail, one chain, one exchange). The adapter is **stateless**.
#[async_trait]
pub trait EgressAdapter: Send + Sync {
    /// Unique identifier for this adapter.
    fn adapter_id(&self) -> &str;

    /// Supported destination denominations.
    fn supported_denominations(&self) -> Vec<String>;

    /// Available settlement capacity (how much can be settled right now).
    async fn available_capacity(&self) -> Result<GoldGrams, UpiError>;

    /// Settle value to an external destination.
    ///
    /// Returns a receipt with finality information.
    async fn settle(
        &self,
        value: GoldGrams,
        destination: &str,
        denomination: &str,
        gold_price_usd: Decimal,
    ) -> Result<SettlementReceipt, UpiError>;

    /// Current capacity ratio (0.0 = empty, 1.0 = full capacity available).
    async fn capacity_ratio(&self) -> Result<Decimal, UpiError>;
}

// ===========================================================================
// Mock adapter for testing
// ===========================================================================

/// Public mock module for SDK consumers to test their adapter integrations.
pub mod testing {
    use super::*;
    use crate::types::SettlementFinality;
    use chrono::Utc;
    use hypermesh_lib::NodeId;

    /// Configurable mock egress adapter for unit tests.
    pub struct MockEgressAdapter {
        /// Capacity in gold grams. When zero, settle returns an error.
        pub capacity: GoldGrams,
    }

    impl MockEgressAdapter {
        pub fn new(capacity: GoldGrams) -> Self {
            Self { capacity }
        }
    }

    #[async_trait]
    impl EgressAdapter for MockEgressAdapter {
        fn adapter_id(&self) -> &str {
            "mock_egress"
        }

        fn supported_denominations(&self) -> Vec<String> {
            vec!["USD".into(), "BTC".into(), "CAES".into()]
        }

        async fn available_capacity(&self) -> Result<GoldGrams, UpiError> {
            Ok(self.capacity)
        }

        async fn settle(
            &self,
            value: GoldGrams,
            destination: &str,
            denomination: &str,
            gold_price_usd: Decimal,
        ) -> Result<SettlementReceipt, UpiError> {
            if self.capacity.is_zero() {
                return Err(UpiError::InsufficientLiquidity {
                    needed: value,
                    available: self.capacity,
                });
            }

            if denomination != "USD" && denomination != "BTC" && denomination != "CAES" {
                return Err(UpiError::UnsupportedDenomination {
                    denomination: denomination.into(),
                });
            }

            let dest_amount = if denomination == "CAES" {
                value.0 // 1:1 for internal CAES denomination
            } else {
                value.0 * gold_price_usd
            };
            Ok(SettlementReceipt {
                settlement_id: "mock-settle-001".into(),
                adapter_id: self.adapter_id().into(),
                value,
                destination_denomination: denomination.into(),
                destination_amount: dest_amount,
                gold_price_at_settlement: gold_price_usd,
                settling_node: NodeId("mock-node".into()),
                settled_at: Utc::now(),
                external_reference: format!("mock-ref-{destination}"),
                finality: SettlementFinality::Attested,
            })
        }

        async fn capacity_ratio(&self) -> Result<Decimal, UpiError> {
            let max = Decimal::new(10_000, 0);
            if max.is_zero() {
                return Ok(Decimal::ZERO);
            }
            Ok(self.capacity.0 / max)
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::testing::MockEgressAdapter;
    use super::*;
    use crate::types::SettlementFinality;
    use rust_decimal::Decimal;

    fn gold_price() -> Decimal {
        Decimal::new(75, 0)
    }

    fn capacity_5000() -> GoldGrams {
        GoldGrams::from_decimal(Decimal::new(5_000, 0))
    }

    #[tokio::test]
    async fn mock_adapter_settles_correctly() {
        let adapter = MockEgressAdapter::new(capacity_5000());
        let value = GoldGrams::from_decimal(Decimal::new(10, 0)); // 10 grams

        let receipt = adapter
            .settle(value, "acct_123", "USD", gold_price())
            .await
            .expect("test: settle should succeed");

        assert_eq!(receipt.adapter_id, "mock_egress");
        assert_eq!(receipt.value, value);
        assert_eq!(receipt.destination_denomination, "USD");
        // 10 grams * $75/gram = $750
        assert_eq!(receipt.destination_amount, Decimal::new(750, 0));
    }

    #[tokio::test]
    async fn capacity_ratio_returns_valid_range() {
        let adapter = MockEgressAdapter::new(capacity_5000());
        let ratio = adapter
            .capacity_ratio()
            .await
            .expect("test: capacity_ratio should succeed");

        assert!(
            ratio >= Decimal::ZERO && ratio <= Decimal::ONE,
            "ratio {ratio} not in [0, 1]",
        );
        // 5000 / 10000 = 0.5
        assert_eq!(ratio, Decimal::new(5, 1));
    }

    #[tokio::test]
    async fn settlement_receipt_has_correct_finality() {
        let adapter = MockEgressAdapter::new(capacity_5000());
        let value = GoldGrams::from_decimal(Decimal::new(1, 0));

        let receipt = adapter
            .settle(value, "acct_456", "BTC", gold_price())
            .await
            .expect("test: settle should succeed");

        assert_eq!(receipt.finality, SettlementFinality::Attested);
    }

    #[tokio::test]
    async fn zero_capacity_returns_error() {
        let adapter = MockEgressAdapter::new(GoldGrams::zero());
        let value = GoldGrams::from_decimal(Decimal::new(10, 0));

        let result = adapter.settle(value, "acct_789", "USD", gold_price()).await;

        assert!(result.is_err());
        let err = result.expect_err("test: should be InsufficientLiquidity");
        assert!(
            matches!(err, UpiError::InsufficientLiquidity { .. }),
            "expected InsufficientLiquidity, got: {err}",
        );
    }

    #[tokio::test]
    async fn unsupported_denomination_returns_error() {
        let adapter = MockEgressAdapter::new(capacity_5000());
        let value = GoldGrams::from_decimal(Decimal::new(1, 0));

        let result = adapter.settle(value, "acct_000", "JPY", gold_price()).await;

        assert!(result.is_err());
        let err = result.expect_err("test: should be UnsupportedDenomination");
        assert!(
            matches!(err, UpiError::UnsupportedDenomination { .. }),
            "expected UnsupportedDenomination, got: {err}",
        );
    }

    #[test]
    fn supported_denominations_are_non_empty() {
        let adapter = MockEgressAdapter::new(GoldGrams::zero());
        let denoms = adapter.supported_denominations();
        assert!(!denoms.is_empty());
        assert!(denoms.contains(&"USD".to_string()));
    }

    #[tokio::test]
    async fn available_capacity_reflects_constructor() {
        let adapter = MockEgressAdapter::new(capacity_5000());
        let cap = adapter
            .available_capacity()
            .await
            .expect("test: available_capacity should succeed");
        assert_eq!(cap, capacity_5000());
    }
}
