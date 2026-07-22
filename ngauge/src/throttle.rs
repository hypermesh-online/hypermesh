// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Governor feedback signal generation.
//!
//! [`NGaugeThrottle`] combines metrics and traffic classification into a
//! single [`ThrottleSignal`] that the Governor uses to adjust band pricing
//! and demurrage rates.
//!
//! - **Organic** traffic receives lower band modifiers (cheaper) and lower
//!   demurrage modifiers (slower decay).
//! - **Speculative** traffic receives higher modifiers on both axes.
//! - **Mixed** traffic blends proportionally via `organic_ratio`.

use serde::{Deserialize, Serialize};

use crate::metrics::MetricsSnapshot;
use crate::organic_detection::TrafficClassification;

// ---------------------------------------------------------------------------
// ThrottleSignal
// ---------------------------------------------------------------------------

/// Governor feedback signal produced by ngauge.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThrottleSignal {
    /// Composite activity score (0.0..1.0).
    pub activity_score: f64,
    /// Band price multiplier (0.5..2.0).
    pub band_modifier: f64,
    /// Demurrage rate multiplier (0.8..1.2).
    pub demurrage_modifier: f64,
    /// Estimated organic fraction (0.0..1.0).
    pub organic_ratio: f64,
}

// ---------------------------------------------------------------------------
// NGaugeThrottle
// ---------------------------------------------------------------------------

/// Combines metrics + classification into a [`ThrottleSignal`].
#[derive(Debug, Clone)]
pub struct NGaugeThrottle {
    /// Band modifier floor for fully organic traffic.
    organic_band_floor: f64,
    /// Band modifier ceiling for fully speculative traffic.
    speculative_band_ceiling: f64,
    /// Demurrage modifier floor for fully organic traffic.
    organic_demurrage_floor: f64,
    /// Demurrage modifier ceiling for fully speculative traffic.
    speculative_demurrage_ceiling: f64,
}

impl NGaugeThrottle {
    /// Create with default modifier ranges.
    pub fn new() -> Self {
        Self {
            organic_band_floor: 0.5,
            speculative_band_ceiling: 2.0,
            organic_demurrage_floor: 0.8,
            speculative_demurrage_ceiling: 1.2,
        }
    }

    /// Generate a throttle signal from a metrics snapshot and classification.
    pub fn generate_signal(
        &self,
        metrics: &MetricsSnapshot,
        classification: &TrafficClassification,
    ) -> ThrottleSignal {
        let organic_ratio = classification.organic_ratio();
        let speculative_ratio = 1.0 - organic_ratio;

        // Band modifier: organic -> floor, speculative -> ceiling
        let band_modifier = self.organic_band_floor * organic_ratio
            + self.speculative_band_ceiling * speculative_ratio;
        let band_modifier = band_modifier.clamp(0.5, 2.0);

        // Demurrage modifier: organic -> floor (slower decay), speculative -> ceiling
        let demurrage_modifier = self.organic_demurrage_floor * organic_ratio
            + self.speculative_demurrage_ceiling * speculative_ratio;
        let demurrage_modifier = demurrage_modifier.clamp(0.8, 1.2);

        let activity_score = metrics.activity_score.composite();

        ThrottleSignal {
            activity_score,
            band_modifier,
            demurrage_modifier,
            organic_ratio,
        }
    }
}

impl Default for NGaugeThrottle {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{ActivityScore, MetricsSnapshot};
    use chrono::Utc;

    fn stub_snapshot(composite: f64) -> MetricsSnapshot {
        // Build a snapshot with a hand-crafted activity score.
        // composite = 0.3*c + 0.3*b + 0.2*l + 0.2*r
        // Set all axes equal so composite matches.
        MetricsSnapshot {
            captured_at: Utc::now(),
            compute_cycles: 0,
            bandwidth_bytes: 0,
            avg_latency_ms: 0.0,
            latency_sample_count: 0,
            receipt_count: 0,
            active_since: Utc::now(),
            activity_score: ActivityScore {
                compute_score: composite,
                bandwidth_score: composite,
                latency_score: composite,
                receipt_density: composite,
            },
        }
    }

