// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! S3.4 — the THIRD accept mode: a foreign asset's verified sub-chain.
//!
//! # The two accept modes that already exist, and why neither fits
//!
//! 1. [`add_block`](super::mutations) — LOCAL production. We author the entry,
//!    we stamp its lineage, we sign it.
//! 2. [`insert_received_block`](super::mutations) — a peer's block joining OUR
//!    node spine. Linkage is **node-index arithmetic**: `blocks[index - 1]`,
//!    with a hard reject on any hash mismatch (F7). A block that does not
//!    continue *our* spine is either buffered as an orphan or refused.
//!
//! A foreign ASSET's history is neither. It is a run of entries from somebody
//! else's container: its blocks carry their indices, their predecessors and
//! their genesis. Fed to mode 2 they can only ever buffer as orphans whose
//! predecessor is a block we will never hold, and TTL-die at
//! [`ORPHAN_TTL`](super::chain::ORPHAN_TTL). The only pre-S3.4 primitive that
//! adopts a foreign root — [`adopt_genesis`](super::chain::NodeBlockchain::adopt_genesis)
//! — **wipes the chain**, which trades one asset's history for all of ours.
//!
//! # What this mode does instead
//!
//! It accepts the asset's chain as **entries, not blocks**, verifies it against
//! its OWN internal lineage (S3.2's `prev_asset_entry` / `asset_seq`) plus every
//! signer's FALCON envelope (H3), and files it in an off-spine
//! [`ForeignChainStore`]. Nothing about the node spine moves: not `head`, not
//! `blocks`, not `stats`, not the S3.1
//! [`AssetChainIndex`](super::asset_index::AssetChainIndex).
//!
//! # Keeping the two rejection domains apart — the shape, and why it cannot
//! fall through
//!
//! F7's *principle* (never splice something whose linkage you cannot verify)
//! generalizes. F7's *implementation* (node-index arithmetic against our own
//! `blocks` map) does not. They are kept apart **by type**, not by a flag:
//!
//! * The spine accept mode consumes a [`Block`](super::block::Block) and is the
//!   only thing that can ever reach `insert_block`.
//! * This mode consumes a [`ForeignAssetChain`] — a `Vec<BlockAssetEntry>` with
//!   no index, no `previous_hash` and no block hash. **There is no `Block`
//!   here to insert**, so no code path from this module can produce one.
//! * This module never calls `add_block`, `insert_block` or
//!   `insert_received_block`, and never takes a write lock on `blocks`,
//!   `head`, `stats`, `hash_index` or `asset_index`.
//!
//! Consequently `insert_received_block` keeps EXACTLY the strictness it had:
//! not one branch of it is relaxed, and it gained no parameter. A foreign
//! asset-chain being acceptable is not, and cannot become, a way for a foreign
//! *block* to enter the spine.
//!
//! # Non-destructive, and non-shadowing
//!
//! Adoption adds; it never clears. Device genesis and every previously-adopted
//! foreign chain survive an accept, and an asset whose title this container
//! already holds on its own spine is REFUSED
//! ([`ForeignChainReject::AlreadyOnSpine`]) rather than shadowed: the spine is
//! the authority for the assets it holds, and an import may not offer a second
//! opinion about one of them.
//!
//! The INVERSE ordering — adopt a foreign chain for asset X, then receive a
//! spine block carrying X — is deliberately not blocked. `insert_received_block`
//! does not consult this store and gains no reason to: letting an off-spine
//! import veto a spine block would hand a remote sender a censorship primitive
//! over our own chain. The spine simply wins, as it does on every read path.
//! What that ordering must NOT leave behind is a public accessor still serving
//! the superseded history as if it were live, so
//! [`foreign_asset_lineage`](NodeBlockchain::foreign_asset_lineage) and
//! [`foreign_asset_head`](NodeBlockchain::foreign_asset_head) return `None`
//! once the spine holds the asset, and
//! [`foreign_chain_is_shadowed`](NodeBlockchain::foreign_chain_is_shadowed) is
//! the explicit signal.

use super::block::BlockAssetEntry;
use super::chain::NodeBlockchain;
use super::lineage::AssetLineage;
use super::mutations::signer_binds_to_author;

mod presented;
mod store;

