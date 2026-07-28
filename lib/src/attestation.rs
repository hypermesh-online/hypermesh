// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! S3.3 — MIRROR ATTESTATIONS: matrix-indexed, sealed by the owner.
//!
//! # The model (locked, D1)
//!
//! An asset's chain is a **linear title** extended only by the holder of the
//! distribution right — create → grant → transfer. That spine is S3.2
//! ([`StateProof::prev_asset_entry`](crate::proof::StateProof::prev_asset_entry)
//! + `asset_seq`), single-parent, and it is NOT what this module touches.
//!
//! Everyone hosting an asset — via its blocks and shards — is a MIRROR
//! (VISION §4: *access IS mirroring*). Each mirror's "I hold and validated
//! this" statement is a [`MirrorAttestation`], and attestations accumulate
//! **OFF the spine**. They never consume a `prev_asset_entry` / `asset_seq`
//! slot, so N mirrors attesting concurrently cannot fork or renumber the title.
//!
//! # Why matrix index, not time
//!
//! *"Think about seeds mirroring on torrent, and how blocks actually hold an
//! index in the matrix."* Attestations are ordered by **matrix position**
//! ([`MatrixIndex`]) — the mirror set is organized **spatially**, not
//! temporally. This is the load-bearing design constraint: a total order over
//! integer lattice points needs no trusted clock, no Lamport counter, and no
//! vector clock. Two mirrors that attest "simultaneously" still have exactly
//! one canonical order, because they occupy different cells of the matrix
//! (and, within a cell, different identities).
//!
//! Consequently [`seal_root`] is a pure function of the SET, not of arrival
//! order: shuffle the input and the root is byte-identical.
//!
//! # The owner's seal
//!
//! Accumulated attestations become durable when the OWNER seals them: a
//! [`MirrorSeal`] (BLAKE3 **Merkle** root over the canonically-ordered set + its
//! cardinality + who sealed it) is written into the `StateProof` body of one
//! checkpoint entry on the spine. The root is a Merkle root rather than a flat
//! fold so that a mirror can OPEN the commitment with its own leaf and path
//! ([`membership_proof`] / [`verify_membership`]) — the on-chain footprint is
//! identical, but the witness travels with the claimant instead of requiring
//! every verifier to retain the full set. Because `proof_hash =
//! BLAKE3(serialize(state_proof))` and `Block::calculate_hash` folds
//! `(asset_hash || proof_hash)`, the seal is hash-committed transitively and
//! FALCON-signed by H3's envelope — with **zero `Block` / `BlockAssetEntry`
//! field change** and **no multi-parent merge**. The spine stays single-parent;
//! mirror history becomes auditable.
//!
//! # Why this cannot ride on `signed_proof`
//!
//! H3's `signed_proof` is `Option<WireSignedProof>` — cardinality **1**, while
//! a title needs N signers; the block-accept gate `signer_binds_to_author`
//! **requires signer == author**, so a third-party signature makes the block
//! REJECT; it signs `json(state_proof)` ("I attest to my own PoS"), not "I
//! attest to this asset at this point"; and it is excluded from
//! `calculate_hash`, so an unchained signature could be stripped undetected.
//! The correct template is `verify_grant` — structural bind + FALCON verify +
//! binding to a **named third party** — and that is what this module follows.
//!
//! # Layering
//!
//! Types and the structural/binding half of verification live here (`lib` is
//! *the types*, VISION §3). FALCON verification lives in BlockMatrix
//! (`blockchain::attestations::verify_attestation`), exactly as `Grant`'s
//! structural half lives here and `verify_grant`'s crypto half lives there —
//! `lib` carries no post-quantum crypto dependency.

use serde::{Deserialize, Serialize};

use crate::proof::WireSignedProof;

/// Domain separator for attestation canonical bytes. A FALCON signature over
/// these bytes can never be replayed as a signature over a `Grant`, a
/// `StateProof`, or a future attestation format.
pub const ATTESTATION_DOMAIN: &[u8] = b"HYPERMESH/S3.3/MIRROR-ATTESTATION/v1";

/// Domain separator for a single attestation's commitment (the leaf folded
/// into a [`MirrorSeal`] root).
pub const ATTESTATION_COMMITMENT_DOMAIN: &[u8] = b"HYPERMESH/S3.3/ATTESTATION-COMMITMENT/v1";

/// S3.4/F1 — largest accepted [`MirrorAttestation::spine_point`], in bytes.
///
/// # Why this field needs a cap at all
///
/// `spine_point` is the ONLY unbounded attacker-chosen field an attestation
/// carries. Everything else is already pinned:
///
/// * `asset_hash`, `matrix_index`, `spine_seq` — fixed width;
/// * `mirror` — [`binds_to_signer`](MirrorAttestation::binds_to_signer) forces
///   it to equal `hex(BLAKE3(pubkey))`, i.e. exactly 64 bytes;
/// * `signature.signer_pubkey` / `signature.signature` — a FALCON-1024 key and
///   detached signature, or the signature does not verify;
/// * `signature.proof_bytes` — must equal
///   [`my_canonical_bytes`](MirrorAttestation::my_canonical_bytes), so it is a
///   function of the fields above **plus `spine_point`**.
///
/// So `spine_point` sets the size of the whole object, and it appears in it
/// TWICE (the field, and again inside `proof_bytes`). Uncapped, an attestor
/// signs a 13 KiB `spine_point` itself — signing imposes no size limit — and
/// each pool slot costs ~30 KiB instead of ~4 KiB.
///
/// # The number, and why nothing honest is near it
///
/// A `spine_point` is a `lineage_id`: `hex(proof_hash)` of the entry the mirror
/// validated (BlockMatrix `BlockAssetEntry::lineage_id`). `proof_hash` is a
/// BLAKE3 digest, so the ONLY value any producer emits is **64 bytes** of
/// lowercase hex. 256 is four times that — enough hex for a 128-byte digest,
/// which is twice the width BLAKE3 or any successor in this codebase produces.
/// Honest material cannot approach it; the 13,400-byte attack cannot come near
/// passing.
///
/// # This is a bound on network input, NOT a proof term
///
/// It gates admission of a third party's statement into local memory. It is not
/// consulted by any [`StateProof`](crate::proof::StateProof), carries no
/// magnitude, and no authorization decision reads it.
pub const MAX_SPINE_POINT_BYTES: usize = 256;

