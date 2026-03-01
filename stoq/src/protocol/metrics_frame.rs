// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ METRICS frame type -- carries Engauge MetricsFrame payloads.
//!
//! Frame type ID: `0xfe000007` (custom extension range).
//! Feature-gated behind the `engauge` feature flag.

use engauge::streaming::{MetricsFrame, ProtocolError};

/// Custom frame type identifier for Engauge metrics.
pub const STOQ_METRICS_FRAME_TYPE: u32 = 0xfe00_0007;

/// Handler for STOQ METRICS frames.
///
/// Deserializes incoming METRICS frames and provides them to registered
/// callbacks. Serializes outgoing [`MetricsFrame`] structs into STOQ frame
/// payloads.
pub struct MetricsFrameHandler {
    /// Callback invoked when a metrics frame is received.
    on_frame: Option<Box<dyn Fn(MetricsFrame) + Send + Sync>>,
}

impl MetricsFrameHandler {
    /// Create a handler with no receive callback.
    pub fn new() -> Self {
        Self { on_frame: None }
    }

    /// Register a callback for received metrics frames.
    pub fn on_receive(&mut self, callback: impl Fn(MetricsFrame) + Send + Sync + 'static) {
        self.on_frame = Some(Box::new(callback));
    }

    /// Encode a [`MetricsFrame`] into a STOQ frame payload.
    pub fn encode_frame(frame: &MetricsFrame) -> Result<Vec<u8>, ProtocolError> {
        frame.encode()
    }

    /// Decode a STOQ frame payload into a [`MetricsFrame`].
    pub fn decode_frame(payload: &[u8]) -> Result<MetricsFrame, ProtocolError> {
        MetricsFrame::decode(payload)
    }

    /// Process a received STOQ frame payload.
    ///
    /// Decodes the payload and, if a callback is registered, invokes it with
    /// the decoded frame. Returns the decoded frame on success.
    pub fn handle_frame(&self, payload: &[u8]) -> Result<MetricsFrame, ProtocolError> {
        let frame = MetricsFrame::decode(payload)?;
        if let Some(ref cb) = self.on_frame {
            cb(frame.clone());
        }
        Ok(frame)
    }
}

impl Default for MetricsFrameHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engauge::streaming::{CapacitySnapshot, MetricsPayload};
    use hypermesh_lib::types::NodeId;
    use hypermesh_lib::PrivacyMode;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    fn test_frame() -> MetricsFrame {
        MetricsFrame {
            source_node: NodeId::from_public_key(b"test-node"),
            timestamp_us: 1000,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Capacity(CapacitySnapshot {
                bytes_served: 1024,
                compute_delivered: 500,
                storage_maintained_bytes: 4096,
                bandwidth_available_bps: 1_000_000,
                uptime_ratio: 0.99,
            }),
            sequence: 1,
        }
    }

    #[test]
    fn frame_type_constant() {
        assert_eq!(STOQ_METRICS_FRAME_TYPE, 0xfe00_0007);
    }

    #[test]
    fn encode_decode_roundtrip() {
        let frame = test_frame();
        let bytes = MetricsFrameHandler::encode_frame(&frame).expect("test: encode");
        let decoded = MetricsFrameHandler::decode_frame(&bytes).expect("test: decode");
        assert_eq!(decoded.source_node, frame.source_node);
        assert_eq!(decoded.sequence, 1);
    }

    #[test]
    fn handle_frame_invokes_callback() {
        let counter = Arc::new(AtomicU64::new(0));
        let counter_clone = counter.clone();
        let mut handler = MetricsFrameHandler::new();
        handler.on_receive(move |_frame| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });
        let bytes = MetricsFrameHandler::encode_frame(&test_frame()).expect("test: encode");
        handler.handle_frame(&bytes).expect("test: handle");
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn handle_invalid_payload() {
        let handler = MetricsFrameHandler::new();
        let result = handler.handle_frame(b"invalid");
        assert!(result.is_err());
    }

    #[test]
    fn default_handler_no_callback() {
        let handler = MetricsFrameHandler::default();
        let bytes = MetricsFrameHandler::encode_frame(&test_frame()).expect("test: encode");
        let frame = handler
            .handle_frame(&bytes)
            .expect("test: handle without callback");
        assert_eq!(frame.sequence, 1);
    }
}
