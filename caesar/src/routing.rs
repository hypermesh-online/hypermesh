// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Capacity-based packet routing.
//!
//! Selects the best next-hop for a value packet based on observable network
//! metrics only -- bandwidth, buffer depth, latency, and current load.
//! No trust scores, no reputation, no subjective inputs.

use hypermesh_lib::economic::MarketTier;
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
    #[allow(dead_code)]
    max_candidates: usize,
}

impl Default for PacketRouter {
    fn default() -> Self {
        Self { max_candidates: 5 }
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
