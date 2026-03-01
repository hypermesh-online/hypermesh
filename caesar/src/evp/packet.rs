// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CaesPacket -- the ephemeral value packet state machine.
//!
//! A packet is born at ingress (Minted), traverses the mesh (InTransit),
//! and dies at egress (Settled/Refunded/Dissolved). Value only exists in-flight.

use chrono::{DateTime, Utc};
use hypermesh_lib::economic::{DemurrageRate, GoldGrams, MarketTier, PacketId, PacketState};
use hypermesh_lib::NodeId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::demurrage::DemurrageEngine;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors arising from invalid packet operations.
#[derive(Debug, thiserror::Error)]
pub enum PacketError {
    #[error("invalid state transition from {from:?} to {to:?}")]
    InvalidTransition { from: PacketState, to: PacketState },

    #[error("packet has expired (TTL exceeded)")]
    Expired,

    #[error("packet is in terminal state {0:?}")]
    TerminalState(PacketState),

    #[error("hop limit exceeded ({count}/{limit})")]
    HopLimitExceeded { count: u16, limit: u16 },
}

// ---------------------------------------------------------------------------
// CaesPacket
// ---------------------------------------------------------------------------

/// A Caesar Ephemeral Value Packet.
///
/// Value exists only in-flight -- born at ingress, dies at egress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaesPacket {
    /// Unique packet identifier (SHA-256 of creation params).
    pub id: PacketId,
    /// Current lifecycle state.
    pub state: PacketState,
    /// Market tier (determines demurrage rate).
    pub tier: MarketTier,
    /// Initial value at minting (gold grams).
    pub initial_value: GoldGrams,
    /// Demurrage rate for this packet.
    pub demurrage_rate: DemurrageRate,
    /// Sender node.
    pub sender: NodeId,
    /// Recipient node.
    pub recipient: NodeId,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last state transition timestamp.
    pub last_transition: DateTime<Utc>,
    /// Transaction fee (deducted at mint).
    pub fee: GoldGrams,
    /// Route through mesh (transit node IDs).
    pub route: Vec<NodeId>,
    /// Number of hold retries.
    pub hold_retries: u32,
    /// Current hop count (incremented on each transit handoff).
    pub hop_count: u16,
    /// Maximum allowed hops (set at mint time).
    pub hop_limit: u16,
    /// Maximum authorized fee (sender's inviolable guarantee).
    pub fee_budget: GoldGrams,
}

impl CaesPacket {
    /// Mint a new value packet.
    ///
    /// The packet starts in `Minted` state. The PacketId is derived from
    /// a SHA-256 hash of (sender + recipient + value + timestamp + nonce).
    pub fn mint(
        sender: NodeId,
        recipient: NodeId,
        value: GoldGrams,
        fee: GoldGrams,
        tier: MarketTier,
        demurrage_rate: DemurrageRate,
        hop_limit: u16,
        fee_budget: GoldGrams,
    ) -> Self {
        let now = Utc::now();
        let nonce = Uuid::new_v4();
        let id = generate_packet_id(&sender, &recipient, &value, now, nonce);

        Self {
            id,
            state: PacketState::Minted,
            tier,
            initial_value: value,
            demurrage_rate,
            sender,
            recipient,
            created_at: now,
            last_transition: now,
            fee,
            route: Vec::new(),
            hold_retries: 0,
            hop_count: 0,
            hop_limit,
            fee_budget,
        }
    }

    // -- State transitions --------------------------------------------------

    /// Minted -> InTransit
    pub fn advance_to_transit(&mut self) -> Result<(), PacketError> {
        self.require_non_terminal()?;
        self.require_state(PacketState::Minted, PacketState::InTransit)?;
        self.transition_to(PacketState::InTransit);
        Ok(())
    }

    /// InTransit -> Delivered
    pub fn deliver(&mut self) -> Result<(), PacketError> {
        self.require_non_terminal()?;
        self.require_state(PacketState::InTransit, PacketState::Delivered)?;
        self.transition_to(PacketState::Delivered);
        Ok(())
    }