pub use presented::{ForeignAssetChain, ForeignChainReceipt, ForeignChainReject, StoreBound};
pub use store::{
    chain_footprint_bytes, entry_footprint_bytes, ForeignChainStore, MAX_FOREIGN_CHAINS,
    MAX_FOREIGN_CHAIN_ENTRIES, MAX_FOREIGN_STORE_BYTES,
};

/// Verify every signer along a presented chain.
///
/// Split out of [`NodeBlockchain::accept_foreign_asset_chain`] so the accept
/// body stays a flat decision table. Uses H3's `signer_binds_to_author` — the
/// SAME function the spine accept mode uses, so "who may author an entry" has
/// exactly one definition in this crate.
fn verify_every_signer(entries: &[BlockAssetEntry]) -> Result<(), ForeignChainReject> {
    for (position, entry) in entries.iter().enumerate() {
        if entry.signed_proof.is_none() {
            return Err(ForeignChainReject::Unsigned { position });
        }
        let signer_pubkey = entry
            .verify_signed_proof()
            .map_err(|detail| ForeignChainReject::BadSignature { position, detail })?;
        if !signer_binds_to_author(&signer_pubkey, entry) {
            return Err(ForeignChainReject::SignerNotAuthor { position });
        }
    }
    Ok(())
}

impl NodeBlockchain {
    /// S3.4 — accept a FOREIGN asset's verified sub-chain, off-spine.
    ///
    /// The third accept mode. See the [module docs](self) for why it exists and
    /// how it stays structurally separate from the node-spine accept mode.
    ///
    /// Order of judgement (all fail-closed, cheapest first — and the ordering
    /// is load-bearing, not decorative):
    /// 1. non-empty, and within [`MAX_FOREIGN_CHAIN_ENTRIES`] — bounds the
    ///    signature work an untrusted caller can provoke;
    /// 2. the asset is not one this container's own spine holds or has held
    ///    ([`ForeignChainReject::AlreadyOnSpine`]) — a cheap pre-filter,
    ///    re-asked authoritatively at step 6;
    /// 3. **capacity** ([`ForeignChainStore::admission_check`]) — asked HERE,
    ///    before any FALCON-1024 work, so that at steady-state capacity a
    ///    refusal costs O(1) rather than up to
    ///    [`MAX_FOREIGN_CHAIN_ENTRIES`] verifications. Skipped when this asset
    ///    is already held off-spine, because extending a held chain is not
    ///    growth and must not be refused by a bound on growth;
    /// 4. **internal lineage** — [`AssetLineage::verify`], i.e. every entry is
    ///    for this asset, `proof_hash == BLAKE3(serialize(state_proof))`, the
    ///    proof is content-bound, the root IS an asset-genesis, and every later
    ///    entry names its predecessor's `lineage_id` with `asset_seq` advancing
    ///    by exactly one. Node-block indices play no part;
    /// 5. **every signer** — FALCON-1024 envelope present, valid, and bound to
    ///    the identity the entry claims as author;
    /// 6. adoption — extension-only, and capacity re-judged authoritatively
    ///    under the write lock, with the spine-ownership question re-asked
    ///    while the `asset_index` read lock is HELD (see below).
    ///
    /// # Why steps 2 and 3 are optimizations and not gates
    ///
    /// No lock spans steps 2/3 and step 6, so both answers can go stale. Each
    /// is therefore re-asked at step 6 against the state that actually admits
    /// the entries. The capacity question is re-asked by calling the SAME
    /// [`ForeignChainStore::admission_check`] with the same inputs (a new chain
    /// charges its whole self in both places), so the early and authoritative
    /// refusals cannot name different bounds or different reasons.
    ///
    /// # The step-6 lock, and the TOCTOU it closes
    ///
    /// Step 2 reads `asset_index` and releases it; a spine block carrying this
    /// asset could land before step 6. Step 6 therefore takes `asset_index`
    /// (read) and holds it across the `foreign_chains` write — the documented
    /// order (`… → asset_index → mirror_attestations → foreign_chains`), so no
    /// inversion — which excludes `insert_block`'s `asset_index` WRITE for the
    /// duration. The spine cannot acquire this asset in the window between the
    /// authoritative check and the adoption.
    ///
    /// On success the chain is queryable through
    /// [`foreign_asset_lineage`](Self::foreign_asset_lineage) and
    /// [`asset_lineage_any`](Self::asset_lineage_any). Node-spine height, head,
    /// block count and the S3.1 index are untouched.
    pub async fn accept_foreign_asset_chain(
        &self,
        chain: ForeignAssetChain,
    ) -> Result<ForeignChainReceipt, ForeignChainReject> {
        let asset_hash = chain.asset_hash;
        let outcome = self.accept_foreign_asset_chain_inner(chain).await;

        // ONE refusal log site, so no rejection path can be added without one.
        // `warn` and not `debug`: `main.rs` caps the default subscriber at INFO,
        // and a refusal nobody sees is a refusal nobody can diagnose.
        if let Err(reject) = &outcome {
            tracing::warn!(
                asset = %&hex::encode(asset_hash)[..16],
                reject = %reject,
                "S3.4: refused a foreign asset-chain (node spine untouched)"
            );
        }
        outcome
    }

