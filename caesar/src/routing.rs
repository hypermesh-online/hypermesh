// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Capacity-based packet routing.
//!
//! Selects the best next-hop for a value packet based on observable network
//! metrics only -- bandwidth, buffer depth, latency, and current load.
//! No trust scores, no reputation, no subjective inputs.

use std::collections::HashMap;

use hypermesh_lib::economic::{GoldGrams, MarketTier};
use hypermesh_lib::NodeId;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors from route selection.
#[derive(Debug, thiserror::Error)]
pub enum RoutingError {
    #[error("no candidates available for routing")]
    NoCandidates,
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Observable capacity metrics for a single node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityMetrics {
    pub node_id: NodeId,
    /// Available egress bandwidth in Mbps.
    pub available_bandwidth_mbps: Decimal,
    /// Free buffer slots (packets).
    pub buffer_capacity_packets: u64,
    /// Average round-trip latency in milliseconds.
    pub avg_latency_ms: Decimal,
    /// Number of packets currently being processed.
    pub active_packet_count: u64,
}

/// Result of route selection -- the chosen next hop plus its score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteSelection {
    pub next_hop: NodeId,
    pub score: Decimal,
    pub metrics: CapacityMetrics,
}

// ---------------------------------------------------------------------------
// Scoring weights
// ---------------------------------------------------------------------------

const WEIGHT_BANDWIDTH: Decimal = dec!(0.35);
const WEIGHT_BUFFER: Decimal = dec!(0.25);
const WEIGHT_LATENCY: Decimal = dec!(0.25);
const WEIGHT_LOAD: Decimal = dec!(0.15);

// ---------------------------------------------------------------------------
// PacketRouter
// ---------------------------------------------------------------------------

/// Capacity-only packet router.
#[derive(Debug, Clone)]
pub struct PacketRouter {
    _max_candidates: usize,
}

impl Default for PacketRouter {
    fn default() -> Self {
        Self { _max_candidates: 5 }
    }
}

