// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Sync/reflector, shard dispatch/announce, and block-fetch handlers.

use anyhow::{anyhow, Result};
use tracing::{debug, info, warn};

use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::shard_transport;
use crate::network::stoq_integration::MatrixMessage;
use crate::network::sync_dispatch;
use hypermesh_lib::ContentHash;

use super::super::PeerContext;
use super::dns_protocol::{DistributedDnsQuery, DistributedDnsResponse};
use super::message_utils::send_sync_reply;
use super::protocol::{TAG_DNS_RESOLVE_RESPONSE, TAG_DNS_RESPONSE};

// ── Shard handler ────────────────────────────────────────────────────

/// Dispatch a shard send/fetch message to the shard store.
pub(super) async fn handle_shard_dispatch(
    data: &[u8],
    stream: &mut stoq::Stream,
    ctx: &PeerContext,
) {
    match shard_transport::handle_shard_message(data, &ctx.shard_store).await {
        Ok(Some(response_data)) => {
            if let Err(e) = stream.send(&response_data).await {
                warn!("Failed to send shard response: {}", e);
            }
        }
        Ok(None) => {}
        Err(e) => {
            warn!("Shard message error: {}", e);
        }
    }
}

/// Record shard fetch demand for engauge swarm analytics.
///
/// Extracts the shard_id from a SHARD_FETCH message (tag 0x02 + 32-byte hash)
/// and records it in the swarm demand tracker. If the message is too short
/// to contain a valid shard_id, the request is silently skipped.
pub(super) async fn record_shard_demand(data: &[u8], peer_node_id: &str, ctx: &PeerContext) {
    // SHARD_FETCH format: tag(1) + shard_id(32)
    if data.len() < 33 {
        return;
    }
    let mut shard_id_bytes = [0u8; 32];
    shard_id_bytes.copy_from_slice(&data[1..33]);
    let shard_id = ContentHash(shard_id_bytes);

    ctx.swarm_demand_tracker
        .record_fetch(shard_id, peer_node_id)
        .await;
}

// ── Shard announce handler ───────────────────────────────────────────

/// Handle a shard availability announcement (tag 0x04).
///
/// Wire format: tag(1) + count(4 bytes u32 LE) + [shard_hash(32)]...
/// Updates the ShardLocationIndex with the announcing peer's available shards.
pub(super) async fn handle_shard_announce(
    data: &[u8],
    peer_node_id: &str,
    ctx: &PeerContext,
) {
    if data.len() < 5 {
        return;
    }
    let count = u32::from_le_bytes([data[1], data[2], data[3], data[4]]) as usize;
    let expected_len = 5 + count * 32;
    if data.len() < expected_len {
        let short_id = &peer_node_id[..8.min(peer_node_id.len())];
        warn!(
            "Shard announce from {} truncated: expected {} bytes, got {}",
            short_id, expected_len, data.len(),
        );
        return;
    }

    let mut shard_ids = Vec::with_capacity(count);
    for i in 0..count {
        let offset = 5 + i * 32;
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&data[offset..offset + 32]);
        shard_ids.push(ContentHash(hash));
    }

    if let Some(ref index) = ctx.shard_location_index {
        index.register_provider(peer_node_id, &shard_ids).await;
        debug!(
            "Shard announce from {}: {} shard(s) registered",
            &peer_node_id[..8.min(peer_node_id.len())],
            count,
        );
    }
}

// ── Shard-locate handler (A2 upstream tracker fallback) ──────────────

