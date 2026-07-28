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

use std::collections::HashMap;

use super::block::BlockAssetEntry;
use super::chain::NodeBlockchain;
use super::lineage::{AssetLineage, LineageBreak};
use super::mutations::signer_binds_to_author;

/// Total bytes of foreign asset-chain material this container will hold
/// off-spine. **This is the bound that binds.**
///
/// # Why a byte budget and not a count
///
/// The count caps below cannot bound memory on their own. A real FALCON-signed
/// entry measures **7,038 bytes** (`s3_4_the_store_bound_is_a_byte_budget_that_fits_r13`
/// prints it): the entry carries its `StateProof` TWICE — once parsed, once as
/// the `proof_bytes` the envelope signs — plus a 1793-byte public key and a
/// ~1280-byte signature. 1024 × 512 × 7,038 B = **3.44 GiB**, i.e. **86 % of
/// R13's entire 4 GB minimum-spec RAM**, claimed by an off-spine cache of other
/// containers' titles. (It does not quite *exceed* 4 GB, as the QA advisory
/// estimated — it is 3.44 GiB, not 2.6 GiB, and it is under the line rather
/// than over it. The conclusion is unchanged: a cache permitted 86 % of a
/// conforming device's RAM is not a bound.)
///
/// # The number, against R13
///
/// R13's floor is 4 GB RAM. 64 MiB is **1.6 %** of it — twice the
/// mirror-attestation pool's byte budget
/// ([`MAX_ATTESTATION_POOL_BYTES`](super::attestations::MAX_ATTESTATION_POOL_BYTES),
/// 32 MiB), which is the sanity reference for what "bounded" means in this
/// crate. (That reference used to be quoted here as "8192 × ~3.3 KiB ≈ 27 MiB",
/// an ESTIMATE, and the estimate was wrong by 8.9×: the pool was count-bounded
/// only, `spine_point` was an unbounded attacker-chosen `String` held twice, and
/// QA measured 30,758 resident bytes per attestation — 240 MiB of real RSS from
/// one keypair. F1 capped the field and gave the pool a measured byte budget of
/// its own, so the reference is now a bound rather than an assumption. The ratio
/// is deliberate: this store caches other containers' titles, the pool holds
/// live third-party statements about assets we already hold.) A
/// minimum-spec node keeps ~3.9 GB for the block store, the matrix, STOQ
/// buffers and the asset pipeline's streaming windows (which R13 already
/// requires to be bounded), so this store cannot be the thing that OOMs it.
///
/// At the measured 7,038-byte entry footprint, 64 MiB is ~9,500 adopted
/// entries — e.g. 1024 chains averaging 9 entries, or a few hundred long
/// histories. That is a working set, not an archive; the off-spine store is a
/// cache of other containers' titles, and an operator reclaims space
/// explicitly with
/// [`forget_foreign_asset_chain`](NodeBlockchain::forget_foreign_asset_chain).
pub const MAX_FOREIGN_STORE_BYTES: usize = 64 * 1024 * 1024;

/// Maximum number of distinct foreign asset-chains held off-spine.
///
/// SECONDARY guard. [`MAX_FOREIGN_STORE_BYTES`] is the memory bound; this one
/// bounds the *key* space, so a flood of one-entry chains cannot cost
/// unbounded `HashMap` bookkeeping under the byte budget.
pub const MAX_FOREIGN_CHAINS: usize = 1024;

/// Maximum number of entries in one foreign asset-chain.
///
/// SECONDARY guard, and a CPU one: verification cost is O(entries) FALCON-1024
/// verifications, so an unbounded chain is a compute-exhaustion primitive as
/// much as a memory one. The cap is applied BEFORE any signature work.
pub const MAX_FOREIGN_CHAIN_ENTRIES: usize = 512;

/// Bytes charged for an entry whose footprint cannot be measured.
///
/// Only reachable for an entry with no `signed_proof` envelope, which the
/// accept path refuses before adoption ([`ForeignChainReject::Unsigned`]), or
/// for one whose pointer/registration will not serialize. Charging a large
/// constant fails in the safe direction: an unmeasurable entry consumes budget
/// as if it were the largest thing the wire could carry.
const UNMEASURED_ENTRY_BYTES: usize = 64 * 1024;

/// Per-entry allocator and container bookkeeping: the `Vec` slot, the
/// `HashMap` bucket amortised over the chain, and allocator size-class
/// rounding on each of the entry's several small allocations.
const ENTRY_BOOKKEEPING_BYTES: usize = 512;

