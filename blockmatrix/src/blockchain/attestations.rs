// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! S3.3 — mirror attestations: FALCON verification, off-spine accumulation,
//! and the owner's seal.
//!
//! # Where this sits
//!
//! S3.2 gave the asset a **linear title** — a single-parent spine extended only
//! by the holder of the distribution right (`prev_asset_entry` / `asset_seq`).
//! S3.3 adds the OTHER dimension: everyone who *holds* the asset is a MIRROR
//! (VISION §4 — access IS mirroring), and a mirror's "I hold and validated
//! this" statement is a [`MirrorAttestation`].
//!
//! Attestations accumulate **off the spine**, in [`MirrorAttestationPool`].
//! Recording one consumes NO `prev_asset_entry` / `asset_seq` slot and writes
//! no block, so N mirrors attesting concurrently can neither fork nor renumber
//! the title. They are keyed — and therefore ORDERED — by **matrix position**
//! (D1: *"seeds mirroring on torrent, and blocks hold an index in the
//! matrix"*), which is what removes any need for a trusted clock, a Lamport
//! counter, or a vector clock in the mirror dimension.
//!
//! # Why the pool lives on the chain
//!
//! Next to the S3.1 [`AssetChainIndex`](super::asset_index::AssetChainIndex),
//! and for the same reason: it is per-asset, in-memory, derived-not-authoritative
//! state that the write path consults. The seal needs the pool and the asset's
//! head in one atomic-enough step, and the ONLY object holding both is the
//! chain. Keeping it anywhere else would mean a second place that has to be
//! kept consistent with the spine.
//!
//! Unlike the index, the pool is NOT rebuilt from blocks: attestations are
//! third-party statements that arrive over the network, not derived from the
//! local block set. That is precisely why they must be SEALED to become
//! durable — an unsealed pool is live state (NGauge's "who is mirroring right
//! now"), a sealed root is the audit trail.
//!
//! # Verification is third-party by design
//!
//! [`verify_attestation`] follows `verify_grant`
//! (`assets/core/authz.rs`) exactly: structural bind → FALCON-1024 verify →
//! identity binding to the **named third party**. It deliberately does NOT do
//! what H3's `signer_binds_to_author` does (require signer == author): a
//! mirror is by definition someone OTHER than the asset's author, and binding
//! to the author is exactly the check that makes `signed_proof` unable to
//! carry this.

use std::collections::{BTreeMap, HashMap};

use hypermesh_lib::attestation::{
    build_seal, MatrixIndex, MembershipProof, MirrorAttestation, MirrorSeal, SealBreak,
};
use tracing::{info, warn};

use super::block::{Block, BlockAssetEntry};
use super::chain::NodeBlockchain;
use crate::matrix::coordinate::MatrixCoordinate;

/// Convert a BlockMatrix matrix coordinate into the `lib`-level ordering key.
///
/// Same `(i64, i64, i64)` lattice point; [`MatrixIndex`] additionally derives
/// `Ord`, which is what makes the sealed set deterministic.
pub fn matrix_index_of(coordinate: &MatrixCoordinate) -> MatrixIndex {
    MatrixIndex::new(coordinate.x, coordinate.y, coordinate.z)
}

/// FALCON-verify a [`MirrorAttestation`] and confirm it is bound to its
/// attesting mirror.
///
/// Returns `true` iff:
/// 1. [`MirrorAttestation::is_structurally_valid`] holds — the WHOLE audit
///    gate, by delegation (envelope covers this attestation's fields, the
///    signer binds to the claimed mirror, and every identity/spine/key field
///    the audit path requires is present), and
/// 2. the FALCON-1024 detached signature over `BLAKE3(proof_bytes || nonce)`
///    verifies against the embedded `signer_pubkey`.
///
/// # The accept gate is a SUPERSET of the audit gate, by construction (B1)
///
/// This function used to re-list the structural checks it cared about
/// (`proof_bytes_match` + `binds_to_signer`) instead of calling the audit gate.
/// That divergence was a denial-of-audit: `is_structurally_valid` ALSO requires
/// a non-empty `spine_point`, so an attestation with `spine_point: ""` — fully
/// FALCON-valid, correctly identity-bound — was ACCEPTED here, recorded, and
/// hash-committed into a `MirrorSeal`. The resulting seal was then permanently
/// unverifiable: [`verify_sealed_set`](hypermesh_lib::attestation::verify_sealed_set)
/// rejects the very set the owner sealed, forever, whether or not anything was
/// stripped. One cheap attestation destroyed the tamper-evidence of an
/// immutable on-chain root.
///
/// The fix is structural, NOT a special case for `spine_point`: the accept gate
/// now *calls* the audit gate, so there is exactly ONE list of structural
/// requirements. A field added to `is_structurally_valid` tomorrow is enforced
/// at accept time the same day, and the two gates cannot drift apart again.
/// `superset_property` in the tests below asserts the implication directly.
///
/// The identity binding is the `verify_grant` binding applied to a named third
/// party. A THIRD-PARTY signature — one from a key that is not the asset's
/// author or owner — passes here, which is the entire point: the attestor is
/// a different actor from the title holder.
pub fn verify_attestation(attestation: &MirrorAttestation) -> bool {
    // (1) The audit gate, in full, by delegation — never re-listed here.
    if !attestation.is_structurally_valid() {
        return false;
    }

    // (2) FALCON-1024 over BLAKE3(proof_bytes || nonce).
    let digest = attestation.signing_digest_bytes();
    <crate::identity::FalconIdentity as hypermesh_lib::NodeSigner>::verify_signature(
        &attestation.signature.signer_pubkey,
        &digest,
        &attestation.signature.signature,
    )
    .unwrap_or(false)
}