/// Handle a shard-locate request (tag 0x52).
///
/// Wire format: `[TAG_SHARD_LOCATE][content_hash(32)]`.
///
/// This peer acts as a mini-tracker: it answers with the provider node_ids it
/// knows for `content_hash` — from its own `ShardLocationIndex` (live mirrors
/// it has learned via `TAG_SHARD_ANNOUNCE`) plus ITSELF when it holds the shard
/// locally. This is the shard analog of the DNS upstream query
/// (`dns/resolver.rs` upstream fallback): a node whose local store + connected
/// peers all miss asks an upstream peer "who has X?".
///
/// The response is `[TAG_SHARD_LOCATE_RESPONSE][count][len+node_id]...`. An
/// empty list is still meaningful — it tells the asker this peer knows no
/// provider, so they can move on.
pub(super) async fn handle_shard_locate(
    data: &[u8],
    stream: &mut stoq::Stream,
    peer_node_id: &str,
    ctx: &PeerContext,
) {
    use crate::network::swarm_provider::{
        build_shard_locate_response, parse_shard_locate_request,
    };

    let short = &peer_node_id[..8.min(peer_node_id.len())];

    let content_hash = match parse_shard_locate_request(data) {
        Some(h) => h,
        None => {
            debug!("Malformed shard-locate request from {}", short);
            return;
        }
    };

    // Providers known to THIS peer from its live-mirror index.
    let mut providers: Vec<String> = match ctx.shard_location_index {
        Some(ref index) => index.get_providers(&content_hash).await,
        None => Vec::new(),
    };

    // If we hold the shard ourselves, advertise our own node id — we are a
    // provider too ("hosts ARE mirrors ARE trackers").
    if ctx.shard_store.has(&content_hash).await && !providers.contains(&ctx.node_id) {
        providers.push(ctx.node_id.clone());
    }

    debug!(
        "Shard-locate from {}: {} provider(s) for {}",
        short,
        providers.len(),
        hex::encode(&content_hash.0[..8]),
    );

    let response = build_shard_locate_response(&providers);
    if let Err(e) = stream.send(&response).await {
        debug!("Failed to send shard-locate response to {}: {}", short, e);
    }
}

// ── Key rotation handler ────────────────────────────────────────────

/// Handle a key rotation announcement (tag 0x08).
///
/// Informational only — not auth-gated. Records the rotation in the
/// peer's rotation chain for continuity verification and split-brain
/// detection. Does NOT reject the peer on discontinuity (bilateral,
/// no consensus — warn only).
pub(super) async fn handle_key_rotation(
    data: &[u8],
    peer_node_id: &str,
    ctx: &PeerContext,
) -> Result<()> {
    if data.len() < 2 {
        return Err(anyhow!("Key rotation message too short"));
    }
    let msg_data = &data[1..];
    let rotation: serde_json::Value = serde_json::from_slice(msg_data)
        .map_err(|e| anyhow!("Invalid key rotation JSON: {e}"))?;

    let old_fp = rotation
        .get("old_key_fingerprint")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let new_fp = rotation
        .get("new_key_fingerprint")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let reason = rotation
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let short_peer = &peer_node_id[..8.min(peer_node_id.len())];
    let short_old = &old_fp[..8.min(old_fp.len())];
    let short_new = &new_fp[..8.min(new_fp.len())];
    info!(
        peer = %short_peer,
        reason = reason,
        "Received key rotation: {} -> {}",
        short_old,
        short_new,
    );

    // Store in peer's rotation chain
    if let Some(ref chains) = ctx.rotation_chains {
        let mut chains = chains.write().await;
        let chain = chains
            .entry(peer_node_id.to_string())
            .or_insert_with(Vec::new);
        chain.push(rotation.clone());

        // Split-brain detection: previous rotation's new_key should match
        // this rotation's old_key.
        if chain.len() > 1 {
            let prev = &chain[chain.len() - 2];
            let prev_new = prev
                .get("new_key_fingerprint")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if prev_new != old_fp && !prev_new.is_empty() {
                warn!(
                    peer = peer_node_id,
                    "SPLIT-BRAIN DETECTED: rotation chain discontinuity. Expected old_fp={}, got {}",
                    prev_new,
                    old_fp,
                );
            }
        }
    }

    Ok(())
}

// ── DNS network resolution handler ──────────────────────────────────

