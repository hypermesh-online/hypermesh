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

use super::super::peer_auth::{self, AuthenticatedPeers};
use super::super::{
    NetworkNode, PeerContext, CONN_TYPE_GOSSIP, CONN_TYPE_HANDSHAKE, CONN_TYPE_METRICS,
    CONN_TYPE_PEER_MESSAGE,
};

use super::asset_chain_handlers::handle_asset_chain;
use super::attestation_handlers::handle_mirror_attestation;
use super::block_handlers::handle_block_announce;
use super::distributed_ca::{
    handle_ca_key_share, handle_ca_sign_request, handle_ca_sign_response, handle_crl_request,
    handle_crl_response,
};
use super::message_utils::{handle_gossip_connection, handle_metrics_connection};
use super::protocol::{
    TAG_ASSET_CHAIN, TAG_BLOCK_ANNOUNCE, TAG_BLOCK_FETCH_REQUEST, TAG_CA_KEY_SHARE,
    TAG_CA_SIGN_REQUEST, TAG_CA_SIGN_RESPONSE, TAG_CRL_REQUEST, TAG_CRL_RESPONSE,
    TAG_DIRECT_MESSAGE, TAG_DNS_QUERY, TAG_DNS_RESOLVE, TAG_GOSSIP, TAG_KEY_ROTATION,
    TAG_MIRROR_ATTEST, TAG_SHARD_ANNOUNCE, TAG_SHARD_FETCH, TAG_SHARD_LOCATE, TAG_SHARD_SEND,
    TAG_SHARE_INVITE, TAG_SYNC_MESSAGE, TAG_TRANSFER, TAG_TRANSFER_LOCK, TAG_TRANSFER_REGISTER_ACK,
    TAG_TRANSFER_REGISTER_REQ, TAG_TRANSFER_RELEASE, TAG_TRANSFER_ROLLBACK,
};
use super::sync_and_reflection::{
    handle_block_fetch_request, handle_direct_message, handle_dns_query,
    handle_dns_resolve_request, handle_key_rotation, handle_shard_announce, handle_shard_dispatch,
    handle_shard_locate, handle_share_invite, handle_sync_message, handle_transfer_message,
    register_peer_as_reflector,
};
use super::transfer_handlers::{
    handle_transfer_lock, handle_transfer_register_ack, handle_transfer_register_req,
    handle_transfer_release, handle_transfer_rollback,
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
    let connection_id = connection.id();
    let short_id = &peer_node_id[..8.min(peer_node_id.len())];
    info!(
        "Starting message loop for peer {} at ({},{},{})",
        short_id, peer_coord.x, peer_coord.y, peer_coord.z,
    );

    loop {
        let Some(mut stream) = accept_peer_stream(&connection, short_id).await else {
            break;
        };
        let Some(data) = receive_peer_message(&mut stream, short_id).await else {
            continue;
        };

        dispatch_message(
            &data,
            &mut stream,
            &peer_node_id,
            &connection_id,
            &peer_coord,
            &ctx,
        )
        .await;
    }

    cleanup_peer_registration(
        &peer_node_id,
        &connection_id,
        &CleanupContext::from(ctx.as_ref()),
    )
    .await;
}

async fn cleanup_peer_registration(
    peer_node_id: &str,
    connection_id: &str,
    ctx: &CleanupContext<'_>,
) {
    // Clean up peer from authenticated map
    peer_auth::remove_authenticated_peer(&ctx.authenticated_peers, &peer_node_id, &connection_id)
        .await;

    crate::network::connected_peer_routes::remove_connection(
        ctx.connected_nodes,
        ctx.connected_routes,
        peer_node_id,
        connection_id,
    )
    .await;

    info!(
        "Cleaned up peer {} from auth and coord maps",
        &peer_node_id[..8.min(peer_node_id.len())],
    );
}

