// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! S3.2 — the asset's provenance chain, walked and verified.
//!
//! An asset is its own chain (the car-title model): it is created once, then
//! extended by whoever holds it, and it carries that history with it when it
//! moves. S3.1 gave us the per-asset INDEX (which blocks hold entries for this
//! asset); S3.2 writes the LINEAGE POINTERS into the proof body
//! ([`StateProof::prev_asset_entry`] / `asset_seq`) so the history is
//! authenticated rather than merely collated.
//!
//! This module reads that lineage back out:
//!
//! - [`NodeBlockchain::asset_lineage`] materializes an asset's entries in chain
//!   order.
//! - [`AssetLineage::verify`] checks the chain is UNBROKEN — every entry names
//!   its predecessor's `lineage_id`, sequence numbers advance by exactly one,
//!   each entry's `proof_hash` really is `BLAKE3(serialize(state_proof))`, and
//!   each proof is bound to the asset it claims.
//!
//! That verification is deliberately self-contained: it takes only the entries,
//! not the chain, because S3.5's transfer hands exactly this list to a
//! recipient who has never seen our container.
//!
//! # D1 — this is the authoritative per-asset object
//!
//! Under the unification inversion, [`AssetLineage`] is THE authority for every
//! per-asset question — "what is this asset's head / predecessor / history?".
//! It answers from the entries themselves, addressed by their `lineage_id`
//! (`= hex(proof_hash)`, spine-offset-free): a predecessor is named by its
//! `lineage_id`, never by a block index. The linear spine
//! (`NodeBlockchain.blocks`, keyed by block index) is DEMOTED to a
//! batching/durability log — it stores the entries and preserves on-disk
//! back-compat and spine-sync, but it holds no per-asset authority and no
//! asset-authority question may consult a block index as identity. Block index
//! survives only as a storage-fetch detail (which block to read an entry out
//! of), routed through the derived [`AssetChainIndex`] cache.
//!
//! [`AssetChainIndex`]: super::asset_index::AssetChainIndex

use super::block::BlockAssetEntry;
use super::chain::NodeBlockchain;

/// One asset's ordered provenance chain, as this container can produce it.
#[derive(Clone, Debug, PartialEq)]
pub struct AssetLineage {
    /// The asset this lineage belongs to (`hA`).
    pub asset_hash: [u8; 32],
    /// The asset's entries, in chain order (oldest first).
    pub entries: Vec<BlockAssetEntry>,
}

/// Why an asset lineage failed verification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineageBreak {
    /// An entry belongs to a different asset than the lineage claims.
    WrongAsset {
        /// Position in the lineage.
        position: usize,
    },
    /// An entry's `proof_hash` is not `BLAKE3(serialize(state_proof))`.
    ProofHashMismatch {
        /// Position in the lineage.
        position: usize,
    },
    /// An entry's proof is not bound to its `asset_hash` (P1).
    NotContentBound {
        /// Position in the lineage.
        position: usize,
    },
    /// The first entry is not an asset genesis (`prev = None, seq = 0`).
    RootIsNotAssetGenesis {
        /// What the root claimed as its predecessor.
        claimed_prev: Option<String>,
        /// What the root claimed as its sequence number.
        claimed_seq: u64,
    },
    /// An entry does not name its predecessor's `lineage_id`.
    PrevPointerMismatch {
        /// Position in the lineage.
        position: usize,
        /// What this entry claimed.
        claimed: Option<String>,
        /// What the preceding entry's identity actually is.
        expected: String,
    },
    /// The predecessor's `asset_seq` is `u64::MAX`: no successor sequence
    /// exists. Fail closed rather than wrapping to 0 and re-rooting the asset.
    SequenceOverflow {
        /// Position in the lineage.
        position: usize,
    },
    /// Sequence numbers do not advance by exactly one.
    SequenceGap {
        /// Position in the lineage.
        position: usize,
        /// What this entry claimed.
        claimed: u64,
        /// What it had to be.
        expected: u64,
    },
}

impl std::fmt::Display for LineageBreak {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongAsset { position } => {
                write!(f, "entry {position} belongs to a different asset")
            }
            Self::ProofHashMismatch { position } => write!(
                f,
                "entry {position} proof_hash != BLAKE3(serialize(state_proof))"
            ),
            Self::NotContentBound { position } => {
                write!(f, "entry {position} proof is not bound to its asset_hash")
            }
            Self::RootIsNotAssetGenesis { claimed_prev, claimed_seq } => write!(
                f,
                "lineage root is not an asset genesis (prev={claimed_prev:?}, seq={claimed_seq})"
            ),
            Self::PrevPointerMismatch { position, claimed, expected } => write!(
                f,
                "entry {position} names predecessor {claimed:?}, expected {expected}"
            ),
            Self::SequenceOverflow { position } => write!(
                f,
                "entry {position} would need asset_seq u64::MAX + 1 — sequence overflow"
            ),
            Self::SequenceGap { position, claimed, expected } => write!(
                f,
                "entry {position} has asset_seq {claimed}, expected {expected}"
            ),
        }
    }
}

impl AssetLineage {
    /// Number of entries in the lineage.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether this container holds no entries for the asset.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The asset's current head entry (most recent), if any.
    pub fn head(&self) -> Option<&BlockAssetEntry> {
        self.entries.last()
    }