/// Handle a DNS resolve request from a peer (tag 0x09).
///
/// Checks local DNS for the requested name and responds with the address
/// (tag 0x0A + address bytes) or an empty response if not found.
pub(super) async fn handle_dns_resolve_request(
    data: &[u8],
    stream: &mut stoq::Stream,
    peer_node_id: &str,
    ctx: &PeerContext,
) {
    if data.len() < 2 {
        debug!("DNS resolve request too short from {}", peer_node_id);
        return;
    }
    let name = match std::str::from_utf8(&data[1..]) {
        Ok(n) => n,
        Err(_) => {
            debug!("DNS resolve request invalid UTF-8 from {}", peer_node_id);
            return;
        }
    };

    debug!(name = name, peer = peer_node_id, "DNS network resolve request");

    // Record in popularity tracker if available.
    if let Some(ref tracker) = ctx.dns_popularity_tracker {
        tracker.record_resolution(name).await;
    }

    // Check local DNS resolver.
    let response = if let Some(ref dns) = ctx.dns_resolver {
        match dns.resolve(name).await {
            Some(addr) => {
                let addr_str = addr.to_string();
                let mut resp = vec![TAG_DNS_RESOLVE_RESPONSE];
                resp.extend_from_slice(addr_str.as_bytes());
                resp
            }
            None => vec![TAG_DNS_RESOLVE_RESPONSE],
        }
    } else {
        vec![TAG_DNS_RESOLVE_RESPONSE]
    };

    if let Err(e) = stream.send(&response).await {
        debug!("Failed to send DNS resolve response to {}: {}", peer_node_id, e);
    }
}

// ── Phase H.1: distributed DNS query handler ─────────────────────────

/// Handle TAG_DNS_QUERY (0x50): rich query with conflict-resolution metadata.
///
/// Wire format: `[TAG_DNS_QUERY][JSON DistributedDnsQuery]`.
///
/// We scan the local blockchain for DNS asset entries matching the
/// requested name, build a [`DistributedDnsResponse`] populated with
/// chain_id / chain_height / registration_timestamp /
/// foundation_grant_present, and write it back as
/// `[TAG_DNS_RESPONSE][JSON]`.
///
/// Empty-records responses are still meaningful — they tell the asker
/// that this peer is alive and has no entry.  Callers fold all peer
/// responses through `select_canonical` to pick the winner.
pub(super) async fn handle_dns_query(
    payload: &[u8],
    stream: &mut stoq::Stream,
    peer_node_id: &str,
    ctx: &PeerContext,
) {
    let short = &peer_node_id[..8.min(peer_node_id.len())];

    let query: DistributedDnsQuery = match serde_json::from_slice(payload) {
        Ok(q) => q,
        Err(e) => {
            debug!("Invalid DNS query JSON from {}: {}", short, e);
            return;
        }
    };

    // Record popularity if available.
    if let Some(ref tracker) = ctx.dns_popularity_tracker {
        tracker.record_resolution(&query.domain_name).await;
    }

    let response = build_dns_response_for_query(&query, ctx).await;

    let json = match serde_json::to_vec(&response) {
        Ok(b) => b,
        Err(e) => {
            debug!("Failed to serialize DNS response: {}", e);
            return;
        }
    };

    let mut frame = Vec::with_capacity(1 + json.len());
    frame.push(TAG_DNS_RESPONSE);
    frame.extend_from_slice(&json);

    if let Err(e) = stream.send(&frame).await {
        debug!("Failed to send DNS response to {}: {}", short, e);
    }
}

