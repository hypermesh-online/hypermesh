// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase I.1 — cross-chain receipt validation.
//!
//! [`CrossChainReceiptValidator`] indexes [`TransferReceipt`] entries
//! found on the local chain and answers the audit-time question:
//! *"For source chain `A` block `Hₐ`, does the linked target chain `B`
//! block `Hᵦ` match what the receipt actually claims?"*
//!
//! The validator is the missing piece called out in
//! `papers/HYPERMESH.md` §G/I — receipts written by
//! [`crate::gateway::transfer_coordinator::TransferCoordinator`] on
//! both source and target chains link via a shared `transfer_id`,
//! `source_block_hash`, and `target_block_hash`. With the receipt
//! present on either chain, an auditor can prove transfer atomicity
//! without consulting the other chain's full history.
//!
//! # Index population
//!
//! Receipts are populated into the index whenever a chain block
//! containing a `TransferReceipt` payload is added (or, on a fresh
//! restart, by walking the chain once via [`Self::rebuild_from_chain`]).
//! The index is a pure read structure — always-on, no security risk.
//!
//! # Wire-format
//!
//! Receipts are stored on chain in `BlockAssetEntry.storage_pointer`
//! as `StoragePointer::Local { path }` where `path` is the JSON
//! payload of [`TransferReceipt`] (see `transfer_coordinator::write_receipt_entry`).

use std::collections::HashMap;
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::RwLock;

use crate::blockchain::block::{Block, StoragePointer};
use crate::blockchain::NodeBlockchain;
use crate::gateway::asset_transfer::TransferReceipt;

/// Errors returned from cross-chain receipt validation.
#[derive(Debug, Error)]
pub enum CrossChainError {
    /// No receipt found for the given (source_chain_id, source_block_hash).
    #[error(
        "no receipt for source chain {source_chain_id} block {source_block_hash}"
    )]
    ReceiptNotFound {
        /// Source chain identifier.
        source_chain_id: String,
        /// Block hash on the source chain.
        source_block_hash: String,
    },

    /// Receipt found but its target_chain_id mismatches the requested target.
    #[error(
        "receipt target chain mismatch: expected {expected}, found {actual}"
    )]
    TargetChainMismatch {
        /// Requested target chain.
        expected: String,
        /// Receipt's actual target chain.
        actual: String,
    },

    /// Receipt found but its target_block_hash mismatches the requested target.
    #[error(
        "receipt target block mismatch: expected {expected}, found {actual}"
    )]
    TargetBlockMismatch {
        /// Requested target block hash.
        expected: String,
        /// Receipt's actual target block hash.
        actual: String,
    },
}

/// Index key for [`CrossChainReceiptValidator::receipts`].
type ReceiptKey = (String, String);

/// Cross-chain receipt validator.
///
/// Owns the `cross_chain_receipts` index keyed by
/// `(source_chain_id, source_block_hash)` and exposes
/// [`validate_cross_chain`](Self::validate_cross_chain) for proving
/// linkage between source and target chain blocks.
///
/// Cheap to clone; internal state is `Arc<RwLock<...>>`.
#[derive(Clone, Debug, Default)]
pub struct CrossChainReceiptValidator {
    /// Indexed by `(source_chain_id, source_block_hash)` so that a
    /// caller holding only the source-side anchor can locate the
    /// linked target-side anchor.
    receipts: Arc<RwLock<HashMap<ReceiptKey, TransferReceipt>>>,
    /// Secondary index by `transfer_id` for IPC `chain.lookup_cross_receipt`.
    by_transfer_id: Arc<RwLock<HashMap<String, TransferReceipt>>>,
}

