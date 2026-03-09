// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! UDP metrics ingestion listener.
//!
//! Receives [`MetricsFrame`] JSON datagrams from co-located services
//! (e.g., the blockmatrix node binary) and feeds them into the
//! [`MetricsIngestionPipeline`]. This is a lightweight, fire-and-forget
//! ingestion path that avoids the overhead of a full QUIC handshake
//! for local metrics delivery.

use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::api::stoq_api::EngaugeAppState;
use crate::ingestion::MetricsIngestionPipeline;
use crate::streaming::protocol::MetricsFrame;

/// Default UDP bind address for metrics ingestion.
pub const DEFAULT_UDP_BIND: &str = "[::1]:9297";

/// Maximum datagram size for a single MetricsFrame (64 KB).
const MAX_DATAGRAM_SIZE: usize = 65536;

/// Configuration for the UDP ingestion listener.
#[derive(Debug, Clone)]
pub struct UdpIngestConfig {
    /// UDP bind address (default: `[::1]:9297`).
    pub bind_address: String,
}

impl Default for UdpIngestConfig {
    fn default() -> Self {
        Self {
            bind_address: DEFAULT_UDP_BIND.to_string(),
        }
    }
}

/// Run the UDP metrics ingestion listener.
///
/// Binds to the configured address and loops forever, decoding incoming
/// datagrams as [`MetricsFrame`] JSON and feeding them into the pipeline.
/// Invalid or oversized datagrams are silently dropped with a debug log.
pub async fn run_udp_ingest(
    config: UdpIngestConfig,
    pipeline: Arc<Mutex<MetricsIngestionPipeline>>,
    app_state: Arc<EngaugeAppState>,
) -> std::io::Result<()> {
    let socket = UdpSocket::bind(&config.bind_address).await?;
    info!(
        "Engauge UDP metrics listener bound on {}",
        config.bind_address
    );

    let mut buf = vec![0u8; MAX_DATAGRAM_SIZE];

    loop {
        let (len, src) = match socket.recv_from(&mut buf).await {
            Ok(pair) => pair,
            Err(e) => {
                warn!("UDP recv error: {e}");
                continue;
            }
        };

        let data = &buf[..len];
        match serde_json::from_slice::<MetricsFrame>(data) {
            Ok(frame) => {
                debug!(
                    "Ingested MetricsFrame from {} (node={}, seq={})",
                    src, frame.source_node, frame.sequence
                );
                app_state.increment_metrics();
                pipeline.lock().await.ingest(frame);
            }
            Err(e) => {
                debug!("Dropped invalid UDP datagram from {src} ({len} bytes): {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestion::IngestionConfig;
    use crate::streaming::protocol::{CapacitySnapshot, MetricsPayload};
    use hypermesh_lib::{NodeId, PrivacyMode};

    fn test_capacity_frame(seq: u64) -> MetricsFrame {
        MetricsFrame {
            source_node: NodeId::from_public_key(b"udp-test-node"),
            timestamp_us: 1_000_000 + seq,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Capacity(CapacitySnapshot {
                bytes_served: 1024 * seq,
                compute_delivered: 500,
                storage_maintained_bytes: 2048,
                bandwidth_available_bps: 1_000_000,
                uptime_ratio: 0.99,
            }),
            sequence: seq,
        }
    }

    #[test]
    fn capacity_frame_serializes_under_max_datagram() {
        let frame = test_capacity_frame(1);
        let bytes = serde_json::to_vec(&frame).expect("test: serialize frame");
        assert!(
            bytes.len() < MAX_DATAGRAM_SIZE,
            "frame {} bytes exceeds {} limit",
            bytes.len(),
            MAX_DATAGRAM_SIZE
        );
    }

    #[tokio::test]
    async fn udp_ingest_receives_frame() {
        // Bind listener on OS-assigned port
        let listener = UdpSocket::bind("[::1]:0")
            .await
            .expect("test: bind listener");
        let listen_addr = listener.local_addr().expect("test: local addr");

        let pipeline = Arc::new(Mutex::new(MetricsIngestionPipeline::new(
            IngestionConfig {
                enable_privacy_filter: false,
                ..Default::default()
            },
        )));
        let app_state = Arc::new(EngaugeAppState::new());

        let pipeline_clone = pipeline.clone();
        let state_clone = app_state.clone();

        // Wrap the listener socket into run_udp_ingest manually
        // (we can't reuse run_udp_ingest because it binds its own socket,
        //  so we test the decode+ingest logic directly)
        let handle = tokio::spawn(async move {
            let mut buf = vec![0u8; MAX_DATAGRAM_SIZE];
            let (len, _src) = listener.recv_from(&mut buf).await.expect("test: recv");
            let frame: MetricsFrame =
                serde_json::from_slice(&buf[..len]).expect("test: decode frame");
            state_clone.increment_metrics();
            pipeline_clone.lock().await.ingest(frame);
        });

        // Send a frame via UDP
        let sender = UdpSocket::bind("[::1]:0")
            .await
            .expect("test: bind sender");
        let frame = test_capacity_frame(42);
        let bytes = serde_json::to_vec(&frame).expect("test: serialize");
        sender
            .send_to(&bytes, listen_addr)
            .await
            .expect("test: send");

        // Wait for the receiver to process
        tokio::time::timeout(std::time::Duration::from_secs(2), handle)
            .await
            .expect("test: timeout")
            .expect("test: join");

        // Verify ingestion
        let pl = pipeline.lock().await;
        assert_eq!(pl.capacity_entry_count(), 1);
        assert_eq!(pl.stats().frames_ingested, 1);
        assert_eq!(
            app_state
                .total_metrics
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
    }
}