/// Walk the local chain looking for DNS entries matching the queried
/// name and build the response struct used by H.1 conflict resolution.
async fn build_dns_response_for_query(
    query: &DistributedDnsQuery,
    ctx: &PeerContext,
) -> DistributedDnsResponse {
    use crate::assets::core::{AssetCategory, BaseSystemType};
    use crate::blockchain::block::StoragePointer;

    let chain = ctx.blockchain.get_chain().await;
    let mut records: Vec<crate::dns::DnsRecord> = Vec::new();
    let mut earliest_ts: Option<u64> = None;
    let mut earliest_height: u64 = 0;
    // Phase I.1: foundation_grant_present is now sourced from
    // DnsBlockEntry.grant_signature (Phase H.1 deferred item). If ANY
    // matching entry on this chain carries a grant signature, the
    // response advertises foundation backing — distributed-DNS
    // resolvers use this flag in conflict resolution to prefer the
    // grant-backed registration over self-registered duplicates.
    let mut foundation_grant_present = false;

    for block in chain.iter() {
        for entry in &block.entries {
            let is_dns = matches!(
                entry.registration.category,
                AssetCategory::BaseSystem(BaseSystemType::Dns)
            );
            if !is_dns {
                continue;
            }
            let dns_json = match &entry.storage_pointer {
                StoragePointer::Local { path } => path.as_str(),
                _ => continue,
            };
            let dns_entry: crate::dns::DnsBlockEntry =
                match serde_json::from_str(dns_json) {
                    Ok(e) => e,
                    Err(_) => continue,
                };
            if dns_entry.domain_name != query.domain_name {
                continue;
            }

            // Phase I.1: surface grant-backing on the response.
            if dns_entry.grant_signature.is_some() {
                foundation_grant_present = true;
            }

            let rec = crate::dns::DnsRecord {
                domain: dns_entry.domain_name.clone(),
                record_type: dns_entry.record_type.clone(),
                data: dns_entry.record_data.clone(),
                ttl: dns_entry.ttl,
                created_at: entry.state_proof.time_proof.time_verification_timestamp,
                expires_at: entry.state_proof.time_proof.time_verification_timestamp,
                owner: dns_entry.owner.clone(),
                tx_hash: Some(block.hash.clone()),
            };
            records.push(rec);

            // Track the *earliest* registration's metadata — older wins
            // in the canonical-cmp ordering.
            let ts = entry
                .state_proof
                .time_proof
                .time_verification_timestamp
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            match earliest_ts {
                None => {
                    earliest_ts = Some(ts);
                    earliest_height = block.index;
                }
                Some(prev) if ts < prev => {
                    earliest_ts = Some(ts);
                    earliest_height = block.index;
                }
                _ => {}
            }
        }
    }

    // Use BLAKE3 of network_id as our advertised chain_id.  This is the
    // canonical identifier other peers will see for our chain on the
    // wire; it does not need to match `our_coordinate` because chain
    // identity is independent of node coordinate.
    let chain_id = blake3::hash(ctx.network_id.as_bytes()).to_hex().to_string();

    DistributedDnsResponse {
        query_id: query.query_id,
        domain_name: query.domain_name.clone(),
        records,
        chain_id,
        chain_height: earliest_height,
        registration_timestamp: earliest_ts.unwrap_or(0),
        foundation_grant_present,
    }
}

// ── Block fetch handler ──────────────────────────────────────────────

/// Handle a block fetch request (tag 0x11).
///
/// Looks up each requested block hash in the local blockchain,
/// serializes found blocks as JSON, and sends a `BlockFetchResponse`
/// back on the same stream.
pub(super) async fn handle_block_fetch_request(
    payload: &[u8],
    stream: &mut stoq::Stream,
    sender_node_id: &str,
    ctx: &PeerContext,
) {
    let short_id = &sender_node_id[..8.min(sender_node_id.len())];

    let msg: MatrixMessage = match serde_json::from_slice(payload) {
        Ok(m) => m,
        Err(e) => {
            warn!("Invalid block fetch JSON from {}: {}", short_id, e);
            return;
        }
    };

    let hashes = match msg {
        MatrixMessage::BlockFetchRequest { block_hashes } => block_hashes,
        other => {
            warn!(
                "Expected BlockFetchRequest from {}, got {:?}",
                short_id, other,
            );
            return;
        }
    };

    debug!(
        "Block fetch request from {}: {} hash(es)",
        short_id,
        hashes.len(),
    );

    let mut serialized_blocks = Vec::with_capacity(hashes.len());
    for hash in &hashes {
        if let Some(block) = ctx.blockchain.get_block_by_hash(hash).await {
            match serde_json::to_string(&block) {
                Ok(json) => serialized_blocks.push(json),
                Err(e) => {
                    debug!("Failed to serialize block {}: {}", hash, e);
                }
            }
        }
    }

    info!(
        "Serving {} of {} requested block(s) to {}",
        serialized_blocks.len(),
        hashes.len(),
        short_id,
    );

    let response = MatrixMessage::BlockFetchResponse {
        blocks: serialized_blocks,
    };
    let response_data = match serde_json::to_vec(&response) {
        Ok(d) => d,
        Err(e) => {
            debug!("Failed to serialize BlockFetchResponse: {}", e);
            return;
        }
    };

    if let Err(e) = stream.send(&response_data).await {
        debug!("Failed to send block fetch response to {}: {}", short_id, e);
    }
}

// ── Sync handler ─────────────────────────────────────────────────────

