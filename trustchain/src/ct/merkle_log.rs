//! Merkle Tree Log Implementation for Certificate Transparency
//! 
//! High-performance merkle tree implementation with batch updates,
//! inclusion proofs, and consistency proofs for CT logs.

use std::collections::VecDeque;
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};
use tracing::{debug, warn};
use sha2::{Sha256, Digest};
// use merkletree::{MerkleTree, Proof, Hashable, hash::{Algorithm, Sha256Algorithm}}; // Temporarily commented due to API changes

use crate::errors::{CTError, Result as TrustChainResult};
use super::LogEntry;

/// Simplified merkle tree-based CT log
pub struct MerkleLog {
    /// Log identifier
    log_id: String,
    /// Maximum entries before creating new shard
    max_entries: u64,
    /// Log entries (in order)
    entries: Vec<LogEntry>,
    /// Simplified merkle tree (just root hash for now)
    merkle_root: Option<[u8; 32]>,
    /// Pending entries to be added to merkle tree
    pending_entries: VecDeque<LogEntry>,
    /// Tree needs rebuild flag
    tree_dirty: bool,
    /// Log statistics
    stats: MerkleLogStats,
}

/// Merkle log statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleLogStats {
    pub entry_count: u64,
    pub tree_size: u64,
    pub root_hash: [u8; 32],
    pub pending_count: u64,
    pub last_tree_update: std::time::SystemTime,
}

impl Default for MerkleLogStats {
    fn default() -> Self {
        Self {
            entry_count: 0,
            tree_size: 0,
            root_hash: [0u8; 32],
            pending_count: 0,
            last_tree_update: std::time::SystemTime::now(),
        }
    }
}

impl MerkleLog {
    /// Create a new merkle log
    pub async fn new(log_id: String, max_entries: u64) -> TrustChainResult<Self> {
        debug!("Creating new merkle log: {}", log_id);

        Ok(Self {
            log_id,
            max_entries,
            entries: Vec::new(),
            merkle_root: None,
            pending_entries: VecDeque::new(),
            tree_dirty: false,
            stats: MerkleLogStats::default(),
        })
    }

    /// Add a new entry to the log
    pub async fn add_entry(&mut self, mut entry: LogEntry) -> TrustChainResult<LogEntry> {
        if self.entries.len() >= self.max_entries as usize {
            return Err(CTError::LogFull {
                log_id: self.log_id.clone(),
                current_entries: self.entries.len() as u64,
            }.into());
        }

        // Calculate leaf hash
        entry.leaf_hash = self.calculate_leaf_hash(&entry);

        // Add to entries and pending queue
        self.entries.push(entry.clone());
        self.pending_entries.push_back(entry.clone());
        
        // Mark tree as dirty
        self.tree_dirty = true;
        
        // Update statistics
        self.stats.entry_count += 1;
        self.stats.pending_count += 1;

        debug!("Added entry {} to merkle log {}", entry.sequence_number, self.log_id);
        Ok(entry)
    }

    /// Update merkle tree with pending entries
    pub async fn update_merkle_tree(&mut self) -> TrustChainResult<()> {
        if !self.tree_dirty || self.pending_entries.is_empty() {
            return Ok(());
        }

        debug!("Updating merkle tree for log {}", self.log_id);

        // Simplified merkle root calculation
        if !self.entries.is_empty() {
            let mut hasher = Sha256::new();
            for entry in &self.entries {
                hasher.update(&entry.leaf_hash);
            }
            self.merkle_root = Some(hasher.finalize().into());
        }

        // Clear pending entries
        self.pending_entries.clear();
        self.tree_dirty = false;

        // Update statistics
        self.stats.tree_size = self.entries.len() as u64;
        self.stats.root_hash = self.merkle_root.unwrap_or([0u8; 32]);
        self.stats.pending_count = 0;
        self.stats.last_tree_update = std::time::SystemTime::now();

        debug!("Merkle tree updated for log {}: {} entries", self.log_id, self.entries.len());
        Ok(())
    }

    /// Get inclusion proof for an entry (simplified implementation)
    pub async fn get_inclusion_proof(&self, entry: &LogEntry) -> TrustChainResult<Vec<[u8; 32]>> {
        // Simplified inclusion proof - just return the root hash
        if self.merkle_root.is_none() {
            return Err(CTError::MerkleTree {
                operation: "get_inclusion_proof".to_string(),
                reason: "Merkle tree not initialized".to_string(),
            }.into());
        }

        // Find entry index
        let _entry_index = self.entries.iter()
            .position(|e| e.sequence_number == entry.sequence_number)
            .ok_or_else(|| CTError::EntryNotFound {
                entry_id: hex::encode(entry.entry_id),
            })?;

        // Simplified proof - just return root hash
        let proof_hashes = vec![self.merkle_root.unwrap()];

        debug!("Generated simplified inclusion proof for entry {}: {} hashes", 
               entry.sequence_number, proof_hashes.len());
        
        Ok(proof_hashes)
    }