/// Domain separator for the seal root over an ordered attestation set.
pub const SEAL_ROOT_DOMAIN: &[u8] = b"HYPERMESH/S3.3/MIRROR-SEAL-ROOT/v1";

/// Prefix byte distinguishing a Merkle LEAF from an internal node, so no leaf
/// commitment can ever be reinterpreted as a subtree hash (second-preimage
/// separation — the classic flat-hash Merkle mistake).
const MERKLE_LEAF_TAG: u8 = 0x00;

/// Prefix byte for an internal Merkle node.
const MERKLE_NODE_TAG: u8 = 0x01;

// ---------------------------------------------------------------------------
// MatrixIndex — the spatial ordering key
// ---------------------------------------------------------------------------

/// Integer index of a cell in the Block-MATRIX — the **spatial ordering key**
/// for mirror attestations.
///
/// Deliberately distinct from [`MatrixPosition`](crate::types::MatrixPosition),
/// which is `f64` geospatial data: floats have no total order (`NaN`) and no
/// stable equality, so they cannot key a deterministic, hash-committed set.
/// An index into a lattice is an integer.
///
/// BlockMatrix's `matrix::coordinate::MatrixCoordinate` is the same `(i64,i64,i64)`
/// shape and converts into this via `From`; it lives in BlockMatrix with a
/// large surface of tensor/routing extensions. Hoisting that whole type into
/// `lib` is the de-monolith's job (VISION §7) — this is the minimal ordering
/// key the sealed set needs, and `Ord` (which `MatrixCoordinate` does not
/// derive) is exactly what makes the seal deterministic.
///
/// Ordering is lexicographic on `(x, y, z)` — a total order over the lattice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MatrixIndex {
    /// X coordinate of the matrix cell.
    pub x: i64,
    /// Y coordinate of the matrix cell.
    pub y: i64,
    /// Z coordinate of the matrix cell.
    pub z: i64,
}

impl MatrixIndex {
    /// Create a matrix index.
    pub fn new(x: i64, y: i64, z: i64) -> Self {
        Self { x, y, z }
    }

    /// The lattice origin.
    pub fn origin() -> Self {
        Self { x: 0, y: 0, z: 0 }
    }

    /// Canonical little-endian encoding — 24 bytes, fixed width, so it needs no
    /// length prefix inside a canonical-bytes stream.
    pub fn canonical_bytes(&self) -> [u8; 24] {
        let mut out = [0u8; 24];
        out[0..8].copy_from_slice(&self.x.to_le_bytes());
        out[8..16].copy_from_slice(&self.y.to_le_bytes());
        out[16..24].copy_from_slice(&self.z.to_le_bytes());
        out
    }
}

impl std::fmt::Display for MatrixIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{},{})", self.x, self.y, self.z)
    }
}

/// Length-prefix a byte string into a canonical stream.
///
/// `Grant::canonical_bytes` uses NUL separators; identities and hex digests
/// cannot contain NUL so that is sound there, but a length prefix is
/// unambiguous for ANY input and removes the separator-injection question
/// entirely. Length is `u32` little-endian.
fn push_lp(out: &mut Vec<u8>, bytes: &[u8]) {
    let len = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&bytes[..len as usize]);
}

// ---------------------------------------------------------------------------
// MirrorAttestation
// ---------------------------------------------------------------------------

/// A mirror's signed statement: *"at matrix cell C, I hold and independently
/// validated asset A as of spine point P."*
///
/// This is a **third-party** signature by construction — the attestor is
/// normally NOT the asset's author or owner (that is the whole point: a mirror
/// is someone else holding your asset). Verification therefore binds the
/// signature to the ATTESTOR, never to the asset's author.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorAttestation {
    /// `hA` — content hash of the asset being mirrored.
    pub asset_hash: [u8; 32],

    /// WHERE in the matrix this mirror sits. The **ordering key** of the
    /// attestation set (D1): spatial index replaces temporal ordering.
    pub matrix_index: MatrixIndex,

    /// The attesting mirror's identity — `hex(BLAKE3(FALCON-1024 pubkey))`,
    /// the universal actor string (same as `peer_node_id`,
    /// `stake_proof.stake_holder_id`, `Owner.identity_id`, `Grant.grantee`).
    /// No translation layer.
    pub mirror: String,

    /// The SPINE POINT this attestation is made against: the `lineage_id`
    /// (lowercase hex of the entry's `proof_hash`) of the asset's head entry
    /// the mirror validated. Naming the spine point is what makes an
    /// attestation about a specific *point in the title*, not a floating claim.
    pub spine_point: String,

    /// `asset_seq` of that spine point — redundant with `spine_point` by
    /// design, exactly as S3.2's `asset_seq` is redundant with
    /// `prev_asset_entry`: it makes staleness an O(1) integer comparison.
    pub spine_seq: u64,

    /// FALCON-1024 envelope over [`canonical_bytes`](Self::canonical_bytes).
    ///
    /// Reuses the canonical [`WireSignedProof`] envelope rather than minting a
    /// third identical shape (`WireSignedProof` / `GrantSig` already exist):
    /// detached FALCON-1024 signature over `BLAKE3(proof_bytes || nonce)` with
    /// the signer's full public key carried alongside.
    pub signature: WireSignedProof,
}

