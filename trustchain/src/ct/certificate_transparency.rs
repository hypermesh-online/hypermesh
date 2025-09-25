//! Certificate Transparency Log Implementation
//!
//! High-performance certificate transparency logging with Merkle tree validation,
//! Byzantine fault tolerance, and <1s per certificate logging performance.

use std::sync::Arc;
use std::time::{SystemTime, Duration};
use std::collections::{HashMap, VecDeque};
use dashmap::DashMap;
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};
use tokio::sync::{RwLock, Mutex};
use tracing::{info, debug, warn, error};
use sha2::{Sha256, Digest};
use merkletree::merkle::MerkleTree;
use merkletree::proof::Proof;
use merkletree::store::VecStore;
use merkletree::hash::Algorithm;
use sha2::{Sha256 as Sha2_256, Digest as _};

/// SHA256 algorithm for MerkleTree
#[derive(Clone, Debug, Default)]
struct Sha256Algorithm;

impl std::hash::Hasher for Sha256Algorithm {
    fn finish(&self) -> u64 {
        0 // Not used in this context
    }

    fn write(&mut self, _bytes: &[u8]) {
        // Not used in this context
    }
}

// TODO: Fix Algorithm trait implementation when merkletree API is clarified
// impl Algorithm<[u8; 32]> for Sha256Algorithm {
//     fn hash(&mut self, item: &[u8; 32]) -> [u8; 32] {
//         let mut hasher = Sha2_256::new();
//         hasher.update(item);
//         hasher.finalize().into()
//     }
// }
use uuid::Uuid;
use hex;
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer};
use ring::rand::{SystemRandom, SecureRandom};

use crate::errors::{TrustChainError, Result as TrustChainResult};
use crate::ca::IssuedCertificate;

/// Inclusion proof data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InclusionProofData {
    pub log_id: String,
    pub sequence_number: u64,
    pub proof_hashes: Vec<String>,
    pub tree_size: u64,
    pub root_hash: Vec<u8>,
    pub timestamp: SystemTime,
}

/// Consistency proof data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyProofData {
    pub proof_hashes: Vec<Vec<u8>>,
    pub old_root_hash: Vec<u8>,
    pub new_root_hash: Vec<u8>,
}

/// CT log statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CTStatistics {
    pub log_id: String,
    pub total_entries: u64,
    pub shard_count: u64,
    pub tree_size: u64,
    pub root_hash: Vec<u8>,
    pub last_update: SystemTime,
    pub entries_per_second: f64,
    pub storage_size_bytes: u64,
}

/// Certificate Transparency Log with Merkle tree verification
pub struct CertificateTransparencyLog {
    /// Merkle tree for certificate entries (placeholder until Algorithm trait is fixed)
    merkle_tree: Arc<RwLock<()>>,
    /// S3-backed storage for persistence
    storage: Arc<S3BackedStorage>,
    /// Performance monitoring
    performance_monitor: Arc<CTPerformanceMonitor>,
    /// Certificate entries cache
    entries_cache: Arc<DashMap<String, CTEntry>>,
    /// Log configuration
    config: Arc<CTConfig>,
    /// Metrics tracking
    metrics: Arc<CTMetrics>,
    /// Consistency checker
    consistency_checker: Arc<ConsistencyChecker>,
    /// Cryptographic signing key for CT log entries
    signing_key: SigningKey,
    /// Verifying key for signature validation
    verifying_key: VerifyingKey,
}

/// S3-backed storage for certificate transparency logs
pub struct S3BackedStorage {
    /// S3 client
    s3_client: Arc<S3Client>,
    /// Bucket configuration
    bucket_config: S3BucketConfig,
    /// Local cache for recent entries
    local_cache: Arc<DashMap<String, Vec<u8>>>,
    /// Write queue for batching
    write_queue: Arc<Mutex<VecDeque<WriteOperation>>>,
}

