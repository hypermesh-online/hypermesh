// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Differential privacy filter using Laplace noise injection.
//!
//! The [`DifferentialPrivacyFilter`] controls which metrics payloads are
//! shared and injects calibrated Laplace noise into numeric fields before
//! they leave the local node.
//!
//! Privacy behavior per [`PrivacyMode`]:
//! - **Anonymous** -- no metrics shared (returns `None`).
//! - **Private** -- Capacity, Congestion, and Verification only, with noise.
//! - **Public** -- all payloads passed through with noise.

use hypermesh_lib::PrivacyMode;
use rand::Rng;

use super::protocol::{
    CapacitySnapshot, CongestionSnapshot, EconomicSnapshot, MetricsFrame,
    MetricsPayload, RoutingSnapshot, VerificationSnapshot,
};

// ---------------------------------------------------------------------------
// DifferentialPrivacyFilter
// ---------------------------------------------------------------------------

/// Applies differential privacy guarantees to outbound metrics frames.
///
/// Uses Laplace noise calibrated to `sensitivity / epsilon`. A smaller
/// `epsilon` provides stronger privacy but more noise.
pub struct DifferentialPrivacyFilter {
    /// Privacy budget (default 1.0, lower = more private).
    epsilon: f64,
}

impl DifferentialPrivacyFilter {
    /// Create a new filter with the given privacy budget.
    ///
    /// # Panics
    /// Panics if `epsilon` is not positive.
    pub fn new(epsilon: f64) -> Self {
        assert!(epsilon > 0.0, "epsilon must be positive, got {epsilon}");
        Self { epsilon }
    }

    /// Returns the configured epsilon.
    pub fn epsilon(&self) -> f64 {
        self.epsilon
    }

    /// Filter a frame according to the source node's privacy mode.
    ///
    /// - Anonymous: returns `None` (no metrics shared).
    /// - Private: passes Capacity and Congestion only, with Laplace noise.
    /// - Public: passes all payloads with Laplace noise.
    pub fn filter_frame(&self, frame: MetricsFrame) -> Option<MetricsFrame> {
        if frame.privacy_mode == PrivacyMode::ANONYMOUS {
            return None;
        }

        let is_private = frame.privacy_mode == PrivacyMode::PRIVATE;

        let noised_payload = match frame.payload {
            MetricsPayload::Capacity(c) => {
                MetricsPayload::Capacity(self.noise_capacity(c))
            }
            MetricsPayload::Congestion(c) => {
                MetricsPayload::Congestion(self.noise_congestion(c))
            }
            MetricsPayload::Routing(r) => {
                if is_private {
                    return None;
                }
                MetricsPayload::Routing(self.noise_routing(r))
            }
            MetricsPayload::Economic(e) => {
                if is_private {
                    return None;
                }
                MetricsPayload::Economic(self.noise_economic(e))
            }
            MetricsPayload::Verification(v) => {
                // Private and Public both receive Verification payloads.
                // Anonymous was already filtered out above.
                MetricsPayload::Verification(self.noise_verification(v))
            }
        };

        Some(MetricsFrame {
            source_node: frame.source_node,
            timestamp_us: frame.timestamp_us,
            privacy_mode: frame.privacy_mode,
            payload: noised_payload,
            sequence: frame.sequence,
        })
    }

    /// Add Laplace noise to a floating-point value.
    ///
    /// The noise is drawn from Laplace(0, sensitivity / epsilon) using the
    /// inverse-CDF method: `noise = -b * sign(u) * ln(1 - 2|u|)` where
    /// `u ~ Uniform(-0.5, 0.5)` and `b = sensitivity / epsilon`.
    pub fn add_laplace_noise(&self, value: f64, sensitivity: f64) -> f64 {
        let b = sensitivity / self.epsilon;
        let mut rng = rand::thread_rng();
        let u: f64 = rng.gen_range(-0.5_f64..0.5_f64);
        let noise = -b * u.signum() * (1.0 - 2.0 * u.abs()).ln();
        value + noise
    }

    // -- internal per-payload noise methods --------------------------------

    fn noise_capacity(&self, mut c: CapacitySnapshot) -> CapacitySnapshot {
        c.bytes_served = self.noise_u64(c.bytes_served, 1_000_000.0);
        c.compute_delivered = self.noise_u64(c.compute_delivered, 10_000.0);
        c.storage_maintained_bytes = self.noise_u64(
            c.storage_maintained_bytes,
            1_000_000.0,
        );
        c.bandwidth_available_bps = self.noise_u64(
            c.bandwidth_available_bps,
            1_000_000.0,
        );
        c.uptime_ratio = self
            .add_laplace_noise(c.uptime_ratio, 0.01)
            .clamp(0.0, 1.0);
        c
    }

