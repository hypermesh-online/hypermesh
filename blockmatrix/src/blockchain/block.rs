// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Block implementation for every-node-blockchain architecture
//!
//! Each block belongs to a specific node's independent blockchain.
//! NO merkle tree consolidation across nodes - fundamental design principle.
//!
//! Block structure:
//! ```text
//! Block_i = {
//!     prev_hash,
//!     entries: [
//!         { hA, hπ, state_proof, ptr },
//!         ...
//!     ]
//! }
//! block_hash_i = BLAKE3(Block_i)
//! ```
//!
//! - `hA` = BLAKE3(Brotli(Asset)) — content hash of the compressed asset
//! - `hπ` = BLAKE3(StateProof) — proof integrity hash
//! - `state_proof` — the full four-proof authentication (WHO/WHEN/WHERE/WHAT)
//! - `ptr` — storage pointer (local path or shard placements)
//!
//! Timestamp and node coordinate are NOT block fields — they live inside
//! the state proof (PoTime = WHEN, PoSpace = WHERE).
//!
//! Ledger secures integrity. Storage layer holds data.

use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::assets::core::AssetRegistration;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::proof_of_state::genesis_proof::{
    generate_genesis_proof, GenesisEpoch, HardwareAssessment,
};
use trustchain::proof_of_state::{StateProof, WireSignedProof};

/// Where the actual asset data lives (the block only stores a pointer).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum StoragePointer {
    /// Asset data stored locally at this path
    Local { path: String },
    /// Asset data sharded across matrix positions
    Sharded {
        /// BLAKE3 hash of each shard
        shard_hashes: Vec<[u8; 32]>,
        /// Matrix positions where shards are placed
        placements: Vec<MatrixCoordinate>,
    },
    /// Genesis assets — no external storage, the registration IS the data
    Genesis,
}

/// Bind a state proof to a specific asset hash and compute its proof hash.
///
/// This is the **signed-to-content** half of the mirror invariant (P1). The
/// asset's content hash is written into `SpaceProof.file_hash` (as lowercase
/// hex) so the proof can no longer be detached from its content and replayed
/// against a *different* asset. Because `proof_hash = BLAKE3(serialize(proof))`
/// covers `file_hash`, and the block hash commits to `(asset_hash, proof_hash)`,
/// moving a proof to another asset would require a BLAKE3 collision.
///
/// Returns `(bound_proof, proof_hash)`. The caller stores both in the entry.
///
/// Note: this does NOT change `Block::calculate_hash` — it only changes the
/// *value* of `file_hash` (and therefore `proof_hash`) at construction time,
/// exactly as the device-auth proof-derivation change did. A fixed `Block`
/// value re-hashes byte-identically.
pub fn bind_proof_to_asset(
    asset_hash: &[u8; 32],
    state_proof: &StateProof,
) -> (StateProof, [u8; 32]) {
    let mut bound = state_proof.clone();
    bound.space_proof.file_hash = hex::encode(asset_hash);
    let proof_bytes = serde_json::to_vec(&bound).unwrap_or_default();
    let proof_hash = *blake3::hash(&proof_bytes).as_bytes();
    (bound, proof_hash)
}

/// A single asset entry within a block.
///
/// Each entry is self-contained: content hash, proof, and storage pointer.
/// Assets within a block can reference each other by content hash (hA).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BlockAssetEntry {
    /// hA = BLAKE3(Brotli(Asset)) — content address of the compressed asset
    pub asset_hash: [u8; 32],

    /// hπ = BLAKE3(StateProof) — integrity hash of the proof
    pub proof_hash: [u8; 32],

    /// The full state proof (PoStake/PoTime/PoSpace/PoWork)
    pub state_proof: StateProof,

    /// FALCON-1024 signed envelope over the (JSON-serialized) `state_proof`.
    ///
    /// H3 (block-accept PoS hardening): the bare `state_proof` above carries no
    /// signature, so block-accept could only validate it STRUCTURALLY. This
    /// envelope binds the proof to the producing node's FALCON-1024 identity key
    /// (`WireSignedProof.signer_pubkey`) with a detached signature over
    /// `BLAKE3(proof_bytes || nonce)` — the SAME recipe the STOQ handshake uses
    /// (`TrustChainProofProvider::generate_proof`). `insert_received_block`
    /// FALCON-verifies it and binds `BLAKE3(signer_pubkey)` to the entry's
    /// claimed author before running structural validation.
    ///
    /// `Option<..>` + `#[serde(default)]`: on the JSON wire path old blocks
    /// (produced before H3) deserialize as `None`; the compat flag on the
    /// accept path decides whether a `None`/legacy entry is tolerated during a
    /// one-release migration. Populated at the single local-write chokepoint
    /// (`NodeBlockchain::add_block`) when the chain has a signer; produced
    /// blocks then carry it to peers over the wire.
    ///
    /// NOTE (bincode on-disk): `#[serde(default)]` only rescues self-describing
    /// formats (JSON). The persisted on-disk format is bincode (non
    /// self-describing, positional) — so old V1 payloads written before this
    /// field existed cannot be read after this schema change regardless of
    /// `#[serde(default)]`. That is the documented wipe-on-block-format-change
    /// situation (CLAUDE.md); we do NOT destructively migrate persisted data
    /// here, and this field is EXCLUDED from `calculate_hash` so it never
    /// changes an existing block's canonical hash.
    #[serde(default)]
    pub signed_proof: Option<WireSignedProof>,

    /// Where the actual data lives
    pub storage_pointer: StoragePointer,

    /// Asset registration metadata (category, network scope, etc.)
    pub registration: AssetRegistration,
}

