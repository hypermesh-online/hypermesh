// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! S3.4 — the presented/wire surface: a foreign asset's chain as offered for
//! adoption, plus the refusal vocabulary and the accept receipt. The accept
//! mode that consumes these lives in the parent [`super`] module.

use crate::blockchain::block::BlockAssetEntry;
use crate::blockchain::lineage::{AssetLineage, LineageBreak};

/// A foreign asset's provenance chain as presented for adoption: the asset it
/// claims to be, and its entries in chain order (oldest first).
///
/// Deliberately NOT a `Block` and deliberately not convertible into one. This
/// is the whole structural separation from the node-spine accept mode.
#[derive(Clone, Debug, PartialEq)]
pub struct ForeignAssetChain {
    /// `hA` — the asset this chain claims to be the history of.
    pub asset_hash: [u8; 32],
    /// The asset's entries, oldest first, starting at its asset-genesis.
    pub entries: Vec<BlockAssetEntry>,
}

impl ForeignAssetChain {
    /// Present `entries` as `asset_hash`'s chain.
    pub fn new(asset_hash: [u8; 32], entries: Vec<BlockAssetEntry>) -> Self {
        Self { asset_hash, entries }
    }

    /// Number of entries presented.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the presentation carries no entries at all.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The head (most recent) entry, if any.
    pub fn head(&self) -> Option<&BlockAssetEntry> {
        self.entries.last()
    }

    /// Read this presentation as an [`AssetLineage`] so S3.2's verifier — the
    /// ONE definition of "is this asset chain unbroken?" — judges it.
    pub fn as_lineage(&self) -> AssetLineage {
        AssetLineage {
            asset_hash: self.asset_hash,
            entries: self.entries.clone(),
        }
    }
}

/// Why a foreign asset-chain was refused.
///
/// Every variant is a REFUSAL of the import only. None of them can affect the
/// node spine, because nothing in this module writes to it.
#[derive(Clone, Debug, PartialEq)]
pub enum ForeignChainReject {
    /// No entries were presented — there is no history to verify.
    Empty,
    /// More entries than [`MAX_FOREIGN_CHAIN_ENTRIES`].
    TooLong {
        /// Entries presented.
        presented: usize,
        /// The cap.
        limit: usize,
    },
    /// The chain's internal lineage is broken (S3.2's verdict, verbatim).
    LineageBroken(LineageBreak),
    /// An entry carries no FALCON `signed_proof` envelope. Unlike the spine
    /// accept mode there is NO legacy-migration tolerance here: a foreign
    /// import has no pre-H3 corpus to rescue, so an unsigned entry is an
    /// unattributable claim.
    Unsigned {
        /// Position in the chain.
        position: usize,
    },
    /// An entry's FALCON envelope failed to verify.
    BadSignature {
        /// Position in the chain.
        position: usize,
        /// Why the envelope failed.
        detail: String,
    },
    /// An entry's signer is not the identity the entry claims as its author
    /// (`hex(BLAKE3(signer_pubkey)) != stake_holder_id`).
    SignerNotAuthor {
        /// Position in the chain.
        position: usize,
    },
    /// This container's own spine already holds (or has held) this asset. The
    /// spine is authoritative for its own assets; an import may not shadow one.
    AlreadyOnSpine,
    /// A different history for this asset is already held off-spine. An
    /// accepted foreign chain may only ever be EXTENDED, never replaced.
    Conflict {
        /// Position at which the presented chain diverges from the held one.
        position: usize,
    },
    /// The presented chain is shorter than the one already held — nothing to
    /// adopt, and truncation is not an update.
    NotAnExtension {
        /// Entries already held.
        held: usize,
        /// Entries presented.
        presented: usize,
    },
    /// The off-spine store cannot fit this chain. Carries WHICH bound was hit,
    /// so a caller and an operator read the same diagnosis.
    ///
    /// Produced in exactly one place — [`ForeignChainStore::admission_check`](super::store::ForeignChainStore::admission_check) —
    /// which both the early capacity probe and the authoritative adopt-time
    /// check call. There is no second capacity rule to drift from this one.
    StoreFull(StoreBound),
}

/// Which of the off-spine store's bounds refused an admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreBound {
    /// The byte budget [`MAX_FOREIGN_STORE_BYTES`] — the memory bound.
    Bytes {
        /// Bytes already held.
        held: usize,
        /// Bytes this admission would add.
        incoming: usize,
        /// The budget.
        budget: usize,
    },
    /// The distinct-chain count [`MAX_FOREIGN_CHAINS`] — the key-space guard.
    Chains {
        /// Chains already held.
        held: usize,
        /// The cap.
        limit: usize,
    },
}

impl std::fmt::Display for StoreBound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bytes {
                held,
                incoming,
                budget,
            } => write!(
                f,
                "byte budget: {held} held + {incoming} incoming exceeds {budget}"
            ),
            Self::Chains { held, limit } => {
                write!(f, "chain count: {held} held, limit is {limit}")
            }
        }
    }
}

impl std::fmt::Display for ForeignChainReject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "foreign asset-chain carries no entries"),
            Self::TooLong { presented, limit } => write!(
                f,
                "foreign asset-chain has {presented} entries, limit is {limit}"
            ),
            Self::LineageBroken(break_) => {
                write!(f, "foreign asset-chain lineage is broken: {break_}")
            }
            Self::Unsigned { position } => write!(
                f,
                "foreign asset-chain entry {position} has no FALCON signed_proof envelope \
                 — an imported history must be attributable at every step"
            ),
            Self::BadSignature { position, detail } => write!(
                f,
                "foreign asset-chain entry {position} signature invalid: {detail}"
            ),
            Self::SignerNotAuthor { position } => write!(
                f,
                "foreign asset-chain entry {position} was signed by a key that does not \
                 derive its claimed author (BLAKE3(pubkey) != stake_holder_id)"
            ),
            Self::AlreadyOnSpine => write!(
                f,
                "this container's own spine already holds this asset — a foreign import \
                 may not shadow a local title"
            ),
            Self::Conflict { position } => write!(
                f,
                "foreign asset-chain diverges at entry {position} from the history already \
                 adopted for this asset — an adopted chain may only be extended"
            ),
            Self::NotAnExtension { held, presented } => write!(
                f,
                "foreign asset-chain presents {presented} entries but {held} are already \
                 held — truncation is not an extension"
            ),
            Self::StoreFull(bound) => write!(
                f,
                "foreign asset-chain store is at capacity ({bound}) — refusing to admit a \
                 new chain (nothing already adopted is evicted)"
            ),
        }
    }
}

/// What an accepted foreign asset-chain produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForeignChainReceipt {
    /// The asset adopted.
    pub asset_hash: [u8; 32],
    /// Entries now held for it.
    pub entries: usize,
    /// Entries this accept ADDED (0 when the presentation was a re-submission
    /// of a chain already held in full).
    pub added: usize,
    /// `lineage_id` of the adopted head.
    pub head_lineage_id: String,
    /// `asset_seq` of the adopted head.
    pub head_seq: u64,
}
