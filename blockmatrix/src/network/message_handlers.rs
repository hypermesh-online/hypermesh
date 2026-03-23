// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Peer message dispatch and block/shard/sync handlers.
//!
//! Extracted from `network/mod.rs` to keep each file under 500 lines.
//! All functions operate on a [`PeerContext`] shared reference.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use anyhow::{anyhow, Result};

use crate::assets::core::asset_id::{AssetCategory, BaseSystemType};
use crate::blockchain::block::Block;
use crate::bootstrap::PrivacyMode;
use crate::dns::DnsBlockEntry;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::hash_bucket::SpatialBucketAssigner;
use crate::network::shard_transport;
use crate::network::stoq_integration::MatrixMessage;
use crate::network::sync_dispatch;
use hypermesh_lib::{BlockchainScope, ContentHash};

use super::{
    NetworkNode, PeerContext,
    CONN_TYPE_HANDSHAKE, CONN_TYPE_PEER_MESSAGE, CONN_TYPE_METRICS, CONN_TYPE_GOSSIP,
};
use crate::network::peer_auth::{self, AuthenticatedPeers};

// ── Wire-protocol tag bytes ──────────────────────────────────────────

/// Shard send (store a shard on this node).
pub(crate) const TAG_SHARD_SEND: u8 = 0x01;
/// Shard fetch (retrieve a shard from this node).
pub(crate) const TAG_SHARD_FETCH: u8 = 0x02;
/// Block announcement.
pub(crate) const TAG_BLOCK_ANNOUNCE: u8 = 0x03;
/// Sync / reflector message.
pub(crate) const TAG_SYNC_MESSAGE: u8 = 0x10;
/// Block fetch request (pull specific blocks by hash).
pub(crate) const TAG_BLOCK_FETCH_REQUEST: u8 = 0x11;
/// Shard availability announcement (consumer-becomes-provider, R12).
pub(crate) const TAG_SHARD_ANNOUNCE: u8 = 0x04;
/// Gossip protocol message.
pub(crate) const TAG_GOSSIP: u8 = 0x20;

// ── Peer message loop ────────────────────────────────────────────────

/// Persistent read loop for a connected peer.
///
/// Accepts new streams from the connection, reads the full payload,
/// dispatches based on the first byte (tag), and handles the message.
/// Runs until the connection is closed.
pub(crate) async fn run_peer_message_loop(
    connection: Arc<stoq::Connection>,
    peer_node_id: String,
    peer_coord: MatrixCoordinate,
    ctx: Arc<PeerContext>,
) {
    let short_id = &peer_node_id[..8.min(peer_node_id.len())];
    info!(
        "Starting message loop for peer {} at ({},{},{})",
        short_id, peer_coord.x, peer_coord.y, peer_coord.z,
    );

    loop {
        let mut stream = match connection.accept_stream().await {
            Ok(s) => s,
            Err(e) => {
                debug!("Peer {} connection closed: {}", short_id, e);
                break;
            }
        };

        let data = match stream.receive().await {
            Ok(d) if !d.is_empty() => d,
            Ok(_) => continue,
            Err(e) => {
                debug!("Stream read error from {}: {}", short_id, e);
                continue;
            }
        };

        dispatch_message(&data, &mut stream, &peer_node_id, &peer_coord, &ctx).await;
    }

    // Clean up peer from authenticated map
    peer_auth::remove_authenticated_peer(&ctx.authenticated_peers, &peer_node_id).await;

    // Remove from connected peer coords
    {
        let mut coords = ctx.connected_peer_coords.write().await;
        coords.retain(|c| !(c.x == peer_coord.x && c.y == peer_coord.y && c.z == peer_coord.z));
    }

    info!(
        "Cleaned up peer {} from auth and coord maps",
        &peer_node_id[..8.min(peer_node_id.len())],
    );
}

