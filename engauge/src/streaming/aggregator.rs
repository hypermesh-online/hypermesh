// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! RegionalAggregator — combines multi-node metrics for routing decisions.
//!
//! Wraps a [`MetricsSubscriber`] and exposes aggregation queries that fold
//! latest frames from multiple sources into a single [`RegionalAggregate`].

use hypermesh_lib::NodeId;

use super::protocol::{MetricsFrame, MetricsPayload};
use super::subscriber::MetricsSubscriber;

// ---------------------------------------------------------------------------
// RegionalAggregate
// ---------------------------------------------------------------------------

/// Aggregated metrics across multiple source nodes in a region.
#[derive(Debug, Clone)]
pub struct RegionalAggregate {
    /// Number of source nodes contributing to this aggregate.
    pub node_count: usize,
    /// Mean buffer fullness ratio across sources (from Congestion payloads).
    pub avg_buffer_fullness: f64,
    /// Mean latency in microseconds across sources (from Routing payloads).
    pub avg_latency_us: f64,
    /// Mean throughput in bits per second across sources (from Routing payloads).
    pub avg_throughput_bps: f64,
    /// Sum of available bandwidth across sources (from Capacity payloads).
    pub total_bandwidth_bps: u64,
    /// Mean capacity score across sources (from Capacity payloads).
    pub avg_capacity_score: f64,
    /// Number of nodes passing PoSPing verification in this region.
    pub verified_node_count: usize,
    /// Average consistency ratio across verified nodes.
    pub avg_consistency_ratio: f64,
}

