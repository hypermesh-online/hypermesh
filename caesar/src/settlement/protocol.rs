// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Settlement Protocol -- Network-as-Processor validates and processes settlements.
//!
//! Any online node can settle packets on behalf of verified users whose
//! AcceptanceCriteria have been published to the Network chain.

use chrono::{DateTime, Utc};
use hypermesh_lib::{GoldGrams, MarketTier, NodeId, PacketId, PacketState};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::acceptance::AcceptanceCriteria;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Settlement protocol errors.
#[derive(Debug, thiserror::Error)]
pub enum SettlementError {
    #[error("packet {0} is in terminal state -- cannot settle")]
    TerminalState(String),

    #[error("packet {0} not in Delivered state -- currently {1:?}")]
    NotDelivered(String, PacketState),

    #[error("recipient criteria rejects tier {0:?}")]
    TierRejected(MarketTier),

    #[error("recipient criteria rejects adapter {0}")]
    AdapterRejected(String),

    #[error("fee {fee} exceeds recipient tolerance {max}")]
    FeeTooHigh { fee: Decimal, max: Decimal },

    #[error("settler {0} not authorized by recipient criteria")]
    UnauthorizedSettler(String),

    #[error("value below minimum settlement threshold")]
    BelowMinimum,

    #[error("packet expired -- TTL exceeded")]
    Expired,

    #[error("egress adapter failed: {reason}")]
    EgressFailed { reason: String },
}

// ---------------------------------------------------------------------------
// Request / Result types
// ---------------------------------------------------------------------------

/// A settlement request submitted by a settling node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementRequest {
    pub packet_id: PacketId,
    pub packet_state: PacketState,
    pub packet_tier: MarketTier,
    pub packet_value: GoldGrams,
    pub fee: GoldGrams,
    pub settler_node: NodeId,
    pub adapter_id: String,
    pub recipient_criteria: AcceptanceCriteria,
}

/// Result of a successful settlement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementResult {
    pub packet_id: PacketId,
    pub settled_value: GoldGrams,
    pub fee_collected: GoldGrams,
    pub settler_node: NodeId,
    pub settled_at: DateTime<Utc>,
}

/// Result of a fully executed settlement (validation + egress + fee distribution).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutedSettlement {
    pub settlement_result: SettlementResult,
    pub fee_distribution: crate::fee_distribution::FeeDistribution,
}

// ---------------------------------------------------------------------------
// Protocol
// ---------------------------------------------------------------------------

/// Settlement protocol -- validates and processes settlements.
pub struct SettlementProtocol;

impl SettlementProtocol {
    /// Validate that a settlement request satisfies all criteria.
    ///
    /// Checks: packet state, tier acceptance, adapter acceptance,
    /// fee tolerance, and settler authorization.
    pub fn validate_settlement(
        request: &SettlementRequest,
    ) -> Result<(), SettlementError> {
        // Terminal state check (Settled/Expired/Dissolved cannot be re-settled)
        if request.packet_state.is_terminal() {
            return Err(SettlementError::TerminalState(
                format!("{}", request.packet_id),
            ));
        }

        // Must be in Delivered state to settle
        if request.packet_state != PacketState::Delivered {
            return Err(SettlementError::NotDelivered(
                format!("{}", request.packet_id),
                request.packet_state,
            ));
        }

        let criteria = &request.recipient_criteria;

        // Tier acceptance
        if !criteria.accepts_tier(request.packet_tier) {
            return Err(SettlementError::TierRejected(request.packet_tier));
        }

        // Adapter acceptance
        if !criteria.accepts_adapter(&request.adapter_id) {
            return Err(SettlementError::AdapterRejected(
                request.adapter_id.clone(),
            ));
        }

        // Fee tolerance: compute fee as fraction of packet value
        let fee_fraction = if request.packet_value.is_zero() {
            Decimal::ZERO
        } else {
            request.fee.0 / request.packet_value.0
        };
        if !criteria.accepts_fee(fee_fraction) {
            return Err(SettlementError::FeeTooHigh {
                fee: fee_fraction,
                max: criteria.max_fee_tolerance,
            });
        }

        // Settler authorization
        if !criteria.is_authorized_settler(&request.settler_node) {
            return Err(SettlementError::UnauthorizedSettler(
                format!("{}", request.settler_node.0),
            ));
        }

        Ok(())
    }

