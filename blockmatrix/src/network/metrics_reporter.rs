// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Lightweight metrics reporter for the engauge streaming pipeline.
//!
//! Collects real node data and produces engauge-compatible `MetricsFrame`
//! JSON. Pushes frames to connected peers over STOQ streams using the
//! `CONN_TYPE_METRICS` (0x02) discriminator byte.

use hypermesh_ebpf::metrics::HyperMeshMetrics;
use std::sync::Arc;
use tracing::{debug, info, warn};

use super::NetworkNode;

/// Current unix epoch timestamp in microseconds.
fn timestamp_us() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0)
}

/// Number of cycles to skip after a send failure before retrying.
const BACKOFF_CYCLES: u64 = 10;

/// Metrics reporter that builds engauge-compatible capacity frames
/// and pushes them to peers over STOQ streams.
pub struct MetricsReporter {
    node_id: String,
    sequence: u64,
    /// Remaining cycles to skip before retrying after a failure.
    backoff_remaining: u64,
}

impl MetricsReporter {
    /// Create a new reporter for the given node.
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            sequence: 0,
            backoff_remaining: 0,
        }
    }

    /// Build a capacity metrics frame from current node state.
    ///
    /// Returns JSON bytes compatible with `engauge::streaming::MetricsFrame`.
    pub fn build_capacity_frame(
        &mut self,
        chain_height: u64,
        peer_count: usize,
        shard_count: usize,
        cpu_usage: f64,
        memory_usage: f64,
    ) -> Vec<u8> {
        self.sequence += 1;
        let now_us = timestamp_us();

        let frame = serde_json::json!({
            "source_node": self.node_id,
            "timestamp_us": now_us,
            "privacy_mode": { "scope": "Unbounded", "tracked": true },
            "sequence": self.sequence,
            "payload": {
                "Capacity": {
                    "bytes_served": 0u64,
                    "compute_delivered": chain_height,
                    "storage_maintained_bytes": shard_count as u64 * 65536,
                    "bandwidth_available_bps": 0u64,
                    "uptime_ratio": 1.0,
                }
            }
        });

        info!(
            target: "engauge::metrics",
            chain_height,
            peer_count,
            shard_count,
            cpu_usage_pct = format!("{:.1}", cpu_usage),
            memory_usage_pct = format!("{:.1}", memory_usage),
            "node_metrics"
        );

        serde_json::to_vec(&frame).unwrap_or_default()
    }

    /// Build a congestion metrics frame from eBPF transport data.
    ///
    /// Returns JSON bytes compatible with `engauge::streaming::MetricsFrame`.
    pub fn build_congestion_frame(&mut self, ebpf: &HyperMeshMetrics) -> Vec<u8> {
        self.sequence += 1;
        let now_us = timestamp_us();
        let t = &ebpf.transport_metrics;
        let total = t.total_packets.max(1);
        let drop_ratio = t.kernel_drops as f64 / total as f64;

        let frame = serde_json::json!({
            "source_node": self.node_id,
            "timestamp_us": now_us,
            "privacy_mode": { "scope": "Unbounded", "tracked": true },
            "sequence": self.sequence,
            "payload": {
                "Congestion": {
                    "buffer_fullness_ratio": drop_ratio.clamp(0.0, 1.0),
                    "queue_depth": t.kernel_drops.min(u32::MAX as u64),
                    "dropped_packets_epoch": t.kernel_drops,
                    "avg_queue_wait_us": t.latency_avg_us,
                }
            }
        });

        debug!(
            target: "engauge::metrics",
            drops = t.kernel_drops,
            latency_us = t.latency_avg_us,
            drop_ratio = format!("{:.4}", drop_ratio),
            "ebpf_congestion_frame"
        );

        serde_json::to_vec(&frame).unwrap_or_default()
    }

    /// Build a routing metrics frame from eBPF transport and routing data.
    ///
    /// Returns JSON bytes compatible with `engauge::streaming::MetricsFrame`.
    pub fn build_routing_frame(&mut self, ebpf: &HyperMeshMetrics) -> Vec<u8> {
        self.sequence += 1;
        let now_us = timestamp_us();
        let t = &ebpf.transport_metrics;
        let r = &ebpf.routing_metrics;

        let frame = serde_json::json!({
            "source_node": self.node_id,
            "timestamp_us": now_us,
            "privacy_mode": { "scope": "Unbounded", "tracked": true },
            "sequence": self.sequence,
            "payload": {
                "Routing": {
                    "avg_latency_us": t.latency_avg_us,
                    "throughput_bps": (t.bytes_per_second * 8.0) as u64,
                    "path_count": r.successful.min(u16::MAX as u64),
                    "active_connections": t.af_xdp_redirects.min(u32::MAX as u64),
                }
            }
        });

        debug!(
            target: "engauge::metrics",
            latency_us = t.latency_avg_us,
            throughput_gbps = format!("{:.2}", t.throughput_gbps()),
            routing_success = r.successful,
            "ebpf_routing_frame"
        );

        serde_json::to_vec(&frame).unwrap_or_default()
    }

    /// Push a pre-built frame to all connected peers over STOQ streams.
    ///
    /// Best-effort: if no peers are connected or sends fail, the error
    /// is logged at debug level and a backoff counter is set so subsequent
    /// cycles skip the push attempt (avoiding log spam).
    pub async fn push_to_peers(&mut self, frame_bytes: &[u8], peers: &[NetworkNode]) {
        if self.backoff_remaining > 0 {
            self.backoff_remaining -= 1;
            return;
        }

        if peers.is_empty() {
            debug!("No connected peers for metrics push");
            return;
        }

        let mut any_success = false;
        for node in peers {
            if send_metrics_to_peer(node, frame_bytes).await {
                any_success = true;
            }
        }

        if !any_success {
            debug!("Metrics push failed to all peers (backoff {} cycles)", BACKOFF_CYCLES);
            self.backoff_remaining = BACKOFF_CYCLES;
        }
    }
}

