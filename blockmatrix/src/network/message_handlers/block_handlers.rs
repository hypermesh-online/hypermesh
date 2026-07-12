// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Block announcement and propagation handlers.

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::assets::core::asset_id::{AssetCategory, BaseSystemType};
use crate::blockchain::block::Block;
use crate::dns::DnsBlockEntry;
use crate::network::hash_bucket::SpatialBucketAssigner;

use super::super::PeerContext;
use hypermesh_lib::BlockchainScope;

// ── Block handlers ───────────────────────────────────────────────────

/// Handle a received block announcement (tag 0x03).
/// Dispatches to scope-specific handlers based on `PeerContext::blockchain_scope`.
pub(super) async fn handle_block_announce(
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
        // Signed-to-content (mirror invariant, P1): the proof MUST be bound to
        // this entry's asset_hash (SpaceProof.file_hash == hex(asset_hash)).
        // Rejects a valid proof detached from asset A and replayed for asset B.
        if !entry.content_binding_ok() {
            warn!(
                "Block {} entry {} proof not bound to its asset_hash from peer {} \
                 (signed-to-content violation)",
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