    #[test]
    fn fully_organic_signal() {
        let throttle = NGaugeThrottle::new();
        let snap = stub_snapshot(0.8);
        let class = TrafficClassification::Organic { confidence: 0.9 };

        let signal = throttle.generate_signal(&snap, &class);

        assert!((signal.organic_ratio - 1.0).abs() < 1e-9);
        assert!(
            (signal.band_modifier - 0.5).abs() < 1e-9,
            "band: {}",
            signal.band_modifier
        );
        assert!(
            (signal.demurrage_modifier - 0.8).abs() < 1e-9,
            "demurrage: {}",
            signal.demurrage_modifier
        );
        assert!((signal.activity_score - 0.8).abs() < 1e-9);
    }

    #[test]
    fn fully_speculative_signal() {
        let throttle = NGaugeThrottle::new();
        let snap = stub_snapshot(0.3);
        let class = TrafficClassification::Speculative { confidence: 0.85 };

        let signal = throttle.generate_signal(&snap, &class);

        assert!((signal.organic_ratio).abs() < 1e-9);
        assert!((signal.band_modifier - 2.0).abs() < 1e-9);
        assert!((signal.demurrage_modifier - 1.2).abs() < 1e-9);
    }

    #[test]
    fn mixed_signal_blends() {
        let throttle = NGaugeThrottle::new();
        let snap = stub_snapshot(0.5);
        let class = TrafficClassification::Mixed {
            confidence: 0.6,
            organic_ratio: 0.5,
        };

        let signal = throttle.generate_signal(&snap, &class);

        // band = 0.5 * 0.5 + 2.0 * 0.5 = 0.25 + 1.0 = 1.25
        assert!(
            (signal.band_modifier - 1.25).abs() < 1e-9,
            "band: {}",
            signal.band_modifier
        );
        // demurrage = 0.8 * 0.5 + 1.2 * 0.5 = 0.4 + 0.6 = 1.0
        assert!((signal.demurrage_modifier - 1.0).abs() < 1e-9);
        assert!((signal.organic_ratio - 0.5).abs() < 1e-9);
    }

    #[test]
    fn band_modifier_clamped() {
        let throttle = NGaugeThrottle::new();
        let snap = stub_snapshot(0.5);

        // Organic ratio 1.0 -> band = 0.5 (at floor)
        let class = TrafficClassification::Organic { confidence: 1.0 };
        let signal = throttle.generate_signal(&snap, &class);
        assert!(signal.band_modifier >= 0.5);
        assert!(signal.band_modifier <= 2.0);

        // Speculative ratio 1.0 -> band = 2.0 (at ceiling)
        let class = TrafficClassification::Speculative { confidence: 1.0 };
        let signal = throttle.generate_signal(&snap, &class);
        assert!(signal.band_modifier >= 0.5);
        assert!(signal.band_modifier <= 2.0);
    }

    #[test]
    fn demurrage_modifier_clamped() {
        let throttle = NGaugeThrottle::new();
        let snap = stub_snapshot(0.5);

        let class = TrafficClassification::Organic { confidence: 1.0 };
        let signal = throttle.generate_signal(&snap, &class);
        assert!(signal.demurrage_modifier >= 0.8);
        assert!(signal.demurrage_modifier <= 1.2);

        let class = TrafficClassification::Speculative { confidence: 1.0 };
        let signal = throttle.generate_signal(&snap, &class);
        assert!(signal.demurrage_modifier >= 0.8);
        assert!(signal.demurrage_modifier <= 1.2);
    }

    #[test]
    fn activity_score_passthrough() {
        let throttle = NGaugeThrottle::new();
        let snap = stub_snapshot(0.42);
        let class = TrafficClassification::Organic { confidence: 0.5 };

        let signal = throttle.generate_signal(&snap, &class);
        assert!(
            (signal.activity_score - 0.42).abs() < 1e-9,
            "score: {}",
            signal.activity_score
        );
    }

    #[test]
    fn signal_serde_roundtrip() {
        let signal = ThrottleSignal {
            activity_score: 0.75,
            band_modifier: 1.1,
            demurrage_modifier: 0.95,
            organic_ratio: 0.6,
        };
        let json = serde_json::to_string(&signal).expect("test: serialize signal");
        let back: ThrottleSignal = serde_json::from_str(&json).expect("test: deserialize signal");
        assert!((signal.activity_score - back.activity_score).abs() < 1e-9);
        assert!((signal.band_modifier - back.band_modifier).abs() < 1e-9);
        assert!((signal.demurrage_modifier - back.demurrage_modifier).abs() < 1e-9);
        assert!((signal.organic_ratio - back.organic_ratio).abs() < 1e-9);
    }
}
