// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! D3 — receive and send presented asset chains over STOQ.
//!
//! # One gate, not two
//!
//! S3.3's blocking QA finding was a SECOND, parallel list of structural checks
//! that drifted from the audit gate. This handler holds the line that fix drew:
//! it performs NO verification of its own. It decodes the payload and calls
//! [`accept_asset_chain`](crate::blockchain::NodeBlockchain::accept_asset_chain),
//! which owns every check — internal prev-link lineage via `AssetLineage::verify`,
//! every signer's FALCON envelope, the `has_ever_seen_asset`/`AlreadyOnSpine`
//! refusal, the received-store byte budget and the extension-only rule. Adding a
//! check here — even a seemingly redundant one — would recreate exactly the
//! divergence S3.3 closed.
//!
//! # The type invariant survives the wire
//!
//! The handler deals in [`PresentedAssetChain`] — a `Vec<BlockAssetEntry>` with
//! no `Block`, no index and no `previous_hash`. It never reaches `add_block`,
//! `insert_block` or `insert_received_block`, and never write-locks the spine.
//! `accept_asset_chain` files entries in the off-spine received store; nothing
//! here can turn a wire-received asset chain into a spine block. That separation
//! is structural (there is no `Block` to insert), not merely careful.
//!
//! # Admission is bounded, and a refusal leaves no partial state
//!
//! The received store is now fed by the network, so its bounds are what protect
//! against a flood: the 64 MiB
//! [`MAX_RECEIVED_STORE_BYTES`](crate::blockchain::MAX_RECEIVED_STORE_BYTES)
//! budget and the
//! [`MAX_RECEIVED_CHAINS`](crate::blockchain::MAX_RECEIVED_CHAINS) /
//! [`MAX_RECEIVED_CHAIN_ENTRIES`](crate::blockchain::MAX_RECEIVED_CHAIN_ENTRIES)
//! guards, all reject-not-evict — a full store refuses newcomers rather than
//! displacing verified history. `accept_asset_chain` judges every bound (and the
//! entry cap) BEFORE any signature work, and adopts under one write lock after
//! all judgement passes, so a refused chain leaves the store byte-for-byte where
//! it was.

use tracing::{info, warn};

use crate::blockchain::PresentedAssetChain;

use super::super::asset_chain_wire::{
    decode_presented_asset_chain, encode_presented_asset_chain,
};
use super::super::PeerContext;

/// Handle a received `ASSET_CHAIN_TAG` message (tag 0x55).
///
/// `data` is the WHOLE message including the tag byte, matching
/// `handle_mirror_attestation` and `handle_block_announce`.
///
/// Returns `true` when the chain was adopted. Failures are logged and swallowed
/// — this is a fire-and-forget presentation, and a peer learns nothing from our
/// silence that it could not learn by re-reading the rules.
pub(super) async fn handle_asset_chain(
    data: &[u8],
    peer_node_id: &str,
    ctx: &PeerContext,
) -> bool {
    let short_id = &peer_node_id[..8.min(peer_node_id.len())];

    let chain = match decode_presented_asset_chain(data) {
        Ok(chain) => chain,
        Err(e) => {
            warn!("Asset chain from {short_id} could not be decoded: {e}");
            return false;
        }
    };

    let asset = hex::encode(chain.asset_hash);
    let asset_short = &asset[..16.min(asset.len())];

    // The ONE gate: internal lineage, every signer's FALCON envelope, the
    // already-on-spine refusal and the bounded received store — all of it inside
    // `accept_asset_chain`, none of it restated here.
    match ctx.blockchain.accept_asset_chain(chain).await {
        Ok(receipt) => {
            info!(
                asset = %asset_short,
                entries = receipt.entries,
                added = receipt.added,
                "D3: adopted a presented asset chain submitted by peer {short_id}"
            );
            true
        }
        Err(e) => {
            warn!("Asset chain {asset_short} from {short_id} REJECTED: {e}");
            false
        }
    }
}

/// Present one asset chain to a connected peer.
///
/// The producing half of the surface: a holder offers an asset's verified
/// sub-chain to a peer that may adopt it. Fire-and-forget — the stream is
/// opened, written and dropped, exactly as `send_mirror_attestation` does.
pub async fn send_presented_asset_chain(
    connection: &stoq::Connection,
    chain: &PresentedAssetChain,
) -> Result<(), String> {
    let payload = encode_presented_asset_chain(chain)
        .map_err(|e| format!("failed to encode presented asset chain: {e}"))?;

    let mut stream = connection
        .open_stream()
        .await
        .map_err(|e| format!("failed to open stream for asset chain: {e}"))?;
    stream
        .send(&payload)
        .await
        .map_err(|e| format!("failed to send asset chain: {e}"))?;
    Ok(())
}