impl BlockAssetEntry {
    /// Construct an entry with its proof cryptographically bound to `asset_hash`.
    ///
    /// This is the sanctioned way to build an entry: it guarantees the
    /// signed-to-content half of the mirror invariant holds by construction —
    /// `state_proof.space_proof.file_hash == hex(asset_hash)` and
    /// `proof_hash == BLAKE3(serialize(bound_proof))`.
    pub fn new_bound(
        asset_hash: [u8; 32],
        state_proof: &StateProof,
        storage_pointer: StoragePointer,
        registration: AssetRegistration,
    ) -> Self {
        let (bound_proof, proof_hash) = bind_proof_to_asset(&asset_hash, state_proof);
        Self {
            asset_hash,
            proof_hash,
            state_proof: bound_proof,
            signed_proof: None,
            storage_pointer,
            registration,
        }
    }

    /// Verify the **signed-to-content** binding: the proof references THIS
    /// entry's `asset_hash` via `SpaceProof.file_hash`.
    ///
    /// A mirror whose proof is not bound to its content is rejected — a valid
    /// proof for asset A cannot be replayed inside an entry claiming asset B.
    pub fn content_binding_ok(&self) -> bool {
        self.state_proof.space_proof.file_hash == hex::encode(self.asset_hash)
    }

    /// S3.2 — stamp this entry's ASSET LINEAGE into the proof body and
    /// re-derive `proof_hash`.
    ///
    /// The lineage pointer lives inside `state_proof` (see
    /// [`StateProof::prev_asset_entry`]), so writing it necessarily changes
    /// `proof_hash = BLAKE3(serialize(state_proof))`. This re-runs
    /// [`bind_proof_to_asset`] so the signed-to-content binding
    /// (`space_proof.file_hash == hex(asset_hash)`) is preserved and the entry
    /// stays internally consistent by construction.
    ///
    /// Any previously attached `signed_proof` is DROPPED: an H3 envelope signs
    /// the proof bytes, and those bytes just changed — keeping the old envelope
    /// would leave an entry whose signature no longer wraps its proof. The
    /// write chokepoint stamps lineage BEFORE signing, so nothing is lost.
    ///
    /// This is a method change only: no `BlockAssetEntry` field is added, and
    /// `Block::calculate_hash` still commits to exactly
    /// `(asset_hash || proof_hash)` — the lineage reaches the block hash
    /// TRANSITIVELY, through `proof_hash`.
    pub fn set_asset_lineage(&mut self, prev_asset_entry: Option<String>, asset_seq: u64) {
        self.state_proof.prev_asset_entry = prev_asset_entry;
        self.state_proof.asset_seq = asset_seq;
        let (bound, proof_hash) = bind_proof_to_asset(&self.asset_hash, &self.state_proof);
        self.state_proof = bound;
        self.proof_hash = proof_hash;
        self.signed_proof = None;
    }

    /// S3.2 — this entry's lineage identity: what a SUCCESSOR entry for the
    /// same asset must carry in `prev_asset_entry`.
    ///
    /// Lowercase hex of `proof_hash`, which is the value
    /// `Block::calculate_hash` already commits to for this entry.
    pub fn lineage_id(&self) -> String {
        hex::encode(self.proof_hash)
    }

    /// S3.2 — the predecessor this entry claims for its asset, if any.
    pub fn prev_asset_entry(&self) -> Option<&str> {
        self.state_proof.prev_asset_entry.as_deref()
    }

    /// S3.2 — this entry's position in its asset's chain (0 = asset genesis).
    pub fn asset_seq(&self) -> u64 {
        self.state_proof.asset_seq
    }

    /// S3.2 — does this entry claim to be its asset's FIRST entry?
    pub fn is_asset_genesis(&self) -> bool {
        self.state_proof.is_asset_genesis()
    }