/// Bytes an entry is charged against [`MAX_FOREIGN_STORE_BYTES`].
///
/// Computed STRUCTURALLY, not by serializing the entry: every variable-length
/// part is reachable directly, so the measurement costs no round-trip on a
/// path that already runs a FALCON-1024 verification per entry.
///
/// The accounting, and why each term is an upper bound on real memory:
///
/// * `size_of::<BlockAssetEntry>()` — the inline struct.
/// * the envelope's three `Vec<u8>` — `proof_bytes`, `signature`,
///   `signer_pubkey` — charged at their exact lengths.
/// * `state_proof` — the entry holds the proof TWICE: parsed here, and as the
///   `proof_bytes` the signature covers. A parsed JSON document's heap is
///   bounded by the text it was parsed from (numbers shrink to fixed-width
///   fields; escapes only shrink; every string is at most its source run), so
///   `proof_bytes.len()` is a sound upper bound for the parsed copy.
/// * `storage_pointer` + `registration` — not covered by `proof_bytes`, so
///   measured directly. These are the only two terms that serialize, and they
///   are the two smallest.
///
/// It is deliberately an OVER-estimate. Under-counting is what turns a byte
/// budget back into the fiction the count caps were.
pub fn entry_footprint_bytes(entry: &BlockAssetEntry) -> usize {
    let Some(envelope) = entry.signed_proof.as_ref() else {
        return UNMEASURED_ENTRY_BYTES;
    };

    let tail = serde_json::to_vec(&(&entry.storage_pointer, &entry.registration))
        .map_or(UNMEASURED_ENTRY_BYTES, |bytes| bytes.len());

    std::mem::size_of::<BlockAssetEntry>()
        .saturating_add(envelope.proof_bytes.len())
        .saturating_add(envelope.signature.len())
        .saturating_add(envelope.signer_pubkey.len())
        // the parsed second copy of the same proof
        .saturating_add(envelope.proof_bytes.len())
        .saturating_add(tail)
        .saturating_add(ENTRY_BOOKKEEPING_BYTES)
}

/// Bytes a whole presented chain would be charged.
pub fn chain_footprint_bytes(entries: &[BlockAssetEntry]) -> usize {
    entries
        .iter()
        .map(entry_footprint_bytes)
        .fold(0usize, usize::saturating_add)
}

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
    /// Produced in exactly one place — [`ForeignChainStore::admission_check`] —
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