/// S3.4 — maximum live attestations held for ONE asset.
///
/// SECONDARY guard. One mirror at one cell has exactly one live attestation, so
/// this is a cap on distinct `(matrix cell, mirror)` attestors per asset. FALCON
/// keypairs are free to mint, so identity count is not a scarce resource; this
/// bounds one asset's *key space*, and
/// [`MAX_ATTESTATION_POOL_BYTES`] bounds the memory.
pub const MAX_ATTESTATIONS_PER_ASSET: usize = 256;

/// S3.4 — maximum live attestations held across all assets.
///
/// SECONDARY guard, backstopping the per-asset cap: without it, an attacker
/// holding N assets could still reach `N × MAX_ATTESTATIONS_PER_ASSET`. The wire
/// surface additionally refuses attestations for assets this container does not
/// hold, which is what keeps N itself bounded by real local state rather than by
/// attacker-chosen hashes.
pub const MAX_TOTAL_ATTESTATIONS: usize = 8192;

/// S3.4/F1 — total bytes of mirror-attestation material this container holds
/// off-spine. **This is the bound that binds.**
///
/// # Why a byte budget and not a count
///
/// The counts above cannot bound memory on their own, and the gap was not
/// theoretical. `MirrorAttestation::spine_point` was an unbounded
/// attacker-chosen `String`, held TWICE (the field, and again inside
/// `signature.proof_bytes`, which must equal the canonical bytes it appears in).
/// QA drove the shipped wire path — `encode → decode → accept_wire_attestation`,
/// one FALCON keypair, 256 attacker-chosen matrix cells per asset — with
/// `spine_point = "S".repeat(13_400)` (the largest that fits
/// `MAX_ATTESTATION_WIRE_BYTES`) and measured:
///
/// ```text
/// attestations accepted   : 8192 (cap 8192)
/// attacker uploaded       : 510.3 MiB
/// victim RSS before/after : 4568 KiB -> 250632 KiB   (+240.3 MiB)
/// resident per attestation: 30,758 bytes
/// attacker keypairs needed: 1
/// ```
///
/// 240 MiB is **6 % of R13's entire 4 GB minimum-spec RAM**, from one identity,
/// unreclaimable short of restart. The count cap was doing nothing about it.
///
/// # The number, against R13
///
/// Two changes bound this, and both are needed. `MAX_SPINE_POINT_BYTES` (256, in
/// `lib`'s audit gate) caps what one attestation can weigh; this budget caps
/// what all of them together can weigh, so no future field can quietly widen the
/// bound the way `spine_point` did.
///
/// With the field cap in force an attestation's charged footprint is
/// ~4.6 KiB at its absolute maximum and ~4.2 KiB for honest material (dominated
/// by FALCON-1024: a 1793-byte public key and a ~1280-byte signature — the
/// irreducible cost of an attestation being a signed third-party statement).
/// **32 MiB** is therefore ~7,300 honest attestations: the byte budget is
/// reached slightly BEFORE [`MAX_TOTAL_ATTESTATIONS`], which is what makes it
/// the operative bound rather than decoration.
///
/// Against R13 (1 Mb/s, 50 GB, 4 GB RAM, 2-core 1 GHz), 32 MiB is **0.8 %** of
/// RAM — half the received-chain store's 64 MiB, which is the right
/// ratio: that store caches other containers' titles, this pool holds live
/// third-party statements about assets we already hold. A minimum-spec node
/// keeps ~3.9 GB for the block store, the matrix, STOQ buffers and the asset
/// pipeline's streaming windows, so this pool cannot be the thing that OOMs it.
///
/// # This is a resource limit, not a proof term
///
/// It gates admission of network input into local memory. No `StateProof` reads
/// it and no authorization decision consults it; PoStake remains authorization,
/// never a magnitude.
pub const MAX_ATTESTATION_POOL_BYTES: usize = 32 * 1024 * 1024;

/// Per-attestation allocator and container bookkeeping charged on top of the
/// measured field lengths: the `BTreeMap` node, the map's own key/value slots,
/// and allocator size-class rounding on each of the attestation's several small
/// allocations.
const ATTESTATION_BOOKKEEPING_BYTES: usize = 512;

/// S3.4/F1 — bytes an attestation is charged against
/// [`MAX_ATTESTATION_POOL_BYTES`].
///
/// Computed STRUCTURALLY, not by serializing: every variable-length part is
/// reachable directly, so the measurement is O(1) on a path that must stay
/// cheap enough to run BEFORE the FALCON verification (F3).
///
/// The accounting, and why each term is an upper bound on real memory:
///
/// * `size_of::<MirrorAttestation>()` — the inline struct, including the
///   `WireSignedProof`'s three `Vec` headers and its fixed 32-byte nonce.
/// * `mirror` charged **twice** — the field, and again as the `String` half of
///   the pool's `(MatrixIndex, String)` key, which is a clone.
/// * `spine_point` — the field itself.
/// * `proof_bytes` — the SECOND copy of `spine_point` (plus ~176 bytes of
///   canonical framing), which is why the field cap is what makes this small.
/// * `signature` + `signer_pubkey` — the FALCON-1024 material, at their exact
///   lengths.
///
/// Deliberately an OVER-estimate: against the QA reproduction it charges
/// ~30,905 bytes where RSS grew 30,758 per attestation. Under-counting is what
/// turns a byte budget back into the fiction the counts were.
pub fn attestation_footprint_bytes(attestation: &MirrorAttestation) -> usize {
    std::mem::size_of::<MirrorAttestation>()
        .saturating_add(attestation.mirror.len())
        // the pool's BTreeMap key holds a second copy of the mirror identity
        .saturating_add(attestation.mirror.len())
        .saturating_add(attestation.spine_point.len())
        .saturating_add(attestation.signature.proof_bytes.len())
        .saturating_add(attestation.signature.signature.len())
        .saturating_add(attestation.signature.signer_pubkey.len())
        .saturating_add(ATTESTATION_BOOKKEEPING_BYTES)
}

