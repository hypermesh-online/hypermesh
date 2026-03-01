// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! MetricsFrame wire format and payload types.
//!
//! Each [`MetricsFrame`] represents a single unit of streaming metrics data
//! flowing between HyperMesh nodes. Frames carry a monotonically increasing
//! sequence number per source and are tagged with the source's privacy mode.

use hypermesh_lib::{NodeId, PrivacyMode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::capacity::CapacityMetrics;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur during frame encoding / decoding.
#[derive(Debug, Error)]
pub enum ProtocolError {
    /// JSON serialization or deserialization failed.
    #[error("frame codec error: {0}")]
    Codec(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// MetricsFrame
// ---------------------------------------------------------------------------

/// A single unit of streaming metrics data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsFrame {
    /// Node that produced this frame.
    pub source_node: NodeId,
    /// Unix epoch timestamp in microseconds.
    pub timestamp_us: u64,
    /// Privacy mode of the source node at frame creation time.
    pub privacy_mode: PrivacyMode,
    /// The metrics payload carried by this frame.
    pub payload: MetricsPayload,
    /// Monotonically increasing sequence number per source node.
    pub sequence: u64,
}

impl MetricsFrame {
    /// Encode this frame to a byte vector (JSON for alpha, swappable to bincode).
    pub fn encode(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(serde_json::to_vec(self)?)
    }

    /// Decode a frame from a byte slice.
    pub fn decode(bytes: &[u8]) -> Result<Self, ProtocolError> {
        Ok(serde_json::from_slice(bytes)?)
    }
}

// ---------------------------------------------------------------------------
// MetricsPayload
// ---------------------------------------------------------------------------

/// Variant payloads carried inside a [`MetricsFrame`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricsPayload {
    /// Node capacity measurements (bytes, compute, storage, bandwidth, uptime).
    Capacity(CapacitySnapshot),
    /// Congestion indicators (buffer fullness, queue depth, drops).
    Congestion(CongestionSnapshot),
    /// Routing quality metrics (latency, throughput, paths, connections).
    Routing(RoutingSnapshot),
    /// Economic activity metrics (in-flight float, settlement rate, packets).
    Economic(EconomicSnapshot),
    /// Spatial verification results from PoSPing probes.
    Verification(VerificationSnapshot),
}

// ---------------------------------------------------------------------------
// Snapshot types
// ---------------------------------------------------------------------------

/// Point-in-time capacity measurements for streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacitySnapshot {
    /// Total bytes served to requestors.
    pub bytes_served: u64,
    /// Total compute units delivered.
    pub compute_delivered: u64,
    /// Bytes of storage currently maintained.
    pub storage_maintained_bytes: u64,
    /// Available bandwidth in bits per second.
    pub bandwidth_available_bps: u64,
    /// Uptime ratio (0.0 - 1.0).
    pub uptime_ratio: f64,
}

impl From<&CapacityMetrics> for CapacitySnapshot {
    fn from(m: &CapacityMetrics) -> Self {
        Self {
            bytes_served: m.bytes_served,
            compute_delivered: m.compute_delivered,
            storage_maintained_bytes: m.storage_maintained_bytes,
            bandwidth_available_bps: m.bandwidth_available_bps,
            uptime_ratio: m.uptime_f64(),
        }
    }
}

/// Point-in-time congestion indicators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CongestionSnapshot {
    /// Buffer fullness ratio (0.0 = empty, 1.0 = full).
    pub buffer_fullness_ratio: f64,
    /// Current queue depth in items.
    pub queue_depth: u32,
    /// Packets dropped during this epoch.
    pub dropped_packets_epoch: u64,
    /// Average queue wait time in microseconds.
    pub avg_queue_wait_us: u64,
}

/// Point-in-time routing quality metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingSnapshot {
    /// Average latency in microseconds.
    pub avg_latency_us: u64,
    /// Throughput in bits per second.
    pub throughput_bps: u64,
    /// Number of available routing paths.
    pub path_count: u16,
    /// Active connection count.
    pub active_connections: u32,
}

/// Point-in-time economic activity metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicSnapshot {
    /// Total in-flight gold-gram float.
    pub in_flight_float_grams: f64,
    /// Settlements completed per epoch.
    pub settlement_rate_per_epoch: f64,
    /// Number of active CAES packets.
    pub active_packets: u32,
}