/// Handle a sync/reflector message (tag 0x10).
/// Dispatches through `SyncDispatcher` and sends reply if produced.
/// Wires `NodeBlockchainBlockProvider` so sync request responses
/// contain real block hashes from the local chain.
pub(super) async fn handle_sync_message(
    payload: &[u8],
    stream: &mut stoq::Stream,
    sender_node_id: &str,
    sender_coord: &MatrixCoordinate,
    ctx: &PeerContext,
) {
    let msg: MatrixMessage = match serde_json::from_slice(payload) {
        Ok(m) => m,
        Err(e) => {
            warn!(
                "Invalid sync message JSON from {}: {}",
                &sender_node_id[..8.min(sender_node_id.len())],
                e,
            );
            return;
        }
    };

    let sender_pos = hypermesh_lib::MatrixPosition {
        x: sender_coord.x as f64,
        y: sender_coord.y as f64,
        z: sender_coord.z as f64,
    };

    // Build a snapshot-based BlockProvider from the current chain
    let chain = ctx.blockchain.get_chain().await;
    let provider =
        crate::blockchain::sync_manager::NodeBlockchainBlockProvider::from_blocks(&chain);

    let (mut sm, mut rp) = tokio::join!(
        ctx.sync_manager.lock(),
        ctx.reflector_pool.lock(),
    );

    let mut dispatcher = sync_dispatch::SyncDispatcher {
        sync_manager: &mut sm,
        reflector_pool: &mut rp,
        block_provider: Some(&provider),
    };

    let response = dispatcher.dispatch(msg, sender_node_id, sender_pos);

    if let sync_dispatch::DispatchResponse::Reply(reply_msg) = response {
        send_sync_reply(stream, &reply_msg).await;
    }
}

/// Register a newly-accepted peer as a reflector in the ReflectorPool.
///
/// Uses the shared `network_id` from the PeerContext so that all nodes
/// on the same `--network-id` register reflectors under the same key.
pub(super) async fn register_peer_as_reflector(
    ctx: &PeerContext,
    peer_node_id: &str,
    peer_coord: MatrixCoordinate,
) {
    let network_id = ctx.network_id.clone();
    if network_id.is_empty() {
        return;
    }

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let reflector = crate::network::reflector_pool::Reflector {
        node_id: peer_node_id.to_string(),
        position: hypermesh_lib::MatrixPosition {
            x: peer_coord.x as f64,
            y: peer_coord.y as f64,
            z: peer_coord.z as f64,
        },
        last_seen: now_secs,
        block_height: 0,
        health_score: 1.0,
        privacy_mode: crate::bootstrap::PrivacyMode::PUBLIC,
    };

    ctx.reflector_pool
        .lock()
        .await
        .register_reflector(&network_id, reflector);

    info!(
        "Registered accepted peer {} as reflector for {}",
        &peer_node_id[..8.min(peer_node_id.len())],
        &network_id,
    );
}

// ── Share invite handler ────────────────────────────────────────────

/// Parse a `[TAG_SHARE_INVITE] ++ serde_json(ShareInvite)` wire payload and
/// store the invite in `inbox`.
///
/// This is the pure parse-and-store core of [`handle_share_invite`], split out
/// so it can be exercised in a loopback test that feeds the exact bytes
/// `share.send` frames straight back into the receiver path — with no
/// `PeerContext` required. `data` includes the leading tag byte.
///
/// Returns `Ok(invite_id)` when the invite was parsed and stored, or an error
/// when the payload is malformed or the inbox rejects it. Signature
/// verification is deferred to the recipient's accept flow (which looks up the
/// sender's FALCON pubkey from the blockchain).
pub(crate) async fn parse_and_store_share_invite(
    data: &[u8],
    inbox: &crate::sharing::inbox::InboxStore,
) -> anyhow::Result<String> {
    if data.len() < 2 {
        anyhow::bail!("share invite payload too short");
    }
    let invite_json = &data[1..]; // skip tag byte
    let invite = serde_json::from_slice::<crate::sharing::invite::ShareInvite>(invite_json)?;
    let invite_id = invite.invite_id.clone();
    inbox.add(invite).await?;
    Ok(invite_id)
}