/// Why a verified attestation could not be recorded.
///
/// Distinct from a verification failure on purpose: the attestation is sound,
/// there is simply no room. Callers log it differently, and a mirror can retry.
///
/// Produced in exactly one place — [`MirrorAttestationPool::admission_check`] —
/// which both the early capacity probe (F3) and the authoritative insert call.
/// There is no second capacity rule to drift from this one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PoolFull {
    /// This asset already holds [`MAX_ATTESTATIONS_PER_ASSET`] attestors.
    Asset {
        /// The per-asset cap.
        limit: usize,
    },
    /// The pool holds [`MAX_TOTAL_ATTESTATIONS`] attestations across all assets.
    Global {
        /// The global cap.
        limit: usize,
    },
    /// The byte budget [`MAX_ATTESTATION_POOL_BYTES`] — the memory bound.
    Bytes {
        /// Bytes already held (net of anything this admission would release).
        held: usize,
        /// Bytes this admission would add.
        incoming: usize,
        /// The budget.
        budget: usize,
    },
}

impl std::fmt::Display for PoolFull {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Asset { limit } => write!(
                f,
                "this asset already holds {limit} mirror attestations — refusing a new \
                 attestor (nothing already recorded is evicted)"
            ),
            Self::Global { limit } => write!(
                f,
                "the mirror attestation pool holds {limit} attestations — refusing a new \
                 attestor (nothing already recorded is evicted)"
            ),
            Self::Bytes {
                held,
                incoming,
                budget,
            } => write!(
                f,
                "the mirror attestation pool's byte budget is full: {held} held + \
                 {incoming} incoming exceeds {budget} — refusing (nothing already \
                 recorded is evicted)"
            ),
        }
    }
}

/// Off-spine accumulation of mirror attestations, organized by matrix position.
///
/// `asset_hash → BTreeMap<(matrix cell, mirror), attestation>`.
///
/// The inner map is a `BTreeMap` on purpose: iteration is already in canonical
/// `(cell, mirror)` order, so [`attestations_for`](Self::attestations_for)
/// hands the sealer a set whose order is a function of the MATRIX, never of
/// arrival. Dedupe falls out of the same key — one mirror at one cell has
/// exactly one live attestation, and re-attesting at a newer spine point
/// replaces it rather than inflating the set.
///
/// # Bounded, and why nothing is evicted (S3.4)
///
/// Once attestations arrive over the wire this map is fed by strangers, and an
/// unbounded map fed by strangers is a memory-exhaustion primitive. The bound
/// that binds is **[`MAX_ATTESTATION_POOL_BYTES`] (32 MiB)**, measured per
/// attestation by [`attestation_footprint_bytes`] and maintained incrementally
/// in `bytes_held`. [`MAX_ATTESTATIONS_PER_ASSET`] and
/// [`MAX_TOTAL_ATTESTATIONS`] remain as key-space guards, but neither is
/// load-bearing for memory: before F1 their product bounded nothing at all,
/// because one slot could be made 30 KiB wide (see
/// [`MAX_ATTESTATION_POOL_BYTES`] for the measured reproduction).
///
/// Worst-case footprint of this pool, stated plainly: **32 MiB of attestation
/// material**, plus `MAX_TOTAL_ATTESTATIONS / MAX_ATTESTATIONS_PER_ASSET` ×
/// (32-byte key + `BTreeMap` header) of outer-map overhead — a few KiB. Against
/// R13's 4 GB that is 0.8 % of RAM. At capacity a NEW attestor is REFUSED.
///
/// Eviction was rejected deliberately. An attestation is a mirror's evidence
/// that it held and validated an asset, and the owner's seal is what makes that
/// evidence durable. Any eviction policy — oldest-first, random, anything — is
/// a lever an attacker pulls by flooding: mint attestors until the honest
/// mirror's record is pushed out, then wait for the owner to seal a set that no
/// longer contains it. Refusing admission at capacity cannot delete evidence
/// that is already recorded; it can only delay a newcomer, and a REPLACEMENT
/// (same cell, same mirror, newer spine point) is never refused because it does
/// not grow the set.
///
/// The residual, stated rather than hidden: an attacker who wins the race can
/// occupy an asset's 256 slots and keep later honest mirrors out until an
/// operator releases them. That is a delay, not a deletion, and it is strictly
/// preferable to handing out a deletion primitive.
#[derive(Clone, Debug, Default)]
pub struct MirrorAttestationPool {
    by_asset: HashMap<[u8; 32], BTreeMap<(MatrixIndex, String), MirrorAttestation>>,
    /// Live attestation count across every asset, maintained on insert/removal
    /// so the global bound is O(1) rather than a walk of every asset.
    total: usize,
    /// Charged bytes across every held attestation, maintained incrementally so
    /// the byte budget is O(1) to test rather than a walk of the whole pool.
    ///
    /// INVARIANT: `bytes_held == by_asset.values().flatten().map(|(_, a)|
    /// attestation_footprint_bytes(a)).sum()`. Every mutation of `by_asset` goes
    /// through [`Self::try_insert`] or [`Self::clear_asset`], and both
    /// re-establish it; `debug_assert_accounting` checks it after each.
    bytes_held: usize,
}

impl MirrorAttestationPool {
    /// Empty pool.
    pub fn new() -> Self {
        Self::default()
    }

