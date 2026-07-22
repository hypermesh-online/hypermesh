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

use std::collections::HashMap;

use super::block::BlockAssetEntry;
use super::chain::NodeBlockchain;
use super::lineage::{AssetLineage, LineageBreak};
use super::mutations::signer_binds_to_author;

/// Maximum number of distinct foreign asset-chains held off-spine.
///
/// This store is fed from outside, so it is bounded. See
/// [`ForeignChainStore`] for the reject-at-capacity rationale.
pub const MAX_FOREIGN_CHAINS: usize = 1024;

/// Maximum number of entries in one foreign asset-chain.
///
/// Verification cost is O(entries) FALCON-1024 verifications, so an unbounded
/// chain is a CPU-exhaustion primitive as much as a memory one. The cap is
/// applied BEFORE any signature work.
pub const MAX_FOREIGN_CHAIN_ENTRIES: usize = 512;

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
    /// The off-spine store is at [`MAX_FOREIGN_CHAINS`].
    StoreFull {
        /// The cap.
        limit: usize,
    },
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
            Self::StoreFull { limit } => write!(
                f,
                "foreign asset-chain store is at capacity ({limit}) — refusing to admit a \
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

/// Off-spine storage for verified foreign asset-chains.
///
/// # Bound, and why eviction is refused
///
/// The store is fed from outside this container, so it is capped at
/// [`MAX_FOREIGN_CHAINS`] chains of [`MAX_FOREIGN_CHAIN_ENTRIES`] entries each.
/// At capacity a NEW chain is REFUSED; nothing already adopted is evicted.
///
/// The alternative — evict-oldest, as the orphan buffer does — is
/// attacker-steerable here in a way it is not there. An orphan is unverified
/// and provisional; an adopted foreign chain is verified history that a caller
/// may already have acted on. Making admission of attacker-supplied chains able
/// to *displace* it would hand an attacker a deletion primitive. Refusing new
/// admissions loses nothing that was already established, and an operator can
/// always release space explicitly with
/// [`forget_foreign_asset_chain`](NodeBlockchain::forget_foreign_asset_chain).
#[derive(Clone, Debug, Default)]
pub struct ForeignChainStore {
    chains: HashMap<[u8; 32], Vec<BlockAssetEntry>>,
}

impl ForeignChainStore {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of distinct foreign asset-chains held.
    pub fn len(&self) -> usize {
        self.chains.len()
    }

    /// Whether nothing has been adopted.
    pub fn is_empty(&self) -> bool {
        self.chains.is_empty()
    }

    /// The entries held for `asset_hash`, in chain order.
    pub fn entries(&self, asset_hash: &[u8; 32]) -> &[BlockAssetEntry] {
        self.chains.get(asset_hash).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Whether a chain is held for `asset_hash`.
    pub fn contains(&self, asset_hash: &[u8; 32]) -> bool {
        self.chains.contains_key(asset_hash)
    }

    /// Every adopted asset hash (unordered).
    pub fn asset_hashes(&self) -> impl Iterator<Item = &[u8; 32]> {
        self.chains.keys()
    }

    /// Drop the chain held for `asset_hash`; returns how many entries went.
    pub fn forget(&mut self, asset_hash: &[u8; 32]) -> usize {
        self.chains.remove(asset_hash).map_or(0, |c| c.len())
    }

    /// Adopt `entries` for `asset_hash`, which the caller has ALREADY verified.
    ///
    /// Enforces the extension rule (a held chain may only grow, and only along
    /// the same history) and the capacity bound.
    ///
    /// # What "the same history" is compared by
    ///
    /// By each entry's `lineage_id` (= `hex(proof_hash)`) and `asset_seq` — the
    /// AUTHENTICATED identity of the entry, the value `Block::calculate_hash`
    /// commits to and the value a successor names in `prev_asset_entry`.
    ///
    /// NOT by whole-entry equality. The `signed_proof` envelope carries a random
    /// nonce, so an honest re-presentation of the same history by the same
    /// author legitimately carries different signature bytes; whole-entry
    /// equality would read that as a conflicting history. The envelope has
    /// already been FALCON-verified and author-bound for every presented entry,
    /// so identity is the right — and the only authenticated — thing to compare.
    ///
    /// On a matching prefix the HELD entries are kept verbatim and only the tail
    /// is appended, so a re-presentation cannot overwrite the copy we adopted
    /// with fields (`registration`, `storage_pointer`) that `proof_hash` does
    /// not cover.
    fn adopt(
        &mut self,
        asset_hash: [u8; 32],
        entries: Vec<BlockAssetEntry>,
    ) -> Result<usize, ForeignChainReject> {
        match self.chains.get_mut(&asset_hash) {
            Some(held) => {
                if entries.len() < held.len() {
                    return Err(ForeignChainReject::NotAnExtension {
                        held: held.len(),
                        presented: entries.len(),
                    });
                }
                for (position, held_entry) in held.iter().enumerate() {
                    let same = entries.get(position).is_some_and(|presented| {
                        presented.lineage_id() == held_entry.lineage_id()
                            && presented.asset_seq() == held_entry.asset_seq()
                    });
                    if !same {
                        return Err(ForeignChainReject::Conflict { position });
                    }
                }
                let tail: Vec<BlockAssetEntry> = entries[held.len()..].to_vec();
                let added = tail.len();
                held.extend(tail);
                Ok(added)
            }
            None => {
                if self.chains.len() >= MAX_FOREIGN_CHAINS {
                    return Err(ForeignChainReject::StoreFull {
                        limit: MAX_FOREIGN_CHAINS,
                    });
                }
                let added = entries.len();
                self.chains.insert(asset_hash, entries);
                Ok(added)
            }
        }
    }
}

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
    /// Order of judgement (all fail-closed, cheapest first):
    /// 1. non-empty, and within [`MAX_FOREIGN_CHAIN_ENTRIES`] — bounds the
    ///    signature work an untrusted caller can provoke;
    /// 2. the asset is not one this container's own spine holds or has held
    ///    ([`ForeignChainReject::AlreadyOnSpine`]);
    /// 3. **internal lineage** — [`AssetLineage::verify`], i.e. every entry is
    ///    for this asset, `proof_hash == BLAKE3(serialize(state_proof))`, the
    ///    proof is content-bound, the root IS an asset-genesis, and every later
    ///    entry names its predecessor's `lineage_id` with `asset_seq` advancing
    ///    by exactly one. Node-block indices play no part;
    /// 4. **every signer** — FALCON-1024 envelope present, valid, and bound to
    ///    the identity the entry claims as author;
    /// 5. adoption — extension-only, capacity-bounded.
    ///
    /// On success the chain is queryable through
    /// [`foreign_asset_lineage`](Self::foreign_asset_lineage) and
    /// [`asset_lineage_any`](Self::asset_lineage_any). Node-spine height, head,
    /// block count and the S3.1 index are untouched.
    pub async fn accept_foreign_asset_chain(
        &self,
        chain: ForeignAssetChain,
    ) -> Result<ForeignChainReceipt, ForeignChainReject> {
        if chain.is_empty() {
            return Err(ForeignChainReject::Empty);
        }
        if chain.len() > MAX_FOREIGN_CHAIN_ENTRIES {
            return Err(ForeignChainReject::TooLong {
                presented: chain.len(),
                limit: MAX_FOREIGN_CHAIN_ENTRIES,
            });
        }

        // The spine owns the assets it holds. F1's tombstone is consulted, so
        // an asset whose entries were pruned still counts as ours.
        if self.has_ever_seen_asset(&chain.asset_hash).await {
            return Err(ForeignChainReject::AlreadyOnSpine);
        }

        chain
            .as_lineage()
            .verify()
            .map_err(ForeignChainReject::LineageBroken)?;

        verify_every_signer(&chain.entries)?;

        let head = chain
            .head()
            .ok_or(ForeignChainReject::Empty)?
            .clone();

        let (entries, added) = {
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

    /// S3.4 — the adopted foreign history for `asset_hash`, if any.
    ///
    /// Returns an [`AssetLineage`], the same shape
    /// [`asset_lineage`](Self::asset_lineage) returns for a spine-held asset,
    /// so a caller verifies an imported title exactly as it verifies a local
    /// one.
    pub async fn foreign_asset_lineage(&self, asset_hash: &[u8; 32]) -> Option<AssetLineage> {
        let store = self.foreign_chains.read().await;
        if !store.contains(asset_hash) {
            return None;
        }
        Some(AssetLineage {
            asset_hash: *asset_hash,
            entries: store.entries(asset_hash).to_vec(),
        })
    }

    /// S3.4 — the adopted head entry for `asset_hash`, if any.
    pub async fn foreign_asset_head(&self, asset_hash: &[u8; 32]) -> Option<BlockAssetEntry> {
        self.foreign_chains
            .read()
            .await
            .entries(asset_hash)
            .last()
            .cloned()
    }

    /// S3.4 — whether a foreign chain has been adopted for `asset_hash`.
    pub async fn has_foreign_asset_chain(&self, asset_hash: &[u8; 32]) -> bool {
        self.foreign_chains.read().await.contains(asset_hash)
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
    pub async fn forget_foreign_asset_chain(&self, asset_hash: &[u8; 32]) -> usize {
        self.foreign_chains.write().await.forget(asset_hash)
    }
}