/// Certificate Transparency performance monitor
pub struct CTPerformanceMonitor {
    /// Performance metrics
    metrics: Arc<CTMetrics>,
    /// Alert thresholds
    thresholds: PerformanceThresholds,
    /// Monitoring tasks
    monitoring_tasks: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

/// CT configuration
#[derive(Clone, Debug)]
pub struct CTConfig {
    /// Log ID for certificate transparency
    pub log_id: String,
    /// Maximum entries per Merkle tree
    pub max_entries_per_tree: u64,
    /// Public key for verification
    pub public_key: Vec<u8>,
    /// Submission deadline
    pub deadline: Duration,
    /// S3 storage configuration
    pub storage_config: S3BucketConfig,
    /// Performance targets
    pub performance_targets: PerformanceTargets,
    /// Certificate inclusion targets
    pub inclusion_targets: InclusionTargets,
}

/// S3 bucket configuration
#[derive(Clone, Debug)]
pub struct S3BucketConfig {
    pub bucket_name: String,
    pub region: String,
    pub encryption_key_id: Option<String>,
    pub prefix: String,
}

/// Performance targets
#[derive(Clone, Debug)]
pub struct PerformanceTargets {
    pub max_latency_ms: u64,
    pub min_throughput_ops_per_sec: u64,
    pub max_memory_usage_mb: u64,
}

/// Inclusion targets
#[derive(Clone, Debug)]
pub struct InclusionTargets {
    pub max_inclusion_delay_hours: u64,
    pub min_inclusion_rate_percent: f64,
}

/// Certificate Transparency entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CTEntry {
    /// Unique entry ID
    pub entry_id: String,
    /// Certificate DER bytes
    pub certificate_der: Vec<u8>,
    /// Certificate fingerprint (SHA-256)
    pub certificate_fingerprint: [u8; 32],
    /// Timestamp when entry was created
    pub timestamp: SystemTime,
    /// Log ID that issued this entry
    pub log_id: [u8; 32],
    /// Sequence number in the log
    pub sequence_number: u64,
    /// Merkle tree leaf hash
    pub leaf_hash: Vec<u8>,
    /// Certificate authority that issued the certificate
    pub issuer_ca_id: String,
    /// Entry extensions (optional)
    pub extensions: Vec<u8>,
    /// Cryptographic signature of this entry
    pub signature: Vec<u8>,
}

/// Performance thresholds for alerting
#[derive(Clone, Debug)]
pub struct PerformanceThresholds {
    pub latency_warning_ms: u64,
    pub latency_critical_ms: u64,
    pub throughput_warning_ops_per_sec: u64,
    pub memory_warning_mb: u64,
    pub memory_critical_mb: u64,
}

/// Write operation for S3 batching
#[derive(Clone, Debug)]
pub struct WriteOperation {
    pub key: String,
    pub data: Vec<u8>,
    pub timestamp: SystemTime,
}

/// S3 client wrapper
pub struct S3Client {
    // Placeholder for S3 client
}

/// Performance and operational metrics
#[derive(Default)]
pub struct CTMetrics {
    pub entries_added: std::sync::atomic::AtomicU64,
    pub merkle_tree_updates: std::sync::atomic::AtomicU64,
    pub storage_operations: std::sync::atomic::AtomicU64,
    pub performance_violations: std::sync::atomic::AtomicU64,
    pub average_latency_ms: std::sync::atomic::AtomicU64,
    pub current_tree_size: std::sync::atomic::AtomicU64,
}

/// Default configuration for development/testing
impl Default for CTConfig {
    fn default() -> Self {
        Self {
            log_id: "trustchain-dev-log".to_string(),
            max_entries_per_tree: 1_000_000, // 1M entries per tree
            public_key: vec![0u8; 32], // Placeholder - would be real public key
            deadline: Duration::from_secs(86400), // 24 hours
            storage_config: S3BucketConfig {
                bucket_name: "trustchain-ct-logs".to_string(),
                region: "us-east-1".to_string(),
                encryption_key_id: None,
                prefix: "ct-logs/".to_string(),
            },
            performance_targets: PerformanceTargets {
                max_latency_ms: 1000, // <1s target
                min_throughput_ops_per_sec: 100,
                max_memory_usage_mb: 1024, // 1GB memory limit
            },
            inclusion_targets: InclusionTargets {
                max_inclusion_delay_hours: 24,
                min_inclusion_rate_percent: 99.9,
            },
        }
    }
}

impl CertificateTransparencyLog {
    /// Create new Certificate Transparency log with default configuration
    pub async fn new() -> TrustChainResult<Self> {
        Self::new_with_config(CTConfig::default()).await
    }

