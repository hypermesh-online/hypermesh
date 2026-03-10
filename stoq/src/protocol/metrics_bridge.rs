// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Bridge between STOQ transport metrics and engauge MetricsFrame streaming.
//!
//! [`MetricsFrameBridge`] accepts an engauge [`MetricsPublisher`] and converts
//! STOQ transport events (latency, throughput, connection health) into
//! Capacity and Congestion [`MetricsFrame`]s for streaming to the mesh.

use engauge::streaming::protocol::{CongestionSnapshot, RoutingSnapshot};
use engauge::streaming::{MetricsFrame, MetricsPublisher};
use engauge::CapacityMetrics;
use hypermesh_ebpf::metrics::HyperMeshMetrics;

/// Bridge that converts STOQ transport metrics into engauge MetricsFrames.
///
/// STOQ creates this bridge and calls [`publish_transport_metrics`] periodically
/// to emit Capacity and Congestion frames via the engauge publisher.
pub struct MetricsFrameBridge {
    publisher: MetricsPublisher,
    /// Accumulated frames for external consumption.
    outbound: Vec<MetricsFrame>,
}

/// Transport metrics snapshot from STOQ.
#[derive(Debug, Clone)]
pub struct TransportSnapshot {
    /// Average RTT in microseconds.
    pub avg_latency_us: u64,
    /// Throughput in bits per second.
    pub throughput_bps: u64,
    /// Active connection count.
    pub active_connections: u32,
    /// Available routing paths.
    pub path_count: u16,
    /// Buffer fullness ratio (0.0 to 1.0).
    pub buffer_fullness: f64,
    /// Queue depth in items.
    pub queue_depth: u32,
    /// Packets dropped this epoch.
    pub dropped_packets: u64,
    /// Average queue wait in microseconds.
    pub avg_queue_wait_us: u64,
    /// Total bytes served.
    pub bytes_served: u64,
    /// Compute units delivered.
    pub compute_delivered: u64,
    /// Storage maintained in bytes.
    pub storage_maintained_bytes: u64,
    /// Available bandwidth in bps.
    pub bandwidth_available_bps: u64,
    /// Uptime ratio (0.0 to 1.0).
    pub uptime_ratio: f64,
}

impl MetricsFrameBridge {
    /// Create a new bridge with the given engauge publisher.
    pub fn new(publisher: MetricsPublisher) -> Self {
        Self {
            publisher,
            outbound: Vec::new(),
        }
    }

    /// Publish transport metrics as Capacity + Congestion + Routing frames.
    ///
    /// Returns the number of frames produced (some may be filtered by privacy).
    pub fn publish_transport_metrics(&mut self, snapshot: &TransportSnapshot) -> usize {
        let mut count = 0;

        // 1. Capacity frame.
        let capacity = CapacityMetrics::new(
            snapshot.bytes_served,
            snapshot.compute_delivered,
            snapshot.storage_maintained_bytes,
            snapshot.bandwidth_available_bps,
            snapshot.uptime_ratio,
        );
        if let Some(frame) = self.publisher.publish_capacity(&capacity) {
            self.outbound.push(frame);
            count += 1;
        }

        // 2. Congestion frame.
        let congestion = CongestionSnapshot {
            buffer_fullness_ratio: snapshot.buffer_fullness,
            queue_depth: snapshot.queue_depth,
            dropped_packets_epoch: snapshot.dropped_packets,
            avg_queue_wait_us: snapshot.avg_queue_wait_us,
        };
        if let Some(frame) = self.publisher.publish_congestion(congestion) {
            self.outbound.push(frame);
            count += 1;
        }

        // 3. Routing frame.
        let routing = RoutingSnapshot {
            avg_latency_us: snapshot.avg_latency_us,
            throughput_bps: snapshot.throughput_bps,
            path_count: snapshot.path_count,
            active_connections: snapshot.active_connections,
        };
        if let Some(frame) = self.publisher.publish_routing(routing) {
            self.outbound.push(frame);
            count += 1;
        }

        count
    }

    /// Publish eBPF metrics as Congestion + Routing frames.
    ///
    /// Maps eBPF transport metrics (drops, latency) to Congestion frames and
    /// routing/throughput metrics to Routing frames. Returns the number of
    /// frames produced (some may be filtered by privacy).
    pub fn publish_ebpf_metrics(&mut self, ebpf: &HyperMeshMetrics) -> usize {
        let mut count = 0;

        // Congestion frame from eBPF transport drops and latency.
        let congestion = congestion_from_ebpf(ebpf);
        if let Some(frame) = self.publisher.publish_congestion(congestion) {
            self.outbound.push(frame);
            count += 1;
        }

        // Routing frame from eBPF transport throughput and routing metrics.
        let routing = routing_from_ebpf(ebpf);
        if let Some(frame) = self.publisher.publish_routing(routing) {
            self.outbound.push(frame);
            count += 1;
        }

        count
    }

