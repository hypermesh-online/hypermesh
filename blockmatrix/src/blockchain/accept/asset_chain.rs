// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Read/query/reclaim accessors for received asset-chains held aside from the
//! node's own block chain. The accept path that fills the store lives in the
//! parent [`super`] module.

use super::super::block::BlockAssetEntry;
use super::super::chain::NodeBlockchain;
use super::super::lineage::AssetLineage;

impl NodeBlockchain {
    /// Whether an adopted received chain for `asset_hash` is SHADOWED by this
    /// container's own block chain.
    ///
    /// True when the store holds a chain for an asset the block chain has since
    /// acquired. The accept path refuses that ordering
    /// ([`AcceptReject::AlreadyOnSpine`](super::AcceptReject::AlreadyOnSpine))
    /// and holds `asset_index` across the adoption so it cannot race in — but
    /// the INVERSE ordering is real and deliberately unblocked:
    /// `insert_received_block` does not consult this store, so a block carrying
    /// an already-adopted asset is accepted on its own merits.
    ///
    /// That is the correct outcome — the block chain is authoritative for the
    /// assets it holds, and making a received import able to veto a block would
    /// hand a remote sender a censorship primitive. What is NOT correct is the
    /// held copy continuing to answer as if it were still a live title. So it
    /// stops answering: see [`Self::received_asset_lineage`].
    pub async fn received_chain_is_shadowed(&self, asset_hash: &[u8; 32]) -> bool {
        self.received_chains.read().await.contains(asset_hash)
            && self.has_ever_seen_asset(asset_hash).await
    }

    /// The adopted received history for `asset_hash`, if any AND if the block
    /// chain has not since taken the asset over.
    ///
    /// Returns an [`AssetLineage`], the same shape
    /// [`asset_lineage`](Self::asset_lineage) returns for a locally-titled
    /// asset, so a caller verifies a received title exactly as it verifies a
    /// local one.
    ///
    /// A4: returns `None` once the asset is on the block chain, even though the
    /// entries are still held. A shadowed import is not a second opinion a
    /// caller should have to know to discard — a public accessor that keeps
    /// serving a competing history with no signal is exactly the wart this
    /// closes. [`Self::received_chain_is_shadowed`] is the explicit signal, and
    /// [`Self::forget_received_asset_chain`] is how the bytes are released.
    pub async fn received_asset_lineage(&self, asset_hash: &[u8; 32]) -> Option<AssetLineage> {
        if self.has_ever_seen_asset(asset_hash).await {
            return None;
        }
        let store = self.received_chains.read().await;
        if !store.contains(asset_hash) {
            return None;
        }
        Some(AssetLineage {
            asset_hash: *asset_hash,
            entries: store.entries(asset_hash).to_vec(),
        })
    }

    /// The adopted head entry for `asset_hash`, if any and unshadowed.
    ///
    /// Shadow-aware for the same reason [`Self::received_asset_lineage`] is: a
    /// head is the sharpest form of "this is the current title", and it must
    /// not be served for an asset the block chain now holds.
    pub async fn received_asset_head(&self, asset_hash: &[u8; 32]) -> Option<BlockAssetEntry> {
        if self.has_ever_seen_asset(asset_hash).await {
            return None;
        }
        self.received_chains
            .read()
            .await
            .entries(asset_hash)
            .last()
            .cloned()
    }

    /// Whether the store holds a chain for `asset_hash`.
    ///
    /// Raw store membership, NOT shadow-aware: this is the question the byte
    /// budget and [`Self::forget_received_asset_chain`] are asked in terms of,
    /// so it must stay true for a shadowed chain whose bytes are still held.
    /// For "is there a received title to read?", use
    /// [`Self::received_asset_lineage`].
    pub async fn has_received_asset_chain(&self, asset_hash: &[u8; 32]) -> bool {
        self.received_chains.read().await.contains(asset_hash)
    }

    /// Bytes of received asset-chain material held, against
    /// [`MAX_RECEIVED_STORE_BYTES`](super::MAX_RECEIVED_STORE_BYTES).
    pub async fn received_chain_bytes(&self) -> usize {
        self.received_chains.read().await.bytes_held()
    }

    /// Number of distinct received asset-chains adopted.
    pub async fn received_chain_count(&self) -> usize {
        self.received_chains.read().await.len()
    }

    /// Does this container hold `asset_hash` at all, on its own block chain or
    /// as an adopted received chain?
    ///
    /// This is the "do we know this asset?" question the attestation wire
    /// surface asks before it will cache a third party's statement about it.
    pub async fn holds_asset(&self, asset_hash: &[u8; 32]) -> bool {
        self.has_ever_seen_asset(asset_hash).await
            || self.has_received_asset_chain(asset_hash).await
    }