/// Route a single message payload to the appropriate handler.
///
/// Asset-level operations (shard send/fetch, sync, block-fetch) are
/// gated on the sender being in the [`AuthenticatedPeers`] map AND
/// belonging to the same network. Block announcements are NOT gated —
/// they are validated by BLAKE3 content integrity, per-entry proof_hash,
/// and state_proof.validate(). Gossip and unknown tags are logged
/// but not gated.
pub(crate) async fn dispatch_message(
    data: &[u8],
    stream: &mut stoq::Stream,
    peer_node_id: &str,
    peer_coord: &MatrixCoordinate,
    ctx: &PeerContext,
) {
    let tag = data[0];
    let short_id = &peer_node_id[..8.min(peer_node_id.len())];

    // Gate asset-level operations on peer authentication + network scope.
    // Block announcements are NOT gated here — blocks are validated by
    // BLAKE3 content integrity + per-entry proof_hash + state_proof.validate().
    // Only shard access (asset-level) and sync operations need PoS gating.
    let needs_auth = matches!(
        tag,
        TAG_SHARD_SEND | TAG_SHARD_FETCH
            | TAG_SYNC_MESSAGE | TAG_BLOCK_FETCH_REQUEST
    );
    if needs_auth
        && !peer_auth::verify_peer_access(
            &ctx.authenticated_peers,
            peer_node_id,
            &ctx.network_id,
        )
        .await
    {
        return;
    }

    match tag {
        TAG_SHARD_SEND | TAG_SHARD_FETCH => {
            // Record shard fetch demand for engauge swarm intelligence.
            if tag == TAG_SHARD_FETCH {
                record_shard_demand(&data, peer_node_id, ctx).await;
            }
            handle_shard_dispatch(&data, stream, ctx).await;
        }
        TAG_BLOCK_ANNOUNCE => {
            handle_block_announce(&data, peer_node_id, ctx).await;
        }
        TAG_SYNC_MESSAGE => {
            handle_sync_message(&data[1..], stream, peer_node_id, peer_coord, ctx).await;
        }
        TAG_BLOCK_FETCH_REQUEST => {
            handle_block_fetch_request(&data[1..], stream, peer_node_id, ctx).await;
        }
        TAG_SHARD_ANNOUNCE => {
            handle_shard_announce(data, peer_node_id, ctx).await;
        }
        TAG_GOSSIP => {
            debug!(
                "Gossip message from peer {} ({} bytes)",
                short_id,
                data.len() - 1,
            );
        }
        _ => {
            warn!("Unknown message tag 0x{:02x} from peer {}", tag, short_id);
        }
    }
}

// ── Shard handler ────────────────────────────────────────────────────

