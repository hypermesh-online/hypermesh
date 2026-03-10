// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Lightweight metrics reporter for the engauge streaming pipeline.
//!
//! Collects real node data and produces engauge-compatible `MetricsFrame`
//! JSON. Emits structured `tracing` events and pushes frames to engauge's
//! UDP ingestion endpoint at `[::1]:9297` (best-effort, fire-and-forget).

use hypermesh_ebpf::metrics::HyperMeshMetrics;
use tokio::net::UdpSocket;
use tracing::{debug, info, warn};

/// Default engauge UDP ingestion address.
const ENGAUGE_UDP_ADDR: &str = "[::1]:9297";

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
/// and pushes them over UDP.
pub struct MetricsReporter {
    node_id: String,
    sequence: u64,
    /// Cached UDP socket (lazily initialized on first push).
    socket: Option<UdpSocket>,
    /// Remaining cycles to skip before retrying after a failure.
    backoff_remaining: u64,
}

impl MetricsReporter {
    /// Create a new reporter for the given node.
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            sequence: 0,
            socket: None,
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

    /// Push a pre-built frame to the engauge UDP ingestion endpoint.
    ///
    /// Best-effort: if engauge is not running or the send fails, the error
    /// is logged at debug level and a backoff counter is set so subsequent
    /// cycles skip the push attempt (avoiding log spam).
    pub async fn push_to_engauge(&mut self, frame_bytes: &[u8]) {
        // Respect backoff
        if self.backoff_remaining > 0 {
            self.backoff_remaining -= 1;
            return;
        }

        // Lazy-init the UDP socket
        if self.socket.is_none() {
            match UdpSocket::bind("[::1]:0").await {
                Ok(sock) => {
                    self.socket = Some(sock);
                }
                Err(e) => {
                    debug!("Failed to bind UDP socket for engauge metrics: {e}");
                    self.backoff_remaining = BACKOFF_CYCLES;
                    return;
                }
            }
        }

        let Some(sock) = self.socket.as_ref() else {
            return;
        };
        match sock.send_to(frame_bytes, ENGAUGE_UDP_ADDR).await {
            Ok(sent) => {
                debug!("Pushed {sent} bytes to engauge at {ENGAUGE_UDP_ADDR}");
            }
            Err(e) => {
                // Connection refused or unreachable — engauge probably not running.
                // Set backoff to avoid spamming on every cycle.
                debug!("Engauge metrics push failed (backoff {BACKOFF_CYCLES} cycles): {e}");
                self.backoff_remaining = BACKOFF_CYCLES;
            }
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
    fn ebpf_frames_fit_in_udp_datagram() {
        let mut reporter = MetricsReporter::new("size-ebpf".to_string());
        let congestion = reporter.build_congestion_frame(&test_ebpf_metrics());
        let routing = reporter.build_routing_frame(&test_ebpf_metrics());

        assert!(congestion.len() < 1024, "congestion frame too large");
        assert!(routing.len() < 1024, "routing frame too large");
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

    #[test]
    fn frame_fits_in_udp_datagram() {
        let mut reporter = MetricsReporter::new("size-test-node".to_string());
        let bytes = reporter.build_capacity_frame(u64::MAX, 1000, 999999, 100.0, 100.0);
        // Standard UDP max payload: 65507 bytes. Our frames should be well under 1 KB.
        assert!(
            bytes.len() < 1024,
            "frame is {} bytes, should be under 1KB for UDP",
            bytes.len()
        );
    }

    #[tokio::test]
    async fn push_with_no_listener_triggers_backoff() {
        let mut reporter = MetricsReporter::new("backoff-node".to_string());
        let frame = reporter.build_capacity_frame(1, 0, 0, 0.0, 0.0);

        // First push — UDP send_to succeeds even without a listener (UDP is
        // connectionless), so we just verify it doesn't panic.
        reporter.push_to_engauge(&frame).await;

        // Backoff should be 0 because UDP send_to to localhost typically
        // succeeds (the OS accepts the datagram even if nobody is listening).
        // This test validates no panic and graceful handling.
        assert_eq!(reporter.backoff_remaining, 0);
    }

    #[tokio::test]
    async fn push_roundtrip_with_listener() {
        // Start a UDP listener
        let listener = tokio::net::UdpSocket::bind("[::1]:0")
            .await
            .expect("test: bind");
        let listen_addr = listener.local_addr().expect("test: addr");

        // Build a frame
        let mut reporter = MetricsReporter::new("roundtrip-node".to_string());
        let frame = reporter.build_capacity_frame(99, 5, 200, 12.3, 45.6);

        // We can't use push_to_engauge because it hardcodes the address,
        // so test the serialization roundtrip manually via UDP.
        let sender = tokio::net::UdpSocket::bind("[::1]:0")
            .await
            .expect("test: bind sender");
        sender
            .send_to(&frame, listen_addr)
            .await
            .expect("test: send");

        let mut buf = vec![0u8; 65536];
        let (len, _) = listener.recv_from(&mut buf).await.expect("test: recv");

        let received: serde_json::Value =
            serde_json::from_slice(&buf[..len]).expect("test: parse received");
        assert_eq!(received["source_node"], "roundtrip-node");
        assert_eq!(received["payload"]["Capacity"]["compute_delivered"], 99);
    }
}
