// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Fee distribution -- splitting transit fees among egress and relay nodes.
//!
//! No value creation occurs here. Fees collected during packet transit are
//! distributed proportionally: egress node gets the lion's share, relay nodes
//! split the remainder weighted by bytes relayed.

use hypermesh_lib::economic::GoldGrams;
use hypermesh_lib::NodeId;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors from fee distribution.
#[derive(Debug, thiserror::Error)]
pub enum FeeError {
    #[error("zero fee -- nothing to distribute")]
    ZeroFee,
}

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Complete distribution of a single fee across participating nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeDistribution {
    /// The total fee that was distributed.
    pub total_fee: GoldGrams,
    /// Payment to the egress (destination) node.
    pub egress_payment: NodePayment,
    /// Payments to transit relay nodes (may be empty).
    pub transit_payments: Vec<NodePayment>,
}

/// A payment to a specific node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePayment {
    pub node_id: NodeId,
    pub amount: GoldGrams,
}

// ---------------------------------------------------------------------------
// FeeDistributor
// ---------------------------------------------------------------------------

/// Stateless fee splitter -- holds the egress/transit share ratio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeDistributor {
    /// Fraction of the total fee allocated to the egress node (default 0.80).
    pub egress_share: Decimal,
    /// Fraction of the total fee allocated to transit nodes (default 0.20).
    pub transit_share: Decimal,
}

impl Default for FeeDistributor {
    fn default() -> Self {
        Self {
            egress_share: dec!(0.80),
            transit_share: dec!(0.20),
        }
    }
}

impl FeeDistributor {
    /// Distribute a fee among the egress node and zero-or-more transit nodes.
    ///
    /// Transit nodes receive shares proportional to `bytes_relayed`. If no
    /// transit nodes participated, the egress node receives the entire fee.
    pub fn distribute_fee(
        &self,
        total_fee: GoldGrams,
        egress_node: NodeId,
        transit_nodes: &[(NodeId, u64)],
    ) -> Result<FeeDistribution, FeeError> {
        if total_fee.is_zero() {
            return Err(FeeError::ZeroFee);
        }

        // No transit nodes -- egress gets everything
        if transit_nodes.is_empty() {
            return Ok(FeeDistribution {
                total_fee,
                egress_payment: NodePayment {
                    node_id: egress_node,
                    amount: total_fee,
                },
                transit_payments: Vec::new(),
            });
        }

        let egress_amount = GoldGrams::from_decimal(total_fee.0 * self.egress_share);
        let transit_pool = GoldGrams::from_decimal(total_fee.0 * self.transit_share);

        let total_bytes: u64 = transit_nodes.iter().map(|(_, b)| b).sum();

        let transit_payments: Vec<NodePayment> = if total_bytes == 0 {
            // All transit nodes relayed zero bytes -- split equally
            let count = Decimal::from_usize(transit_nodes.len())
                .unwrap_or(Decimal::ONE);
            let per_node = GoldGrams::from_decimal(transit_pool.0 / count);
            transit_nodes
                .iter()
                .map(|(node_id, _)| NodePayment {
                    node_id: node_id.clone(),
                    amount: per_node,
                })
                .collect()
        } else {
            let total_dec = Decimal::from_u64(total_bytes)
                .unwrap_or(Decimal::ONE);
            transit_nodes
                .iter()
                .map(|(node_id, bytes)| {
                    let bytes_dec = Decimal::from_u64(*bytes)
                        .unwrap_or(Decimal::ZERO);
                    let share = bytes_dec / total_dec;
                    NodePayment {
                        node_id: node_id.clone(),
                        amount: GoldGrams::from_decimal(transit_pool.0 * share),
                    }
                })
                .collect()
        };

        Ok(FeeDistribution {
            total_fee,
            egress_payment: NodePayment {
                node_id: egress_node,
                amount: egress_amount,
            },
            transit_payments,
        })
    }

