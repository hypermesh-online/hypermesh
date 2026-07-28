// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Receiving someone else's asset chain IS receiving an asset.
//!
//! Under "node ≡ asset ≡ index, each asset is its own chain," adopting a
//! presented asset's verified history and "receiving an asset" are the same
//! operation. This module is that accept path.
//!
//! # D2a — a shell over the still-present internals
//!
//! This commit introduces the *target* public surface — [`accept_asset_chain`],
//! [`ReceivedAssetStore`], [`PresentedAssetChain`] and the query accessors — but
//! its bodies delegate into the existing `foreign/` internals, which remain
//! unchanged. The move of the logic itself, and the deletion of `foreign/`,
//! happen in the next commit. Keeping the two steps apart makes the rename
//! bisectable: this commit changes only names and adds a thin surface; the next
//! moves bodies without changing behavior.
//!
//! The public names re-export `foreign/`'s types under model-neutral aliases so
//! callers can already speak the target vocabulary.
//!
//! [`accept_asset_chain`]: NodeBlockchain::accept_asset_chain

use super::block::BlockAssetEntry;
use super::chain::NodeBlockchain;
use super::lineage::AssetLineage;

pub use super::foreign::{
    chain_footprint_bytes, entry_footprint_bytes, ForeignAssetChain as PresentedAssetChain,
    ForeignChainReceipt as AcceptReceipt, ForeignChainReject as AcceptReject,
    ForeignChainStore as ReceivedAssetStore, StoreBound, MAX_FOREIGN_CHAINS as MAX_RECEIVED_CHAINS,
    MAX_FOREIGN_CHAIN_ENTRIES as MAX_RECEIVED_CHAIN_ENTRIES,
    MAX_FOREIGN_STORE_BYTES as MAX_RECEIVED_STORE_BYTES,
};

impl NodeBlockchain {
    /// Accept a presented asset's verified sub-chain, off-spine.
    ///
    /// Delegates (D2a) to the still-present `foreign/` internals. Consumes a
    /// [`PresentedAssetChain`] — a `Vec<BlockAssetEntry>` with no `Block`, no
    /// index and no `previous_hash` — so no path from here can reach the node
    /// spine's `insert_block`.
    pub async fn accept_asset_chain(
        &self,
        chain: PresentedAssetChain,
    ) -> Result<AcceptReceipt, AcceptReject> {
        self.accept_foreign_asset_chain(chain).await
    }

    /// Whether an adopted received chain for `asset_hash` is SHADOWED by this
    /// container's own spine.
    pub async fn received_chain_is_shadowed(&self, asset_hash: &[u8; 32]) -> bool {
        self.foreign_chain_is_shadowed(asset_hash).await
    }

    /// The adopted received history for `asset_hash`, if any AND if the spine
    /// has not since taken the asset over.
    pub async fn received_asset_lineage(&self, asset_hash: &[u8; 32]) -> Option<AssetLineage> {
        self.foreign_asset_lineage(asset_hash).await
    }

    /// The adopted head entry for `asset_hash`, if any and unshadowed.
    pub async fn received_asset_head(&self, asset_hash: &[u8; 32]) -> Option<BlockAssetEntry> {
        self.foreign_asset_head(asset_hash).await
    }

    /// Whether the off-spine store holds a chain for `asset_hash`.
    pub async fn has_received_asset_chain(&self, asset_hash: &[u8; 32]) -> bool {
        self.has_foreign_asset_chain(asset_hash).await
    }

    /// Bytes of received asset-chain material held off-spine.
    pub async fn received_chain_bytes(&self) -> usize {
        self.foreign_chain_bytes().await
    }

    /// Number of distinct received asset-chains adopted.
    pub async fn received_chain_count(&self) -> usize {
        self.foreign_chain_count().await
    }

    /// Release the adopted chain for `asset_hash`; returns entries dropped.
    pub async fn forget_received_asset_chain(&self, asset_hash: &[u8; 32]) -> usize {
        self.forget_foreign_asset_chain(asset_hash).await
    }

    /// Every received asset-chain this container holds off-spine.
    pub async fn received_asset_hashes(&self) -> Vec<[u8; 32]> {
        self.foreign_asset_hashes().await
    }

    /// The adopted chains that are SHADOWED by the spine.
    pub async fn shadowed_received_asset_chains(&self) -> Vec<[u8; 32]> {
        self.shadowed_foreign_asset_chains().await
    }

    /// Release every SHADOWED received chain; returns how many were dropped.
    pub async fn forget_shadowed_received_asset_chains(&self) -> usize {
        self.forget_shadowed_foreign_asset_chains().await
    }
}