/// Off-spine storage for verified foreign asset-chains.
///
/// # Bound: a byte budget, with the counts as secondary guards
///
/// The store is fed from outside this container, so it is bounded — and the
/// bound that binds is **[`MAX_FOREIGN_STORE_BYTES`] (64 MiB)**, measured per
/// entry by [`entry_footprint_bytes`] and maintained incrementally in
/// `bytes_held`. [`MAX_FOREIGN_CHAINS`] and [`MAX_FOREIGN_CHAIN_ENTRIES`]
/// remain, as a key-space guard and a per-message compute guard respectively,
/// but neither is load-bearing for memory: their product at the measured
/// 7,038-byte entry footprint is 3.44 GiB — 86 % of R13's 4 GB minimum-spec
/// RAM.
///
/// Worst-case footprint of this store, stated plainly: **64 MiB of entry
/// material**, plus `MAX_FOREIGN_CHAINS` × (32-byte key + `Vec` header) ≈
/// 56 KiB of map overhead — call it 64.1 MiB. Against R13 (1 Mb/s, 50 GB,
/// 4 GB RAM, 2-core 1 GHz) that is 1.6 % of RAM, leaving the rest of the node
/// its working set. See [`MAX_FOREIGN_STORE_BYTES`] for the derivation.
///
/// # Why eviction is refused
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
    /// Charged bytes across every held chain, maintained incrementally so the
    /// byte budget is O(1) to test rather than a walk of every entry.
    ///
    /// INVARIANT: `bytes_held == chains.values().flatten().map(entry_footprint_bytes).sum()`.
    /// Every mutation of `chains` goes through [`Self::adopt`] or
    /// [`Self::forget`], and both re-establish it; `debug_assert_accounting`
    /// checks it after each so a future prune/expiry that touches `chains`
    /// directly fails loudly in debug and test builds instead of silently
    /// desyncing the bound.
    bytes_held: usize,
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

    /// Charged bytes across every held chain.
    pub fn bytes_held(&self) -> usize {
        self.bytes_held
    }

    /// Recompute the byte accounting from the entries actually held.
    ///
    /// The definition `bytes_held` is a cache OF. Used by the debug assertion
    /// and by the tests that prove the two cannot drift.
    pub fn recomputed_bytes(&self) -> usize {
        self.chains
            .values()
            .map(|entries| chain_footprint_bytes(entries))
            .fold(0usize, usize::saturating_add)
    }

    /// A6 — tie the incrementally-maintained counter to its definition.
    ///
    /// Compiled out of release builds (the walk is O(entries)), but any test or
    /// debug run of a future prune/expiry path that mutates `chains` without
    /// adjusting `bytes_held` aborts here rather than quietly widening the
    /// bound. The equivalent guard for the attestation pool's `total` lives in
    /// [`MirrorAttestationPool`](super::attestations::MirrorAttestationPool).
    #[inline]
    fn debug_assert_accounting(&self) {
        debug_assert_eq!(
            self.bytes_held,
            self.recomputed_bytes(),
            "ForeignChainStore.bytes_held desynced from the entries held — a mutation \
             bypassed adopt()/forget()"
        );
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
        let dropped = self.chains.remove(asset_hash);
        let released = dropped
            .as_deref()
            .map_or(0, |entries| chain_footprint_bytes(entries));
        self.bytes_held = self.bytes_held.saturating_sub(released);
        self.debug_assert_accounting();
        dropped.map_or(0, |c| c.len())
    }

    /// A3 — the ONE capacity rule, asked in one place.
    ///
    /// Called twice per accept, deliberately:
    ///
    /// 1. as an EARLY probe, under a read lock, before any FALCON-1024 work —
    ///    so a refusal at steady-state capacity costs O(1) instead of up to
    ///    [`MAX_FOREIGN_CHAIN_ENTRIES`] signature verifications;
    /// 2. as the AUTHORITATIVE check inside [`Self::adopt`], under the write
    ///    lock that actually admits the entries.
    ///
    /// Both are required. The lock is not held between them, so the early probe
    /// is only an optimization and can never be the sole gate. Because they
    /// call THIS function — not two transcriptions of the same rule — they
    /// cannot disagree about whether the store is full or about which
    /// [`StoreBound`] refused, which is the divergence class that failed S3.3's
    /// gate.
    ///
    /// `incoming_bytes` is what the admission would ADD. An extension charges
    /// only its tail; a new chain charges all of it. `is_new_chain` distinguishes
    /// growth of the key space from growth of an existing chain — extending a
    /// held chain is not a new key and is never refused by the chain-count
    /// guard.
    pub fn admission_check(
        &self,
        is_new_chain: bool,
        incoming_bytes: usize,
    ) -> Result<(), ForeignChainReject> {
        if is_new_chain && self.chains.len() >= MAX_FOREIGN_CHAINS {
            return Err(ForeignChainReject::StoreFull(StoreBound::Chains {
                held: self.chains.len(),
                limit: MAX_FOREIGN_CHAINS,
            }));
        }
        if self.bytes_held.saturating_add(incoming_bytes) > MAX_FOREIGN_STORE_BYTES {
            return Err(ForeignChainReject::StoreFull(StoreBound::Bytes {
                held: self.bytes_held,
                incoming: incoming_bytes,
                budget: MAX_FOREIGN_STORE_BYTES,
            }));
        }
        Ok(())
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
        // Read-only judgement first, so a refusal leaves the store untouched
        // and its accounting trivially intact.
        let (is_new_chain, split_at) = match self.chains.get(&asset_hash) {
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
                (false, held.len())
            }
            None => (true, 0),
        };

        let tail: &[BlockAssetEntry] = entries.get(split_at..).unwrap_or(&[]);
        let incoming_bytes = chain_footprint_bytes(tail);

        // A3 — the authoritative capacity gate, under the write lock, charging
        // exactly the bytes this admission adds. Same function as the early
        // probe, so the same inputs give the same verdict.
        self.admission_check(is_new_chain, incoming_bytes)?;

        let added = tail.len();
        let mut entries = entries;
        if is_new_chain {
            self.chains.insert(asset_hash, entries);
        } else {
            let tail = entries.split_off(split_at);
            match self.chains.get_mut(&asset_hash) {
                Some(held) => held.extend(tail),
                // Unreachable: `is_new_chain` is false only because the lookup
                // above found a chain, and `&mut self` has held the store since.
                None => {
                    self.chains.insert(asset_hash, tail);
                }
            }
        }
        self.bytes_held = self.bytes_held.saturating_add(incoming_bytes);
        self.debug_assert_accounting();
        Ok(added)
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
