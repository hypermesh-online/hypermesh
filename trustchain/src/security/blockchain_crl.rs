// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Blockchain-Based CRL Distribution
//!
//! Stores Certificate Revocation List entries as blockchain-compatible blocks
//! so that offline devices can verify revocation status from a synced chain.
//! TrustChain does not maintain its own blockchain -- it uses BlockMatrix for
//! that. This module serializes CRL data into storable blocks and supports
//! incremental (delta) CRL updates.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::errors::Result as TrustChainResult;

/// A single revoked certificate entry stored in a CRL block.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrlBlockEntry {
    /// Serial number of the revoked certificate.
    pub serial_number: String,
    /// Reason for revocation.
    pub reason: String,
    /// When the certificate was revoked.
    pub revoked_at: SystemTime,
}

/// A CRL block suitable for storage on a blockchain.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrlBlock {
    /// Monotonically increasing block sequence number.
    pub sequence: u64,
    /// Whether this is a full CRL or an incremental (delta) update.
    pub block_type: CrlBlockType,
    /// CRL entries in this block.
    pub entries: Vec<CrlBlockEntry>,
    /// BLAKE3 hash of the previous CRL block (empty for the first block).
    pub previous_hash: [u8; 32],
    /// BLAKE3 hash of this block's content.
    pub block_hash: [u8; 32],
    /// When this block was created.
    pub created_at: SystemTime,
    /// Issuer CA identifier.
    pub issuer_ca_id: String,
}

/// Discriminates full vs incremental CRL blocks.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CrlBlockType {
    /// Complete CRL snapshot.
    Full,
    /// Incremental update since the last full CRL.
    Delta {
        /// Sequence number of the base full CRL.
        base_sequence: u64,
    },
}

/// Manages CRL blocks for blockchain-based distribution.
pub struct BlockchainCrl {
    /// Issuer CA identifier embedded in every block.
    issuer_ca_id: String,
    /// Chain of CRL blocks.
    blocks: Arc<RwLock<Vec<CrlBlock>>>,
    /// Current full CRL state (serial -> entry).
    full_state: Arc<RwLock<HashMap<String, CrlBlockEntry>>>,
    /// Sequence of the last full CRL block.
    last_full_sequence: Arc<RwLock<u64>>,
}

impl BlockchainCrl {
    /// Create a new blockchain CRL for the given issuer.
    pub fn new(issuer_ca_id: String) -> Self {
        Self {
            issuer_ca_id,
            blocks: Arc::new(RwLock::new(Vec::new())),
            full_state: Arc::new(RwLock::new(HashMap::new())),
            last_full_sequence: Arc::new(RwLock::new(0)),
        }
    }

    /// Store a full CRL snapshot as a new block.
    pub async fn store_full_crl(
        &self,
        entries: Vec<CrlBlockEntry>,
    ) -> TrustChainResult<CrlBlock> {
        let blocks = self.blocks.read().await;
        let previous_hash = blocks
            .last()
            .map(|b| b.block_hash)
            .unwrap_or([0u8; 32]);
        let sequence = blocks.len() as u64;
        drop(blocks);

        let block_hash = Self::compute_block_hash(
            sequence,
            &CrlBlockType::Full,
            &entries,
            &previous_hash,
        );

        let block = CrlBlock {
            sequence,
            block_type: CrlBlockType::Full,
            entries: entries.clone(),
            previous_hash,
            block_hash,
            created_at: SystemTime::now(),
            issuer_ca_id: self.issuer_ca_id.clone(),
        };

        // Update full state
        {
            let mut state = self.full_state.write().await;
            state.clear();
            for entry in &entries {
                state.insert(entry.serial_number.clone(), entry.clone());
            }
        }

        *self.last_full_sequence.write().await = sequence;
        self.blocks.write().await.push(block.clone());

        info!(
            "Stored full CRL block #{} with {} entries",
            sequence,
            entries.len()
        );
        Ok(block)
    }

    /// Store an incremental (delta) CRL update.
    ///
    /// The delta contains only newly revoked certificates since the last full CRL.
    pub async fn store_delta_crl(
        &self,
        new_entries: Vec<CrlBlockEntry>,
    ) -> TrustChainResult<CrlBlock> {
        let base_sequence = *self.last_full_sequence.read().await;

        let blocks = self.blocks.read().await;
        let previous_hash = blocks
            .last()
            .map(|b| b.block_hash)
            .unwrap_or([0u8; 32]);
        let sequence = blocks.len() as u64;
        drop(blocks);

        let block_type = CrlBlockType::Delta { base_sequence };
        let block_hash = Self::compute_block_hash(
            sequence,
            &block_type,
            &new_entries,
            &previous_hash,
        );

        let block = CrlBlock {
            sequence,
            block_type,
            entries: new_entries.clone(),
            previous_hash,
            block_hash,
            created_at: SystemTime::now(),
            issuer_ca_id: self.issuer_ca_id.clone(),
        };

        // Merge delta into full state
        {
            let mut state = self.full_state.write().await;
            for entry in &new_entries {
                state.insert(entry.serial_number.clone(), entry.clone());
            }
        }

        self.blocks.write().await.push(block.clone());

        info!(
            "Stored delta CRL block #{} with {} new entries (base: #{})",
            sequence,
            new_entries.len(),
            base_sequence
        );
        Ok(block)
    }

    /// Get the complete CRL state by replaying all blocks.
    pub async fn get_full_state(&self) -> Vec<CrlBlockEntry> {
        self.full_state
            .read()
            .await
            .values()
            .cloned()
            .collect()
    }