    /// Release the adopted chain for `asset_hash`; returns how many entries were
    /// dropped.
    ///
    /// The explicit counterpart to reject-at-capacity: space in the store is
    /// reclaimed by a local decision, never by an attacker's traffic.
    ///
    /// # F1 — the attestation pool is released with it
    ///
    /// Forgetting a received chain is the one place an asset genuinely stops
    /// being held (the block chain never forgets: F1's tombstone keeps
    /// `has_ever_seen_asset` true forever). Once it is gone,
    /// [`holds_asset`](Self::holds_asset) is false for that asset, so
    /// [`accept_wire_attestation`](Self::accept_wire_attestation) would refuse a
    /// NEW attestation about it — and keeping the OLD ones would charge
    /// [`MAX_ATTESTATION_POOL_BYTES`](super::super::attestations::MAX_ATTESTATION_POOL_BYTES)
    /// for statements about an asset this container no longer has. Attestations
    /// for an asset the block chain holds are never touched: the block chain is
    /// authoritative for its own assets and this call says nothing about them.
    ///
    /// This is not eviction. It is driven by a local decision about local state,
    /// so no remote input can steer it.
    ///
    /// Locks are taken in the documented order
    /// (`… → mirror_attestations → received_chains`) and held together, so the
    /// asset cannot be re-adopted between the two releases.
    pub async fn forget_received_asset_chain(&self, asset_hash: &[u8; 32]) -> usize {
        let chain_holds = self.has_ever_seen_asset(asset_hash).await;
        let mut attestations = self.mirror_attestations.write().await;
        let dropped = self.received_chains.write().await.forget(asset_hash);
        if !chain_holds {
            let released = attestations.clear_asset(asset_hash);
            if released > 0 {
                tracing::info!(
                    asset = %&hex::encode(asset_hash)[..16],
                    released,
                    "F1: released pooled mirror attestations for a no-longer-held asset"
                );
            }
        }
        dropped
    }

    /// F4 — every received asset-chain this container holds.
    ///
    /// [`ReceivedAssetStore::asset_hashes`](super::ReceivedAssetStore::asset_hashes)
    /// is `pub`, but the store itself is `pub(crate)` and no accessor returned
    /// the adopted set — so
    /// [`forget_received_asset_chain`](Self::forget_received_asset_chain) only
    /// ever worked for a hash the caller already remembered. A store full of
    /// chains nobody remembers charges the byte budget and refuses every new
    /// admission, with no enumerable way to reclaim. This is that enumeration.
    pub async fn received_asset_hashes(&self) -> Vec<[u8; 32]> {
        self.received_chains
            .read()
            .await
            .asset_hashes()
            .copied()
            .collect()
    }

    /// F4 — the adopted chains that are SHADOWED: the block chain has since
    /// taken the asset over, so
    /// [`received_asset_lineage`](Self::received_asset_lineage) already returns
    /// `None` for them while their bytes are still charged.
    ///
    /// These are the entries a reclaim should act on first — they answer no
    /// query and can never answer one again, because the block chain never
    /// forgets an asset it has held.
    pub async fn shadowed_received_asset_chains(&self) -> Vec<[u8; 32]> {
        let mut shadowed = Vec::new();
        for asset_hash in self.received_asset_hashes().await {
            if self.has_ever_seen_asset(&asset_hash).await {
                shadowed.push(asset_hash);
            }
        }
        shadowed
    }

    /// F4 — release every SHADOWED received chain; returns how many chains were
    /// dropped.
    ///
    /// A LOCAL decision, never automatic: nothing on the wire path calls this,
    /// and it acts only on chains the block chain has already superseded, so it
    /// cannot delete a history that is still anybody's answer. That is what
    /// keeps it a reclaim rather than the attacker-steerable eviction
    /// [`ReceivedAssetStore`](super::ReceivedAssetStore) refuses.
    pub async fn forget_shadowed_received_asset_chains(&self) -> usize {
        let mut dropped = 0usize;
        for asset_hash in self.shadowed_received_asset_chains().await {
            if self.forget_received_asset_chain(&asset_hash).await > 0 {
                dropped = dropped.saturating_add(1);
            }
        }
        if dropped > 0 {
            tracing::info!(
                chains = dropped,
                "F4: released shadowed received asset-chains (the block chain holds those assets)"
            );
        }
        dropped
    }
}
