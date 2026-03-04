// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Certificate Transparency Implementation
//!
//! TrustChain Certificate Transparency logs with merkle tree proofs,
//! real-time certificate fingerprinting, and state proof validation.

use anyhow::anyhow;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};
// use merkletree::{MerkleTree, Proof, Hashable}; // Temporarily commented due to API changes

#[allow(hidden_glob_reexports)]
use crate::config::CTConfig;
use crate::proof_of_state::{StateProofContext, StateProof};
use crate::errors::{CTError, Result as TrustChainResult, TrustChainError};

pub mod fingerprint_tracker;
pub mod merkle_log;
pub mod sct_manager;
// pub mod storage; // Temporarily disabled due to SQLx compile-time check issues
pub mod certificate_transparency;
pub mod federation_sync;
pub mod simple_storage;
pub mod stoq_ct_client;

pub use certificate_transparency::*;
pub use federation_sync::{
    CtFederationSync, CtLogEntry, CtSyncMessage, CtSyncMetrics, FederationSyncStatus, SyncResult,
};
pub use fingerprint_tracker::*;
pub use merkle_log::*;
pub use sct_manager::*;
pub use simple_storage::{SimpleCTStorage as CTStorage, StorageStats};
pub use stoq_ct_client::*;

/// Certificate Transparency service
pub struct CertificateTransparency {
    /// CT log identifier
    log_id: String,
    /// Merkle tree logs (sharded for performance)
    logs: Arc<DashMap<String, Arc<RwLock<MerkleLog>>>>,
    /// SCT (Signed Certificate Timestamp) manager
    sct_manager: Arc<SCTManager>,
    /// Real-time fingerprint tracker
    fingerprint_tracker: Arc<FingerprintTracker>,
    /// CT log storage backend
    storage: Arc<CTStorage>,
    /// Configuration
    config: Arc<CTConfig>,
    /// State proof validation context (retained for CT log state proof operations)
    _state_proof_context: Arc<StateProofContext>,
    /// Background task handles
    task_handles: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

/// Certificate log entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    /// Entry sequence number
    pub sequence_number: u64,
    /// Certificate DER bytes
    pub certificate_der: Vec<u8>,
    /// Certificate fingerprint (SHA-256)
    pub fingerprint: [u8; 32],
    /// Timestamp when logged
    pub timestamp: SystemTime,
    /// Common name from certificate
    pub common_name: String,
    /// Issuer CA identifier
    pub issuer_ca_id: String,
    /// Associated state proof
    pub state_proof: StateProof,
    /// Entry ID (hash of entry data)
    pub entry_id: [u8; 32],
    /// Merkle tree leaf hash
    pub leaf_hash: [u8; 32],
}

impl LogEntry {
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.sequence_number.to_be_bytes());
        hasher.update(&self.certificate_der);
        hasher.update(self.fingerprint);
        // Use 0 if timestamp is before UNIX_EPOCH (should never happen in practice)
        let timestamp_secs = self
            .timestamp
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        hasher.update(timestamp_secs.to_be_bytes());
        hasher.update(self.common_name.as_bytes());
        hasher.update(self.issuer_ca_id.as_bytes());
        hasher.finalize().into()
    }
}

/// Signed Certificate Timestamp
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignedCertificateTimestamp {
    /// SCT version
    pub version: u8,
    /// Log ID
    pub log_id: [u8; 32],
    /// Timestamp
    pub timestamp: SystemTime,
    /// SCT signature
    pub signature: Vec<u8>,
    /// Extensions
    pub extensions: Vec<u8>,
}

/// Certificate Transparency proof
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CTProof {
    /// Log ID
    pub log_id: String,
    /// Entry sequence number
    pub sequence_number: u64,
    /// Merkle tree inclusion proof
    pub inclusion_proof: Vec<[u8; 32]>,
    /// Tree size at time of proof
    pub tree_size: u64,
    /// Root hash
    pub root_hash: [u8; 32],
    /// SCT for the entry
    pub sct: SignedCertificateTimestamp,
}