    /// Process a settlement: validate then compute the result.
    ///
    /// Net settled value = packet_value - fee.
    pub fn process_settlement(
        request: SettlementRequest,
    ) -> Result<SettlementResult, SettlementError> {
        Self::validate_settlement(&request)?;

        let settled_value = request.packet_value - request.fee;

        Ok(SettlementResult {
            packet_id: request.packet_id,
            settled_value,
            fee_collected: request.fee,
            settler_node: request.settler_node,
            settled_at: Utc::now(),
        })
    }

    /// Execute a full settlement: validate -> egress -> distribute fees.
    ///
    /// If the egress adapter fails, returns `EgressFailed` (caller should
    /// transition the packet to Dispersed state for retry).
    pub async fn execute_settlement(
        request: SettlementRequest,
        egress_adapter: &dyn crate::upi::EgressAdapter,
        fee_distributor: &crate::fee_distribution::FeeDistributor,
        transit_nodes: &[(NodeId, u64)],
        gold_price_usd: rust_decimal::Decimal,
    ) -> Result<ExecutedSettlement, SettlementError> {
        Self::validate_settlement(&request)?;

        let settled_value = request.packet_value - request.fee;

        let receipt = egress_adapter
            .settle(
                settled_value,
                &format!("{}", request.settler_node.0),
                "CAES",
                gold_price_usd,
            )
            .await
            .map_err(|e| SettlementError::EgressFailed {
                reason: format!("{}", e),
            })?;

        let settlement_result = SettlementResult {
            packet_id: request.packet_id,
            settled_value,
            fee_collected: request.fee,
            settler_node: request.settler_node.clone(),
            settled_at: receipt.settled_at,
        };

        let fee_distribution = if request.fee.is_zero() {
            crate::fee_distribution::FeeDistribution {
                total_fee: request.fee,
                egress_payment: crate::fee_distribution::NodePayment {
                    node_id: request.settler_node.clone(),
                    amount: GoldGrams::zero(),
                },
                transit_payments: Vec::new(),
            }
        } else {
            fee_distributor
                .distribute_fee(
                    request.fee,
                    request.settler_node.clone(),
                    transit_nodes,
                )
                .map_err(|e| SettlementError::EgressFailed {
                    reason: format!("fee distribution: {}", e),
                })?
        };

        Ok(ExecutedSettlement {
            settlement_result,
            fee_distribution,
        })
    }