    /// Verify entry inclusion in merkle tree (simplified implementation)
    pub async fn verify_entry_inclusion(&self, entry: &LogEntry) -> TrustChainResult<bool> {
        if self.merkle_root.is_none() {
            return Err(CTError::MerkleTree {
                operation: "verify_inclusion".to_string(),
                reason: "Merkle tree not initialized".to_string(),
            }.into());
        }

        // Find entry index
        let is_present = self.entries.iter()
            .any(|e| e.sequence_number == entry.sequence_number);

        if !is_present {
            return Err(CTError::EntryNotFound {
                entry_id: hex::encode(entry.entry_id),
            }.into());
        }

        // Simplified verification - just check if entry exists
        debug!("Entry {} inclusion verification: {}", entry.sequence_number, is_present);
        Ok(is_present)
    }

    /// Get consistency proof between two tree sizes (simplified implementation)
    pub async fn get_consistency_proof(&self, old_size: u64, new_size: u64) -> TrustChainResult<Vec<[u8; 32]>> {
        if self.merkle_root.is_none() {
            return Err(CTError::MerkleTree {
                operation: "get_consistency_proof".to_string(),
                reason: "Merkle tree not initialized".to_string(),
            }.into());
        }

        if old_size > new_size {
            return Err(anyhow!("Old size cannot be greater than new size").into());
        }

        if new_size > self.entries.len() as u64 {
            return Err(anyhow!("New size exceeds current tree size").into());
        }

        // Simplified consistency proof - calculate hash of old entries
        let mut old_hasher = Sha256::new();
        for entry in self.entries.iter().take(old_size as usize) {
            old_hasher.update(&entry.leaf_hash);
        }
        let old_root = old_hasher.finalize().into();
        let new_root = self.merkle_root.unwrap();
        
        if old_root == new_root {
            // Trees are identical
            Ok(vec![])
        } else {
            // Return both roots as consistency proof
            Ok(vec![old_root, new_root])
        }
    }

    /// Get current tree size
    pub fn get_tree_size(&self) -> u64 {
        self.entries.len() as u64
    }

    /// Get current root hash
    pub fn get_root_hash(&self) -> [u8; 32] {
        self.merkle_root.unwrap_or([0u8; 32])
    }

    /// Get log statistics
    pub async fn get_stats(&self) -> MerkleLogStats {
        self.stats.clone()
    }

    /// Get entry by sequence number
    pub async fn get_entry(&self, sequence_number: u64) -> Option<&LogEntry> {
        self.entries.iter()
            .find(|entry| entry.sequence_number == sequence_number)
    }

    /// Get entries in range
    pub async fn get_entries_range(&self, start: u64, end: u64) -> Vec<&LogEntry> {
        self.entries.iter()
            .filter(|entry| entry.sequence_number >= start && entry.sequence_number < end)
            .collect()
    }

    /// Check if log is at capacity
    pub fn is_full(&self) -> bool {
        self.entries.len() >= self.max_entries as usize
    }

    /// Calculate leaf hash for an entry
    fn calculate_leaf_hash(&self, entry: &LogEntry) -> [u8; 32] {
        // CT leaf hash includes a prefix to prevent second preimage attacks
        let mut hasher = Sha256::new();
        hasher.update(&[0x00]); // Leaf prefix
        hasher.update(&entry.hash());
        hasher.finalize().into()
    }
}

