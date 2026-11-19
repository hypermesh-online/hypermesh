use super::*;

pub(in crate::network::message_handlers) async fn handle_key_rotation(
    data: &[u8],
    peer_node_id: &str,
    ctx: &PeerContext,
) -> Result<()> {
    if data.len() < 2 {
        return Err(anyhow!("Key rotation message too short"));
    }
    let msg_data = &data[1..];
    let rotation: serde_json::Value =
        serde_json::from_slice(msg_data).map_err(|e| anyhow!("Invalid key rotation JSON: {e}"))?;

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

    log_key_rotation(peer_node_id, old_fp, new_fp, reason);

    // Store in peer's rotation chain
    if let Some(ref chains) = ctx.rotation_chains {
        let mut chains = chains.write().await;
        let chain = chains
            .entry(peer_node_id.to_string())
            .or_insert_with(Vec::new);
        append_rotation(chain, &rotation, peer_node_id, old_fp);
    }

    Ok(())
}

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

pub(in crate::network::message_handlers) async fn handle_share_invite(
    data: &[u8],
    peer_node_id: &str,
    ctx: &PeerContext,
) {
    // Peek the invite for a log line before storing (best-effort).
    if let Ok(invite) =
        serde_json::from_slice::<crate::sharing::invite::ShareInvite>(&data[1.min(data.len())..])
    {
        let sender_display = invite
            .sender_name
            .as_deref()
            .unwrap_or_else(|| &invite.sender_node_id[..8.min(invite.sender_node_id.len())]);
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

pub(in crate::network::message_handlers) async fn handle_transfer_message(
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

            respond_to_transfer(transfer_id, stream, short_id).await;
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

pub(in crate::network::message_handlers) async fn handle_direct_message(
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
                .unwrap_or_else(|| &msg.sender_node_id[..8.min(msg.sender_node_id.len())]);
            info!(
                "Received message {} from {} to {}",
                &msg.message_id[..8.min(msg.message_id.len())],
                sender_display,
                &msg.recipient_node_id[..8.min(msg.recipient_node_id.len())],
            );
            store_direct_message(msg, ctx).await;
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

fn log_key_rotation(peer_node_id: &str, old_fp: &str, new_fp: &str, reason: &str) {
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
}

fn append_rotation(
    chain: &mut Vec<serde_json::Value>,
    rotation: &serde_json::Value,
    peer_node_id: &str,
    old_fp: &str,
) {
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

async fn respond_to_transfer(transfer_id: String, stream: &mut stoq::Stream, short_id: &str) {
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
    tagged.push(super::super::protocol::TAG_TRANSFER);
    tagged.extend_from_slice(&reply);
    if let Err(e) = stream.send(&tagged).await {
        warn!("Failed to send transfer response to {}: {}", short_id, e);
    }
}

async fn store_direct_message(msg: crate::messaging::message::DirectMessage, ctx: &PeerContext) {
    if let Some(ref store) = ctx.message_store {
        if let Err(e) = store.add(msg).await {
            warn!("Failed to store direct message: {e}");
        }
    }
}
