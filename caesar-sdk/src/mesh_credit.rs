// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! MeshCredit adapter -- internal BlockMatrix ledger for the mesh economy.
//!
//! The primary internal economy adapter (whitepaper section 13.1). 1:1 gold-gram conversion
//! for "CAES" denomination. No external dependencies.

use async_trait::async_trait;
use chrono::Utc;
use hypermesh_lib::{GoldGrams, NodeId};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::egress::EgressAdapter;
use crate::ingress::IngressAdapter;
use crate::types::*;

// ---------------------------------------------------------------------------
// MeshCreditAdapter
// ---------------------------------------------------------------------------

/// Internal BlockMatrix ledger adapter for the mesh economy.
///
/// Implements both [`IngressAdapter`] and [`EgressAdapter`] with a simple
/// in-memory balance map. 1:1 gold-gram conversion for "CAES" denomination.
#[derive(Debug, Clone)]
pub struct MeshCreditAdapter {
    node_id: NodeId,
    balances: Arc<RwLock<HashMap<String, GoldGrams>>>,
}

impl MeshCreditAdapter {
    /// Create a new adapter with empty balances.
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            balances: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Credit an account with the given amount.
    pub async fn credit(&self, account: &str, amount: GoldGrams) {
        let mut balances = self.balances.write().await;
        let entry = balances
            .entry(account.to_string())
            .or_insert(GoldGrams::zero());
        *entry = *entry + amount;
    }

    /// Debit an account by the given amount.
    ///
    /// Returns an error if the account has insufficient balance.
    pub async fn debit(&self, account: &str, amount: GoldGrams) -> Result<(), UpiError> {
        let mut balances = self.balances.write().await;
        let current = balances.get(account).copied().unwrap_or(GoldGrams::zero());
        if current < amount {
            return Err(UpiError::InsufficientLiquidity {
                needed: amount,
                available: current,
            });
        }
        balances.insert(account.to_string(), current - amount);
        Ok(())
    }

    /// Get the balance for an account (zero if not found).
    pub async fn balance(&self, account: &str) -> GoldGrams {
        let balances = self.balances.read().await;
        balances.get(account).copied().unwrap_or(GoldGrams::zero())
    }

    /// The adapter identifier (shared by both trait impls).
    const ADAPTER_ID: &'static str = "mesh_credit";