    /// F1/F3 — the ONE capacity rule, asked in one place.
    ///
    /// Called twice per accept, deliberately:
    ///
    /// 1. as an EARLY probe, under a read lock, before any FALCON-1024 work —
    ///    so a refusal at steady-state capacity costs O(1) map probes instead of
    ///    a signature verification (F3);
    /// 2. as the AUTHORITATIVE check inside [`Self::try_insert`], under the
    ///    write lock that actually admits the attestation.
    ///
    /// Both are required. The lock is not held between them, so the early probe
    /// is only an optimization and can never be the sole gate. Because they call
    /// THIS function with THIS attestation — not two transcriptions of the same
    /// rule — they cannot disagree about whether the pool is full or about which
    /// [`PoolFull`] refused. That is the divergence class S3.3's B1 finding was.
    ///
    /// # Replacements
    ///
    /// A replacement (same asset, same `(cell, mirror)`) is not growth of the
    /// SET, so neither count guard applies to it. It can still be growth in
    /// BYTES — a mirror re-attesting with a longer `spine_point` — so the byte
    /// budget is judged against the net change, releasing what the incumbent
    /// charged and charging what the newcomer weighs.
    pub fn admission_check(&self, attestation: &MirrorAttestation) -> Result<(), PoolFull> {
        let key = (attestation.matrix_index, attestation.mirror.clone());

        // Probe WITHOUT creating the asset's slot: a refused attestation must
        // not leave an empty set behind (which would itself be an unbounded-key
        // primitive keyed by an attacker-chosen asset hash).
        let existing = self.by_asset.get(&attestation.asset_hash);
        let incumbent = existing.and_then(|set| set.get(&key));

        if incumbent.is_none() {
            if existing.map_or(0, BTreeMap::len) >= MAX_ATTESTATIONS_PER_ASSET {
                return Err(PoolFull::Asset {
                    limit: MAX_ATTESTATIONS_PER_ASSET,
                });
            }
            if self.total >= MAX_TOTAL_ATTESTATIONS {
                return Err(PoolFull::Global {
                    limit: MAX_TOTAL_ATTESTATIONS,
                });
            }
        }

        let released = incumbent.map_or(0, attestation_footprint_bytes);
        let incoming = attestation_footprint_bytes(attestation);
        let held = self.bytes_held.saturating_sub(released);
        if held.saturating_add(incoming) > MAX_ATTESTATION_POOL_BYTES {
            return Err(PoolFull::Bytes {
                held,
                incoming,
                budget: MAX_ATTESTATION_POOL_BYTES,
            });
        }
        Ok(())
    }

    /// Record a VERIFIED attestation, replacing any prior attestation from the
    /// same mirror at the same matrix cell.
    ///
    /// Returns `Ok(displaced)` — the attestation this one replaced, if any — or
    /// [`PoolFull`] when the admission would exceed a bound, as judged by
    /// [`Self::admission_check`].
    ///
    /// The caller must have run [`verify_attestation`] first —
    /// [`NodeBlockchain::record_mirror_attestation`] is the gate that
    /// guarantees it.
    pub fn try_insert(
        &mut self,
        attestation: MirrorAttestation,
    ) -> Result<Option<MirrorAttestation>, PoolFull> {
        // The authoritative gate. Read-only, so a refusal leaves the pool
        // untouched and its accounting trivially intact.
        self.admission_check(&attestation)?;

        let key = (attestation.matrix_index, attestation.mirror.clone());
        let incoming = attestation_footprint_bytes(&attestation);
        let displaced = self
            .by_asset
            .entry(attestation.asset_hash)
            .or_default()
            .insert(key, attestation);

        let released = displaced.as_ref().map_or(0, attestation_footprint_bytes);
        if displaced.is_none() {
            self.total = self.total.saturating_add(1);
        }
        self.bytes_held = self
            .bytes_held
            .saturating_sub(released)
            .saturating_add(incoming);
        self.debug_assert_accounting();
        Ok(displaced)
    }

    /// Live attestations held across every asset.
    pub fn total(&self) -> usize {
        self.total
    }

    /// F1 — charged bytes across every held attestation, against
    /// [`MAX_ATTESTATION_POOL_BYTES`].
    pub fn bytes_held(&self) -> usize {
        self.bytes_held
    }

    /// Recompute `total` from the attestations actually held — the definition
    /// the incremental counter is a cache of.
    pub fn recomputed_total(&self) -> usize {
        self.by_asset
            .values()
            .map(BTreeMap::len)
            .fold(0usize, usize::saturating_add)
    }

    /// Recompute the byte accounting from the attestations actually held — the
    /// definition `bytes_held` is a cache OF.
    pub fn recomputed_bytes(&self) -> usize {
        self.by_asset
            .values()
            .flat_map(BTreeMap::values)
            .map(attestation_footprint_bytes)
            .fold(0usize, usize::saturating_add)
    }

    /// A6 — tie `total` and `bytes_held` to their definitions.
    ///
    /// Both are private fields maintained incrementally so the bounds are O(1);
    /// today only [`Self::try_insert`] and [`Self::clear_asset`] mutate
    /// `by_asset`, and both maintain them. Nothing in the type SAID so. This
    /// assertion says so: a future prune or expiry that touches `by_asset`
    /// directly aborts here in debug and test builds instead of silently
    /// desyncing [`MAX_TOTAL_ATTESTATIONS`] or
    /// [`MAX_ATTESTATION_POOL_BYTES`] — which, being bounds on
    /// attacker-supplied input, are bounds that must not be able to drift
    /// upward unobserved. Compiled out of release builds; the walk is O(n).
    #[inline]
    fn debug_assert_accounting(&self) {
        debug_assert_eq!(
            self.total,
            self.recomputed_total(),
            "MirrorAttestationPool.total desynced from the attestations held — a mutation \
             bypassed try_insert()/clear_asset()"
        );
        debug_assert_eq!(
            self.bytes_held,
            self.recomputed_bytes(),
            "MirrorAttestationPool.bytes_held desynced from the attestations held — a \
             mutation bypassed try_insert()/clear_asset()"
        );
    }