impl MirrorAttestation {
    /// Canonical bytes of the attested fields (everything except the
    /// signature). This is what `signature.proof_bytes` must equal, and what
    /// gets signed.
    ///
    /// Domain-separated and length-prefixed, so no two distinct attestations —
    /// and nothing outside this format — can produce the same byte string.
    pub fn canonical_bytes(
        asset_hash: &[u8; 32],
        matrix_index: MatrixIndex,
        mirror: &str,
        spine_point: &str,
        spine_seq: u64,
    ) -> Vec<u8> {
        let mut out = Vec::with_capacity(ATTESTATION_DOMAIN.len() + 128);
        push_lp(&mut out, ATTESTATION_DOMAIN);
        out.extend_from_slice(asset_hash);
        out.extend_from_slice(&matrix_index.canonical_bytes());
        push_lp(&mut out, mirror.as_bytes());
        push_lp(&mut out, spine_point.as_bytes());
        out.extend_from_slice(&spine_seq.to_le_bytes());
        out
    }

    /// Canonical bytes for *this* attestation, from its own fields.
    pub fn my_canonical_bytes(&self) -> Vec<u8> {
        Self::canonical_bytes(
            &self.asset_hash,
            self.matrix_index,
            &self.mirror,
            &self.spine_point,
            self.spine_seq,
        )
    }

    /// The BLAKE3 digest the FALCON signature covers, given canonical bytes and
    /// the envelope nonce — the same recipe as `WireSignedProof` and `GrantSig`.
    pub fn signing_digest(proof_bytes: &[u8], nonce: &[u8; 32]) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(proof_bytes);
        hasher.update(nonce);
        *hasher.finalize().as_bytes()
    }

    /// Digest for this attestation's signature.
    pub fn signing_digest_bytes(&self) -> [u8; 32] {
        Self::signing_digest(&self.signature.proof_bytes, &self.signature.nonce)
    }

    /// (1) Structural binding: the envelope covers THIS attestation's fields.
    /// Tampering with the asset, the matrix index, the mirror identity or the
    /// spine point after signing breaks this.
    pub fn proof_bytes_match(&self) -> bool {
        self.signature.proof_bytes == self.my_canonical_bytes()
    }

    /// (3) Identity binding: `hex(BLAKE3(signer_pubkey)) == mirror`.
    ///
    /// Without this any key could mint an attestation asserting that SOMEONE
    /// ELSE mirrors an asset. Note the crucial difference from H3's
    /// `signer_binds_to_author`: the signer binds to the ATTESTOR, which is a
    /// named third party — that is exactly the check `verify_grant` performs
    /// against `Grant.grantor`.
    pub fn binds_to_signer(&self) -> bool {
        let derived = blake3::hash(&self.signature.signer_pubkey).to_hex().to_string();
        self.mirror == derived
    }

    /// Structural validity: non-empty identities, a `spine_point` within
    /// [`MAX_SPINE_POINT_BYTES`], the envelope covering these fields, and the
    /// signer binding to the claimed mirror.
    ///
    /// This is the whole check MINUS the FALCON verification, which lives in
    /// BlockMatrix (step (2) of `verify_attestation`). Never treat this alone
    /// as acceptance.
    ///
    /// # Why the size cap is HERE and nowhere else (S3.4/F1)
    ///
    /// This function is the ONE audit gate. BlockMatrix's `verify_attestation`
    /// calls it, and `record_mirror_attestation` calls that, so a requirement
    /// added here reaches the accept path automatically — which is exactly the
    /// property S3.3's B1 finding forced (a second list at the accept gate is
    /// what diverged before). Putting the cap at the accept gate only would
    /// recreate that divergence in the opposite direction: memory bounded on
    /// the way in, but [`verify_sealed_set`] and [`verify_membership`] still
    /// willing to open a seal over material the pool would never hold.
    pub fn is_structurally_valid(&self) -> bool {
        !self.mirror.is_empty()
            && !self.spine_point.is_empty()
            && self.spine_point.len() <= MAX_SPINE_POINT_BYTES
            && !self.signature.signer_pubkey.is_empty()
            && !self.signature.signature.is_empty()
            && self.proof_bytes_match()
            && self.binds_to_signer()
    }

    /// The key this attestation is ORDERED and DEDUPED by: `(matrix cell,
    /// mirror identity)`.
    ///
    /// Matrix cell first — spatial index is the primary order (D1). Identity
    /// breaks ties, because one cell can hold more than one mirror. One mirror
    /// at one cell has exactly one live attestation; re-attesting at a newer
    /// spine point replaces it rather than accumulating duplicates.
    pub fn order_key(&self) -> (MatrixIndex, &str) {
        (self.matrix_index, self.mirror.as_str())
    }

    /// This attestation's LEAF COMMITMENT — what the seal root folds.
    ///
    /// Covers the canonical bytes **and** the full signature envelope
    /// (pubkey, nonce, signature). Swapping one mirror's signature for
    /// another's, or re-signing with a different nonce, changes the
    /// commitment and therefore the root.
    pub fn commitment(&self) -> [u8; 32] {
        let mut buf = Vec::new();
        push_lp(&mut buf, ATTESTATION_COMMITMENT_DOMAIN);
        push_lp(&mut buf, &self.my_canonical_bytes());
        push_lp(&mut buf, &self.signature.signer_pubkey);
        buf.extend_from_slice(&self.signature.nonce);
        push_lp(&mut buf, &self.signature.signature);
        *blake3::hash(&buf).as_bytes()
    }
}

