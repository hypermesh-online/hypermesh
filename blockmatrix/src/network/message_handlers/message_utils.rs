// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Utility handlers — sync reply, DNS peer resolution, metrics, gossip.

use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;
use uuid::Uuid;

use crate::network::stoq_integration::MatrixMessage;

use super::super::PeerContext;
use super::dns_protocol::{
    select_canonical, DistributedDnsQuery, DistributedDnsResponse,
};
use super::protocol::{
    TAG_DNS_QUERY, TAG_DNS_RESOLVE, TAG_DNS_RESOLVE_RESPONSE, TAG_DNS_RESPONSE, TAG_SYNC_MESSAGE,
};

/// Resolve a DNS name by querying connected peers (network fallback).
///
/// Used when local DNS resolution fails. Queries up to 6 peers and
/// returns the first successful response. Similar to shard fetch fallback.
pub async fn resolve_from_network(
    name: &str,
    peers: &[Arc<stoq::Connection>],
) -> Option<std::net::IpAddr> {
    let max_peers = 6.min(peers.len());
    if max_peers == 0 {
        return None;
    }

    tracing::debug!(
        name = name,
        peers = max_peers,
        "DNS network resolution attempt",
    );

    for peer_conn in &peers[..max_peers] {
        match try_resolve_from_peer(name, peer_conn).await {
            Some(addr) => return Some(addr),
            None => continue,
        }
    }

    None
}

/// Phase H.1 — distributed DNS resolution with conflict resolution.
///
/// Broadcasts a [`DistributedDnsQuery`] to up to `max_peers` connected
/// peers, collects [`DistributedDnsResponse`]s within `deadline`, and
/// returns the canonical winner per the H.1 ordering tuple
/// `(foundation_grant_present DESC, registration_timestamp ASC,
/// chain_height DESC)`.
///
/// Returns `None` when no peer responded with non-empty records
/// (i.e., no peer holds the name) — callers fall back to a `not found`
/// response. Empty-records responses are still collected because they
/// participate in tiebreak (in practice they only "win" when no peer
/// has any record, and `None` is returned in that case).
pub async fn distributed_dns_resolve(
    name: &str,
    peers: &[Arc<stoq::Connection>],
    deadline: Duration,
) -> Option<DistributedDnsResponse> {
    let max_peers = 8.min(peers.len());
    if max_peers == 0 {
        return None;
    }

    let query = DistributedDnsQuery {
        query_id: Uuid::new_v4(),
        domain_name: name.to_string(),
    };
    let query_bytes = match serde_json::to_vec(&query) {
        Ok(b) => b,
        Err(e) => {
            debug!("Failed to serialize H.1 DNS query: {}", e);
            return None;
        }
    };

    debug!(
        name = name,
        peers = max_peers,
        "H.1 distributed DNS query starting"
    );

    // Issue queries in parallel and gather responses up to deadline.
    let mut handles = Vec::with_capacity(max_peers);
    for peer_conn in &peers[..max_peers] {
        let conn = peer_conn.clone();
        let query_bytes = query_bytes.clone();
        let expected_id = query.query_id;
        handles.push(tokio::spawn(async move {
            try_query_peer_h1(&conn, &query_bytes, expected_id).await
        }));
    }

    let mut responses: Vec<DistributedDnsResponse> = Vec::new();
    let collection = async {
        for h in handles {
            if let Ok(Some(r)) = h.await {
                responses.push(r);
            }
        }
    };
    let _ = tokio::time::timeout(deadline, collection).await;

    if responses.is_empty() {
        return None;
    }

    // Drop empty-records responses unless every response is empty.
    // Empty-records means "I don't have this name" — keep the chain
    // metadata for diagnostics but prefer a peer with actual records.
    let any_with_records = responses.iter().any(|r| !r.records.is_empty());
    if any_with_records {
        responses.retain(|r| !r.records.is_empty());
    }

    select_canonical(&responses).cloned()
}

/// H.1 — query a single peer with `DistributedDnsQuery`, return the
/// matching `DistributedDnsResponse` if `query_id` lines up.
async fn try_query_peer_h1(
    conn: &stoq::Connection,
    query_bytes: &[u8],
    expected_query_id: Uuid,
) -> Option<DistributedDnsResponse> {
    let mut stream = conn.open_stream().await.ok()?;

    let mut frame = Vec::with_capacity(1 + query_bytes.len());
    frame.push(TAG_DNS_QUERY);
    frame.extend_from_slice(query_bytes);
    stream.send(&frame).await.ok()?;

    let raw = tokio::time::timeout(Duration::from_secs(4), stream.receive())
        .await
        .ok()?
        .ok()?;

    if raw.len() < 2 || raw[0] != TAG_DNS_RESPONSE {
        return None;
    }
    let resp: DistributedDnsResponse = serde_json::from_slice(&raw[1..]).ok()?;
    if resp.query_id != expected_query_id {
        debug!(
            "H.1 DNS response query_id mismatch: expected {}, got {}",
            expected_query_id, resp.query_id
        );
        return None;
    }
    Some(resp)
}