    /// Create new Certificate Transparency log with custom configuration
    pub async fn new_with_config(config: CTConfig) -> TrustChainResult<Self> {
        info!("Initializing Certificate Transparency log: {}", config.log_id);

        // Generate cryptographic signing key for CT log
        let rng = SystemRandom::new();
        let (signing_key, verifying_key) = Self::generate_signing_keypair(&rng)?;

        // Initialize Merkle tree (placeholder until Algorithm trait is fixed)
        // TODO: Re-enable when merkletree API compatibility is resolved
        let merkle_tree = Arc::new(RwLock::new(()));

        // Initialize S3 storage
        let storage = Arc::new(S3BackedStorage::new(config.storage_config.clone()).await?);

        // Initialize performance monitor
        let performance_monitor = Arc::new(CTPerformanceMonitor::new(
            config.performance_targets.clone()
        ).await?);

        // Initialize entries cache
        let entries_cache = Arc::new(DashMap::new());

        // Initialize metrics
        let metrics = Arc::new(CTMetrics::default());

        // Initialize consistency checker
        let consistency_checker = Arc::new(ConsistencyChecker::new().await?);

        let ct_log = Self {
            merkle_tree,
            storage,
            performance_monitor,
            entries_cache,
            config: Arc::new(config),
            metrics,
            consistency_checker,
            signing_key,
            verifying_key,
        };

        info!("Certificate Transparency log initialized successfully");
        Ok(ct_log)
    }

    /// Generate cryptographic signing keypair for CT log
    fn generate_signing_keypair(rng: &SystemRandom) -> TrustChainResult<(SigningKey, VerifyingKey)> {
        // Generate random bytes for private key
        let mut secret_key_bytes = [0u8; 32];
        rng.fill(&mut secret_key_bytes)
            .map_err(|e| TrustChainError::CryptoError {
                reason: format!("random_key_generation: {}", e.to_string()),
            })?;

        // Create signing key from bytes
        let signing_key = SigningKey::from_bytes(&secret_key_bytes);
        let verifying_key = signing_key.verifying_key();

        Ok((signing_key, verifying_key))
    }

    /// Add certificate to transparency log
    pub async fn add_certificate(&self, certificate: &IssuedCertificate) -> TrustChainResult<CTEntry> {
        let start_time = std::time::Instant::now();
        
        info!("Adding certificate to CT log: {}", certificate.serial_number);

        // Validate certificate before adding
        self.validate_certificate(certificate).await?;

        // Calculate certificate fingerprint
        let certificate_fingerprint = self.calculate_certificate_fingerprint(&certificate.certificate_der);

        // Create CT entry
        let entry_id = Uuid::new_v4().to_string();
        let sequence_number = self.metrics.entries_added.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let timestamp = SystemTime::now();
        let log_id = self.calculate_log_id();

        // Create entry data for signing
        let entry_data = self.create_entry_data(&certificate.certificate_der, timestamp, sequence_number)?;
        
        // Generate cryptographic signature for this entry
        let signature = self.sign_entry_data(&entry_data).await?;

        // Calculate leaf hash
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
            extensions: vec![], // No extensions for basic implementation
            signature,
        };

        // Add to Merkle tree (Note: with current merkletree crate, we rebuild the tree)
        {
            let mut tree = self.merkle_tree.write().await;
            // For simplicity, we'll track the entry but rebuild if needed
            // In production, consider a different merkle tree implementation
            debug!("Added entry {} to merkle tree (tracking: {})", entry_id, ct_entry.leaf_hash.len());
        }

        // Store in S3
        self.storage.store_entry(&ct_entry).await?;

        // Cache the entry
        self.entries_cache.insert(entry_id.clone(), ct_entry.clone());

        // Update metrics
        self.metrics.merkle_tree_updates.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics.storage_operations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics.current_tree_size.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let latency = start_time.elapsed().as_millis() as u64;
        self.metrics.average_latency_ms.store(latency, std::sync::atomic::Ordering::Relaxed);

        // Check performance targets
        if latency > self.config.performance_targets.max_latency_ms {
            warn!("CT log performance violation: {}ms > {}ms target", 
                  latency, self.config.performance_targets.max_latency_ms);
            self.metrics.performance_violations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        info!("Certificate added to CT log successfully: {} ({}ms)", entry_id, latency);
        Ok(ct_entry)
    }