    /// S3.2 — is this entry a well-formed successor of `predecessor`?
    ///
    /// Both halves are checked: the prev-pointer must name the predecessor's
    /// `lineage_id`, and the sequence must advance by exactly one.
    ///
    /// The successor sequence is computed with `checked_add`: at `u64::MAX` the
    /// answer is "no successor exists", not a wrap to 0 — an asset chain cannot
    /// be re-rooted by overflowing its counter.
    pub fn succeeds(&self, predecessor: &BlockAssetEntry) -> bool {
        let Some(expected_seq) = predecessor.asset_seq().checked_add(1) else {
            return false;
        };
        self.asset_hash == predecessor.asset_hash
            && self.prev_asset_entry() == Some(predecessor.lineage_id().as_str())
            && self.asset_seq() == expected_seq
    }

    /// Attach a FALCON-1024 signed envelope over this entry's `state_proof`.
    ///
    /// H3: this is the single local-write signing step. It serializes the
    /// **bound** `state_proof` (whose `space_proof.file_hash` already equals
    /// `hex(asset_hash)`) exactly as `TrustChainProofProvider::generate_proof`
    /// does, signs `BLAKE3(proof_bytes || nonce)` with the node's FALCON key,
    /// and stores the resulting [`WireSignedProof`] in `signed_proof`.
    ///
    /// The envelope covers the proof bytes (not the block hash), so it does NOT
    /// perturb `Block::calculate_hash` (which excludes both `state_proof` and
    /// `signed_proof`). A caller signs at construction so the produced block
    /// carries the envelope to peers.
    pub fn sign_proof(
        &mut self,
        signer: &(dyn hypermesh_lib::NodeSigner + Send + Sync),
    ) -> Result<(), String> {
        let proof_bytes = serde_json::to_vec(&self.state_proof)
            .map_err(|e| format!("failed to serialize state_proof for signing: {e}"))?;

        let mut nonce = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce);

        let mut hasher = Hasher::new();
        hasher.update(&proof_bytes);
        hasher.update(&nonce);
        let digest = hasher.finalize();

        let signature = signer
            .sign(digest.as_bytes())
            .map_err(|e| format!("FALCON signing failed: {e}"))?;

        self.signed_proof = Some(WireSignedProof {
            proof_bytes,
            signature,
            signer_pubkey: signer.public_key_bytes().to_vec(),
            nonce,
        });
        Ok(())
    }

    /// FALCON-verify the attached `signed_proof` and confirm it wraps THIS
    /// entry's `state_proof`.
    ///
    /// H3 accept-path verify. Returns `Ok(signer_pubkey)` when:
    /// 1. `signed_proof` is present,
    /// 2. its `proof_bytes` equal the JSON serialization of `self.state_proof`
    ///    (the envelope signs the proof we actually carry — no bait-and-switch),
    /// 3. the FALCON-1024 detached signature over `BLAKE3(proof_bytes || nonce)`
    ///    verifies against the embedded `signer_pubkey`.
    ///
    /// The caller then binds `BLAKE3(signer_pubkey)` to the entry's claimed
    /// author/owner. `Err` on any failure; `Ok(None)` never — absence is an
    /// error here (the accept path decides legacy tolerance separately).
    pub fn verify_signed_proof(&self) -> Result<Vec<u8>, String> {
        let wire = self
            .signed_proof
            .as_ref()
            .ok_or_else(|| "entry has no signed_proof envelope".to_string())?;

        // (1) The envelope must wrap the proof this entry actually carries.
        let expected = serde_json::to_vec(&self.state_proof)
            .map_err(|e| format!("failed to serialize state_proof: {e}"))?;
        if wire.proof_bytes != expected {
            return Err(
                "signed_proof envelope does not wrap this entry's state_proof".to_string(),
            );
        }

        // (2) FALCON-1024 detached signature over BLAKE3(proof_bytes || nonce).
        let mut hasher = Hasher::new();
        hasher.update(&wire.proof_bytes);
        hasher.update(&wire.nonce);
        let digest = hasher.finalize();

        // FALCON verify via the `NodeSigner` trait method on the concrete
        // identity type (single source of truth for FALCON-1024 verification).
        let ok = <crate::identity::FalconIdentity as hypermesh_lib::NodeSigner>::verify_signature(
            &wire.signer_pubkey,
            digest.as_bytes(),
            &wire.signature,
        )
        .map_err(|e| format!("FALCON verification error: {e}"))?;
        if !ok {
            return Err("FALCON signature verification failed on signed_proof".to_string());
        }

        Ok(wire.signer_pubkey.clone())
    }
}

/// Lightweight block header for chain integrity verification.
///
/// Nodes store headers for blocks they don't fully participate in,
/// enabling selective chain reconstruction without full block data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockHeader {
    /// Block index in the chain
    pub index: u64,
    /// This block's hash (BLAKE3 hex)
    pub hash: String,
    /// Hash of the previous block (BLAKE3 hex)
    pub previous_hash: String,
    /// BLAKE3 hash of the serialized entries, proving header matches block content.
    pub entries_hash: [u8; 32],
    /// Number of asset entries in the block
    pub entry_count: usize,
}