    /// Every attestation held for `asset_hash`, in canonical matrix order.
    pub fn attestations_for(&self, asset_hash: &[u8; 32]) -> Vec<MirrorAttestation> {
        self.by_asset
            .get(asset_hash)
            .map(|set| set.values().cloned().collect())
            .unwrap_or_default()
    }

    /// How many mirrors have attested to `asset_hash`.
    pub fn count_for(&self, asset_hash: &[u8; 32]) -> usize {
        self.by_asset.get(asset_hash).map_or(0, BTreeMap::len)
    }

    /// The attestation from `mirror` at `cell` for `asset_hash`, if held.
    pub fn attestation_by(
        &self,
        asset_hash: &[u8; 32],
        cell: MatrixIndex,
        mirror: &str,
    ) -> Option<&MirrorAttestation> {
        self.by_asset
            .get(asset_hash)
            .and_then(|set| set.get(&(cell, mirror.to_string())))
    }

    /// Number of distinct assets with at least one attestation.
    pub fn asset_count(&self) -> usize {
        self.by_asset.len()
    }

    /// Drop every attestation for `asset_hash` — the asset left this container.
    /// Returns how many were dropped.
    ///
    /// # The reclaim path, and why it is not eviction (F1)
    ///
    /// This is the explicit counterpart to refuse-at-capacity: pool space is
    /// released by a LOCAL decision about local state, never by an attacker's
    /// traffic. It is reached from
    /// [`NodeBlockchain::clear_mirror_attestations`] (an operator/host
    /// decision) and from
    /// [`NodeBlockchain::forget_received_asset_chain`], which is the one place
    /// where an asset genuinely stops being held: once the received chain is
    /// gone, `holds_asset` is false, so
    /// [`accept_wire_attestation`](NodeBlockchain::accept_wire_attestation)
    /// would refuse a NEW attestation for it — keeping the old ones would charge
    /// the budget for statements about an asset we no longer have.
    ///
    /// Nothing remote can trigger either caller, so this cannot become the
    /// deletion primitive an eviction policy would be.
    pub fn clear_asset(&mut self, asset_hash: &[u8; 32]) -> usize {
        let Some(set) = self.by_asset.remove(asset_hash) else {
            return 0;
        };
        let dropped = set.len();
        let released = set
            .values()
            .map(attestation_footprint_bytes)
            .fold(0usize, usize::saturating_add);
        self.total = self.total.saturating_sub(dropped);
        self.bytes_held = self.bytes_held.saturating_sub(released);
        self.debug_assert_accounting();
        dropped
    }
}

/// What an owner's seal produced.
#[derive(Clone, Debug)]
pub struct MirrorSealReceipt {
    /// The checkpoint block appended to the spine, carrying the seal inside the
    /// entry's `state_proof`.
    pub block: Block,
    /// The seal itself — BLAKE3 root, cardinality, sealing identity.
    pub seal: MirrorSeal,
    /// The exact attestation set the root commits to, in canonical matrix
    /// order. Whole-set verification
    /// ([`hypermesh_lib::attestation::verify_sealed_set`]) requires this set and
    /// is therefore a local-session capability; the durable way to open the seal
    /// is a per-attestation [`MembershipProof`] minted here — see
    /// [`membership_proof`](Self::membership_proof) (F1).
    pub attestations: Vec<MirrorAttestation>,
}

impl MirrorSealReceipt {
    /// Mint the witness a mirror keeps so it can prove — after any restart, and
    /// without anyone retaining the sealed set — that it was inside this seal.
    ///
    /// The seal root is a Merkle root, so the witness is one leaf plus an
    /// `O(log n)` path; the verifier folds it with
    /// [`verify_membership`](hypermesh_lib::attestation::verify_membership)
    /// against the on-chain [`MirrorSeal`] alone.
    ///
    /// Returns `None` if `attestation` is not in the sealed set.
    pub fn membership_proof(&self, attestation: &MirrorAttestation) -> Option<MembershipProof> {
        hypermesh_lib::attestation::membership_proof(&self.attestations, attestation)
    }
}

/// The ONE place a [`PoolFull`] becomes a logged refusal and an error string.
///
/// Both capacity sites — the F3 early probe and the authoritative
/// [`MirrorAttestationPool::try_insert`] — funnel through here, so a refusal
/// reads identically whichever one produced it and no capacity rejection path
/// can be added without a `warn!`. `warn` and not `debug`: `main.rs` caps the
/// default subscriber at INFO, and a refusal nobody sees is a refusal nobody can
/// diagnose.
fn refuse_pool_full(attestation: &MirrorAttestation, full: &PoolFull) -> String {
    warn!(
        mirror = %attestation.mirror,
        cell = %attestation.matrix_index,
        asset = %&hex::encode(attestation.asset_hash)[..16],
        "S3.4: mirror attestation not recorded — {full}"
    );
    format!("mirror attestation not recorded: {full}")
}