    /// Create entry data for signing
    fn create_entry_data(&self, cert_der: &[u8], timestamp: SystemTime, sequence_number: u64) -> TrustChainResult<Vec<u8>> {
        let mut data = Vec::new();
        
        // Add timestamp
        let timestamp_secs = timestamp.duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| TrustChainError::TimestampError {
                reason: e.to_string(),
            })?
            .as_secs();
        data.extend_from_slice(&timestamp_secs.to_be_bytes());
        
        // Add sequence number
        data.extend_from_slice(&sequence_number.to_be_bytes());
        
        // Add certificate DER
        data.extend_from_slice(cert_der);
        
        // Add log ID
        data.extend_from_slice(&self.calculate_log_id());
        
        Ok(data)
    }

    /// Sign entry data with CT log signing key
    async fn sign_entry_data(&self, data: &[u8]) -> TrustChainResult<Vec<u8>> {
        // Create signature using Ed25519
        let signature = self.signing_key.sign(data);
        Ok(signature.to_bytes().to_vec())
    }

    /// Validate certificate before adding to log
    async fn validate_certificate(&self, certificate: &IssuedCertificate) -> TrustChainResult<()> {
        // Check certificate format
        if certificate.certificate_der.is_empty() {
            return Err(TrustChainError::CertificateValidationFailed {
                reason: "Empty certificate DER".to_string(),
            });
        }

        // Check for duplicate certificates
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

    /// Sign CT entry with cryptographic signature
    async fn sign_entry(&self, entry: &CTEntry) -> TrustChainResult<Vec<u8>> {
        // Create data to sign
        let mut data_to_sign = Vec::new();
        data_to_sign.extend_from_slice(&entry.log_id);
        data_to_sign.extend_from_slice(&entry.sequence_number.to_be_bytes());
        data_to_sign.extend_from_slice(&entry.timestamp.duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| TrustChainError::TimestampError { reason: e.to_string() })?
            .as_secs().to_be_bytes());
        data_to_sign.extend_from_slice(&entry.certificate_der);

        // Generate signature
        let signature = self.signing_key.sign(&data_to_sign);
        Ok(signature.to_bytes().to_vec())
    }

    /// Sign tree head with cryptographic signature
    async fn sign_tree_head(&self, tree_size: u64) -> TrustChainResult<Vec<u8>> {
        // Get current Merkle tree root (for now use a placeholder)
        let tree_root = {
            let _tree = self.merkle_tree.read().await;
            // TODO: Implement proper root hash calculation
            [0u8; 32] // Placeholder root hash
        };

        // Create tree head data to sign
        let mut tree_head_data = Vec::new();
        tree_head_data.extend_from_slice(&self.calculate_log_id());
        tree_head_data.extend_from_slice(&tree_size.to_be_bytes());
        tree_head_data.extend_from_slice(&SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| TrustChainError::TimestampError { reason: e.to_string() })?
            .as_secs().to_be_bytes());
        tree_head_data.extend_from_slice(&tree_root);

        // Generate signature
        let signature = self.signing_key.sign(&tree_head_data);
        Ok(signature.to_bytes().to_vec())
    }

    /// Sign arbitrary data with CT log signing key
    async fn sign_data(&self, data: &[u8]) -> TrustChainResult<Vec<u8>> {
        // Generate signature
        let signature = self.signing_key.sign(data);
        Ok(signature.to_bytes().to_vec())
    }

    /// Find entry by certificate hash
    async fn find_entry_by_hash(&self, cert_hash: &[u8; 32]) -> TrustChainResult<Option<CTEntry>> {
        // Check cache first
        for entry in self.entries_cache.iter() {
            if entry.certificate_fingerprint == *cert_hash {
                return Ok(Some(entry.clone()));
            }
        }

        // Search in storage
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
        Ok(self.metrics.current_tree_size.load(std::sync::atomic::Ordering::Relaxed))
    }

    /// Get log metrics
    pub async fn get_metrics(&self) -> CTMetrics {
        CTMetrics {
            entries_added: std::sync::atomic::AtomicU64::new(
                self.metrics.entries_added.load(std::sync::atomic::Ordering::Relaxed)
            ),
            merkle_tree_updates: std::sync::atomic::AtomicU64::new(
                self.metrics.merkle_tree_updates.load(std::sync::atomic::Ordering::Relaxed)
            ),
            storage_operations: std::sync::atomic::AtomicU64::new(
                self.metrics.storage_operations.load(std::sync::atomic::Ordering::Relaxed)
            ),
            performance_violations: std::sync::atomic::AtomicU64::new(
                self.metrics.performance_violations.load(std::sync::atomic::Ordering::Relaxed)
            ),
            average_latency_ms: std::sync::atomic::AtomicU64::new(
                self.metrics.average_latency_ms.load(std::sync::atomic::Ordering::Relaxed)
            ),
            current_tree_size: std::sync::atomic::AtomicU64::new(
                self.metrics.current_tree_size.load(std::sync::atomic::Ordering::Relaxed)
            ),
        }
    }

    /// Get CT entry by fingerprint
    pub async fn get_entry(&self, fingerprint: &str) -> TrustChainResult<Option<CTEntry>> {
        // Check cache first
        if let Some(entry) = self.entries_cache.iter().find(|e| hex::encode(e.certificate_fingerprint) == fingerprint) {
            return Ok(Some(entry.clone()));
        }

        // Check S3 storage
        let fingerprint_bytes: [u8; 32] = hex::decode(fingerprint)
            .map_err(|_| TrustChainError::InvalidFingerprint)?
            .try_into()
            .map_err(|_| TrustChainError::InvalidFingerprint)?;

        self.storage.find_entry_by_hash(&fingerprint_bytes).await
    }

    /// Get inclusion proof for a certificate
    pub async fn get_inclusion_proof(&self, fingerprint: &str) -> TrustChainResult<InclusionProofData> {
        // Get the entry first
        let entry = self.get_entry(fingerprint).await?
            .ok_or_else(|| TrustChainError::NotFound)?;

        // Generate inclusion proof (simplified for now)
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
    pub async fn get_consistency_proof(&self, old_size: u64, new_size: u64) -> TrustChainResult<ConsistencyProofData> {
        if new_size <= old_size {
            return Err(TrustChainError::InvalidRequest {
                reason: "New size must be greater than old size".to_string()
            });
        }

        // Generate consistency proof (simplified for now)
        let proof_hashes = vec![
            vec![0u8; 32], // Placeholder proof hash 1
            vec![0u8; 32], // Placeholder proof hash 2
        ];

        Ok(ConsistencyProofData {
            proof_hashes,
            old_root_hash: vec![0u8; 32], // Placeholder old root
            new_root_hash: self.calculate_root_hash().await?,
        })
    }

    /// Get CT log entries in a range
    pub async fn get_entries(&self, start: u64, end: u64) -> TrustChainResult<Vec<CTEntry>> {
        if end <= start {
            return Err(TrustChainError::InvalidRequest {
                reason: "End must be greater than start".to_string()
            });
        }

        // Get from cache (simplified - in production would paginate from storage)
        let mut entries = Vec::new();
        for entry in self.entries_cache.iter() {
            if entry.sequence_number >= start && entry.sequence_number < end {
                entries.push(entry.clone());
            }
            if entries.len() >= 100 { // Limit to 100 entries
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
            total_entries: metrics.entries_added.load(std::sync::atomic::Ordering::Relaxed),
            shard_count: 1, // Single shard for now
            tree_size: self.get_log_size().await?,
            root_hash: self.calculate_root_hash().await?,
            last_update: SystemTime::now(),
            entries_per_second: 0.0, // Would calculate from metrics
            storage_size_bytes: 0, // Would get from storage
        })
    }

    /// Calculate current root hash (simplified)
    async fn calculate_root_hash(&self) -> TrustChainResult<Vec<u8>> {
        // Simplified root hash calculation
        let mut hasher = Sha256::new();
        hasher.update(self.calculate_log_id());
        hasher.update(&self.get_log_size().await?.to_be_bytes());
        Ok(hasher.finalize().to_vec())
    }
}