    /// Distribute fees weighted by engauge capacity scores instead of raw bytes.
    ///
    /// Transit nodes with higher capacity scores receive proportionally more of
    /// the transit fee pool. Falls back to equal split if all scores are zero.
    #[cfg(feature = "engauge")]
    pub fn distribute_with_capacity_weights(
        &self,
        total_fee: GoldGrams,
        egress_node: NodeId,
        transit_nodes: &[(NodeId, engauge::CapacityMetrics)],
    ) -> Result<FeeDistribution, FeeError> {
        if total_fee.is_zero() {
            return Err(FeeError::ZeroFee);
        }

        if transit_nodes.is_empty() {
            return Ok(FeeDistribution {
                total_fee,
                egress_payment: NodePayment {
                    node_id: egress_node,
                    amount: total_fee,
                },
                transit_payments: Vec::new(),
            });
        }

        let egress_amount = GoldGrams::from_decimal(total_fee.0 * self.egress_share);
        let transit_pool = GoldGrams::from_decimal(total_fee.0 * self.transit_share);

        let scores: Vec<f64> = transit_nodes
            .iter()
            .map(|(_, m)| engauge::CapacityScore::calculate(m).value())
            .collect();
        let total_score: f64 = scores.iter().sum();

        let transit_payments: Vec<NodePayment> = if total_score < f64::EPSILON {
            let count = Decimal::from_usize(transit_nodes.len())
                .unwrap_or(Decimal::ONE);
            let per_node = GoldGrams::from_decimal(transit_pool.0 / count);
            transit_nodes
                .iter()
                .map(|(node_id, _)| NodePayment {
                    node_id: node_id.clone(),
                    amount: per_node,
                })
                .collect()
        } else {
            transit_nodes
                .iter()
                .zip(scores.iter())
                .map(|((node_id, _), &score)| {
                    let share = Decimal::from_f64(score / total_score)
                        .unwrap_or(Decimal::ZERO);
                    NodePayment {
                        node_id: node_id.clone(),
                        amount: GoldGrams::from_decimal(transit_pool.0 * share),
                    }
                })
                .collect()
        };

        Ok(FeeDistribution {
            total_fee,
            egress_payment: NodePayment {
                node_id: egress_node,
                amount: egress_amount,
            },
            transit_payments,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn distributor() -> FeeDistributor {
        FeeDistributor::default()
    }

    #[test]
    fn distribute_with_transit_nodes() {
        let dist = distributor();
        let result = dist
            .distribute_fee(
                GoldGrams(dec!(10)),
                NodeId::from("egress"),
                &[
                    (NodeId::from("relay-1"), 500),
                    (NodeId::from("relay-2"), 500),
                ],
            )
            .expect("test: should distribute");

        assert_eq!(result.egress_payment.amount, GoldGrams(dec!(8)));
        assert_eq!(result.transit_payments.len(), 2);
        assert_eq!(result.transit_payments[0].amount, GoldGrams(dec!(1)));
        assert_eq!(result.transit_payments[1].amount, GoldGrams(dec!(1)));
    }

    #[test]
    fn distribute_no_transit_nodes() {
        let dist = distributor();
        let result = dist
            .distribute_fee(
                GoldGrams(dec!(10)),
                NodeId::from("egress"),
                &[],
            )
            .expect("test: egress-only distribution");

        assert_eq!(result.egress_payment.amount, GoldGrams(dec!(10)));
        assert!(result.transit_payments.is_empty());
    }

    #[test]
    fn distribute_weighted_by_bytes() {
        let dist = distributor();
        let result = dist
            .distribute_fee(
                GoldGrams(dec!(100)),
                NodeId::from("egress"),
                &[
                    (NodeId::from("relay-1"), 750),
                    (NodeId::from("relay-2"), 250),
                ],
            )
            .expect("test: weighted distribution");

        // Transit pool = 20g, relay-1 gets 75% = 15g, relay-2 gets 25% = 5g
        assert_eq!(result.egress_payment.amount, GoldGrams(dec!(80)));
        assert_eq!(result.transit_payments[0].amount, GoldGrams(dec!(15)));
        assert_eq!(result.transit_payments[1].amount, GoldGrams(dec!(5)));
    }

    #[test]
    fn distribute_zero_fee_error() {
        let dist = distributor();
        let err = dist.distribute_fee(
            GoldGrams::zero(),
            NodeId::from("egress"),
            &[],
        );
        assert!(
            matches!(err, Err(FeeError::ZeroFee)),
            "expected ZeroFee, got {err:?}"
        );
    }

    #[test]
    fn distribute_single_transit_node() {
        let dist = distributor();
        let result = dist
            .distribute_fee(
                GoldGrams(dec!(10)),
                NodeId::from("egress"),
                &[(NodeId::from("relay-1"), 1000)],
            )
            .expect("test: single transit node");

        assert_eq!(result.egress_payment.amount, GoldGrams(dec!(8)));
        assert_eq!(result.transit_payments.len(), 1);
        assert_eq!(result.transit_payments[0].amount, GoldGrams(dec!(2)));
    }

    #[test]
    fn custom_split_ratio() {
        let dist = FeeDistributor {
            egress_share: dec!(0.70),
            transit_share: dec!(0.30),
        };
        let result = dist
            .distribute_fee(
                GoldGrams(dec!(100)),
                NodeId::from("egress"),
                &[(NodeId::from("relay-1"), 100)],
            )
            .expect("test: custom 70/30 split");

        assert_eq!(result.egress_payment.amount, GoldGrams(dec!(70)));
        assert_eq!(result.transit_payments[0].amount, GoldGrams(dec!(30)));
    }

    #[cfg(feature = "engauge")]
    #[test]
    fn distribute_with_capacity_weights_proportional() {
        let dist = distributor();
        let high_cap = engauge::CapacityMetrics::new(
            1_073_741_824, 1_000_000, 10_737_418_240, 1_000_000_000, 1.0,
        );
        let low_cap = engauge::CapacityMetrics::new(
            100_000, 10_000, 100_000, 100_000, 0.5,
        );
        let result = dist
            .distribute_with_capacity_weights(
                GoldGrams(dec!(100)),
                NodeId::from("egress"),
                &[
                    (NodeId::from("high-cap"), high_cap),
                    (NodeId::from("low-cap"), low_cap),
                ],
            )
            .expect("test: capacity-weighted distribution");

        assert_eq!(result.egress_payment.amount, GoldGrams(dec!(80)));
        assert_eq!(result.transit_payments.len(), 2);
        // High-cap node should get more than low-cap
        assert!(
            result.transit_payments[0].amount.0 > result.transit_payments[1].amount.0,
            "high-cap {} should exceed low-cap {}",
            result.transit_payments[0].amount, result.transit_payments[1].amount
        );
    }
}