impl NodeBlockchain {
    /// Record a mirror's attestation for an asset — OFF-SPINE.
    ///
    /// Verifies the attestation with [`verify_attestation`] — which is the FULL
    /// audit gate ([`MirrorAttestation::is_structurally_valid`]) plus FALCON, so
    /// nothing that a later `verify_sealed_set` would reject can ever be
    /// recorded (B1) — then files it by `(matrix cell, mirror)`. This writes NO
    /// block and
    /// touches NO asset's `prev_asset_entry` / `asset_seq`: the title is
    /// unchanged, which is what lets any number of mirrors attest
    /// concurrently.
    ///
    /// The attestation's `spine_point` is NOT required to be the asset's
    /// current head — mirrors legitimately attest to the point they validated,
    /// and the head may have moved since. Staleness is visible (`spine_point` /
    /// `spine_seq` are both carried) and is a policy question for the sealer,
    /// not an accept gate here.
    ///
    /// # Order of judgement (F3): capacity BEFORE the signature
    ///
    /// At steady-state capacity every junk submission used to cost a full
    /// FALCON-1024 verification before the bound was consulted — a
    /// CPU-exhaustion primitive reachable from the wire. Capacity is now probed
    /// first, under a read lock, and the probe is not a second rule: it calls
    /// [`MirrorAttestationPool::admission_check`], the same function
    /// [`MirrorAttestationPool::try_insert`] calls authoritatively under the
    /// write lock. The lock is not held between them, so the probe is only an
    /// optimization — never the gate — and a REPLACEMENT passes it, because a
    /// replacement is not growth.
    pub async fn record_mirror_attestation(
        &self,
        attestation: MirrorAttestation,
    ) -> Result<(), String> {
        // (1) F3 — capacity, before any FALCON work.
        if let Err(full) = self
            .mirror_attestations
            .read()
            .await
            .admission_check(&attestation)
        {
            return Err(refuse_pool_full(&attestation, &full));
        }

        // (2) the one audit gate, plus FALCON-1024.
        if !verify_attestation(&attestation) {
            warn!(
                mirror = %attestation.mirror,
                cell = %attestation.matrix_index,
                "S3.3: rejected mirror attestation — structural validity (B1: the \
                 full audit gate) or FALCON signature failed"
            );
            return Err(
                "mirror attestation rejected: it failed the audit gate \
                 (MirrorAttestation::is_structurally_valid — non-empty mirror, \
                 spine_point, pubkey and signature; envelope covers these fields; \
                 hex(BLAKE3(signer_pubkey)) == mirror) or its FALCON-1024 signature"
                    .to_string(),
            );
        }

        // (3) S3.4: the pool is bounded, and THIS is the authoritative gate —
        // the probe at (1) ran without the write lock and can have gone stale.
        // Same rule, same function, so the two cannot name different bounds.
        let displaced = self
            .mirror_attestations
            .write()
            .await
            .try_insert(attestation.clone())
            .map_err(|full| refuse_pool_full(&attestation, &full))?;

        info!(
            mirror = %attestation.mirror,
            cell = %attestation.matrix_index,
            replaced = displaced.is_some(),
            "S3.3: recorded mirror attestation off-spine"
        );
        Ok(())
    }

    /// S3.4 — the accept path for an attestation that arrived OVER THE WIRE.
    ///
    /// One admission rule on top of
    /// [`record_mirror_attestation`](Self::record_mirror_attestation), and NOT a
    /// second list of verification checks: **we only cache statements about
    /// assets this container holds** — on its own spine, or as a received
    /// asset-chain adopted by
    /// [`accept_asset_chain`](Self::accept_asset_chain).
    ///
    /// The pool is keyed by a 32-byte asset hash that a remote sender chooses
    /// freely. Without this rule the key space belongs to the sender, and the
    /// global pool bound becomes a lever rather than a protection: fill it with
    /// statements about assets nobody has, and honest attestations for real
    /// assets get refused. Tying the key space to assets we actually hold makes
    /// the bound a function of local state.
    ///
    /// It is also the cheap gate: an unknown asset is refused by a map probe,
    /// before any FALCON-1024 verification is attempted.
    ///
    /// Everything else — envelope structure, identity binding, the signature
    /// itself, the pool bound — is [`verify_attestation`] and the pool, reached
    /// by delegation. Nothing is restated here.
    pub async fn accept_wire_attestation(
        &self,
        attestation: MirrorAttestation,
    ) -> Result<(), String> {
        // S3.5: `holds_asset` spans the block chain AND adopted received chains, so an
        // attestation about a RECEIVED-only asset is admitted here and consumes
        // the same global budget from which block-chain assets' attestations are
        // sealed. Received-accepted material can therefore influence which
        // evidence reaches the block chain — a coupling NOT covered by the
        // "received chains are not in AssetChainIndex / authorizes_shard"
        // exclusion, which is about lineage authority rather than a shared bound.
        //
        // The gate S3.5 will need: the pool must budget block-chain-held and
        // received-held assets SEPARATELY, so received material cannot crowd out
        // attestations for assets this container actually owns. It is deferred
        // rather than added now because Part A (accept_asset_chain) has
        // no wire caller — no remote input can reach the received side of
        // `holds_asset` in the shipped binary — so the coupling is real but
        // unreachable, and splitting the budget is a policy change that must be
        // designed with the cross-scope transfer path rather than bolted on
        // under a memory-bound fix.
        if !self.holds_asset(&attestation.asset_hash).await {
            return Err(format!(
                "mirror attestation not cached: this container holds no asset {} \
                 (neither on its block chain nor as an adopted received chain)",
                &hex::encode(attestation.asset_hash)[..16],
            ));
        }
        self.record_mirror_attestation(attestation).await
    }

    /// Every attestation accumulated for `asset_hash`, in canonical matrix
    /// order (the order [`seal_root`](hypermesh_lib::attestation::seal_root)
    /// folds).
    pub async fn mirror_attestations(&self, asset_hash: &[u8; 32]) -> Vec<MirrorAttestation> {
        self.mirror_attestations
            .read()
            .await
            .attestations_for(asset_hash)
    }

    /// How many mirrors have attested to `asset_hash`.
    pub async fn mirror_attestation_count(&self, asset_hash: &[u8; 32]) -> usize {
        self.mirror_attestations.read().await.count_for(asset_hash)
    }

    /// S3.4 — live attestations held across every asset. This is the quantity
    /// [`MAX_TOTAL_ATTESTATIONS`] bounds.
    pub async fn mirror_attestation_total(&self) -> usize {
        self.mirror_attestations.read().await.total()
    }

    /// S3.4/F1 — charged bytes of mirror-attestation material held off-spine,
    /// against [`MAX_ATTESTATION_POOL_BYTES`]. This is the quantity that binds.
    pub async fn mirror_attestation_bytes(&self) -> usize {
        self.mirror_attestations.read().await.bytes_held()
    }