impl BlockHeader {
    /// Verify that this header chains to the given previous header.
    pub fn chains_to(&self, previous: &BlockHeader) -> bool {
        self.previous_hash == previous.hash && self.index == previous.index + 1
    }
}

/// A block in a node's independent blockchain.
///
/// The block is purely: hash linkage + asset entries.
/// All metadata (timestamp, location) lives in the state proofs.
/// Same content = same hash = same block. No nonce, no timestamp on the block itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Block {
    /// Block index in this node's chain
    pub index: u64,

    /// Hash of the previous block in THIS node's chain (BLAKE3 hex)
    pub previous_hash: String,

    /// This block's hash (BLAKE3 hex)
    pub hash: String,

    /// Asset entries: each contains { hA, hπ, state_proof, ptr, registration }
    pub entries: Vec<BlockAssetEntry>,
}

impl Block {
    /// Create a new block from asset entries.
    pub fn new(
        index: u64,
        entries: Vec<BlockAssetEntry>,
        previous_hash: String,
    ) -> Self {
        assert!(!entries.is_empty(), "Block must contain at least one entry");

        let mut block = Block {
            index,
            previous_hash,
            hash: String::new(),
            entries,
        };

        block.hash = block.calculate_hash();
        block
    }

    /// Create the genesis block for a node (coordinate-only path).
    ///
    /// Genesis entries use `StoragePointer::Genesis` and a hardware-assessed
    /// StateProof (self-authorized — sovereignty from boot). The device
    /// fingerprint is captured UNCONDITIONALLY from the OS; the genesis node
    /// ID is `genesis_<device_fingerprint_short>` when no canonical FALCON
    /// identity is supplied.
    ///
    /// Prefer [`Block::genesis_with_identity`] on the real boot path so the
    /// collapsed `BLAKE3(falcon_pubkey)` node ID flows into every proof.
    ///
    /// Per R1: hardware assessed, not self-reported.
    /// Per section 8.2: "Usage IS verification."
    pub fn genesis(node_coordinate: MatrixCoordinate) -> Self {
        Self::build_genesis_block(node_coordinate, None, GenesisEpoch::now())
    }

    /// Create the genesis block bound to a canonical device identity.
    ///
    /// `device_node_id` is the collapsed canonical node ID
    /// (`BLAKE3(falcon_pubkey)` hex). It becomes `PoStake.stake_holder_id`,
    /// `PoSpace.node_id`, and `PoWork.owner_id`, collapsing the three
    /// historical node IDs into one. The device fingerprint (from the OS) is
    /// folded into all four proofs; the genesis label is
    /// `genesis_<device_node_id>`.
    pub fn genesis_with_identity(
        node_coordinate: MatrixCoordinate,
        device_node_id: &str,
    ) -> Self {
        Self::genesis_with_identity_at(node_coordinate, device_node_id, GenesisEpoch::now())
    }

    /// [`genesis_with_identity`](Self::genesis_with_identity) with an EXPLICIT
    /// genesis epoch (S3.0/B2).
    ///
    /// This is the reproducible form: given the same device, coordinate,
    /// identity and epoch it yields the same block, byte for byte. The live
    /// daemon calls it with `GenesisEpoch::now()` at first boot — one explicit
    /// clock read, recorded on-chain — instead of scattering `SystemTime::now()`
    /// through the four proofs.
    pub fn genesis_with_identity_at(
        node_coordinate: MatrixCoordinate,
        device_node_id: &str,
        epoch: GenesisEpoch,
    ) -> Self {
        Self::build_genesis_block(node_coordinate, Some(device_node_id), epoch)
    }

    /// Build a genesis block from an EXPLICIT hardware assessment and epoch —
    /// no OS probe, no clock read (S3.0/B2).
    ///
    /// This is the purely functional core: everything the block depends on is
    /// an argument, which is what makes "two nodes, identical inputs, identical
    /// genesis" a checkable property rather than an aspiration.
    pub fn genesis_from_assessment(hw: &HardwareAssessment, epoch: GenesisEpoch) -> Self {
        Self::assemble_genesis(hw.coordinate, generate_genesis_proof(hw, epoch), epoch)
    }

    fn build_genesis_block(
        node_coordinate: MatrixCoordinate,
        device_node_id: Option<&str>,
        epoch: GenesisEpoch,
    ) -> Self {
        let state_proof = Self::build_genesis_proof(node_coordinate, device_node_id, epoch);
        Self::assemble_genesis(node_coordinate, state_proof, epoch)
    }