impl RegionalAggregate {
    /// An empty aggregate with zero values.
    fn empty() -> Self {
        Self {
            node_count: 0,
            avg_buffer_fullness: 0.0,
            avg_latency_us: 0.0,
            avg_throughput_bps: 0.0,
            total_bandwidth_bps: 0,
            avg_capacity_score: 0.0,
            verified_node_count: 0,
            avg_consistency_ratio: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// RegionalAggregator
// ---------------------------------------------------------------------------

/// Combines per-node metrics into regional aggregates for routing intelligence.
pub struct RegionalAggregator {
    subscriber: MetricsSubscriber,
    /// Total frames ingested since creation.
    ingest_counter: u64,
    /// Value of `ingest_counter` at the last successful `aggregate_periodic` call.
    last_aggregate_ingest: u64,
}

impl RegionalAggregator {
    /// Create an aggregator with the given per-source window size.
    pub fn new(max_window_size: usize) -> Self {
        Self {
            subscriber: MetricsSubscriber::new(max_window_size),
            ingest_counter: 0,
            last_aggregate_ingest: 0,
        }
    }

    /// Ingest a frame (delegates to the underlying subscriber).
    pub fn ingest(&mut self, frame: MetricsFrame) {
        self.subscriber.receive(frame);
        self.ingest_counter += 1;
    }

    /// Return an aggregate only if new data has been ingested since the last call.
    ///
    /// Returns `None` when no new frames have arrived, avoiding redundant
    /// aggregation work in periodic loops.
    pub fn aggregate_periodic(&mut self) -> Option<RegionalAggregate> {
        if self.ingest_counter == self.last_aggregate_ingest {
            return None;
        }
        self.last_aggregate_ingest = self.ingest_counter;
        Some(self.aggregate())
    }

    /// Aggregate latest frames across **all** tracked sources.
    pub fn aggregate(&self) -> RegionalAggregate {
        let sources: Vec<NodeId> = self.subscriber.sources().into_iter().copied().collect();
        self.aggregate_for_sources(&sources)
    }

    /// Aggregate latest frames for a specific set of source node IDs.
    pub fn aggregate_for_sources(&self, sources: &[NodeId]) -> RegionalAggregate {
        if sources.is_empty() {
            return RegionalAggregate::empty();
        }

        let frames: Vec<&MetricsFrame> = sources
            .iter()
            .filter_map(|s| self.subscriber.latest(s))
            .collect();

        if frames.is_empty() {
            return RegionalAggregate::empty();
        }

        let accum = accumulate_frames(&frames);

        RegionalAggregate {
            node_count: frames.len(),
            avg_buffer_fullness: safe_avg(accum.buf_sum, accum.buf_count),
            avg_latency_us: safe_avg(accum.lat_sum, accum.lat_count),
            avg_throughput_bps: safe_avg(accum.thr_sum, accum.thr_count),
            total_bandwidth_bps: accum.bw_total,
            avg_capacity_score: safe_avg(accum.cap_sum, accum.cap_count),
            verified_node_count: accum.verif_count,
            avg_consistency_ratio: safe_avg(accum.verif_sum, accum.verif_count),
        }
    }

    /// Number of distinct source nodes in the underlying subscriber.
    pub fn source_count(&self) -> usize {
        self.subscriber.source_count()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Running totals collected while iterating over metrics frames.
struct FrameAccumulator {
    buf_sum: f64,
    buf_count: usize,
    lat_sum: f64,
    lat_count: usize,
    thr_sum: f64,
    thr_count: usize,
    bw_total: u64,
    cap_sum: f64,
    cap_count: usize,
    verif_sum: f64,
    verif_count: usize,
}

/// Accumulate metric values from a slice of frames into running totals.
fn accumulate_frames(frames: &[&MetricsFrame]) -> FrameAccumulator {
    let mut acc = FrameAccumulator {
        buf_sum: 0.0,
        buf_count: 0,
        lat_sum: 0.0,
        lat_count: 0,
        thr_sum: 0.0,
        thr_count: 0,
        bw_total: 0,
        cap_sum: 0.0,
        cap_count: 0,
        verif_sum: 0.0,
        verif_count: 0,
    };

    for frame in frames {
        match &frame.payload {
            MetricsPayload::Capacity(c) => {
                acc.bw_total = acc.bw_total.saturating_add(c.bandwidth_available_bps);
                acc.cap_sum += capacity_score_from_snapshot(c);
                acc.cap_count += 1;
            }
            MetricsPayload::Congestion(c) => {
                acc.buf_sum += c.buffer_fullness_ratio;
                acc.buf_count += 1;
            }
            MetricsPayload::Routing(r) => {
                acc.lat_sum += r.avg_latency_us as f64;
                acc.lat_count += 1;
                acc.thr_sum += r.throughput_bps as f64;
                acc.thr_count += 1;
            }
            MetricsPayload::Economic(_) => {}
            MetricsPayload::Verification(v) => {
                acc.verif_sum += v.consistency_ratio;
                acc.verif_count += 1;
            }
        }
    }

    acc
}

/// Compute a capacity score from a snapshot using the same weights as
/// [`crate::capacity::CapacityScore`].
fn capacity_score_from_snapshot(c: &super::protocol::CapacitySnapshot) -> f64 {
    let bytes_norm = (c.bytes_served as f64 / 1_073_741_824.0).clamp(0.0, 1.0);
    let compute_norm = (c.compute_delivered as f64 / 1_000_000.0).clamp(0.0, 1.0);
    let storage_norm = (c.storage_maintained_bytes as f64 / 10_737_418_240.0).clamp(0.0, 1.0);
    let bw_norm = (c.bandwidth_available_bps as f64 / 1_000_000_000.0).clamp(0.0, 1.0);
    let uptime = c.uptime_ratio.clamp(0.0, 1.0);

    bytes_norm * 0.25 + compute_norm * 0.25 + storage_norm * 0.20 + bw_norm * 0.20 + uptime * 0.10
}

fn safe_avg(sum: f64, count: usize) -> f64 {
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::protocol::{
        CapacitySnapshot, CongestionSnapshot, MetricsPayload, RoutingSnapshot,
    };
    use hypermesh_lib::{NodeId, PrivacyMode};

    fn cap_frame(node: &str, bytes: u64, bw: u64) -> MetricsFrame {
        MetricsFrame {
            source_node: NodeId::from_public_key(node.as_bytes()),
            timestamp_us: 1_000_000,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Capacity(CapacitySnapshot {
                bytes_served: bytes,
                compute_delivered: 500_000,
                storage_maintained_bytes: 5_000_000_000,
                bandwidth_available_bps: bw,
                uptime_ratio: 0.95,
            }),
            sequence: 0,
        }
    }

    fn routing_frame(node: &str, latency: u64, throughput: u64) -> MetricsFrame {
        MetricsFrame {
            source_node: NodeId::from_public_key(node.as_bytes()),
            timestamp_us: 1_000_000,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Routing(RoutingSnapshot {
                avg_latency_us: latency,
                throughput_bps: throughput,
                path_count: 2,
                active_connections: 5,
            }),
            sequence: 0,
        }
    }

    fn congestion_frame(node: &str, fullness: f64) -> MetricsFrame {
        MetricsFrame {
            source_node: NodeId::from_public_key(node.as_bytes()),
            timestamp_us: 1_000_000,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Congestion(CongestionSnapshot {
                buffer_fullness_ratio: fullness,
                queue_depth: 10,
                dropped_packets_epoch: 1,
                avg_queue_wait_us: 50,
            }),
            sequence: 0,
        }
    }

    #[test]
    fn aggregate_with_no_data_returns_zeros() {
        let agg = RegionalAggregator::new(10);
        let result = agg.aggregate();
        assert_eq!(result.node_count, 0);
        assert!((result.avg_buffer_fullness).abs() < 1e-9);
        assert!((result.avg_latency_us).abs() < 1e-9);
        assert!((result.avg_throughput_bps).abs() < 1e-9);
        assert_eq!(result.total_bandwidth_bps, 0);
        assert!((result.avg_capacity_score).abs() < 1e-9);
    }

    #[test]
    fn aggregate_single_source() {
        let mut agg = RegionalAggregator::new(10);
        agg.ingest(cap_frame("solo", 1_073_741_824, 1_000_000_000));

        let result = agg.aggregate();
        assert_eq!(result.node_count, 1);
        assert_eq!(result.total_bandwidth_bps, 1_000_000_000);
        // Full baseline bytes + half compute + ~half storage + full bw + 0.95 uptime.
        assert!(
            result.avg_capacity_score > 0.5,
            "single full-ish node should score > 0.5, got {}",
            result.avg_capacity_score
        );
    }

    #[test]
    fn aggregate_multiple_sources() {
        let mut agg = RegionalAggregator::new(10);
        agg.ingest(congestion_frame("node-a", 0.4));
        agg.ingest(congestion_frame("node-b", 0.8));

        let result = agg.aggregate();
        assert_eq!(result.node_count, 2);
        // Average buffer fullness: (0.4 + 0.8) / 2 = 0.6
        assert!(
            (result.avg_buffer_fullness - 0.6).abs() < 1e-9,
            "expected avg_buffer_fullness ~0.6, got {}",
            result.avg_buffer_fullness
        );
    }

    #[test]
    fn aggregate_for_sources_filters_correctly() {
        let mut agg = RegionalAggregator::new(10);
        agg.ingest(routing_frame("fast", 1000, 500_000_000));
        agg.ingest(routing_frame("slow", 10_000, 100_000_000));
        agg.ingest(routing_frame("medium", 5000, 250_000_000));

        // Only aggregate fast + medium (exclude slow).
        let fast = NodeId::from_public_key(b"fast");
        let medium = NodeId::from_public_key(b"medium");
        let subset = vec![fast, medium];
        let result = agg.aggregate_for_sources(&subset);

        assert_eq!(result.node_count, 2);
        // avg latency: (1000 + 5000) / 2 = 3000
        assert!(
            (result.avg_latency_us - 3000.0).abs() < 1e-9,
            "expected avg_latency_us ~3000, got {}",
            result.avg_latency_us
        );
        // avg throughput: (500M + 250M) / 2 = 375M
        assert!(
            (result.avg_throughput_bps - 375_000_000.0).abs() < 1e-9,
            "expected avg_throughput_bps ~375M, got {}",
            result.avg_throughput_bps
        );
    }

    #[test]
    fn aggregate_for_unknown_sources_returns_empty() {
        let agg = RegionalAggregator::new(10);
        let ghost = NodeId::from_public_key(b"ghost");
        let result = agg.aggregate_for_sources(&[ghost]);
        assert_eq!(result.node_count, 0);
    }

    #[test]
    fn aggregate_verification_payloads() {
        use crate::streaming::protocol::VerificationSnapshot;

        let mut agg = RegionalAggregator::new(10);
        agg.ingest(MetricsFrame {
            source_node: NodeId::from_public_key(b"verif-a"),
            timestamp_us: 1_000_000,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Verification(VerificationSnapshot {
                probes_sent: 100,
                probes_passed: 90,
                avg_response_time_us: 1000,
                consistency_ratio: 0.9,
                epoch: 1,
            }),
            sequence: 0,
        });
        agg.ingest(MetricsFrame {
            source_node: NodeId::from_public_key(b"verif-b"),
            timestamp_us: 1_000_000,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Verification(VerificationSnapshot {
                probes_sent: 200,
                probes_passed: 196,
                avg_response_time_us: 800,
                consistency_ratio: 0.98,
                epoch: 1,
            }),
            sequence: 0,
        });

        let result = agg.aggregate();
        assert_eq!(result.node_count, 2);
        assert_eq!(result.verified_node_count, 2);
        // avg consistency: (0.9 + 0.98) / 2 = 0.94
        assert!(
            (result.avg_consistency_ratio - 0.94).abs() < 1e-9,
            "expected avg_consistency_ratio ~0.94, got {}",
            result.avg_consistency_ratio
        );
    }
}