// ---------------------------------------------------------------------------
// MirrorSeal
// ---------------------------------------------------------------------------

/// The owner's checkpoint over an accumulated attestation set.
///
/// Carried INSIDE [`StateProof`](crate::proof::StateProof) on one spine entry,
/// so it inherits `proof_hash` commitment (and thus the block hash) plus H3's
/// FALCON envelope — with no `Block` / `BlockAssetEntry` field change.
///
/// # Opening the commitment (F1)
///
/// Two capabilities, deliberately different in scope:
/// * [`verify_membership`] — the party ASSERTING that it mirrored an asset
///   supplies its own leaf + [`MembershipProof`]. Needs no global state, works
///   after any restart, and is the durable audit path.
/// * [`verify_sealed_set`] — whole-set verification. Requires the exact set,
///   which the in-memory pool only holds for the current session, so this is a
///   LOCAL-SESSION capability (and the one the sealer itself uses at seal time
///   to mint membership proofs).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorSeal {
    /// Lowercase hex of the BLAKE3 Merkle root over the canonically-ordered
    /// set — see [`seal_root`].
    pub root: String,
    /// Number of attestations under the root. Redundant with the root (a
    /// stripped attestation already changes it) but makes truncation an O(1)
    /// check and states the cardinality on-chain.
    pub count: u64,
    /// Identity that sealed — must be an OWNER of the asset (distribution
    /// right). `hex(BLAKE3(FALCON pubkey))`.
    pub sealed_by: String,
}

impl MirrorSeal {
    /// Root as raw bytes, if it is well-formed hex.
    pub fn root_bytes(&self) -> Option<[u8; 32]> {
        let raw = hex::decode(&self.root).ok()?;
        <[u8; 32]>::try_from(raw.as_slice()).ok()
    }
}

/// Why a sealed attestation set failed verification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SealBreak {
    /// The presented set has a different cardinality than the seal states.
    CountMismatch {
        /// Cardinality the seal committed to.
        sealed: u64,
        /// Cardinality of the presented set.
        presented: usize,
    },
    /// Two attestations share a `(matrix cell, mirror)` key — the set is not a
    /// well-formed mirror set and its root is not well-defined.
    DuplicateAttestor {
        /// The duplicated matrix cell.
        matrix_index: MatrixIndex,
        /// The duplicated mirror identity.
        mirror: String,
    },
    /// An attestation in the set is not structurally valid (envelope does not
    /// cover its fields, or the signer does not bind to the claimed mirror).
    NotStructurallyValid {
        /// Position in the canonically-ordered set.
        position: usize,
    },
    /// An attestation in the set is for a different asset than the seal claims.
    WrongAsset {
        /// Position in the canonically-ordered set.
        position: usize,
    },
    /// The recomputed root does not equal the sealed root — an attestation was
    /// stripped, added, or altered after sealing.
    RootMismatch {
        /// Root the seal committed to.
        sealed: String,
        /// Root recomputed over the presented set.
        recomputed: String,
    },
}

impl std::fmt::Display for SealBreak {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CountMismatch { sealed, presented } => write!(
                f,
                "seal committed to {sealed} attestations, {presented} presented"
            ),
            Self::DuplicateAttestor { matrix_index, mirror } => write!(
                f,
                "duplicate attestor {mirror} at matrix cell {matrix_index}"
            ),
            Self::NotStructurallyValid { position } => {
                write!(f, "attestation {position} is not structurally valid")
            }
            Self::WrongAsset { position } => {
                write!(f, "attestation {position} is for a different asset")
            }
            Self::RootMismatch { sealed, recomputed } => write!(
                f,
                "seal root {sealed} != recomputed {recomputed} — the attestation set was altered"
            ),
        }
    }
}

/// Canonically order an attestation set: by `(matrix index, mirror)`.
///
/// Returns references in order — the order is a pure function of the SET, so
/// callers may pass attestations in any arrival order.
pub fn canonical_order(attestations: &[MirrorAttestation]) -> Vec<&MirrorAttestation> {
    let mut ordered: Vec<&MirrorAttestation> = attestations.iter().collect();
    ordered.sort_by(|a, b| a.order_key().cmp(&b.order_key()));
    ordered
}

/// Hash of a Merkle LEAF: the attestation's commitment under the leaf tag.
fn leaf_hash(commitment: &[u8; 32]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[MERKLE_LEAF_TAG]);
    hasher.update(commitment);
    *hasher.finalize().as_bytes()
}

/// Hash of an internal Merkle node over its two children, in order.
fn node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[MERKLE_NODE_TAG]);
    hasher.update(left);
    hasher.update(right);
    *hasher.finalize().as_bytes()
}

/// Merkle root over already-ordered, already-tagged leaves.
///
/// An ODD node at a level is PROMOTED unchanged to the next level — it is never
/// duplicated against itself (the Bitcoin CVE-2012-2459 shape, where a
/// duplicated tail makes two different leaf sets share a root). Promotion plus
/// the cardinality folded into [`fold_root`] fixes the tree shape exactly, so
/// no two distinct sets can produce the same seal root.
///
/// The empty set has root `[0u8; 32]`; its cardinality (0) is still folded in
/// by [`fold_root`], so an empty seal is a well-defined, distinct commitment.
fn merkle_root_of(leaves: &[[u8; 32]]) -> [u8; 32] {
    let mut level = leaves.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        let mut i = 0;
        while i < level.len() {
            match level.get(i + 1) {
                Some(right) => next.push(node_hash(&level[i], right)),
                None => next.push(level[i]),
            }
            i += 2;
        }
        level = next;
    }
    level.first().copied().unwrap_or([0u8; 32])
}