    /// Delivered | Settling -> Settled (TERMINAL)
    pub fn settle(&mut self) -> Result<(), PacketError> {
        self.require_non_terminal()?;
        let valid = self.state == PacketState::Delivered || self.state == PacketState::Settling;
        if !valid {
            return Err(PacketError::InvalidTransition {
                from: self.state,
                to: PacketState::Settled,
            });
        }
        self.transition_to(PacketState::Settled);
        Ok(())
    }

    /// Delivered -> Settling
    pub fn begin_settling(&mut self) -> Result<(), PacketError> {
        self.require_non_terminal()?;
        self.require_state(PacketState::Delivered, PacketState::Settling)?;
        self.transition_to(PacketState::Settling);
        Ok(())
    }

    /// Settling -> Dispersed
    pub fn disperse(&mut self) -> Result<(), PacketError> {
        self.require_non_terminal()?;
        self.require_state(PacketState::Settling, PacketState::Dispersed)?;
        self.transition_to(PacketState::Dispersed);
        Ok(())
    }

    /// Dispersed -> Settling (retry settlement after dispersal)
    pub fn retry_settlement(&mut self) -> Result<(), PacketError> {
        self.require_non_terminal()?;
        self.require_state(PacketState::Dispersed, PacketState::Settling)?;
        self.transition_to(PacketState::Settling);
        Ok(())
    }

    /// Expired -> Refunded (TERMINAL)
    pub fn refund(&mut self) -> Result<(), PacketError> {
        self.require_non_terminal()?;
        self.require_state(PacketState::Expired, PacketState::Refunded)?;
        self.transition_to(PacketState::Refunded);
        Ok(())
    }

    /// InTransit | Delivered | Stalled -> Held
    pub fn hold(&mut self) -> Result<(), PacketError> {
        self.require_non_terminal()?;
        let valid = self.state == PacketState::InTransit
            || self.state == PacketState::Delivered
            || self.state == PacketState::Stalled;
        if !valid {
            return Err(PacketError::InvalidTransition {
                from: self.state,
                to: PacketState::Held,
            });
        }
        self.transition_to(PacketState::Held);
        Ok(())
    }

    /// InTransit | Held -> Stalled
    pub fn stall(&mut self) -> Result<(), PacketError> {
        self.require_non_terminal()?;
        let valid = self.state == PacketState::InTransit || self.state == PacketState::Held;
        if !valid {
            return Err(PacketError::InvalidTransition {
                from: self.state,
                to: PacketState::Stalled,
            });
        }
        self.transition_to(PacketState::Stalled);
        Ok(())
    }

    /// Held -> InTransit (retry delivery)
    pub fn retry_from_hold(&mut self) -> Result<(), PacketError> {
        self.require_non_terminal()?;
        self.require_state(PacketState::Held, PacketState::InTransit)?;
        self.hold_retries += 1;
        self.transition_to(PacketState::InTransit);
        Ok(())
    }

    /// Any non-terminal -> Expired (non-terminal, leads to Refunded)
    pub fn expire(&mut self) -> Result<(), PacketError> {
        self.require_non_terminal()?;
        self.transition_to(PacketState::Expired);
        Ok(())
    }

    /// Held | Stalled -> Dissolved (TERMINAL)
    pub fn dissolve(&mut self) -> Result<(), PacketError> {
        self.require_non_terminal()?;
        let valid = self.state == PacketState::Held || self.state == PacketState::Stalled;
        if !valid {
            return Err(PacketError::InvalidTransition {
                from: self.state,
                to: PacketState::Dissolved,
            });
        }
        self.transition_to(PacketState::Dissolved);
        Ok(())
    }

    // -- Hop tracking -------------------------------------------------------

    /// Increment the hop count. Returns error if hop limit would be exceeded.
    pub fn increment_hop(&mut self) -> Result<(), PacketError> {
        if self.hop_count >= self.hop_limit {
            return Err(PacketError::HopLimitExceeded {
                count: self.hop_count,
                limit: self.hop_limit,
            });
        }
        self.hop_count += 1;
        Ok(())
    }

    // -- Value queries ------------------------------------------------------

    /// Current value after demurrage from creation time to now.
    pub fn current_value(&self) -> GoldGrams {
        let elapsed = Utc::now()
            .signed_duration_since(self.created_at)
            .num_seconds()
            .max(0) as u64;
        DemurrageEngine::calculate_remaining(self.initial_value, elapsed, &self.demurrage_rate)
    }

