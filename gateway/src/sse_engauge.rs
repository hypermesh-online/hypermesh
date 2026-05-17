// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Engauge SSE streaming endpoint (Phase M.6b).
//!
//! This module hosts the gateway-side Server-Sent Events stream for engauge
//! telemetry. Clients connect via `GET /api/v1/blockmatrix/engauge/stream`
//! (or the alias `/api/v1/engauge/stream`) carrying an `X-HyperMesh-Capability`
//! header (validated only when capability tokens land; in alpha we accept
//! anonymous reads).
//!
//! ## Design rationale
//!
//! The engauge daemon currently speaks a custom QUIC stream protocol (JSON
//! request/response over `quinn` bidi streams) and does **not** speak HTTP/3.
//! Forwarding HTTP/SSE through the existing `Http3Proxy` would require engauge
//! to terminate HTTP, which it does not.
//!
//! Until engauge gains a full HTTP/3 listener, the gateway hosts the SSE
//! endpoint locally. It owns an [`engauge::streaming::SseFrameBridge`] and
//! drives it with alpha-honest periodic frames generated from a stub source.
//! A future revision can replace the stub source with frames pulled from the
//! engauge daemon over its native stream protocol.
//!
//! Streaming semantics:
//! - Each `MetricsFrame` is encoded as `data: <json>\n\n`.
//! - A heartbeat comment is emitted every 15s to keep the connection alive.
//! - The handler returns a [`SseResponse`] that the HTTP/3 transport layer
//!   pumps chunk-by-chunk into the response body.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use bytes::Bytes;
use engauge::capacity::CapacityMetrics;
use engauge::streaming::{
    CongestionSnapshot, EconomicSnapshot, MetricsFrame, RoutingSnapshot, SseFrameBridge,
};
use http::{header, HeaderMap, Response, StatusCode};
use hypermesh_lib::{NodeId, PrivacyMode};
use tokio::sync::broadcast::{self, error::RecvError};
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// SSE path under `/api/v1/blockmatrix` (the spec-mandated location).
pub const SSE_PATH_BLOCKMATRIX: &str = "/api/v1/blockmatrix/engauge/stream";
/// Alias under `/api/v1/engauge` (convenience for direct consumers).
pub const SSE_PATH_ENGAUGE: &str = "/api/v1/engauge/stream";

/// Header consulted for capability tokens.
pub const CAPABILITY_HEADER: &str = "x-hypermesh-capability";

/// Period at which the gateway emits alpha-honest stub frames.
const STUB_PUBLISH_INTERVAL: Duration = Duration::from_secs(5);
/// Period at which the gateway emits SSE keep-alive comments.
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Returns true if `path` targets the engauge SSE endpoint.
pub fn is_sse_path(path: &str) -> bool {
    path == SSE_PATH_BLOCKMATRIX || path == SSE_PATH_ENGAUGE
}

/// Outcome of an SSE handshake: either the streaming response (headers + a
/// receiver yielding `text/event-stream` chunks) or an error response.
pub enum SseHandshake {
    /// Successful handshake — start streaming.
    Stream(SseResponse),
    /// Error response — send back and close.
    Error(Response<Bytes>),
}

/// A streaming SSE response composed of HTTP response parts and a chunk receiver.
///
/// The HTTP transport must:
/// 1. Send `parts` as the response head.
/// 2. Loop receiving from `chunks` and writing each `Bytes` to the body.
/// 3. Stop when the receiver closes (None) or the client disconnects.
pub struct SseResponse {
    pub parts: http::response::Parts,
    pub chunks: tokio::sync::mpsc::Receiver<Bytes>,
}

// ---------------------------------------------------------------------------
// EngaugeStream
// ---------------------------------------------------------------------------

/// Shared engauge SSE streaming state held by the gateway router.
///
/// Holds a single [`SseFrameBridge`] and spawns a background task that emits
/// periodic alpha-honest frames. Cloning is cheap (Arc-wrapped).
#[derive(Clone)]
pub struct EngaugeStream {
    bridge: SseFrameBridge,
    require_capability: Arc<std::sync::atomic::AtomicBool>,
}

