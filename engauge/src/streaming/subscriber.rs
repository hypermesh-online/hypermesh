// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! MetricsSubscriber — receives [`MetricsFrame`]s from peers and maintains
//! per-source rolling windows for downstream analysis.

use std::collections::{HashMap, VecDeque};

use super::protocol::{MetricsFrame, MetricsPayload};

// ---------------------------------------------------------------------------
// MetricsSubscriber
// ---------------------------------------------------------------------------

/// Receives [`MetricsFrame`]s and stores them in per-source rolling windows.
///
/// The window size is bounded by `max_window_size`; when a new frame exceeds
/// the limit, the oldest frame for that source is discarded.
pub struct MetricsSubscriber {
    /// Per-source-node rolling window of received frames.
    windows: HashMap<String, VecDeque<MetricsFrame>>,
    /// Maximum frames retained per source.
    max_window_size: usize,
}

impl MetricsSubscriber {
    /// Create a subscriber with the given per-source window capacity.
    pub fn new(max_window_size: usize) -> Self {
        let max_window_size = if max_window_size == 0 { 1 } else { max_window_size };
        Self {
            windows: HashMap::new(),
            max_window_size,
        }
    }

    /// Ingest a frame, storing it in the appropriate source window.
    ///
    /// Automatically prunes the oldest frame when the window exceeds capacity.
    pub fn receive(&mut self, frame: MetricsFrame) {
        let key = frame.source_node.0.clone();
        let window = self.windows.entry(key).or_insert_with(VecDeque::new);
        window.push_back(frame);
        while window.len() > self.max_window_size {
            window.pop_front();
        }
    }

    /// Most recent frame from a given source, or `None` if not tracked.
    pub fn latest(&self, source_node: &str) -> Option<&MetricsFrame> {
        self.windows
            .get(source_node)
            .and_then(|w| w.back())
    }

    /// Full rolling window for a given source.
    pub fn window(&self, source_node: &str) -> Option<&VecDeque<MetricsFrame>> {
        self.windows.get(source_node)
    }

    /// Number of distinct source nodes currently tracked.
    pub fn source_count(&self) -> usize {
        self.windows.len()
    }

    /// Average capacity score across all Capacity payloads in a source's window.
    ///
    /// Only considers frames that carry a [`MetricsPayload::Capacity`] variant.
    /// Returns `None` if the source has no Capacity frames.
    pub fn avg_capacity_score(&self, source_node: &str) -> Option<f64> {
        let window = self.windows.get(source_node)?;
        let mut sum = 0.0_f64;
        let mut count = 0_usize;

        for frame in window.iter() {
            if let MetricsPayload::Capacity(ref cap) = frame.payload {
                // Compute a simple normalized score from the snapshot.
                // Mirrors CapacityScore weights: bytes 0.25, compute 0.25,
                // storage 0.20, bandwidth 0.20, uptime 0.10.
                let bytes_norm = (cap.bytes_served as f64 / 1_073_741_824.0)
                    .clamp(0.0, 1.0);
                let compute_norm = (cap.compute_delivered as f64 / 1_000_000.0)
                    .clamp(0.0, 1.0);
                let storage_norm = (cap.storage_maintained_bytes as f64
                    / 10_737_418_240.0)
                    .clamp(0.0, 1.0);
                let bw_norm = (cap.bandwidth_available_bps as f64
                    / 1_000_000_000.0)
                    .clamp(0.0, 1.0);
                let uptime = cap.uptime_ratio.clamp(0.0, 1.0);

                let score = bytes_norm * 0.25
                    + compute_norm * 0.25
                    + storage_norm * 0.20
                    + bw_norm * 0.20
                    + uptime * 0.10;

                sum += score;
                count += 1;
            }
        }

        if count == 0 {
            return None;
        }
        Some(sum / count as f64)
    }

