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

use super::asset_chain_handlers::handle_asset_chain;
use super::attestation_handlers::handle_mirror_attestation;
use super::block_handlers::handle_block_announce;
use super::distributed_ca::{handle_ca_key_share, handle_ca_sign_request, handle_ca_sign_response};
use super::message_utils::{handle_gossip_connection, handle_metrics_connection};
use super::protocol::{
    TAG_ASSET_CHAIN, TAG_BLOCK_ANNOUNCE, TAG_BLOCK_FETCH_REQUEST, TAG_CA_KEY_SHARE,
    TAG_CA_SIGN_REQUEST, TAG_CA_SIGN_RESPONSE, TAG_DIRECT_MESSAGE, TAG_DNS_QUERY, TAG_DNS_RESOLVE,
    TAG_GOSSIP, TAG_KEY_ROTATION, TAG_MIRROR_ATTEST, TAG_SHARD_ANNOUNCE, TAG_SHARD_FETCH,
    TAG_SHARD_LOCATE, TAG_SHARD_SEND,
    TAG_SHARE_INVITE, TAG_SYNC_MESSAGE, TAG_TRANSFER, TAG_TRANSFER_LOCK,
    TAG_TRANSFER_REGISTER_ACK, TAG_TRANSFER_REGISTER_REQ, TAG_TRANSFER_RELEASE,
    TAG_TRANSFER_ROLLBACK,
};
use super::transfer_handlers::{
    handle_transfer_lock, handle_transfer_register_ack, handle_transfer_register_req,
    handle_transfer_release, handle_transfer_rollback,
};
use super::sync_and_reflection::{
    handle_block_fetch_request, handle_direct_message, handle_dns_query,
    handle_dns_resolve_request, handle_key_rotation, handle_shard_announce,
    handle_shard_dispatch, handle_shard_locate, handle_share_invite, handle_sync_message,
    handle_transfer_message, record_shard_demand, register_peer_as_reflector,
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

/// Whether a wire tag may only be handled for an AUTHENTICATED, same-network
/// peer.
///
/// This is the single source of truth for the dispatch auth gate — extracted so
/// the security boundary is a testable predicate rather than an inline
/// `matches!` no test can reach. A tag that returns `true` here is refused for
/// any peer not in the [`AuthenticatedPeers`] map (see [`dispatch_message`]):
/// requiring auth ⟹ an unauthenticated connection never reaches the handler.
///
/// Block announcements ARE gated (defense-in-depth, P1): although each block is
/// independently validated by BLAKE3 content integrity, per-entry `proof_hash`,
/// signed-to-content binding, and `state_proof.validate()`, the announcing peer
/// must also have passed the bilateral PoS handshake. Shard access, sync,
/// distributed-CA and cross-network transfer operations are gated for the same
/// reason.
pub fn message_requires_auth(tag: u8) -> bool {
    matches!(
        tag,
        TAG_SHARD_SEND | TAG_SHARD_FETCH
            | TAG_BLOCK_ANNOUNCE
            | TAG_SYNC_MESSAGE | TAG_BLOCK_FETCH_REQUEST
            | TAG_CA_KEY_SHARE | TAG_CA_SIGN_REQUEST | TAG_CA_SIGN_RESPONSE
            | TAG_TRANSFER_LOCK | TAG_TRANSFER_REGISTER_REQ | TAG_TRANSFER_REGISTER_ACK
            | TAG_TRANSFER_RELEASE | TAG_TRANSFER_ROLLBACK
            // S3.4: an attestation is a third party's statement about an asset
            // we hold, cached in a BOUNDED pool and eventually sealed on-chain
            // by the owner. Same standing as a block announcement: each one is
            // independently FALCON-verified, AND the submitting peer must have
            // passed the bilateral PoS handshake for this network.
            | TAG_MIRROR_ATTEST
            // D3: a presented asset chain is a peer offering an asset's verified
            // sub-chain for adoption into our off-spine received store. Its
            // internal lineage and every signer are FALCON-verified inside
            // `accept_asset_chain`, AND — because the store is now network-fed —
            // an UNAUTHENTICATED peer must never reach the accept path. This
            // gate is the only thing standing between an anonymous connection
            // and the received store; it is not optional.
            | TAG_ASSET_CHAIN
    )
}

/// The handler that owns a given wire tag.
///
/// A pure name for "which arm of the dispatch table runs this tag", extracted
/// so the tag→handler wiring is a value a test can assert on, rather than an
/// inline `match` arm no test can reach. [`dispatch_to_handler`] is the single
/// place that turns one of these back into the actual handler call, so this
/// enum and that `match` are the only two things that must agree — and
/// [`route_message`] (which produces these) is exhaustively unit-tested.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handler {
    /// Shard send/fetch (fetch additionally F6-authorized). `TAG_SHARD_SEND`
    /// / `TAG_SHARD_FETCH`.
    ShardDispatch,
    /// Block announcement. `TAG_BLOCK_ANNOUNCE`.
    BlockAnnounce,
    /// Sync / reflector message. `TAG_SYNC_MESSAGE`.
    SyncMessage,
    /// Block fetch request. `TAG_BLOCK_FETCH_REQUEST`.
    BlockFetchRequest,
    /// Shard availability announcement. `TAG_SHARD_ANNOUNCE`.
    ShardAnnounce,
    /// S3.4 mirror attestation. `TAG_MIRROR_ATTEST` (0x54).
    MirrorAttestation,
    /// D3 presented asset chain. `TAG_ASSET_CHAIN` (0x55).
    AssetChain,
    /// A2 shard-locate query. `TAG_SHARD_LOCATE`.
    ShardLocate,
    /// Share invite. `TAG_SHARE_INVITE`.
    ShareInvite,
    /// Direct message. `TAG_DIRECT_MESSAGE`.
    DirectMessage,
    /// Cross-network transfer message. `TAG_TRANSFER`.
    Transfer,
    /// Distributed-CA key share. `TAG_CA_KEY_SHARE`.
    CaKeyShare,
    /// Distributed-CA threshold sign request. `TAG_CA_SIGN_REQUEST`.
    CaSignRequest,
    /// Distributed-CA threshold sign response. `TAG_CA_SIGN_RESPONSE`.
    CaSignResponse,
    /// Key rotation announcement. `TAG_KEY_ROTATION`.
    KeyRotation,
    /// DNS resolution request. `TAG_DNS_RESOLVE`.
    DnsResolve,
    /// Phase H.1 rich DNS query. `TAG_DNS_QUERY`.
    DnsQuery,
    /// Cross-network transfer lock. `TAG_TRANSFER_LOCK`.
    TransferLock,
    /// Cross-network transfer register request. `TAG_TRANSFER_REGISTER_REQ`.
    TransferRegisterReq,
    /// Cross-network transfer register ack. `TAG_TRANSFER_REGISTER_ACK`.
    TransferRegisterAck,
    /// Cross-network transfer release. `TAG_TRANSFER_RELEASE`.
    TransferRelease,
    /// Cross-network transfer rollback. `TAG_TRANSFER_ROLLBACK`.
    TransferRollback,
    /// Gossip message (logged, not gated). `TAG_GOSSIP`.
    Gossip,
}

