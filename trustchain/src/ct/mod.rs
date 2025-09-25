//! Certificate Transparency Implementation
//! 
//! TrustChain Certificate Transparency logs with merkle tree proofs,
//! real-time certificate fingerprinting, and consensus validation.

use std::sync::Arc;
use std::time::{SystemTime, Duration, UNIX_EPOCH};
use std::collections::{HashMap, VecDeque};
use dashmap::DashMap;
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};
use tokio::sync::{RwLock, Mutex};
use tracing::{info, debug, warn, error};
use sha2::{Sha256, Digest};
// use merkletree::{MerkleTree, Proof, Hashable}; // Temporarily commented due to API changes

use crate::config::CTConfig;
use crate::consensus::{ConsensusProof, ConsensusContext, ConsensusRequirements};
use crate::errors::{CTError, TrustChainError, Result as TrustChainResult};

pub mod merkle_log;
pub mod sct_manager;
pub mod fingerprint_tracker;
// pub mod storage; // Temporarily disabled due to SQLx compile-time check issues
pub mod simple_storage;
pub mod certificate_transparency;
pub mod stoq_ct_client;

pub use merkle_log::*;
pub use sct_manager::*;
pub use fingerprint_tracker::*;
pub use simple_storage::{SimpleCTStorage as CTStorage, StorageStats};
pub use certificate_transparency::*;
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
    /// Consensus validation context
    consensus_context: Arc<ConsensusContext>,
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
    /// Associated consensus proof
    pub consensus_proof: ConsensusProof,
    /// Entry ID (hash of entry data)
    pub entry_id: [u8; 32],
    /// Merkle tree leaf hash
    pub leaf_hash: [u8; 32],
}

