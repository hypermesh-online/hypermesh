// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! S3.4 — the bounded received-chain store: byte accounting, the capacity bounds,
//! and extension-only adoption of already-verified received asset-chains.

use std::collections::HashMap;

use crate::blockchain::block::BlockAssetEntry;

use super::presented::{AcceptReject, StoreBound};

/// Total bytes of received asset-chain material this container will hold
/// aside from its own block chain. **This is the bound that binds.**
///
/// # Why a byte budget and not a count
///
/// The count caps below cannot bound memory on their own. A real FALCON-signed
/// entry measures **7,038 bytes** (`s3_4_the_store_bound_is_a_byte_budget_that_fits_r13`
/// prints it): the entry carries its `StateProof` TWICE — once parsed, once as
/// the `proof_bytes` the envelope signs — plus a 1793-byte public key and a
/// ~1280-byte signature. 1024 × 512 × 7,038 B = **3.44 GiB**, i.e. **86 % of
/// R13's entire 4 GB minimum-spec RAM**, claimed by a cache of other
/// containers' titles. (It does not quite *exceed* 4 GB, as the QA advisory
/// estimated — it is 3.44 GiB, not 2.6 GiB, and it is under the line rather
/// than over it. The conclusion is unchanged: a cache permitted 86 % of a
/// conforming device's RAM is not a bound.)
///
/// # The number, against R13
///
/// R13's floor is 4 GB RAM. 64 MiB is **1.6 %** of it — twice the
/// mirror-attestation pool's byte budget
/// ([`MAX_ATTESTATION_POOL_BYTES`](crate::blockchain::attestations::MAX_ATTESTATION_POOL_BYTES),
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
/// histories. That is a working set, not an archive; the received-chain store is a
/// cache of other containers' titles, and an operator reclaims space
/// explicitly with
/// [`forget_received_asset_chain`](crate::blockchain::chain::NodeBlockchain::forget_received_asset_chain).
pub const MAX_RECEIVED_STORE_BYTES: usize = 64 * 1024 * 1024;

/// Maximum number of distinct received asset-chains held.
///
/// SECONDARY guard. [`MAX_RECEIVED_STORE_BYTES`] is the memory bound; this one
/// bounds the *key* space, so a flood of one-entry chains cannot cost
/// unbounded `HashMap` bookkeeping under the byte budget.
pub const MAX_RECEIVED_CHAINS: usize = 1024;

/// Maximum number of entries in one received asset-chain.
///
/// SECONDARY guard, and a CPU one: verification cost is O(entries) FALCON-1024
/// verifications, so an unbounded chain is a compute-exhaustion primitive as
/// much as a memory one. The cap is applied BEFORE any signature work.
pub const MAX_RECEIVED_CHAIN_ENTRIES: usize = 512;

/// Bytes charged for an entry whose footprint cannot be measured.
///
/// Only reachable for an entry with no `signed_proof` envelope, which the
/// accept path refuses before adoption ([`AcceptReject::Unsigned`]), or
/// for one whose pointer/registration will not serialize. Charging a large
/// constant fails in the safe direction: an unmeasurable entry consumes budget
/// as if it were the largest thing the wire could carry.
const UNMEASURED_ENTRY_BYTES: usize = 64 * 1024;

/// Per-entry allocator and container bookkeeping: the `Vec` slot, the
/// `HashMap` bucket amortised over the chain, and allocator size-class
/// rounding on each of the entry's several small allocations.
const ENTRY_BOOKKEEPING_BYTES: usize = 512;

/// Bytes an entry is charged against [`MAX_RECEIVED_STORE_BYTES`].
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

/// Storage for verified received asset-chains, held aside from the node's own block chain.
///
/// # Bound: a byte budget, with the counts as secondary guards
///
/// The store is fed from outside this container, so it is bounded — and the
/// bound that binds is **[`MAX_RECEIVED_STORE_BYTES`] (64 MiB)**, measured per
/// entry by [`entry_footprint_bytes`] and maintained incrementally in
/// `bytes_held`. [`MAX_RECEIVED_CHAINS`] and [`MAX_RECEIVED_CHAIN_ENTRIES`]
/// remain, as a key-space guard and a per-message compute guard respectively,
/// but neither is load-bearing for memory: their product at the measured
/// 7,038-byte entry footprint is 3.44 GiB — 86 % of R13's 4 GB minimum-spec
/// RAM.
///
/// Worst-case footprint of this store, stated plainly: **64 MiB of entry
/// material**, plus `MAX_RECEIVED_CHAINS` × (32-byte key + `Vec` header) ≈
/// 56 KiB of map overhead — call it 64.1 MiB. Against R13 (1 Mb/s, 50 GB,
/// 4 GB RAM, 2-core 1 GHz) that is 1.6 % of RAM, leaving the rest of the node
/// its working set. See [`MAX_RECEIVED_STORE_BYTES`] for the derivation.
///
/// # Why eviction is refused
///
/// The alternative — evict-oldest, as the orphan buffer does — is
/// attacker-steerable here in a way it is not there. An orphan is unverified
/// and provisional; an adopted received chain is verified history that a caller
/// may already have acted on. Making admission of attacker-supplied chains able
/// to *displace* it would hand an attacker a deletion primitive. Refusing new
/// admissions loses nothing that was already established, and an operator can
/// always release space explicitly with
/// [`forget_received_asset_chain`](crate::blockchain::chain::NodeBlockchain::forget_received_asset_chain).
#[derive(Clone, Debug, Default)]
pub struct ReceivedAssetStore {
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

impl ReceivedAssetStore {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of distinct received asset-chains held.
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
    /// [`MirrorAttestationPool`](crate::blockchain::attestations::MirrorAttestationPool).
    #[inline]
    fn debug_assert_accounting(&self) {
        debug_assert_eq!(
            self.bytes_held,
            self.recomputed_bytes(),
            "ReceivedAssetStore.bytes_held desynced from the entries held — a mutation \
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
    ///    [`MAX_RECEIVED_CHAIN_ENTRIES`] signature verifications;
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
    ) -> Result<(), AcceptReject> {
        if is_new_chain && self.chains.len() >= MAX_RECEIVED_CHAINS {
            return Err(AcceptReject::StoreFull(StoreBound::Chains {
                held: self.chains.len(),
                limit: MAX_RECEIVED_CHAINS,
            }));
        }
        if self.bytes_held.saturating_add(incoming_bytes) > MAX_RECEIVED_STORE_BYTES {
            return Err(AcceptReject::StoreFull(StoreBound::Bytes {
                held: self.bytes_held,
                incoming: incoming_bytes,
                budget: MAX_RECEIVED_STORE_BYTES,
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
    pub(super) fn adopt(
        &mut self,
        asset_hash: [u8; 32],
        entries: Vec<BlockAssetEntry>,
    ) -> Result<usize, AcceptReject> {
        // Read-only judgement first, so a refusal leaves the store untouched
        // and its accounting trivially intact.
        let (is_new_chain, split_at) = match self.chains.get(&asset_hash) {
            Some(held) => {
                if entries.len() < held.len() {
                    return Err(AcceptReject::NotAnExtension {
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
                        return Err(AcceptReject::Conflict { position });
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
