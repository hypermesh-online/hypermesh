// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Peer connection entry points: incoming-connection routing, handshake
//! flow, message-loop, and dispatch by wire tag.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::bootstrap::PrivacyMode;
use crate::matrix::coordinate::MatrixCoordinate;

use super::super::{
    NetworkNode, PeerContext, CONN_TYPE_GOSSIP, CONN_TYPE_HANDSHAKE, CONN_TYPE_METRICS,
    CONN_TYPE_PEER_MESSAGE,
};
use super::super::peer_auth::{self, AuthenticatedPeers};

use super::block_handlers::handle_block_announce;
use super::distributed_ca::{handle_ca_key_share, handle_ca_sign_request, handle_ca_sign_response};
use super::message_utils::{handle_gossip_connection, handle_metrics_connection};
use super::protocol::{
    TAG_BLOCK_ANNOUNCE, TAG_BLOCK_FETCH_REQUEST, TAG_CA_KEY_SHARE, TAG_CA_SIGN_REQUEST,
    TAG_CA_SIGN_RESPONSE, TAG_DIRECT_MESSAGE, TAG_DNS_QUERY, TAG_DNS_RESOLVE, TAG_GOSSIP,
    TAG_KEY_ROTATION, TAG_SHARD_ANNOUNCE, TAG_SHARD_FETCH, TAG_SHARD_SEND, TAG_SHARE_INVITE,
    TAG_SYNC_MESSAGE, TAG_TRANSFER, TAG_TRANSFER_LOCK, TAG_TRANSFER_REGISTER_ACK,
    TAG_TRANSFER_REGISTER_REQ, TAG_TRANSFER_RELEASE, TAG_TRANSFER_ROLLBACK,
};
use super::transfer_handlers::{
    handle_transfer_lock, handle_transfer_register_ack, handle_transfer_register_req,
    handle_transfer_release, handle_transfer_rollback,
};
use super::sync_and_reflection::{
    handle_block_fetch_request, handle_direct_message, handle_dns_query,
    handle_dns_resolve_request, handle_key_rotation, handle_shard_announce,
    handle_shard_dispatch, handle_share_invite, handle_sync_message, handle_transfer_message,
    record_shard_demand, register_peer_as_reflector,
};

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
            | TAG_CA_KEY_SHARE | TAG_CA_SIGN_REQUEST | TAG_CA_SIGN_RESPONSE
            | TAG_TRANSFER_LOCK | TAG_TRANSFER_REGISTER_REQ | TAG_TRANSFER_REGISTER_ACK
            | TAG_TRANSFER_RELEASE | TAG_TRANSFER_ROLLBACK
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
        TAG_SHARE_INVITE => {
            handle_share_invite(data, peer_node_id, ctx).await;
        }
        TAG_DIRECT_MESSAGE => {
            handle_direct_message(data, peer_node_id, ctx).await;
        }
        TAG_TRANSFER => {
            handle_transfer_message(&data[1..], stream, peer_node_id, ctx).await;
        }
        TAG_CA_KEY_SHARE => {
            if let Err(e) = handle_ca_key_share(&data, peer_node_id, ctx).await {
                warn!("Failed to handle CA key share: {e}");
            }
        }
        TAG_CA_SIGN_REQUEST => {
            if let Err(e) = handle_ca_sign_request(&data, peer_node_id, ctx).await {
                warn!("Failed to handle CA sign request: {e}");
            }
        }
        TAG_CA_SIGN_RESPONSE => {
            if let Err(e) = handle_ca_sign_response(&data, peer_node_id, ctx).await {
                warn!("Failed to handle CA sign response: {e}");
            }
        }
        TAG_KEY_ROTATION => {
            if let Err(e) = handle_key_rotation(data, peer_node_id, ctx).await {
                warn!("Failed to handle key rotation: {e}");
            }
        }
        TAG_DNS_RESOLVE => {
            handle_dns_resolve_request(data, stream, peer_node_id, ctx).await;
        }
        TAG_DNS_QUERY => {
            // Phase H.1: rich DNS query — wire format `[tag][JSON]`.
            // Strip the tag byte; handler parses the rest as JSON.
            handle_dns_query(&data[1..], stream, peer_node_id, ctx).await;
        }
        TAG_TRANSFER_LOCK => {
            handle_transfer_lock(&data[1..], peer_node_id, ctx).await;
        }
        TAG_TRANSFER_REGISTER_REQ => {
            handle_transfer_register_req(&data[1..], stream, peer_node_id, ctx).await;
        }
        TAG_TRANSFER_REGISTER_ACK => {
            handle_transfer_register_ack(&data[1..], peer_node_id, ctx).await;
        }
        TAG_TRANSFER_RELEASE => {
            handle_transfer_release(&data[1..], peer_node_id, ctx).await;
        }
        TAG_TRANSFER_ROLLBACK => {
            handle_transfer_rollback(&data[1..], peer_node_id, ctx).await;
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
            let peer_meta: super::super::HandshakeMetadata =
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
    let our_meta = super::super::HandshakeMetadata {
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
        let state_proof = match super::super::ca_enrollment::generate_node_state_proof(&node_id).await {
            Ok(sp) => sp,
            Err(e) => {
                warn!("CA enrollment (acceptor): state proof generation failed: {e}");
                return;
            }
        };
        super::super::ca_enrollment::spawn_ca_enrollment(cert_manager, node_id, state_proof);
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