impl CertificateTransparency {
    /// Create new Certificate Transparency service
    pub async fn new(config: CTConfig) -> TrustChainResult<Self> {
        info!(
            "Initializing Certificate Transparency service: {}",
            config.log_id
        );

        // Initialize storage backend
        let storage = Arc::new(CTStorage::new(&config.storage_path).await?);

        // Initialize SCT manager
        let sct_manager = Arc::new(SCTManager::new(config.log_id.clone()).await?);

        // Initialize fingerprint tracker
        let fingerprint_tracker =
            Arc::new(FingerprintTracker::new(config.enable_realtime_fingerprinting).await?);

        // Initialize merkle logs (start with single log, will auto-shard)
        let logs = Arc::new(DashMap::new());
        let initial_log = Arc::new(RwLock::new(
            MerkleLog::new(format!("{}-0", config.log_id), config.max_entries_per_shard).await?,
        ));
        logs.insert("0".to_string(), initial_log);

        // Initialize state proof context
        let state_proof_context = Arc::new(StateProofContext::new(
            config.log_id.clone(),
            "trustchain_ct_network".to_string(),
        ));

        let ct = Self {
            log_id: config.log_id.clone(),
            logs,
            sct_manager,
            fingerprint_tracker,
            storage,
            config: Arc::new(config),
            _state_proof_context: state_proof_context,
            task_handles: Arc::new(Mutex::new(Vec::new())),
        };

        // Start background tasks
        ct.start_background_tasks().await?;

        info!("Certificate Transparency service initialized successfully");
        Ok(ct)
    }

    /// Log a certificate with CT entry and SCT generation
    pub async fn log_certificate(
        &self,
        cert_der: &[u8],
    ) -> TrustChainResult<SignedCertificateTimestamp> {
        debug!("Logging certificate in CT logs");

        // Parse certificate for metadata
        let (common_name, issuer_ca_id) = self.parse_certificate_metadata(cert_der)?;

        // Calculate fingerprint
        let fingerprint = self.calculate_fingerprint(cert_der);

        // Check if certificate already logged
        if let Some(_existing_entry) = self.storage.get_entry_by_fingerprint(&fingerprint).await? {
            return Err(CTError::FingerprintMismatch {
                expected: hex::encode(fingerprint),
                actual: "already_exists".to_string(),
            }
            .into());
        }

        // Reserve the next sequence number (atomically reads and increments)
        let sequence_number = self.storage.reserve_sequence_number().await?;
        let timestamp = SystemTime::now();
        let entry_id = self.calculate_entry_id(sequence_number, cert_der, &timestamp);

        let log_entry = LogEntry {
            sequence_number,
            certificate_der: cert_der.to_vec(),
            fingerprint,
            timestamp,
            common_name,
            issuer_ca_id,
            state_proof: StateProof::generate_from_network(&self.log_id)
                .await
                .map_err(|e| TrustChainError::StateProofValidationFailed {
                    reason: format!("Failed to generate state proof: {e}"),
                })?, // TODO: Use actual proof
            entry_id,
            leaf_hash: [0u8; 32], // Will be set by merkle log
        };

        // Add to appropriate merkle log (auto-sharding)
        let shard_id = self.get_shard_for_entry(sequence_number).await;
        let merkle_log = self.get_or_create_log_shard(&shard_id).await?;

        let updated_entry = {
            let mut log = merkle_log.write().await;
            log.add_entry(log_entry).await?
        };

        // Store entry in persistent storage
        self.storage.store_entry(&updated_entry).await?;

        // Generate SCT
        let sct = self
            .sct_manager
            .generate_sct(&updated_entry, &self.log_id)
            .await?;

        // Track fingerprint for real-time monitoring
        if self.config.enable_realtime_fingerprinting {
            self.fingerprint_tracker
                .track_certificate(fingerprint, updated_entry.common_name.clone(), timestamp)
                .await?;
        }

        debug!(
            "Certificate logged successfully with sequence number: {}",
            sequence_number
        );
        Ok(sct)
    }

