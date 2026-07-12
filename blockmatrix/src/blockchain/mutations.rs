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

impl NodeBlockchain {
    /// Add a new block containing the given entries.
    ///
    /// Each `BlockAssetEntry` carries its own `StateProof` which is
    /// validated independently.  The block is built, structurally
    /// validated, and inserted.
    pub async fn add_block(
        &self,
        entries: Vec<BlockAssetEntry>,
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
    /// Mirror invariant (P1) + accept-a-mirror refactor (F7): a received block
    /// enters this chain ONLY with authenticated, verified linkage. Nothing is
    /// ever spliced in on a non-matching or missing predecessor:
    ///
    /// 1. Block hash must recompute (`verify_hash`).
    /// 2. Every entry's proof must be bound to its `asset_hash`
    ///    (signed-to-content: `content_binding_ok`).
    /// 3. Linkage (for non-genesis):
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

        // Signed-to-content: reject any mirror whose proof is not bound to the
        // content it claims. A valid proof for asset A must not be replayable
        // inside an entry claiming asset B.
        for (i, entry) in block.entries.iter().enumerate() {
            if !entry.content_binding_ok() {
                return Err(format!(
                    "Block {} entry {i} proof not bound to its asset_hash \
                     (signed-to-content violation) — mirror rejected",
                    block.index,
                ));
            }
        }

        // Genesis has no predecessor to verify — insert directly.
        if block.index == 0 {
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
            let mut orphans = self.orphans.write().await;
            warn!(
                "Block {} predecessor unknown — buffering as orphan (prev={})",
                block.index,
                &block.previous_hash[..16.min(block.previous_hash.len())],
            );
            orphans.insert(block.previous_hash.clone(), block);
            return Ok(());
        }

        // Linkage verified — insert, then attempt to drain any orphan chain
        // that was waiting on this newly-inserted block.
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
                orphans.remove(&parent_hash)
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
        bad_proof.stake_proof.stake_amount = 0;

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
        let entry = test_entry(coord);
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

    /// FORGED MIRROR (c) part 1: a foreign block-1 (previous_hash != our
    /// genesis) is HARD REJECTED — no cross-genesis warn-insert graft (F7).
    #[tokio::test]
    async fn test_insert_received_block_rejects_foreign_block_one() {
        let coord = MatrixCoordinate::new(21, 21, 21).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // A block-1 that references a FOREIGN genesis (not ours).
        let entry = test_entry(coord);
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
        let block1 = Block::new(1, vec![test_entry(coord)], genesis.hash.clone());
        let block2 = Block::new(2, vec![test_entry(coord)], block1.hash.clone());

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
}
