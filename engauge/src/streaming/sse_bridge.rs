// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! SSE bridge — broadcast-channel adapter for streaming [`MetricsFrame`]s to
//! HTTP/SSE consumers (e.g. the HyperMesh gateway).
//!
//! The bridge holds an internal [`MetricsPublisher`] and a `tokio::sync::broadcast`
//! channel. Producers call `publish_capacity` / `publish_congestion` / etc., which
//! both filter the frame through differential privacy and forward the result to all
//! active subscribers.
//!
//! Subscribers obtain a `broadcast::Receiver<MetricsFrame>` via [`SseFrameBridge::subscribe`]
//! and stream frames out as `text/event-stream` payloads.
//!
//! This module deliberately does NOT speak HTTP. HTTP/SSE serving is the concern
//! of the gateway (or any other transport), which converts each frame into
//! `data: <json>\n\n` chunks. Keeping HTTP out of engauge avoids pulling an HTTP
//! server dependency into the metrics crate.

use std::sync::{Arc, Mutex};

use hypermesh_lib::{NodeId, PrivacyMode};
use tokio::sync::broadcast;

use crate::capacity::CapacityMetrics;

use super::protocol::{
    CongestionSnapshot, EconomicSnapshot, MetricsFrame, RoutingSnapshot,
};
use super::publisher::MetricsPublisher;

/// Default broadcast channel capacity. Slow subscribers that lag beyond this
/// receive [`broadcast::error::RecvError::Lagged`] and the gateway logs it.
pub const DEFAULT_CHANNEL_CAPACITY: usize = 256;

// ---------------------------------------------------------------------------
// SseFrameBridge
// ---------------------------------------------------------------------------

/// Bridge between a producing [`MetricsPublisher`] and N streaming subscribers.
///
/// Cloning the bridge is cheap (Arc internally) and lets multiple producers
/// share the same publisher + broadcast channel.
#[derive(Clone)]
pub struct SseFrameBridge {
    publisher: Arc<Mutex<MetricsPublisher>>,
    tx: broadcast::Sender<MetricsFrame>,
}

impl SseFrameBridge {
    /// Create a new bridge for the given node.
    pub fn new(
        node_id: NodeId,
        privacy_mode: PrivacyMode,
        epsilon: f64,
        publish_interval_secs: u64,
    ) -> Self {
        Self::with_capacity(
            node_id,
            privacy_mode,
            epsilon,
            publish_interval_secs,
            DEFAULT_CHANNEL_CAPACITY,
        )
    }

    /// Create a bridge with a custom broadcast channel capacity.
    pub fn with_capacity(
        node_id: NodeId,
        privacy_mode: PrivacyMode,
        epsilon: f64,
        publish_interval_secs: u64,
        channel_capacity: usize,
    ) -> Self {
        let publisher = MetricsPublisher::new(node_id, privacy_mode, epsilon, publish_interval_secs);
        let (tx, _) = broadcast::channel(channel_capacity.max(1));
        Self {
            publisher: Arc::new(Mutex::new(publisher)),
            tx,
        }
    }

    /// Subscribe to the frame stream. Returns a receiver that yields each frame
    /// produced after subscription.
    pub fn subscribe(&self) -> broadcast::Receiver<MetricsFrame> {
        self.tx.subscribe()
    }

    /// Current number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Publish a capacity frame to all subscribers.
    ///
    /// Returns the filtered frame if it passed the privacy filter and was sent,
    /// or `None` if suppressed by privacy mode.
    pub fn publish_capacity(&self, metrics: &CapacityMetrics) -> Option<MetricsFrame> {
        let frame = {
            let mut guard = self.publisher.lock().ok()?;
            guard.publish_capacity(metrics)
        }?;
        // Ignore send errors: 0 subscribers is a valid steady state.
        let _ = self.tx.send(frame.clone());
        Some(frame)
    }

    /// Publish a congestion snapshot to all subscribers.
    pub fn publish_congestion(&self, snapshot: CongestionSnapshot) -> Option<MetricsFrame> {
        let frame = {
            let mut guard = self.publisher.lock().ok()?;
            guard.publish_congestion(snapshot)
        }?;
        let _ = self.tx.send(frame.clone());
        Some(frame)
    }

    /// Publish a routing snapshot to all subscribers.
    pub fn publish_routing(&self, snapshot: RoutingSnapshot) -> Option<MetricsFrame> {
        let frame = {
            let mut guard = self.publisher.lock().ok()?;
            guard.publish_routing(snapshot)
        }?;
        let _ = self.tx.send(frame.clone());
        Some(frame)
    }

    /// Publish an economic snapshot to all subscribers.
    pub fn publish_economic(&self, snapshot: EconomicSnapshot) -> Option<MetricsFrame> {
        let frame = {
            let mut guard = self.publisher.lock().ok()?;
            guard.publish_economic(snapshot)
        }?;
        let _ = self.tx.send(frame.clone());
        Some(frame)
    }

    /// Publish an arbitrary already-filtered frame directly (bypassing the
    /// internal publisher). Useful for tests and for forwarding frames received
    /// from peers.
    pub fn publish_raw(&self, frame: MetricsFrame) {
        let _ = self.tx.send(frame);
    }