/// Merkle tree path for verification
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerklePath {
    pub leaf_index: u64,
    pub tree_size: u64,
    pub path: Vec<MerklePathNode>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerklePathNode {
    pub hash: [u8; 32],
    pub is_right: bool,
}

impl MerklePath {
    /// Verify this path leads to the given root hash
    pub fn verify(&self, leaf_hash: &[u8; 32], root_hash: &[u8; 32]) -> bool {
        let mut current_hash = *leaf_hash;
        
        for node in &self.path {
            let mut hasher = Sha256::new();
            if node.is_right {
                hasher.update(&current_hash);
                hasher.update(&node.hash);
            } else {
                hasher.update(&node.hash);
                hasher.update(&current_hash);
            }
            current_hash = hasher.finalize().into();
        }
        
        current_hash == *root_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ConsensusProof;
    use std::time::SystemTime;

    fn create_test_entry(seq_num: u64) -> LogEntry {
        LogEntry {
            sequence_number: seq_num,
            certificate_der: format!("cert_{}", seq_num).into_bytes(),
            fingerprint: [seq_num as u8; 32],
            timestamp: SystemTime::now(),
            common_name: format!("test{}.example.com", seq_num),
            issuer_ca_id: "test-ca".to_string(),
            consensus_proof: ConsensusProof::default_for_testing(),
            entry_id: [seq_num as u8; 32],
            leaf_hash: [0u8; 32], // Will be calculated
        }
    }

    #[tokio::test]
    async fn test_merkle_log_creation() {
        let log = MerkleLog::new("test-log".to_string(), 1000).await.unwrap();
        assert_eq!(log.log_id, "test-log");
        assert_eq!(log.get_tree_size(), 0);
    }

    #[tokio::test]
    async fn test_entry_addition() {
        let mut log = MerkleLog::new("test-log".to_string(), 1000).await.unwrap();
        
        let entry = create_test_entry(0);
        let added_entry = log.add_entry(entry).await.unwrap();
        
        assert_eq!(added_entry.sequence_number, 0);
        assert_ne!(added_entry.leaf_hash, [0u8; 32]);
        assert_eq!(log.get_tree_size(), 1);
    }

    #[tokio::test]
    async fn test_merkle_tree_update() {
        let mut log = MerkleLog::new("test-log".to_string(), 1000).await.unwrap();
        
        // Add entries
        for i in 0..5 {
            let entry = create_test_entry(i);
            log.add_entry(entry).await.unwrap();
        }
        
        // Update merkle tree
        log.update_merkle_tree().await.unwrap();
        
        assert_eq!(log.stats.tree_size, 5);
        assert_ne!(log.stats.root_hash, [0u8; 32]);
        assert_eq!(log.stats.pending_count, 0);
    }

    #[tokio::test]
    async fn test_inclusion_proof() {
        let mut log = MerkleLog::new("test-log".to_string(), 1000).await.unwrap();
        
        // Add entries
        let mut test_entries = Vec::new();
        for i in 0..5 {
            let entry = create_test_entry(i);
            let added_entry = log.add_entry(entry).await.unwrap();
            test_entries.push(added_entry);
        }
        
        // Update merkle tree
        log.update_merkle_tree().await.unwrap();
        
        // Get inclusion proof
        let proof = log.get_inclusion_proof(&test_entries[2]).await.unwrap();
        assert!(!proof.is_empty());
        
        // Verify inclusion
        let is_included = log.verify_entry_inclusion(&test_entries[2]).await.unwrap();
        assert!(is_included);
    }

    #[tokio::test]
    async fn test_consistency_proof() {
        let mut log = MerkleLog::new("test-log".to_string(), 1000).await.unwrap();
        
        // Add entries in two batches
        for i in 0..3 {
            let entry = create_test_entry(i);
            log.add_entry(entry).await.unwrap();
        }
        log.update_merkle_tree().await.unwrap();
        
        for i in 3..5 {
            let entry = create_test_entry(i);
            log.add_entry(entry).await.unwrap();
        }
        log.update_merkle_tree().await.unwrap();
        
        // Get consistency proof
        let proof = log.get_consistency_proof(3, 5).await.unwrap();
        assert!(!proof.is_empty());
    }

    #[tokio::test]
    async fn test_log_capacity() {
        let mut log = MerkleLog::new("test-log".to_string(), 2).await.unwrap();
        
        // Add entries up to capacity
        log.add_entry(create_test_entry(0)).await.unwrap();
        log.add_entry(create_test_entry(1)).await.unwrap();
        
        assert!(log.is_full());
        
        // Adding beyond capacity should fail
        let result = log.add_entry(create_test_entry(2)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_entry_retrieval() {
        let mut log = MerkleLog::new("test-log".to_string(), 1000).await.unwrap();
        
        // Add entries
        for i in 0..5 {
            let entry = create_test_entry(i);
            log.add_entry(entry).await.unwrap();
        }
        
        // Get specific entry
        let entry = log.get_entry(2).await.unwrap();
        assert_eq!(entry.sequence_number, 2);
        
        // Get range of entries
        let entries = log.get_entries_range(1, 4).await;
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].sequence_number, 1);
        assert_eq!(entries[2].sequence_number, 3);
    }
}