/// The routing decision for a wire tag: which handler owns it, that an
/// unauthenticated peer must be dropped BEFORE any handler, or that no arm
/// handles the tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Route {
    /// The tag is handled; [`dispatch_to_handler`] runs the named handler.
    Handler(Handler),
    /// The tag requires authentication and the peer is not authenticated — the
    /// message is dropped before the handler runs (see [`message_requires_auth`]).
    DropUnauthenticated,
    /// No arm handles this tag.
    Unknown,
}

/// The dispatch decision as a pure function of `(tag, is_authenticated)`.
///
/// This is the wiring [`dispatch_message`] executes, lifted out of the async
/// I/O path so it is exhaustively unit-testable WITHOUT a QUIC stream. Two
/// invariants live here and are proved in-module:
///
///  1. **Auth gate.** Every tag for which [`message_requires_auth`] is `true`
///     routes to [`Route::DropUnauthenticated`] when `is_authenticated` is
///     `false` — an unauthenticated peer never reaches the handler. Ungated
///     tags ignore `is_authenticated`.
///  2. **Tag→handler.** Each handled tag maps to exactly one [`Handler`];
///     everything else is [`Route::Unknown`].
///
/// `dispatch_message` computes `is_authenticated` from the live
/// [`AuthenticatedPeers`] map (only consulting it for gated tags, preserving
/// the original short-circuit) and then executes this decision. A future
/// refactor that misroutes a tag or drops the gate on one fails the unit tests
/// below without needing a live connection.
pub(crate) fn route_message(tag: u8, is_authenticated: bool) -> Route {
    // Gate asset-level operations on peer authentication + network scope.
    if message_requires_auth(tag) && !is_authenticated {
        return Route::DropUnauthenticated;
    }

    match tag {
        TAG_SHARD_SEND | TAG_SHARD_FETCH => Route::Handler(Handler::ShardDispatch),
        TAG_BLOCK_ANNOUNCE => Route::Handler(Handler::BlockAnnounce),
        TAG_SYNC_MESSAGE => Route::Handler(Handler::SyncMessage),
        TAG_BLOCK_FETCH_REQUEST => Route::Handler(Handler::BlockFetchRequest),
        TAG_SHARD_ANNOUNCE => Route::Handler(Handler::ShardAnnounce),
        TAG_MIRROR_ATTEST => Route::Handler(Handler::MirrorAttestation),
        TAG_ASSET_CHAIN => Route::Handler(Handler::AssetChain),
        TAG_SHARD_LOCATE => Route::Handler(Handler::ShardLocate),
        TAG_SHARE_INVITE => Route::Handler(Handler::ShareInvite),
        TAG_DIRECT_MESSAGE => Route::Handler(Handler::DirectMessage),
        TAG_TRANSFER => Route::Handler(Handler::Transfer),
        TAG_CA_KEY_SHARE => Route::Handler(Handler::CaKeyShare),
        TAG_CA_SIGN_REQUEST => Route::Handler(Handler::CaSignRequest),
        TAG_CA_SIGN_RESPONSE => Route::Handler(Handler::CaSignResponse),
        TAG_KEY_ROTATION => Route::Handler(Handler::KeyRotation),
        TAG_DNS_RESOLVE => Route::Handler(Handler::DnsResolve),
        TAG_DNS_QUERY => Route::Handler(Handler::DnsQuery),
        TAG_TRANSFER_LOCK => Route::Handler(Handler::TransferLock),
        TAG_TRANSFER_REGISTER_REQ => Route::Handler(Handler::TransferRegisterReq),
        TAG_TRANSFER_REGISTER_ACK => Route::Handler(Handler::TransferRegisterAck),
        TAG_TRANSFER_RELEASE => Route::Handler(Handler::TransferRelease),
        TAG_TRANSFER_ROLLBACK => Route::Handler(Handler::TransferRollback),
        TAG_GOSSIP => Route::Handler(Handler::Gossip),
        _ => Route::Unknown,
    }
}

