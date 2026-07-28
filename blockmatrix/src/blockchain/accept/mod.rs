// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Receiving someone else's asset chain IS receiving an asset.
//!
//! Under "node ≡ asset ≡ index, each asset is its own chain," adopting a
//! presented asset's verified history and "receiving an asset" are the same
//! operation. This module is that accept path.
//!
//! # It accepts ENTRIES, never a `Block`
//!
//! A presented asset's history is a run of entries from somebody else's
//! container: each entry carries the asset's own internal lineage (S3.2's
//! `prev_asset_entry` / `asset_seq`), not a node-block index. It is accepted as
//! **entries**, verified against that internal lineage plus every signer's
//! FALCON envelope (H3), and filed in a [`ReceivedAssetStore`]. None of the
//! node's block-chain state moves: not `head`, not `blocks`, not `stats`, not
//! the S3.1 [`AssetChainIndex`](super::asset_index::AssetChainIndex).
//!
//! # The type-level separation that makes that structural, not merely careful
//!
//! Two accept paths coexist, kept apart **by type**:
//!
//! * The block-accept path consumes a [`Block`](super::block::Block) and is the
//!   only thing that can reach `insert_block`.
//! * This path consumes a [`PresentedAssetChain`] — a `Vec<BlockAssetEntry>`
//!   with no index, no `previous_hash` and no block hash. **There is no `Block`
//!   here to insert**, so no code path from this module can produce one.
//!
//! This module never calls `add_block`, `insert_block` or `insert_received_block`,
//! and never write-locks `blocks`, `head`, `stats`, `hash_index` or
//! `asset_index`. Consequently `insert_received_block` keeps EXACTLY the
//! strictness it had (F7): a received asset-chain being acceptable is not, and
//! cannot become, a way for a received *block* to enter the node's chain.
//!
//! # Non-destructive, and non-shadowing
//!
//! Adoption adds; it never clears. Device genesis and every previously-adopted
//! chain survive an accept, and an asset whose title this container already
//! holds on its own block chain is REFUSED
//! ([`AcceptReject::AlreadyOnSpine`]) rather than shadowed: the block chain is
//! authoritative for the assets it holds, and an import may not offer a second
//! opinion about one of them.
//!
//! The INVERSE ordering — accept a chain for asset X, then receive a block
//! carrying X — is deliberately not blocked. `insert_received_block` does not
//! consult this store: letting a received import veto a block would hand a remote
//! sender a censorship primitive over our own chain. The block chain simply
//! wins, as it does on every read path. What that ordering must NOT leave behind
//! is a public accessor still serving the superseded history as if it were live,
//! so [`received_asset_lineage`](NodeBlockchain::received_asset_lineage) and
//! [`received_asset_head`](NodeBlockchain::received_asset_head) return `None`
//! once the block chain holds the asset, and
//! [`received_chain_is_shadowed`](NodeBlockchain::received_chain_is_shadowed) is
//! the explicit signal.

use super::block::BlockAssetEntry;
use super::chain::NodeBlockchain;
use super::mutations::signer_binds_to_author;

mod asset_chain;
mod presented;
mod store;

pub use presented::{AcceptReceipt, AcceptReject, PresentedAssetChain, StoreBound};
pub use store::{
    chain_footprint_bytes, entry_footprint_bytes, ReceivedAssetStore, MAX_RECEIVED_CHAINS,
    MAX_RECEIVED_CHAIN_ENTRIES, MAX_RECEIVED_STORE_BYTES,
};

/// Verify every signer along a presented chain.
///
/// Split out of [`NodeBlockchain::accept_asset_chain`] so the accept body stays
/// a flat decision table. Uses H3's `signer_binds_to_author` — the SAME function
/// the block-accept path uses, so "who may author an entry" has exactly one
/// definition in this crate.
fn verify_every_signer(entries: &[BlockAssetEntry]) -> Result<(), AcceptReject> {
    for (position, entry) in entries.iter().enumerate() {
        if entry.signed_proof.is_none() {
            return Err(AcceptReject::Unsigned { position });
        }
        let signer_pubkey = entry
            .verify_signed_proof()
            .map_err(|detail| AcceptReject::BadSignature { position, detail })?;
        if !signer_binds_to_author(&signer_pubkey, entry) {
            return Err(AcceptReject::SignerNotAuthor { position });
        }
    }
    Ok(())
}

