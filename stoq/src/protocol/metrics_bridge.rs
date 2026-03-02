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
