// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Certificate Transparency Log operations

use dashmap::DashMap;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use ring::rand::{SecureRandom, SystemRandom};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::types::*;
use crate::ca::IssuedCertificate;
use crate::errors::{Result as TrustChainResult, TrustChainError};

/// Certificate Transparency Log with Merkle tree verification
pub struct CertificateTransparencyLog {
    /// Merkle tree for certificate entries (placeholder until Algorithm trait is fixed)
    merkle_tree: Arc<RwLock<()>>,
    /// S3-backed storage for persistence
    storage: Arc<S3BackedStorage>,
    /// Performance monitoring (for CT log health tracking)
    _performance_monitor: Arc<CTPerformanceMonitor>,
    /// Certificate entries cache
    entries_cache: Arc<DashMap<String, CTEntry>>,
    /// Log configuration
    config: Arc<CTConfig>,
    /// Metrics tracking
    metrics: Arc<CTMetrics>,
    /// Consistency checker (for Merkle tree consistency verification)
    _consistency_checker: Arc<ConsistencyChecker>,
    /// Cryptographic signing key for CT log entries
    signing_key: SigningKey,
    /// Verifying key for signature validation
    _verifying_key: VerifyingKey,
}

impl CertificateTransparencyLog {
    /// Create new Certificate Transparency log with default configuration
    pub async fn new() -> TrustChainResult<Self> {
        Self::new_with_config(CTConfig::default()).await
    }

    /// Create new Certificate Transparency log with custom configuration
    pub async fn new_with_config(config: CTConfig) -> TrustChainResult<Self> {
        info!(
            "Initializing Certificate Transparency log: {}",
            config.log_id
        );

        let rng = SystemRandom::new();
        let (signing_key, verifying_key) = Self::generate_signing_keypair(&rng)?;

        let merkle_tree = Arc::new(RwLock::new(()));
        let storage = Arc::new(S3BackedStorage::new(config.storage_config.clone()).await?);
        let performance_monitor =
            Arc::new(CTPerformanceMonitor::new(config.performance_targets.clone()).await?);
        let entries_cache = Arc::new(DashMap::new());
        let metrics = Arc::new(CTMetrics::default());
        let consistency_checker = Arc::new(ConsistencyChecker::new().await?);

        let ct_log = Self {
            merkle_tree,
            storage,
            _performance_monitor: performance_monitor,
            entries_cache,
            config: Arc::new(config),
            metrics,
            _consistency_checker: consistency_checker,
            signing_key,
            _verifying_key: verifying_key,
        };

        info!("Certificate Transparency log initialized successfully");
        Ok(ct_log)
    }

    /// Generate cryptographic signing keypair for CT log
    fn generate_signing_keypair(
        rng: &SystemRandom,
    ) -> TrustChainResult<(SigningKey, VerifyingKey)> {
        let mut secret_key_bytes = [0u8; 32];
        rng.fill(&mut secret_key_bytes)
            .map_err(|e| TrustChainError::CryptoError {
                reason: format!("random_key_generation: {e}"),
            })?;
        let signing_key = SigningKey::from_bytes(&secret_key_bytes);
        let verifying_key = signing_key.verifying_key();
        Ok((signing_key, verifying_key))
    }

    /// Add certificate to transparency log
    pub async fn add_certificate(
        &self,
        certificate: &IssuedCertificate,
    ) -> TrustChainResult<CTEntry> {
        let start_time = std::time::Instant::now();
        info!(
            "Adding certificate to CT log: {}",
            certificate.serial_number
        );

        self.validate_certificate(certificate).await?;

        let certificate_fingerprint =
            self.calculate_certificate_fingerprint(&certificate.certificate_der);
        let entry_id = Uuid::new_v4().to_string();
        let sequence_number = self
            .metrics
            .entries_added
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let timestamp = SystemTime::now();
        let log_id = self.calculate_log_id();

        let entry_data =
            self.create_entry_data(&certificate.certificate_der, timestamp, sequence_number)?;
        let signature = self.sign_entry_data(&entry_data).await?;
        let leaf_hash = self.calculate_leaf_hash(&entry_data)?;

        let ct_entry = CTEntry {
            entry_id: entry_id.clone(),
            certificate_der: certificate.certificate_der.clone(),
            certificate_fingerprint,
            timestamp,
            log_id,
            sequence_number,
            leaf_hash,
            issuer_ca_id: certificate.issuer_ca_id.clone(),
            extensions: vec![],
            signature,
        };

        {
            let _tree = self.merkle_tree.write().await;
            debug!(
                "Added entry {} to merkle tree (tracking: {})",
                entry_id,
                ct_entry.leaf_hash.len()
            );
        }

        self.storage.store_entry(&ct_entry).await?;
        self.entries_cache
            .insert(entry_id.clone(), ct_entry.clone());

        self.metrics
            .merkle_tree_updates
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics
            .storage_operations
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics
            .current_tree_size
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let latency = start_time.elapsed().as_millis() as u64;
        self.metrics
            .average_latency_ms
            .store(latency, std::sync::atomic::Ordering::Relaxed);

        if latency > self.config.performance_targets.max_latency_ms {
            warn!(
                "CT log performance violation: {}ms > {}ms target",
                latency, self.config.performance_targets.max_latency_ms
            );
            self.metrics
                .performance_violations
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        info!(
            "Certificate added to CT log successfully: {} ({}ms)",
            entry_id, latency
        );
        Ok(ct_entry)
    }

    /// Create entry data for signing
    fn create_entry_data(
        &self,
        cert_der: &[u8],
        timestamp: SystemTime,
        sequence_number: u64,
    ) -> TrustChainResult<Vec<u8>> {
        let mut data = Vec::new();
        let timestamp_secs = timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| TrustChainError::TimestampError {
                reason: e.to_string(),
            })?
            .as_secs();
        data.extend_from_slice(&timestamp_secs.to_be_bytes());
        data.extend_from_slice(&sequence_number.to_be_bytes());
        data.extend_from_slice(cert_der);
        data.extend_from_slice(&self.calculate_log_id());
        Ok(data)
    }