/// Handle a received share invite (tag 0x05).
///
/// Deserializes the invite from JSON (after the tag byte) and stores
/// it in the peer context's inbox. Signature verification is deferred
/// to the recipient's accept flow (requires looking up the sender's
/// FALCON pubkey from the blockchain).
pub(super) async fn handle_share_invite(
    data: &[u8],
    peer_node_id: &str,
    ctx: &PeerContext,
) {
    // Peek the invite for a log line before storing (best-effort).
    if let Ok(invite) =
        serde_json::from_slice::<crate::sharing::invite::ShareInvite>(&data[1.min(data.len())..])
    {
        let sender_display = invite.sender_name.as_deref().unwrap_or_else(|| {
            &invite.sender_node_id[..8.min(invite.sender_node_id.len())]
        });
        info!(
            "Received share invite {} from {} for asset {} ({})",
            &invite.invite_id,
            sender_display,
            &invite.asset_id[..8.min(invite.asset_id.len())],
            invite.asset_name,
        );
    }

    let inbox = match ctx.inbox_store.as_ref() {
        Some(inbox) => inbox,
        None => return,
    };
    if let Err(e) = parse_and_store_share_invite(data, inbox).await {
        debug!(
            "Invalid or unstorable share invite from {}: {}",
            &peer_node_id[..8.min(peer_node_id.len())],
            e,
        );
    }
}

// ── Transfer handler ────────────────────────────────────────────────

/// Handle a cross-network asset transfer message (tag 0x07).
///
/// Wire format: tag(1) + JSON-serialized MatrixMessage (TransferRequest or TransferResponse).
/// For alpha: logs the request and responds with acceptance. Full cross-node
/// coordination will be wired when bilateral PoS validation is production-ready.
pub(super) async fn handle_transfer_message(
    data: &[u8],
    stream: &mut stoq::Stream,
    peer_node_id: &str,
    _ctx: &PeerContext,
) {
    let short_id = &peer_node_id[..8.min(peer_node_id.len())];

    let msg: MatrixMessage = match serde_json::from_slice(data) {
        Ok(m) => m,
        Err(e) => {
            warn!("Invalid transfer message from {}: {}", short_id, e);
            return;
        }
    };

    match msg {
        MatrixMessage::TransferRequest {
            transfer_id,
            asset_id,
            source_scope,
            target_scope,
            ..
        } => {
            info!(
                "Transfer request {} from {}: asset={} ({} -> {})",
                transfer_id, short_id, asset_id, source_scope, target_scope,
            );

            // Alpha: accept all transfer requests. Full PoS bilateral
            // validation will gate acceptance in production.
            let response = MatrixMessage::TransferResponse {
                transfer_id,
                accepted: true,
                target_proof_bytes: Vec::new(),
            };
            let reply = match serde_json::to_vec(&response) {
                Ok(d) => d,
                Err(e) => {
                    warn!("Failed to serialize transfer response: {}", e);
                    return;
                }
            };
            let mut tagged = Vec::with_capacity(1 + reply.len());
            tagged.push(super::protocol::TAG_TRANSFER);
            tagged.extend_from_slice(&reply);
            if let Err(e) = stream.send(&tagged).await {
                warn!("Failed to send transfer response to {}: {}", short_id, e);
            }
        }
        MatrixMessage::TransferResponse {
            transfer_id,
            accepted,
            ..
        } => {
            info!(
                "Transfer response {} from {}: accepted={}",
                transfer_id, short_id, accepted,
            );
        }
        _ => {
            debug!("Non-transfer message on TAG_TRANSFER from {}", short_id);
        }
    }
}

/// Handle an incoming direct message from a peer.
pub(super) async fn handle_direct_message(
    data: &[u8],
    peer_node_id: &str,
    ctx: &PeerContext,
) {
    if data.len() < 2 {
        return;
    }
    let msg_json = &data[1..]; // skip tag byte
    match serde_json::from_slice::<crate::messaging::message::DirectMessage>(msg_json) {
        Ok(msg) => {
            let sender_display = msg
                .sender_name
                .as_deref()
                .unwrap_or_else(|| {
                    &msg.sender_node_id[..8.min(msg.sender_node_id.len())]
                });
            info!(
                "Received message {} from {} to {}",
                &msg.message_id[..8.min(msg.message_id.len())],
                sender_display,
                &msg.recipient_node_id[..8.min(msg.recipient_node_id.len())],
            );
            if let Some(ref store) = ctx.message_store {
                if let Err(e) = store.add(msg).await {
                    warn!("Failed to store direct message: {e}");
                }
            }
        }
        Err(e) => {
            debug!(
                "Invalid direct message from {}: {}",
                &peer_node_id[..8.min(peer_node_id.len())],
                e,
            );
        }
    }
}