impl EngaugeStream {
    /// Build a new engauge SSE stream backed by an internal bridge.
    ///
    /// In alpha, `require_capability` is `false` (anonymous reads accepted).
    /// When the capability-token system lands, callers can flip this to true.
    pub fn new(node_id: NodeId, privacy_mode: PrivacyMode) -> Self {
        let bridge = SseFrameBridge::new(node_id, privacy_mode, 1.0, STUB_PUBLISH_INTERVAL.as_secs());
        Self {
            bridge,
            require_capability: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Build a stream from an externally-provided bridge (useful for tests
    /// or when the daemon supplies its own publisher).
    pub fn from_bridge(bridge: SseFrameBridge) -> Self {
        Self {
            bridge,
            require_capability: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Toggle capability-token enforcement.
    pub fn set_require_capability(&self, required: bool) {
        self.require_capability
            .store(required, std::sync::atomic::Ordering::Relaxed);
    }

    /// Returns the underlying bridge so producers (e.g. engauge integration)
    /// can publish frames into the stream.
    pub fn bridge(&self) -> &SseFrameBridge {
        &self.bridge
    }

    /// Spawn the background frame producer. Returns the task handle so it can
    /// be aborted on shutdown.
    pub fn spawn_stub_producer(&self) -> tokio::task::JoinHandle<()> {
        let bridge = self.bridge.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(STUB_PUBLISH_INTERVAL);
            // First tick fires immediately — skip it so the producer doesn't
            // race ahead of any subscribers.
            interval.tick().await;
            let mut tick: u64 = 0;
            loop {
                interval.tick().await;
                tick = tick.wrapping_add(1);
                // Alpha-honest stub: emit a Capacity frame every tick. When the
                // real engauge integration lands, replace this with frames
                // pulled from the engauge daemon's publisher.
                let metrics = CapacityMetrics::new(
                    tick.saturating_mul(1024),
                    tick.saturating_mul(100),
                    tick.saturating_mul(2048),
                    1_000_000_000,
                    0.99,
                );
                if let Some(frame) = bridge.publish_capacity(&metrics) {
                    debug!("engauge SSE: published stub frame seq={}", frame.sequence);
                }
                // Every 4 ticks (~20s) also publish a congestion sample.
                if tick % 4 == 0 {
                    let _ = bridge.publish_congestion(CongestionSnapshot {
                        buffer_fullness_ratio: 0.1,
                        queue_depth: 0,
                        dropped_packets_epoch: 0,
                        avg_queue_wait_us: 0,
                    });
                }
                // Every 6 ticks (~30s) emit routing and economic frames.
                if tick % 6 == 0 {
                    let _ = bridge.publish_routing(RoutingSnapshot {
                        avg_latency_us: 5_000,
                        throughput_bps: 1_000_000_000,
                        path_count: 1,
                        active_connections: bridge.subscriber_count() as u32,
                    });
                    let _ = bridge.publish_economic(EconomicSnapshot::default());
                }
            }
        })
    }

    /// Handle an incoming SSE connection. Validates auth, opens a subscriber
    /// on the broadcast channel, and returns an [`SseResponse`] that the
    /// HTTP transport must drain into the wire.
    pub fn handle(&self, headers: &HeaderMap) -> SseHandshake {
        // Capability-token enforcement (alpha: off by default).
        if self
            .require_capability
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            let cap = headers
                .get(CAPABILITY_HEADER)
                .and_then(|v| v.to_str().ok())
                .map(str::trim)
                .filter(|v| !v.is_empty());
            if cap.is_none() {
                return SseHandshake::Error(error_response(
                    StatusCode::UNAUTHORIZED,
                    "missing X-HyperMesh-Capability header",
                ));
            }
        }

        let rx = self.bridge.subscribe();

        // Bounded buffer of pre-encoded SSE chunks. 64 is plenty: the producer
        // emits every 5s, the keep-alive every 15s; a slow consumer will see
        // backpressure long before we approach this.
        let (chunk_tx, chunk_rx) = tokio::sync::mpsc::channel::<Bytes>(64);

        tokio::spawn(pump_frames(rx, chunk_tx));

        let response = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .header(header::CACHE_CONTROL, "no-cache, no-transform")
            .header("x-accel-buffering", "no")
            .header(header::CONNECTION, "keep-alive")
            .body(())
            .expect("static SSE headers always build");

        let (parts, _) = response.into_parts();

        info!("engauge SSE subscriber connected, total={}", self.bridge.subscriber_count());

        SseHandshake::Stream(SseResponse { parts, chunks: chunk_rx })
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Pump frames from a broadcast receiver into an mpsc chunk sender, encoding
/// each frame as an SSE `data:` event. Emits keep-alive comments on a timer.
async fn pump_frames(
    mut rx: broadcast::Receiver<MetricsFrame>,
    chunk_tx: tokio::sync::mpsc::Sender<Bytes>,
) {
    // Initial comment so clients see "open" immediately even if no frame yet.
    if chunk_tx
        .send(Bytes::from(SseFrameBridge::encode_keepalive()))
        .await
        .is_err()
    {
        return;
    }

    let mut keepalive = tokio::time::interval(KEEPALIVE_INTERVAL);
    // Skip the immediate first tick; the open comment above already primed
    // the connection.
    keepalive.tick().await;

    loop {
        tokio::select! {
            recv = rx.recv() => match recv {
                Ok(frame) => {
                    if let Some(payload) = SseFrameBridge::encode_sse(&frame) {
                        if chunk_tx.send(Bytes::from(payload)).await.is_err() {
                            debug!("engauge SSE: client disconnected, ending pump");
                            return;
                        }
                    }
                }
                Err(RecvError::Lagged(skipped)) => {
                    warn!("engauge SSE subscriber lagged by {skipped} frames");
                    // Emit a comment hinting the client to expect a gap.
                    let comment = format!(": lag {skipped}\n\n");
                    if chunk_tx.send(Bytes::from(comment)).await.is_err() {
                        return;
                    }
                }
                Err(RecvError::Closed) => {
                    debug!("engauge SSE: broadcast channel closed");
                    return;
                }
            },
            _ = keepalive.tick() => {
                if chunk_tx
                    .send(Bytes::from(SseFrameBridge::encode_keepalive()))
                    .await
                    .is_err()
                {
                    debug!("engauge SSE: client disconnected during keepalive");
                    return;
                }
            }
        }
    }
}

fn error_response(status: StatusCode, message: &str) -> Response<Bytes> {
    let body = serde_json::json!({ "error": message });
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Bytes::from(serde_json::to_vec(&body).unwrap_or_default()))
        .unwrap_or_else(|_| Response::new(Bytes::from_static(b"{}")))
}

/// Helper for constructing a `Result<()>` indicating SSE handshake completion.
/// (Re-exported for the HTTP transport layer.)
pub fn into_response_head(parts: http::response::Parts) -> Result<Response<()>> {
    Ok(Response::from_parts(parts, ()))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;

    fn make_stream() -> EngaugeStream {
        EngaugeStream::new(NodeId::from_public_key(b"gw-test"), PrivacyMode::PUBLIC)
    }

    #[test]
    fn path_detection_matches_blockmatrix_and_alias() {
        assert!(is_sse_path("/api/v1/blockmatrix/engauge/stream"));
        assert!(is_sse_path("/api/v1/engauge/stream"));
        assert!(!is_sse_path("/api/v1/blockmatrix/engauge/foo"));
        assert!(!is_sse_path("/api/v1/blockmatrix"));
        assert!(!is_sse_path("/"));
    }

    #[tokio::test]
    async fn handshake_accepts_when_capability_not_required() {
        let stream = make_stream();
        let headers = HeaderMap::new();
        match stream.handle(&headers) {
            SseHandshake::Stream(resp) => {
                assert_eq!(resp.parts.status, StatusCode::OK);
                assert_eq!(
                    resp.parts.headers.get(header::CONTENT_TYPE),
                    Some(&HeaderValue::from_static("text/event-stream"))
                );
            }
            SseHandshake::Error(_) => unreachable!("test: should accept anonymous in alpha"),
        }
    }

    #[tokio::test]
    async fn handshake_rejects_when_capability_required_and_missing() {
        let stream = make_stream();
        stream.set_require_capability(true);
        let headers = HeaderMap::new();
        match stream.handle(&headers) {
            SseHandshake::Error(resp) => {
                assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
            }
            SseHandshake::Stream(_) => unreachable!("test: should reject without capability"),
        }
    }

    #[tokio::test]
    async fn handshake_accepts_with_capability_header() {
        let stream = make_stream();
        stream.set_require_capability(true);
        let mut headers = HeaderMap::new();
        headers.insert(CAPABILITY_HEADER, HeaderValue::from_static("test-token"));
        match stream.handle(&headers) {
            SseHandshake::Stream(_) => {}
            SseHandshake::Error(_) => unreachable!("test: should accept with capability header"),
        }
    }

    #[tokio::test]
    async fn stream_emits_initial_keepalive_then_published_frame() {
        let stream = make_stream();
        let headers = HeaderMap::new();
        let mut resp = match stream.handle(&headers) {
            SseHandshake::Stream(r) => r,
            SseHandshake::Error(_) => unreachable!("test: should accept"),
        };

        // Initial keepalive should arrive promptly.
        let first = tokio::time::timeout(Duration::from_millis(100), resp.chunks.recv())
            .await
            .expect("test: keepalive within 100ms")
            .expect("test: chunk present");
        let s = std::str::from_utf8(&first).expect("test: utf8");
        assert!(s.starts_with(":"), "first chunk should be SSE comment");

        // Now publish a frame and expect it on the channel.
        let metrics = CapacityMetrics::new(1, 1, 1, 1, 0.5);
        stream
            .bridge()
            .publish_capacity(&metrics)
            .expect("test: frame published");

        let next = tokio::time::timeout(Duration::from_millis(100), resp.chunks.recv())
            .await
            .expect("test: frame within 100ms")
            .expect("test: chunk");
        let payload = std::str::from_utf8(&next).expect("test: utf8");
        assert!(payload.starts_with("data: "), "frame chunk should start with `data: `");
        assert!(payload.ends_with("\n\n"));
    }

    #[tokio::test]
    async fn stub_producer_emits_frames_on_interval() {
        let stream = make_stream();
        let mut rx = stream.bridge().subscribe();
        let handle = stream.spawn_stub_producer();

        // STUB_PUBLISH_INTERVAL is 5s; wait up to 6s for the first frame.
        let first = tokio::time::timeout(Duration::from_secs(6), rx.recv())
            .await
            .expect("test: producer should emit within 6s")
            .expect("test: recv ok");
        match first.payload {
            engauge::streaming::MetricsPayload::Capacity(_) => {}
            other => unreachable!("test: expected Capacity, got {other:?}"),
        }
        handle.abort();
    }
}