/// Fold the cardinality and the domain separator over a Merkle root to produce
/// the SEAL root. Splitting this out is what lets a membership proof reconstruct
/// the sealed root from a leaf + path + `seal.count` alone.
fn fold_root(count: u64, merkle_root: &[u8; 32]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(SEAL_ROOT_DOMAIN);
    hasher.update(&count.to_le_bytes());
    hasher.update(merkle_root);
    *hasher.finalize().as_bytes()
}

/// BLAKE3 **Merkle** root over an attestation set, in canonical matrix order.
///
/// Deterministic by construction: the input is sorted by `(matrix index,
/// mirror)` before the tree is built, so shuffling the slice cannot change the
/// result. The cardinality is folded over the tree root, so a set cannot be
/// re-partitioned into a different set with the same commitment.
///
/// # Why a tree and not a flat fold (F1)
///
/// The attestation pool is in-memory and deliberately not rebuilt from blocks —
/// attestations are third-party statements that arrive over the network, not
/// derived state. A flat root can only ever be opened by presenting the WHOLE
/// set, so after a restart (or after any attestation is replaced by a newer one
/// from the same mirror at the same cell) the on-chain commitment had no
/// retrievable witness: the seal was on-chain and unopenable.
///
/// A Merkle root has an identical on-chain footprint — still one 32-byte root
/// in [`MirrorSeal`] — but it can be opened *partially*: the party ASSERTING
/// membership supplies its own leaf and path
/// ([`membership_proof`] / [`verify_membership`]), and the verifier needs no
/// global state at all. That is the normal way a commitment is opened, and it
/// puts the burden of retention on the claimant rather than on every verifier.
/// Whole-set verification ([`verify_sealed_set`]) is unchanged and keeps every
/// tamper-evidence property it had.
pub fn seal_root(attestations: &[MirrorAttestation]) -> [u8; 32] {
    let leaves: Vec<[u8; 32]> = canonical_order(attestations)
        .iter()
        .map(|attestation| leaf_hash(&attestation.commitment()))
        .collect();
    fold_root(attestations.len() as u64, &merkle_root_of(&leaves))
}

/// One step up a Merkle path: the sibling hash and which side it sits on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleStep {
    /// The sibling subtree hash at this level.
    pub sibling: [u8; 32],
    /// `true` when the sibling is the RIGHT child (so the accumulator is the
    /// left one), `false` when it is the left child.
    pub sibling_on_right: bool,
}

/// A witness that one attestation is inside the set a [`MirrorSeal`] commits to.
///
/// This is the F1 answer to "the sealed root has no retrievable witness": the
/// claimant retains its own `O(log n)` path, and the verifier reconstructs the
/// sealed root from it — nobody has to hold the full set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipProof {
    /// Position of the leaf in the canonically-ordered set. Carried for
    /// diagnostics and for [`SealBreak`] positions; the path itself is what the
    /// verification folds.
    pub leaf_index: usize,
    /// Sibling hashes from the leaf's level up to the root.
    pub path: Vec<MerkleStep>,
}

/// Build a [`MembershipProof`] for `needle` against `attestations`.
///
/// Returns `None` when `needle` is not in the set (compared by commitment, so a
/// byte-identical re-signature is the same leaf and an altered one is not).
///
/// The set is needed HERE, once, by whoever holds it at seal time — the point
/// of the proof is that nobody needs it afterwards.
pub fn membership_proof(
    attestations: &[MirrorAttestation],
    needle: &MirrorAttestation,
) -> Option<MembershipProof> {
    let ordered = canonical_order(attestations);
    let target = needle.commitment();
    let leaf_index = ordered
        .iter()
        .position(|attestation| attestation.commitment() == target)?;

    let mut level: Vec<[u8; 32]> = ordered
        .iter()
        .map(|attestation| leaf_hash(&attestation.commitment()))
        .collect();
    let mut index = leaf_index;
    let mut path = Vec::new();

    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        let mut i = 0;
        while i < level.len() {
            match level.get(i + 1) {
                Some(right) => {
                    if index == i {
                        path.push(MerkleStep { sibling: *right, sibling_on_right: true });
                    } else if index == i + 1 {
                        path.push(MerkleStep { sibling: level[i], sibling_on_right: false });
                    }
                    next.push(node_hash(&level[i], right));
                }
                // Promoted odd node: no sibling, so no path step.
                None => next.push(level[i]),
            }
            i += 2;
        }
        index /= 2;
        level = next;
    }

    Some(MembershipProof { leaf_index, path })
}

/// Verify that `attestation` is a member of the set `seal` commits to, using
/// only `proof` — no access to the rest of the set.
///
/// The attestation must be for `asset_hash` and must pass the same structural
/// gate the whole-set path applies, so a membership proof can never launder an
/// attestation that [`verify_sealed_set`] would reject.
pub fn verify_membership(
    asset_hash: &[u8; 32],
    attestation: &MirrorAttestation,
    proof: &MembershipProof,
    seal: &MirrorSeal,
) -> Result<(), SealBreak> {
    if attestation.asset_hash != *asset_hash {
        return Err(SealBreak::WrongAsset { position: proof.leaf_index });
    }
    if !attestation.is_structurally_valid() {
        return Err(SealBreak::NotStructurallyValid { position: proof.leaf_index });
    }

    let mut accumulator = leaf_hash(&attestation.commitment());
    for step in &proof.path {
        accumulator = if step.sibling_on_right {
            node_hash(&accumulator, &step.sibling)
        } else {
            node_hash(&step.sibling, &accumulator)
        };
    }

    let recomputed = hex::encode(fold_root(seal.count, &accumulator));
    if recomputed != seal.root {
        return Err(SealBreak::RootMismatch {
            sealed: seal.root.clone(),
            recomputed,
        });
    }
    Ok(())
}