    /// Assemble the genesis block from a coordinate and its (already built)
    /// state proof. Shared by every genesis constructor so the entry shape and
    /// content binding are defined exactly once.
    fn assemble_genesis(
        node_coordinate: MatrixCoordinate,
        state_proof: StateProof,
        epoch: GenesisEpoch,
    ) -> Self {
        // B2: the registration is stamped with the genesis epoch, not the live
        // clock — its metadata feeds `content_hash` and its `creation_timestamp`
        // feeds `to_string()`, so both reach the block hash.
        let genesis_reg =
            AssetRegistration::genesis_at(node_coordinate, epoch.as_system_time());
        let content_hash = {
            let serialized = genesis_reg.to_string();
            *blake3::hash(serialized.as_bytes()).as_bytes()
        };

        // Bind the genesis proof to the genesis asset's content hash so the
        // signed-to-content invariant holds from block 0 (P1). `new_bound`
        // sets `space_proof.file_hash = hex(content_hash)` and derives
        // `proof_hash` over the bound proof.
        let genesis_entry = BlockAssetEntry::new_bound(
            content_hash,
            &state_proof,
            StoragePointer::Genesis,
            genesis_reg,
        );

        Block::new(
            0,
            vec![genesis_entry],
            String::from("0000000000000000000000000000000000000000000000000000000000000000"),
        )
    }

    /// Build a StateProof for the genesis block from real hardware.
    ///
    /// Attempts OS hardware detection (which also captures the device
    /// fingerprint); falls back to safe defaults that still satisfy
    /// `StateRequirements::default()` for R13-compliant devices. The genesis
    /// node ID is the canonical device node ID when supplied, otherwise
    /// `genesis_<fingerprint_short>` (still device-bound, not coord-derived).
    fn build_genesis_proof(
        coordinate: MatrixCoordinate,
        device_node_id: Option<&str>,
        epoch: GenesisEpoch,
    ) -> StateProof {
        match crate::create_os_abstraction() {
            Ok(os) => {
                let node_id = device_node_id.map(|s| s.to_string()).unwrap_or_else(|| {
                    let fp = os.device_fingerprint();
                    format!("genesis_{}", &fp.hex()[..16.min(fp.hex().len())])
                });
                let hw = HardwareAssessment::from_os(os.as_ref(), &node_id, coordinate);
                generate_genesis_proof(&hw, epoch)
            }
            Err(_) => {
                // Fallback: no OS access — synthesize an empty (zero-source)
                // fingerprint. Capture still "happens" (records zero sources);
                // enforcement gates elsewhere refuse to trust it.
                let device_fingerprint =
                    crate::os_integration::DeviceFingerprint::compose(Default::default());
                let node_id = device_node_id.map(|s| s.to_string()).unwrap_or_else(|| {
                    format!(
                        "genesis_({},{},{})",
                        coordinate.x, coordinate.y, coordinate.z
                    )
                });
                let hw = HardwareAssessment {
                    cpu_cores: num_cpus::get() as u32,
                    cpu_mhz: 1000,
                    memory_bytes: 4 * 1024 * 1024 * 1024,
                    storage_bytes: 50 * 1024 * 1024 * 1024,
                    storage_available_bytes: 25 * 1024 * 1024 * 1024,
                    node_id,
                    coordinate,
                    device_fingerprint,
                    disk_serial: None,
                };
                generate_genesis_proof(&hw, epoch)
            }
        }
    }

    /// Compute the BLAKE3 hash of all entries (deterministic commitment).
    ///
    /// Hashes the concatenation of `(asset_hash || proof_hash)` for each entry.
    /// This is deterministic regardless of serialization format.
    pub fn compute_entries_hash(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        for entry in &self.entries {
            hasher.update(&entry.asset_hash);
            hasher.update(&entry.proof_hash);
        }
        *hasher.finalize().as_bytes()
    }

    /// Extract a lightweight header from this block.
    pub fn header(&self) -> BlockHeader {
        BlockHeader {
            index: self.index,
            hash: self.hash.clone(),
            previous_hash: self.previous_hash.clone(),
            entries_hash: self.compute_entries_hash(),
            entry_count: self.entries.len(),
        }
    }

    /// Verify that this block matches a given header.
    pub fn verify_against_header(&self, header: &BlockHeader) -> bool {
        self.index == header.index
            && self.hash == header.hash
            && self.previous_hash == header.previous_hash
            && self.entries.len() == header.entry_count
            && self.compute_entries_hash() == header.entries_hash
    }

    /// Calculate the hash of this block using BLAKE3.
    ///
    /// `block_hash = BLAKE3(index || prev_hash || entries...)`
    pub fn calculate_hash(&self) -> String {
        let mut hasher = Hasher::new();

        hasher.update(&self.index.to_le_bytes());
        hasher.update(self.previous_hash.as_bytes());

        for entry in &self.entries {
            hasher.update(&entry.asset_hash);
            hasher.update(&entry.proof_hash);
        }

        format!("{}", hasher.finalize())
    }