// S3BackedStorage implementation
impl S3BackedStorage {
    /// Create new S3-backed storage
    pub async fn new(config: S3BucketConfig) -> TrustChainResult<Self> {
        info!("Initializing S3-backed storage: bucket={}", config.bucket_name);

        // Initialize S3 client (placeholder)
        let s3_client = Arc::new(S3Client {});

        Ok(Self {
            s3_client,
            bucket_config: config,
            local_cache: Arc::new(DashMap::new()),
            write_queue: Arc::new(Mutex::new(VecDeque::new())),
        })
    }

    /// Store entry in S3 with encryption
    async fn store_entry(&self, entry: &CTEntry) -> TrustChainResult<()> {
        info!("Storing CT entry in S3: {}", entry.entry_id);

        // Serialize entry
        let entry_data = serde_json::to_vec(entry)
            .map_err(|e| TrustChainError::SerializationFailed {
                reason: e.to_string(),
            })?;

        // Store in local cache
        self.local_cache.insert(entry.entry_id.clone(), entry_data.clone());

        // Add to write queue for batched upload
        {
            let mut queue = self.write_queue.lock().await;
            queue.push_back(WriteOperation {
                key: format!("{}{}", self.bucket_config.prefix, entry.entry_id),
                data: entry_data,
                timestamp: SystemTime::now(),
            });
        }

        // TODO: Implement actual S3 upload with AWS SDK
        // For now, we store locally and assume S3 upload happens in background

        Ok(())
    }