impl CrossChainReceiptValidator {
    /// Construct an empty validator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a receipt into the index.
    ///
    /// Both directions of the cross-chain link are recorded. If a
    /// receipt is observed on the *source* chain its
    /// `(source_chain_id, source_block_hash)` is the lookup key. If a
    /// receipt is observed on the *target* chain its
    /// `(target_chain_id, target_block_hash)` is also recorded so a
    /// node holding only the target-side anchor can still find the
    /// link. Both keys point to the same receipt struct.
    pub async fn insert(&self, receipt: TransferReceipt) {
        let mut by_key = self.receipts.write().await;
        let mut by_id = self.by_transfer_id.write().await;
        by_key.insert(
            (
                receipt.source_chain_id.clone(),
                receipt.source_block_hash.clone(),
            ),
            receipt.clone(),
        );
        // Also index by the target-side anchor so an auditor holding
        // only the target chain can still resolve the link.
        by_key.insert(
            (
                receipt.target_chain_id.clone(),
                receipt.target_block_hash.clone(),
            ),
            receipt.clone(),
        );
        by_id.insert(receipt.transfer_id.clone(), receipt);
    }

    /// Walk the given local chain, parse every `TransferReceipt` block
    /// entry, and populate the index.
    ///
    /// This is the canonical population path used by:
    ///   - `NodeBlockchain` after it appends a receipt block
    ///     (`add_block` callback / explicit call).
    ///   - Daemon startup, when reconstructing in-memory state from
    ///     persisted blocks.
    ///
    /// Returns the number of receipts indexed.
    pub async fn rebuild_from_chain(&self, chain: &Arc<NodeBlockchain>) -> usize {
        let blocks = chain.get_chain().await;
        self.rebuild_from_blocks(&blocks).await
    }

    /// Variant of [`rebuild_from_chain`] that takes pre-fetched blocks.
    pub async fn rebuild_from_blocks(&self, blocks: &[Block]) -> usize {
        let mut count = 0usize;
        for block in blocks {
            for entry in &block.entries {
                let payload = match &entry.storage_pointer {
                    StoragePointer::Local { path } => path.as_str(),
                    _ => continue,
                };
                // Defensive parse — the chain holds many entry shapes;
                // a `serde_json` failure is normal for non-receipt entries.
                let receipt: TransferReceipt = match serde_json::from_str(payload) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                // Disambiguate from look-alike entries (Lock/Register/Release).
                // TransferReceipt requires non-empty source/target hashes
                // and a non-zero `completed_at` — unique to the receipt shape.
                if receipt.transfer_id.is_empty()
                    || receipt.source_block_hash.is_empty()
                    || receipt.target_block_hash.is_empty()
                    || receipt.completed_at == 0
                    || !payload.contains("\"completed_at\"")
                    || !payload.contains("\"target_block_hash\"")
                {
                    continue;
                }
                self.insert(receipt).await;
                count += 1;
            }
        }
        count
    }

    /// Look up a receipt by source-chain anchor.
    pub async fn get_by_source(
        &self,
        source_chain_id: &str,
        source_block_hash: &str,
    ) -> Option<TransferReceipt> {
        self.receipts
            .read()
            .await
            .get(&(source_chain_id.to_string(), source_block_hash.to_string()))
            .cloned()
    }

    /// Look up a receipt by `transfer_id`.
    pub async fn get_by_transfer_id(
        &self,
        transfer_id: &str,
    ) -> Option<TransferReceipt> {
        self.by_transfer_id
            .read()
            .await
            .get(transfer_id)
            .cloned()
    }

    /// Validate that a claimed cross-chain link matches a recorded receipt.
    ///
    /// Steps:
    ///   1. Look up the receipt by `(source_chain_id, source_block_hash)`.
    ///   2. Verify the receipt's `target_chain_id` matches the request.
    ///   3. Verify the receipt's `target_block_hash` matches the request.
    ///
    /// Returns `Ok(())` on full match. Specific error variants on
    /// missing receipt, wrong target chain, or wrong target hash.
    ///
    /// Note: federation-peer signing checks (the optional step 4 in
    /// the design doc) are not enforced here — receipts are validated
    /// by being on chain (every block carries a state proof) and by
    /// the originating coordinator's own bilateral verification at
    /// write time. A separate federation-gating layer can wrap this
    /// validator if a deployment requires stricter trust.
    pub async fn validate_cross_chain(
        &self,
        source_chain_id: &str,
        source_block_hash: &str,
        target_chain_id: &str,
        target_block_hash: &str,
    ) -> Result<(), CrossChainError> {
        let receipt = self
            .get_by_source(source_chain_id, source_block_hash)
            .await
            .ok_or_else(|| CrossChainError::ReceiptNotFound {
                source_chain_id: source_chain_id.to_string(),
                source_block_hash: source_block_hash.to_string(),
            })?;

        if receipt.target_chain_id != target_chain_id {
            return Err(CrossChainError::TargetChainMismatch {
                expected: target_chain_id.to_string(),
                actual: receipt.target_chain_id,
            });
        }
        if receipt.target_block_hash != target_block_hash {
            return Err(CrossChainError::TargetBlockMismatch {
                expected: target_block_hash.to_string(),
                actual: receipt.target_block_hash,
            });
        }
        Ok(())
    }