/// Dispatch a shard send/fetch message to the shard store.
async fn handle_shard_dispatch(
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
async fn record_shard_demand(data: &[u8], peer_node_id: &str, ctx: &PeerContext) {
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
async fn handle_shard_announce(
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

// ── Block handlers ───────────────────────────────────────────────────

/// Handle a received block announcement (tag 0x03).
/// Dispatches to scope-specific handlers based on `PeerContext::blockchain_scope`.
async fn handle_block_announce(
    data: &[u8],
    peer_node_id: &str,
    ctx: &PeerContext,
) {
    let block = match parse_and_verify_block(data, peer_node_id) {
        Some(b) => b,
        None => return,
    };

    match ctx.blockchain_scope {
        BlockchainScope::Device => {
            handle_block_device_scope(&block, peer_node_id, ctx).await;
        }
        BlockchainScope::Network => {
            if let Some(ref assigner) = ctx.spatial_bucket_assigner {
                handle_block_public_scope(&block, peer_node_id, ctx, assigner).await;
            } else {
                handle_block_network_scope(&block, peer_node_id, ctx).await;
            }
        }
    }
}

/// Parse a block announcement payload and verify its BLAKE3 hash
/// and per-entry proof integrity.
fn parse_and_verify_block(data: &[u8], peer_node_id: &str) -> Option<Block> {
    if data.len() < 9 {
        warn!("Block announce too short ({} bytes)", data.len());
        return None;
    }

    let block_json_len = u64::from_le_bytes(
        data[1..9].try_into().unwrap_or([0u8; 8]),
    ) as usize;

    if data.len() < 9 + block_json_len {
        warn!(
            "Block announce truncated: need {} bytes, have {}",
            9 + block_json_len,
            data.len(),
        );
        return None;
    }

    let short_id = &peer_node_id[..8.min(peer_node_id.len())];

    let block: Block = match serde_json::from_slice(&data[9..9 + block_json_len]) {
        Ok(b) => b,
        Err(e) => {
            warn!("Invalid block JSON from {}: {}", short_id, e);
            return None;
        }
    };

    if !block.verify_hash() {
        warn!("Block {} hash mismatch from peer {}", block.index, short_id);
        return None;
    }

    // Verify proof integrity for each block entry:
    // proof_hash must equal BLAKE3(serialize(state_proof)) and proof must validate.
    for (i, entry) in block.entries.iter().enumerate() {
        let proof_bytes = match serde_json::to_vec(&entry.state_proof) {
            Ok(b) => b,
            Err(e) => {
                warn!(
                    "Block {} entry {} proof serialization failed from {}: {}",
                    block.index, i, short_id, e,
                );
                return None;
            }
        };
        let computed_hash: [u8; 32] = blake3::hash(&proof_bytes).into();
        if computed_hash != entry.proof_hash {
            warn!(
                "Block {} entry {} proof_hash mismatch from peer {}",
                block.index, i, short_id,
            );
            return None;
        }
        if !entry.state_proof.validate() {
            warn!(
                "Block {} entry {} state proof validation failed from peer {}",
                block.index, i, short_id,
            );
            return None;
        }
    }

    Some(block)
}

/// Device scope: independent chains. Accept only if newer than our height.
async fn handle_block_device_scope(
    block: &Block,
    peer_node_id: &str,
    ctx: &PeerContext,
) {
    let our_height = ctx.blockchain.get_height().await;
    if block.index <= our_height {
        debug!(
            "Device scope: already have block {} (height {}), skipping",
            block.index, our_height,
        );
        return;
    }

    insert_block(block, peer_node_id, ctx).await;
}

/// Network scope (Private): shared chain. Deduplicate by hash, not index.
/// Re-propagate to other peers after successful insertion.
async fn handle_block_network_scope(
    block: &Block,
    peer_node_id: &str,
    ctx: &PeerContext,
) {
    if ctx.blockchain.has_block(&block.hash).await {
        debug!(
            "Network scope: already have block {} by hash, skipping",
            block.index,
        );
        return;
    }

    if insert_block(block, peer_node_id, ctx).await {
        repropagate_block(block, peer_node_id, ctx).await;
    }
}

/// Public scope (Network + spatial bucket filtering): accept blocks
/// whose shard placements fall within our neighborhood radius.
/// Re-propagate accepted blocks to other peers.
async fn handle_block_public_scope(
    block: &Block,
    peer_node_id: &str,
    ctx: &PeerContext,
    assigner: &Arc<RwLock<SpatialBucketAssigner>>,
) {
    if ctx.blockchain.has_block(&block.hash).await {
        debug!(
            "Public scope: already have block {} by hash, skipping",
            block.index,
        );
        return;
    }

    let in_neighborhood = assigner.read().await.block_in_our_neighborhood(block);
    if !in_neighborhood {
        debug!(
            "Public scope: block {} has no shard placements in our neighborhood, skipping",
            block.index,
        );
        return;
    }

    if insert_block(block, peer_node_id, ctx).await {
        repropagate_block(block, peer_node_id, ctx).await;
    }
}

/// Insert a block into our chain. Returns `true` on success.
async fn insert_block(
    block: &Block,
    peer_node_id: &str,
    ctx: &PeerContext,
) -> bool {
    let short_id = &peer_node_id[..8.min(peer_node_id.len())];
    match ctx.blockchain.insert_received_block(block.clone()).await {
        Ok(()) => {
            info!("Received and stored block #{} from peer {}", block.index, short_id);
            extract_dns_entries_from_block(block, ctx).await;
            true
        }
        Err(e) => {
            debug!(
                "Block {} insertion failed: {} (from peer {})",
                block.index, e, short_id,
            );
            false
        }
    }
}

/// Scan a block's entries for DNS assets and register them in the local resolver.
async fn extract_dns_entries_from_block(block: &Block, ctx: &PeerContext) {
    let resolver = match ctx.dns_resolver.as_ref() {
        Some(r) => r,
        None => return,
    };

    for entry in &block.entries {
        let is_dns = matches!(
            entry.registration.category,
            AssetCategory::BaseSystem(BaseSystemType::Dns)
        );
        if !is_dns {
            continue;
        }

        // DNS data is stored as JSON in StoragePointer::Local { path }
        let dns_json = match &entry.storage_pointer {
            crate::blockchain::block::StoragePointer::Local { path } => path.as_str(),
            _ => {
                debug!(
                    "Block #{} DNS entry has no local storage data, skipping",
                    block.index,
                );
                continue;
            }
        };

        let dns_entry: DnsBlockEntry = match serde_json::from_str(dns_json) {
            Ok(e) => e,
            Err(e) => {
                debug!(
                    "Block #{} DNS entry parse failed: {}",
                    block.index, e,
                );
                continue;
            }
        };

        // Extract IP address from the DNS record data
        let ip_addr = match &dns_entry.record_data {
            crate::dns::DnsRecordData::AAAA(addr) => {
                std::net::IpAddr::V6(*addr)
            }
            _ => {
                debug!(
                    "Block #{} DNS entry '{}' is not AAAA, skipping resolver insert",
                    block.index, dns_entry.domain_name,
                );
                continue;
            }
        };

        info!(
            "Extracted DNS from block #{}: {} -> {}",
            block.index, dns_entry.domain_name, ip_addr,
        );
        resolver.register(dns_entry.domain_name, ip_addr).await;
    }
}

/// Re-propagate a block to peers via the block propagator.
async fn repropagate_block(
    block: &Block,
    peer_node_id: &str,
    ctx: &PeerContext,
) {
    let coords = ctx.connected_peer_coords.read().await;
    if coords.is_empty() {
        debug!("No connected peers, skipping block re-propagation");
        return;
    }
    let propagator = ctx.block_propagator.lock().await;
    let result = propagator.propagate_block(block, &coords).await;
    if !result.failed_nodes.is_empty() {
        let short_id = &peer_node_id[..8.min(peer_node_id.len())];
        debug!(
            "Re-propagation of block {} from {}: {} reached, {} failed",
            block.index, short_id, result.reached_nodes.len(), result.failed_nodes.len(),
        );
    }
}

// ── Incoming connection handler ───────────────────────────────────────

/// Read the 1-byte discriminator to route handshake vs peer-message connections.
pub(crate) async fn handle_incoming_connection(
    connection: Arc<stoq::Connection>,
    nodes: Arc<RwLock<HashMap<String, NetworkNode>>>,
    local_coord: MatrixCoordinate,
    signer: Arc<dyn hypermesh_lib::NodeSigner>,
    proof_provider: Arc<dyn hypermesh_lib::StateProofProvider>,
    cert_manager: Arc<stoq::transport::certificates::CertificateManager>,
    peer_ctx: Option<Arc<PeerContext>>,
    authenticated_peers: AuthenticatedPeers,
) -> Result<()> {
    let mut stream = connection.accept_stream().await?;
    let conn_type = stream.read_discriminator().await?;

    match conn_type {
        CONN_TYPE_HANDSHAKE => {
            handle_handshake_connection(
                connection, &mut stream, nodes, local_coord,
                signer, proof_provider, cert_manager, peer_ctx,
                authenticated_peers,
            ).await
        }
        CONN_TYPE_PEER_MESSAGE => {
            handle_peer_message_connection(&mut stream, &connection, local_coord, peer_ctx).await
        }
        CONN_TYPE_METRICS => {
            handle_metrics_connection(&mut stream, peer_ctx).await
        }
        CONN_TYPE_GOSSIP => {
            handle_gossip_connection(&mut stream, peer_ctx).await
        }
        other => {
            warn!("Unknown connection discriminator 0x{:02x} — dropping", other);
            Ok(())
        }
    }
}

/// Run bilateral PoS handshake, register the peer, optionally spawn message loop.
async fn handle_handshake_connection(
    connection: Arc<stoq::Connection>,
    stream: &mut stoq::Stream,
    nodes: Arc<RwLock<HashMap<String, NetworkNode>>>,
    local_coord: MatrixCoordinate,
    signer: Arc<dyn hypermesh_lib::NodeSigner>,
    proof_provider: Arc<dyn hypermesh_lib::StateProofProvider>,
    cert_manager: Arc<stoq::transport::certificates::CertificateManager>,
    peer_ctx: Option<Arc<PeerContext>>,
    authenticated_peers: AuthenticatedPeers,
) -> Result<()> {
    debug!("Accepted incoming connection — handshake discriminator");
    let coord_tuple = (local_coord.x, local_coord.y, local_coord.z);

    let result = stoq::accept_handshake(
        stream,
        signer.as_ref(),
        proof_provider.as_ref(),
        coord_tuple,
    )
    .await?;

    // Post-handshake metadata exchange (blockmatrix layer).
    // Acceptor reads initiator's metadata first, then sends its own.
    let peer_network_id = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        stream.read_msg(),
    )
    .await
    {
        Ok(Ok(peer_meta_bytes)) => {
            let peer_meta: super::HandshakeMetadata =
                serde_json::from_slice(&peer_meta_bytes).unwrap_or_default();
            info!("Received peer network_id from metadata: '{}'", peer_meta.network_id);
            peer_meta.network_id
        }
        Ok(Err(e)) => {
            warn!("Peer did not send handshake metadata: {e}");
            String::new()
        }
        Err(_) => {
            warn!("Timeout waiting for peer handshake metadata — assuming old node");
            String::new()
        }
    };

    // Send our metadata back to the initiator
    let our_network_id = peer_ctx
        .as_ref()
        .map(|c| c.network_id.as_str())
        .unwrap_or("");
    let our_meta = super::HandshakeMetadata {
        network_id: our_network_id.to_string(),
    };
    if let Ok(meta_bytes) = serde_json::to_vec(&our_meta) {
        if let Err(e) = stream.write_msg(&meta_bytes).await {
            debug!("Failed to send handshake metadata to peer: {e}");
        }
    }

    let coordinate = MatrixCoordinate::new(
        result.peer_coordinate.0,
        result.peer_coordinate.1,
        result.peer_coordinate.2,
    )
    .map_err(|e| anyhow!("Invalid peer coordinate: {e}"))?;

    let peer_node_id = result.peer_node_id;

    let node = NetworkNode {
        coordinate,
        address: connection.endpoint().to_socket_addr(),
        node_id: peer_node_id.clone(),
        privacy_mode: PrivacyMode::PUBLIC,
        connection: Some(connection.clone()),
    };

    nodes.write().await.insert(peer_node_id.clone(), node);

    // Register the accepted peer as authenticated (PoS handshake passed).
    // register_authenticated_peer enforces that proof_bytes and pubkey are
    // non-empty (R11 bilateral verification).
    // Use the network_id received from the peer during metadata exchange,
    // NOT our own network_id.
    let registered = peer_auth::register_authenticated_peer(
        &authenticated_peers,
        peer_auth::AuthenticatedPeer {
            node_id: peer_node_id.clone(),
            pubkey: result.peer_pubkey.clone(),
            coordinate: (coordinate.x as i32, coordinate.y as i32, coordinate.z as i32),
            network_id: peer_network_id,
            authenticated_at: std::time::Instant::now(),
            proof_bytes: result.peer_proof.clone(),
        },
    )
    .await;

    if !registered {
        warn!(
            "Peer {} failed authentication registration — bilateral PoS incomplete, disconnecting",
            &peer_node_id[..8.min(peer_node_id.len())]
        );
        nodes.write().await.remove(&peer_node_id);
        return Err(anyhow!(
            "Peer {} bilateral PoS verification incomplete — proof or pubkey missing",
            peer_node_id
        ));
    }

    info!(
        "Bilateral verification complete — added authenticated node {} (proof={} bytes, pubkey={} bytes)",
        &peer_node_id[..8.min(peer_node_id.len())],
        result.peer_proof.len(),
        result.peer_pubkey.len(),
    );

    // Request CA certificate in background (Phase 2 bootstrap)
    spawn_acceptor_ca_enrollment(cert_manager, signer.node_id().to_string());

    // Register accepted peer as a reflector for proactive sync
    if let Some(ref ctx) = peer_ctx {
        register_peer_as_reflector(ctx, &peer_node_id, coordinate).await;
    }

    if let Some(ctx) = peer_ctx {
        tokio::spawn(async move {
            run_peer_message_loop(connection, peer_node_id, coordinate, ctx).await;
        });
    }

    Ok(())
}

/// Spawn CA enrollment on the acceptor side after a successful handshake.
fn spawn_acceptor_ca_enrollment(
    cert_manager: Arc<stoq::transport::certificates::CertificateManager>,
    node_id: String,
) {
    tokio::spawn(async move {
        let state_proof = match super::ca_enrollment::generate_node_state_proof(&node_id).await {
            Ok(sp) => sp,
            Err(e) => {
                warn!("CA enrollment (acceptor): state proof generation failed: {e}");
                return;
            }
        };
        super::ca_enrollment::spawn_ca_enrollment(cert_manager, node_id, state_proof);
    });
}

/// Process a single peer-message connection (non-handshake).
///
/// This path handles standalone CONN_TYPE_PEER_MESSAGE connections.
/// Block propagation uses the handshake connection's `run_peer_message_loop`
/// instead, so this path is mainly for ad-hoc peer messages.
///
/// Uses the remote socket address as a placeholder peer identity since
/// no node_id prefix is included in the wire format.
async fn handle_peer_message_connection(
    stream: &mut stoq::Stream,
    connection: &Arc<stoq::Connection>,
    local_coord: MatrixCoordinate,
    peer_ctx: Option<Arc<PeerContext>>,
) -> Result<()> {
    debug!("Accepted incoming connection — peer message discriminator");
    let data = match stream.receive().await {
        Ok(d) if !d.is_empty() => d.to_vec(),
        Ok(_) => return Ok(()),
        Err(e) => return Err(anyhow!("Failed to read peer message: {e}")),
    };

    if let Some(ctx) = peer_ctx {
        let peer_node_id = connection.endpoint().to_socket_addr().to_string();
        dispatch_message(&data, stream, &peer_node_id, &local_coord, &ctx).await;
    } else {
        debug!("Peer message received but no PeerContext — dropping");
    }

    Ok(())
}

// ── Metrics handler ─────────────────────────────────────────────

/// Handle an incoming metrics stream (discriminator 0x02).
///
/// Reads the frame payload and logs it. In the future this should
/// feed into engauge's `MetricsIngestionPipeline` if available.
async fn handle_metrics_connection(
    stream: &mut stoq::Stream,
    peer_ctx: Option<Arc<PeerContext>>,
) -> Result<()> {
    let data = match stream.receive().await {
        Ok(d) if !d.is_empty() => d.to_vec(),
        Ok(_) => return Ok(()),
        Err(e) => return Err(anyhow!("Failed to read metrics frame: {e}")),
    };

    // Validate it's parseable JSON (MetricsFrame format)
    match serde_json::from_slice::<serde_json::Value>(&data) {
        Ok(frame) => {
            let source = frame.get("source_node")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            debug!(
                "Received metrics frame from {} ({} bytes)",
                &source[..8.min(source.len())],
                data.len(),
            );
        }
        Err(e) => {
            debug!("Invalid metrics frame ({} bytes): {}", data.len(), e);
        }
    }

    let _ = peer_ctx; // Reserved for future engauge pipeline integration
    Ok(())
}

// ── Gossip handler ──────────────────────────────────────────────

/// Handle an incoming gossip stream (discriminator 0x03).
///
/// Reads the gossip message payload and processes it through the
/// gossip protocol if a `PeerContext` is available. The gossip
/// protocol merges newer entries into local state.
async fn handle_gossip_connection(
    stream: &mut stoq::Stream,
    peer_ctx: Option<Arc<PeerContext>>,
) -> Result<()> {
    let data = match stream.receive().await {
        Ok(d) if !d.is_empty() => d.to_vec(),
        Ok(_) => return Ok(()),
        Err(e) => return Err(anyhow!("Failed to read gossip message: {e}")),
    };

    let msg: super::gossip::GossipMessage = match serde_json::from_slice(&data) {
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

// ── Block fetch handler ──────────────────────────────────────────────

/// Handle a block fetch request (tag 0x11).
///
/// Looks up each requested block hash in the local blockchain,
/// serializes found blocks as JSON, and sends a `BlockFetchResponse`
/// back on the same stream.
async fn handle_block_fetch_request(
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
async fn handle_sync_message(
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
async fn register_peer_as_reflector(
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

/// Serialize and send a sync reply on the given stream.
async fn send_sync_reply(stream: &mut stoq::Stream, reply_msg: &MatrixMessage) {
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