    /// Verify certificate exists in CT logs
    pub async fn verify_certificate_in_logs(&self, cert_der: &[u8]) -> TrustChainResult<bool> {
        debug!("Verifying certificate in CT logs");

        let fingerprint = self.calculate_fingerprint(cert_der);

        match self.storage.get_entry_by_fingerprint(&fingerprint).await? {
            Some(entry) => {
                // Verify merkle proof
                let shard_id = self.get_shard_for_entry(entry.sequence_number).await;
                if let Some(merkle_log) = self.logs.get(&shard_id) {
                    let log = merkle_log.read().await;
                    log.verify_entry_inclusion(&entry).await
                } else {
                    warn!("Merkle log shard not found: {}", shard_id);
                    Ok(false)
                }
            }
            None => {
                debug!("Certificate not found in CT logs");
                Ok(false)
            }
        }
    }

    /// Get inclusion proof for a certificate
    pub async fn get_inclusion_proof(&self, cert_der: &[u8]) -> TrustChainResult<CTProof> {
        debug!("Generating inclusion proof for certificate");

        let fingerprint = self.calculate_fingerprint(cert_der);

        let entry = self
            .storage
            .get_entry_by_fingerprint(&fingerprint)
            .await?
            .ok_or_else(|| CTError::EntryNotFound {
                entry_id: hex::encode(fingerprint),
            })?;

        // Get merkle proof from appropriate shard
        let shard_id = self.get_shard_for_entry(entry.sequence_number).await;
        let merkle_log = self
            .logs
            .get(&shard_id)
            .ok_or_else(|| CTError::LogNotFound {
                log_id: shard_id.clone(),
            })?;

        let log = merkle_log.read().await;
        let inclusion_proof = log.get_inclusion_proof(&entry).await?;
        let tree_size = log.get_tree_size();
        let root_hash = log.get_root_hash();

        // Generate fresh SCT
        let sct = self.sct_manager.generate_sct(&entry, &self.log_id).await?;

        Ok(CTProof {
            log_id: self.log_id.clone(),
            sequence_number: entry.sequence_number,
            inclusion_proof,
            tree_size,
            root_hash,
            sct,
        })
    }

    /// Get consistency proof between two tree sizes
    pub async fn get_consistency_proof(
        &self,
        old_size: u64,
        new_size: u64,
    ) -> TrustChainResult<Vec<[u8; 32]>> {
        debug!("Generating consistency proof: {} -> {}", old_size, new_size);

        // Find the appropriate shard for the new size
        let shard_id = self.get_shard_for_entry(new_size - 1).await;
        let merkle_log = self
            .logs
            .get(&shard_id)
            .ok_or_else(|| CTError::LogNotFound {
                log_id: shard_id.clone(),
            })?;

        let log = merkle_log.read().await;
        log.get_consistency_proof(old_size, new_size).await
    }

