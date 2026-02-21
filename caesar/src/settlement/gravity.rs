// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Gravity Dissolution -- distributes abandoned value to qualified nodes.
//!
//! When BOTH sender and recipient abandon a packet (fail to reconnect
//! for 90 days), the residual value is distributed as a "gravity bonus"
//! to qualified nodes that held the shards.

use chrono::{DateTime, Utc};
use hypermesh_lib::{GoldGrams, NodeId, PacketId};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use serde::{Deserialize, Serialize};

/// Dissolution timeout: 90 days (mirrors standard wire transfer timeframes).
pub const DISSOLUTION_TIMEOUT_SECS: u64 = 90 * 24 * 60 * 60; // 7,776,000 seconds

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Gravity dissolution errors.
#[derive(Debug, thiserror::Error)]
pub enum GravityError {
    #[error("no qualified nodes available for gravity distribution")]
    NoQualifiedNodes,

    #[error("residual value is zero -- nothing to dissolve")]
    ZeroResidualValue,

    #[error("dissolution not eligible -- 90-day timeout not reached")]
    NotEligible,
}

// ---------------------------------------------------------------------------
// Qualification
// ---------------------------------------------------------------------------

/// A node's qualification status for receiving gravity bonuses.
///
/// All six conditions must be met (upi_active, ngauge_active, kyc_attested,
/// caesar_active, demonstrable_capacity, active_routing_current_epoch).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GravityQualification {
    pub node_id: NodeId,
    /// Has an active UPI adapter (ingress or egress).
    pub upi_active: bool,
    /// Participates in NGauge governor (future).
    pub ngauge_active: bool,
    /// Has KYC attestation (self-sovereign, network sees attestation only).
    pub kyc_attested: bool,
    /// Active Caesar participant.
    pub caesar_active: bool,
    /// Demonstrable capacity -- PoSpace + bandwidth + compute meets minimum thresholds.
    pub demonstrable_capacity: bool,
    /// Active routing participation in the current settlement epoch.
    pub active_routing_current_epoch: bool,
}

impl GravityQualification {
    /// Whether this node meets ALL qualification requirements.
    pub fn is_qualified(&self) -> bool {
        self.upi_active
            && self.ngauge_active
            && self.kyc_attested
            && self.caesar_active
            && self.demonstrable_capacity
            && self.active_routing_current_epoch
    }
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

/// Result of a gravity dissolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DissolutionResult {
    pub packet_id: PacketId,
    pub total_dissolved: GoldGrams,
    pub distributions: Vec<GravityDistribution>,
    pub dissolved_at: DateTime<Utc>,
}

/// Individual distribution to a qualified node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GravityDistribution {
    pub node_id: NodeId,
    pub amount: GoldGrams,
    /// Whether this node held shards of the packet.
    pub held_shards: bool,
}

// ---------------------------------------------------------------------------
// Protocol
// ---------------------------------------------------------------------------

/// Gravity dissolution -- distributes abandoned value to qualified nodes.
pub struct GravityDissolution;