/// Route a single message payload to the appropriate handler.
///
/// Asset-level operations (shard send/fetch, sync, block-fetch) AND block
/// announcements are gated on the sender being in the [`AuthenticatedPeers`]
/// map AND belonging to the same network. Announced blocks are ADDITIONALLY
/// validated by BLAKE3 content integrity, per-entry proof_hash, signed-to-
/// content binding, and state_proof.validate(). Gossip and unknown tags are
/// logged but not gated.
///
/// The routing decision itself is the pure [`route_message`]; this function
/// supplies the live authentication state and executes the result. Behavior is
/// byte-identical to the previous inline gate + match.
pub(crate) async fn dispatch_message(
    data: &[u8],
    stream: &mut stoq::Stream,
    peer_node_id: &str,
    peer_coord: &MatrixCoordinate,
    ctx: &PeerContext,
) {
    let tag = data[0];

    // Consult the auth gate ONLY for tags that demand it — preserving the
    // original short-circuit in which `verify_peer_access` is never called for
    // an ungated tag. For ungated tags the value is unused by `route_message`.
    let is_authenticated = !message_requires_auth(tag)
        || peer_auth::verify_peer_access(
            &ctx.authenticated_peers,
            peer_node_id,
            &ctx.network_id,
        )
        .await;

    match route_message(tag, is_authenticated) {
        // Gate rejected: drop silently, exactly as the previous early `return`.
        // `verify_peer_access` already logged the rejection.
        Route::DropUnauthenticated => {}
        Route::Unknown => {
            let short_id = &peer_node_id[..8.min(peer_node_id.len())];
            warn!("Unknown message tag 0x{:02x} from peer {}", tag, short_id);
        }
        Route::Handler(handler) => {
            dispatch_to_handler(handler, tag, data, stream, peer_node_id, peer_coord, ctx).await;
        }
    }
}

