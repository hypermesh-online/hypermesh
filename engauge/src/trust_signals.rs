// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Trust signals exposed to trustchain federation gating.
//!
//! Engauge already tracks per-node activity, capacity, and traffic
//! classification.  Phase F.1 wires those signals into TrustChain's
//! `FederationManager` so federation peers are admitted at a trust level
//! consistent with their observed behaviour rather than what they request.
//!
//! The mapping is deliberately conservative: a peer must show high activity
//! AND adequate capacity AND organic traffic to earn `Full` trust.  Any
//! Speculative classification, low capacity, or low activity demotes the
//! peer to `Conditional`.  Byzantine signals (handled by the caller) force
//! `Untrusted` regardless of these signals.

use serde::{Deserialize, Serialize};

use crate::capacity::CapacityMetrics;
use crate::metrics::ActivityScore;
use crate::organic_detection::TrafficClassification;

/// Aggregate trust signals for a single peer, drawn directly from
/// engauge's existing measurement subsystems.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerTrustSignals {
    /// Composite activity score (0.0..1.0 axes — compute, bandwidth,
    /// latency, receipts).
    pub activity_score: ActivityScore,
    /// Raw capacity measurements for the peer.
    pub capacity: CapacityMetrics,
    /// Aggregate traffic classification (Organic / Speculative / Mixed).
    pub traffic_classification: TrafficClassification,
}

/// Coarse trust band derived from engauge signals.
///
/// Maps directly onto `trustchain::ca::federation::FederationTrustLevel`
/// at the call site.  Kept here as an engauge-owned enum so this crate
/// doesn't take a dependency on trustchain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustBand {
    /// High activity + good capacity + organic-only traffic.
    Full,
    /// Mid metrics OR speculative-tagged traffic.
    Conditional,
    /// Forced by the caller when a byzantine signal is present
    /// (engauge itself never returns this — see
    /// `trustchain::security::ByzantineDetector`).
    Untrusted,
}

/// Activity-score composite threshold below which we will not promote
/// a peer to `Full`.
const FULL_ACTIVITY_FLOOR: f64 = 0.6;
/// Bandwidth (bps) below which we drop to `Conditional`.
const FULL_BANDWIDTH_FLOOR_BPS: u64 = 10_000_000; // 10 Mbps
/// Uptime ratio below which we drop to `Conditional`.
const FULL_UPTIME_FLOOR: f64 = 0.9;
/// Speculative-traffic confidence above which we always demote.
const SPECULATIVE_CONFIDENCE_CEILING: f64 = 0.5;

impl PeerTrustSignals {
    /// Construct from raw signals.
    pub fn new(
        activity_score: ActivityScore,
        capacity: CapacityMetrics,
        traffic_classification: TrafficClassification,
    ) -> Self {
        Self {
            activity_score,
            capacity,
            traffic_classification,
        }
    }

    /// Coarse trust band derived from the underlying signals.
    ///
    /// - `Full` requires: composite activity >= floor, bandwidth >= floor,
    ///   uptime >= floor, AND traffic classified as `Organic`.
    /// - `Conditional` is returned when capacity is adequate but activity
    ///   or traffic patterns are mixed/speculative.
    /// - This function never returns `Untrusted` — that is reserved for
    ///   the caller's byzantine override.
    pub fn trust_band(&self) -> TrustBand {
        // Speculative traffic above the confidence ceiling always demotes.
        if let TrafficClassification::Speculative { confidence } = &self.traffic_classification {
            if *confidence >= SPECULATIVE_CONFIDENCE_CEILING {
                return TrustBand::Conditional;
            }
        }

        let activity_ok = self.activity_score.composite() >= FULL_ACTIVITY_FLOOR;
        let bandwidth_ok = self.capacity.bandwidth_available_bps >= FULL_BANDWIDTH_FLOOR_BPS;
        let uptime_ok = self.capacity.uptime_f64() >= FULL_UPTIME_FLOOR;
        let organic = matches!(self.traffic_classification, TrafficClassification::Organic { .. });

        if activity_ok && bandwidth_ok && uptime_ok && organic {
            TrustBand::Full
        } else {
            TrustBand::Conditional
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn high_activity() -> ActivityScore {
        ActivityScore {
            compute_score: 0.8,
            bandwidth_score: 0.8,
            latency_score: 0.7,
            receipt_density: 0.7,
        }
    }

    fn low_activity() -> ActivityScore {
        ActivityScore {
            compute_score: 0.1,
            bandwidth_score: 0.1,
            latency_score: 0.1,
            receipt_density: 0.1,
        }
    }

    fn good_capacity() -> CapacityMetrics {
        CapacityMetrics::new(
            10_000_000,                // bytes_served
            1_000_000,                 // compute_delivered
            10 * 1024 * 1024 * 1024,   // 10 GB storage
            100_000_000,               // 100 Mbps
            0.99,                      // uptime
        )
    }

    fn poor_capacity() -> CapacityMetrics {
        CapacityMetrics::new(100, 10, 1024 * 1024, 1_000_000, 0.50)
    }

    #[test]
    fn high_signals_organic_yield_full() {
        let signals = PeerTrustSignals::new(
            high_activity(),
            good_capacity(),
            TrafficClassification::Organic { confidence: 0.95 },
        );
        assert_eq!(signals.trust_band(), TrustBand::Full);
    }

    #[test]
    fn low_capacity_demotes_to_conditional() {
        let signals = PeerTrustSignals::new(
            high_activity(),
            poor_capacity(),
            TrafficClassification::Organic { confidence: 0.95 },
        );
        assert_eq!(signals.trust_band(), TrustBand::Conditional);
    }

    #[test]
    fn low_activity_demotes_to_conditional() {
        let signals = PeerTrustSignals::new(
            low_activity(),
            good_capacity(),
            TrafficClassification::Organic { confidence: 0.95 },
        );
        assert_eq!(signals.trust_band(), TrustBand::Conditional);
    }

    #[test]
    fn speculative_traffic_demotes_to_conditional() {
        let signals = PeerTrustSignals::new(
            high_activity(),
            good_capacity(),
            TrafficClassification::Speculative { confidence: 0.9 },
        );
        assert_eq!(signals.trust_band(), TrustBand::Conditional);
    }

    #[test]
    fn mixed_traffic_demotes_to_conditional() {
        let signals = PeerTrustSignals::new(
            high_activity(),
            good_capacity(),
            TrafficClassification::Mixed {
                confidence: 0.7,
                organic_ratio: 0.5,
            },
        );
        // Mixed isn't Organic, so it falls to Conditional.
        assert_eq!(signals.trust_band(), TrustBand::Conditional);
    }
}