    /// Verify the block's hash is correct
    pub fn verify_hash(&self) -> bool {
        self.hash == self.calculate_hash()
    }

    /// Check if this is a genesis block
    pub fn is_genesis(&self) -> bool {
        self.index == 0
            && self.previous_hash
                == "0000000000000000000000000000000000000000000000000000000000000000"
    }

    /// Get the block size in bytes (estimate)
    pub fn size(&self) -> usize {
        8 + // index
        64 + // previous_hash
        64 + // hash
        self.entries.len() * (32 + 32 + 256 + 64) // entry estimate (hA + hπ + proof + ptr)
    }

    /// Get asset registrations from entries (compatibility helper)
    pub fn get_assets(&self) -> Vec<&AssetRegistration> {
        self.entries.iter().map(|e| &e.registration).collect()
    }

    /// Get the number of asset entries
    pub fn asset_count(&self) -> usize {
        self.entries.len()
    }

    /// Check if this block's recorded matrix cell matches the given coordinate.
    ///
    /// Device-auth invariant: under device-bound genesis, `PoSpace.node_id`
    /// is the DEVICE identity (not the coordinate). The cell is a DERIVED
    /// attribute recorded inside `PoSpace.storage_path` as `#cell=(x,y,z)`
    /// (see `genesis_proof::build_space_proof`). This method checks that
    /// derived cell.
    ///
    /// It also accepts the legacy encoding (`PoSpace.node_id == "(x,y,z)"`)
    /// so blocks produced before the device-auth binding still validate —
    /// keeping the change reversible and back-compatible.
    pub fn belongs_to_node(&self, node_coordinate: &MatrixCoordinate) -> bool {
        let Some(first) = self.entries.first() else {
            return false;
        };
        let coord_str = format!(
            "({},{},{})",
            node_coordinate.x, node_coordinate.y, node_coordinate.z
        );
        let space = &first.state_proof.space_proof;
        // Preferred: derived cell recorded in storage_path (`#cell=(x,y,z)`).
        let cell_marker = format!("#cell={coord_str}");
        space.storage_path.contains(&cell_marker)
            // Legacy: coordinate stored directly as the PoSpace node_id.
            || space.node_id == coord_str
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Block #{} | {} entries | Hash: {}...{}",
            self.index,
            self.entries.len(),
            &self.hash[..8.min(self.hash.len())],
            &self.hash[self.hash.len().saturating_sub(8)..]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_genesis_entry(coord: MatrixCoordinate) -> BlockAssetEntry {
        let reg = AssetRegistration::genesis(coord);
        let content_hash = *blake3::hash(reg.to_string().as_bytes()).as_bytes();
        let state_proof = StateProof::default();
        let proof_bytes = serde_json::to_vec(&state_proof).unwrap_or_default();
        let proof_hash = *blake3::hash(&proof_bytes).as_bytes();

        BlockAssetEntry {
            asset_hash: content_hash,
            proof_hash,
            state_proof,
            signed_proof: None,
            storage_pointer: StoragePointer::Genesis,
            registration: reg,
        }
    }

    #[test]
    fn test_genesis_block_creation() {
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coord");
        let genesis = Block::genesis(coord);

        assert_eq!(genesis.index, 0);
        assert!(genesis.is_genesis());
        assert!(genesis.verify_hash());
        assert_eq!(genesis.asset_count(), 1);
    }

    /// HASH-SAFETY FIXTURE (device-auth invariant / P2).
    ///
    /// `Block::calculate_hash` commits ONLY to
    /// `index || previous_hash || (asset_hash || proof_hash)*`. The
    /// device-auth change alters PROOF DERIVATION, which changes the
    /// `state_proof` content and therefore `proof_hash` for FRESH genesis
    /// blocks — but the HASHING ALGORITHM is unchanged, so a fixed `Block`
    /// value re-hashes byte-identically before and after.
    ///
    /// This pins the algorithm to a known-answer digest. If any future edit
    /// changes what `calculate_hash` commits to (field order, added fields,
    /// hash function), this test fails loudly — proving old persisted blocks
    /// still deserialize and re-hash identically (no block wipe, reversible).
    #[test]
    fn calculate_hash_is_byte_stable_for_fixed_block() {
        // Fully fixed inputs — no SystemTime, no nonce, no randomness.
        let entry = BlockAssetEntry {
            asset_hash: [0x11u8; 32],
            proof_hash: [0x22u8; 32],
            state_proof: StateProof::default(),
            signed_proof: None,
            storage_pointer: StoragePointer::Genesis,
            registration: AssetRegistration::genesis(
                MatrixCoordinate::new(0, 0, 0).expect("test: valid coord"),
            ),
        };
        let previous_hash =
            "0000000000000000000000000000000000000000000000000000000000000000".to_string();

        // Compute the expected digest exactly as `calculate_hash` must:
        // index(LE) || previous_hash bytes || (asset_hash || proof_hash).
        let expected = {
            let mut h = Hasher::new();
            h.update(&7u64.to_le_bytes());
            h.update(previous_hash.as_bytes());
            h.update(&[0x11u8; 32]);
            h.update(&[0x22u8; 32]);
            format!("{}", h.finalize())
        };

        let block = Block {
            index: 7,
            previous_hash: previous_hash.clone(),
            hash: String::new(),
            entries: vec![entry],
        };

        // (1) The algorithm output must equal the independently-derived
        //     digest — pins field order + hash function.
        assert_eq!(
            block.calculate_hash(),
            expected,
            "calculate_hash algorithm changed — device-auth hash-safety broken"
        );

        // (2) The digest must NOT depend on state_proof/registration content
        //     (only on asset_hash || proof_hash). This is precisely what makes
        //     the device-auth proof-derivation change hash-safe: a fresh
        //     genesis with different proof content re-hashes to the same block
        //     hash as long as asset_hash + proof_hash are held fixed.
        let mut block_other_proof = block.clone();
        block_other_proof.entries[0].state_proof = StateProof::new_for_testing();
        block_other_proof.entries[0].registration = AssetRegistration::genesis(
            MatrixCoordinate::new(9, 9, 9).expect("test: valid coord"),
        );
        assert_eq!(
            block.calculate_hash(),
            block_other_proof.calculate_hash(),
            "calculate_hash must ignore state_proof/registration content \
             (commits only to asset_hash || proof_hash)"
        );

        // (3) Deterministic across calls — no hidden state.
        assert_eq!(block.calculate_hash(), block.calculate_hash());

        // (4) H3: the FALCON `signed_proof` envelope is EXCLUDED from
        //     calculate_hash. Attaching a signed_proof to an entry must NOT
        //     change the block hash — otherwise adding this field would have
        //     rewritten every existing block's canonical hash (chain wipe).
        let mut block_signed = block.clone();
        block_signed.entries[0].signed_proof = Some(WireSignedProof {
            proof_bytes: b"any-proof-bytes".to_vec(),
            signature: vec![0xAB; 64],
            signer_pubkey: vec![0xCD; 128],
            nonce: [0x33u8; 32],
        });
        assert_eq!(
            block.calculate_hash(),
            block_signed.calculate_hash(),
            "calculate_hash must ignore signed_proof (H3 envelope excluded — \
             no existing block hash may change when the field is added)"
        );
    }

    #[test]
    fn test_block_creation() {
        let coord = MatrixCoordinate::new(5, 5, 5).expect("test: valid coord");
        let entry = test_genesis_entry(coord);
        let prev_hash = "abc123".to_string();

        let block = Block::new(1, vec![entry.clone()], prev_hash.clone());

        assert_eq!(block.index, 1);
        assert_eq!(block.entries.len(), 1);
        assert_eq!(block.entries[0], entry);
        assert_eq!(block.previous_hash, prev_hash);
        assert!(!block.hash.is_empty());
        assert!(block.verify_hash());
    }

    #[test]
    fn test_hash_verification() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let entry = test_genesis_entry(coord);
        let mut block = Block::new(1, vec![entry], "prev".to_string());

        assert!(block.verify_hash());

        // Tamper with an entry's asset hash
        block.entries[0].asset_hash = [0xFFu8; 32];
        assert!(!block.verify_hash());

        // Fix the hash
        block.hash = block.calculate_hash();
        assert!(block.verify_hash());
    }