    fn noise_congestion(&self, mut c: CongestionSnapshot) -> CongestionSnapshot {
        c.buffer_fullness_ratio = self
            .add_laplace_noise(c.buffer_fullness_ratio, 0.05)
            .clamp(0.0, 1.0);
        c.queue_depth = self.noise_u32(c.queue_depth, 5.0);
        c.dropped_packets_epoch = self.noise_u64(c.dropped_packets_epoch, 10.0);
        c.avg_queue_wait_us = self.noise_u64(c.avg_queue_wait_us, 50.0);
        c
    }

    fn noise_routing(&self, mut r: RoutingSnapshot) -> RoutingSnapshot {
        r.avg_latency_us = self.noise_u64(r.avg_latency_us, 100.0);
        r.throughput_bps = self.noise_u64(r.throughput_bps, 1_000_000.0);
        // path_count and active_connections are small integers; add small noise.
        r.path_count = self.noise_u16(r.path_count, 1.0);
        r.active_connections = self.noise_u32(r.active_connections, 2.0);
        r
    }

    fn noise_economic(&self, mut e: EconomicSnapshot) -> EconomicSnapshot {
        e.in_flight_float_grams = self
            .add_laplace_noise(e.in_flight_float_grams, 1.0)
            .max(0.0);
        e.settlement_rate_per_epoch = self
            .add_laplace_noise(e.settlement_rate_per_epoch, 0.5)
            .max(0.0);
        e.active_packets = self.noise_u32(e.active_packets, 1.0);
        e
    }

    fn noise_verification(&self, mut v: VerificationSnapshot) -> VerificationSnapshot {
        v.probes_sent = self.noise_u32(v.probes_sent, 1.0);
        v.probes_passed = self.noise_u32(v.probes_passed, 1.0);
        v.avg_response_time_us = self.noise_u64(v.avg_response_time_us, 100.0);
        v.consistency_ratio = self
            .add_laplace_noise(v.consistency_ratio, 0.01)
            .clamp(0.0, 1.0);
        // epoch is not noised — it is a logical identifier, not a measurement.
        v
    }

    // -- integer noise helpers ---------------------------------------------

    fn noise_u64(&self, value: u64, sensitivity: f64) -> u64 {
        let noised = self.add_laplace_noise(value as f64, sensitivity);
        noised.round().max(0.0) as u64
    }

    fn noise_u32(&self, value: u32, sensitivity: f64) -> u32 {
        let noised = self.add_laplace_noise(value as f64, sensitivity);
        noised.round().clamp(0.0, u32::MAX as f64) as u32
    }

