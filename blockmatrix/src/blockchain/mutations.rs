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

        // 1. Validate every entry's state proof (binary: pass or fail)
        for (i, entry) in entries.iter().enumerate() {
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
        let proof_bytes = serde_json::to_vec(state_proof).unwrap_or_default();
        let proof_hash = *blake3::hash(&proof_bytes).as_bytes();

        let entry = BlockAssetEntry {
            asset_hash,
            proof_hash,
            state_proof: state_proof.clone(),
            storage_pointer: StoragePointer::Local {
                path: String::new(),
            },
            registration,
        };

        self.add_block(vec![entry]).await
    }

    /// Insert a pre-built block received from a peer.
    ///
    /// Unlike [`add_block`] which creates a new block, this inserts an
    /// existing block that has already been validated by the sender.
    /// The caller MUST verify the block hash before calling.
    pub async fn insert_received_block(&self, block: Block) -> Result<(), String> {
        if !block.verify_hash() {
            return Err(format!(
                "Block {} hash mismatch: expected {}, got {}",
                block.index,
                block.calculate_hash(),
                block.hash,
            ));
        }

        // Check previous_hash linkage (skip for genesis)
        if block.index > 0 {
            let blocks = self.blocks.read().await;
            if let Some(prev) = blocks.get(&(block.index - 1)) {
                if block.previous_hash != prev.hash {
                    // For cross-genesis sync: when a peer's block at index 1
                    // references its own genesis (not ours), allow insertion
                    // with a warning rather than rejecting outright.
                    if block.index == 1 {
                        warn!(
                            "Block 1 previous_hash differs from our genesis \
                             (cross-genesis sync): peer={}, ours={}",
                            &block.previous_hash[..16.min(block.previous_hash.len())],
                            &prev.hash[..16.min(prev.hash.len())],
                        );
                    } else {
                        return Err(format!(
                            "Block {} previous_hash {} does not match block {}'s hash {}",
                            block.index,
                            block.previous_hash,
                            block.index - 1,
                            prev.hash,
                        ));
                    }
                }
            }
            // If we don't have the predecessor, we still insert (gap-fill later)
        }

        self.insert_block(block).await
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
        let proof_bytes = serde_json::to_vec(state_proof).unwrap_or_default();
        let proof_hash = *blake3::hash(&proof_bytes).as_bytes();

        let entry = BlockAssetEntry {
            asset_hash,
            proof_hash,
            state_proof: state_proof.clone(),
            storage_pointer: StoragePointer::Genesis,
            registration,
        };

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
        let proof_bytes = serde_json::to_vec(state_proof).unwrap_or_default();
        let proof_hash = *blake3::hash(&proof_bytes).as_bytes();

        // Store serialized DnsBlockEntry in the path field so receivers
        // can deserialize it without reversing the content hash.
        let dns_payload = String::from_utf8(dns_json).unwrap_or_default();

        let entry = BlockAssetEntry {
            asset_hash,
            proof_hash,
            state_proof: state_proof.clone(),
            storage_pointer: StoragePointer::Local { path: dns_payload },
            registration,
        };

        self.add_block(vec![entry]).await
    }

    /// Write a key rotation entry to the blockchain.
    ///
    /// Records old->new key transition with FALCON-signed proof (§6.2.2).
    /// The rotation entry is stored as a `StoragePointer::Local` payload
    /// so peers receiving the block can extract and verify the chain.
    pub async fn add_key_rotation_block(
        &self,
        entry: &trustchain::identity::KeyRotationEntry,
    ) -> Result<Block, String> {
        let entry_bytes = serde_json::to_vec(entry).map_err(|e| {
            format!("Failed to serialize key rotation entry: {e}")
        })?;
        let asset_hash = *blake3::hash(&entry_bytes).as_bytes();
        let proof_hash = *blake3::hash(&entry.rotation_signature).as_bytes();

        let block_entry = BlockAssetEntry {
            asset_hash,
            proof_hash,
            state_proof: StateProof::new_for_testing(), // alpha: bootstrap phase
            storage_pointer: StoragePointer::Local {
                path: String::from_utf8_lossy(&entry_bytes).to_string(),
            },
            registration: AssetRegistration::genesis(self.node_coordinate),
        };

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

        let proof_bytes = serde_json::to_vec(state_proof).unwrap_or_default();
        let proof_hash = *blake3::hash(&proof_bytes).as_bytes();

        let entries: Vec<BlockAssetEntry> = registrations
            .into_iter()
            .map(|reg| {
                let asset_hash = reg.content_hash;
                BlockAssetEntry {
                    asset_hash,
                    proof_hash,
                    state_proof: state_proof.clone(),
                    storage_pointer: StoragePointer::Genesis,
                    registration: reg,
                }
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
        let state_proof = StateProof::default();
        let proof_bytes = serde_json::to_vec(&state_proof).unwrap_or_default();
        let proof_hash = *blake3::hash(&proof_bytes).as_bytes();
        BlockAssetEntry {
            asset_hash: content_hash,
            proof_hash,
            state_proof,
            storage_pointer: StoragePointer::Genesis,
            registration: reg,
        }
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

        let mut entry = test_entry(coord);
        entry.state_proof = StateProof::new_for_testing();

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
}