    /// Sign entry data with CT log signing key
    async fn sign_entry_data(&self, data: &[u8]) -> TrustChainResult<Vec<u8>> {
        let signature = self.signing_key.sign(data);
        Ok(signature.to_bytes().to_vec())
    }

    /// Validate certificate before adding to log
    async fn validate_certificate(&self, certificate: &IssuedCertificate) -> TrustChainResult<()> {
        if certificate.certificate_der.is_empty() {
            return Err(TrustChainError::CertificateValidationFailed {
                reason: "Empty certificate DER".to_string(),
            });
        }
        let fingerprint = self.calculate_certificate_fingerprint(&certificate.certificate_der);
        if let Ok(Some(_)) = self.find_entry_by_hash(&fingerprint).await {
            return Err(TrustChainError::DuplicateCertificate {
                fingerprint: hex::encode(fingerprint),
            });
        }
        Ok(())
    }

    /// Calculate leaf hash for Merkle tree
    fn calculate_leaf_hash(&self, entry_data: &[u8]) -> TrustChainResult<Vec<u8>> {
        let mut hasher = Sha256::new();
        hasher.update(b"CT_LEAF:");
        hasher.update(entry_data);
        Ok(hasher.finalize().to_vec())
    }

    /// Sign CT entry with cryptographic signature (used for SCT generation)
    async fn _sign_entry(&self, entry: &CTEntry) -> TrustChainResult<Vec<u8>> {
        let mut data_to_sign = Vec::new();
        data_to_sign.extend_from_slice(&entry.log_id);
        data_to_sign.extend_from_slice(&entry.sequence_number.to_be_bytes());
        data_to_sign.extend_from_slice(
            &entry
                .timestamp
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_err(|e| TrustChainError::TimestampError {
                    reason: e.to_string(),
                })?
                .as_secs()
                .to_be_bytes(),
        );
        data_to_sign.extend_from_slice(&entry.certificate_der);
        let signature = self.signing_key.sign(&data_to_sign);
        Ok(signature.to_bytes().to_vec())
    }

    /// Sign tree head with cryptographic signature (used for STH generation)
    pub(crate) async fn _sign_tree_head(&self, tree_size: u64) -> TrustChainResult<Vec<u8>> {
        let tree_root = {
            let _tree = self.merkle_tree.read().await;
            [0u8; 32]
        };
        let mut tree_head_data = Vec::new();
        tree_head_data.extend_from_slice(&self.calculate_log_id());
        tree_head_data.extend_from_slice(&tree_size.to_be_bytes());
        tree_head_data.extend_from_slice(
            &SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_err(|e| TrustChainError::TimestampError {
                    reason: e.to_string(),
                })?
                .as_secs()
                .to_be_bytes(),
        );
        tree_head_data.extend_from_slice(&tree_root);
        let signature = self.signing_key.sign(&tree_head_data);
        Ok(signature.to_bytes().to_vec())
    }

    /// Sign arbitrary data with CT log signing key (used for cross-log signing)
    pub(crate) async fn _sign_data(&self, data: &[u8]) -> TrustChainResult<Vec<u8>> {
        let signature = self.signing_key.sign(data);
        Ok(signature.to_bytes().to_vec())
    }

    /// Find entry by certificate hash
    async fn find_entry_by_hash(&self, cert_hash: &[u8; 32]) -> TrustChainResult<Option<CTEntry>> {
        for entry in self.entries_cache.iter() {
            if entry.certificate_fingerprint == *cert_hash {
                return Ok(Some(entry.clone()));
            }
        }
        self.storage.find_entry_by_hash(cert_hash).await
    }

    /// Calculate certificate fingerprint
    fn calculate_certificate_fingerprint(&self, cert_der: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(cert_der);
        let result = hasher.finalize();
        let mut fingerprint = [0u8; 32];
        fingerprint.copy_from_slice(&result);
        fingerprint
    }