    /// Whether a packet can be auto-settled (validates + checks threshold).
    pub fn can_auto_settle(request: &SettlementRequest) -> bool {
        if Self::validate_settlement(request).is_err() {
            return false;
        }
        request
            .recipient_criteria
            .can_auto_settle(request.packet_value)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::upi::egress::testing::MockEgressAdapter;
    use hypermesh_lib::NodeId;
    use rust_decimal_macros::dec;

    fn test_criteria() -> AcceptanceCriteria {
        AcceptanceCriteria::new(NodeId::from("recipient-node"))
    }

    fn test_request() -> SettlementRequest {
        SettlementRequest {
            packet_id: PacketId::zero(),
            packet_state: PacketState::Delivered,
            packet_tier: MarketTier::L0,
            packet_value: GoldGrams::from_decimal(dec!(5)),
            fee: GoldGrams::from_decimal(dec!(0.05)), // 1% fee on 5g
            settler_node: NodeId::from("settler-node"),
            adapter_id: "stripe_us".to_string(),
            recipient_criteria: test_criteria(),
        }
    }

    // -- Happy path ---------------------------------------------------------

    #[test]
    fn successful_settlement() {
        let request = test_request();
        let result = SettlementProtocol::process_settlement(request)
            .expect("test: settlement should succeed");
        assert_eq!(result.settled_value.0, dec!(4.95)); // 5 - 0.05
        assert_eq!(result.fee_collected.0, dec!(0.05));
    }

    // -- State validation ---------------------------------------------------

    #[test]
    fn reject_terminal_settled_state() {
        let mut request = test_request();
        request.packet_state = PacketState::Settled;
        let err = SettlementProtocol::validate_settlement(&request)
            .expect_err("test: terminal state should fail");
        assert!(
            matches!(err, SettlementError::TerminalState(_)),
            "expected TerminalState, got: {err}"
        );
    }

    #[test]
    fn reject_expired_state() {
        // Since Sprint 18, Expired is non-terminal (intermediate → Refunded).
        // Settlement validation rejects it as NotDelivered, not TerminalState.
        let mut request = test_request();
        request.packet_state = PacketState::Expired;
        let err = SettlementProtocol::validate_settlement(&request)
            .expect_err("test: expired should fail");
        assert!(
            matches!(err, SettlementError::NotDelivered(_, PacketState::Expired)),
            "expected NotDelivered(Expired), got: {err}"
        );
    }

    #[test]
    fn reject_terminal_dissolved_state() {
        let mut request = test_request();
        request.packet_state = PacketState::Dissolved;
        let err = SettlementProtocol::validate_settlement(&request)
            .expect_err("test: dissolved should fail");
        assert!(matches!(err, SettlementError::TerminalState(_)));
    }

    #[test]
    fn reject_non_delivered_state() {
        let mut request = test_request();
        request.packet_state = PacketState::InTransit;
        let err = SettlementProtocol::validate_settlement(&request)
            .expect_err("test: in_transit should fail");
        assert!(
            matches!(err, SettlementError::NotDelivered(_, PacketState::InTransit)),
            "expected NotDelivered(InTransit), got: {err}"
        );
    }

    #[test]
    fn reject_minted_state() {
        let mut request = test_request();
        request.packet_state = PacketState::Minted;
        let err = SettlementProtocol::validate_settlement(&request)
            .expect_err("test: minted should fail");
        assert!(matches!(err, SettlementError::NotDelivered(_, PacketState::Minted)));
    }

    // -- Tier validation ----------------------------------------------------

    #[test]
    fn reject_unaccepted_tier() {
        let mut request = test_request();
        request.recipient_criteria.accepted_tiers = vec![MarketTier::L0, MarketTier::L1];
        request.packet_tier = MarketTier::L3;
        let err = SettlementProtocol::validate_settlement(&request)
            .expect_err("test: L3 tier should be rejected");
        assert!(
            matches!(err, SettlementError::TierRejected(MarketTier::L3)),
            "expected TierRejected(L3), got: {err}"
        );
    }

    // -- Adapter validation -------------------------------------------------

    #[test]
    fn reject_unaccepted_adapter() {
        let mut request = test_request();
        request.recipient_criteria.accepted_adapters =
            vec!["uniswap_v3".to_string()];
        request.adapter_id = "stripe_us".to_string();
        let err = SettlementProtocol::validate_settlement(&request)
            .expect_err("test: stripe_us should be rejected");
        assert!(
            matches!(err, SettlementError::AdapterRejected(ref id) if id == "stripe_us"),
            "expected AdapterRejected(stripe_us), got: {err}"
        );
    }

    // -- Fee validation -----------------------------------------------------

    #[test]
    fn reject_excessive_fee() {
        let mut request = test_request();
        // 3% fee on 100g = 3g fee
        request.packet_value = GoldGrams::from_decimal(dec!(100));
        request.fee = GoldGrams::from_decimal(dec!(3));
        let err = SettlementProtocol::validate_settlement(&request)
            .expect_err("test: 3% fee should exceed 2% tolerance");
        assert!(
            matches!(err, SettlementError::FeeTooHigh { .. }),
            "expected FeeTooHigh, got: {err}"
        );
    }

    #[test]
    fn accept_fee_at_tolerance() {
        let mut request = test_request();
        // Exactly 2% fee on 100g = 2g fee
        request.packet_value = GoldGrams::from_decimal(dec!(100));
        request.fee = GoldGrams::from_decimal(dec!(2));
        SettlementProtocol::validate_settlement(&request)
            .expect("test: 2% fee should be accepted at 2% tolerance");
    }

    // -- Settler authorization ----------------------------------------------

    #[test]
    fn reject_unauthorized_settler() {
        let mut request = test_request();
        request.recipient_criteria.delegates =
            vec![NodeId::from("trusted-only")];
        request.settler_node = NodeId::from("untrusted-settler");
        let err = SettlementProtocol::validate_settlement(&request)
            .expect_err("test: unauthorized settler should fail");
        assert!(
            matches!(err, SettlementError::UnauthorizedSettler(_)),
            "expected UnauthorizedSettler, got: {err}"
        );
    }

    #[test]
    fn accept_authorized_delegate() {
        let mut request = test_request();
        request.recipient_criteria.delegates =
            vec![NodeId::from("settler-node")];
        request.settler_node = NodeId::from("settler-node");
        SettlementProtocol::validate_settlement(&request)
            .expect("test: authorized delegate should pass");
    }

    // -- Auto-settle --------------------------------------------------------

    #[test]
    fn auto_settle_below_threshold() {
        let request = test_request();
        // Default threshold is 10g, packet value is 5g
        assert!(
            SettlementProtocol::can_auto_settle(&request),
            "5g should auto-settle (threshold 10g)"
        );
    }

    #[test]
    fn auto_settle_above_threshold_returns_false() {
        let mut request = test_request();
        request.packet_value = GoldGrams::from_decimal(dec!(50));
        request.fee = GoldGrams::from_decimal(dec!(0.5)); // keep fee at 1%
        assert!(
            !SettlementProtocol::can_auto_settle(&request),
            "50g should not auto-settle (threshold 10g)"
        );
    }

    #[test]
    fn auto_settle_invalid_request_returns_false() {
        let mut request = test_request();
        request.packet_state = PacketState::Minted; // invalid for settlement
        assert!(
            !SettlementProtocol::can_auto_settle(&request),
            "invalid request should not auto-settle"
        );
    }

    // -- Net value calculation ----------------------------------------------

    #[test]
    fn net_value_is_packet_minus_fee() {
        let mut request = test_request();
        request.packet_value = GoldGrams::from_decimal(dec!(100));
        request.fee = GoldGrams::from_decimal(dec!(1)); // 1%
        let result = SettlementProtocol::process_settlement(request)
            .expect("test: settlement should succeed");
        assert_eq!(result.settled_value.0, dec!(99));
        assert_eq!(result.fee_collected.0, dec!(1));
    }

    #[test]
    fn zero_fee_settlement() {
        let mut request = test_request();
        request.fee = GoldGrams::zero();
        let result = SettlementProtocol::process_settlement(request)
            .expect("test: zero fee settlement should succeed");
        assert_eq!(result.settled_value.0, dec!(5));
        assert_eq!(result.fee_collected.0, dec!(0));
    }

    // -- execute_settlement tests -------------------------------------------

    fn mock_adapter() -> MockEgressAdapter {
        MockEgressAdapter::new(GoldGrams::from_decimal(dec!(10000)))
    }

    fn fee_distributor() -> crate::fee_distribution::FeeDistributor {
        crate::fee_distribution::FeeDistributor::default()
    }

    #[tokio::test]
    async fn execute_settlement_success() {
        let request = test_request();
        let adapter = mock_adapter();
        let distributor = fee_distributor();

        let result = SettlementProtocol::execute_settlement(
            request,
            &adapter,
            &distributor,
            &[],
            dec!(75),
        )
        .await
        .expect("test: execute_settlement should succeed");

        assert_eq!(result.settlement_result.settled_value.0, dec!(4.95));
        assert_eq!(result.settlement_result.fee_collected.0, dec!(0.05));
        assert_eq!(
            result.fee_distribution.egress_payment.node_id,
            NodeId::from("settler-node"),
        );
    }

    #[tokio::test]
    async fn execute_settlement_egress_failure() {
        let request = test_request();
        let adapter = MockEgressAdapter::new(GoldGrams::zero());
        let distributor = fee_distributor();

        let err = SettlementProtocol::execute_settlement(
            request,
            &adapter,
            &distributor,
            &[],
            dec!(75),
        )
        .await
        .expect_err("test: zero-capacity adapter should fail");

        assert!(
            matches!(err, SettlementError::EgressFailed { .. }),
            "expected EgressFailed, got: {err}",
        );
    }

    #[tokio::test]
    async fn execute_settlement_validation_failure() {
        let mut request = test_request();
        request.packet_state = PacketState::Minted;
        let adapter = mock_adapter();
        let distributor = fee_distributor();

        let err = SettlementProtocol::execute_settlement(
            request,
            &adapter,
            &distributor,
            &[],
            dec!(75),
        )
        .await
        .expect_err("test: Minted state should be rejected");

        assert!(
            matches!(err, SettlementError::NotDelivered(_, PacketState::Minted)),
            "expected NotDelivered(Minted), got: {err}",
        );
    }

    #[tokio::test]
    async fn execute_settlement_zero_fee() {
        let mut request = test_request();
        request.fee = GoldGrams::zero();
        let adapter = mock_adapter();
        let distributor = fee_distributor();

        let result = SettlementProtocol::execute_settlement(
            request,
            &adapter,
            &distributor,
            &[],
            dec!(75),
        )
        .await
        .expect("test: zero-fee settlement should succeed");

        assert_eq!(result.settlement_result.settled_value.0, dec!(5));
        assert_eq!(result.settlement_result.fee_collected.0, dec!(0));
        assert!(result.fee_distribution.transit_payments.is_empty());
        assert!(result.fee_distribution.egress_payment.amount.is_zero());
    }

    #[tokio::test]
    async fn execute_settlement_with_transit_nodes() {
        let mut request = test_request();
        request.packet_value = GoldGrams::from_decimal(dec!(100));
        request.fee = GoldGrams::from_decimal(dec!(1)); // 1% fee
        let adapter = mock_adapter();
        let distributor = fee_distributor();
        let transit = vec![
            (NodeId::from("relay-a"), 500_u64),
            (NodeId::from("relay-b"), 500_u64),
        ];

        let result = SettlementProtocol::execute_settlement(
            request,
            &adapter,
            &distributor,
            &transit,
            dec!(75),
        )
        .await
        .expect("test: transit-node settlement should succeed");

        assert_eq!(result.fee_distribution.transit_payments.len(), 2);
        assert_eq!(
            result.fee_distribution.transit_payments[0].node_id,
            NodeId::from("relay-a"),
        );
        assert_eq!(
            result.fee_distribution.transit_payments[1].node_id,
            NodeId::from("relay-b"),
        );
    }

    #[tokio::test]
    async fn execute_settlement_fee_distribution_matches() {
        let mut request = test_request();
        request.packet_value = GoldGrams::from_decimal(dec!(100));
        request.fee = GoldGrams::from_decimal(dec!(2)); // 2% fee = 2g (at tolerance)
        let adapter = mock_adapter();
        let distributor = fee_distributor();
        let transit = vec![
            (NodeId::from("relay-1"), 500_u64),
            (NodeId::from("relay-2"), 500_u64),
        ];

        let result = SettlementProtocol::execute_settlement(
            request,
            &adapter,
            &distributor,
            &transit,
            dec!(75),
        )
        .await
        .expect("test: fee distribution should succeed");

        // Default split: 80% egress, 20% transit
        // Egress gets 80% of 2g = 1.6g
        assert_eq!(
            result.fee_distribution.egress_payment.amount.0,
            dec!(1.6),
            "egress should get 80% of fee",
        );
        // Transit pool = 0.4g, split 50/50 = 0.2g each
        assert_eq!(
            result.fee_distribution.transit_payments[0].amount.0,
            dec!(0.2),
            "relay-1 should get 0.2g",
        );
        assert_eq!(
            result.fee_distribution.transit_payments[1].amount.0,
            dec!(0.2),
            "relay-2 should get 0.2g",
        );
    }
}