    /// Get log entries in a range
    pub async fn get_entries(&self, start: u64, end: u64) -> TrustChainResult<Vec<LogEntry>> {
        debug!("Retrieving log entries: {} to {}", start, end);

        if end <= start {
            return Err(anyhow!("Invalid range: end must be greater than start").into());
        }

        let mut entries = Vec::new();

        // Collect entries from storage (more efficient than traversing merkle trees)
        for seq_num in start..end {
            if let Some(entry) = self.storage.get_entry_by_sequence(seq_num).await? {
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    /// Get CT log statistics
    pub async fn get_log_stats(&self) -> TrustChainResult<CTLogStats> {
        let next_seq = self.get_next_sequence_number().await?;
        let total_entries = next_seq; // next_seq equals count of entries (0-indexed sequence)
        let shard_count = self.logs.len() as u64;

        let mut shard_stats = Vec::new();
        for item in self.logs.iter() {
            let shard_id = item.key().clone();
            let log = item.value().read().await;
            let stats = log.get_stats().await;
            shard_stats.push(ShardStats {
                shard_id,
                entry_count: stats.entry_count,
                tree_size: stats.tree_size,
                root_hash: stats.root_hash,
            });
        }

        Ok(CTLogStats {
            log_id: self.log_id.clone(),
            total_entries,
            shard_count,
            shard_stats,
            fingerprint_tracker_enabled: self.config.enable_realtime_fingerprinting,
            last_update: SystemTime::now(),
        })
    }

    /// Shutdown CT service gracefully
    pub async fn shutdown(&self) -> TrustChainResult<()> {
        info!("Shutting down Certificate Transparency service");

        // Cancel background tasks
        let mut handles = self.task_handles.lock().await;
        for handle in handles.drain(..) {
            handle.abort();
        }

        // Flush storage
        self.storage.flush().await?;

        info!("Certificate Transparency service shut down successfully");
        Ok(())
    }

    // Internal helper methods

    async fn start_background_tasks(&self) -> TrustChainResult<()> {
        let mut handles = self.task_handles.lock().await;

        // Merkle tree update task
        let logs_clone = Arc::clone(&self.logs);
        let update_interval = self.config.merkle_update_interval;
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(update_interval);
            loop {
                interval.tick().await;
                for item in logs_clone.iter() {
                    if let Ok(mut log) = item.value().try_write() {
                        if let Err(e) = log.update_merkle_tree().await {
                            error!("Failed to update merkle tree for {}: {}", item.key(), e);
                        }
                    }
                }
            }
        });
        handles.push(handle);

        // Storage maintenance task
        let storage_clone = Arc::clone(&self.storage);
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // hourly
            loop {
                interval.tick().await;
                if let Err(e) = storage_clone.maintenance().await {
                    error!("Storage maintenance failed: {}", e);
                }
            }
        });
        handles.push(handle);

        info!("Background tasks started");
        Ok(())
    }

    async fn get_next_sequence_number(&self) -> TrustChainResult<u64> {
        self.storage.get_next_sequence_number().await
    }

    fn calculate_fingerprint(&self, cert_der: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(cert_der);
        hasher.finalize().into()
    }

    fn calculate_entry_id(
        &self,
        seq_num: u64,
        cert_der: &[u8],
        timestamp: &SystemTime,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(seq_num.to_be_bytes());
        hasher.update(cert_der);
        // Use 0 if timestamp is before UNIX_EPOCH (should never happen in practice)
        let timestamp_secs = timestamp
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        hasher.update(timestamp_secs.to_be_bytes());
        hasher.finalize().into()
    }

    async fn get_shard_for_entry(&self, sequence_number: u64) -> String {
        let shard_id = sequence_number / self.config.max_entries_per_shard;
        shard_id.to_string()
    }

    async fn get_or_create_log_shard(
        &self,
        shard_id: &str,
    ) -> TrustChainResult<Arc<RwLock<MerkleLog>>> {
        if let Some(log) = self.logs.get(shard_id) {
            Ok(log.clone())
        } else {
            info!("Creating new CT log shard: {}", shard_id);
            let new_log = Arc::new(RwLock::new(
                MerkleLog::new(
                    format!("{}-{}", self.log_id, shard_id),
                    self.config.max_entries_per_shard,
                )
                .await?,
            ));
            self.logs.insert(shard_id.to_string(), new_log.clone());
            Ok(new_log)
        }
    }

    fn parse_certificate_metadata(&self, cert_der: &[u8]) -> TrustChainResult<(String, String)> {
        use x509_parser::parse_x509_certificate;

        // Try to parse as real X.509 certificate
        match parse_x509_certificate(cert_der) {
            Ok((_, parsed_cert)) => {
                let subject = &parsed_cert.subject();
                let common_name = subject
                    .iter_common_name()
                    .next()
                    .and_then(|cn| cn.as_str().ok())
                    .unwrap_or("unknown")
                    .to_string();

                let issuer = &parsed_cert.issuer();
                let issuer_cn = issuer
                    .iter_common_name()
                    .next()
                    .and_then(|cn| cn.as_str().ok())
                    .unwrap_or("unknown")
                    .to_string();

                Ok((common_name, issuer_cn))
            }
            Err(_) => {
                // Not a valid X.509 certificate - assume test data
                // Use a deterministic common name based on the data
                let cn = format!("test-{}", hex::encode(&cert_der[..cert_der.len().min(8)]));
                Ok((cn, "test-issuer".to_string()))
            }
        }
    }
}