    /// Current value using a caller-supplied timestamp (useful for testing).
    pub fn value_at(&self, at: DateTime<Utc>) -> GoldGrams {
        let elapsed = at
            .signed_duration_since(self.created_at)
            .num_seconds()
            .max(0) as u64;
        DemurrageEngine::calculate_remaining(self.initial_value, elapsed, &self.demurrage_rate)
    }

    /// Whether this packet has exceeded its TTL.
    pub fn is_expired(&self) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.created_at)
            .num_seconds()
            .max(0) as u64;
        DemurrageEngine::is_expired(elapsed, &self.demurrage_rate)
    }

    // -- Helpers ------------------------------------------------------------

    fn require_non_terminal(&self) -> Result<(), PacketError> {
        if self.state.is_terminal() {
            return Err(PacketError::TerminalState(self.state));
        }
        Ok(())
    }

    fn require_state(&self, expected: PacketState, target: PacketState) -> Result<(), PacketError> {
        if self.state != expected {
            return Err(PacketError::InvalidTransition {
                from: self.state,
                to: target,
            });
        }
        Ok(())
    }

    fn transition_to(&mut self, new_state: PacketState) {
        self.state = new_state;
        self.last_transition = Utc::now();
    }
}

// ---------------------------------------------------------------------------
// PacketId generation
// ---------------------------------------------------------------------------