/// Try resolving a DNS name from a single peer.
///
/// Opens a new stream, sends TAG_DNS_RESOLVE + name, reads response.
async fn try_resolve_from_peer(
    name: &str,
    conn: &stoq::Connection,
) -> Option<std::net::IpAddr> {
    let mut stream = conn.open_stream().await.ok()?;

    let mut request = vec![TAG_DNS_RESOLVE];
    request.extend_from_slice(name.as_bytes());
    stream.send(&request).await.ok()?;

    let response = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        stream.receive(),
    )
    .await
    .ok()?
    .ok()?;

    // Response format: [TAG_DNS_RESOLVE_RESPONSE][addr_string_bytes]
    if response.len() <= 1 {
        return None; // Empty response = name not found on peer
    }

    if response[0] != TAG_DNS_RESOLVE_RESPONSE {
        return None;
    }

    let addr_str = std::str::from_utf8(&response[1..]).ok()?;
    addr_str.parse::<std::net::IpAddr>().ok()
}

/// Serialize and send a sync reply on the given stream.
pub(super) async fn send_sync_reply(stream: &mut stoq::Stream, reply_msg: &MatrixMessage) {
    let reply_data = match serde_json::to_vec(reply_msg) {
        Ok(d) => d,
        Err(e) => {
            debug!("Failed to serialize sync reply: {}", e);
            return;
        }
    };
    let mut tagged = Vec::with_capacity(1 + reply_data.len());
    tagged.push(TAG_SYNC_MESSAGE);
    tagged.extend_from_slice(&reply_data);
    if let Err(e) = stream.send(&tagged).await {
        debug!("Failed to send sync reply: {}", e);
    }
}

// ── Metrics handler ─────────────────────────────────────────────

/// Handle an incoming metrics stream (discriminator 0x02).
///
/// Reads the frame payload, logs it, and when the `intelligence` feature is
/// enabled feeds it into the engauge `MetricsIngestionPipeline` attached to
/// the `PeerContext`.
pub(super) async fn handle_metrics_connection(
    stream: &mut stoq::Stream,
    peer_ctx: Option<Arc<PeerContext>>,
) -> Result<()> {
    let data = match stream.receive().await {
        Ok(d) if !d.is_empty() => d.to_vec(),
        Ok(_) => return Ok(()),
        Err(e) => return Err(anyhow!("Failed to read metrics frame: {e}")),
    };

    // Validate it's parseable JSON (MetricsFrame format)
    let frame_json = match serde_json::from_slice::<serde_json::Value>(&data) {
        Ok(frame) => {
            let source = frame.get("source_node")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            debug!(
                "Received metrics frame from {} ({} bytes)",
                &source[..8.min(source.len())],
                data.len(),
            );
            Some(frame)
        }
        Err(e) => {
            debug!("Invalid metrics frame ({} bytes): {}", data.len(), e);
            None
        }
    };

    // Feed into engauge ingestion pipeline when available (H2).
    #[cfg(feature = "intelligence")]
    if let (Some(ctx), Some(_json)) = (&peer_ctx, &frame_json) {
        if let Some(ref ingestion) = ctx.engauge_ingestion {
            // Try to deserialize the JSON into an engauge MetricsFrame.
            // MetricsReporter builds JSON manually, so this may fail for
            // older formats — log and continue rather than crashing.
            match serde_json::from_value::<engauge::MetricsFrame>(_json.clone()) {
                Ok(metrics_frame) => {
                    match ingestion.lock() {
                        Ok(mut pipeline) => {
                            pipeline.ingest(metrics_frame);
                            debug!("Ingested metrics frame into engauge pipeline");
                        }
                        Err(e) => {
                            debug!("Failed to lock ingestion pipeline: {e}");
                        }
                    }
                }
                Err(e) => {
                    debug!("Metrics frame not engauge-compatible (ok for alpha): {e}");
                }
            }
        }
    }

    // Suppress unused warnings when intelligence feature is off.
    let _ = (&peer_ctx, &frame_json);
    Ok(())
}

// ── Gossip handler ──────────────────────────────────────────────

/// Handle an incoming gossip stream (discriminator 0x03).
///
/// Reads the gossip message payload and processes it through the
/// gossip protocol if a `PeerContext` is available. The gossip
/// protocol merges newer entries into local state.
pub(super) async fn handle_gossip_connection(
    stream: &mut stoq::Stream,
    peer_ctx: Option<Arc<PeerContext>>,
) -> Result<()> {
    let data = match stream.receive().await {
        Ok(d) if !d.is_empty() => d.to_vec(),
        Ok(_) => return Ok(()),
        Err(e) => return Err(anyhow!("Failed to read gossip message: {e}")),
    };

    let msg: super::super::gossip::GossipMessage = match serde_json::from_slice(&data) {
        Ok(m) => m,
        Err(e) => {
            debug!("Invalid gossip message ({} bytes): {}", data.len(), e);
            return Ok(());
        }
    };

    let sender_short = msg.sender[..8.min(msg.sender.len())].to_string();
    let entry_count = msg.entries.len();

    if let Some(ctx) = peer_ctx {
        if let Some(ref gossip) = ctx.gossip_protocol {
            let updated = gossip.process_incoming(msg).await;
            debug!(
                "Gossip from {}: {} entries, {} updated",
                sender_short, entry_count, updated,
            );
        } else {
            debug!("Gossip from {} but no gossip protocol configured", sender_short);
        }
    } else {
        debug!("Gossip from {} but no PeerContext — dropping", sender_short);
    }

    Ok(())
}

