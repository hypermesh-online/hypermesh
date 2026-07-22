// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! MetricsPublisher — aggregates local metrics and produces privacy-filtered
//! [`MetricsFrame`]s suitable for transmission to peer nodes.

use std::sync::Arc;

use chrono::Utc;
use hypermesh_lib::{NodeId, PrivacyMode};

use crate::capacity::CapacityMetrics;

use super::privacy_filter::DifferentialPrivacyFilter;
use super::protocol::{
    CapacitySnapshot, CongestionSnapshot, EconomicSnapshot, MetricsFrame, MetricsPayload,
    RoutingSnapshot,
};

// ---------------------------------------------------------------------------
// MetricsPublisher
// ---------------------------------------------------------------------------

/// Produces privacy-filtered [`MetricsFrame`]s from local metrics sources.
///
/// Each published frame receives a monotonically increasing sequence number
/// and is filtered through the configured [`DifferentialPrivacyFilter`]
/// before being returned to the caller for transmission.
pub struct MetricsPublisher {
    node_id: NodeId,
    privacy_mode: PrivacyMode,
    filter: DifferentialPrivacyFilter,
    sequence: u64,
    /// Publish interval hint (seconds); informational, not enforced here.
    publish_interval_secs: u64,
    /// Optional transport callback invoked for each published frame.
    transport_fn: Option<Arc<dyn Fn(MetricsFrame) + Send + Sync>>,
}

impl MetricsPublisher {
    /// Create a new publisher for the given node.
    ///
    /// - `epsilon`: differential privacy budget (passed to filter).
    /// - `interval_secs`: advisory publish cadence (not enforced internally).
    pub fn new(
        node_id: NodeId,
        privacy_mode: PrivacyMode,
        epsilon: f64,
        interval_secs: u64,
    ) -> Self {
        Self {
            node_id,
            privacy_mode,
            filter: DifferentialPrivacyFilter::new(epsilon),
            sequence: 0,
            publish_interval_secs: interval_secs,
            transport_fn: None,
        }
    }

    /// Returns the configured publish interval in seconds.
    pub fn publish_interval_secs(&self) -> u64 {
        self.publish_interval_secs
    }

    /// Register a transport callback invoked for each successfully published frame.
    ///
    /// The callback receives a clone of the privacy-filtered frame before it
    /// is returned to the caller, enabling cross-node transmission without
    /// changing the publish API.
    pub fn set_transport(&mut self, f: Arc<dyn Fn(MetricsFrame) + Send + Sync>) {
        self.transport_fn = Some(f);
    }

    /// Publish a capacity frame from [`CapacityMetrics`].
    ///
    /// Returns `None` if the privacy filter suppresses the frame.
    pub fn publish_capacity(&mut self, metrics: &CapacityMetrics) -> Option<MetricsFrame> {
        let snapshot = CapacitySnapshot::from(metrics);
        let frame = self.build_frame(MetricsPayload::Capacity(snapshot));
        let filtered = self.filter.filter_frame(frame);
        if let Some(ref f) = filtered {
            self.invoke_transport(f);
        }
        filtered
    }

    /// Publish a congestion snapshot.
    ///
    /// Returns `None` if the privacy filter suppresses the frame.
    pub fn publish_congestion(&mut self, snapshot: CongestionSnapshot) -> Option<MetricsFrame> {
        let frame = self.build_frame(MetricsPayload::Congestion(snapshot));
        let filtered = self.filter.filter_frame(frame);
        if let Some(ref f) = filtered {
            self.invoke_transport(f);
        }
        filtered
    }

    /// Publish a routing snapshot. Only passed through for Public privacy.
    ///
    /// Returns `None` if the privacy filter suppresses the frame.
    pub fn publish_routing(&mut self, snapshot: RoutingSnapshot) -> Option<MetricsFrame> {
        let frame = self.build_frame(MetricsPayload::Routing(snapshot));
        let filtered = self.filter.filter_frame(frame);
        if let Some(ref f) = filtered {
            self.invoke_transport(f);
        }
        filtered
    }

    /// Publish an economic snapshot. Only passed through for Public privacy.
    ///
    /// Returns `None` if the privacy filter suppresses the frame.
    pub fn publish_economic(&mut self, snapshot: EconomicSnapshot) -> Option<MetricsFrame> {
        let frame = self.build_frame(MetricsPayload::Economic(snapshot));
        let filtered = self.filter.filter_frame(frame);
        if let Some(ref f) = filtered {
            self.invoke_transport(f);
        }
        filtered
    }

