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
/// One mirror at one cell has exactly one live attestation, so this is a cap on
/// distinct `(matrix cell, mirror)` attestors per asset. FALCON keypairs are
/// free to mint, so identity count is not a scarce resource and this bound is
/// what stops one asset's mirror set from growing without limit.
pub const MAX_ATTESTATIONS_PER_ASSET: usize = 256;

/// S3.4 — maximum live attestations held across all assets.
///
/// The backstop for the per-asset cap: without it, an attacker holding N assets
/// could still reach `N × MAX_ATTESTATIONS_PER_ASSET`. The wire surface
/// additionally refuses attestations for assets this container does not hold,
/// which is what keeps N itself bounded by real local state rather than by
/// attacker-chosen hashes.
pub const MAX_TOTAL_ATTESTATIONS: usize = 8192;

/// Why a verified attestation could not be recorded.
///
/// Distinct from a verification failure on purpose: the attestation is sound,
/// there is simply no room. Callers log it differently, and a mirror can retry.
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
/// unbounded map fed by strangers is a memory-exhaustion primitive. It is
/// therefore capped per asset ([`MAX_ATTESTATIONS_PER_ASSET`]) and globally
/// ([`MAX_TOTAL_ATTESTATIONS`]), and at capacity a NEW attestor is REFUSED.
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
}

impl MirrorAttestationPool {
    /// Empty pool.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a VERIFIED attestation, replacing any prior attestation from the
    /// same mirror at the same matrix cell.
    ///
    /// Returns `Ok(displaced)` — the attestation this one replaced, if any — or
    /// [`PoolFull`] when admitting a NEW attestor would exceed a bound. A
    /// replacement is always admitted: it does not grow the set.
    ///
    /// The caller must have run [`verify_attestation`] first —
    /// [`NodeBlockchain::record_mirror_attestation`] is the gate that
    /// guarantees it.
    pub fn try_insert(
        &mut self,
        attestation: MirrorAttestation,
    ) -> Result<Option<MirrorAttestation>, PoolFull> {
        let key = (attestation.matrix_index, attestation.mirror.clone());

        // Judge capacity WITHOUT creating the asset's slot: a refused
        // attestation must not leave an empty set behind (which would itself be
        // an unbounded-key primitive keyed by an attacker-chosen asset hash).
        let existing = self.by_asset.get(&attestation.asset_hash);
        let is_replacement = existing.is_some_and(|set| set.contains_key(&key));
        if !is_replacement {
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
            self.total += 1;
        }

        Ok(self
            .by_asset
            .entry(attestation.asset_hash)
            .or_default()
            .insert(key, attestation))
    }

    /// Live attestations held across every asset.
    pub fn total(&self) -> usize {
        self.total
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

    /// Drop every attestation for `asset_hash` (e.g. the asset left this
    /// container). Returns how many were dropped.
    pub fn clear_asset(&mut self, asset_hash: &[u8; 32]) -> usize {
        let dropped = self.by_asset.remove(asset_hash).map_or(0, |set| set.len());
        self.total = self.total.saturating_sub(dropped);
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
    pub async fn record_mirror_attestation(
        &self,
        attestation: MirrorAttestation,
    ) -> Result<(), String> {
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

        // S3.4: the pool is bounded. A verified attestation can still be
        // refused for want of room — see [`MirrorAttestationPool`] for why the
        // answer is refuse-the-newcomer rather than evict-an-incumbent.
        let displaced = self
            .mirror_attestations
            .write()
            .await
            .try_insert(attestation.clone())
            .map_err(|full| {
                warn!(
                    mirror = %attestation.mirror,
                    cell = %attestation.matrix_index,
                    "S3.4: mirror attestation verified but not recorded — {full}"
                );
                format!("mirror attestation not recorded: {full}")
            })?;

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
    /// assets this container holds** — on its own spine, or as a foreign
    /// asset-chain adopted by
    /// [`accept_foreign_asset_chain`](Self::accept_foreign_asset_chain).
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
        if !self.holds_asset(&attestation.asset_hash).await {
            return Err(format!(
                "mirror attestation not cached: this container holds no asset {} \
                 (neither on its spine nor as an adopted foreign chain)",
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