impl PacketRouter {
    /// Score and select the best next hop from a set of candidates.
    ///
    /// Score formula:
    ///   score = W_bw * bandwidth + W_buf * buffer - W_lat * latency - W_load * active
    ///
    /// Higher is better.
    pub fn find_route(
        &self,
        candidates: &[CapacityMetrics],
        _packet_tier: MarketTier,
    ) -> Result<RouteSelection, RoutingError> {
        if candidates.is_empty() {
            return Err(RoutingError::NoCandidates);
        }

        let scored: Vec<(usize, Decimal)> = candidates
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let buffer_dec = Decimal::from_u64(m.buffer_capacity_packets)
                    .unwrap_or(Decimal::ZERO);
                let active_dec = Decimal::from_u64(m.active_packet_count)
                    .unwrap_or(Decimal::ZERO);

                let score = WEIGHT_BANDWIDTH * m.available_bandwidth_mbps
                    + WEIGHT_BUFFER * buffer_dec
                    - WEIGHT_LATENCY * m.avg_latency_ms
                    - WEIGHT_LOAD * active_dec;

                (i, score)
            })
            .collect();

        let (best_idx, best_score) = scored
            .iter()
            .max_by(|a, b| a.1.cmp(&b.1))
            .expect("candidates is non-empty");

        let best = &candidates[*best_idx];
        Ok(RouteSelection {
            next_hop: best.node_id.clone(),
            score: *best_score,
            metrics: best.clone(),
        })
    }

    /// Select next hop using engauge capacity reports for scoring.
    ///
    /// Uses engauge's normalized capacity score (bytes served, compute,
    /// storage, bandwidth, uptime) instead of raw network metrics.
    #[cfg(feature = "engauge")]
    pub fn route_with_capacity(
        &self,
        reports: &[engauge::CapacityReport],
        _packet_tier: MarketTier,
    ) -> Result<RouteSelection, RoutingError> {
        if reports.is_empty() {
            return Err(RoutingError::NoCandidates);
        }

        let (best_idx, best_score) = reports
            .iter()
            .enumerate()
            .map(|(i, r)| (i, r.score.value()))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .ok_or(RoutingError::NoCandidates)?;

        let best = &reports[best_idx];
        let score = Decimal::from_f64(best_score).unwrap_or(Decimal::ZERO);

        Ok(RouteSelection {
            next_hop: best.node_id.clone(),
            score,
            metrics: CapacityMetrics {
                node_id: best.node_id.clone(),
                available_bandwidth_mbps: Decimal::from_u64(
                    best.metrics.bandwidth_available_bps / 1_000_000,
                )
                .unwrap_or(Decimal::ZERO),
                buffer_capacity_packets: 0,
                avg_latency_ms: Decimal::ZERO,
                active_packet_count: 0,
            },
        })
    }

    /// Select next hop incorporating operator soft preferences.
    ///
    /// Each candidate's base capacity score is multiplied by the operator's
    /// tier weight for the packet's tier. If the packet value falls outside
    /// the operator's preferred range, a 0.5x penalty is applied. Nodes in
    /// `auto_mode` always use 1.0x multipliers (no preferences).
    pub fn find_route_with_preferences(
        &self,
        candidates: &[CapacityMetrics],
        packet_tier: MarketTier,
        packet_value: GoldGrams,
        operator_prefs: &HashMap<NodeId, crate::models::OperatorPreferences>,
    ) -> Result<RouteSelection, RoutingError> {
        if candidates.is_empty() {
            return Err(RoutingError::NoCandidates);
        }

        let scored: Vec<(usize, Decimal)> = candidates
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let buffer_dec = Decimal::from_u64(m.buffer_capacity_packets)
                    .unwrap_or(Decimal::ZERO);
                let active_dec = Decimal::from_u64(m.active_packet_count)
                    .unwrap_or(Decimal::ZERO);

                let mut score = WEIGHT_BANDWIDTH * m.available_bandwidth_mbps
                    + WEIGHT_BUFFER * buffer_dec
                    - WEIGHT_LATENCY * m.avg_latency_ms
                    - WEIGHT_LOAD * active_dec;

                if let Some(prefs) = operator_prefs.get(&m.node_id) {
                    if !prefs.auto_mode {
                        let tier_weight = match packet_tier {
                            MarketTier::L0 => prefs.tier_weights.l0,
                            MarketTier::L1 => prefs.tier_weights.l1,
                            MarketTier::L2 => prefs.tier_weights.l2,
                            MarketTier::L3 => prefs.tier_weights.l3,
                        };
                        score *= tier_weight;

                        let outside_range =
                            packet_value.0 < prefs.preferred_min_packet.0
                                || packet_value.0 > prefs.preferred_max_packet.0;
                        if outside_range {
                            score *= dec!(0.5);
                        }
                    }
                }

                (i, score)
            })
            .collect();

        let (best_idx, best_score) = scored
            .iter()
            .max_by(|a, b| a.1.cmp(&b.1))
            .expect("candidates is non-empty");

        let best = &candidates[*best_idx];
        Ok(RouteSelection {
            next_hop: best.node_id.clone(),
            score: *best_score,
            metrics: best.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_metrics(
        id: &str,
        bw: Decimal,
        buffer: u64,
        latency: Decimal,
        active: u64,
    ) -> CapacityMetrics {
        CapacityMetrics {
            node_id: NodeId::from(id),
            available_bandwidth_mbps: bw,
            buffer_capacity_packets: buffer,
            avg_latency_ms: latency,
            active_packet_count: active,
        }
    }

    #[test]
    fn route_selects_best_candidate() {
        let router = PacketRouter::default();
        let candidates = vec![
            make_metrics("low", dec!(100), 50, dec!(20), 5),
            make_metrics("best", dec!(500), 200, dec!(5), 2),
            make_metrics("mid", dec!(300), 100, dec!(10), 10),
        ];

        let result = router
            .find_route(&candidates, MarketTier::L0)
            .expect("test: should select best");

        assert_eq!(result.next_hop, NodeId::from("best"));
    }

    #[test]
    fn route_no_candidates_error() {
        let router = PacketRouter::default();
        let err = router.find_route(&[], MarketTier::L0);
        assert!(
            matches!(err, Err(RoutingError::NoCandidates)),
            "expected NoCandidates, got {err:?}"
        );
    }

    #[test]
    fn route_prefers_low_latency() {
        let router = PacketRouter::default();
        // Same bandwidth and buffer, different latency
        let candidates = vec![
            make_metrics("high-lat", dec!(100), 100, dec!(50), 0),
            make_metrics("low-lat", dec!(100), 100, dec!(5), 0),
        ];

        let result = router
            .find_route(&candidates, MarketTier::L1)
            .expect("test: should prefer low latency");

        assert_eq!(result.next_hop, NodeId::from("low-lat"));
    }

    #[test]
    fn route_prefers_high_bandwidth() {
        let router = PacketRouter::default();
        // Same latency and buffer, different bandwidth
        let candidates = vec![
            make_metrics("low-bw", dec!(100), 100, dec!(10), 0),
            make_metrics("high-bw", dec!(500), 100, dec!(10), 0),
        ];

        let result = router
            .find_route(&candidates, MarketTier::L2)
            .expect("test: should prefer high bandwidth");

        assert_eq!(result.next_hop, NodeId::from("high-bw"));
    }

    #[test]
    fn route_avoids_high_load() {
        let router = PacketRouter::default();
        // Same everything except active packet count
        let candidates = vec![
            make_metrics("busy", dec!(200), 100, dec!(10), 500),
            make_metrics("idle", dec!(200), 100, dec!(10), 1),
        ];

        let result = router
            .find_route(&candidates, MarketTier::L0)
            .expect("test: should avoid high load");

        assert_eq!(result.next_hop, NodeId::from("idle"));
    }

    #[test]
    fn route_with_preferences_auto_mode() {
        use crate::models::OperatorPreferences;

        let router = PacketRouter::default();
        let candidates = vec![
            make_metrics("a", dec!(200), 100, dec!(10), 5),
            make_metrics("b", dec!(500), 200, dec!(5), 2),
        ];

        // Both nodes in auto_mode — preferences ignored, highest capacity wins
        let mut prefs = HashMap::new();
        prefs.insert(
            NodeId::from("a"),
            OperatorPreferences { auto_mode: true, ..Default::default() },
        );
        prefs.insert(
            NodeId::from("b"),
            OperatorPreferences { auto_mode: true, ..Default::default() },
        );

        let result = router
            .find_route_with_preferences(
                &candidates,
                MarketTier::L0,
                GoldGrams::from_decimal(dec!(100)),
                &prefs,
            )
            .expect("test: auto_mode routing should succeed");

        // Same result as find_route — node "b" has higher base score
        assert_eq!(result.next_hop, NodeId::from("b"));
    }

    #[test]
    fn route_with_preferences_tier_weight() {
        use crate::models::{OperatorPreferences, TierWeights};

        let router = PacketRouter::default();
        // Equal capacity so base scores are identical
        let candidates = vec![
            make_metrics("high-w", dec!(200), 100, dec!(10), 5),
            make_metrics("low-w", dec!(200), 100, dec!(10), 5),
        ];

        let mut prefs = HashMap::new();
        prefs.insert(
            NodeId::from("high-w"),
            OperatorPreferences {
                tier_weights: TierWeights {
                    l0: dec!(2.0),
                    ..Default::default()
                },
                auto_mode: false,
                ..Default::default()
            },
        );
        prefs.insert(
            NodeId::from("low-w"),
            OperatorPreferences {
                tier_weights: TierWeights {
                    l0: dec!(0.5),
                    ..Default::default()
                },
                auto_mode: false,
                ..Default::default()
            },
        );

        let result = router
            .find_route_with_preferences(
                &candidates,
                MarketTier::L0,
                GoldGrams::from_decimal(dec!(100)),
                &prefs,
            )
            .expect("test: tier weight routing should succeed");

        assert_eq!(result.next_hop, NodeId::from("high-w"));
    }

    #[test]
    fn route_with_preferences_value_penalty() {
        use crate::models::OperatorPreferences;

        let router = PacketRouter::default();
        // Equal capacity
        let candidates = vec![
            make_metrics("strict", dec!(200), 100, dec!(10), 5),
            make_metrics("open", dec!(200), 100, dec!(10), 5),
        ];

        let mut prefs = HashMap::new();
        // "strict" prefers packets >= 50g, packet is only 10g → penalty
        prefs.insert(
            NodeId::from("strict"),
            OperatorPreferences {
                preferred_min_packet: GoldGrams::from_decimal(dec!(50)),
                preferred_max_packet: GoldGrams::from_decimal(dec!(1000)),
                auto_mode: false,
                ..Default::default()
            },
        );
        // "open" accepts anything
        prefs.insert(
            NodeId::from("open"),
            OperatorPreferences {
                preferred_min_packet: GoldGrams::from_decimal(dec!(1)),
                preferred_max_packet: GoldGrams::from_decimal(dec!(1000)),
                auto_mode: false,
                ..Default::default()
            },
        );

        let result = router
            .find_route_with_preferences(
                &candidates,
                MarketTier::L1,
                GoldGrams::from_decimal(dec!(10)),
                &prefs,
            )
            .expect("test: value penalty routing should succeed");

        // "strict" gets 0.5x penalty, "open" does not → "open" wins
        assert_eq!(result.next_hop, NodeId::from("open"));
    }

    #[test]
    fn route_with_preferences_no_prefs_defaults() {
        let router = PacketRouter::default();
        let candidates = vec![
            make_metrics("a", dec!(100), 50, dec!(20), 5),
            make_metrics("b", dec!(500), 200, dec!(5), 2),
        ];

        // Empty prefs map — candidates not found → treated as auto_mode
        let prefs: HashMap<NodeId, crate::models::OperatorPreferences> = HashMap::new();

        let result = router
            .find_route_with_preferences(
                &candidates,
                MarketTier::L0,
                GoldGrams::from_decimal(dec!(100)),
                &prefs,
            )
            .expect("test: no-prefs routing should succeed");

        // Same as find_route — node "b" has higher base score
        assert_eq!(result.next_hop, NodeId::from("b"));
    }

    #[cfg(feature = "engauge")]
    #[test]
    fn route_with_capacity_selects_highest() {
        let router = PacketRouter::default();
        let high = engauge::CapacityReport::new(
            NodeId::from("high-cap"),
            engauge::CapacityMetrics::new(
                1_073_741_824, 1_000_000, 10_737_418_240, 1_000_000_000, 1.0,
            ),
            1,
        );
        let low = engauge::CapacityReport::new(
            NodeId::from("low-cap"),
            engauge::CapacityMetrics::new(100, 100, 100, 100, 0.1),
            1,
        );

        let result = router
            .route_with_capacity(&[low, high], MarketTier::L0)
            .expect("test: capacity routing");

        assert_eq!(result.next_hop, NodeId::from("high-cap"));
    }
}