    /// Check if a certificate serial is revoked in the current CRL state.
    pub async fn is_revoked(&self, serial_number: &str) -> bool {
        self.full_state
            .read()
            .await
            .contains_key(serial_number)
    }

    /// Get a specific CRL block by sequence number.
    pub async fn get_block(&self, sequence: u64) -> Option<CrlBlock> {
        let blocks = self.blocks.read().await;
        blocks.get(sequence as usize).cloned()
    }

    /// Get the total number of CRL blocks.
    pub async fn block_count(&self) -> u64 {
        self.blocks.read().await.len() as u64
    }

    /// Verify the block chain integrity (each block's previous_hash links correctly).
    pub async fn verify_chain(&self) -> TrustChainResult<bool> {
        let blocks = self.blocks.read().await;
        for (i, block) in blocks.iter().enumerate() {
            if i == 0 {
                // First block should reference zero hash
                if block.previous_hash != [0u8; 32] {
                    return Ok(false);
                }
            } else {
                let expected = blocks[i - 1].block_hash;
                if block.previous_hash != expected {
                    debug!(
                        "Chain break at block #{}: expected {:?}, got {:?}",
                        i,
                        &expected[..4],
                        &block.previous_hash[..4]
                    );
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    fn compute_block_hash(
        sequence: u64,
        block_type: &CrlBlockType,
        entries: &[CrlBlockEntry],
        previous_hash: &[u8; 32],
    ) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&sequence.to_le_bytes());
        hasher.update(previous_hash);

        // Include block type discriminant
        match block_type {
            CrlBlockType::Full => {
                hasher.update(b"full");
            }
            CrlBlockType::Delta { base_sequence } => {
                hasher.update(b"delta");
                hasher.update(&base_sequence.to_le_bytes());
            }
        }

        for entry in entries {
            hasher.update(entry.serial_number.as_bytes());
            hasher.update(entry.reason.as_bytes());
        }

        *hasher.finalize().as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(serial: &str) -> CrlBlockEntry {
        CrlBlockEntry {
            serial_number: serial.to_string(),
            reason: "test revocation".to_string(),
            revoked_at: SystemTime::now(),
        }
    }

    #[tokio::test]
    async fn test_full_crl_store_and_retrieve() {
        let bcrl = BlockchainCrl::new("test-ca".to_string());

        let entries = vec![make_entry("cert-1"), make_entry("cert-2")];
        let block = bcrl
            .store_full_crl(entries)
            .await
            .expect("test: store full CRL");

        assert_eq!(block.sequence, 0);
        assert_eq!(block.block_type, CrlBlockType::Full);
        assert_eq!(block.entries.len(), 2);
        assert_eq!(block.previous_hash, [0u8; 32]);
        assert_ne!(block.block_hash, [0u8; 32]);

        // Verify state
        assert!(bcrl.is_revoked("cert-1").await);
        assert!(bcrl.is_revoked("cert-2").await);
        assert!(!bcrl.is_revoked("cert-3").await);

        assert_eq!(bcrl.block_count().await, 1);
    }

    #[tokio::test]
    async fn test_delta_crl_incremental_update() {
        let bcrl = BlockchainCrl::new("test-ca".to_string());

        // Store initial full CRL
        let full_entries = vec![make_entry("cert-a")];
        bcrl.store_full_crl(full_entries)
            .await
            .expect("test: store full");

        // Store delta with new entry
        let delta_entries = vec![make_entry("cert-b")];
        let delta_block = bcrl
            .store_delta_crl(delta_entries)
            .await
            .expect("test: store delta");

        assert_eq!(delta_block.sequence, 1);
        assert_eq!(
            delta_block.block_type,
            CrlBlockType::Delta { base_sequence: 0 }
        );

        // Both entries should be in the full state
        assert!(bcrl.is_revoked("cert-a").await);
        assert!(bcrl.is_revoked("cert-b").await);

        let full_state = bcrl.get_full_state().await;
        assert_eq!(full_state.len(), 2);
    }

    #[tokio::test]
    async fn test_chain_integrity_verification() {
        let bcrl = BlockchainCrl::new("test-ca".to_string());

        bcrl.store_full_crl(vec![make_entry("c1")])
            .await
            .expect("test: block 0");
        bcrl.store_delta_crl(vec![make_entry("c2")])
            .await
            .expect("test: block 1");
        bcrl.store_delta_crl(vec![make_entry("c3")])
            .await
            .expect("test: block 2");

        assert_eq!(bcrl.block_count().await, 3);

        let valid = bcrl.verify_chain().await.expect("test: verify chain");
        assert!(valid, "Chain should be valid");

        // Verify hash linking
        let b0 = bcrl.get_block(0).await.expect("test: block 0");
        let b1 = bcrl.get_block(1).await.expect("test: block 1");
        assert_eq!(b1.previous_hash, b0.block_hash);
    }

    #[tokio::test]
    async fn test_full_crl_replaces_state() {
        let bcrl = BlockchainCrl::new("test-ca".to_string());

        // First full CRL
        bcrl.store_full_crl(vec![make_entry("old-cert")])
            .await
            .expect("test: first full");
        assert!(bcrl.is_revoked("old-cert").await);

        // Second full CRL replaces state
        bcrl.store_full_crl(vec![make_entry("new-cert")])
            .await
            .expect("test: second full");
        assert!(!bcrl.is_revoked("old-cert").await);
        assert!(bcrl.is_revoked("new-cert").await);
    }
}