/// Execute a routed message: the tag→handler dispatch table, keyed on the
/// [`Handler`] that [`route_message`] chose.
///
/// This is the ONE place that turns a [`Handler`] back into a real handler
/// call; `tag` is still passed because the shard arm distinguishes send from
/// fetch. Every arm is fire-and-forget or logs its own error, matching the
/// original inline match byte-for-byte.
async fn dispatch_to_handler(
    handler: Handler,
    tag: u8,
    data: &[u8],
    stream: &mut stoq::Stream,
    peer_node_id: &str,
    peer_coord: &MatrixCoordinate,
    ctx: &PeerContext,
) {
    match handler {
        Handler::ShardDispatch => {
            // Record shard fetch demand for ngauge swarm intelligence.
            if tag == TAG_SHARD_FETCH {
                record_shard_demand(&data, peer_node_id, ctx).await;

                // F6: per-asset shard-fetch authorization. Beyond the coarse
                // same-network membership gate above, bind the fetch to (a)
                // the requester's PoS proof (verify_shard_proof_binding) and
                // (b) the shard belonging to an asset registered on our
                // chain (blockchain.authorizes_shard). A peer that passed
                // handshake but has no proof-of-state stake, or requests a
                // shard for an asset not on a shared chain, is refused.
                if !authorize_shard_fetch(&data, peer_node_id, ctx).await {
                    return;
                }
            }
            handle_shard_dispatch(&data, stream, ctx).await;
        }
        Handler::BlockAnnounce => {
            handle_block_announce(&data, peer_node_id, ctx).await;
        }
        Handler::SyncMessage => {
            handle_sync_message(&data[1..], stream, peer_node_id, peer_coord, ctx).await;
        }
        Handler::BlockFetchRequest => {
            handle_block_fetch_request(&data[1..], stream, peer_node_id, ctx).await;
        }
        Handler::ShardAnnounce => {
            handle_shard_announce(data, peer_node_id, ctx).await;
        }
        Handler::MirrorAttestation => {
            // S3.4: a mirror's signed "I hold and validated this" statement.
            // Fire-and-forget; the handler owns every rejection path.
            handle_mirror_attestation(data, peer_node_id, ctx).await;
        }
        Handler::AssetChain => {
            // D3: a peer presents an asset's verified sub-chain for adoption.
            // Fire-and-forget; `accept_asset_chain` owns every verification and
            // every rejection path. Only reachable for an authenticated peer
            // (gated above), and it can never produce a spine block — there is
            // no `Block` in a `PresentedAssetChain`.
            handle_asset_chain(data, peer_node_id, ctx).await;
        }
        Handler::ShardLocate => {
            // A2 upstream tracker fallback: answer "who has content_hash X?"
            // from our own live-mirror index + local store. This is a LOCATE
            // query (returns provider node_ids), not a data fetch.
            handle_shard_locate(&data, stream, peer_node_id, ctx).await;
        }
        Handler::ShareInvite => {
            handle_share_invite(data, peer_node_id, ctx).await;
        }
        Handler::DirectMessage => {
            handle_direct_message(data, peer_node_id, ctx).await;
        }
        Handler::Transfer => {
            handle_transfer_message(&data[1..], stream, peer_node_id, ctx).await;
        }
        Handler::CaKeyShare => {
            if let Err(e) = handle_ca_key_share(&data, peer_node_id, ctx).await {
                warn!("Failed to handle CA key share: {e}");
            }
        }
        Handler::CaSignRequest => {
            if let Err(e) = handle_ca_sign_request(&data, peer_node_id, ctx).await {
                warn!("Failed to handle CA sign request: {e}");
            }
        }
        Handler::CaSignResponse => {
            if let Err(e) = handle_ca_sign_response(&data, peer_node_id, ctx).await {
                warn!("Failed to handle CA sign response: {e}");
            }
        }
        Handler::KeyRotation => {
            if let Err(e) = handle_key_rotation(data, peer_node_id, ctx).await {
                warn!("Failed to handle key rotation: {e}");
            }
        }
        Handler::DnsResolve => {
            handle_dns_resolve_request(data, stream, peer_node_id, ctx).await;
        }
        Handler::DnsQuery => {
            // Phase H.1: rich DNS query — wire format `[tag][JSON]`.
            // Strip the tag byte; handler parses the rest as JSON.
            handle_dns_query(&data[1..], stream, peer_node_id, ctx).await;
        }
        Handler::TransferLock => {
            handle_transfer_lock(&data[1..], peer_node_id, ctx).await;
        }
        Handler::TransferRegisterReq => {
            handle_transfer_register_req(&data[1..], stream, peer_node_id, ctx).await;
        }
        Handler::TransferRegisterAck => {
            handle_transfer_register_ack(&data[1..], peer_node_id, ctx).await;
        }
        Handler::TransferRelease => {
            handle_transfer_release(&data[1..], peer_node_id, ctx).await;
        }
        Handler::TransferRollback => {
            handle_transfer_rollback(&data[1..], peer_node_id, ctx).await;
        }
        Handler::Gossip => {
            let short_id = &peer_node_id[..8.min(peer_node_id.len())];
            debug!(
                "Gossip message from peer {} ({} bytes)",
                short_id,
                data.len() - 1,
            );
        }
    }
}

