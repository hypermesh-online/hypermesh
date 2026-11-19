use super::*;

pub(in crate::network::message_handlers) async fn handle_block_fetch_request(
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

    let serialized_blocks = serialize_requested_blocks(&hashes, &ctx.blockchain).await;

    info!(
        "Serving {} of {} requested block(s) to {}",
        serialized_blocks.len(),
        hashes.len(),
        short_id,
    );

    send_block_fetch_response(serialized_blocks, stream, short_id).await;
}

pub(in crate::network::message_handlers) async fn handle_sync_message(
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

    let (mut sm, mut rp) = tokio::join!(ctx.sync_manager.lock(), ctx.reflector_pool.lock(),);

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

pub(in crate::network::message_handlers) async fn register_peer_as_reflector(
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

async fn serialize_requested_blocks(
    hashes: &[String],
    blockchain: &crate::blockchain::node_chain::NodeBlockchain,
) -> Vec<String> {
    let mut serialized_blocks = Vec::with_capacity(hashes.len());
    for hash in hashes {
        if let Some(block) = blockchain.get_block_by_hash(hash).await {
            serialized_blocks.extend(serialize_requested_block(&block, hash));
        }
    }
    serialized_blocks
}

fn serialize_requested_block(
    block: &crate::blockchain::block::Block,
    hash: &str,
) -> Option<String> {
    match serde_json::to_string(block) {
        Ok(json) => Some(json),
        Err(e) => {
            debug!("Failed to serialize block {}: {}", hash, e);
            None
        }
    }
}

async fn send_block_fetch_response(
    serialized_blocks: Vec<String>,
    stream: &mut stoq::Stream,
    short_id: &str,
) {
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
