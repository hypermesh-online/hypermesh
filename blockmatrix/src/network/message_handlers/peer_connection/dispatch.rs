use super::*;

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
    connection_id: &str,
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
            connection_id,
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
            dispatch_to_handler(
                handler,
                tag,
                data,
                stream,
                peer_node_id,
                connection_id,
                peer_coord,
                ctx,
            )
            .await;
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
    connection_id: &str,
    peer_coord: &MatrixCoordinate,
    ctx: &PeerContext,
) {
    match handler {
        Handler::ShardDispatch => {
            dispatch_shard(tag, data, stream, peer_node_id, connection_id, ctx).await
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
            handle_mirror_attestation(data, peer_node_id, ctx).await;
        }
        Handler::AssetChain => {
            handle_asset_chain(data, peer_node_id, ctx).await;
        }
        Handler::ShardLocate => {
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
        other => dispatch_control(other, data, stream, peer_node_id, ctx).await,
    }
}

async fn dispatch_shard(
    tag: u8,
    data: &[u8],
    stream: &mut stoq::Stream,
    peer_node_id: &str,
    connection_id: &str,
    ctx: &PeerContext,
) {
    // Record shard fetch demand for ngauge swarm intelligence.
    if tag == TAG_SHARD_FETCH {
        if !super::fetch::prepare_shard_fetch(
            data,
            peer_node_id,
            connection_id,
            &super::fetch::FetchContext::from(ctx),
        )
        .await
        {
            return;
        }
    }
    handle_shard_dispatch(&data, stream, ctx).await;
}

async fn dispatch_control(
    handler: Handler,
    data: &[u8],
    stream: &mut stoq::Stream,
    peer_node_id: &str,
    ctx: &PeerContext,
) {
    match handler {
        Handler::CaKeyShare => log_control_result(
            handle_ca_key_share(&data, peer_node_id, ctx).await,
            "Failed to handle CA key share: ",
        ),
        Handler::CaSignRequest => log_control_result(
            handle_ca_sign_request(&data, peer_node_id, ctx).await,
            "Failed to handle CA sign request: ",
        ),
        Handler::CaSignResponse => log_control_result(
            handle_ca_sign_response(&data, peer_node_id, ctx).await,
            "Failed to handle CA sign response: ",
        ),
        Handler::CrlRequest => log_control_result(
            handle_crl_request(data, peer_node_id, ctx).await,
            "Failed to handle CRL request: ",
        ),
        Handler::CrlResponse => log_control_result(
            handle_crl_response(data, peer_node_id, ctx).await,
            "Failed to handle CRL response: ",
        ),
        Handler::KeyRotation => log_control_result(
            handle_key_rotation(data, peer_node_id, ctx).await,
            "Failed to handle key rotation: ",
        ),
        other => dispatch_auxiliary(other, data, stream, peer_node_id, ctx).await,
    }
}

async fn dispatch_auxiliary(
    handler: Handler,
    data: &[u8],
    stream: &mut stoq::Stream,
    peer_node_id: &str,
    ctx: &PeerContext,
) {
    match handler {
        Handler::DnsResolve => {
            handle_dns_resolve_request(data, stream, peer_node_id, ctx).await;
        }
        Handler::DnsQuery => {
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
        _ => unreachable!("handler already dispatched by the preceding tables"),
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

fn log_control_result(result: Result<()>, message: &str) {
    if let Err(e) = result {
        warn!("{message}{e}");
    }
}