    /// Body of [`Self::accept_foreign_asset_chain`]; see its docs for the
    /// order of judgement. Split out only so refusals have a single log site.
    async fn accept_foreign_asset_chain_inner(
        &self,
        chain: ForeignAssetChain,
    ) -> Result<ForeignChainReceipt, ForeignChainReject> {
        // (1) shape.
        if chain.is_empty() {
            return Err(ForeignChainReject::Empty);
        }
        if chain.len() > MAX_FOREIGN_CHAIN_ENTRIES {
            return Err(ForeignChainReject::TooLong {
                presented: chain.len(),
                limit: MAX_FOREIGN_CHAIN_ENTRIES,
            });
        }

        // (2) The spine owns the assets it holds. F1's tombstone is consulted,
        // so an asset whose entries were pruned still counts as ours.
        if self.has_ever_seen_asset(&chain.asset_hash).await {
            return Err(ForeignChainReject::AlreadyOnSpine);
        }

        // (3) A3 — capacity BEFORE signature work.
        {
            let store = self.foreign_chains.read().await;
            if !store.contains(&chain.asset_hash) {
                store.admission_check(true, chain_footprint_bytes(&chain.entries))?;
            }
        }

        // (4) internal lineage, (5) every signer — the expensive half.
        chain
            .as_lineage()
            .verify()
            .map_err(ForeignChainReject::LineageBroken)?;

        verify_every_signer(&chain.entries)?;

        let head = chain.head().ok_or(ForeignChainReject::Empty)?.clone();

        // (6) authoritative. `asset_index` (read) is held across the
        // `foreign_chains` write, in the documented order.
        let (entries, added) = {
            let asset_index = self.asset_index.read().await;
            if asset_index.has_ever_seen_asset(&chain.asset_hash) {
                return Err(ForeignChainReject::AlreadyOnSpine);
            }
            let mut store = self.foreign_chains.write().await;
            let added = store.adopt(chain.asset_hash, chain.entries)?;
            (store.entries(&chain.asset_hash).len(), added)
        };

        tracing::info!(
            asset = %&hex::encode(chain.asset_hash)[..16],
            entries,
            added,
            "S3.4: adopted a foreign asset-chain off-spine (node spine untouched)"
        );

        Ok(ForeignChainReceipt {
            asset_hash: chain.asset_hash,
            entries,
            added,
            head_lineage_id: head.lineage_id(),
            head_seq: head.asset_seq(),
        })
    }

    /// S3.4 (A4) — whether an adopted foreign chain for `asset_hash` is
    /// SHADOWED by this container's own spine.
    ///
    /// True when the store holds a chain for an asset the spine has since
    /// acquired. The accept path refuses that ordering
    /// ([`ForeignChainReject::AlreadyOnSpine`]) and holds `asset_index` across
    /// the adoption so it cannot race in — but the INVERSE ordering is real and
    /// deliberately unblocked: `insert_received_block` does not consult this
    /// store, so a spine block carrying an already-adopted asset is accepted on
    /// its own merits.
    ///
    /// That is the correct outcome — the spine is authoritative for the assets
    /// it holds, and making a foreign import able to veto a spine block would
    /// hand a remote sender a censorship primitive. What is NOT correct is the
    /// off-spine copy continuing to answer as if it were still a live title.
    /// So it stops answering: see [`Self::foreign_asset_lineage`].
    pub async fn foreign_chain_is_shadowed(&self, asset_hash: &[u8; 32]) -> bool {
        self.foreign_chains.read().await.contains(asset_hash)
            && self.has_ever_seen_asset(asset_hash).await
    }