    /// Drain all outbound frames for transmission.
    pub fn drain_outbound(&mut self) -> Vec<MetricsFrame> {
        std::mem::take(&mut self.outbound)
    }

    /// Encode all outbound frames as byte vectors for STOQ transmission.
    pub fn drain_encoded(&mut self) -> Vec<Vec<u8>> {
        self.outbound
            .drain(..)
            .filter_map(|frame| frame.encode().ok())
            .collect()
    }

    /// Number of pending outbound frames.
    pub fn outbound_count(&self) -> usize {
        self.outbound.len()
    }

    /// Access the underlying publisher.
    pub fn publisher(&self) -> &MetricsPublisher {
        &self.publisher
    }
}

// ===========================================================================
// eBPF → MetricsFrame converters
// ===========================================================================

/// Convert eBPF transport metrics into a CongestionSnapshot.
///
/// Maps kernel drops to dropped_packets_epoch, latency to avg_queue_wait_us,
/// and derives buffer fullness from the drop ratio.
fn congestion_from_ebpf(ebpf: &HyperMeshMetrics) -> CongestionSnapshot {
    let t = &ebpf.transport_metrics;
    let total = t.total_packets.max(1);
    let drop_ratio = t.kernel_drops as f64 / total as f64;

    CongestionSnapshot {
        buffer_fullness_ratio: drop_ratio.clamp(0.0, 1.0),
        queue_depth: t.kernel_drops.min(u32::MAX as u64) as u32,
        dropped_packets_epoch: t.kernel_drops,
        avg_queue_wait_us: t.latency_avg_us,
    }
}