    /// Encode a frame as an SSE `data:` event payload terminated by the
    /// mandatory `\n\n`. Returns `None` if JSON serialization fails (treated
    /// as best-effort — caller may skip and continue).
    pub fn encode_sse(frame: &MetricsFrame) -> Option<String> {
        let json = serde_json::to_string(frame).ok()?;
        Some(format!("data: {json}\n\n"))
    }

    /// Encode a frame as an SSE `data:` event with a named event type.
    pub fn encode_sse_with_event(frame: &MetricsFrame, event_name: &str) -> Option<String> {
        let json = serde_json::to_string(frame).ok()?;
        Some(format!("event: {event_name}\ndata: {json}\n\n"))
    }

    /// Encode an SSE comment line for keep-alive / heartbeat. Per the SSE spec,
    /// lines beginning with `:` are ignored by the client.
    pub fn encode_keepalive() -> String {
        ": keep-alive\n\n".to_string()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::protocol::MetricsPayload;

    fn pub_bridge() -> SseFrameBridge {
        SseFrameBridge::new(
            NodeId::from_public_key(b"sse-bridge-test"),
            PrivacyMode::PUBLIC,
            1.0,
            10,
        )
    }

    fn test_capacity() -> CapacityMetrics {
        CapacityMetrics::new(1024, 100, 2048, 1_000_000, 0.99)
    }

    #[test]
    fn subscribe_zero_initial() {
        let bridge = pub_bridge();
        assert_eq!(bridge.subscriber_count(), 0);
    }

    #[test]
    fn subscriber_count_tracks_active() {
        let bridge = pub_bridge();
        let _rx1 = bridge.subscribe();
        let _rx2 = bridge.subscribe();
        assert_eq!(bridge.subscriber_count(), 2);
        drop(_rx1);
        assert_eq!(bridge.subscriber_count(), 1);
    }

    #[tokio::test]
    async fn publish_capacity_reaches_subscriber() {
        let bridge = pub_bridge();
        let mut rx = bridge.subscribe();

        let frame = bridge
            .publish_capacity(&test_capacity())
            .expect("test: public capacity frame should pass filter");
        assert_eq!(frame.sequence, 0);

        let received = tokio::time::timeout(std::time::Duration::from_millis(50), rx.recv())
            .await
            .expect("test: receive within 50ms")
            .expect("test: broadcast recv ok");
        match received.payload {
            MetricsPayload::Capacity(_) => {}
            other => unreachable!("test: expected Capacity payload, got {other:?}"),
        }
    }

    #[test]
    fn anonymous_publisher_emits_nothing() {
        let bridge = SseFrameBridge::new(
            NodeId::from_public_key(b"anon"),
            PrivacyMode::ANONYMOUS,
            1.0,
            10,
        );
        let result = bridge.publish_capacity(&test_capacity());
        assert!(result.is_none(), "Anonymous must suppress capacity frame");
    }

    #[tokio::test]
    async fn three_frames_are_received_in_order() {
        let bridge = pub_bridge();
        let mut rx = bridge.subscribe();

        for _ in 0..3 {
            bridge
                .publish_capacity(&test_capacity())
                .expect("test: frame published");
        }

        let f1 = rx.recv().await.expect("test: f1");
        let f2 = rx.recv().await.expect("test: f2");
        let f3 = rx.recv().await.expect("test: f3");
        assert_eq!(f1.sequence, 0);
        assert_eq!(f2.sequence, 1);
        assert_eq!(f3.sequence, 2);
    }

    #[test]
    fn encode_sse_produces_data_prefix() {
        let bridge = pub_bridge();
        let frame = bridge
            .publish_capacity(&test_capacity())
            .expect("test: frame");
        let sse = SseFrameBridge::encode_sse(&frame).expect("test: encode_sse");
        assert!(sse.starts_with("data: "), "must start with `data: `");
        assert!(sse.ends_with("\n\n"), "must end with double newline");
    }

    #[test]
    fn encode_sse_with_event_includes_event_line() {
        let bridge = pub_bridge();
        let frame = bridge
            .publish_capacity(&test_capacity())
            .expect("test: frame");
        let sse = SseFrameBridge::encode_sse_with_event(&frame, "capacity")
            .expect("test: encode_sse_with_event");
        assert!(sse.starts_with("event: capacity\n"));
        assert!(sse.contains("data: "));
        assert!(sse.ends_with("\n\n"));
    }

    #[test]
    fn keepalive_is_a_comment() {
        let ka = SseFrameBridge::encode_keepalive();
        assert!(ka.starts_with(":"));
        assert!(ka.ends_with("\n\n"));
    }

    #[tokio::test]
    async fn publish_raw_forwards_without_filter() {
        let bridge = pub_bridge();
        let mut rx = bridge.subscribe();

        let frame = MetricsFrame {
            source_node: NodeId::from_public_key(b"raw"),
            timestamp_us: 100,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Routing(RoutingSnapshot {
                avg_latency_us: 1000,
                throughput_bps: 1_000_000,
                path_count: 1,
                active_connections: 1,
            }),
            sequence: 99,
        };
        bridge.publish_raw(frame.clone());

        let recv = tokio::time::timeout(std::time::Duration::from_millis(50), rx.recv())
            .await
            .expect("test: receive within 50ms")
            .expect("test: broadcast recv ok");
        assert_eq!(recv.sequence, 99);
    }
}