    #[test]
    fn test_deterministic_hash() {
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coord");
        let entry = test_genesis_entry(coord);

        // Same content = same hash (no nonce, no timestamp on block)
        let block1 = Block::new(1, vec![entry.clone()], "prev".to_string());
        let block2 = Block::new(1, vec![entry.clone()], "prev".to_string());

        assert_eq!(block1.hash, block2.hash);
    }

    #[test]
    fn test_block_size() {
        let entries: Vec<BlockAssetEntry> = (0..10)
            .map(|i| {
                test_genesis_entry(
                    MatrixCoordinate::new(i, i, i).expect("test: valid coord"),
                )
            })
            .collect();

        let block = Block::new(100, entries, "x".repeat(64));

        let size = block.size();
        assert!(size >= 320);
    }

    #[test]
    fn test_block_display() {
        let coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coord");
        let block = Block::genesis(coord);

        let display = format!("{block}");
        assert!(display.contains("Block #0"));
        assert!(display.contains("1 entries"));
    }

    #[test]
    fn test_serialization() {
        let coord = MatrixCoordinate::new(7, 8, 9).expect("test: valid coord");
        let block = Block::genesis(coord);

        let json = serde_json::to_string(&block).expect("test: serialize");
        assert!(json.contains("\"index\":0"));

        let decoded: Block = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(block, decoded);
    }

