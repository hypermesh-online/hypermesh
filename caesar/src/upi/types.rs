// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! UPI shared types: lock proofs, settlement receipts, liquidity pressure,
//! and error definitions.

use chrono::{DateTime, Utc};
use hypermesh_lib::{GoldGrams, NodeId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Ingress
// ---------------------------------------------------------------------------

/// Proof that external value has been locked for ingress.
///
/// Returned by [`super::IngressAdapter::lock_external_value`] after the
/// external system confirms a hold on the funds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngressLockProof {
    /// Unique lock identifier from the external system.
    pub lock_id: String,
    /// External system identifier (e.g. "stripe", "uniswap_v3", "btc_mainnet").
    pub adapter_id: String,
    /// Value locked in gold-gram equivalent.
    pub value: GoldGrams,
    /// Original denomination (e.g. "USD", "ETH", "BTC").
    pub source_denomination: String,
    /// Original amount in source denomination.
    pub source_amount: Decimal,
    /// Gold price used for conversion (USD per gram).
    pub gold_price_at_lock: Decimal,
    /// Node that performed the lock.
    pub locking_node: NodeId,
    /// Timestamp of lock confirmation.
    pub locked_at: DateTime<Utc>,
    /// Lock expiry (external system's hold period).
    pub expires_at: DateTime<Utc>,
    /// External transaction reference (tx hash, payment ID, etc.).
    pub external_reference: String,
}

// ---------------------------------------------------------------------------
// Egress / Settlement
// ---------------------------------------------------------------------------

/// Receipt confirming external value delivery (egress settlement).
///
/// Returned by [`super::EgressAdapter::settle`] after the external system
/// confirms delivery of the value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementReceipt {
    /// Unique settlement identifier.
    pub settlement_id: String,
    /// External system identifier.
    pub adapter_id: String,
    /// Value settled in gold-gram equivalent.
    pub value: GoldGrams,
    /// Destination denomination.
    pub destination_denomination: String,
    /// Amount delivered in destination denomination.
    pub destination_amount: Decimal,
    /// Gold price used for conversion.
    pub gold_price_at_settlement: Decimal,
    /// Settling node.
    pub settling_node: NodeId,
    /// Settlement timestamp.
    pub settled_at: DateTime<Utc>,
    /// External transaction reference.
    pub external_reference: String,
    /// Settlement finality status.
    pub finality: SettlementFinality,
}

/// Settlement finality -- how certain we are the external value was delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementFinality {
    /// Cryptographic proof (on-chain tx confirmed, SPV proof).
    Trustless,
    /// Adapter attestation + reputation + hold period.
    Attested,
    /// Pending confirmation from external system.
    Pending,
}

// ---------------------------------------------------------------------------
// Liquidity
// ---------------------------------------------------------------------------

/// Liquidity pressure snapshot from an adapter -- feeds into Governor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityPressure {
    /// Adapter identifier.
    pub adapter_id: String,
    /// Available capacity for ingress (gold grams equivalent).
    pub ingress_capacity: GoldGrams,
    /// Available capacity for egress (gold grams equivalent).
    pub egress_capacity: GoldGrams,
    /// Utilization ratio (0.0 - 1.0).
    pub utilization: Decimal,
    /// Estimated processing time in seconds.
    pub estimated_latency_secs: u64,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors specific to UPI operations.
#[derive(Debug, thiserror::Error)]
pub enum UpiError {
    #[error("insufficient external liquidity: need {needed}, available {available}")]
    InsufficientLiquidity {
        needed: GoldGrams,
        available: GoldGrams,
    },

    #[error("lock expired at {expired_at}")]
    LockExpired { expired_at: DateTime<Utc> },

    #[error("adapter unavailable: {adapter_id} -- {reason}")]
    AdapterUnavailable {
        adapter_id: String,
        reason: String,
    },

    #[error("settlement failed: {reason}")]
    SettlementFailed { reason: String },

    #[error("verification failed: {reason}")]
    VerificationFailed { reason: String },

    #[error("unsupported denomination: {denomination}")]
    UnsupportedDenomination { denomination: String },
}