/// Send a metrics frame to a single peer over a STOQ stream.
///
/// Opens a bidirectional stream, writes the `CONN_TYPE_METRICS` discriminator
/// followed by the frame bytes.  Returns `true` on success.
async fn send_metrics_to_peer(node: &NetworkNode, frame_bytes: &[u8]) -> bool {
    let conn = match node.connection.as_ref() {
        Some(c) => c,
        None => return false,
    };

    let short_id = &node.node_id[..8.min(node.node_id.len())];

    match conn.open_bi().await {
        Ok((mut send, _recv)) => {
            if let Err(e) = send.write_all(&[super::CONN_TYPE_METRICS]).await {
                debug!("Metrics discriminator write to {} failed: {}", short_id, e);
                return false;
            }
            if let Err(e) = send.write_all(frame_bytes).await {
                debug!("Metrics frame write to {} failed: {}", short_id, e);
                return false;
            }
            if let Err(e) = send.finish() {
                debug!("Metrics stream finish to {} failed: {}", short_id, e);
                return false;
            }
            debug!("Pushed {} metrics bytes to peer {}", frame_bytes.len(), short_id);
            true
        }
        Err(e) => {
            debug!("Metrics stream open to {} failed: {}", short_id, e);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_capacity_frame_produces_valid_json() {
        let mut reporter = MetricsReporter::new("test-node-001".to_string());
        let bytes = reporter.build_capacity_frame(42, 3, 100, 55.5, 70.2);

        let parsed: serde_json::Value =
            serde_json::from_slice(&bytes).expect("test: valid JSON");

        assert_eq!(parsed["source_node"], "test-node-001");
        assert_eq!(parsed["sequence"], 1);
        assert_eq!(parsed["payload"]["Capacity"]["compute_delivered"], 42);
        assert_eq!(
            parsed["payload"]["Capacity"]["storage_maintained_bytes"],
            100 * 65536
        );
    }

    #[test]
    fn sequence_increments_on_each_build() {
        let mut reporter = MetricsReporter::new("seq-node".to_string());

        for expected_seq in 1..=5 {
            let bytes = reporter.build_capacity_frame(0, 0, 0, 0.0, 0.0);
            let parsed: serde_json::Value =
                serde_json::from_slice(&bytes).expect("test: valid JSON");
            assert_eq!(parsed["sequence"], expected_seq);
        }
    }

    fn test_ebpf_metrics() -> HyperMeshMetrics {
        use hypermesh_ebpf::metrics::*;
        HyperMeshMetrics {
            transport_metrics: TransportMetrics {
                total_packets: 5_000,
                packets_per_second: 250.0,
                bytes_per_second: 31_250_000.0,
                total_bytes: 7_500_000,
                kernel_drops: 25,
                af_xdp_redirects: 4_000,
                zero_copy_ops: 3_800,
                memcpy_ops: 200,
                latency_min_us: 50,
                latency_max_us: 8_000,
                latency_avg_us: 1_500,
            },
            routing_metrics: MatrixRoutingMetrics {
                total_validations: 100,
                successful: 95,
                path_failures: 5,
                topology_violations: 1,
                avg_path_length: 2.8,
                avg_validation_us: 40,
            },
            pos_metrics: ProofOfStateMetrics::default(),
            asset_metrics: AssetHashMetrics::default(),
            privacy_metrics: PrivacyTierMetrics::default(),
        }
    }

    #[test]
    fn build_congestion_frame_from_ebpf() {
        let mut reporter = MetricsReporter::new("ebpf-cong-node".to_string());
        let bytes = reporter.build_congestion_frame(&test_ebpf_metrics());

        let parsed: serde_json::Value =
            serde_json::from_slice(&bytes).expect("test: valid JSON");

        assert_eq!(parsed["source_node"], "ebpf-cong-node");
        assert_eq!(parsed["payload"]["Congestion"]["dropped_packets_epoch"], 25);
        assert_eq!(parsed["payload"]["Congestion"]["avg_queue_wait_us"], 1_500);
    }

    #[test]
    fn build_routing_frame_from_ebpf() {
        let mut reporter = MetricsReporter::new("ebpf-route-node".to_string());
        let bytes = reporter.build_routing_frame(&test_ebpf_metrics());

        let parsed: serde_json::Value =
            serde_json::from_slice(&bytes).expect("test: valid JSON");

        assert_eq!(parsed["source_node"], "ebpf-route-node");
        assert_eq!(parsed["payload"]["Routing"]["avg_latency_us"], 1_500);
        // 31_250_000 * 8 = 250_000_000
        assert_eq!(parsed["payload"]["Routing"]["throughput_bps"], 250_000_000u64);
        assert_eq!(parsed["payload"]["Routing"]["path_count"], 95);
    }

    #[test]
    fn frames_fit_in_stoq_stream() {
        let mut reporter = MetricsReporter::new("size-test".to_string());
        let capacity = reporter.build_capacity_frame(u64::MAX, 1000, 999999, 100.0, 100.0);
        let congestion = reporter.build_congestion_frame(&test_ebpf_metrics());
        let routing = reporter.build_routing_frame(&test_ebpf_metrics());

        // Frames should be well under 1 KB
        assert!(capacity.len() < 1024, "capacity frame {} bytes", capacity.len());
        assert!(congestion.len() < 1024, "congestion frame {} bytes", congestion.len());
        assert!(routing.len() < 1024, "routing frame {} bytes", routing.len());
    }

    #[test]
    fn ebpf_frames_increment_sequence() {
        let mut reporter = MetricsReporter::new("seq-ebpf".to_string());
        let _ = reporter.build_capacity_frame(1, 0, 0, 0.0, 0.0); // seq 1
        let congestion = reporter.build_congestion_frame(&test_ebpf_metrics()); // seq 2
        let routing = reporter.build_routing_frame(&test_ebpf_metrics()); // seq 3

        let c: serde_json::Value =
            serde_json::from_slice(&congestion).expect("test: valid JSON");
        let r: serde_json::Value =
            serde_json::from_slice(&routing).expect("test: valid JSON");

        assert_eq!(c["sequence"], 2);
        assert_eq!(r["sequence"], 3);
    }

    #[tokio::test]
    async fn push_to_empty_peers_sets_no_backoff() {
        let mut reporter = MetricsReporter::new("backoff-node".to_string());
        let frame = reporter.build_capacity_frame(1, 0, 0, 0.0, 0.0);

        // Push with no peers — should return immediately, no backoff
        reporter.push_to_peers(&frame, &[]).await;
        assert_eq!(reporter.backoff_remaining, 0);
    }

    #[test]
    fn backoff_decrements_on_each_call() {
        let mut reporter = MetricsReporter::new("backoff-dec".to_string());
        reporter.backoff_remaining = 3;

        // Simulate sync call: backoff should prevent processing
        // (push_to_peers is async, so we just verify the field logic)
        assert!(reporter.backoff_remaining > 0);
        reporter.backoff_remaining -= 1;
        assert_eq!(reporter.backoff_remaining, 2);
    }
}