/// Mirror an authenticated peer into the kernel eBPF fast-path maps (P5).
///
/// Extracts the peer's IPv6 source address from the connection endpoint and
/// writes `pos_header_map[src].validated=1` + `policy_map[src].requires_pos=1`
/// via the shared orchestrator. This makes the kernel gate key on the SAME
/// source the userspace bilateral-PoS handshake authorized. IPv4-mapped
/// addresses are converted to their IPv6 form. No-op when no orchestrator is
/// present or no XDP program is attached (userspace-only tier unchanged).
fn mirror_peer_auth_to_kernel(ctx: &PeerContext, connection: &stoq::Connection) {
    let Some(ref ebpf) = ctx.ebpf else {
        return;
    };
    let addr = connection.endpoint().to_socket_addr();
    let ipv6 = match addr {
        std::net::SocketAddr::V6(v6) => *v6.ip(),
        std::net::SocketAddr::V4(v4) => v4.ip().to_ipv6_mapped(),
    };
    let src_ip = ipv6.octets();
    // HyperMesh peers sign with FALCON-1024 (algorithm indicator 0x01).
    if let Err(e) = ebpf.set_peer_pos_validated(src_ip, true, hypermesh_ebpf::ALG_FALCON_1024) {
        debug!("eBPF peer-auth mirror skipped: {e}");
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
    connected_routes: crate::network::connected_peer_routes::ConnectedPeerRoutes,
) -> Result<()> {
    let mut stream = connection.accept_stream().await?;
    let conn_type = stream.read_discriminator().await?;

    match conn_type {
        CONN_TYPE_HANDSHAKE => {
            handle_handshake_connection(
                connection,
                &mut stream,
                nodes,
                local_coord,
                signer,
                proof_provider,
                cert_manager,
                peer_ctx,
                authenticated_peers,
                connected_routes,
            )
            .await
        }
        CONN_TYPE_PEER_MESSAGE => {
            handle_peer_message_connection(&mut stream, &connection, local_coord, peer_ctx).await
        }
        CONN_TYPE_METRICS => handle_metrics_connection(&mut stream, peer_ctx).await,
        CONN_TYPE_GOSSIP => handle_gossip_connection(&mut stream, peer_ctx).await,
        other => {
            warn!(
                "Unknown connection discriminator 0x{:02x} — dropping",
                other
            );
            Ok(())
        }
    }
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
        dispatch_message(
            &data,
            stream,
            &peer_node_id,
            &connection.id(),
            &local_coord,
            &ctx,
        )
        .await;
    } else {
        debug!("Peer message received but no PeerContext — dropping");
    }

    Ok(())
}

// ── Dispatch-routing unit tests ───────────────────────────────────────
//
// These drive the REAL `route_message` — the pure routing decision that
// `dispatch_message` executes — so the tag→handler wiring and the auth gate
// are proved BY STRUCTURE, without constructing a QUIC `stream`. The
// integration suites (`tests/s3_4_*`) prove the codec and the accept gate; this
// closes the remaining gap: that a framed `TAG_ASSET_CHAIN` / `TAG_MIRROR_ATTEST`
// from an authenticated peer actually routes to its handler, and that an
// unauthenticated one is dropped by the auth gate before the handler.
mod dispatch;
mod handshake;
mod routes;
#[cfg(test)]
mod tests;
pub(crate) use dispatch::dispatch_message;
use handshake::handle_handshake_connection;
pub use routes::message_requires_auth;
use routes::{route_message, Handler, Route};

async fn accept_peer_stream(connection: &stoq::Connection, short_id: &str) -> Option<stoq::Stream> {
    match connection.accept_stream().await {
        Ok(stream) => Some(stream),
        Err(e) => {
            debug!("Peer {} connection closed: {}", short_id, e);
            None
        }
    }
}

async fn receive_peer_message(stream: &mut stoq::Stream, short_id: &str) -> Option<bytes::Bytes> {
    match stream.receive().await {
        Ok(data) if !data.is_empty() => Some(data),
        Ok(_) => None,
        Err(e) => {
            debug!("Stream read error from {}: {}", short_id, e);
            None
        }
    }
}

struct CleanupContext<'a> {
    authenticated_peers: &'a AuthenticatedPeers,
    connected_routes: &'a crate::network::connected_peer_routes::ConnectedPeerRoutes,
    connected_nodes: &'a crate::network::connected_peer_routes::NetworkNodes,
}

impl<'a> From<&'a PeerContext> for CleanupContext<'a> {
    fn from(ctx: &'a PeerContext) -> Self {
        Self {
            authenticated_peers: &ctx.authenticated_peers,
            connected_routes: &ctx.connected_routes,
            connected_nodes: &ctx.connected_nodes,
        }
    }
}
mod fetch;