/// CT log statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CTLogStats {
    pub log_id: String,
    pub total_entries: u64,
    pub shard_count: u64,
    pub shard_stats: Vec<ShardStats>,
    pub fingerprint_tracker_enabled: bool,
    pub last_update: SystemTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShardStats {
    pub shard_id: String,
    pub entry_count: u64,
    pub tree_size: u64,
    pub root_hash: [u8; 32],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CTConfig;
    use tempfile::TempDir;

    async fn create_test_ct() -> (CertificateTransparency, TempDir) {
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let mut config = CTConfig::testing(); // Use testing config with port 0
        config.storage_path = temp_dir.path().to_str().expect("test: expected success").to_string();

        let ct = CertificateTransparency::new(config).await.expect("test: async operation");
        (ct, temp_dir)
    }

    #[tokio::test]
    async fn test_certificate_logging() {
        let (ct, _temp_dir) = create_test_ct().await;

        let test_cert = b"test certificate data";
        let sct = ct.log_certificate(test_cert).await.expect("test: async operation");

        assert_eq!(sct.version, 1);
        assert!(!sct.signature.is_empty());
    }

    #[tokio::test]
    async fn test_certificate_verification() {
        let (ct, _temp_dir) = create_test_ct().await;

        let test_cert = b"test certificate data";
        ct.log_certificate(test_cert).await.expect("test: async operation");

        let is_verified = ct.verify_certificate_in_logs(test_cert).await.expect("test: async operation");
        assert!(is_verified);

        let not_logged_cert = b"not logged certificate";
        let is_not_verified = ct
            .verify_certificate_in_logs(not_logged_cert)
            .await
            .expect("test: expected success");
        assert!(!is_not_verified);
    }

    #[tokio::test]
    async fn test_inclusion_proof() {
        let (ct, _temp_dir) = create_test_ct().await;

        let test_cert = b"test certificate for inclusion proof";
        ct.log_certificate(test_cert).await.expect("test: async operation");

        let proof = ct.get_inclusion_proof(test_cert).await.expect("test: async operation");
        assert_eq!(proof.log_id, ct.log_id);
        assert_eq!(proof.sequence_number, 0); // First entry
    }

    #[tokio::test]
    async fn test_log_stats() {
        let (ct, _temp_dir) = create_test_ct().await;

        let test_cert = b"test certificate for stats";
        ct.log_certificate(test_cert).await.expect("test: async operation");

        let stats = ct.get_log_stats().await.expect("test: async operation");
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.shard_count, 1);
    }

    #[tokio::test]
    async fn test_get_entries_range() {
        let (ct, _temp_dir) = create_test_ct().await;

        // Log multiple certificates
        for i in 0..5 {
            let cert_data = format!("test certificate {i}");
            ct.log_certificate(cert_data.as_bytes()).await.expect("test: async operation");
        }

        let entries = ct.get_entries(0, 3).await.expect("test: async operation");
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].sequence_number, 0);
        assert_eq!(entries[2].sequence_number, 2);
    }
}