    /// Snapshot of currently-indexed receipts (test/debug helper).
    pub async fn len(&self) -> usize {
        self.receipts.read().await.len()
    }

    /// Whether the index has zero entries.
    pub async fn is_empty(&self) -> bool {
        self.receipts.read().await.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::BlockchainScope;

    fn sample_receipt(
        transfer_id: &str,
        src_chain: &str,
        src_hash: &str,
        tgt_chain: &str,
        tgt_hash: &str,
    ) -> TransferReceipt {
        TransferReceipt {
            transfer_id: transfer_id.to_string(),
            source_chain_id: src_chain.to_string(),
            target_chain_id: tgt_chain.to_string(),
            source_block_hash: src_hash.to_string(),
            target_block_hash: tgt_hash.to_string(),
            completed_at: 1_700_000_000,
            asset_id: "asset-xyz".to_string(),
            source_scope: BlockchainScope::Device,
            target_scope: BlockchainScope::Network,
        }
    }

    #[tokio::test]
    async fn validates_correct_link() {
        let v = CrossChainReceiptValidator::new();
        v.insert(sample_receipt("t-1", "chain-A", "hashA", "chain-B", "hashB"))
            .await;
        v.validate_cross_chain("chain-A", "hashA", "chain-B", "hashB")
            .await
            .expect("test: link should validate");
    }

    #[tokio::test]
    async fn missing_receipt_errors() {
        let v = CrossChainReceiptValidator::new();
        let err = v
            .validate_cross_chain("chain-A", "hashA", "chain-B", "hashB")
            .await
            .expect_err("test: should fail");
        assert!(matches!(err, CrossChainError::ReceiptNotFound { .. }));
    }

    #[tokio::test]
    async fn target_chain_mismatch_errors() {
        let v = CrossChainReceiptValidator::new();
        v.insert(sample_receipt("t-1", "chain-A", "hashA", "chain-B", "hashB"))
            .await;
        let err = v
            .validate_cross_chain("chain-A", "hashA", "chain-X", "hashB")
            .await
            .expect_err("test: should fail");
        assert!(matches!(err, CrossChainError::TargetChainMismatch { .. }));
    }

    #[tokio::test]
    async fn target_hash_mismatch_errors() {
        let v = CrossChainReceiptValidator::new();
        v.insert(sample_receipt("t-1", "chain-A", "hashA", "chain-B", "hashB"))
            .await;
        let err = v
            .validate_cross_chain("chain-A", "hashA", "chain-B", "hashWRONG")
            .await
            .expect_err("test: should fail");
        assert!(matches!(err, CrossChainError::TargetBlockMismatch { .. }));
    }

    #[tokio::test]
    async fn lookup_by_target_anchor_works() {
        // Auditor holding only the target side can still find the link.
        let v = CrossChainReceiptValidator::new();
        v.insert(sample_receipt("t-1", "chain-A", "hashA", "chain-B", "hashB"))
            .await;
        let got = v
            .get_by_source("chain-B", "hashB")
            .await
            .expect("test: should find via target anchor");
        assert_eq!(got.transfer_id, "t-1");
    }

    #[tokio::test]
    async fn lookup_by_transfer_id_works() {
        let v = CrossChainReceiptValidator::new();
        v.insert(sample_receipt("t-42", "chain-A", "hA", "chain-B", "hB"))
            .await;
        let got = v
            .get_by_transfer_id("t-42")
            .await
            .expect("test: lookup by id");
        assert_eq!(got.source_chain_id, "chain-A");
        assert_eq!(got.target_chain_id, "chain-B");
    }
}