    /// The asset's genesis entry (oldest held), if any.
    pub fn root(&self) -> Option<&BlockAssetEntry> {
        self.entries.first()
    }

    /// Sequence numbers in order — the cheap shape check.
    pub fn sequence(&self) -> Vec<u64> {
        self.entries.iter().map(|e| e.asset_seq()).collect()
    }

    /// Verify the lineage is a complete, unbroken chain for one asset.
    ///
    /// Checks, in order, for every entry:
    /// 1. it is an entry for THIS asset,
    /// 2. `proof_hash == BLAKE3(serialize(state_proof))` (integrity),
    /// 3. the proof is bound to the asset (`content_binding_ok`, P1),
    /// 4. the root is an asset genesis, and every later entry names its
    ///    predecessor's `lineage_id` with `asset_seq` advancing by one.
    ///
    /// An EMPTY lineage verifies trivially (there is nothing to contradict);
    /// callers that require the asset to exist check [`is_empty`](Self::is_empty).
    ///
    /// This does NOT verify signatures — the FALCON envelope
    /// (`BlockAssetEntry::verify_signed_proof`) is a separate, per-entry check
    /// the caller runs when it needs to know WHO extended the chain.
    pub fn verify(&self) -> Result<(), LineageBreak> {
        for (position, entry) in self.entries.iter().enumerate() {
            if entry.asset_hash != self.asset_hash {
                return Err(LineageBreak::WrongAsset { position });
            }

            let proof_bytes = serde_json::to_vec(&entry.state_proof).unwrap_or_default();
            if *blake3::hash(&proof_bytes).as_bytes() != entry.proof_hash {
                return Err(LineageBreak::ProofHashMismatch { position });
            }

            if !entry.content_binding_ok() {
                return Err(LineageBreak::NotContentBound { position });
            }

            match position.checked_sub(1).and_then(|p| self.entries.get(p)) {
                None => {
                    if !entry.is_asset_genesis() {
                        return Err(LineageBreak::RootIsNotAssetGenesis {
                            claimed_prev: entry.prev_asset_entry().map(str::to_string),
                            claimed_seq: entry.asset_seq(),
                        });
                    }
                }
                Some(predecessor) => {
                    let expected = predecessor.lineage_id();
                    if entry.prev_asset_entry() != Some(expected.as_str()) {
                        return Err(LineageBreak::PrevPointerMismatch {
                            position,
                            claimed: entry.prev_asset_entry().map(str::to_string),
                            expected,
                        });
                    }
                    // F5: fail closed on overflow — a predecessor at
                    // `u64::MAX` has NO valid successor; wrapping to 0 would
                    // present a re-rooted chain as an unbroken one.
                    let Some(expected_seq) = predecessor.asset_seq().checked_add(1) else {
                        return Err(LineageBreak::SequenceOverflow { position });
                    };
                    if entry.asset_seq() != expected_seq {
                        return Err(LineageBreak::SequenceGap {
                            position,
                            claimed: entry.asset_seq(),
                            expected: expected_seq,
                        });
                    }
                }
            }
        }

        Ok(())
    }
}

impl NodeBlockchain {
    /// S3.2 — an asset's provenance chain from this container, in chain order,
    /// from whichever side of the container holds it.
    ///
    /// The block chain is consulted FIRST — it is authoritative for the assets
    /// it holds — and its answer, when non-empty, is returned as-is. Only when
    /// the block chain has no history for the asset does this fall through to a
    /// RECEIVED chain adopted by
    /// [`accept_asset_chain`](Self::accept_asset_chain). That fall-through is
    /// safe precisely because the accept path refuses any asset the block chain
    /// has ever held, and [`received_asset_lineage`](Self::received_asset_lineage)
    /// returns `None` the moment the block chain takes an asset over — so a
    /// received chain can only ever surface here for an asset the block chain
    /// genuinely does not hold, never as a second opinion about a local title.
    ///
    /// Backed by the S3.1 index for the block-chain side, so that half is
    /// O(entries for the asset), not a chain scan. Entries whose block has been
    /// pruned to a header are absent (they are also absent from the index) — a
    /// lineage with a hole fails [`AssetLineage::verify`] rather than silently
    /// pretending to be whole.
    ///
    /// This is the object S3.5's transfer hands to a recipient: the asset's
    /// full, self-verifying history.
    pub async fn asset_lineage(&self, asset_hash: &[u8; 32]) -> AssetLineage {
        let spine = self.asset_history_entries(asset_hash).await;
        if !spine.is_empty() {
            return AssetLineage {
                asset_hash: *asset_hash,
                entries: spine,
            };
        }
        self.received_asset_lineage(asset_hash)
            .await
            .unwrap_or(AssetLineage {
                asset_hash: *asset_hash,
                entries: Vec::new(),
            })
    }

    /// [`asset_lineage`](Self::asset_lineage) plus its verification verdict.
    pub async fn verify_asset_lineage(
        &self,
        asset_hash: &[u8; 32],
    ) -> Result<AssetLineage, LineageBreak> {
        let lineage = self.asset_lineage(asset_hash).await;
        lineage.verify()?;
        Ok(lineage)
    }
}