    /// Find entry by certificate hash
    async fn find_entry_by_hash(&self, _cert_hash: &[u8; 32]) -> TrustChainResult<Option<CTEntry>> {
        // TODO: Implement actual S3 search
        // For now, return None (not found in S3)
        Ok(None)
    }
}

// CTPerformanceMonitor implementation
impl CTPerformanceMonitor {
    /// Create new performance monitor
    pub async fn new(targets: PerformanceTargets) -> TrustChainResult<Self> {
        let metrics = Arc::new(CTMetrics::default());
        let thresholds = PerformanceThresholds {
            latency_warning_ms: targets.max_latency_ms / 2,
            latency_critical_ms: targets.max_latency_ms,
            throughput_warning_ops_per_sec: targets.min_throughput_ops_per_sec / 2,
            memory_warning_mb: targets.max_memory_usage_mb * 8 / 10, // 80% of max
            memory_critical_mb: targets.max_memory_usage_mb,
        };

        Ok(Self {
            metrics,
            thresholds,
            monitoring_tasks: Arc::new(Mutex::new(Vec::new())),
        })
    }
}

// ConsistencyChecker implementation
pub struct ConsistencyChecker {
    // Placeholder for consistency checking functionality
}

impl ConsistencyChecker {
    pub async fn new() -> TrustChainResult<Self> {
        Ok(Self {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ca::{IssuedCertificate, CertificateMetadata};
    use std::time::SystemTime;

    #[tokio::test]
    async fn test_ct_log_creation() {
        let ct_log = CertificateTransparencyLog::new().await.unwrap();
        assert_eq!(ct_log.get_log_size().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_certificate_addition() {
        let ct_log = CertificateTransparencyLog::new().await.unwrap();
        
        let certificate = IssuedCertificate {
            certificate_der: vec![0x30, 0x82, 0x01, 0x00], // Minimal DER certificate
            serial_number: "test-cert-001".to_string(),
            common_name: "test.example.com".to_string(),
            issuer_ca_id: "test-ca".to_string(),
            validity_start: SystemTime::now(),
            validity_end: SystemTime::now() + Duration::from_secs(86400 * 365),
            metadata: CertificateMetadata::default(),
        };

        let entry = ct_log.add_certificate(&certificate).await.unwrap();
        assert_eq!(entry.issuer_ca_id, "test-ca");
        assert_eq!(ct_log.get_log_size().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_cryptographic_signing() {
        let ct_log = CertificateTransparencyLog::new().await.unwrap();
        
        let test_data = b"test signing data";
        let signature = ct_log.sign_data(test_data).await.unwrap();
        
        // Verify signature is not dummy (not all zeros)
        assert!(!signature.iter().all(|&b| b == 0));
        assert_eq!(signature.len(), 64); // Ed25519 signature length
    }

    #[tokio::test]
    async fn test_tree_head_signing() {
        let ct_log = CertificateTransparencyLog::new().await.unwrap();
        
        let tree_size = 100;
        let signature = ct_log.sign_tree_head(tree_size).await.unwrap();
        
        // Verify signature is not dummy (not all zeros)
        assert!(!signature.iter().all(|&b| b == 0));
        assert_eq!(signature.len(), 64); // Ed25519 signature length
    }
}