    /// S3.4/F1 — release every pooled attestation for `asset_hash`; returns how
    /// many were dropped.
    ///
    /// The explicit counterpart to refuse-at-capacity, and the reason
    /// [`MirrorAttestationPool::clear_asset`] is reachable at all: pool space is
    /// reclaimed by a LOCAL decision, never by an attacker's traffic. Nothing on
    /// the wire path calls this.
    ///
    /// # Why NOT at seal time
    ///
    /// Draining the pool when the owner seals would also reclaim, and the seal
    /// is durable on-chain — but it would silently change what a seal MEANS. A
    /// seal is a cumulative snapshot of an asset's whole mirror set
    /// ([`seal_mirror_attestations`](Self::seal_mirror_attestations)); drain it
    /// and every later seal commits only to mirrors that arrived since the last
    /// one, so a verifier reading the newest seal sees a shrinking mirror set
    /// and an honest mirror sealed once disappears from the record thereafter.
    /// Changing the semantics of an on-chain, hash-committed root is not
    /// something a memory bound gets to do. The byte budget is what makes the
    /// pool bounded; this is the release valve.
    pub async fn clear_mirror_attestations(&self, asset_hash: &[u8; 32]) -> usize {
        self.mirror_attestations.write().await.clear_asset(asset_hash)
    }

    /// Seal the accumulated mirror attestations for `asset_hash` into the
    /// spine as an owner checkpoint.
    ///
    /// Steps:
    /// 1. **Owner gate** — `sealed_by` must hold the distribution right in the
    ///    asset's head-entry `AuthorizationSet`.
    /// 2. **Self gate** — on a chain with a signer, `sealed_by` must be THIS
    ///    node's identity. You seal as yourself; otherwise the H3 envelope
    ///    (signed by us) would assert someone else's WHO and peers would
    ///    reject the block at `signer_binds_to_author`.
    /// 3. **Re-verify** every pooled attestation (the pool only holds verified
    ///    ones; re-verifying makes the sealed set self-evidently sound).
    /// 4. Compute the [`MirrorSeal`] over the set in canonical MATRIX order and
    ///    write it into the checkpoint entry's `state_proof`, from which
    ///    `proof_hash` — and therefore the block hash — commits to it.
    ///
    /// The checkpoint entry is an ordinary spine entry: `add_block` stamps its
    /// lineage as the asset's next `asset_seq`, so the title stays a
    /// single-parent, gap-free chain. Attestations are NOT drained — verifying
    /// a seal requires the set it committed to, and a later seal is a fresh
    /// cumulative snapshot.
    pub async fn seal_mirror_attestations(
        &self,
        asset_hash: &[u8; 32],
        sealed_by: &str,
    ) -> Result<MirrorSealReceipt, String> {
        let head = self.asset_head_entry(asset_hash).await.ok_or_else(|| {
            format!(
                "cannot seal mirror attestations: this container holds no entry for asset {}",
                &hex::encode(asset_hash)[..16]
            )
        })?;

        self.check_seal_authority(&head, sealed_by)?;

        let attestations = self.mirror_attestations(asset_hash).await;
        for (position, attestation) in attestations.iter().enumerate() {
            if !verify_attestation(attestation) {
                return Err(format!(
                    "cannot seal: pooled attestation {position} (mirror {}) no longer verifies",
                    attestation.mirror,
                ));
            }
        }

        let seal = build_seal(sealed_by, &attestations);
        let entry = build_seal_entry(&head, sealed_by, seal.clone());
        let block = self.add_block(vec![entry]).await?;

        info!(
            asset = %&hex::encode(asset_hash)[..16],
            count = seal.count,
            root = %&seal.root[..16.min(seal.root.len())],
            block = block.index,
            "S3.3: sealed mirror attestations into the spine"
        );

        Ok(MirrorSealReceipt { block, seal, attestations })
    }

    /// Verify a previously sealed set: does `attestations` still hash to the
    /// root the owner committed to?
    ///
    /// A stripped, added or altered attestation changes the recomputed root and
    /// is reported as [`SealBreak::RootMismatch`] (or `CountMismatch` for a
    /// cardinality change).
    pub fn verify_sealed_attestations(
        asset_hash: &[u8; 32],
        attestations: &[MirrorAttestation],
        seal: &MirrorSeal,
    ) -> Result<(), SealBreak> {
        hypermesh_lib::attestation::verify_sealed_set(asset_hash, attestations, seal)
    }

    /// The most recent seal on `asset_hash`'s spine, with the entry carrying
    /// it — the on-chain, hash-committed mirror history.
    pub async fn latest_mirror_seal(
        &self,
        asset_hash: &[u8; 32],
    ) -> Option<(BlockAssetEntry, MirrorSeal)> {
        self.asset_history_entries(asset_hash)
            .await
            .into_iter()
            .rev()
            .find_map(|entry| {
                entry
                    .state_proof
                    .mirror_seal
                    .clone()
                    .map(|seal| (entry, seal))
            })
    }

    /// The asset's current head ENTRY (not just its locator).
    async fn asset_head_entry(&self, asset_hash: &[u8; 32]) -> Option<BlockAssetEntry> {
        let locator = self.asset_head(asset_hash).await?;
        self.entry_at(&locator).await
    }