    /// All tracked source node identifiers.
    pub fn sources(&self) -> Vec<&String> {
        self.windows.keys().collect()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::protocol::{
        CapacitySnapshot, CongestionSnapshot, MetricsPayload,
    };
    use hypermesh_lib::{NodeId, PrivacyMode};

    fn cap_frame(node: &str, seq: u64, bytes: u64) -> MetricsFrame {
        MetricsFrame {
            source_node: NodeId::from(node),
            timestamp_us: 1_000_000 + seq,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Capacity(CapacitySnapshot {
                bytes_served: bytes,
                compute_delivered: 500_000,
                storage_maintained_bytes: 5_000_000_000,
                bandwidth_available_bps: 500_000_000,
                uptime_ratio: 0.95,
            }),
            sequence: seq,
        }
    }

    fn congestion_frame(node: &str, seq: u64) -> MetricsFrame {
        MetricsFrame {
            source_node: NodeId::from(node),
            timestamp_us: 1_000_000 + seq,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Congestion(CongestionSnapshot {
                buffer_fullness_ratio: 0.6,
                queue_depth: 10,
                dropped_packets_epoch: 2,
                avg_queue_wait_us: 80,
            }),
            sequence: seq,
        }
    }

    #[test]
    fn receive_and_retrieve_latest() {
        let mut sub = MetricsSubscriber::new(10);
        sub.receive(cap_frame("node-a", 0, 100));
        sub.receive(cap_frame("node-a", 1, 200));

        let latest = sub.latest("node-a").expect("test: latest should exist");
        assert_eq!(latest.sequence, 1);
    }

    #[test]
    fn window_pruning_at_max_size() {
        let mut sub = MetricsSubscriber::new(3);
        for i in 0..5 {
            sub.receive(cap_frame("pruner", i, 100 * (i + 1)));
        }
        let window = sub.window("pruner").expect("test: window should exist");
        assert_eq!(window.len(), 3, "window must be bounded at max_window_size");
        // Oldest surviving frame should be sequence 2 (0 and 1 pruned).
        assert_eq!(window.front().expect("test: front").sequence, 2);
    }

    #[test]
    fn multiple_sources_tracked_independently() {
        let mut sub = MetricsSubscriber::new(10);
        sub.receive(cap_frame("alpha", 0, 100));
        sub.receive(cap_frame("alpha", 1, 200));
        sub.receive(cap_frame("beta", 0, 300));

        assert_eq!(sub.source_count(), 2);

        let alpha_latest = sub.latest("alpha").expect("test: alpha latest");
        assert_eq!(alpha_latest.sequence, 1);

        let beta_latest = sub.latest("beta").expect("test: beta latest");
        assert_eq!(beta_latest.sequence, 0);
    }

    #[test]
    fn avg_capacity_score_computation() {
        let mut sub = MetricsSubscriber::new(10);

        // Full-baseline capacity: score should be ~1.0
        sub.receive(MetricsFrame {
            source_node: NodeId::from("full-node"),
            timestamp_us: 1_000_000,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Capacity(CapacitySnapshot {
                bytes_served: 1_073_741_824,
                compute_delivered: 1_000_000,
                storage_maintained_bytes: 10_737_418_240,
                bandwidth_available_bps: 1_000_000_000,
                uptime_ratio: 1.0,
            }),
            sequence: 0,
        });

        let score = sub
            .avg_capacity_score("full-node")
            .expect("test: score should exist");
        assert!(
            (score - 1.0).abs() < 1e-6,
            "full-baseline capacity should yield score ~1.0, got {score}"
        );
    }

    #[test]
    fn avg_capacity_score_ignores_non_capacity_frames() {
        let mut sub = MetricsSubscriber::new(10);
        sub.receive(congestion_frame("cong-only", 0));

        assert!(
            sub.avg_capacity_score("cong-only").is_none(),
            "no Capacity frames means avg_capacity_score should be None"
        );
    }

    #[test]
    fn empty_subscriber_returns_none() {
        let sub = MetricsSubscriber::new(10);
        assert!(sub.latest("ghost").is_none());
        assert!(sub.window("ghost").is_none());
        assert!(sub.avg_capacity_score("ghost").is_none());
        assert_eq!(sub.source_count(), 0);
    }
}