/// F6 per-asset shard-fetch authorization.
///
/// `data` is a raw SHARD_FETCH message: `tag(1) + shard_id(32)`. Returns
/// `true` only when BOTH hold:
///   1. the requester carries a bound PoS proof for our network
///      (`verify_shard_proof_binding`), and
///   2. the requested shard belongs to an asset registered on our chain
///      (`blockchain.authorizes_shard`).
///
/// A `false` result causes the caller to drop the request silently (the
/// requester already logged the reason). SHARD_SEND (peers pushing shards
/// to us, integrity-checked by BLAKE3 in the store) is NOT gated here.
async fn authorize_shard_fetch(
    data: &[u8],
    peer_node_id: &str,
    ctx: &PeerContext,
) -> bool {
    // Parse shard_id: tag(1) + shard_id(32).
    if data.len() < 33 {
        debug!("F6: malformed SHARD_FETCH ({} bytes) — refused", data.len());
        return false;
    }
    let mut shard_id = [0u8; 32];
    shard_id.copy_from_slice(&data[1..33]);

    // (1) Requester-side proof binding.
    if !peer_auth::verify_shard_proof_binding(
        &ctx.authenticated_peers,
        peer_node_id,
        &ctx.network_id,
    )
    .await
    {
        return false;
    }

    // (2) Asset-level anchor: the shard must belong to a registered asset
    // on our chain. This binds the fetch to the asset's on-chain,
    // content-bound StateProof rather than to coarse membership.
    if !ctx.blockchain.authorizes_shard(&shard_id).await {
        let short_id = &peer_node_id[..8.min(peer_node_id.len())];
        warn!(
            "F6: shard {} not authorized for {} (no registered asset on our chain)",
            hex::encode(&shard_id[..8]),
            short_id,
        );
        return false;
    }

    true
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
    if let Err(e) =
        ebpf.set_peer_pos_validated(src_ip, true, hypermesh_ebpf::ALG_FALCON_1024)
    {
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

    // P5 unification: mirror this authenticated peer into the kernel
    // fast-path maps, keyed on the SAME IPv6 source P1 just authorized.
    // Writes pos_header_map[src].validated=1 + policy_map[src].requires_pos=1
    // so the XDP gate admits this source's HyperMesh traffic. No-op unless
    // an XDP program is attached (graceful degradation).
    if let Some(ref ctx) = peer_ctx {
        mirror_peer_auth_to_kernel(ctx, &connection);
    }

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

// ── Dispatch-routing unit tests ───────────────────────────────────────
//
// These drive the REAL `route_message` — the pure routing decision that
// `dispatch_message` executes — so the tag→handler wiring and the auth gate
// are proved BY STRUCTURE, without constructing a QUIC `stream`. The
// integration suites (`tests/s3_4_*`) prove the codec and the accept gate; this
// closes the remaining gap: that a framed `TAG_ASSET_CHAIN` / `TAG_MIRROR_ATTEST`
// from an authenticated peer actually routes to its handler, and that an
// unauthenticated one is dropped by the auth gate before the handler.
#[cfg(test)]
mod route_tests {
    use super::*;

    /// Every wire tag the dispatcher handles, paired with the handler that owns
    /// it and whether it is auth-gated. This table is the test's independent
    /// statement of the wiring; the tests below cross-check it against BOTH
    /// `route_message` and `message_requires_auth`, so a drift in any of the
    /// three is caught.
    const ROUTES: &[(u8, Handler, bool)] = &[
        (TAG_SHARD_SEND, Handler::ShardDispatch, true),
        (TAG_SHARD_FETCH, Handler::ShardDispatch, true),
        (TAG_BLOCK_ANNOUNCE, Handler::BlockAnnounce, true),
        (TAG_SYNC_MESSAGE, Handler::SyncMessage, true),
        (TAG_BLOCK_FETCH_REQUEST, Handler::BlockFetchRequest, true),
        (TAG_SHARD_ANNOUNCE, Handler::ShardAnnounce, false),
        (TAG_MIRROR_ATTEST, Handler::MirrorAttestation, true),
        (TAG_ASSET_CHAIN, Handler::AssetChain, true),
        (TAG_SHARD_LOCATE, Handler::ShardLocate, false),
        (TAG_SHARE_INVITE, Handler::ShareInvite, false),
        (TAG_DIRECT_MESSAGE, Handler::DirectMessage, false),
        (TAG_TRANSFER, Handler::Transfer, false),
        (TAG_CA_KEY_SHARE, Handler::CaKeyShare, true),
        (TAG_CA_SIGN_REQUEST, Handler::CaSignRequest, true),
        (TAG_CA_SIGN_RESPONSE, Handler::CaSignResponse, true),
        (TAG_KEY_ROTATION, Handler::KeyRotation, false),
        (TAG_DNS_RESOLVE, Handler::DnsResolve, false),
        (TAG_DNS_QUERY, Handler::DnsQuery, false),
        (TAG_TRANSFER_LOCK, Handler::TransferLock, true),
        (TAG_TRANSFER_REGISTER_REQ, Handler::TransferRegisterReq, true),
        (TAG_TRANSFER_REGISTER_ACK, Handler::TransferRegisterAck, true),
        (TAG_TRANSFER_RELEASE, Handler::TransferRelease, true),
        (TAG_TRANSFER_ROLLBACK, Handler::TransferRollback, true),
        (TAG_GOSSIP, Handler::Gossip, false),
    ];

    /// An authenticated peer's framed message reaches the handler that owns its
    /// tag; the gated flag in the table matches `message_requires_auth` exactly.
    #[test]
    fn authenticated_peer_routes_every_tag_to_its_handler() {
        for &(tag, handler, gated) in ROUTES {
            assert_eq!(
                route_message(tag, true),
                Route::Handler(handler),
                "tag 0x{tag:02x} must route to {handler:?} for an authenticated peer",
            );
            assert_eq!(
                message_requires_auth(tag),
                gated,
                "the gated flag for tag 0x{tag:02x} must match message_requires_auth",
            );
        }
    }

    /// An UNAUTHENTICATED peer is dropped before the handler for every gated
    /// tag, and still reaches the handler for every ungated tag. This is the
    /// auth gate proved end-to-end at the routing layer.
    #[test]
    fn unauthenticated_peer_is_dropped_for_gated_tags_only() {
        for &(tag, handler, gated) in ROUTES {
            let route = route_message(tag, false);
            if gated {
                assert_eq!(
                    route,
                    Route::DropUnauthenticated,
                    "gated tag 0x{tag:02x} must be dropped for an unauthenticated peer",
                );
            } else {
                assert_eq!(
                    route,
                    Route::Handler(handler),
                    "ungated tag 0x{tag:02x} must route regardless of authentication",
                );
            }
        }
    }

    /// The two arms task #51 names — 0x55 asset-chain and 0x54 attestation —
    /// stated explicitly: authenticated routes to the handler, unauthenticated
    /// is dropped. Both are the same gap; both are covered.
    #[test]
    fn asset_chain_and_attestation_arms_are_wired_and_gated() {
        assert_eq!(TAG_ASSET_CHAIN, 0x55, "asset-chain tag is 0x55");
        assert_eq!(TAG_MIRROR_ATTEST, 0x54, "mirror-attestation tag is 0x54");

        assert_eq!(
            route_message(TAG_ASSET_CHAIN, true),
            Route::Handler(Handler::AssetChain),
        );
        assert_eq!(
            route_message(TAG_ASSET_CHAIN, false),
            Route::DropUnauthenticated,
        );
        assert_eq!(
            route_message(TAG_MIRROR_ATTEST, true),
            Route::Handler(Handler::MirrorAttestation),
        );
        assert_eq!(
            route_message(TAG_MIRROR_ATTEST, false),
            Route::DropUnauthenticated,
        );
    }

    /// Exhaustive over all 256 tag values: the routing table is total and
    /// consistent. Exactly the tags in `ROUTES` are handled; every other tag is
    /// `Unknown`. No tag is ever gated by `message_requires_auth` without
    /// appearing (as gated) in `ROUTES`, so the gate can never guard a tag the
    /// table does not know about.
    #[test]
    fn routing_is_total_and_consistent_over_all_tags() {
        for tag in 0..=u8::MAX {
            let known = ROUTES.iter().find(|&&(t, _, _)| t == tag);
            match known {
                Some(&(_, handler, gated)) => {
                    assert_eq!(route_message(tag, true), Route::Handler(handler));
                    assert_eq!(
                        route_message(tag, false),
                        if gated {
                            Route::DropUnauthenticated
                        } else {
                            Route::Handler(handler)
                        },
                    );
                }
                None => {
                    // An unknown tag is never auth-gated, so authentication
                    // state cannot change its verdict.
                    assert!(
                        !message_requires_auth(tag),
                        "tag 0x{tag:02x} is gated but has no route",
                    );
                    assert_eq!(route_message(tag, true), Route::Unknown);
                    assert_eq!(route_message(tag, false), Route::Unknown);
                }
            }
        }
    }
}