impl NodeBlockchain {
    /// Accept a presented asset's verified sub-chain.
    ///
    /// See the [module docs](self) for why it accepts entries (never a `Block`)
    /// and how that keeps it structurally separate from the block-accept path.
    ///
    /// Order of judgement (all fail-closed, cheapest first — and the ordering
    /// is load-bearing, not decorative):
    /// 1. non-empty, and within [`MAX_RECEIVED_CHAIN_ENTRIES`] — bounds the
    ///    signature work an untrusted caller can provoke;
    /// 2. the asset is not one this container's own block chain holds or has
    ///    held ([`AcceptReject::AlreadyOnSpine`]) — a cheap pre-filter,
    ///    re-asked authoritatively at step 6;
    /// 3. **capacity** ([`ReceivedAssetStore::admission_check`]) — asked HERE,
    ///    before any FALCON-1024 work, so that at steady-state capacity a
    ///    refusal costs O(1) rather than up to [`MAX_RECEIVED_CHAIN_ENTRIES`]
    ///    verifications. Skipped when this asset is already held here, because
    ///    extending a held chain is not growth and must not be refused by a
    ///    bound on growth;
    /// 4. **internal lineage** — [`AssetLineage::verify`](super::lineage::AssetLineage::verify),
    ///    i.e. every entry is for this asset, `proof_hash == BLAKE3(serialize(state_proof))`,
    ///    the proof is content-bound, the root IS an asset-genesis, and every
    ///    later entry names its predecessor's `lineage_id` with `asset_seq`
    ///    advancing by exactly one. Node-block indices play no part;
    /// 5. **every signer** — FALCON-1024 envelope present, valid, and bound to
    ///    the identity the entry claims as author;
    /// 6. adoption — extension-only, and capacity re-judged authoritatively
    ///    under the write lock, with the ownership question re-asked while the
    ///    `asset_index` read lock is HELD (see below).
    ///
    /// # Why steps 2 and 3 are optimizations and not gates
    ///
    /// No lock spans steps 2/3 and step 6, so both answers can go stale. Each
    /// is therefore re-asked at step 6 against the state that actually admits
    /// the entries. The capacity question is re-asked by calling the SAME
    /// [`ReceivedAssetStore::admission_check`] with the same inputs (a new chain
    /// charges its whole self in both places), so the early and authoritative
    /// refusals cannot name different bounds or different reasons.
    ///
    /// # The step-6 lock, and the TOCTOU it closes
    ///
    /// Step 2 reads `asset_index` and releases it; a block carrying this asset
    /// could land before step 6. Step 6 therefore takes `asset_index` (read)
    /// and holds it across the `received_chains` write — the documented order
    /// (`… → asset_index → mirror_attestations → received_chains`), so no
    /// inversion — which excludes `insert_block`'s `asset_index` WRITE for the
    /// duration. The block chain cannot acquire this asset in the window between
    /// the authoritative check and the adoption.
    ///
    /// On success the chain is queryable through
    /// [`received_asset_lineage`](Self::received_asset_lineage) and
    /// [`asset_lineage`](Self::asset_lineage). Block-chain height, head, block
    /// count and the S3.1 index are untouched.
    pub async fn accept_asset_chain(
        &self,
        chain: PresentedAssetChain,
    ) -> Result<AcceptReceipt, AcceptReject> {
        let asset_hash = chain.asset_hash;
        let outcome = self.accept_asset_chain_inner(chain).await;

        // ONE refusal log site, so no rejection path can be added without one.
        // `warn` and not `debug`: `main.rs` caps the default subscriber at INFO,
        // and a refusal nobody sees is a refusal nobody can diagnose.
        if let Err(reject) = &outcome {
            tracing::warn!(
                asset = %&hex::encode(asset_hash)[..16],
                reject = %reject,
                "refused a received asset-chain (the node's block chain is untouched)"
            );
        }
        outcome
    }

    /// Body of [`Self::accept_asset_chain`]; see its docs for the order of
    /// judgement. Split out only so refusals have a single log site.
    async fn accept_asset_chain_inner(
        &self,
        chain: PresentedAssetChain,
    ) -> Result<AcceptReceipt, AcceptReject> {
        // (1) shape.
        if chain.is_empty() {
            return Err(AcceptReject::Empty);
        }
        if chain.len() > MAX_RECEIVED_CHAIN_ENTRIES {
            return Err(AcceptReject::TooLong {
                presented: chain.len(),
                limit: MAX_RECEIVED_CHAIN_ENTRIES,
            });
        }

        // (2) The block chain owns the assets it holds. F1's tombstone is
        // consulted, so an asset whose entries were pruned still counts as ours.
        if self.has_ever_seen_asset(&chain.asset_hash).await {
            return Err(AcceptReject::AlreadyOnSpine);
        }

        // (3) A3 — capacity BEFORE signature work.
        {
            let store = self.received_chains.read().await;
            if !store.contains(&chain.asset_hash) {
                store.admission_check(true, chain_footprint_bytes(&chain.entries))?;
            }
        }

        // (4) internal lineage, (5) every signer — the expensive half.
        chain
            .as_lineage()
            .verify()
            .map_err(AcceptReject::LineageBroken)?;

        verify_every_signer(&chain.entries)?;

        let head = chain.head().ok_or(AcceptReject::Empty)?.clone();

        // (6) authoritative. `asset_index` (read) is held across the
        // `received_chains` write, in the documented order.
        let (entries, added) = {
            let asset_index = self.asset_index.read().await;
            if asset_index.has_ever_seen_asset(&chain.asset_hash) {
                return Err(AcceptReject::AlreadyOnSpine);
            }
            let mut store = self.received_chains.write().await;
            let added = store.adopt(chain.asset_hash, chain.entries)?;
            (store.entries(&chain.asset_hash).len(), added)
        };

        tracing::info!(
            asset = %&hex::encode(chain.asset_hash)[..16],
            entries,
            added,
            "adopted a received asset-chain (the node's block chain is untouched)"
        );

        Ok(AcceptReceipt {
            asset_hash: chain.asset_hash,
            entries,
            added,
            head_lineage_id: head.lineage_id(),
            head_seq: head.asset_seq(),
        })
    }
}