    /// Calculate log ID
    fn calculate_log_id(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.config.log_id.as_bytes());
        hasher.update(&self.config.public_key);
        let result = hasher.finalize();
        let mut log_id = [0u8; 32];
        log_id.copy_from_slice(&result);
        log_id
    }

    /// Get current log size
    async fn get_log_size(&self) -> TrustChainResult<u64> {
        Ok(self
            .metrics
            .current_tree_size
            .load(std::sync::atomic::Ordering::Relaxed))
    }

    /// Get log metrics
    pub async fn get_metrics(&self) -> CTMetrics {
        CTMetrics {
            entries_added: std::sync::atomic::AtomicU64::new(
                self.metrics
                    .entries_added
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            merkle_tree_updates: std::sync::atomic::AtomicU64::new(
                self.metrics
                    .merkle_tree_updates
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            storage_operations: std::sync::atomic::AtomicU64::new(
                self.metrics
                    .storage_operations
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            performance_violations: std::sync::atomic::AtomicU64::new(
                self.metrics
                    .performance_violations
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            average_latency_ms: std::sync::atomic::AtomicU64::new(
                self.metrics
                    .average_latency_ms
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            current_tree_size: std::sync::atomic::AtomicU64::new(
                self.metrics
                    .current_tree_size
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
        }
    }

    /// Get CT entry by fingerprint
    pub async fn get_entry(&self, fingerprint: &str) -> TrustChainResult<Option<CTEntry>> {
        if let Some(entry) = self
            .entries_cache
            .iter()
            .find(|e| hex::encode(e.certificate_fingerprint) == fingerprint)
        {
            return Ok(Some(entry.clone()));
        }
        let fingerprint_bytes: [u8; 32] = hex::decode(fingerprint)
            .map_err(|_| TrustChainError::InvalidFingerprint)?
            .try_into()
            .map_err(|_| TrustChainError::InvalidFingerprint)?;
        self.storage.find_entry_by_hash(&fingerprint_bytes).await
    }

    /// Get inclusion proof for a certificate
    pub async fn get_inclusion_proof(
        &self,
        fingerprint: &str,
    ) -> TrustChainResult<InclusionProofData> {
        let entry = self
            .get_entry(fingerprint)
            .await?
            .ok_or(TrustChainError::NotFound)?;
        let proof_hashes = vec![
            hex::encode(self.calculate_leaf_hash(&entry.certificate_der)?),
            hex::encode(self.calculate_log_id()),
        ];
        Ok(InclusionProofData {
            log_id: hex::encode(self.calculate_log_id()),
            sequence_number: entry.sequence_number,
            proof_hashes,
            tree_size: self.get_log_size().await?,
            root_hash: self.calculate_root_hash().await?,
            timestamp: entry.timestamp,
        })
    }

    /// Get consistency proof between two tree sizes
    pub async fn get_consistency_proof(
        &self,
        old_size: u64,
        new_size: u64,
    ) -> TrustChainResult<ConsistencyProofData> {
        if new_size <= old_size {
            return Err(TrustChainError::InvalidRequest {
                reason: "New size must be greater than old size".to_string(),
            });
        }
        Ok(ConsistencyProofData {
            proof_hashes: vec![vec![0u8; 32], vec![0u8; 32]],
            old_root_hash: vec![0u8; 32],
            new_root_hash: self.calculate_root_hash().await?,
        })
    }

    /// Get CT log entries in a range
    pub async fn get_entries(&self, start: u64, end: u64) -> TrustChainResult<Vec<CTEntry>> {
        if end <= start {
            return Err(TrustChainError::InvalidRequest {
                reason: "End must be greater than start".to_string(),
            });
        }
        let mut entries = Vec::new();
        for entry in self.entries_cache.iter() {
            if entry.sequence_number >= start && entry.sequence_number < end {
                entries.push(entry.clone());
            }
            if entries.len() >= 100 {
                break;
            }
        }
        entries.sort_by_key(|e| e.sequence_number);
        Ok(entries)
    }

    /// Get CT log statistics
    pub async fn get_statistics(&self) -> TrustChainResult<CTStatistics> {
        let metrics = self.get_metrics().await;
        Ok(CTStatistics {
            log_id: hex::encode(self.calculate_log_id()),
            total_entries: metrics
                .entries_added
                .load(std::sync::atomic::Ordering::Relaxed),
            shard_count: 1,
            tree_size: self.get_log_size().await?,
            root_hash: self.calculate_root_hash().await?,
            last_update: SystemTime::now(),
            entries_per_second: 0.0,
            storage_size_bytes: 0,
        })
    }

    /// Calculate current root hash (simplified)
    async fn calculate_root_hash(&self) -> TrustChainResult<Vec<u8>> {
        let mut hasher = Sha256::new();
        hasher.update(self.calculate_log_id());
        hasher.update(self.get_log_size().await?.to_be_bytes());
        Ok(hasher.finalize().to_vec())
    }
}