    /// S3.4 — the adopted foreign history for `asset_hash`, if any AND if the
    /// spine has not since taken the asset over.
    ///
    /// Returns an [`AssetLineage`], the same shape
    /// [`asset_lineage`](Self::asset_lineage) returns for a spine-held asset,
    /// so a caller verifies an imported title exactly as it verifies a local
    /// one.
    ///
    /// A4: returns `None` once the asset is on the spine, even though the
    /// entries are still held. A shadowed import is not a second opinion a
    /// caller should have to know to discard — a public accessor that keeps
    /// serving a competing history with no signal is exactly the wart this
    /// closes. [`Self::foreign_chain_is_shadowed`] is the explicit signal, and
    /// [`Self::forget_foreign_asset_chain`] is how the bytes are released.
    pub async fn foreign_asset_lineage(&self, asset_hash: &[u8; 32]) -> Option<AssetLineage> {
        if self.has_ever_seen_asset(asset_hash).await {
            return None;
        }
        let store = self.foreign_chains.read().await;
        if !store.contains(asset_hash) {
            return None;
        }
        Some(AssetLineage {
            asset_hash: *asset_hash,
            entries: store.entries(asset_hash).to_vec(),
        })
    }

    /// S3.4 — the adopted head entry for `asset_hash`, if any and unshadowed.
    ///
    /// Shadow-aware for the same reason [`Self::foreign_asset_lineage`] is: a
    /// head is the sharpest form of "this is the current title", and it must
    /// not be served for an asset the spine now holds.
    pub async fn foreign_asset_head(&self, asset_hash: &[u8; 32]) -> Option<BlockAssetEntry> {
        if self.has_ever_seen_asset(asset_hash).await {
            return None;
        }
        self.foreign_chains
            .read()
            .await
            .entries(asset_hash)
            .last()
            .cloned()
    }

    /// S3.4 — whether the off-spine STORE holds a chain for `asset_hash`.
    ///
    /// Raw store membership, NOT shadow-aware: this is the question the byte
    /// budget and [`Self::forget_foreign_asset_chain`] are asked in terms of,
    /// so it must stay true for a shadowed chain whose bytes are still held.
    /// For "is there a foreign title to read?", use
    /// [`Self::foreign_asset_lineage`].
    pub async fn has_foreign_asset_chain(&self, asset_hash: &[u8; 32]) -> bool {
        self.foreign_chains.read().await.contains(asset_hash)
    }

    /// S3.4 — bytes of foreign asset-chain material held off-spine, against
    /// [`MAX_FOREIGN_STORE_BYTES`].
    pub async fn foreign_chain_bytes(&self) -> usize {
        self.foreign_chains.read().await.bytes_held()
    }

    /// S3.4 — number of distinct foreign asset-chains adopted.
    pub async fn foreign_chain_count(&self) -> usize {
        self.foreign_chains.read().await.len()
    }

    /// S3.4 — does this container hold `asset_hash` at all, on its own spine or
    /// as an adopted foreign chain?
    ///
    /// This is the "do we know this asset?" question the attestation wire
    /// surface asks before it will cache a third party's statement about it.
    pub async fn holds_asset(&self, asset_hash: &[u8; 32]) -> bool {
        self.has_ever_seen_asset(asset_hash).await
            || self.has_foreign_asset_chain(asset_hash).await
    }

    /// S3.4 — `asset_hash`'s history from whichever side of the container holds
    /// it: the SPINE first (it is authoritative for its own assets), then the
    /// adopted foreign chains.
    ///
    /// [`asset_lineage`](Self::asset_lineage) is deliberately left alone —
    /// it remains the spine's answer, so nothing that reasons about local
    /// title starts silently reading imported history.
    pub async fn asset_lineage_any(&self, asset_hash: &[u8; 32]) -> AssetLineage {
        let spine = self.asset_lineage(asset_hash).await;
        if !spine.is_empty() {
            return spine;
        }
        self.foreign_asset_lineage(asset_hash)
            .await
            .unwrap_or(AssetLineage {
                asset_hash: *asset_hash,
                entries: Vec::new(),
            })
    }

