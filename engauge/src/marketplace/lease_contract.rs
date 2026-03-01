// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Lease contract representing a time-bounded resource access agreement.
//!
//! Each lease is conceptually a BlockMatrix asset with PoS verification.
//! The types here define the contract structure; actual blockchain
//! registration happens in BlockMatrix.

use chrono::{DateTime, Utc};
use hypermesh_lib::economic::{GoldGrams, MarketTier, PacketId};
use hypermesh_lib::{AssetId, NodeId};
use serde::{Deserialize, Serialize};

use super::resource_pool::LeaseableResource;

/// State of a lease contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeaseState {
    /// Proposed, awaiting provider confirmation.
    Proposed,
    /// Active -- consumer has access to the resource.
    Active,
    /// Completed -- lease expired normally.
    Completed,
    /// Cancelled -- terminated early by either party.
    Cancelled,
}

/// A lease contract between a provider and consumer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseContract {
    pub lease_id: AssetId,
    pub provider: NodeId,
    pub consumer: NodeId,
    pub resource_kind: LeaseableResource,
    pub allocation_percentage: u8,
    pub price_per_epoch: GoldGrams,
    pub tier: MarketTier,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub state: LeaseState,
    pub settlement_evp: Option<PacketId>,
}

impl LeaseContract {
    /// Create a new proposed lease.
    pub fn propose(
        provider: NodeId,
        consumer: NodeId,
        resource_kind: LeaseableResource,
        allocation_percentage: u8,
        price_per_epoch: GoldGrams,
        tier: MarketTier,
        duration: chrono::Duration,
    ) -> Self {
        let now = Utc::now();
        let lease_id = AssetId(format!("lease-{}", uuid::Uuid::new_v4()));
        Self {
            lease_id,
            provider,
            consumer,
            resource_kind,
            allocation_percentage: allocation_percentage.min(100),
            price_per_epoch,
            tier,
            start_time: now,
            end_time: now + duration,
            state: LeaseState::Proposed,
            settlement_evp: None,
        }
    }

    /// Activate the lease (provider accepted).
    pub fn activate(&mut self) -> Result<(), LeaseError> {
        if self.state != LeaseState::Proposed {
            return Err(LeaseError::InvalidTransition {
                from: self.state,
                to: LeaseState::Active,
            });
        }
        self.state = LeaseState::Active;
        Ok(())
    }

    /// Complete the lease (expired normally).
    pub fn complete(&mut self) -> Result<(), LeaseError> {
        if self.state != LeaseState::Active {
            return Err(LeaseError::InvalidTransition {
                from: self.state,
                to: LeaseState::Completed,
            });
        }
        self.state = LeaseState::Completed;
        Ok(())
    }

    /// Cancel the lease.
    pub fn cancel(&mut self) -> Result<(), LeaseError> {
        if self.state == LeaseState::Completed || self.state == LeaseState::Cancelled {
            return Err(LeaseError::InvalidTransition {
                from: self.state,
                to: LeaseState::Cancelled,
            });
        }
        self.state = LeaseState::Cancelled;
        Ok(())
    }

    /// Whether the lease is currently active.
    pub fn is_active(&self) -> bool {
        self.state == LeaseState::Active
    }

    /// Whether the lease has expired based on wall clock time.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.end_time
    }

    /// Attach a Caesar EVP settlement packet to this lease.
    pub fn attach_settlement(&mut self, packet_id: PacketId) {
        self.settlement_evp = Some(packet_id);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LeaseError {
    #[error("invalid lease state transition: {from:?} -> {to:?}")]
    InvalidTransition { from: LeaseState, to: LeaseState },
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::economic::GoldGrams;
    use rust_decimal::Decimal;

    fn test_price() -> GoldGrams {
        GoldGrams::from_decimal(Decimal::new(1, 3)) // 0.001g
    }

    #[test]
    fn propose_creates_proposed_state() {
        let contract = LeaseContract::propose(
            NodeId::from_public_key(b"provider-1"),
            NodeId::from_public_key(b"consumer-1"),
            LeaseableResource::Cpu,
            50,
            test_price(),
            MarketTier::L0,
            chrono::Duration::hours(1),
        );
        assert_eq!(contract.state, LeaseState::Proposed);
        assert!(contract.lease_id.0.starts_with("lease-"));
        assert_eq!(contract.allocation_percentage, 50);
    }

    #[test]
    fn activate_transitions_proposed_to_active() {
        let mut contract = LeaseContract::propose(
            NodeId::from_public_key(b"provider-1"),
            NodeId::from_public_key(b"consumer-1"),
            LeaseableResource::Gpu,
            30,
            test_price(),
            MarketTier::L1,
            chrono::Duration::hours(2),
        );
        contract.activate().expect("test: activate lease");
        assert_eq!(contract.state, LeaseState::Active);
        assert!(contract.is_active());
    }

    #[test]
    fn complete_transitions_active_to_completed() {
        let mut contract = LeaseContract::propose(
            NodeId::from_public_key(b"provider-1"),
            NodeId::from_public_key(b"consumer-1"),
            LeaseableResource::Memory,
            60,
            test_price(),
            MarketTier::L2,
            chrono::Duration::hours(1),
        );
        contract.activate().expect("test: activate");
        contract.complete().expect("test: complete lease");
        assert_eq!(contract.state, LeaseState::Completed);
        assert!(!contract.is_active());
    }

    #[test]
    fn cancel_from_active_succeeds() {
        let mut contract = LeaseContract::propose(
            NodeId::from_public_key(b"provider-1"),
            NodeId::from_public_key(b"consumer-1"),
            LeaseableResource::Storage,
            40,
            test_price(),
            MarketTier::L0,
            chrono::Duration::hours(1),
        );
        contract.activate().expect("test: activate");
        contract.cancel().expect("test: cancel active lease");
        assert_eq!(contract.state, LeaseState::Cancelled);
    }

    #[test]
    fn invalid_transitions_return_errors() {
        let mut contract = LeaseContract::propose(
            NodeId::from_public_key(b"provider-1"),
            NodeId::from_public_key(b"consumer-1"),
            LeaseableResource::Bandwidth,
            20,
            test_price(),
            MarketTier::L3,
            chrono::Duration::hours(1),
        );

        // Complete from Proposed should fail (must be Active first).
        let err = contract
            .complete()
            .expect_err("test: complete from proposed");
        match err {
            LeaseError::InvalidTransition { from, to } => {
                assert_eq!(from, LeaseState::Proposed);
                assert_eq!(to, LeaseState::Completed);
            }
        }

        // Activate, complete, then try to cancel -- should fail.
        contract.activate().expect("test: activate");
        contract.complete().expect("test: complete");
        let err = contract.cancel().expect_err("test: cancel from completed");
        match err {
            LeaseError::InvalidTransition { from, to } => {
                assert_eq!(from, LeaseState::Completed);
                assert_eq!(to, LeaseState::Cancelled);
            }
        }
    }
}
