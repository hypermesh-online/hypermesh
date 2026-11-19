use super::*;
use hypermesh_lib::ContentHash;

pub(super) struct FetchContext<'a> {
    pub(super) authenticated_peers: &'a AuthenticatedPeers,
    pub(super) network_id: &'a str,
    pub(super) blockchain: &'a crate::blockchain::node_chain::NodeBlockchain,
    pub(super) swarm_demand_tracker: &'a crate::network::SwarmDemandTracker,
}
impl<'a> From<&'a PeerContext> for FetchContext<'a> {
    fn from(ctx: &'a PeerContext) -> Self {
        Self {
            authenticated_peers: &ctx.authenticated_peers,
            network_id: &ctx.network_id,
            blockchain: &ctx.blockchain,
            swarm_demand_tracker: &ctx.swarm_demand_tracker,
        }
    }
}

pub(super) fn parse_shard_fetch(data: &[u8]) -> Option<[u8; 32]> {
    if data.first() != Some(&TAG_SHARD_FETCH) || data.len() < 33 {
        debug!("F6: malformed SHARD_FETCH ({} bytes) — refused", data.len());
        return None;
    }
    let mut shard_id = [0u8; 32];
    shard_id.copy_from_slice(&data[1..33]);
    Some(shard_id)
}

pub(super) async fn prepare_shard_fetch(
    data: &[u8],
    peer_node_id: &str,
    connection_id: &str,
    ctx: &FetchContext<'_>,
) -> bool {
    let Some(shard_id) = parse_shard_fetch(data) else {
        return false;
    };
    if !authorize_shard_fetch(&shard_id, peer_node_id, connection_id, ctx).await {
        return false;
    }
    commit_authorized_fetch_demand(shard_id, peer_node_id, connection_id, ctx).await
}

pub(super) async fn commit_authorized_fetch_demand(
    shard_id: [u8; 32],
    peer_node_id: &str,
    connection_id: &str,
    ctx: &FetchContext<'_>,
) -> bool {
    ctx.swarm_demand_tracker
        .commit_authorized_fetch(
            ctx.authenticated_peers,
            peer_node_id,
            connection_id,
            ctx.network_id,
            ContentHash(shard_id),
        )
        .await
}

pub(super) async fn authorize_shard_fetch(
    shard_id: &[u8; 32],
    peer_node_id: &str,
    connection_id: &str,
    ctx: &FetchContext<'_>,
) -> bool {
    // (1) Requester-side proof binding.
    if !peer_auth::verify_shard_proof_binding(
        &ctx.authenticated_peers,
        peer_node_id,
        connection_id,
        &ctx.network_id,
    )
    .await
    {
        return false;
    }

    // (2) Asset-level anchor: the shard must belong to a registered asset
    // on our chain. This binds the fetch to the asset's on-chain,
    // content-bound StateProof rather than to coarse membership.
    if !ctx.blockchain.authorizes_shard(shard_id).await {
        let short_id = &peer_node_id[..8.min(peer_node_id.len())];
        warn!(
            "F6: shard {} not authorized for {} (no registered asset on our chain)",
            hex::encode(&shard_id[..8]),
            short_id,
        );
        return false;
    }

    // Requester/session/network checks above use one registry row. Finer
    // per-asset scope authorization remains outside this isolation fix.

    true
}