    /// Validate that denomination is "CAES".
    fn check_denomination(denomination: &str) -> Result<(), UpiError> {
        if denomination != "CAES" {
            return Err(UpiError::UnsupportedDenomination {
                denomination: denomination.into(),
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// EgressAdapter
// ---------------------------------------------------------------------------

#[async_trait]
impl EgressAdapter for MeshCreditAdapter {
    fn adapter_id(&self) -> &str {
        "mesh_credit"
    }

    fn supported_denominations(&self) -> Vec<String> {
        vec!["CAES".into()]
    }

    async fn available_capacity(&self) -> Result<GoldGrams, UpiError> {
        Ok(GoldGrams::from_decimal(Decimal::MAX))
    }

    async fn settle(
        &self,
        value: GoldGrams,
        destination: &str,
        denomination: &str,
        _gold_price_usd: Decimal,
    ) -> Result<SettlementReceipt, UpiError> {
        Self::check_denomination(denomination)?;

        self.credit(destination, value).await;

        let now = Utc::now();
        let settlement_id = format!("mesh-{}", now.timestamp_nanos_opt().unwrap_or(0),);

        Ok(SettlementReceipt {
            settlement_id,
            adapter_id: Self::ADAPTER_ID.into(),
            value,
            destination_denomination: denomination.into(),
            destination_amount: value.0, // 1:1 for CAES
            gold_price_at_settlement: _gold_price_usd,
            settling_node: self.node_id.clone(),
            settled_at: now,
            external_reference: format!("mesh-credit-{destination}"),
            finality: SettlementFinality::Trustless,
        })
    }

    async fn capacity_ratio(&self) -> Result<Decimal, UpiError> {
        Ok(Decimal::ONE)
    }
}

// ---------------------------------------------------------------------------
// IngressAdapter
// ---------------------------------------------------------------------------

#[async_trait]
impl IngressAdapter for MeshCreditAdapter {
    fn adapter_id(&self) -> &str {
        "mesh_credit"
    }

    fn supported_denominations(&self) -> Vec<String> {
        vec!["CAES".into()]
    }

    async fn lock_external_value(
        &self,
        amount: Decimal,
        denomination: &str,
        _gold_price_usd: Decimal,
    ) -> Result<IngressLockProof, UpiError> {
        Self::check_denomination(denomination)?;

        let value = GoldGrams::from_decimal(amount); // 1:1 for CAES
                                                     // Debit the node's own account as the "source"
        let source = self.node_id.0.to_string();
        self.debit(&source, value).await?;

        let now = Utc::now();
        let lock_id = format!("mesh-lock-{}", now.timestamp_nanos_opt().unwrap_or(0),);

        Ok(IngressLockProof {
            lock_id,
            adapter_id: Self::ADAPTER_ID.into(),
            value,
            source_denomination: denomination.into(),
            source_amount: amount, // 1:1 for CAES
            gold_price_at_lock: _gold_price_usd,
            locking_node: self.node_id.clone(),
            locked_at: now,
            expires_at: now + chrono::Duration::hours(24),
            external_reference: format!("mesh-lock-{source}"),
        })
    }

    async fn verify_lock(&self, _proof: &IngressLockProof) -> Result<bool, UpiError> {
        // Internal ledger -- locks are instant and always valid
        Ok(true)
    }

    async fn release_lock(&self, _proof: &IngressLockProof) -> Result<(), UpiError> {
        // No-op for internal ledger
        Ok(())
    }

    async fn liquidity_pressure(&self) -> Result<LiquidityPressure, UpiError> {
        Ok(LiquidityPressure {
            adapter_id: Self::ADAPTER_ID.into(),
            ingress_capacity: GoldGrams::from_decimal(Decimal::MAX),
            egress_capacity: GoldGrams::from_decimal(Decimal::MAX),
            utilization: Decimal::ZERO,
            estimated_latency_secs: 0,
        })
    }

    fn to_gold_grams(
        &self,
        amount: Decimal,
        denomination: &str,
        _gold_price_usd: Decimal,
    ) -> Result<GoldGrams, UpiError> {
        Self::check_denomination(denomination)?;
        Ok(GoldGrams::from_decimal(amount)) // 1:1 for CAES
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn adapter() -> MeshCreditAdapter {
        MeshCreditAdapter::new(NodeId::from("test-node"))
    }

    #[tokio::test]
    async fn mesh_credit_settle() {
        let a = adapter();
        let value = GoldGrams::from_decimal(dec!(10));
        let receipt = a
            .settle(value, "alice", "CAES", dec!(75))
            .await
            .expect("test: settle should succeed");

        assert_eq!(receipt.finality, SettlementFinality::Trustless);
        assert_eq!(receipt.destination_amount, dec!(10)); // 1:1
        assert_eq!(a.balance("alice").await.0, dec!(10));
    }

    #[tokio::test]
    async fn mesh_credit_unsupported_denomination() {
        let a = adapter();
        let value = GoldGrams::from_decimal(dec!(10));
        let err = a
            .settle(value, "alice", "USD", dec!(75))
            .await
            .expect_err("test: USD should be unsupported");

        assert!(
            matches!(err, UpiError::UnsupportedDenomination { .. }),
            "expected UnsupportedDenomination, got: {err}",
        );
    }

    #[tokio::test]
    async fn mesh_credit_lock_and_verify() {
        let a = adapter();
        // Fund the node's account first
        a.credit("test-node", GoldGrams::from_decimal(dec!(100)))
            .await;

        let proof = a
            .lock_external_value(dec!(50), "CAES", dec!(75))
            .await
            .expect("test: lock should succeed");

        assert_eq!(a.balance("test-node").await.0, dec!(50));
        assert_eq!(proof.value.0, dec!(50));

        let valid = a
            .verify_lock(&proof)
            .await
            .expect("test: verify should succeed");
        assert!(valid, "lock should be valid");
    }

    #[tokio::test]
    async fn mesh_credit_lock_insufficient() {
        let a = adapter();
        // No funds -- debit should fail
        let err = a
            .lock_external_value(dec!(50), "CAES", dec!(75))
            .await
            .expect_err("test: insufficient funds should fail");

        assert!(
            matches!(err, UpiError::InsufficientLiquidity { .. }),
            "expected InsufficientLiquidity, got: {err}",
        );
    }

    #[tokio::test]
    async fn mesh_credit_capacity_unlimited() {
        let a = adapter();
        let cap = a
            .available_capacity()
            .await
            .expect("test: capacity should succeed");
        assert_eq!(cap.0, Decimal::MAX);

        let ratio = a
            .capacity_ratio()
            .await
            .expect("test: ratio should succeed");
        assert_eq!(ratio, Decimal::ONE);
    }

    #[tokio::test]
    async fn mesh_credit_multiple_operations() {
        let a = adapter();

        // Credit alice with 100g
        a.credit("alice", GoldGrams::from_decimal(dec!(100))).await;
        assert_eq!(a.balance("alice").await.0, dec!(100));

        // Settle 30g to bob
        a.settle(GoldGrams::from_decimal(dec!(30)), "bob", "CAES", dec!(75))
            .await
            .expect("test: settle to bob should succeed");

        assert_eq!(a.balance("alice").await.0, dec!(100)); // alice unchanged
        assert_eq!(a.balance("bob").await.0, dec!(30));

        // Debit alice 40g
        a.debit("alice", GoldGrams::from_decimal(dec!(40)))
            .await
            .expect("test: debit alice should succeed");
        assert_eq!(a.balance("alice").await.0, dec!(60));
    }
}
