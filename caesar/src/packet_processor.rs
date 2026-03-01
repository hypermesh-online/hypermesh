// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Packet processing -- validation, hop tracking, and batch handoff.
//!
//! Replaces the old wallet-based `transactions.rs`. Every value transfer is now
//! an ephemeral packet that decays via demurrage while in flight.

use hypermesh_lib::economic::{GoldGrams, PacketId, PacketState};
use hypermesh_lib::NodeId;
use serde::{Deserialize, Serialize};

use crate::evp::packet::PacketError;
use crate::evp::CaesPacket;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Processor-level configuration (independent of per-packet limits).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorConfig {
    /// Default hop limit applied when a packet does not specify one.
    pub default_hop_limit: u16,
    /// Whether to reject handoffs whose fee exceeds the packet's fee budget.
    pub enforce_fee_budget: bool,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            default_hop_limit: 32,
            enforce_fee_budget: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors arising from packet processing operations.
#[derive(Debug, thiserror::Error)]
pub enum ProcessorError {
    #[error("packet is in terminal state {0:?}")]
    TerminalPacket(PacketState),

    #[error("hop limit exceeded ({count}/{limit})")]
    HopLimitExceeded { count: u16, limit: u16 },

    #[error("fee {fee} exceeds budget {budget}")]
    FeeBudgetExceeded { fee: GoldGrams, budget: GoldGrams },

    #[error("packet has zero value")]
    ZeroValue,

    #[error("packet error: {0}")]
    PacketError(#[from] PacketError),
}

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Outcome of a successful packet handoff between nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffResult {
    pub packet_id: PacketId,
    pub from_node: NodeId,
    pub to_node: NodeId,
    pub fee_deducted: GoldGrams,
    pub remaining_value: GoldGrams,
    pub hop_count: u16,
}

// ---------------------------------------------------------------------------
// PacketProcessor
// ---------------------------------------------------------------------------

/// Validates packets and processes transit handoffs.
#[derive(Debug, Clone)]
pub struct PacketProcessor {
    config: ProcessorConfig,
}

impl PacketProcessor {
    /// Create a new processor with the given configuration.
    pub fn new(config: ProcessorConfig) -> Self {
        Self { config }
    }

    /// Validate that a packet is eligible for further processing.
    ///
    /// Checks:
    /// - The packet is not in a terminal state.
    /// - The hop count has not reached the hop limit.
    /// - The packet still carries non-zero value.
    pub fn validate_packet(&self, packet: &CaesPacket) -> Result<(), ProcessorError> {
        if packet.state.is_terminal() {
            return Err(ProcessorError::TerminalPacket(packet.state));
        }
        if packet.hop_count >= packet.hop_limit {
            return Err(ProcessorError::HopLimitExceeded {
                count: packet.hop_count,
                limit: packet.hop_limit,
            });
        }
        if packet.initial_value.is_zero() {
            return Err(ProcessorError::ZeroValue);
        }
        Ok(())
    }

    /// Hand a packet off to the next transit node.
    ///
    /// Increments the hop counter, deducts the transit fee from `initial_value`
    /// (the nominal value -- demurrage is separate), and transitions the packet
    /// to `InTransit` if it is still `Minted`.
    pub fn process_handoff(
        &self,
        packet: &mut CaesPacket,
        next_node: NodeId,
        fee: GoldGrams,
    ) -> Result<HandoffResult, ProcessorError> {
        self.validate_packet(packet)?;

        // Fee budget enforcement
        if self.config.enforce_fee_budget && fee.0 > packet.fee_budget.0 {
            return Err(ProcessorError::FeeBudgetExceeded {
                fee,
                budget: packet.fee_budget,
            });
        }

        let from_node = packet.sender.clone();

        // Increment hop (may fail if at limit)
        packet.increment_hop()?;

        // Record transit node in route for fee distribution
        packet.route.push(next_node.clone());

        // Deduct fee from the packet's initial_value
        packet.initial_value = GoldGrams::from_decimal(packet.initial_value.0 - fee.0);

        // Transition Minted -> InTransit on first handoff
        if packet.state == PacketState::Minted {
            packet.advance_to_transit()?;
        }

        Ok(HandoffResult {
            packet_id: packet.id,
            from_node,
            to_node: next_node,
            fee_deducted: fee,
            remaining_value: packet.initial_value,
            hop_count: packet.hop_count,
        })
    }

    /// Process a batch of handoffs, returning per-packet results.
    pub fn process_batch(
        &self,
        handoffs: Vec<(CaesPacket, NodeId, GoldGrams)>,
    ) -> Vec<Result<HandoffResult, ProcessorError>> {
        handoffs
            .into_iter()
            .map(|(mut packet, next_node, fee)| self.process_handoff(&mut packet, next_node, fee))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::economic::MarketTier;
    use rust_decimal_macros::dec;

    fn test_sender() -> NodeId {
        NodeId::from_public_key(b"node-sender")
    }

    fn test_recipient() -> NodeId {
        NodeId::from_public_key(b"node-recipient")
    }

    fn mint_packet(value: GoldGrams, fee_budget: GoldGrams) -> CaesPacket {
        let tier = MarketTier::L0;
        CaesPacket::mint(
            test_sender(),
            test_recipient(),
            value,
            GoldGrams(dec!(0.1)),
            tier,
            tier.default_demurrage_rate(),
            20,
            fee_budget,
        )
    }

    fn default_processor() -> PacketProcessor {
        PacketProcessor::new(ProcessorConfig::default())
    }

    #[test]
    fn validate_valid_packet() {
        let pkt = mint_packet(GoldGrams(dec!(100)), GoldGrams(dec!(5)));
        let proc = default_processor();
        proc.validate_packet(&pkt)
            .expect("test: valid packet should pass");
    }

    #[test]
    fn validate_terminal_packet() {
        let mut pkt = mint_packet(GoldGrams(dec!(100)), GoldGrams(dec!(5)));
        pkt.advance_to_transit().expect("test: transit");
        pkt.deliver().expect("test: deliver");
        pkt.settle().expect("test: settle");

        let proc = default_processor();
        let err = proc.validate_packet(&pkt);
        assert!(
            matches!(
                err,
                Err(ProcessorError::TerminalPacket(PacketState::Settled))
            ),
            "expected TerminalPacket, got {err:?}"
        );
    }

    #[test]
    fn validate_zero_value() {
        let pkt = mint_packet(GoldGrams::zero(), GoldGrams(dec!(5)));
        let proc = default_processor();
        let err = proc.validate_packet(&pkt);
        assert!(
            matches!(err, Err(ProcessorError::ZeroValue)),
            "expected ZeroValue, got {err:?}"
        );
    }

    #[test]
    fn handoff_deducts_fee() {
        let mut pkt = mint_packet(GoldGrams(dec!(100)), GoldGrams(dec!(10)));
        let proc = default_processor();

        let result = proc
            .process_handoff(&mut pkt, NodeId::from_public_key(b"relay-1"), GoldGrams(dec!(2)))
            .expect("test: handoff should succeed");

        assert_eq!(result.remaining_value, GoldGrams(dec!(98)));
        assert_eq!(result.fee_deducted, GoldGrams(dec!(2)));
    }

    #[test]
    fn handoff_increments_hop() {
        let mut pkt = mint_packet(GoldGrams(dec!(100)), GoldGrams(dec!(10)));
        let proc = default_processor();

        let result = proc
            .process_handoff(&mut pkt, NodeId::from_public_key(b"relay-1"), GoldGrams(dec!(1)))
            .expect("test: handoff should succeed");

        assert_eq!(result.hop_count, 1);
        assert_eq!(pkt.hop_count, 1);
    }

    #[test]
    fn handoff_fee_budget_exceeded() {
        let mut pkt = mint_packet(GoldGrams(dec!(100)), GoldGrams(dec!(5)));
        let proc = default_processor();

        let err = proc.process_handoff(&mut pkt, NodeId::from_public_key(b"relay-1"), GoldGrams(dec!(10)));
        assert!(
            matches!(err, Err(ProcessorError::FeeBudgetExceeded { .. })),
            "expected FeeBudgetExceeded, got {err:?}"
        );
    }

    #[test]
    fn handoff_fee_budget_not_enforced() {
        let mut pkt = mint_packet(GoldGrams(dec!(100)), GoldGrams(dec!(5)));
        let config = ProcessorConfig {
            enforce_fee_budget: false,
            ..Default::default()
        };
        let proc = PacketProcessor::new(config);

        let result = proc
            .process_handoff(&mut pkt, NodeId::from_public_key(b"relay-1"), GoldGrams(dec!(10)))
            .expect("test: should succeed without budget enforcement");

        assert_eq!(result.remaining_value, GoldGrams(dec!(90)));
    }

    #[test]
    fn batch_processing() {
        let proc = default_processor();
        let handoffs = vec![
            (
                mint_packet(GoldGrams(dec!(100)), GoldGrams(dec!(10))),
                NodeId::from_public_key(b"relay-1"),
                GoldGrams(dec!(1)),
            ),
            (
                mint_packet(GoldGrams(dec!(200)), GoldGrams(dec!(10))),
                NodeId::from_public_key(b"relay-2"),
                GoldGrams(dec!(2)),
            ),
            (
                mint_packet(GoldGrams(dec!(300)), GoldGrams(dec!(10))),
                NodeId::from_public_key(b"relay-3"),
                GoldGrams(dec!(3)),
            ),
        ];

        let results = proc.process_batch(handoffs);
        assert_eq!(results.len(), 3);
        for r in &results {
            assert!(r.is_ok(), "all batch items should succeed: {r:?}");
        }

        let r0 = results[0].as_ref().expect("test: result 0");
        assert_eq!(r0.remaining_value, GoldGrams(dec!(99)));
        let r1 = results[1].as_ref().expect("test: result 1");
        assert_eq!(r1.remaining_value, GoldGrams(dec!(198)));
        let r2 = results[2].as_ref().expect("test: result 2");
        assert_eq!(r2.remaining_value, GoldGrams(dec!(297)));
    }

    #[test]
    fn handoff_records_route() {
        let mut pkt = mint_packet(GoldGrams(dec!(100)), GoldGrams(dec!(10)));
        let proc = default_processor();

        proc.process_handoff(&mut pkt, NodeId::from_public_key(b"relay-1"), GoldGrams(dec!(1)))
            .expect("test: first handoff");
        proc.process_handoff(&mut pkt, NodeId::from_public_key(b"relay-2"), GoldGrams(dec!(1)))
            .expect("test: second handoff");

        assert_eq!(pkt.route.len(), 2);
        assert_eq!(pkt.route[0], NodeId::from_public_key(b"relay-1"));
        assert_eq!(pkt.route[1], NodeId::from_public_key(b"relay-2"));
    }

    #[test]
    fn batch_with_failures() {
        let proc = default_processor();

        let mut settled = mint_packet(GoldGrams(dec!(100)), GoldGrams(dec!(10)));
        settled.advance_to_transit().expect("test: transit");
        settled.deliver().expect("test: deliver");
        settled.settle().expect("test: settle");

        let handoffs = vec![
            // Valid
            (
                mint_packet(GoldGrams(dec!(50)), GoldGrams(dec!(10))),
                NodeId::from_public_key(b"relay-1"),
                GoldGrams(dec!(1)),
            ),
            // Invalid -- terminal state
            (settled, NodeId::from_public_key(b"relay-2"), GoldGrams(dec!(1))),
            // Valid
            (
                mint_packet(GoldGrams(dec!(75)), GoldGrams(dec!(10))),
                NodeId::from_public_key(b"relay-3"),
                GoldGrams(dec!(2)),
            ),
        ];

        let results = proc.process_batch(handoffs);
        assert_eq!(results.len(), 3);
        assert!(results[0].is_ok(), "first should succeed");
        assert!(results[1].is_err(), "second should fail (terminal)");
        assert!(results[2].is_ok(), "third should succeed");
    }
}