/// Build a [`MirrorSeal`] over `attestations`, sealed by `sealed_by`.
///
/// Caller is responsible for the OWNER check and for having verified each
/// attestation's FALCON signature — this function only commits.
pub fn build_seal(sealed_by: impl Into<String>, attestations: &[MirrorAttestation]) -> MirrorSeal {
    MirrorSeal {
        root: hex::encode(seal_root(attestations)),
        count: attestations.len() as u64,
        sealed_by: sealed_by.into(),
    }
}

/// Verify a presented attestation set against a seal for `asset_hash`.
///
/// Checks, in order: cardinality, per-attestation asset binding and structural
/// validity, no duplicate `(cell, mirror)` key, and finally the recomputed
/// root. A stripped, added, or altered attestation fails.
///
/// This does NOT FALCON-verify — the caller (BlockMatrix) runs
/// `verify_attestation` per attestation when it needs cryptographic proof of
/// WHO attested, exactly as `AssetLineage::verify` leaves signatures to its
/// caller.
///
/// # Scope (F1)
///
/// Whole-set verification needs the whole set, and the attestation pool is
/// in-memory and not rebuilt from blocks — so this is a **local-session**
/// capability. The durable, restart-surviving way to open a seal is
/// [`verify_membership`], where the claimant carries its own witness.
pub fn verify_sealed_set(
    asset_hash: &[u8; 32],
    attestations: &[MirrorAttestation],
    seal: &MirrorSeal,
) -> Result<(), SealBreak> {
    if seal.count != attestations.len() as u64 {
        return Err(SealBreak::CountMismatch {
            sealed: seal.count,
            presented: attestations.len(),
        });
    }

    let ordered = canonical_order(attestations);
    for (position, attestation) in ordered.iter().enumerate() {
        if attestation.asset_hash != *asset_hash {
            return Err(SealBreak::WrongAsset { position });
        }
        if !attestation.is_structurally_valid() {
            return Err(SealBreak::NotStructurallyValid { position });
        }
        if let Some(previous) = position.checked_sub(1).and_then(|p| ordered.get(p)) {
            if previous.order_key() == attestation.order_key() {
                return Err(SealBreak::DuplicateAttestor {
                    matrix_index: attestation.matrix_index,
                    mirror: attestation.mirror.clone(),
                });
            }
        }
    }

    let recomputed = hex::encode(seal_root(attestations));
    if recomputed != seal.root {
        return Err(SealBreak::RootMismatch {
            sealed: seal.root.clone(),
            recomputed,
        });
    }

    Ok(())
}

