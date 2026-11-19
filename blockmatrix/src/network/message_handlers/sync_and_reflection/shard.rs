use super::*;

pub(in crate::network::message_handlers) async fn handle_shard_dispatch(
    data: &[u8],
    stream: &mut stoq::Stream,
    ctx: &PeerContext,
) {
    // A6.6: pass the serving node's chain so a SHARD_FETCH hit is answered
    // with an ENVELOPE carrying the shard's on-chain registration — the
    // fetcher re-anchors it on its own chain (torrent share-compute model).
    match shard_transport::handle_shard_message(data, &ctx.shard_store, Some(&ctx.blockchain)).await
    {
        Ok(Some(response_data)) => {
            send_shard_response(stream, &response_data).await;
        }
        Ok(None) => {}
        Err(e) => {
            warn!("Shard message error: {}", e);
        }
    }
}

pub(in crate::network::message_handlers) async fn handle_shard_announce(
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
            short_id,
            expected_len,
            data.len(),
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
        // Shards belong to their network: register the announced providers under
        // THIS node's network. Under a single joined network this is the same
        // canonical NetworkId every reader (the replication poll loop, the
        // shard-locate handler, the IPC fetch path) keys on, so the index stays
        // consistent; it stops silently flattening to the default network once
        // multiple networks exist.
        let network = NetworkId::from_wire_str(&ctx.network_id);
        index
            .register_provider_in_network(network, peer_node_id, &shard_ids)
            .await;
        debug!(
            "Shard announce from {}: {} shard(s) registered",
            &peer_node_id[..8.min(peer_node_id.len())],
            count,
        );
    }
}

pub(in crate::network::message_handlers) async fn handle_shard_locate(
    data: &[u8],
    stream: &mut stoq::Stream,
    peer_node_id: &str,
    ctx: &PeerContext,
) {
    use crate::network::swarm_provider::{build_shard_locate_response, parse_shard_locate_request};

    let short = &peer_node_id[..8.min(peer_node_id.len())];

    let content_hash = match parse_shard_locate_request(data) {
        Some(h) => h,
        None => {
            debug!("Malformed shard-locate request from {}", short);
            return;
        }
    };

    // Providers known to THIS peer from its live-mirror index, keyed on THIS
    // node's network (the same NetworkId `handle_shard_announce` registers under).
    let network = NetworkId::from_wire_str(&ctx.network_id);
    let mut providers: Vec<String> = match ctx.shard_location_index {
        Some(ref index) => index.get_providers_in_network(network, &content_hash).await,
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

async fn send_shard_response(stream: &mut stoq::Stream, response_data: &[u8]) {
    if let Err(e) = stream.send(response_data).await {
        warn!("Failed to send shard response: {}", e);
    }
}