    fn noise_u16(&self, value: u16, sensitivity: f64) -> u16 {
        let noised = self.add_laplace_noise(value as f64, sensitivity);
        noised.round().clamp(0.0, u16::MAX as f64) as u16
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::NodeId;

    fn make_frame(
        privacy: PrivacyMode,
        payload: MetricsPayload,
    ) -> MetricsFrame {
        MetricsFrame {
            source_node: NodeId::from("filter-test-node"),
            timestamp_us: 1_700_000_000_000_000,
            privacy_mode: privacy,
            payload,
            sequence: 1,
        }
    }

    fn sample_capacity() -> MetricsPayload {
        MetricsPayload::Capacity(CapacitySnapshot {
            bytes_served: 1_000_000,
            compute_delivered: 50_000,
            storage_maintained_bytes: 5_000_000,
            bandwidth_available_bps: 500_000_000,
            uptime_ratio: 0.95,
        })
    }

    fn sample_congestion() -> MetricsPayload {
        MetricsPayload::Congestion(CongestionSnapshot {
            buffer_fullness_ratio: 0.5,
            queue_depth: 20,
            dropped_packets_epoch: 5,
            avg_queue_wait_us: 100,
        })
    }

    fn sample_economic() -> MetricsPayload {
        MetricsPayload::Economic(EconomicSnapshot {
            in_flight_float_grams: 10.0,
            settlement_rate_per_epoch: 5.0,
            active_packets: 3,
        })
    }

    fn sample_routing() -> MetricsPayload {
        MetricsPayload::Routing(RoutingSnapshot {
            avg_latency_us: 5000,
            throughput_bps: 100_000_000,
            path_count: 4,
            active_connections: 10,
        })
    }

    fn sample_verification() -> MetricsPayload {
        MetricsPayload::Verification(VerificationSnapshot {
            probes_sent: 100,
            probes_passed: 95,
            avg_response_time_us: 1200,
            consistency_ratio: 0.95,
            epoch: 42,
        })
    }

    #[test]
    fn anonymous_returns_none() {
        let filter = DifferentialPrivacyFilter::new(1.0);
        let frame = make_frame(PrivacyMode::ANONYMOUS, sample_capacity());
        assert!(
            filter.filter_frame(frame).is_none(),
            "Anonymous must suppress all metrics"
        );
    }

    #[test]
    fn private_filters_out_economic() {
        let filter = DifferentialPrivacyFilter::new(1.0);
        let frame = make_frame(PrivacyMode::PRIVATE, sample_economic());
        assert!(
            filter.filter_frame(frame).is_none(),
            "Private must suppress Economic payload"
        );
    }

    #[test]
    fn private_filters_out_routing() {
        let filter = DifferentialPrivacyFilter::new(1.0);
        let frame = make_frame(PrivacyMode::PRIVATE, sample_routing());
        assert!(
            filter.filter_frame(frame).is_none(),
            "Private must suppress Routing payload"
        );
    }

    #[test]
    fn private_passes_capacity_and_congestion() {
        let filter = DifferentialPrivacyFilter::new(1.0);

        let cap = make_frame(PrivacyMode::PRIVATE, sample_capacity());
        assert!(
            filter.filter_frame(cap).is_some(),
            "Private must pass Capacity"
        );

        let cong = make_frame(PrivacyMode::PRIVATE, sample_congestion());
        assert!(
            filter.filter_frame(cong).is_some(),
            "Private must pass Congestion"
        );
    }

    #[test]
    fn public_passes_all_payloads() {
        let filter = DifferentialPrivacyFilter::new(1.0);

        let payloads = vec![
            sample_capacity(),
            sample_congestion(),
            sample_routing(),
            sample_economic(),
            sample_verification(),
        ];

        for payload in payloads {
            let frame = make_frame(PrivacyMode::PUBLIC, payload);
            assert!(
                filter.filter_frame(frame).is_some(),
                "Public must pass all payload variants"
            );
        }
    }

    #[test]
    fn anonymous_suppresses_verification() {
        let filter = DifferentialPrivacyFilter::new(1.0);
        let frame = make_frame(PrivacyMode::ANONYMOUS, sample_verification());
        assert!(
            filter.filter_frame(frame).is_none(),
            "Anonymous must suppress Verification payload"
        );
    }

    #[test]
    fn private_passes_verification() {
        let filter = DifferentialPrivacyFilter::new(1.0);
        let frame = make_frame(PrivacyMode::PRIVATE, sample_verification());
        assert!(
            filter.filter_frame(frame).is_some(),
            "Private must pass Verification payload"
        );
    }

    #[test]
    fn public_passes_verification() {
        let filter = DifferentialPrivacyFilter::new(1.0);
        let frame = make_frame(PrivacyMode::PUBLIC, sample_verification());
        assert!(
            filter.filter_frame(frame).is_some(),
            "Public must pass Verification payload"
        );
    }

    #[test]
    fn noise_adds_randomness() {
        let filter = DifferentialPrivacyFilter::new(1.0);
        let base_value = 1000.0;
        let sensitivity = 100.0;
        let iterations = 200;

        let mut values: Vec<f64> = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            values.push(filter.add_laplace_noise(base_value, sensitivity));
        }

        // At least some values should differ from the base (noise is non-zero).
        let distinct_count = values
            .iter()
            .filter(|v| (**v - base_value).abs() > 1e-9)
            .count();

        assert!(
            distinct_count > iterations / 2,
            "Expected majority of noised values to differ from base; \
             only {distinct_count}/{iterations} differed"
        );

        // Standard deviation should be roughly sensitivity/epsilon = 100.
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
            / values.len() as f64;
        let stddev = variance.sqrt();

        // Laplace std dev = b * sqrt(2) = (100/1) * 1.414 ~ 141.
        // Allow generous bounds for statistical test.
        assert!(
            stddev > 30.0 && stddev < 500.0,
            "stddev {stddev} outside expected range for Laplace(0, 100)"
        );
    }
}