    /// S3.4 — release the adopted chain for `asset_hash`; returns how many
    /// entries were dropped.
    ///
    /// The explicit counterpart to reject-at-capacity: space in the off-spine
    /// store is reclaimed by a local decision, never by an attacker's traffic.
    ///
    /// # F1 — the attestation pool is released with it
    ///
    /// Forgetting a foreign chain is the one place an asset genuinely stops
    /// being held (the spine never forgets: F1's tombstone keeps
    /// `has_ever_seen_asset` true forever). Once it is gone,
    /// [`holds_asset`](Self::holds_asset) is false for that asset, so
    /// [`accept_wire_attestation`](Self::accept_wire_attestation) would refuse a
    /// NEW attestation about it — and keeping the OLD ones would charge
    /// [`MAX_ATTESTATION_POOL_BYTES`](super::attestations::MAX_ATTESTATION_POOL_BYTES)
    /// for statements about an asset this container no longer has. Attestations
    /// for an asset the SPINE holds are never touched: the spine is
    /// authoritative for its own assets and this call says nothing about them.
    ///
    /// This is not eviction. It is driven by a local decision about local state,
    /// so no remote input can steer it.
    ///
    /// Locks are taken in the documented order
    /// (`… → mirror_attestations → foreign_chains`) and held together, so the
    /// asset cannot be re-adopted between the two releases.
    pub async fn forget_foreign_asset_chain(&self, asset_hash: &[u8; 32]) -> usize {
        let spine_holds = self.has_ever_seen_asset(asset_hash).await;
        let mut attestations = self.mirror_attestations.write().await;
        let dropped = self.foreign_chains.write().await.forget(asset_hash);
        if !spine_holds {
            let released = attestations.clear_asset(asset_hash);
            if released > 0 {
                tracing::info!(
                    asset = %&hex::encode(asset_hash)[..16],
                    released,
                    "S3.4/F1: released pooled mirror attestations for a no-longer-held asset"
                );
            }
        }
        dropped
    }

    /// S3.4/F4 — every foreign asset-chain this container holds off-spine.
    ///
    /// [`ForeignChainStore::asset_hashes`] is `pub`, but the store itself is
    /// `pub(crate)` and no accessor returned the adopted set — so
    /// [`forget_foreign_asset_chain`](Self::forget_foreign_asset_chain) only
    /// ever worked for a hash the caller already remembered. A store full of
    /// chains nobody remembers charges the byte budget and refuses every new
    /// admission, with no enumerable way to reclaim. This is that enumeration.
    pub async fn foreign_asset_hashes(&self) -> Vec<[u8; 32]> {
        self.foreign_chains.read().await.asset_hashes().copied().collect()
    }

    /// S3.4/F4 — the adopted chains that are SHADOWED: the spine has since taken
    /// the asset over, so [`foreign_asset_lineage`](Self::foreign_asset_lineage)
    /// already returns `None` for them while their bytes are still charged.
    ///
    /// These are the entries a reclaim should act on first — they answer no
    /// query and can never answer one again, because the spine never forgets an
    /// asset it has held.
    pub async fn shadowed_foreign_asset_chains(&self) -> Vec<[u8; 32]> {
        let mut shadowed = Vec::new();
        for asset_hash in self.foreign_asset_hashes().await {
            if self.has_ever_seen_asset(&asset_hash).await {
                shadowed.push(asset_hash);
            }
        }
        shadowed
    }

    /// S3.4/F4 — release every SHADOWED foreign chain; returns how many chains
    /// were dropped.
    ///
    /// A LOCAL decision, never automatic: nothing on the wire path calls this,
    /// and it acts only on chains the spine has already superseded, so it cannot
    /// delete a history that is still anybody's answer. That is what keeps it a
    /// reclaim rather than the attacker-steerable eviction
    /// [`ForeignChainStore`] refuses.
    pub async fn forget_shadowed_foreign_asset_chains(&self) -> usize {
        let mut dropped = 0usize;
        for asset_hash in self.shadowed_foreign_asset_chains().await {
            if self.forget_foreign_asset_chain(&asset_hash).await > 0 {
                dropped = dropped.saturating_add(1);
            }
        }
        if dropped > 0 {
            tracing::info!(
                chains = dropped,
                "S3.4/F4: released shadowed foreign asset-chains (spine holds those assets)"
            );
        }
        dropped
    }
}