    /// Return and increment the per-source sequence counter.
    pub fn next_sequence(&mut self) -> u64 {
        let seq = self.sequence;
        self.sequence += 1;
        seq
    }

    // -- internal ----------------------------------------------------------

    /// Invoke the transport callback with a clone of the frame, if set.
    fn invoke_transport(&self, frame: &MetricsFrame) {
        if let Some(ref cb) = self.transport_fn {
            cb(frame.clone());
        }
    }

    /// Build a raw (un-filtered) frame with the next sequence number.
    fn build_frame(&mut self, payload: MetricsPayload) -> MetricsFrame {
        let seq = self.next_sequence();
        MetricsFrame {
            source_node: self.node_id.clone(),
            timestamp_us: Utc::now().timestamp_micros() as u64,
            privacy_mode: self.privacy_mode,
            payload,
            sequence: seq,
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_capacity_metrics() -> CapacityMetrics {
        CapacityMetrics::new(
            1_073_741_824,
            1_000_000,
            10_737_418_240,
            1_000_000_000,
            0.99,
        )
    }

    #[test]
    fn publish_capacity_anonymous_returns_none() {
        let mut pub_anon =
            MetricsPublisher::new(NodeId::from_public_key(b"anon-pub"), PrivacyMode::ANONYMOUS, 1.0, 10);
        let result = pub_anon.publish_capacity(&test_capacity_metrics());
        assert!(
            result.is_none(),
            "Anonymous publisher must suppress capacity frames"
        );
    }

    #[test]
    fn publish_capacity_public_returns_some() {
        let mut publisher =
            MetricsPublisher::new(NodeId::from_public_key(b"pub-node"), PrivacyMode::PUBLIC, 1.0, 10);
        let result = publisher.publish_capacity(&test_capacity_metrics());
        assert!(
            result.is_some(),
            "Public publisher must return capacity frames"
        );
        let frame = result.expect("test: capacity frame expected");
        assert_eq!(frame.source_node, NodeId::from_public_key(b"pub-node"));
        match &frame.payload {
            MetricsPayload::Capacity(_) => {}
            _ => panic!("test: expected Capacity payload"),
        }
    }

    #[test]
    fn sequence_numbers_increment() {
        let mut publisher =
            MetricsPublisher::new(NodeId::from_public_key(b"seq-node"), PrivacyMode::PUBLIC, 1.0, 10);

        let f1 = publisher
            .publish_capacity(&test_capacity_metrics())
            .expect("test: first frame");
        let f2 = publisher
            .publish_capacity(&test_capacity_metrics())
            .expect("test: second frame");
        let f3 = publisher
            .publish_capacity(&test_capacity_metrics())
            .expect("test: third frame");

        assert_eq!(f1.sequence, 0);
        assert_eq!(f2.sequence, 1);
        assert_eq!(f3.sequence, 2);
    }

    #[test]
    fn privacy_filter_applied_to_published_frames() {
        let mut publisher =
            MetricsPublisher::new(NodeId::from_public_key(b"filter-node"), PrivacyMode::PRIVATE, 1.0, 10);

        // Private should pass capacity...
        let cap = publisher.publish_capacity(&test_capacity_metrics());
        assert!(cap.is_some(), "Private must pass Capacity");

        // ...but suppress economic.
        let econ = publisher.publish_economic(EconomicSnapshot {
            in_flight_float_grams: 10.0,
            settlement_rate_per_epoch: 5.0,
            active_packets: 3,
            ..Default::default()
        });
        assert!(econ.is_none(), "Private must suppress Economic payload");
    }

    #[test]
    fn publish_economic_private_returns_none() {
        let mut publisher =
            MetricsPublisher::new(NodeId::from_public_key(b"priv-econ"), PrivacyMode::PRIVATE, 1.0, 10);
        let result = publisher.publish_economic(EconomicSnapshot {
            in_flight_float_grams: 5.0,
            settlement_rate_per_epoch: 2.0,
            active_packets: 1,
            ..Default::default()
        });
        assert!(
            result.is_none(),
            "Private publisher must suppress economic frames"
        );
    }
}