fn generate_packet_id(
    sender: &NodeId,
    recipient: &NodeId,
    value: &GoldGrams,
    timestamp: DateTime<Utc>,
    nonce: Uuid,
) -> PacketId {
    let mut hasher = Sha256::new();
    hasher.update(sender.0.as_bytes());
    hasher.update(recipient.0.as_bytes());
    hasher.update(value.0.to_string().as_bytes());
    hasher.update(timestamp.to_rfc3339().as_bytes());
    hasher.update(nonce.as_bytes());
    let hash = hasher.finalize();
    let mut id_bytes = [0u8; 32];
    id_bytes.copy_from_slice(&hash);
    PacketId::new(id_bytes)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn test_sender() -> NodeId {
        NodeId::from("node-sender")
    }

    fn test_recipient() -> NodeId {
        NodeId::from("node-recipient")
    }

    fn test_packet() -> CaesPacket {
        let tier = MarketTier::L0;
        CaesPacket::mint(
            test_sender(),
            test_recipient(),
            GoldGrams(dec!(100)),
            GoldGrams(dec!(0.1)),
            tier,
            tier.default_demurrage_rate(),
            20,
            GoldGrams(dec!(5)),
        )
    }

    // -- Happy path ---------------------------------------------------------

    #[test]
    fn happy_path_mint_to_settle() {
        let mut pkt = test_packet();
        assert_eq!(pkt.state, PacketState::Minted);

        pkt.advance_to_transit()
            .expect("test: minted -> in_transit");
        assert_eq!(pkt.state, PacketState::InTransit);

        pkt.deliver().expect("test: in_transit -> delivered");
        assert_eq!(pkt.state, PacketState::Delivered);

        pkt.settle().expect("test: delivered -> settled");
        assert_eq!(pkt.state, PacketState::Settled);
        assert!(pkt.state.is_terminal());
    }

    // -- Invalid transitions ------------------------------------------------

    #[test]
    fn minted_cannot_settle_directly() {
        let mut pkt = test_packet();
        let err = pkt.settle().unwrap_err();
        assert!(
            matches!(err, PacketError::InvalidTransition { .. }),
            "expected InvalidTransition, got {err:?}"
        );
    }

    #[test]
    fn minted_cannot_deliver() {
        let mut pkt = test_packet();
        let err = pkt.deliver().unwrap_err();
        assert!(matches!(err, PacketError::InvalidTransition { .. }));
    }

    #[test]
    fn in_transit_cannot_settle() {
        let mut pkt = test_packet();
        pkt.advance_to_transit().expect("test: to transit");
        let err = pkt.settle().unwrap_err();
        assert!(matches!(err, PacketError::InvalidTransition { .. }));
    }

    // -- Terminal state rejects transitions ---------------------------------

    #[test]
    fn settled_rejects_all_transitions() {
        let mut pkt = test_packet();
        pkt.advance_to_transit().expect("test: to transit");
        pkt.deliver().expect("test: deliver");
        pkt.settle().expect("test: settle");

        assert!(pkt.advance_to_transit().is_err());
        assert!(pkt.deliver().is_err());
        assert!(pkt.settle().is_err());
        assert!(pkt.hold().is_err());
        assert!(pkt.stall().is_err());
        assert!(pkt.expire().is_err());
        assert!(pkt.dissolve().is_err());
        assert!(pkt.begin_settling().is_err());
        assert!(pkt.disperse().is_err());
        assert!(pkt.refund().is_err());
    }

    #[test]
    fn expired_allows_only_refund() {
        let mut pkt = test_packet();
        pkt.expire().expect("test: expire from minted");

        // These should fail (wrong source state, not terminal block)
        assert!(pkt.advance_to_transit().is_err());
        assert!(pkt.deliver().is_err());
        assert!(pkt.hold().is_err());
        assert!(pkt.settle().is_err());
        assert!(pkt.begin_settling().is_err());
        assert!(pkt.dissolve().is_err());

        // Refund should succeed from Expired
        pkt.refund().expect("test: expired -> refunded");
        assert!(pkt.state.is_terminal());
    }

    // -- Hold / retry cycle -------------------------------------------------

    #[test]
    fn hold_retry_cycle() {
        let mut pkt = test_packet();
        pkt.advance_to_transit().expect("test: transit");
        pkt.deliver().expect("test: deliver");

        pkt.hold().expect("test: delivered -> held");
        assert_eq!(pkt.state, PacketState::Held);
        assert_eq!(pkt.hold_retries, 0);

        pkt.retry_from_hold().expect("test: held -> in_transit");
        assert_eq!(pkt.state, PacketState::InTransit);
        assert_eq!(pkt.hold_retries, 1);

        // Complete the delivery after retry
        pkt.deliver().expect("test: deliver again");
        pkt.settle().expect("test: settle after retry");
    }

    // -- Stall transitions --------------------------------------------------

    #[test]
    fn stall_from_in_transit() {
        let mut pkt = test_packet();
        pkt.advance_to_transit().expect("test: transit");
        pkt.stall().expect("test: in_transit -> stalled");
        assert_eq!(pkt.state, PacketState::Stalled);
    }

    #[test]
    fn stall_from_held() {
        let mut pkt = test_packet();
        pkt.advance_to_transit().expect("test: transit");
        pkt.deliver().expect("test: deliver");
        pkt.hold().expect("test: hold");
        pkt.stall().expect("test: held -> stalled");
        assert_eq!(pkt.state, PacketState::Stalled);
    }

    #[test]
    fn stall_from_delivered_rejected() {
        let mut pkt = test_packet();
        pkt.advance_to_transit().expect("test: transit");
        pkt.deliver().expect("test: deliver");
        let err = pkt.stall().unwrap_err();
        assert!(matches!(err, PacketError::InvalidTransition { .. }));
    }

    // -- Dissolve -----------------------------------------------------------

    #[test]
    fn dissolve_from_held() {
        let mut pkt = test_packet();
        pkt.advance_to_transit().expect("test: transit");
        pkt.deliver().expect("test: deliver");
        pkt.hold().expect("test: hold");
        pkt.dissolve().expect("test: held -> dissolved");
        assert!(pkt.state.is_terminal());
    }

    #[test]
    fn dissolve_from_stalled() {
        let mut pkt = test_packet();
        pkt.advance_to_transit().expect("test: transit");
        pkt.stall().expect("test: stall");
        pkt.dissolve().expect("test: stalled -> dissolved");
        assert_eq!(pkt.state, PacketState::Dissolved);
    }

    #[test]
    fn dissolve_from_minted_rejected() {
        let mut pkt = test_packet();
        let err = pkt.dissolve().unwrap_err();
        assert!(matches!(err, PacketError::InvalidTransition { .. }));
    }

    #[test]
    fn dissolve_from_in_transit_rejected() {
        let mut pkt = test_packet();
        pkt.advance_to_transit().expect("test: transit");
        let err = pkt.dissolve().unwrap_err();
        assert!(matches!(err, PacketError::InvalidTransition { .. }));
    }

    // -- Expire from various states -----------------------------------------

    #[test]
    fn expire_from_minted() {
        let mut pkt = test_packet();
        pkt.expire().expect("test: expire from minted");
        assert_eq!(pkt.state, PacketState::Expired);
    }

    #[test]
    fn expire_from_in_transit() {
        let mut pkt = test_packet();
        pkt.advance_to_transit().expect("test: transit");
        pkt.expire().expect("test: expire from in_transit");
        assert_eq!(pkt.state, PacketState::Expired);
    }

    #[test]
    fn expire_from_held() {
        let mut pkt = test_packet();
        pkt.advance_to_transit().expect("test: transit");
        pkt.deliver().expect("test: deliver");
        pkt.hold().expect("test: hold");
        pkt.expire().expect("test: expire from held");
        assert_eq!(pkt.state, PacketState::Expired);
    }

    // -- Value decay --------------------------------------------------------

    #[test]
    fn current_value_at_creation_is_initial() {
        let pkt = test_packet();
        // Immediately after creation, value should be very close to initial
        let val = pkt.current_value();
        let diff = (val.0 - pkt.initial_value.0).abs();
        assert!(
            diff < dec!(0.01),
            "current_value should be near initial: diff={diff}"
        );
    }

    #[test]
    fn value_at_future_timestamp_is_less() {
        let pkt = test_packet();
        let future = pkt.created_at + chrono::Duration::hours(24);
        let val_now = pkt.value_at(pkt.created_at);
        let val_future = pkt.value_at(future);
        assert!(
            val_future.0 < val_now.0,
            "value should decrease: now={}, future={}",
            val_now.0,
            val_future.0
        );
    }

    // -- Packet ID ----------------------------------------------------------

    #[test]
    fn packet_id_is_nonzero() {
        let pkt = test_packet();
        assert_ne!(pkt.id, PacketId::zero(), "PacketId should be non-zero");
    }

    #[test]
    fn two_packets_have_different_ids() {
        let pkt1 = test_packet();
        let pkt2 = test_packet();
        assert_ne!(pkt1.id, pkt2.id, "Two packets should have unique IDs");
    }

    // -- Hold from stalled --------------------------------------------------

    #[test]
    fn hold_from_stalled() {
        let mut pkt = test_packet();
        pkt.advance_to_transit().expect("test: transit");
        pkt.stall().expect("test: stall");
        pkt.hold().expect("test: stalled -> held");
        assert_eq!(pkt.state, PacketState::Held);
    }

    // -- Retry increments counter -------------------------------------------

    #[test]
    fn multiple_retries_increment_counter() {
        let mut pkt = test_packet();
        pkt.advance_to_transit().expect("test: transit");
        pkt.deliver().expect("test: deliver");

        for i in 1..=3u32 {
            pkt.hold().expect("test: hold");
            pkt.retry_from_hold().expect("test: retry");
            assert_eq!(pkt.hold_retries, i);
            if i < 3 {
                pkt.deliver().expect("test: re-deliver");
            }
        }
    }

    // -- New state transitions (18C) ----------------------------------------

    #[test]
    fn happy_path_with_settling() {
        let mut pkt = test_packet();
        pkt.advance_to_transit()
            .expect("test: minted -> in_transit");
        pkt.deliver().expect("test: in_transit -> delivered");
        pkt.begin_settling().expect("test: delivered -> settling");
        assert_eq!(pkt.state, PacketState::Settling);
        pkt.settle().expect("test: settling -> settled");
        assert_eq!(pkt.state, PacketState::Settled);
        assert!(pkt.state.is_terminal());
    }

    #[test]
    fn settling_to_dispersed_to_retry() {
        let mut pkt = test_packet();
        pkt.advance_to_transit().expect("test: transit");
        pkt.deliver().expect("test: deliver");
        pkt.begin_settling().expect("test: delivered -> settling");
        pkt.disperse().expect("test: settling -> dispersed");
        assert_eq!(pkt.state, PacketState::Dispersed);
        pkt.retry_settlement().expect("test: dispersed -> settling");
        assert_eq!(pkt.state, PacketState::Settling);
        pkt.settle().expect("test: settling -> settled");
        assert_eq!(pkt.state, PacketState::Settled);
    }

    #[test]
    fn expire_then_refund() {
        let mut pkt = test_packet();
        pkt.expire().expect("test: minted -> expired");
        assert_eq!(pkt.state, PacketState::Expired);
        pkt.refund().expect("test: expired -> refunded");
        assert_eq!(pkt.state, PacketState::Refunded);
        assert!(pkt.state.is_terminal());
    }

    #[test]
    fn expired_is_not_terminal() {
        assert!(!PacketState::Expired.is_terminal());
    }

    #[test]
    fn hold_from_in_transit() {
        let mut pkt = test_packet();
        pkt.advance_to_transit().expect("test: transit");
        pkt.hold().expect("test: in_transit -> held");
        assert_eq!(pkt.state, PacketState::Held);
    }

    #[test]
    fn hop_limit_enforcement() {
        let tier = MarketTier::L0;
        let mut pkt = CaesPacket::mint(
            test_sender(),
            test_recipient(),
            GoldGrams(dec!(100)),
            GoldGrams(dec!(0.1)),
            tier,
            tier.default_demurrage_rate(),
            3,
            GoldGrams(dec!(5)),
        );

        pkt.increment_hop().expect("test: hop 1");
        assert_eq!(pkt.hop_count, 1);
        pkt.increment_hop().expect("test: hop 2");
        assert_eq!(pkt.hop_count, 2);
        pkt.increment_hop().expect("test: hop 3");
        assert_eq!(pkt.hop_count, 3);

        let err = pkt.increment_hop().unwrap_err();
        assert!(
            matches!(err, PacketError::HopLimitExceeded { count: 3, limit: 3 }),
            "expected HopLimitExceeded, got {err:?}"
        );
    }

    #[test]
    fn settle_from_delivered_directly() {
        let mut pkt = test_packet();
        pkt.advance_to_transit().expect("test: transit");
        pkt.deliver().expect("test: deliver");
        pkt.settle().expect("test: delivered -> settled");
        assert_eq!(pkt.state, PacketState::Settled);
    }

    #[test]
    fn settle_from_settling() {
        let mut pkt = test_packet();
        pkt.advance_to_transit().expect("test: transit");
        pkt.deliver().expect("test: deliver");
        pkt.begin_settling().expect("test: delivered -> settling");
        pkt.settle().expect("test: settling -> settled");
        assert_eq!(pkt.state, PacketState::Settled);
    }

    #[test]
    fn disperse_only_from_settling() {
        let mut pkt = test_packet();
        pkt.advance_to_transit().expect("test: transit");
        pkt.deliver().expect("test: deliver");

        // Dispersed not allowed from Delivered
        let err = pkt.disperse().unwrap_err();
        assert!(
            matches!(err, PacketError::InvalidTransition { .. }),
            "expected InvalidTransition from Delivered, got {err:?}"
        );

        // Dispersed allowed from Settling
        pkt.begin_settling().expect("test: delivered -> settling");
        pkt.disperse().expect("test: settling -> dispersed");
        assert_eq!(pkt.state, PacketState::Dispersed);
    }

    #[test]
    fn refund_only_from_expired() {
        let mut pkt = test_packet();

        // Refund not allowed from Minted
        let err = pkt.refund().unwrap_err();
        assert!(
            matches!(err, PacketError::InvalidTransition { .. }),
            "expected InvalidTransition from Minted, got {err:?}"
        );

        // Refund allowed from Expired
        pkt.expire().expect("test: minted -> expired");
        pkt.refund().expect("test: expired -> refunded");
        assert_eq!(pkt.state, PacketState::Refunded);
        assert!(pkt.state.is_terminal());
    }

    // -- New fields defaults ------------------------------------------------

    #[test]
    fn mint_initializes_hop_fields() {
        let pkt = test_packet();
        assert_eq!(pkt.hop_count, 0);
        assert_eq!(pkt.hop_limit, 20);
        assert_eq!(pkt.fee_budget, GoldGrams(dec!(5)));
    }
}
