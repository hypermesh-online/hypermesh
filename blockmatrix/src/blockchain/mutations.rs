// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Block addition and asset registration methods for `NodeBlockchain`.
//!
//! All mutation methods that create new blocks or insert received blocks
//! live here.  Core chain state and queries are in [`super::chain`].

use tracing::{info, warn};

use super::block::{Block, BlockAssetEntry, StoragePointer};
use super::chain::NodeBlockchain;
use crate::assets::core::AssetRegistration;
use crate::proof_of_state::validation_service::StateProofValidationService;
use trustchain::proof_of_state::StateProof;

/// H3 one-release migration flag: tolerate a received entry that carries NO
/// `signed_proof` envelope (legacy pre-H3 block) instead of rejecting it.
///
/// Default is REJECT-UNSIGNED — the secure end state. Set
/// `HYPERMESH_ACCEPT_UNSIGNED_BLOCKS=1` for a single migration release to let
/// legacy `None` entries through (they still pass the existing structural
/// checks). A PRESENT-but-INVALID or MIS-BOUND envelope is ALWAYS rejected,
/// flag or not — the flag only governs absence.
fn accept_unsigned_blocks() -> bool {
    std::env::var("HYPERMESH_ACCEPT_UNSIGNED_BLOCKS")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// H3 accept-path binding: the FALCON key that signed the envelope must be the
/// SAME identity the entry claims as author. The collapsed-identity model sets
/// `stake_proof.stake_holder_id = BLAKE3(falcon_pubkey)` hex (== node_id), so
/// we bind `hex(BLAKE3(signer_pubkey))` to that claimed WHO. A valid signature
/// from key K over a proof claiming a DIFFERENT author is rejected — otherwise
/// any node could sign a proof asserting someone else's stake.
///
/// `pub(crate)` because S3.4's foreign asset-chain accept
/// ([`super::foreign`]) asks the identical question of every entry in an
/// imported history. Sharing the function — rather than restating the rule —
/// is what keeps "who may author an entry" a single definition in this crate.
pub(crate) fn signer_binds_to_author(signer_pubkey: &[u8], entry: &BlockAssetEntry) -> bool {
    let derived = blake3::hash(signer_pubkey).to_hex().to_string();
    entry.state_proof.stake_proof.stake_holder_id == derived
}

/// S3.2 accept-side check for ONE entry against the lineage state we already
/// have for its asset (`expected` = `(lineage_id, asset_seq)` of the last entry
/// we recorded, or `None` if the asset is unknown here).
///
/// Kept out of [`NodeBlockchain::verify_block_lineage`] so the walk stays short
/// and the rejection rules read as one flat decision table.
fn check_entry_lineage(
    entry: &BlockAssetEntry,
    expected: Option<(String, u64)>,
    block_index: u64,
    entry_ix: usize,
) -> Result<(), String> {
    let Some((head_id, head_seq)) = expected else {
        // Asset unknown here: only a proper asset-genesis may enter. Anything
        // else is a foreign asset-chain — S3.4's job, explicitly rejected until
        // then rather than accepted with unverifiable provenance.
        if entry.is_asset_genesis() {
            return Ok(());
        }
        return Err(format!(
            "Block {block_index} entry {entry_ix} carries a FOREIGN asset-chain: it claims \
             predecessor {:?} at seq {} for an asset whose history this container has never \
             seen. Verifying and grafting a foreign asset-chain is S3.4 — rejected until then \
             (never silently accepted with unverifiable provenance)",
            entry.prev_asset_entry(),
            entry.asset_seq(),
        ));
    };

    match entry.prev_asset_entry() {
        Some(claimed) if claimed == head_id => {}
        Some(claimed) => {
            return Err(format!(
                "Block {block_index} entry {entry_ix} asset lineage broken: claims predecessor \
                 {} but our recorded head for this asset is {} — mirror rejected",
                &claimed[..16.min(claimed.len())],
                &head_id[..16.min(head_id.len())],
            ));
        }
        None => {
            // F1: `head_seq` here may come from the asset's surviving
            // high-water TOMBSTONE rather than a held entry — an asset whose
            // entries we pruned is still an asset we have seen, so re-rooting
            // it is refused exactly as if we still held the bodies.
            return Err(format!(
                "Block {block_index} entry {entry_ix} asset lineage broken: claims to be an \
                 asset-genesis for an asset this container has already seen (head seq \
                 {head_seq}) — mirror rejected",
            ));
        }
    }

    // F5: fail closed on overflow. A head at `u64::MAX` has NO valid successor;
    // `head_seq + 1` would wrap to 0 and let a re-rooted entry read as a
    // continuation.
    let Some(expected_seq) = head_seq.checked_add(1) else {
        return Err(format!(
            "Block {block_index} entry {entry_ix} asset lineage broken: our head seq is \
             u64::MAX — no successor sequence exists, mirror rejected",
        ));
    };
    if entry.asset_seq() != expected_seq {
        return Err(format!(
            "Block {block_index} entry {entry_ix} asset lineage broken: asset_seq {} is not \
             our head seq {head_seq} + 1 — mirror rejected",
            entry.asset_seq(),
        ));
    }

    Ok(())
}

impl NodeBlockchain {
    /// Add a new block containing the given entries.
    ///
    /// Each `BlockAssetEntry` carries its own `StateProof` which is
    /// validated independently.  The block is built, structurally
    /// validated, and inserted.
    ///
    /// CONCURRENCY (S3.0 QA follow-up, FIX 2): the sequence
    /// read-head → derive index → sign → validate → insert runs under the
    /// chain's `append_lock` head reservation. Without it every concurrent
    /// caller reads the same head, derives the same index, and all but one
    /// fails the duplicate-index check inside `insert_block` — silently
    /// discarding an already-built, already-FALCON-signed block. S3.0's
    /// durable write-through widened that window by the fsync duration
    /// (2 of 8 concurrent writers survived; now 8 of 8).
    ///
    /// Serialising is the right shape rather than retry-on-collision: appends
    /// to a single linear chain are inherently serial (index N+1 is not
    /// derivable until N exists), a retry loop would have to re-sign on every
    /// attempt (FALCON-1024 is the expensive step) and offers no liveness
    /// guarantee under sustained contention, and — critically — with the
    /// reservation the H3 signature is produced exactly once, on the index the
    /// block is actually inserted at.
    ///
    /// The reservation does NOT cover `insert_received_block`: a received
    /// block arrives with its index already fixed by its producer, so it
    /// cannot be re-indexed. A local append that collides with a concurrently
    /// received block still returns a real error to its caller — never a
    /// silent drop.
    pub async fn add_block(
        &self,
        mut entries: Vec<BlockAssetEntry>,
    ) -> Result<Block, String> {
        if entries.is_empty() {
            return Err("Cannot add block with zero entries".to_string());
        }

        // 1. Validate every entry's state proof (binary: pass or fail) and its
        //    signed-to-content binding (mirror invariant, P1): the proof MUST
        //    reference the entry's asset_hash via SpaceProof.file_hash.
        for (i, entry) in entries.iter().enumerate() {
            if !entry.content_binding_ok() {
                return Err(format!(
                    "Entry {i} proof not bound to its asset_hash (signed-to-content violation)"
                ));
            }
            self.state_proof_validator
                .validate(&entry.state_proof)
                .map_err(|e| {
                    format!("State proof validation failed for entry {i}: {e}")
                })?;
        }

        // FIX 2 — head reservation. Everything from here to the insert must be
        // serialised: the derived index, the H3 signature produced for it, and
        // the insert that claims it. Held across `insert_block`, which takes
        // the chain write locks itself — we hold NONE of them here, so the
        // documented order (append_lock → blocks → headers → hash_index → head
        // → stats) is respected and no cycle exists. Entry proof validation
        // above deliberately stays OUTSIDE the reservation: it is pure,
        // index-independent CPU work.
        let _append_reservation = self.append_lock.lock().await;

        // 1a. S3.2 — ASSET LINEAGE stamping. This is the single write
        //     chokepoint for locally-produced entries: every production
        //     producer (`add_block_with_data`, `register_asset_record(s)`,
        //     `register_dns_asset`, `add_key_rotation_block`, the IPC store /
        //     shard / auth / dashboard handlers, the gateway bridges) reaches
        //     the chain through `add_block`. Stamping here — under the head
        //     reservation, so the asset head we read cannot move — and BEFORE
        //     H3 signing means the FALCON envelope covers the lineage.
        self.stamp_asset_lineage(&mut entries).await?;

        // 1b. H3 — single local-write signing chokepoint. When this chain has a
        //     node signer (live daemon), attach a FALCON-1024 `signed_proof`
        //     envelope to EVERY entry over its (already content-bound)
        //     `state_proof`. Produced blocks then carry an identity-bound,
        //     verifiable envelope to peers; `insert_received_block` on the
        //     remote node FALCON-verifies it. Chains without a signer
        //     (dev/test/library) leave `signed_proof = None`.
        if let Some(signer) = self.signer.as_ref() {
            for (i, entry) in entries.iter_mut().enumerate() {
                entry.sign_proof(signer.as_ref()).map_err(|e| {
                    format!("Failed to FALCON-sign proof for entry {i}: {e}")
                })?;
            }
        }

        let head = self.head.read().await;
        let previous = head
            .as_ref()
            .ok_or_else(|| "No head block found".to_string())?;

        let new_index = previous.index + 1;
        let new_block = Block::new(new_index, entries, previous.hash.clone());

        let previous_clone = previous.clone();
        drop(head); // Release read lock

        // 2. Validate block structure (hash linkage, size)
        if !self
            .validator
            .validate_block(&new_block, Some(&previous_clone))
        {
            return Err("Block structural validation failed".to_string());
        }

        // 3. Insert validated block
        self.insert_block(new_block.clone()).await?;

        info!(
            "Added block #{} to node ({},{},{}) chain",
            new_index,
            self.node_coordinate.x,
            self.node_coordinate.y,
            self.node_coordinate.z,
        );

        Ok(new_block)
    }

    /// S3.2 — stamp asset lineage onto every entry of a block being appended.
    ///
    /// For each entry: read the asset's current head from the S3.1
    /// [`AssetChainIndex`](super::asset_index::AssetChainIndex) (O(1)), and set
    /// `prev_asset_entry = head.lineage_id()`, `asset_seq = head.asset_seq + 1`.
    /// An asset this container has never seen becomes an ASSET GENESIS
    /// (`prev = None, seq = 0`).
    ///
    /// A single block may carry several entries for the SAME asset
    /// (`register_asset_records` batches), so the in-block continuation is
    /// tracked locally: the second entry for an asset succeeds the first
    /// entry of the same block, not the on-chain head.
    ///
    /// Called under the head reservation (`append_lock`), so the head this
    /// reads cannot move before the block is inserted.
    async fn stamp_asset_lineage(
        &self,
        entries: &mut [BlockAssetEntry],
    ) -> Result<(), String> {
        use std::collections::HashMap;

        // asset_hash -> (lineage_id, asset_seq) of the last entry stamped for
        // that asset within THIS block.
        let mut in_block: HashMap<[u8; 32], (String, u64)> = HashMap::new();

        for entry in entries.iter_mut() {
            let predecessor = match in_block.get(&entry.asset_hash) {
                Some(previous) => Some(previous.clone()),
                None => self.asset_lineage_head(&entry.asset_hash).await.map_err(
                    |locator| {
                        format!(
                            "asset head at block {} entry {} is no longer held in full \
                             — refusing to append without its lineage",
                            locator.block_index, locator.entry_ix,
                        )
                    },
                )?,
            };

            match predecessor {
                Some((prev_id, prev_seq)) => {
                    // F5: fail closed rather than wrapping to 0 — an asset
                    // chain is never re-rooted by overflowing its counter.
                    let next_seq = prev_seq.checked_add(1).ok_or_else(|| {
                        "asset_seq is u64::MAX — no successor sequence exists, \
                         refusing to append"
                            .to_string()
                    })?;
                    entry.set_asset_lineage(Some(prev_id), next_seq);
                }
                None => entry.set_asset_lineage(None, 0),
            }

            in_block.insert(entry.asset_hash, (entry.lineage_id(), entry.asset_seq()));
        }

        Ok(())
    }

    /// S3.2 — accept-side asset-lineage verification for a RECEIVED block.
    ///
    /// Runs immediately before the block is inserted (never on the
    /// orphan-buffering path: an orphan's asset predecessor may well live in
    /// the predecessor block that has not arrived yet — it is re-checked when
    /// the orphan is drained and actually linked).
    ///
    /// Two cases, both fail-closed:
    ///
    /// - **Asset known locally** — the entry must be a proper successor of OUR
    ///   recorded head for that asset: `prev_asset_entry == head.lineage_id()`
    ///   AND `asset_seq == head.asset_seq + 1`. A forged prev-pointer, a
    ///   re-rooted "asset genesis" for an asset we already hold, or a skipped /
    ///   replayed sequence number is REJECTED.
    /// - **Asset unknown locally** — only a proper asset genesis
    ///   (`prev = None, seq = 0`) is accepted. Anything else is a FOREIGN
    ///   ASSET-CHAIN whose history we have never seen; verifying and grafting
    ///   such a chain is **S3.4**, and until then it is rejected explicitly
    ///   rather than silently accepted with unverifiable provenance.
    ///
    /// In-block continuation is handled exactly as on the write side: a second
    /// entry for the same asset within one block succeeds the first.
    async fn verify_block_lineage(&self, block: &Block) -> Result<(), String> {
        use std::collections::HashMap;

        // asset_hash -> (lineage_id, asset_seq) of the last entry SEEN for that
        // asset: from earlier in this same block if present, otherwise our
        // recorded on-chain head.
        let mut seen: HashMap<[u8; 32], (String, u64)> = HashMap::new();

        for (i, entry) in block.entries.iter().enumerate() {
            let expected = match seen.get(&entry.asset_hash) {
                Some(previous) => Some(previous.clone()),
                None => self.recorded_asset_head(&entry.asset_hash, block.index, i).await?,
            };

            check_entry_lineage(entry, expected, block.index, i)?;

            seen.insert(entry.asset_hash, (entry.lineage_id(), entry.asset_seq()));
        }

        Ok(())
    }

    /// The `(lineage_id, asset_seq)` of our recorded head for `asset_hash`, or
    /// `None` when this container has never seen the asset.
    ///
    /// F1: "never seen" means never — [`asset_lineage_head`] falls back to the
    /// asset's surviving high-water tombstone when pruning removed the entry
    /// bodies, so a pruned asset is still recognised and a fresh `(None, 0)`
    /// genesis for it is still rejected.
    ///
    /// Errors only when the head is indexed but its block is no longer held in
    /// full — we then cannot judge lineage at all, and fail closed.
    ///
    /// [`asset_lineage_head`]: NodeBlockchain::asset_lineage_head
    async fn recorded_asset_head(
        &self,
        asset_hash: &[u8; 32],
        block_index: u64,
        entry_ix: usize,
    ) -> Result<Option<(String, u64)>, String> {
        self.asset_lineage_head(asset_hash).await.map_err(|locator| {
            format!(
                "Block {block_index} entry {entry_ix}: our head for this asset (block {}) \
                 is no longer held in full — cannot verify lineage, mirror rejected",
                locator.block_index,
            )
        })
    }

    /// Create an asset from raw data and add it as a block.
    pub async fn add_block_with_data(
        &self,
        data: Vec<u8>,
        state_proof: &StateProof,
    ) -> Result<Block, String> {
        use crate::assets::core::asset_id::{
            AssetCategory, AssetData, BaseSystemType, NetworkScope,
        };

        let asset_data = AssetData {
            config: Vec::new(),
            definition: data.clone(),
            metadata: "Block data".to_string().into_bytes(),
        };

        let registration = AssetRegistration::from_asset_data(
            &asset_data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Container),
        );

        let asset_hash = *blake3::hash(&data).as_bytes();

        // Bind the proof to the content hash (signed-to-content invariant, P1).
        let entry = BlockAssetEntry::new_bound(
            asset_hash,
            state_proof,
            StoragePointer::Local {
                path: String::new(),
            },
            registration,
        );

        self.add_block(vec![entry]).await
    }

    /// Accept a mirror: insert a block received from a peer, zero-trust.
    ///
    /// Mirror invariant (P1) + accept-a-mirror refactor (F7) + sync-fetch
    /// symmetry (task #22.1): a received block enters this chain ONLY with
    /// authenticated, verified linkage AND full per-entry proof integrity.
    /// This is now the SINGLE symmetric gate — both the block-announce path
    /// (`parse_and_verify_block`) and the reflector sync-fetch path
    /// (`fetch_blocks` → here) run the identical per-entry checks, so a
    /// block cannot enter via one path with weaker verification than the
    /// other. Nothing is ever spliced in on a non-matching/missing
    /// predecessor:
    ///
    /// 1. Block hash must recompute (`verify_hash`).
    /// 2. Every entry's `proof_hash` must equal `BLAKE3(serialize(proof))`
    ///    (integrity — task #22.1, previously only the announce path did this).
    /// 3. Every entry's proof must be bound to its `asset_hash`
    ///    (signed-to-content: `content_binding_ok`).
    /// 4. Every entry's `state_proof.validate()` must pass (task #22.1,
    ///    previously only the announce path did this).
    /// 5. Linkage (for non-genesis):
    ///    - Predecessor present + hash matches → insert, then drain any orphan
    ///      that was waiting on THIS block.
    ///    - Predecessor present + hash does NOT match → **hard reject** (this
    ///      includes the former cross-genesis block-1 warn-insert graft hole).
    ///    - Predecessor absent → **buffer as orphan** (do not insert) until a
    ///      verified predecessor with the matching hash arrives.
    pub async fn insert_received_block(&self, block: Block) -> Result<(), String> {
        if !block.verify_hash() {
            return Err(format!(
                "Block {} hash mismatch: expected {}, got {}",
                block.index,
                block.calculate_hash(),
                block.hash,
            ));
        }

        // Per-entry proof integrity + binding + validity. Task #22.1: these
        // checks previously lived ONLY in the announce path
        // (`parse_and_verify_block`); the reflector sync-fetch path
        // (`fetch_blocks`) verified only the block hash. Centralizing them
        // here makes BOTH paths symmetric — a fetched block is now held to
        // the exact same standard as an announced one.
        for (i, entry) in block.entries.iter().enumerate() {
            // (a) proof_hash integrity: hπ == BLAKE3(serialize(state_proof)).
            let proof_bytes = serde_json::to_vec(&entry.state_proof).map_err(|e| {
                format!(
                    "Block {} entry {i} proof serialization failed — mirror rejected: {e}",
                    block.index,
                )
            })?;
            let computed_hash: [u8; 32] = blake3::hash(&proof_bytes).into();
            if computed_hash != entry.proof_hash {
                return Err(format!(
                    "Block {} entry {i} proof_hash mismatch \
                     (integrity violation) — mirror rejected",
                    block.index,
                ));
            }

            // (b) Signed-to-content: reject any mirror whose proof is not
            // bound to the content it claims. A valid proof for asset A must
            // not be replayable inside an entry claiming asset B.
            if !entry.content_binding_ok() {
                return Err(format!(
                    "Block {} entry {i} proof not bound to its asset_hash \
                     (signed-to-content violation) — mirror rejected",
                    block.index,
                ));
            }

            // (c) State proof validity: the four-proof StateProof must pass
            // its own structural/temporal validation.
            if !entry.state_proof.validate() {
                return Err(format!(
                    "Block {} entry {i} state proof validation failed — mirror rejected",
                    block.index,
                ));
            }

            // (d) H3 — FALCON-in-block PoS verification (the untrusted-remote
            // path). Previously block-accept was STRUCTURAL only because the
            // entry carried no signature. Now every received entry MUST carry a
            // `signed_proof` envelope that:
            //   1. wraps THIS entry's state_proof (no bait-and-switch),
            //   2. verifies under FALCON-1024 against its embedded pubkey,
            //   3. is signed by the SAME identity the proof claims as author
            //      (BLAKE3(signer_pubkey) == stake_holder_id).
            // Absent envelope → rejected UNLESS the one-release compat flag is
            // set (legacy migration). Present-but-invalid/mis-bound → ALWAYS
            // rejected.
            match &entry.signed_proof {
                Some(_) => {
                    let signer_pubkey = entry.verify_signed_proof().map_err(|e| {
                        format!(
                            "Block {} entry {i} signed_proof invalid — mirror rejected: {e}",
                            block.index,
                        )
                    })?;
                    if !signer_binds_to_author(&signer_pubkey, entry) {
                        return Err(format!(
                            "Block {} entry {i} signed_proof signer does not match \
                             claimed author (BLAKE3(pubkey) != stake_holder_id) \
                             — mirror rejected",
                            block.index,
                        ));
                    }
                }
                None => {
                    if !accept_unsigned_blocks() {
                        return Err(format!(
                            "Block {} entry {i} has no FALCON signed_proof envelope \
                             — mirror rejected (set HYPERMESH_ACCEPT_UNSIGNED_BLOCKS=1 \
                             for one-release legacy migration)",
                            block.index,
                        ));
                    }
                    tracing::warn!(
                        "Block {} entry {i}: accepting UNSIGNED legacy entry \
                         (HYPERMESH_ACCEPT_UNSIGNED_BLOCKS set) — H3 migration only",
                        block.index,
                    );
                }
            }

            // S3.4: an entry carrying a `state_proof.mirror_seal` is accepted
            // here WITHOUT checking WHO sealed it. Locally,
            // `check_seal_authority` requires `sealed_by` to be an OWNER of the
            // asset AND to be this node's own signing identity; a RECEIVED entry
            // gets neither check, so a peer may hand us a block whose
            // `MirrorSeal.sealed_by` is neither the entry's claimed author
            // (`stake_proof.stake_holder_id`, already FALCON-bound above) nor an
            // owner in the asset's `AuthorizationSet`. The seal then becomes
            // on-chain, hash-committed mirror history attributed to an identity
            // that never authorised it.
            //
            // The gate belongs here, next to the H3 signer binding it depends
            // on: require `mirror_seal.sealed_by == stake_holder_id` and that
            // the identity holds the distribution right on the asset's head
            // entry. Deferred to S3.4 with the rest of the received-entry
            // authority checks, and NOT implemented in S3.3 — see the S3.5 note
            // on `check_seal_authority`: assets are ownerless at creation today,
            // so an owner check on the receive path would reject every seal.
        }

        // Genesis has no predecessor to verify — insert directly (after S3.2
        // asset-lineage verification, which applies to every entry regardless
        // of the block's position in the container spine).
        if block.index == 0 {
            self.verify_block_lineage(&block).await?;
            return self.insert_block(block).await;
        }

        // Non-genesis: require verified linkage to a known predecessor.
        let has_matching_predecessor = {
            let blocks = self.blocks.read().await;
            match blocks.get(&(block.index - 1)) {
                Some(prev) => {
                    if block.previous_hash != prev.hash {
                        // Hard reject — no warn-insert graft, including the
                        // former cross-genesis block-1 hole (F7 = hard reject).
                        return Err(format!(
                            "Block {} previous_hash {} does not match block {}'s hash {} \
                             — rejecting foreign/forked block (no chain graft)",
                            block.index,
                            &block.previous_hash[..16.min(block.previous_hash.len())],
                            block.index - 1,
                            &prev.hash[..16.min(prev.hash.len())],
                        ));
                    }
                    true
                }
                None => false,
            }
        };

        if !has_matching_predecessor {
            // Predecessor unknown → buffer as an orphan keyed by its
            // previous_hash. It is NOT in the chain until a verified
            // predecessor arrives (zero-trust: nothing enters without linkage).
            //
            // P6 (task #22.2): bound the buffer so an authenticated peer
            // cannot flood distinct-prev-hash orphans and exhaust memory.
            let mut orphans = self.orphans.write().await;
            let now = std::time::Instant::now();

            // 1. TTL sweep: drop orphans whose predecessor never showed up.
            orphans.retain(|_, (_, arrived)| {
                now.duration_since(*arrived) < super::chain::ORPHAN_TTL
            });

            // 2. Capacity cap: if still full and this is a NEW key, evict the
            //    oldest buffered orphan to make room. (Re-buffering an existing
            //    key just refreshes it below and needs no eviction.)
            if orphans.len() >= super::chain::MAX_ORPHANS
                && !orphans.contains_key(&block.previous_hash)
            {
                if let Some(oldest_key) = orphans
                    .iter()
                    .min_by_key(|(_, (_, arrived))| *arrived)
                    .map(|(k, _)| k.clone())
                {
                    warn!(
                        "Orphan buffer at capacity ({}) — evicting oldest orphan (prev={})",
                        super::chain::MAX_ORPHANS,
                        &oldest_key[..16.min(oldest_key.len())],
                    );
                    orphans.remove(&oldest_key);
                }
            }

            warn!(
                "Block {} predecessor unknown — buffering as orphan (prev={})",
                block.index,
                &block.previous_hash[..16.min(block.previous_hash.len())],
            );
            orphans.insert(block.previous_hash.clone(), (block, now));
            return Ok(());
        }

        // Container linkage verified. S3.2: the ASSET lineage of every entry
        // must also be verified before the block enters — checked here, at the
        // point of actual insertion, so an orphan that arrived early is judged
        // against the head it will really extend.
        self.verify_block_lineage(&block).await?;

        // Insert, then attempt to drain any orphan chain that was waiting on
        // this newly-inserted block.
        let inserted_hash = block.hash.clone();
        self.insert_block(block).await?;
        self.drain_orphans_from(inserted_hash).await;
        Ok(())
    }

    /// Drain buffered orphans that chain from a just-inserted block.
    ///
    /// Follows the orphan buffer forward: if an orphan's `previous_hash`
    /// matches `parent_hash`, it is now linkable — insert it and continue from
    /// its hash. Each drained orphan is re-checked for content-binding before
    /// insertion (defense in depth). Stops when no orphan links to the frontier.
    async fn drain_orphans_from(&self, mut parent_hash: String) {
        loop {
            let next = {
                let mut orphans = self.orphans.write().await;
                // P6: unwrap the (Block, Instant) tuple — the arrival
                // timestamp is buffer bookkeeping only and is discarded on
                // link. The buffer-then-link behavior is unchanged.
                orphans.remove(&parent_hash).map(|(block, _arrived)| block)
            };
            let Some(orphan) = next else { break };

            // Re-verify content binding on the orphan before it enters.
            let binding_ok = orphan.entries.iter().all(|e| e.content_binding_ok());
            if !binding_ok || !orphan.verify_hash() {
                warn!(
                    "Dropping orphan block {} on drain (failed re-verification)",
                    orphan.index,
                );
                break;
            }

            // S3.2: the orphan's asset lineage is verified HERE, not when it
            // was buffered — its asset predecessor may have arrived in the
            // block that just linked it.
            if let Err(e) = self.verify_block_lineage(&orphan).await {
                warn!("Dropping orphan block {} on drain: {e}", orphan.index);
                break;
            }

            let orphan_hash = orphan.hash.clone();
            match self.insert_block(orphan).await {
                Ok(()) => {
                    info!("Linked buffered orphan into chain (prev={})",
                        &parent_hash[..16.min(parent_hash.len())]);
                    parent_hash = orphan_hash;
                }
                Err(e) => {
                    warn!("Orphan drain insert failed: {e}");
                    break;
                }
            }
        }
    }

    /// Register an asset record on this node's blockchain.
    ///
    /// Creates a new block containing the [`AssetRegistration`], validates
    /// it against the chain, and appends it.
    pub async fn register_asset_record(
        &self,
        registration: AssetRegistration,
        state_proof: &StateProof,
    ) -> Result<Block, String> {
        info!(
            "Registering asset on blockchain at ({},{},{})",
            self.node_coordinate.x,
            self.node_coordinate.y,
            self.node_coordinate.z,
        );

        let asset_hash = registration.content_hash;

        // Bind the proof to the content hash (signed-to-content invariant, P1).
        let entry = BlockAssetEntry::new_bound(
            asset_hash,
            state_proof,
            StoragePointer::Genesis,
            registration,
        );

        self.add_block(vec![entry]).await
    }

    /// Register a DNS asset on this node's blockchain.
    ///
    /// Unlike [`register_asset_record`] which uses `StoragePointer::Genesis`,
    /// this stores the serialized DNS record JSON in `StoragePointer::Local`
    /// so that peers receiving the block can extract and resolve the name.
    pub async fn register_dns_asset(
        &self,
        registration: AssetRegistration,
        state_proof: &StateProof,
        dns_json: Vec<u8>,
    ) -> Result<Block, String> {
        info!(
            "Registering DNS asset on blockchain at ({},{},{})",
            self.node_coordinate.x,
            self.node_coordinate.y,
            self.node_coordinate.z,
        );

        let asset_hash = registration.content_hash;

        // Store serialized DnsBlockEntry in the path field so receivers
        // can deserialize it without reversing the content hash.
        let dns_payload = String::from_utf8(dns_json).unwrap_or_default();

        // Bind the proof to the content hash (signed-to-content invariant, P1).
        // Note: `asset_hash` is the registration content hash (identifies the
        // DNS asset), NOT `BLAKE3(dns_payload)`; the payload is auxiliary
        // resolver data, so it is not content-addressed by `asset_hash`.
        let entry = BlockAssetEntry::new_bound(
            asset_hash,
            state_proof,
            StoragePointer::Local { path: dns_payload },
            registration,
        );

        self.add_block(vec![entry]).await
    }

    /// Write a key rotation entry to the blockchain.
    ///
    /// Records old->new key transition with FALCON-signed proof (§6.2.2).
    /// The rotation entry is stored as a `StoragePointer::Local` payload
    /// so peers receiving the block can extract and verify the chain.
    ///
    /// The caller supplies a real `&StateProof` for the owning node
    /// (mirroring [`register_asset_records`]); this method never fabricates
    /// a proof.
    pub async fn add_key_rotation_block(
        &self,
        entry: &trustchain::identity::KeyRotationEntry,
        state_proof: &StateProof,
    ) -> Result<Block, String> {
        let entry_bytes = serde_json::to_vec(entry).map_err(|e| {
            format!("Failed to serialize key rotation entry: {e}")
        })?;
        let asset_hash = *blake3::hash(&entry_bytes).as_bytes();

        // Bind the proof to the content hash (signed-to-content invariant, P1).
        // Here the Local payload IS the content (`entry_bytes`) and
        // `asset_hash == BLAKE3(entry_bytes)`, so content-validity of the
        // payload is also directly checkable by receivers.
        let block_entry = BlockAssetEntry::new_bound(
            asset_hash,
            state_proof,
            StoragePointer::Local {
                path: String::from_utf8_lossy(&entry_bytes).to_string(),
            },
            AssetRegistration::genesis(self.node_coordinate),
        );

        self.add_block(vec![block_entry]).await
    }

    /// Register multiple asset records in a single block.
    ///
    /// Useful during genesis to batch all hardware assets into one block.
    pub async fn register_asset_records(
        &self,
        registrations: Vec<AssetRegistration>,
        state_proof: &StateProof,
    ) -> Result<Block, String> {
        if registrations.is_empty() {
            return Err("cannot register empty asset list".to_string());
        }

        info!(
            "Registering {} assets on blockchain at ({},{},{})",
            registrations.len(),
            self.node_coordinate.x,
            self.node_coordinate.y,
            self.node_coordinate.z,
        );

        // Each entry binds the proof to its OWN content hash (signed-to-content
        // invariant, P1) — so every entry carries a distinct `proof_hash`
        // derived over a proof whose `file_hash` equals that entry's asset hash.
        let entries: Vec<BlockAssetEntry> = registrations
            .into_iter()
            .map(|reg| {
                let asset_hash = reg.content_hash;
                BlockAssetEntry::new_bound(
                    asset_hash,
                    state_proof,
                    StoragePointer::Genesis,
                    reg,
                )
            })
            .collect();

        self.add_block(entries).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::coordinate::MatrixCoordinate;

    fn test_entry(coord: MatrixCoordinate) -> BlockAssetEntry {
        let reg = AssetRegistration::genesis(coord);
        let content_hash = *blake3::hash(reg.to_string().as_bytes()).as_bytes();
        // Bind the proof to the content hash so the entry satisfies the
        // signed-to-content invariant (P1) enforced at insert.
        BlockAssetEntry::new_bound(
            content_hash,
            &StateProof::new_for_testing(),
            StoragePointer::Genesis,
            reg,
        )
    }

    /// H3 test helper: a fully valid RECEIVED entry — content-bound, proof
    /// author = signer identity, and FALCON `signed_proof` attached. This is
    /// what an honest peer produces via `add_block` with a signer. The proof's
    /// `stake_holder_id` is set to the signer's `node_id` (BLAKE3(pubkey)) so
    /// the accept-path author binding holds.
    fn signed_test_entry(coord: MatrixCoordinate) -> BlockAssetEntry {
        use hypermesh_lib::NodeSigner;
        let id = trustchain::identity::FalconIdentity::generate();
        let reg = AssetRegistration::genesis(coord);
        let content_hash = *blake3::hash(reg.to_string().as_bytes()).as_bytes();
        let mut proof = StateProof::new_for_testing();
        // Bind WHO to the signer identity so BLAKE3(pubkey) == stake_holder_id.
        proof.stake_proof.stake_holder_id = id.node_id().to_string();
        let mut entry =
            BlockAssetEntry::new_bound(content_hash, &proof, StoragePointer::Genesis, reg);
        entry.sign_proof(&id).expect("test: sign proof");
        entry
    }

    fn test_proof() -> StateProof {
        StateProof::new_for_testing()
    }

    #[tokio::test]
    async fn test_add_blocks() {
        let coord = MatrixCoordinate::new(5, 5, 5).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);
        let proof = test_proof();

        let block1 = chain
            .add_block_with_data(b"First block".to_vec(), &proof)
            .await
            .expect("test: expected success");
        assert_eq!(block1.index, 1);
        assert_eq!(chain.get_height().await, 1);

        let block2 = chain
            .add_block_with_data(b"Second block".to_vec(), &proof)
            .await
            .expect("test: expected success");
        assert_eq!(block2.index, 2);
        assert_eq!(block2.previous_hash, block1.hash);
        assert_eq!(chain.get_height().await, 2);

        assert!(chain.validate_chain().await);
    }

    #[tokio::test]
    async fn test_add_block_entries() {
        let coord = MatrixCoordinate::new(5, 5, 5).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // test_entry already binds a `new_for_testing` proof to its asset_hash;
        // use it directly so the signed-to-content binding stays intact.
        let entry = test_entry(coord);

        let block = chain
            .add_block(vec![entry])
            .await
            .expect("test: add_block");
        assert_eq!(block.index, 1);
        assert_eq!(block.entries.len(), 1);
    }

    #[tokio::test]
    async fn test_add_block_empty_entries_fails() {
        let coord = MatrixCoordinate::new(5, 5, 5).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);
        let result = chain.add_block(vec![]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_proof_rejected() {
        let coord = MatrixCoordinate::new(13, 13, 13).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        let mut bad_proof = StateProof::new_for_testing();
        bad_proof.stake_proof.stake_holder_id = String::new(); // CANONICAL: empty identity invalidates PoStake (authorization, not amount)

        let result = chain
            .add_block_with_data(b"should fail".to_vec(), &bad_proof)
            .await;
        assert!(result.is_err(), "Invalid state proof must be rejected");
        assert!(
            result
                .unwrap_err()
                .contains("State proof validation failed"),
            "Error should mention state proof"
        );
    }

    #[tokio::test]
    async fn test_register_asset_record() {
        let coord = MatrixCoordinate::new(8, 8, 8).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);
        let proof = test_proof();

        let asset = AssetRegistration::genesis(coord);
        let block = chain
            .register_asset_record(asset.clone(), &proof)
            .await
            .expect("test: registration");

        assert_eq!(block.index, 1);
        assert_eq!(block.entries.len(), 1);
        assert_eq!(block.entries[0].registration, asset);
        assert!(chain.validate_chain().await);
    }

    #[tokio::test]
    async fn test_register_multiple_asset_records() {
        let coord = MatrixCoordinate::new(9, 9, 9).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);
        let proof = test_proof();

        let assets = vec![
            AssetRegistration::genesis(coord),
            AssetRegistration::genesis(
                MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate"),
            ),
        ];
        let block = chain
            .register_asset_records(assets, &proof)
            .await
            .expect("test: batch registration");

        assert_eq!(block.index, 1);
        assert_eq!(block.entries.len(), 2);
        assert!(chain.validate_chain().await);
    }

    #[tokio::test]
    async fn test_register_empty_assets_fails() {
        let coord = MatrixCoordinate::new(10, 10, 10).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);
        let proof = test_proof();

        let result = chain.register_asset_records(vec![], &proof).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_insert_received_block() {
        let coord = MatrixCoordinate::new(11, 11, 11).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        let genesis = chain.get_head().await.expect("test: genesis");
        // H3: a received block must carry a FALCON signed_proof bound to its
        // author, so use a signed entry.
        let entry = signed_test_entry(coord);
        let block = Block::new(1, vec![entry], genesis.hash.clone());

        chain
            .insert_received_block(block.clone())
            .await
            .expect("test: insert received");

        let retrieved = chain.get_block(1).await.expect("test: get block");
        assert_eq!(retrieved, block);
    }

    #[tokio::test]
    async fn test_insert_received_block_bad_hash() {
        let coord = MatrixCoordinate::new(12, 12, 12).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        let genesis = chain.get_head().await.expect("test: genesis");
        let entry = test_entry(coord);
        let mut block = Block::new(1, vec![entry], genesis.hash.clone());
        block.hash = "tampered".to_string();

        let result = chain.insert_received_block(block).await;
        assert!(result.is_err());
    }

    /// FORGED MIRROR (b): a block whose entry proof is NOT bound to its
    /// asset_hash is rejected at block-receive (signed-to-content, P1).
    #[tokio::test]
    async fn test_insert_received_block_rejects_unbound_proof() {
        let coord = MatrixCoordinate::new(20, 20, 20).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);
        let genesis = chain.get_head().await.expect("test: genesis");

        // Build an entry whose proof is NOT bound to the asset_hash: the proof's
        // file_hash points at a DIFFERENT asset. This is the detached-proof
        // attack — a valid proof for asset A replayed against asset B.
        let reg = AssetRegistration::genesis(coord);
        let asset_b = *blake3::hash(reg.to_string().as_bytes()).as_bytes();
        let (proof_for_a, _) =
            crate::blockchain::block::bind_proof_to_asset(&[0xAAu8; 32], &StateProof::new_for_testing());
        let proof_bytes = serde_json::to_vec(&proof_for_a).unwrap_or_default();
        let proof_hash = *blake3::hash(&proof_bytes).as_bytes();
        let forged = BlockAssetEntry {
            asset_hash: asset_b,
            proof_hash,
            state_proof: proof_for_a, // file_hash == hex([0xAA;32]) != asset_b
            signed_proof: None,
            storage_pointer: StoragePointer::Genesis,
            registration: reg,
        };
        let block = Block::new(1, vec![forged], genesis.hash.clone());

        let result = chain.insert_received_block(block).await;
        assert!(result.is_err(), "unbound proof must be rejected");
        assert!(
            result.unwrap_err().contains("signed-to-content"),
            "error should cite the signed-to-content violation",
        );
    }

    /// SYNC-FETCH SYMMETRY (task #22.1): a mirror whose entry `proof_hash`
    /// does NOT equal `BLAKE3(serialize(state_proof))` is rejected at
    /// insert. Previously this check lived ONLY in the announce path; now
    /// it is centralized in `insert_received_block` so the reflector
    /// sync-fetch path (`fetch_blocks`) is held to the same standard.
    #[tokio::test]
    async fn test_insert_received_block_rejects_bad_proof_hash() {
        let coord = MatrixCoordinate::new(30, 30, 30).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);
        let genesis = chain.get_head().await.expect("test: genesis");

        // Build a properly content-bound entry, then CORRUPT its proof_hash
        // so it no longer matches BLAKE3(serialize(state_proof)).
        let mut entry = test_entry(coord);
        entry.proof_hash = [0xEE; 32]; // wrong hash
        let block = Block::new(1, vec![entry], genesis.hash.clone());

        let result = chain.insert_received_block(block).await;
        assert!(result.is_err(), "corrupted proof_hash must be rejected");
        assert!(
            result.unwrap_err().contains("proof_hash mismatch"),
            "error should cite the proof_hash integrity violation",
        );
        assert_eq!(chain.get_height().await, 0, "chain must be untouched");
    }

    /// SYNC-FETCH SYMMETRY (task #22.1): a mirror whose entry state proof
    /// FAILS `state_proof.validate()` is rejected at insert. Previously
    /// this ran only in the announce path; now both paths share it.
    #[tokio::test]
    async fn test_insert_received_block_rejects_invalid_state_proof() {
        let coord = MatrixCoordinate::new(31, 31, 31).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);
        let genesis = chain.get_head().await.expect("test: genesis");

        // Build an entry whose StateProof is invalid (zero stake fails
        // stake_proof.validate()), keeping proof_hash + content-binding
        // internally consistent so this test isolates the validate() gate.
        let reg = AssetRegistration::genesis(coord);
        let asset_hash = *blake3::hash(reg.to_string().as_bytes()).as_bytes();
        let mut bad_proof = StateProof::new_for_testing();
        bad_proof.stake_proof.stake_holder_id = String::new(); // CANONICAL: empty identity invalidates PoStake (authorization, not amount)
        let entry = BlockAssetEntry::new_bound(
            asset_hash,
            &bad_proof,
            StoragePointer::Genesis,
            reg,
        );
        // Sanity: the proof really is invalid.
        assert!(!entry.state_proof.validate(), "test setup: proof must be invalid");
        let block = Block::new(1, vec![entry], genesis.hash.clone());

        let result = chain.insert_received_block(block).await;
        assert!(result.is_err(), "invalid state proof must be rejected");
        assert!(
            result.unwrap_err().contains("state proof validation failed"),
            "error should cite state proof validation failure",
        );
        assert_eq!(chain.get_height().await, 0, "chain must be untouched");
    }

    /// FORGED MIRROR (c) part 1: a foreign block-1 (previous_hash != our
    /// genesis) is HARD REJECTED — no cross-genesis warn-insert graft (F7).
    #[tokio::test]
    async fn test_insert_received_block_rejects_foreign_block_one() {
        let coord = MatrixCoordinate::new(21, 21, 21).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // A block-1 that references a FOREIGN genesis (not ours). H3: sign the
        // entry so it passes the per-entry FALCON gate and the rejection is
        // proven to come from the LINKAGE check (predecessor mismatch), not the
        // signed_proof check.
        let entry = signed_test_entry(coord);
        let foreign_prev =
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string();
        let block = Block::new(1, vec![entry], foreign_prev);

        let result = chain.insert_received_block(block).await;
        assert!(result.is_err(), "foreign block-1 must be hard-rejected");
        assert!(
            result.unwrap_err().contains("does not match"),
            "error should cite predecessor mismatch (no graft)",
        );
        assert_eq!(chain.get_height().await, 0, "chain must be untouched");
    }

    /// FORGED MIRROR (c) part 2: a block with an unknown predecessor is
    /// BUFFERED as an orphan (not inserted); once its verified predecessor
    /// arrives, the orphan is linked. HONEST MIRROR accepted end-to-end.
    #[tokio::test]
    async fn test_insert_received_block_buffers_orphan_then_links() {
        let coord = MatrixCoordinate::new(22, 22, 22).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);
        let genesis = chain.get_head().await.expect("test: genesis");

        // Build a valid, content-bound chain: genesis -> block1 -> block2.
        let block1 = Block::new(1, vec![signed_test_entry(coord)], genesis.hash.clone());
        let block2 = Block::new(2, vec![signed_test_entry(coord)], block1.hash.clone());

        // Deliver block2 FIRST — predecessor (block1) unknown → orphan buffered.
        chain
            .insert_received_block(block2.clone())
            .await
            .expect("test: orphan buffering returns Ok");
        assert_eq!(chain.get_height().await, 0, "block2 must NOT be in the chain yet");
        assert!(chain.get_block(2).await.is_none(), "orphan not inserted");

        // Now deliver block1 — verified linkage → insert, then drain block2.
        chain
            .insert_received_block(block1.clone())
            .await
            .expect("test: honest block1 accepted");

        assert_eq!(chain.get_height().await, 2, "orphan block2 linked after block1");
        assert_eq!(
            chain.get_block(1).await.expect("test: block1"),
            block1,
        );
        assert_eq!(
            chain.get_block(2).await.expect("test: block2 linked"),
            block2,
        );
        assert!(chain.validate_chain().await, "linked chain must validate");
    }

    /// P6 (task #22.2): the orphan buffer is BOUNDED. Flooding many blocks with
    /// distinct unknown predecessors must NOT grow the buffer past MAX_ORPHANS
    /// (an authenticated peer cannot exhaust memory), and buffer-then-link must
    /// still work for a validly-linked orphan afterward.
    #[tokio::test]
    async fn test_orphan_buffer_is_bounded_under_flood() {
        use super::super::chain::MAX_ORPHANS;

        let coord = MatrixCoordinate::new(31, 31, 31).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // Flood MAX_ORPHANS + 500 blocks, each with a DISTINCT unknown
        // previous_hash (so every one is a fresh orphan key). Each block is
        // otherwise valid (content-bound, correct hash) so it passes the
        // per-entry gate and reaches the orphan-buffering branch.
        let flood = MAX_ORPHANS + 500;
        // One signed entry, reused across the flood: FALCON keygen is expensive
        // and this test exercises orphan buffering, not per-block signing. The
        // entry is identical across blocks; only previous_hash differs.
        let flood_entry = signed_test_entry(coord);
        for i in 0..flood {
            // Unique 64-hex previous_hash per block → distinct orphan keys.
            let fake_prev = format!("{i:064x}");
            let orphan =
                Block::new(9_000_000 + i as u64, vec![flood_entry.clone()], fake_prev);
            chain
                .insert_received_block(orphan)
                .await
                .expect("test: orphan buffering returns Ok");
        }

        let count = chain.orphan_count().await;
        assert!(
            count <= MAX_ORPHANS,
            "orphan buffer must stay bounded (<= {MAX_ORPHANS}), got {count}"
        );
        // Nothing was spliced into the actual chain.
        assert_eq!(chain.get_height().await, 0, "flood must not touch the chain");

        // Buffer-then-link STILL works: deliver a valid block2 (orphan), then
        // its verified predecessor block1 → block2 gets drained and linked.
        let genesis = chain.get_head().await.expect("test: genesis");
        let block1 = Block::new(1, vec![signed_test_entry(coord)], genesis.hash.clone());
        let block2 = Block::new(2, vec![signed_test_entry(coord)], block1.hash.clone());

        chain
            .insert_received_block(block2.clone())
            .await
            .expect("test: orphan buffering returns Ok");
        assert!(chain.get_block(2).await.is_none(), "block2 buffered, not inserted");

        chain
            .insert_received_block(block1.clone())
            .await
            .expect("test: honest block1 accepted");

        assert_eq!(
            chain.get_height().await,
            2,
            "buffer-then-link must still work after a flood"
        );
        assert_eq!(chain.get_block(2).await.expect("test: block2 linked"), block2);
        assert!(chain.validate_chain().await, "linked chain must validate");
    }

    /// P6 (task #22.2): TTL eviction reclaims orphans whose predecessor never
    /// arrives. We can't fast-forward wall time, so this asserts the eviction
    /// path is exercised via the capacity cap (oldest-first) — a proxy that the
    /// eviction machinery drops the oldest buffered orphan when full.
    #[tokio::test]
    async fn test_orphan_capacity_evicts_oldest() {
        use super::super::chain::MAX_ORPHANS;

        let coord = MatrixCoordinate::new(32, 32, 32).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // The very first orphan we insert (oldest) must be the first evicted
        // once the buffer overflows.
        let oldest_prev = format!("{:064x}", 0xAAAA_u64);
        let oldest = Block::new(8_000_000, vec![signed_test_entry(coord)], oldest_prev.clone());
        chain
            .insert_received_block(oldest)
            .await
            .expect("test: buffered");

        // Fill to capacity with distinct keys; the (MAX+1)th insert must evict
        // the oldest. Reuse one signed entry (FALCON keygen is expensive; this
        // test exercises capacity eviction, not per-block signing).
        let fill_entry = signed_test_entry(coord);
        for i in 0..MAX_ORPHANS {
            let fake_prev = format!("{:064x}", 0x1_0000_u64 + i as u64);
            let orphan =
                Block::new(8_100_000 + i as u64, vec![fill_entry.clone()], fake_prev);
            chain
                .insert_received_block(orphan)
                .await
                .expect("test: buffered");
        }

        assert!(
            chain.orphan_count().await <= MAX_ORPHANS,
            "buffer stays capped"
        );

        // Delivering the oldest orphan's predecessor should now find nothing to
        // drain (it was evicted), proving the oldest was the eviction victim.
        // We reconstruct a block whose hash the evicted orphan pointed to; since
        // it's gone, the chain height stays 0 for that lineage.
        assert_eq!(chain.get_height().await, 0, "no lineage grafted");
    }

    // ===================================================================
    // H3 — FALCON-in-block PoS block-accept hardening
    // ===================================================================

    /// H3 (a): a received block whose entry carries a VALID FALCON
    /// `signed_proof` bound to the claimed author is ACCEPTED.
    #[tokio::test]
    async fn test_h3_received_signed_block_accepted() {
        let coord = MatrixCoordinate::new(40, 40, 40).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);
        let genesis = chain.get_head().await.expect("test: genesis");

        let entry = signed_test_entry(coord);
        // Sanity: the envelope verifies and binds to its author.
        let pubkey = entry.verify_signed_proof().expect("test: envelope verifies");
        assert_eq!(
            blake3::hash(&pubkey).to_hex().to_string(),
            entry.state_proof.stake_proof.stake_holder_id,
            "test setup: signer must bind to claimed author",
        );

        let block = Block::new(1, vec![entry], genesis.hash.clone());
        chain
            .insert_received_block(block)
            .await
            .expect("test: signed, author-bound block must be accepted");
        assert_eq!(chain.get_height().await, 1, "block accepted onto chain");
    }

    /// H3 (b) part 1: a received block whose entry has NO `signed_proof`
    /// envelope is REJECTED by default (reject-unsigned end state).
    #[tokio::test]
    async fn test_h3_received_unsigned_block_rejected() {
        let coord = MatrixCoordinate::new(41, 41, 41).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);
        let genesis = chain.get_head().await.expect("test: genesis");

        // `test_entry` is content-bound + valid but carries NO signed_proof.
        let block = Block::new(1, vec![test_entry(coord)], genesis.hash.clone());
        let result = chain.insert_received_block(block).await;
        assert!(result.is_err(), "unsigned block must be rejected");
        assert!(
            result.unwrap_err().contains("no FALCON signed_proof"),
            "error should cite the missing signed_proof envelope",
        );
        assert_eq!(chain.get_height().await, 0, "chain untouched");
    }

    /// H3 (b) part 2: a received block whose `signed_proof` signature is
    /// INVALID (tampered) is REJECTED.
    #[tokio::test]
    async fn test_h3_received_invalid_signature_rejected() {
        let coord = MatrixCoordinate::new(42, 42, 42).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);
        let genesis = chain.get_head().await.expect("test: genesis");

        let mut entry = signed_test_entry(coord);
        // Corrupt the FALCON signature bytes — verification must now fail.
        if let Some(wire) = entry.signed_proof.as_mut() {
            for b in wire.signature.iter_mut().take(8) {
                *b ^= 0xFF;
            }
        }
        let block = Block::new(1, vec![entry], genesis.hash.clone());
        let result = chain.insert_received_block(block).await;
        assert!(result.is_err(), "invalid signature must be rejected");
        assert!(
            result.unwrap_err().contains("signed_proof invalid"),
            "error should cite the invalid signed_proof",
        );
        assert_eq!(chain.get_height().await, 0, "chain untouched");
    }

    /// H3 (b) part 3: a received block whose `signed_proof` is signed by a
    /// DIFFERENT identity than the one the proof claims as author (mis-bound)
    /// is REJECTED — a node may not sign a proof asserting someone else's
    /// stake, even with a valid signature.
    #[tokio::test]
    async fn test_h3_received_misbound_signer_rejected() {
        use hypermesh_lib::NodeSigner;

        let coord = MatrixCoordinate::new(43, 43, 43).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);
        let genesis = chain.get_head().await.expect("test: genesis");

        // Author claims identity A, but a DIFFERENT identity B signs.
        let id_a = trustchain::identity::FalconIdentity::generate();
        let id_b = trustchain::identity::FalconIdentity::generate();
        let reg = AssetRegistration::genesis(coord);
        let content_hash = *blake3::hash(reg.to_string().as_bytes()).as_bytes();
        let mut proof = StateProof::new_for_testing();
        proof.stake_proof.stake_holder_id = id_a.node_id().to_string(); // claims A
        let mut entry =
            BlockAssetEntry::new_bound(content_hash, &proof, StoragePointer::Genesis, reg);
        entry.sign_proof(&id_b).expect("test: sign with B"); // but B signs

        // The signature itself is valid (B really signed), so verify_signed_proof
        // succeeds; the accept path must reject on the author binding.
        assert!(entry.verify_signed_proof().is_ok(), "B's signature is valid");

        let block = Block::new(1, vec![entry], genesis.hash.clone());
        let result = chain.insert_received_block(block).await;
        assert!(result.is_err(), "mis-bound signer must be rejected");
        assert!(
            result.unwrap_err().contains("claimed author"),
            "error should cite the author-binding violation",
        );
        assert_eq!(chain.get_height().await, 0, "chain untouched");
    }

    /// H3: the one-release compat flag lets a legacy UNSIGNED entry through.
    /// Guarded so it does not race other tests on the shared env var — it sets
    /// and clears the flag within its own scope. (Serial via a process-wide
    /// mutex is overkill for a single flag test; we simply set/remove around
    /// the single call.)
    #[tokio::test]
    async fn test_h3_compat_flag_accepts_unsigned() {
        let coord = MatrixCoordinate::new(44, 44, 44).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);
        let genesis = chain.get_head().await.expect("test: genesis");

        std::env::set_var("HYPERMESH_ACCEPT_UNSIGNED_BLOCKS", "1");
        let block = Block::new(1, vec![test_entry(coord)], genesis.hash.clone());
        let result = chain.insert_received_block(block).await;
        std::env::remove_var("HYPERMESH_ACCEPT_UNSIGNED_BLOCKS");

        result.expect("test: compat flag must accept a legacy unsigned block");
        assert_eq!(chain.get_height().await, 1, "legacy block accepted under flag");
    }
}