impl GravityDissolution {
    /// Whether the packet is eligible for dissolution (90 days since last activity).
    pub fn is_eligible_for_dissolution(last_activity: DateTime<Utc>) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(last_activity)
            .num_seconds()
            .max(0) as u64;
        elapsed >= DISSOLUTION_TIMEOUT_SECS
    }

    /// Dissolve residual value among qualified nodes.
    ///
    /// Shard-holding qualified nodes receive 2x weight in the distribution.
    /// Non-shard-holding qualified nodes receive 1x weight.
    pub fn dissolve(
        packet_id: PacketId,
        residual_value: GoldGrams,
        qualified_nodes: &[GravityQualification],
        shard_holders: &[NodeId],
    ) -> Result<DissolutionResult, GravityError> {
        if residual_value.is_zero() {
            return Err(GravityError::ZeroResidualValue);
        }

        // Filter to only qualified nodes
        let qualified: Vec<&GravityQualification> = qualified_nodes
            .iter()
            .filter(|q| q.is_qualified())
            .collect();

        if qualified.is_empty() {
            return Err(GravityError::NoQualifiedNodes);
        }

        // Calculate weights: shard holders get 2x, others get 1x
        let mut weights: Vec<(&GravityQualification, u32, bool)> = Vec::new();
        let mut total_weight: u32 = 0;

        for q in &qualified {
            let is_holder = shard_holders.iter().any(|sh| sh == &q.node_id);
            let weight: u32 = if is_holder { 2 } else { 1 };
            total_weight += weight;
            weights.push((q, weight, is_holder));
        }

        // Distribute proportionally
        let total_dec = Decimal::from_u32(total_weight)
            .expect("test: total weight should convert to Decimal");
        let distributions: Vec<GravityDistribution> = weights
            .iter()
            .map(|(q, weight, is_holder)| {
                let weight_dec = Decimal::from_u32(*weight)
                    .expect("test: weight should convert to Decimal");
                let share = residual_value.0 * weight_dec / total_dec;
                GravityDistribution {
                    node_id: q.node_id.clone(),
                    amount: GoldGrams::from_decimal(share),
                    held_shards: *is_holder,
                }
            })
            .collect();

        Ok(DissolutionResult {
            packet_id,
            total_dissolved: residual_value,
            distributions,
            dissolved_at: Utc::now(),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn qualified_node(id: &str) -> GravityQualification {
        GravityQualification {
            node_id: NodeId::from(id),
            upi_active: true,
            ngauge_active: true,
            kyc_attested: true,
            caesar_active: true,
            demonstrable_capacity: true,
            active_routing_current_epoch: true,
        }
    }

    fn unqualified_node(id: &str) -> GravityQualification {
        GravityQualification {
            node_id: NodeId::from(id),
            upi_active: true,
            ngauge_active: false, // missing ngauge
            kyc_attested: true,
            caesar_active: true,
            demonstrable_capacity: true,
            active_routing_current_epoch: true,
        }
    }

    // -- Qualification ------------------------------------------------------

    #[test]
    fn fully_qualified_node() {
        let q = qualified_node("node-a");
        assert!(q.is_qualified());
    }

    #[test]
    fn missing_any_requirement_disqualifies() {
        let mut q = qualified_node("node-a");

        q.upi_active = false;
        assert!(!q.is_qualified(), "missing UPI should disqualify");

        q.upi_active = true;
        q.ngauge_active = false;
        assert!(!q.is_qualified(), "missing NGauge should disqualify");

        q.ngauge_active = true;
        q.kyc_attested = false;
        assert!(!q.is_qualified(), "missing KYC should disqualify");

        q.kyc_attested = true;
        q.caesar_active = false;
        assert!(!q.is_qualified(), "missing Caesar should disqualify");

        q.caesar_active = true;
        q.demonstrable_capacity = false;
        assert!(!q.is_qualified(), "missing capacity should disqualify");

        q.demonstrable_capacity = true;
        q.active_routing_current_epoch = false;
        assert!(!q.is_qualified(), "missing routing participation should disqualify");
    }

    #[test]
    fn missing_capacity_disqualifies() {
        let mut q = qualified_node("node-cap");
        q.demonstrable_capacity = false;
        assert!(!q.is_qualified(), "missing demonstrable_capacity should disqualify");
    }

    #[test]
    fn missing_routing_participation_disqualifies() {
        let mut q = qualified_node("node-route");
        q.active_routing_current_epoch = false;
        assert!(!q.is_qualified(), "missing active_routing_current_epoch should disqualify");
    }

    // -- Eligibility --------------------------------------------------------

    #[test]
    fn eligible_after_90_days() {
        let past = Utc::now()
            - chrono::Duration::seconds(DISSOLUTION_TIMEOUT_SECS as i64 + 1);
        assert!(GravityDissolution::is_eligible_for_dissolution(past));
    }

    #[test]
    fn not_eligible_before_90_days() {
        let recent = Utc::now()
            - chrono::Duration::seconds(DISSOLUTION_TIMEOUT_SECS as i64 - 3600);
        assert!(!GravityDissolution::is_eligible_for_dissolution(recent));
    }

    #[test]
    fn eligible_at_exact_boundary() {
        // At exactly 90 days, should be eligible (>=)
        let boundary = Utc::now()
            - chrono::Duration::seconds(DISSOLUTION_TIMEOUT_SECS as i64);
        assert!(GravityDissolution::is_eligible_for_dissolution(boundary));
    }

    // -- Dissolution --------------------------------------------------------

    #[test]
    fn dissolve_equal_distribution() {
        let nodes = vec![qualified_node("a"), qualified_node("b")];
        let result = GravityDissolution::dissolve(
            PacketId::zero(),
            GoldGrams::from_decimal(dec!(100)),
            &nodes,
            &[], // no shard holders
        )
        .expect("test: dissolution should succeed");

        assert_eq!(result.distributions.len(), 2);
        // Equal split: 50g each
        for dist in &result.distributions {
            assert_eq!(dist.amount.0, dec!(50));
            assert!(!dist.held_shards);
        }
    }

    #[test]
    fn shard_holders_get_2x_weight() {
        let nodes = vec![qualified_node("holder"), qualified_node("non-holder")];
        let shard_holders = vec![NodeId::from("holder")];

        let result = GravityDissolution::dissolve(
            PacketId::zero(),
            GoldGrams::from_decimal(dec!(90)),
            &nodes,
            &shard_holders,
        )
        .expect("test: dissolution should succeed");

        // Total weight: 2 (holder) + 1 (non-holder) = 3
        // holder gets 2/3 * 90 = 60g, non-holder gets 1/3 * 90 = 30g
        let holder_dist = result
            .distributions
            .iter()
            .find(|d| d.node_id == NodeId::from("holder"))
            .expect("test: holder should be in distributions");
        let non_holder_dist = result
            .distributions
            .iter()
            .find(|d| d.node_id == NodeId::from("non-holder"))
            .expect("test: non-holder should be in distributions");

        assert_eq!(holder_dist.amount.0, dec!(60));
        assert!(holder_dist.held_shards);
        assert_eq!(non_holder_dist.amount.0, dec!(30));
        assert!(!non_holder_dist.held_shards);
    }

    #[test]
    fn single_qualified_node_gets_everything() {
        let nodes = vec![qualified_node("solo")];
        let result = GravityDissolution::dissolve(
            PacketId::zero(),
            GoldGrams::from_decimal(dec!(42)),
            &nodes,
            &[],
        )
        .expect("test: single node dissolution should succeed");

        assert_eq!(result.distributions.len(), 1);
        assert_eq!(result.distributions[0].amount.0, dec!(42));
    }

    #[test]
    fn unqualified_nodes_excluded() {
        let nodes = vec![
            qualified_node("good"),
            unqualified_node("bad"),
        ];
        let result = GravityDissolution::dissolve(
            PacketId::zero(),
            GoldGrams::from_decimal(dec!(100)),
            &nodes,
            &[],
        )
        .expect("test: dissolution with mixed nodes should succeed");

        // Only qualified node receives the full amount
        assert_eq!(result.distributions.len(), 1);
        assert_eq!(result.distributions[0].node_id, NodeId::from("good"));
        assert_eq!(result.distributions[0].amount.0, dec!(100));
    }

    #[test]
    fn no_qualified_nodes_returns_error() {
        let nodes = vec![unqualified_node("bad-1"), unqualified_node("bad-2")];
        let err = GravityDissolution::dissolve(
            PacketId::zero(),
            GoldGrams::from_decimal(dec!(100)),
            &nodes,
            &[],
        )
        .expect_err("test: no qualified nodes should fail");
        assert!(
            matches!(err, GravityError::NoQualifiedNodes),
            "expected NoQualifiedNodes, got: {err}"
        );
    }

    #[test]
    fn zero_residual_value_returns_error() {
        let nodes = vec![qualified_node("a")];
        let err = GravityDissolution::dissolve(
            PacketId::zero(),
            GoldGrams::zero(),
            &nodes,
            &[],
        )
        .expect_err("test: zero value should fail");
        assert!(
            matches!(err, GravityError::ZeroResidualValue),
            "expected ZeroResidualValue, got: {err}"
        );
    }

    #[test]
    fn mix_of_shard_holders_and_non_holders() {
        let nodes = vec![
            qualified_node("h1"),
            qualified_node("h2"),
            qualified_node("n1"),
        ];
        let shard_holders = vec![NodeId::from("h1"), NodeId::from("h2")];

        let result = GravityDissolution::dissolve(
            PacketId::zero(),
            GoldGrams::from_decimal(dec!(100)),
            &nodes,
            &shard_holders,
        )
        .expect("test: mixed dissolution should succeed");

        // Total weight: 2 + 2 + 1 = 5
        // h1 gets 2/5 * 100 = 40, h2 gets 2/5 * 100 = 40, n1 gets 1/5 * 100 = 20
        assert_eq!(result.distributions.len(), 3);

        let h1 = result.distributions.iter()
            .find(|d| d.node_id == NodeId::from("h1"))
            .expect("test: h1 should be in distributions");
        let h2 = result.distributions.iter()
            .find(|d| d.node_id == NodeId::from("h2"))
            .expect("test: h2 should be in distributions");
        let n1 = result.distributions.iter()
            .find(|d| d.node_id == NodeId::from("n1"))
            .expect("test: n1 should be in distributions");

        assert_eq!(h1.amount.0, dec!(40));
        assert!(h1.held_shards);
        assert_eq!(h2.amount.0, dec!(40));
        assert!(h2.held_shards);
        assert_eq!(n1.amount.0, dec!(20));
        assert!(!n1.held_shards);
    }

    #[test]
    fn total_dissolved_matches_input() {
        let nodes = vec![qualified_node("a"), qualified_node("b")];
        let value = GoldGrams::from_decimal(dec!(77.5));
        let result = GravityDissolution::dissolve(
            PacketId::zero(),
            value,
            &nodes,
            &[],
        )
        .expect("test: dissolution should succeed");
        assert_eq!(result.total_dissolved, value);
    }
}