    #[test]
    fn test_genesis_block_properties() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let genesis = Block::genesis(coord);

        assert_eq!(genesis.index, 0);
        assert!(genesis.is_genesis());
        assert_eq!(
            genesis.previous_hash,
            "0000000000000000000000000000000000000000000000000000000000000000"
        );
        assert_eq!(genesis.asset_count(), 1);
        assert!(genesis.verify_hash());
    }

    #[test]
    fn test_block_must_have_entries() {
        let result = std::panic::catch_unwind(|| Block::new(1, vec![], "prev".to_string()));
        assert!(
            result.is_err(),
            "Block creation with empty entries should panic"
        );
    }

    #[test]
    fn test_multiple_entries() {
        let entries: Vec<BlockAssetEntry> = (0..5)
            .map(|i| {
                test_genesis_entry(
                    MatrixCoordinate::new(i, i, i).expect("test: valid coord"),
                )
            })
            .collect();

        let block = Block::new(1, entries, "prev".to_string());

        assert_eq!(block.asset_count(), 5);
        assert_eq!(block.get_assets().len(), 5);
    }

    // --- BlockHeader tests ---

    #[test]
    fn test_block_header_round_trip() {
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coord");
        let entry = test_genesis_entry(coord);
        let block = Block::new(1, vec![entry], "prev".to_string());

        let header = block.header();
        assert!(block.verify_against_header(&header));
    }

    #[test]
    fn test_block_header_verify_fails_different_entries() {
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coord");
        let entry1 = test_genesis_entry(coord);
        let block = Block::new(1, vec![entry1], "prev".to_string());
        let header = block.header();

        // Build a different block with same index/prev but different entry
        let coord2 = MatrixCoordinate::new(4, 5, 6).expect("test: valid coord");
        let entry2 = test_genesis_entry(coord2);
        let block2 = Block::new(1, vec![entry2], "prev".to_string());

        assert!(!block2.verify_against_header(&header));
    }

    #[test]
    fn test_block_header_chains_to_sequential() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let entry = test_genesis_entry(coord);

        let block0 = Block::new(0, vec![entry.clone()], "genesis".to_string());
        let block1 = Block::new(1, vec![entry], block0.hash.clone());

        let h0 = block0.header();
        let h1 = block1.header();

        assert!(h1.chains_to(&h0));
    }

    #[test]
    fn test_block_header_chains_to_fails_non_sequential() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let entry = test_genesis_entry(coord);

        let block0 = Block::new(0, vec![entry.clone()], "genesis".to_string());
        let block2 = Block::new(2, vec![entry], block0.hash.clone());

        let h0 = block0.header();
        let h2 = block2.header();

        // Index gap: 2 != 0 + 1
        assert!(!h2.chains_to(&h0));
    }

    #[test]
    fn test_block_header_chains_to_fails_wrong_hash() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let entry = test_genesis_entry(coord);

        let block0 = Block::new(0, vec![entry.clone()], "genesis".to_string());
        let block1 = Block::new(1, vec![entry], "wrong_prev_hash".to_string());

        let h0 = block0.header();
        let h1 = block1.header();

        assert!(!h1.chains_to(&h0));
    }

    #[test]
    fn test_block_header_entries_hash_deterministic() {
        let coord = MatrixCoordinate::new(3, 3, 3).expect("test: valid coord");
        let entry = test_genesis_entry(coord);

        let block = Block::new(1, vec![entry], "prev".to_string());
        let hash1 = block.compute_entries_hash();
        let hash2 = block.compute_entries_hash();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_block_header_genesis_block() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let genesis = Block::genesis(coord);
        let header = genesis.header();

        assert_eq!(header.index, 0);
        assert_eq!(header.entry_count, 1);
        assert_eq!(header.hash, genesis.hash);
        assert!(genesis.verify_against_header(&header));
    }

    #[test]
    fn test_storage_pointer_variants() {
        let coord = MatrixCoordinate::new(1, 1, 1).expect("test: valid coord");
        let mut entry = test_genesis_entry(coord);

        // Test Local pointer
        entry.storage_pointer = StoragePointer::Local {
            path: "/data/assets/abc123".to_string(),
        };
        let block = Block::new(1, vec![entry.clone()], "prev".to_string());
        assert!(block.verify_hash());

        // Test Sharded pointer
        entry.storage_pointer = StoragePointer::Sharded {
            shard_hashes: vec![[1u8; 32], [2u8; 32]],
            placements: vec![
                MatrixCoordinate::new(1, 0, 0).expect("test: valid"),
                MatrixCoordinate::new(0, 1, 0).expect("test: valid"),
            ],
        };
        let block2 = Block::new(2, vec![entry], "prev2".to_string());
        assert!(block2.verify_hash());
    }
}