/// Convert eBPF transport and routing metrics into a RoutingSnapshot.
///
/// Maps throughput to throughput_bps, latency to avg_latency_us,
/// and routing validations to path_count and active_connections.
fn routing_from_ebpf(ebpf: &HyperMeshMetrics) -> RoutingSnapshot {
    let t = &ebpf.transport_metrics;
    let r = &ebpf.routing_metrics;

    RoutingSnapshot {
        avg_latency_us: t.latency_avg_us,
        throughput_bps: (t.bytes_per_second * 8.0) as u64,
        path_count: r.successful.min(u16::MAX as u64) as u16,
        active_connections: t.af_xdp_redirects.min(u32::MAX as u64) as u32,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use engauge::streaming::MetricsPayload;
    use hypermesh_lib::types::NodeId;
    use hypermesh_lib::PrivacyMode;

    fn test_publisher(privacy: PrivacyMode) -> MetricsPublisher {
        MetricsPublisher::new(
            NodeId::from_public_key(b"bridge-test-node"),
            privacy,
            1.0,
            10,
        )
    }

    fn test_snapshot() -> TransportSnapshot {
        TransportSnapshot {
            avg_latency_us: 5000,
            throughput_bps: 500_000_000,
            active_connections: 10,
            path_count: 3,
            buffer_fullness: 0.4,
            queue_depth: 15,
            dropped_packets: 2,
            avg_queue_wait_us: 100,
            bytes_served: 1_073_741_824,
            compute_delivered: 500_000,
            storage_maintained_bytes: 5_000_000_000,
            bandwidth_available_bps: 1_000_000_000,
            uptime_ratio: 0.99,
        }
    }

    #[test]
    fn bridge_publishes_public_frames() {
        let publisher = test_publisher(PrivacyMode::PUBLIC);
        let mut bridge = MetricsFrameBridge::new(publisher);

        let count = bridge.publish_transport_metrics(&test_snapshot());
        assert_eq!(count, 3, "Public should produce Capacity + Congestion + Routing");
        assert_eq!(bridge.outbound_count(), 3);

        let frames = bridge.drain_outbound();
        assert_eq!(frames.len(), 3);

        // Verify payload types.
        let has_capacity = frames
            .iter()
            .any(|f| matches!(f.payload, MetricsPayload::Capacity(_)));
        let has_congestion = frames
            .iter()
            .any(|f| matches!(f.payload, MetricsPayload::Congestion(_)));
        let has_routing = frames
            .iter()
            .any(|f| matches!(f.payload, MetricsPayload::Routing(_)));

        assert!(has_capacity, "should include Capacity frame");
        assert!(has_congestion, "should include Congestion frame");
        assert!(has_routing, "should include Routing frame");
    }

    #[test]
    fn bridge_filters_anonymous_frames() {
        let publisher = test_publisher(PrivacyMode::ANONYMOUS);
        let mut bridge = MetricsFrameBridge::new(publisher);

        let count = bridge.publish_transport_metrics(&test_snapshot());
        assert_eq!(count, 0, "Anonymous should produce no frames");
        assert_eq!(bridge.outbound_count(), 0);
    }

    #[test]
    fn bridge_private_produces_capacity_and_congestion_only() {
        let publisher = test_publisher(PrivacyMode::PRIVATE);
        let mut bridge = MetricsFrameBridge::new(publisher);

        let count = bridge.publish_transport_metrics(&test_snapshot());
        assert_eq!(
            count, 2,
            "Private should produce Capacity + Congestion only"
        );

        let frames = bridge.drain_outbound();
        let has_routing = frames
            .iter()
            .any(|f| matches!(f.payload, MetricsPayload::Routing(_)));
        assert!(
            !has_routing,
            "Private should NOT include Routing frame"
        );
    }

    fn test_ebpf_metrics() -> HyperMeshMetrics {
        use hypermesh_ebpf::metrics::*;
        HyperMeshMetrics {
            transport_metrics: TransportMetrics {
                total_packets: 10_000,
                packets_per_second: 500.0,
                bytes_per_second: 62_500_000.0, // 500 Mbps
                total_bytes: 15_000_000,
                kernel_drops: 50,
                af_xdp_redirects: 8_000,
                zero_copy_ops: 7_500,
                memcpy_ops: 500,
                latency_min_us: 100,
                latency_max_us: 10_000,
                latency_avg_us: 2_500,
            },
            routing_metrics: MatrixRoutingMetrics {
                total_validations: 200,
                successful: 190,
                path_failures: 10,
                topology_violations: 2,
                avg_path_length: 3.5,
                avg_validation_us: 50,
            },
            pos_metrics: ProofOfStateMetrics::default(),
            asset_metrics: AssetHashMetrics::default(),
            privacy_metrics: PrivacyTierMetrics::default(),
        }
    }

    #[test]
    fn bridge_publishes_ebpf_public_frames() {
        let publisher = test_publisher(PrivacyMode::PUBLIC);
        let mut bridge = MetricsFrameBridge::new(publisher);

        let count = bridge.publish_ebpf_metrics(&test_ebpf_metrics());
        assert_eq!(count, 2, "Public eBPF should produce Congestion + Routing");

        let frames = bridge.drain_outbound();
        let has_congestion = frames
            .iter()
            .any(|f| matches!(f.payload, MetricsPayload::Congestion(_)));
        let has_routing = frames
            .iter()
            .any(|f| matches!(f.payload, MetricsPayload::Routing(_)));

        assert!(has_congestion, "should include Congestion frame from eBPF");
        assert!(has_routing, "should include Routing frame from eBPF");
    }

    #[test]
    fn bridge_ebpf_anonymous_produces_no_frames() {
        let publisher = test_publisher(PrivacyMode::ANONYMOUS);
        let mut bridge = MetricsFrameBridge::new(publisher);

        let count = bridge.publish_ebpf_metrics(&test_ebpf_metrics());
        assert_eq!(count, 0, "Anonymous should produce no eBPF frames");
    }

    #[test]
    fn congestion_from_ebpf_maps_correctly() {
        let metrics = test_ebpf_metrics();
        let snap = super::congestion_from_ebpf(&metrics);

        assert_eq!(snap.dropped_packets_epoch, 50);
        assert_eq!(snap.avg_queue_wait_us, 2_500);
        // drop ratio = 50/10000 = 0.005
        assert!(snap.buffer_fullness_ratio < 0.01);
        assert!(snap.buffer_fullness_ratio > 0.0);
    }

    #[test]
    fn routing_from_ebpf_maps_correctly() {
        let metrics = test_ebpf_metrics();
        let snap = super::routing_from_ebpf(&metrics);

        assert_eq!(snap.avg_latency_us, 2_500);
        // 62_500_000 bytes/s * 8 = 500_000_000 bps
        assert_eq!(snap.throughput_bps, 500_000_000);
        // 190 successful routing validations
        assert_eq!(snap.path_count, 190);
        assert_eq!(snap.active_connections, 8_000);
    }

    #[test]
    fn drain_encoded_produces_valid_bytes() {
        let publisher = test_publisher(PrivacyMode::PUBLIC);
        let mut bridge = MetricsFrameBridge::new(publisher);

        bridge.publish_transport_metrics(&test_snapshot());
        let encoded = bridge.drain_encoded();

        assert_eq!(encoded.len(), 3);
        for bytes in &encoded {
            assert!(!bytes.is_empty(), "encoded frame should not be empty");
            // Verify it can be decoded back.
            let decoded = MetricsFrame::decode(bytes);
            assert!(decoded.is_ok(), "encoded bytes should be decodable");
        }
    }
}