    /// The owner gate, plus the "seal as yourself" gate.
    ///
    /// **S3.5 TODO (explicit, not a silent allow-anyone):** every
    /// `AssetRegistration` constructor currently hardcodes
    /// `AuthorizationSet::default()` and `with_owner` has zero production
    /// callers, so a daemon-created asset is OWNERLESS today. That case is
    /// REJECTED here with a labelled error rather than defaulting to "anyone
    /// may seal" — S3.5 sets ownership at creation, after which this gate
    /// starts passing for real owners.
    fn check_seal_authority(&self, head: &BlockAssetEntry, sealed_by: &str) -> Result<(), String> {
        if sealed_by.is_empty() {
            return Err("cannot seal: sealer identity is empty".to_string());
        }

        let authorization = &head.registration.authorization;
        if authorization.owners.is_empty() {
            return Err(
                "cannot seal mirror attestations: this asset has NO owner. Every \
                 AssetRegistration constructor still hardcodes AuthorizationSet::default() \
                 and `with_owner` has no production caller — setting ownership at creation \
                 is S3.5. Failing closed rather than letting anyone seal."
                    .to_string(),
            );
        }
        if !authorization.is_owner(sealed_by) {
            return Err(format!(
                "cannot seal mirror attestations: {sealed_by} does not hold the distribution \
                 right for this asset — only an OWNER may seal"
            ));
        }

        // Seal as yourself: our H3 envelope signs the checkpoint entry, and a
        // peer binds hex(BLAKE3(signer_pubkey)) to the entry's claimed author.
        if let Some(signer) = self.signer.as_ref() {
            let ours = blake3::hash(signer.public_key_bytes()).to_hex().to_string();
            if ours != sealed_by {
                return Err(format!(
                    "cannot seal mirror attestations as {sealed_by}: this node signs as {ours} \
                     — a seal must be signed by the owner who issues it"
                ));
            }
        }

        Ok(())
    }
}

/// Domain separator for the checkpoint's derived time-proof nonce (F2). A
/// nonce derived under this tag can never collide with one derived for any
/// other purpose, and never with the wall-clock nanos `TimeProof::new` uses.
const SEAL_TIME_NONCE_DOMAIN: &[u8] = b"HYPERMESH/S3.3/SEAL-CHECKPOINT-TIME-NONCE/v1";

/// The checkpoint's own WHEN proof (F2).
///
/// # Why a derived nonce and not a fresh clock read
///
/// `build_seal_entry` used to clone the head's [`TimeProof`] verbatim, so the
/// checkpoint and the head carried an IDENTICAL nonce — two distinct spine
/// entries asserting the same replay-freshness token, which is precisely what a
/// nonce exists to prevent. The nonce is now derived from the SEAL ITSELF
/// (`asset_hash || root || count || sealed_by`), which makes it:
/// * **distinct** from the head's, and from any other seal's, and
/// * **reproducible** — anyone holding the same seal derives the same nonce, so
///   the seal path stays a pure function of its inputs (the S3.0/B2 determinism
///   seam). A `SystemTime::now()` here would have re-introduced exactly the
///   wall-clock read S3.0 removed.
///
/// # Why the timestamp is inherited, not invented
///
/// A timestamp is a claim about the world. The only wall clock we could consult
/// is the local one, and reading it is what the determinism seam forbids — so
/// we do NOT fabricate a fresh WHEN we cannot defend. The checkpoint carries
/// forward the asset's temporal anchor and states its real temporal position
/// through the spine: `add_block` stamps this entry as the asset's next
/// `asset_seq`, and S3.2 makes that sequence gap-free and single-parent. The
/// authoritative "this seal came after that entry" is the lineage, not a
/// self-reported clock — which is the same reason HyperMesh orders mirror
/// attestations by MATRIX INDEX rather than by time.
///
/// [`TimeProof`]: hypermesh_lib::proof::TimeProof
fn seal_time_proof(
    head: &hypermesh_lib::proof::TimeProof,
    asset_hash: &[u8; 32],
    seal: &MirrorSeal,
) -> hypermesh_lib::proof::TimeProof {
    let mut hasher = blake3::Hasher::new();
    hasher.update(SEAL_TIME_NONCE_DOMAIN);
    hasher.update(asset_hash);
    hasher.update(seal.root.as_bytes());
    hasher.update(&seal.count.to_le_bytes());
    hasher.update(seal.sealed_by.as_bytes());
    let digest = hasher.finalize();
    let mut nonce_bytes = [0u8; 8];
    nonce_bytes.copy_from_slice(&digest.as_bytes()[..8]);
    // `TimeProof::is_structurally_valid` accepts any nonce; `max(1)` matches
    // `TimeProof::new`'s "never zero" convention so a derived nonce is never
    // mistaken for an unset one.
    let nonce = u64::from_le_bytes(nonce_bytes).max(1);

    hypermesh_lib::proof::TimeProof::new_at(
        head.network_time_offset,
        head.time_verification_timestamp,
        nonce,
    )
}

/// Build the checkpoint entry that carries `seal` into the spine.
///
/// Reuses the asset's HEAD entry as the template: same registration, same
/// storage pointer, same WHERE/WHAT proofs (a seal moves no data and does no new
/// work — its content correlation is unchanged). WHO is re-stamped to the
/// sealer, so the entry's claimed author is the identity that signs it, and
/// WHEN gets its own seal-derived nonce — see [`seal_time_proof`] (F2).
///
/// Lineage is deliberately left unset: `add_block`'s S3.2 stamper overwrites
/// caller-supplied lineage under the head reservation, and re-derives
/// `proof_hash` and the H3 signature after doing so.
fn build_seal_entry(
    head: &BlockAssetEntry,
    sealed_by: &str,
    seal: MirrorSeal,
) -> BlockAssetEntry {
    let mut proof = head.state_proof.clone();
    proof.stake_proof.stake_holder_id = sealed_by.to_string();
    proof.time_proof = seal_time_proof(&head.state_proof.time_proof, &head.asset_hash, &seal);
    proof.mirror_seal = Some(seal);
    proof.prev_asset_entry = None;
    proof.asset_seq = 0;

    BlockAssetEntry::new_bound(
        head.asset_hash,
        &proof,
        head.storage_pointer.clone(),
        head.registration.clone(),
    )
}