impl LogEntry {
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.sequence_number.to_be_bytes());
        hasher.update(&self.certificate_der);
        hasher.update(&self.fingerprint);
        hasher.update(&self.timestamp.duration_since(UNIX_EPOCH).unwrap().as_secs().to_be_bytes());
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
        info!("Initializing Certificate Transparency service: {}", config.log_id);

        // Initialize storage backend
        let storage = Arc::new(CTStorage::new(&config.storage_path).await?);

        // Initialize SCT manager
        let sct_manager = Arc::new(SCTManager::new(config.log_id.clone()).await?);

        // Initialize fingerprint tracker
        let fingerprint_tracker = Arc::new(FingerprintTracker::new(
            config.enable_realtime_fingerprinting,
        ).await?);

        // Initialize merkle logs (start with single log, will auto-shard)
        let logs = Arc::new(DashMap::new());
        let initial_log = Arc::new(RwLock::new(MerkleLog::new(
            format!("{}-0", config.log_id),
            config.max_entries_per_shard,
        ).await?));
        logs.insert("0".to_string(), initial_log);

        // Initialize consensus context
        let consensus_context = Arc::new(ConsensusContext::new(
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
            consensus_context,
            task_handles: Arc::new(Mutex::new(Vec::new())),
        };

        // Start background tasks
        ct.start_background_tasks().await?;

        info!("Certificate Transparency service initialized successfully");
        Ok(ct)
    }

    /// Log a certificate with CT entry and SCT generation
    pub async fn log_certificate(&self, cert_der: &[u8]) -> TrustChainResult<SignedCertificateTimestamp> {
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
            }.into());
        }

        // Create log entry
        let sequence_number = self.get_next_sequence_number().await?;
        let timestamp = SystemTime::now();
        let entry_id = self.calculate_entry_id(sequence_number, cert_der, &timestamp);
        
        let log_entry = LogEntry {
            sequence_number,
            certificate_der: cert_der.to_vec(),
            fingerprint,
            timestamp,
            common_name,
            issuer_ca_id,
            consensus_proof: ConsensusProof::generate_from_network(&self.log_id).await
                .map_err(|e| TrustChainError::ConsensusValidationFailed {
                    reason: format!("Failed to generate consensus proof: {}", e)
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
        let sct = self.sct_manager.generate_sct(
            &updated_entry,
            &self.log_id,
        ).await?;

        // Track fingerprint for real-time monitoring
        if self.config.enable_realtime_fingerprinting {
            self.fingerprint_tracker.track_certificate(
                fingerprint,
                updated_entry.common_name.clone(),
                timestamp,
            ).await?;
        }

        debug!("Certificate logged successfully with sequence number: {}", sequence_number);
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
        
        let entry = self.storage.get_entry_by_fingerprint(&fingerprint).await?
            .ok_or_else(|| CTError::EntryNotFound {
                entry_id: hex::encode(fingerprint),
            })?;

        // Get merkle proof from appropriate shard
        let shard_id = self.get_shard_for_entry(entry.sequence_number).await;
        let merkle_log = self.logs.get(&shard_id)
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
    pub async fn get_consistency_proof(&self, old_size: u64, new_size: u64) -> TrustChainResult<Vec<[u8; 32]>> {
        debug!("Generating consistency proof: {} -> {}", old_size, new_size);

        // Find the appropriate shard for the new size
        let shard_id = self.get_shard_for_entry(new_size - 1).await;
        let merkle_log = self.logs.get(&shard_id)
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
        let total_entries = self.get_next_sequence_number().await? - 1;
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

    fn calculate_entry_id(&self, seq_num: u64, cert_der: &[u8], timestamp: &SystemTime) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&seq_num.to_be_bytes());
        hasher.update(cert_der);
        hasher.update(&timestamp.duration_since(UNIX_EPOCH).unwrap().as_secs().to_be_bytes());
        hasher.finalize().into()
    }

    async fn get_shard_for_entry(&self, sequence_number: u64) -> String {
        let shard_id = sequence_number / self.config.max_entries_per_shard;
        shard_id.to_string()
    }

    async fn get_or_create_log_shard(&self, shard_id: &str) -> TrustChainResult<Arc<RwLock<MerkleLog>>> {
        if let Some(log) = self.logs.get(shard_id) {
            Ok(log.clone())
        } else {
            info!("Creating new CT log shard: {}", shard_id);
            let new_log = Arc::new(RwLock::new(MerkleLog::new(
                format!("{}-{}", self.log_id, shard_id),
                self.config.max_entries_per_shard,
            ).await?));
            self.logs.insert(shard_id.to_string(), new_log.clone());
            Ok(new_log)
        }
    }

    fn parse_certificate_metadata(&self, cert_der: &[u8]) -> TrustChainResult<(String, String)> {
        use x509_parser::parse_x509_certificate;
        
        let (_, parsed_cert) = parse_x509_certificate(cert_der)
            .map_err(|e| CTError::RealtimeFingerprinting {
                certificate_id: "unknown".to_string(),
            })?;

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
        let temp_dir = TempDir::new().unwrap();
        let mut config = CTConfig::default();
        config.storage_path = temp_dir.path().to_str().unwrap().to_string();
        config.enable_realtime_fingerprinting = false; // Disable for testing
        
        let ct = CertificateTransparency::new(config).await.unwrap();
        (ct, temp_dir)
    }

    #[tokio::test]
    async fn test_certificate_logging() {
        let (ct, _temp_dir) = create_test_ct().await;
        
        let test_cert = b"test certificate data";
        let sct = ct.log_certificate(test_cert).await.unwrap();
        
        assert_eq!(sct.version, 1);
        assert!(!sct.signature.is_empty());
    }

    #[tokio::test]
    async fn test_certificate_verification() {
        let (ct, _temp_dir) = create_test_ct().await;
        
        let test_cert = b"test certificate data";
        ct.log_certificate(test_cert).await.unwrap();
        
        let is_verified = ct.verify_certificate_in_logs(test_cert).await.unwrap();
        assert!(is_verified);
        
        let not_logged_cert = b"not logged certificate";
        let is_not_verified = ct.verify_certificate_in_logs(not_logged_cert).await.unwrap();
        assert!(!is_not_verified);
    }

    #[tokio::test]
    async fn test_inclusion_proof() {
        let (ct, _temp_dir) = create_test_ct().await;
        
        let test_cert = b"test certificate for inclusion proof";
        ct.log_certificate(test_cert).await.unwrap();
        
        let proof = ct.get_inclusion_proof(test_cert).await.unwrap();
        assert_eq!(proof.log_id, ct.log_id);
        assert_eq!(proof.sequence_number, 0); // First entry
    }

    #[tokio::test]
    async fn test_log_stats() {
        let (ct, _temp_dir) = create_test_ct().await;
        
        let test_cert = b"test certificate for stats";
        ct.log_certificate(test_cert).await.unwrap();
        
        let stats = ct.get_log_stats().await.unwrap();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.shard_count, 1);
    }

    #[tokio::test]
    async fn test_get_entries_range() {
        let (ct, _temp_dir) = create_test_ct().await;
        
        // Log multiple certificates
        for i in 0..5 {
            let cert_data = format!("test certificate {}", i);
            ct.log_certificate(cert_data.as_bytes()).await.unwrap();
        }
        
        let entries = ct.get_entries(0, 3).await.unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].sequence_number, 0);
        assert_eq!(entries[2].sequence_number, 2);
    }
}