/// Point-in-time spatial verification results from PoSPing probes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationSnapshot {
    /// Number of PoSPing probes sent this epoch.
    pub probes_sent: u32,
    /// Number of probes that returned consistent.
    pub probes_passed: u32,
    /// Average response time in microseconds.
    pub avg_response_time_us: u64,
    /// Consistency ratio (probes_passed / probes_sent), 0.0 to 1.0.
    pub consistency_ratio: f64,
    /// Epoch number these results cover.
    pub epoch: u64,
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node() -> NodeId {
        NodeId::from("stream-node-001")
    }

    fn capacity_frame() -> MetricsFrame {
        MetricsFrame {
            source_node: test_node(),
            timestamp_us: 1_700_000_000_000_000,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Capacity(CapacitySnapshot {
                bytes_served: 1024,
                compute_delivered: 500,
                storage_maintained_bytes: 2048,
                bandwidth_available_bps: 1_000_000,
                uptime_ratio: 0.99,
            }),
            sequence: 1,
        }
    }

    fn congestion_frame() -> MetricsFrame {
        MetricsFrame {
            source_node: test_node(),
            timestamp_us: 1_700_000_000_000_001,
            privacy_mode: PrivacyMode::PRIVATE,
            payload: MetricsPayload::Congestion(CongestionSnapshot {
                buffer_fullness_ratio: 0.75,
                queue_depth: 42,
                dropped_packets_epoch: 3,
                avg_queue_wait_us: 150,
            }),
            sequence: 2,
        }
    }

    fn routing_frame() -> MetricsFrame {
        MetricsFrame {
            source_node: test_node(),
            timestamp_us: 1_700_000_000_000_002,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Routing(RoutingSnapshot {
                avg_latency_us: 5000,
                throughput_bps: 500_000_000,
                path_count: 3,
                active_connections: 15,
            }),
            sequence: 3,
        }
    }

    fn economic_frame() -> MetricsFrame {
        MetricsFrame {
            source_node: test_node(),
            timestamp_us: 1_700_000_000_000_003,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Economic(EconomicSnapshot {
                in_flight_float_grams: 42.5,
                settlement_rate_per_epoch: 10.0,
                active_packets: 7,
            }),
            sequence: 4,
        }
    }

    #[test]
    fn encode_decode_capacity_roundtrip() {
        let frame = capacity_frame();
        let bytes = frame.encode().expect("test: encode frame");
        let decoded = MetricsFrame::decode(&bytes).expect("test: decode capacity frame");
        assert_eq!(decoded.source_node, test_node());
        assert_eq!(decoded.sequence, 1);
        match &decoded.payload {
            MetricsPayload::Capacity(c) => {
                assert_eq!(c.bytes_served, 1024);
                assert!((c.uptime_ratio - 0.99).abs() < 1e-9);
            }
            _ => panic!("test: expected Capacity payload"),
        }
    }

    #[test]
    fn encode_decode_congestion_roundtrip() {
        let frame = congestion_frame();
        let bytes = frame.encode().expect("test: encode frame");
        let decoded = MetricsFrame::decode(&bytes).expect("test: decode congestion frame");
        assert_eq!(decoded.sequence, 2);
        match &decoded.payload {
            MetricsPayload::Congestion(c) => {
                assert_eq!(c.queue_depth, 42);
                assert_eq!(c.dropped_packets_epoch, 3);
            }
            _ => panic!("test: expected Congestion payload"),
        }
    }

    #[test]
    fn encode_decode_routing_roundtrip() {
        let frame = routing_frame();
        let bytes = frame.encode().expect("test: encode frame");
        let decoded = MetricsFrame::decode(&bytes).expect("test: decode routing frame");
        assert_eq!(decoded.sequence, 3);
        match &decoded.payload {
            MetricsPayload::Routing(r) => {
                assert_eq!(r.avg_latency_us, 5000);
                assert_eq!(r.path_count, 3);
            }
            _ => panic!("test: expected Routing payload"),
        }
    }

    #[test]
    fn encode_decode_economic_roundtrip() {
        let frame = economic_frame();
        let bytes = frame.encode().expect("test: encode frame");
        let decoded = MetricsFrame::decode(&bytes).expect("test: decode economic frame");
        assert_eq!(decoded.sequence, 4);
        match &decoded.payload {
            MetricsPayload::Economic(e) => {
                assert!((e.in_flight_float_grams - 42.5).abs() < 1e-9);
                assert_eq!(e.active_packets, 7);
            }
            _ => panic!("test: expected Economic payload"),
        }
    }

    fn verification_frame() -> MetricsFrame {
        MetricsFrame {
            source_node: test_node(),
            timestamp_us: 1_700_000_000_000_004,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Verification(VerificationSnapshot {
                probes_sent: 100,
                probes_passed: 95,
                avg_response_time_us: 1200,
                consistency_ratio: 0.95,
                epoch: 42,
            }),
            sequence: 5,
        }
    }

    #[test]
    fn encode_decode_verification_roundtrip() {
        let frame = verification_frame();
        let bytes = frame.encode().expect("test: encode frame");
        let decoded = MetricsFrame::decode(&bytes).expect("test: decode verification frame");
        assert_eq!(decoded.sequence, 5);
        match &decoded.payload {
            MetricsPayload::Verification(v) => {
                assert_eq!(v.probes_sent, 100);
                assert_eq!(v.probes_passed, 95);
                assert_eq!(v.avg_response_time_us, 1200);
                assert!((v.consistency_ratio - 0.95).abs() < 1e-9);
                assert_eq!(v.epoch, 42);
            }
            _ => unreachable!("test: expected Verification payload"), // test-only
        }
    }

    #[test]
    fn capacity_snapshot_from_capacity_metrics() {
        let metrics = CapacityMetrics::new(
            1_073_741_824,
            1_000_000,
            10_737_418_240,
            1_000_000_000,
            0.95,
        );
        let snapshot = CapacitySnapshot::from(&metrics);
        assert_eq!(snapshot.bytes_served, 1_073_741_824);
        assert_eq!(snapshot.compute_delivered, 1_000_000);
        assert_eq!(snapshot.storage_maintained_bytes, 10_737_418_240);
        assert_eq!(snapshot.bandwidth_available_bps, 1_000_000_000);
        assert!((snapshot.uptime_ratio - 0.95).abs() < 1e-3);
    }

    #[test]
    fn frame_with_each_privacy_mode_serializes() {
        for mode in &[
            PrivacyMode::ANONYMOUS,
            PrivacyMode::PRIVATE,
            PrivacyMode::PUBLIC,
        ] {
            let frame = MetricsFrame {
                source_node: test_node(),
                timestamp_us: 100,
                privacy_mode: *mode,
                payload: MetricsPayload::Capacity(CapacitySnapshot {
                    bytes_served: 0,
                    compute_delivered: 0,
                    storage_maintained_bytes: 0,
                    bandwidth_available_bps: 0,
                    uptime_ratio: 0.0,
                }),
                sequence: 0,
            };
            let bytes = frame.encode().expect("test: encode frame");
            let decoded =
                MetricsFrame::decode(&bytes).expect("test: decode frame with privacy mode");
            assert_eq!(decoded.privacy_mode, *mode);
        }
    }

    #[test]
    fn zero_values_handle_correctly() {
        let frame = MetricsFrame {
            source_node: NodeId::from("zero-node"),
            timestamp_us: 0,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Capacity(CapacitySnapshot {
                bytes_served: 0,
                compute_delivered: 0,
                storage_maintained_bytes: 0,
                bandwidth_available_bps: 0,
                uptime_ratio: 0.0,
            }),
            sequence: 0,
        };
        let bytes = frame.encode().expect("test: encode frame");
        let decoded = MetricsFrame::decode(&bytes).expect("test: decode zero-value frame");
        assert_eq!(decoded.timestamp_us, 0);
        assert_eq!(decoded.sequence, 0);
        match &decoded.payload {
            MetricsPayload::Capacity(c) => {
                assert_eq!(c.bytes_served, 0);
                assert!((c.uptime_ratio).abs() < 1e-9);
            }
            _ => panic!("test: expected Capacity payload"),
        }
    }

    #[test]
    fn sequence_numbering_monotonic() {
        let frames: Vec<MetricsFrame> = (0..5)
            .map(|i| MetricsFrame {
                source_node: test_node(),
                timestamp_us: 100 + i,
                privacy_mode: PrivacyMode::PUBLIC,
                payload: MetricsPayload::Capacity(CapacitySnapshot {
                    bytes_served: i,
                    compute_delivered: 0,
                    storage_maintained_bytes: 0,
                    bandwidth_available_bps: 0,
                    uptime_ratio: 0.0,
                }),
                sequence: i,
            })
            .collect();

        for window in frames.windows(2) {
            assert!(
                window[1].sequence > window[0].sequence,
                "sequence must be monotonically increasing"
            );
        }
    }
}