/// Is `needle` a member of the set the seal commits to?
///
/// Membership is only meaningful against an INTACT set, so the set is verified
/// against the seal first: `Ok(true)` means "this attestation is inside a set
/// that hashes to the sealed root".
pub fn sealed_set_contains(
    asset_hash: &[u8; 32],
    attestations: &[MirrorAttestation],
    seal: &MirrorSeal,
    needle: &MirrorAttestation,
) -> Result<bool, SealBreak> {
    verify_sealed_set(asset_hash, attestations, seal)?;
    let target = needle.commitment();
    Ok(attestations.iter().any(|a| a.commitment() == target))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use pqcrypto_falcon::falcon1024;
    use pqcrypto_traits::sign::{DetachedSignature, PublicKey};

    /// Mint a real FALCON-signed attestation whose mirror identity binds to the
    /// signing key — the way an actual mirror would produce one.
    fn attest(
        asset_hash: [u8; 32],
        matrix_index: MatrixIndex,
        spine_point: &str,
        spine_seq: u64,
        nonce_seed: u8,
    ) -> MirrorAttestation {
        let (pk, sk) = falcon1024::keypair();
        let mirror = blake3::hash(pk.as_bytes()).to_hex().to_string();
        let proof_bytes = MirrorAttestation::canonical_bytes(
            &asset_hash,
            matrix_index,
            &mirror,
            spine_point,
            spine_seq,
        );
        let mut nonce = [0u8; 32];
        nonce[0] = nonce_seed;
        let digest = MirrorAttestation::signing_digest(&proof_bytes, &nonce);
        let sig = falcon1024::detached_sign(&digest, &sk);
        MirrorAttestation {
            asset_hash,
            matrix_index,
            mirror,
            spine_point: spine_point.to_string(),
            spine_seq,
            signature: WireSignedProof {
                proof_bytes,
                signature: sig.as_bytes().to_vec(),
                signer_pubkey: pk.as_bytes().to_vec(),
                nonce,
            },
        }
    }

    /// The FALCON half — lives in BlockMatrix in production; exercised here so
    /// the recipe is proven to round-trip at the type's own layer.
    fn falcon_ok(attestation: &MirrorAttestation) -> bool {
        let Ok(pk) = falcon1024::PublicKey::from_bytes(&attestation.signature.signer_pubkey) else {
            return false;
        };
        let Ok(sig) = falcon1024::DetachedSignature::from_bytes(&attestation.signature.signature)
        else {
            return false;
        };
        let digest = attestation.signing_digest_bytes();
        falcon1024::verify_detached_signature(&sig, &digest, &pk).is_ok()
    }

    fn set_of(n: u8, asset: [u8; 32]) -> Vec<MirrorAttestation> {
        (0..n)
            .map(|i| {
                attest(
                    asset,
                    MatrixIndex::new(i as i64, (n - i) as i64, 3),
                    "spine-point-abc",
                    7,
                    i,
                )
            })
            .collect()
    }

    #[test]
    fn attestation_roundtrips_and_binds() {
        let a = attest([9u8; 32], MatrixIndex::new(1, 2, 3), "head-id", 4, 1);
        assert!(a.proof_bytes_match());
        assert!(a.binds_to_signer());
        assert!(falcon_ok(&a));
        assert!(a.is_structurally_valid());
    }

    #[test]
    fn tampered_matrix_index_rejected() {
        let mut a = attest([9u8; 32], MatrixIndex::new(1, 2, 3), "head-id", 4, 1);
        a.matrix_index = MatrixIndex::new(1, 2, 4);
        assert!(!a.proof_bytes_match(), "tampered cell breaks structural binding");
        assert!(!a.is_structurally_valid());
    }

    #[test]
    fn tampered_spine_point_rejected() {
        let mut a = attest([9u8; 32], MatrixIndex::new(1, 2, 3), "head-id", 4, 1);
        a.spine_point = "other-head".to_string();
        assert!(!a.is_structurally_valid());
    }

    #[test]
    fn identity_binding_mismatch_rejected() {
        let mut a = attest([9u8; 32], MatrixIndex::new(1, 2, 3), "head-id", 4, 1);
        // Re-sign under a stranger's identity string: canonical bytes are
        // consistent, but the signer no longer derives the claimed mirror.
        let (pk, sk) = falcon1024::keypair();
        a.mirror = "0".repeat(64);
        a.signature.proof_bytes = a.my_canonical_bytes();
        let digest = MirrorAttestation::signing_digest(&a.signature.proof_bytes, &a.signature.nonce);
        a.signature.signature = falcon1024::detached_sign(&digest, &sk).as_bytes().to_vec();
        a.signature.signer_pubkey = pk.as_bytes().to_vec();
        assert!(a.proof_bytes_match(), "structural bind still holds");
        assert!(falcon_ok(&a), "FALCON still verifies");
        assert!(!a.binds_to_signer(), "identity binding must fail");
        assert!(!a.is_structurally_valid());
    }

    /// S3.4/F1 — the `spine_point` cap, at the ONE gate.
    ///
    /// The attack is a 13,400-byte `spine_point` (the largest that fits the
    /// 64 KiB wire cap), signed by the attacker itself so the envelope is
    /// perfectly valid. What refuses it is the SIZE, not the crypto.
    #[test]
    fn spine_point_is_length_capped_at_the_audit_gate() {
        let asset = [0x5Au8; 32];
        let cell = MatrixIndex::new(1, 2, 3);

        // The only length any producer emits: hex of a BLAKE3 digest.
        let honest = attest(asset, cell, &"ab".repeat(32), 4, 1);
        assert_eq!(honest.spine_point.len(), 64);
        assert!(honest.is_structurally_valid(), "an honest lineage_id must pass");

        // Exactly at the cap: accepted. One byte over: refused.
        let at_cap = attest(asset, cell, &"c".repeat(MAX_SPINE_POINT_BYTES), 4, 2);
        assert!(at_cap.is_structurally_valid(), "the cap is inclusive");
        let over = attest(asset, cell, &"c".repeat(MAX_SPINE_POINT_BYTES + 1), 4, 3);
        assert!(!over.is_structurally_valid(), "one byte over must be refused");
        assert!(over.proof_bytes_match() && over.binds_to_signer() && falcon_ok(&over),
            "the refusal is by SIZE — the envelope and identity binding are sound");

        // The measured attack.
        let attack = attest(asset, cell, &"S".repeat(13_400), 4, 4);
        assert!(!attack.is_structurally_valid());

        // And because the gate is one gate, the seal paths refuse it too — an
        // oversized attestation cannot be laundered through a membership proof.
        let seal = build_seal("owner-1", std::slice::from_ref(&attack));
        assert!(matches!(
            verify_sealed_set(&asset, std::slice::from_ref(&attack), &seal),
            Err(SealBreak::NotStructurallyValid { .. })
        ));
        let proof = membership_proof(std::slice::from_ref(&attack), &attack)
            .expect("test: leaf exists regardless of size");
        assert!(matches!(
            verify_membership(&asset, &attack, &proof, &seal),
            Err(SealBreak::NotStructurallyValid { .. })
        ));
    }

    #[test]
    fn seal_root_is_order_independent() {
        let asset = [3u8; 32];
        let mut set = set_of(6, asset);
        let root = seal_root(&set);
        set.reverse();
        assert_eq!(root, seal_root(&set), "reversal must not change the root");
        set.swap(0, 3);
        set.swap(1, 5);
        assert_eq!(root, seal_root(&set), "shuffle must not change the root");
    }

    #[test]
    fn seal_detects_strip_and_add() {
        let asset = [3u8; 32];
        let set = set_of(5, asset);
        let seal = build_seal("owner-1", &set);
        assert!(verify_sealed_set(&asset, &set, &seal).is_ok());

        let mut stripped = set.clone();
        stripped.pop();
        assert!(matches!(
            verify_sealed_set(&asset, &stripped, &seal),
            Err(SealBreak::CountMismatch { .. })
        ));

        let mut added = set.clone();
        added.push(attest(asset, MatrixIndex::new(99, 99, 99), "spine-point-abc", 7, 99));
        assert!(matches!(
            verify_sealed_set(&asset, &added, &seal),
            Err(SealBreak::CountMismatch { .. })
        ));

        // Same cardinality, different member: root must catch it.
        let mut swapped = set.clone();
        swapped[2] = attest(asset, MatrixIndex::new(2, 3, 3), "spine-point-abc", 7, 42);
        assert!(matches!(
            verify_sealed_set(&asset, &swapped, &seal),
            Err(SealBreak::RootMismatch { .. })
        ));
    }

    #[test]
    fn seal_rejects_duplicate_attestor_and_wrong_asset() {
        let asset = [3u8; 32];
        let mut set = set_of(3, asset);
        let clone = set[0].clone();
        set.push(clone);
        let seal = build_seal("owner-1", &set);
        assert!(matches!(
            verify_sealed_set(&asset, &set, &seal),
            Err(SealBreak::DuplicateAttestor { .. })
        ));

        let other = set_of(2, [4u8; 32]);
        let other_seal = build_seal("owner-1", &other);
        assert!(matches!(
            verify_sealed_set(&asset, &other, &other_seal),
            Err(SealBreak::WrongAsset { .. })
        ));
    }

    #[test]
    fn membership_holds_only_against_intact_set() {
        let asset = [3u8; 32];
        let set = set_of(4, asset);
        let seal = build_seal("owner-1", &set);
        let member = set[1].clone();
        assert_eq!(sealed_set_contains(&asset, &set, &seal, &member), Ok(true));

        let stranger = attest(asset, MatrixIndex::new(-5, -5, -5), "spine-point-abc", 7, 77);
        assert_eq!(sealed_set_contains(&asset, &set, &seal, &stranger), Ok(false));

        let mut altered = set.clone();
        altered[0].signature.nonce[31] ^= 1;
        assert!(sealed_set_contains(&asset, &altered, &seal, &member).is_err());
    }

    #[test]
    fn canonical_order_is_matrix_lexicographic() {
        let asset = [1u8; 32];
        let a = attest(asset, MatrixIndex::new(0, 5, 0), "p", 0, 1);
        let b = attest(asset, MatrixIndex::new(0, 1, 9), "p", 0, 2);
        let c = attest(asset, MatrixIndex::new(-1, 9, 9), "p", 0, 3);
        let set = vec![a.clone(), b.clone(), c.clone()];
        let ordered: Vec<MatrixIndex> =
            canonical_order(&set).iter().map(|x| x.matrix_index).collect();
        assert_eq!(
            ordered,
            vec![
                MatrixIndex::new(-1, 9, 9),
                MatrixIndex::new(0, 1, 9),
                MatrixIndex::new(0, 5, 0)
            ]
        );
    }

    #[test]
    fn membership_proof_opens_the_root_without_the_set() {
        // F1: the witness travels with the claimant. Every arity from 1..=9
        // covers both balanced levels and PROMOTED odd nodes.
        for n in 1..=9u8 {
            let asset = [n; 32];
            let set = set_of(n, asset);
            let seal = build_seal("owner-1", &set);
            for member in &set {
                let proof = membership_proof(&set, member).expect("test: member has a witness");
                assert_eq!(
                    verify_membership(&asset, member, &proof, &seal),
                    Ok(()),
                    "n={n}: a member must open the seal with its own witness"
                );
            }
            let stranger = attest(asset, MatrixIndex::new(-77, -77, -77), "sp", 1, 200);
            assert!(membership_proof(&set, &stranger).is_none());
            let borrowed = membership_proof(&set, &set[0]).expect("test: witness");
            assert!(
                verify_membership(&asset, &stranger, &borrowed, &seal).is_err(),
                "a non-member cannot ride someone else's path"
            );
        }
    }

    #[test]
    fn membership_proof_is_bound_to_count_and_asset() {
        let asset = [8u8; 32];
        let set = set_of(5, asset);
        let seal = build_seal("owner-1", &set);
        let proof = membership_proof(&set, &set[3]).expect("test: witness");

        // Truncating the claimed cardinality breaks the fold.
        let mut lied = seal.clone();
        lied.count -= 1;
        assert!(matches!(
            verify_membership(&asset, &set[3], &proof, &lied),
            Err(SealBreak::RootMismatch { .. })
        ));

        // Wrong asset, and structurally invalid members, are refused.
        assert!(matches!(
            verify_membership(&[7u8; 32], &set[3], &proof, &seal),
            Err(SealBreak::WrongAsset { .. })
        ));
        let mut hollow = set[3].clone();
        hollow.spine_point = String::new();
        assert!(matches!(
            verify_membership(&asset, &hollow, &proof, &seal),
            Err(SealBreak::NotStructurallyValid { .. })
        ));
    }

    #[test]
    fn merkle_root_is_still_order_independent_and_distinct_per_set() {
        let asset = [2u8; 32];
        let mut set = set_of(7, asset);
        let root = seal_root(&set);
        set.swap(0, 6);
        set.swap(2, 4);
        assert_eq!(root, seal_root(&set), "shuffle cannot change the Merkle root");
        // Distinct cardinalities never collide, even on promoted-odd shapes.
        let roots: std::collections::HashSet<[u8; 32]> =
            (0..8u8).map(|n| seal_root(&set_of(n, asset))).collect();
        assert_eq!(roots.len(), 8, "each cardinality must have its own root");
    }

    #[test]
    fn domain_separation_blocks_cross_type_replay() {
        // A grant's canonical bytes can never equal an attestation's: the
        // attestation stream begins with its own length-prefixed domain tag.
        let a = MirrorAttestation::canonical_bytes(&[0u8; 32], MatrixIndex::origin(), "m", "s", 0);
        let g = crate::authz::Grant::canonical_bytes("m", "s", crate::authz::GrantScope::Read, None);
        assert_ne!(a, g);
        assert!(a.starts_with(&(ATTESTATION_DOMAIN.len() as u32).to_le_bytes()));
    }
}
