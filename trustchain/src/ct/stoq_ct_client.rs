//! Certificate Transparency STOQ Client Integration
//!
//! Production-ready CT log operations using STOQ transport for
//! certificate logging, verification, and retrieval through TrustChain CT.

use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::net::Ipv6Addr;
use std::collections::HashMap;
use dashmap::DashMap;
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};
use tokio::sync::{RwLock, Mutex};
use tracing::{info, debug, warn, error};
use bytes::Bytes;

use crate::errors::{TrustChainError, Result as TrustChainResult};
use crate::stoq_client::{
    TrustChainStoqClient, CtLogSubmission,
    ServiceEndpoint, ServiceType
};

/// CT log operations via STOQ transport
pub struct CtStoqClient {
    /// STOQ client for transport
    stoq_client: Arc<TrustChainStoqClient>,
    /// CT log cache for performance
    ct_cache: Arc<DashMap<String, CachedCtEntry>>,
    /// CT client configuration
    config: CtStoqConfig,
    /// Performance metrics
    metrics: Arc<CtStoqMetrics>,
    /// Available CT log endpoints
    ct_endpoints: Arc<RwLock<Vec<ServiceEndpoint>>>,
}

/// Configuration for CT STOQ client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CtStoqConfig {
    /// CT log submission timeout
    pub submission_timeout: Duration,
    /// CT log query timeout
    pub query_timeout: Duration,
    /// CT entry cache TTL
    pub cache_ttl: Duration,
    /// Maximum cache entries
    pub max_cache_entries: usize,
    /// Enable automatic CT logging
    pub enable_auto_logging: bool,
    /// CT log endpoints
    pub ct_endpoints: Vec<ServiceEndpoint>,
    /// Maximum retries for CT operations
    pub max_retries: u32,
    /// Retry delay
    pub retry_delay: Duration,
}

/// Cached CT entry
#[derive(Debug, Clone)]
struct CachedCtEntry {
    entry: StoqCtEntry,
    cached_at: SystemTime,
    expires_at: SystemTime,
    fingerprint: String,
}

/// Performance metrics for CT STOQ operations
#[derive(Debug, Default)]
pub struct CtStoqMetrics {
    /// CT log submissions processed
    pub log_submissions: std::sync::atomic::AtomicU64,
    /// CT log queries performed
    pub log_queries: std::sync::atomic::AtomicU64,
    /// Merkle proof verifications
    pub proof_verifications: std::sync::atomic::AtomicU64,
    /// SCTs generated
    pub scts_generated: std::sync::atomic::AtomicU64,
    /// Cache hits
    pub cache_hits: std::sync::atomic::AtomicU64,
    /// Cache misses
    pub cache_misses: std::sync::atomic::AtomicU64,
    /// Failed operations
    pub failed_operations: std::sync::atomic::AtomicU64,
    /// Average operation latency (microseconds)
    pub avg_latency_us: std::sync::atomic::AtomicU64,
}

/// CT log submission request via STOQ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqCtSubmission {
    /// Certificate to log (DER encoded)
    pub certificate: Bytes,
    /// Certificate chain (DER encoded)
    pub chain: Vec<Bytes>,
    /// Log ID to submit to
    pub log_id: String,
    /// Submission timestamp
    pub timestamp: SystemTime,
    /// Pre-certificate flag
    pub is_precertificate: bool,
    /// Issuer key hash (for pre-certificates)
    pub issuer_key_hash: Option<Bytes>,
    /// Extensions (optional)
    pub extensions: Option<Bytes>,
}

/// CT log submission response via STOQ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqCtSubmissionResponse {
    /// Signed Certificate Timestamp
    pub sct: StoqSct,
    /// CT log entry index
    pub entry_index: u64,
    /// Submission timestamp
    pub submitted_at: SystemTime,
    /// Log ID that accepted the submission
    pub log_id: String,
}

/// Signed Certificate Timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqSct {
    /// SCT version
    pub version: u8,
    /// Log ID
    pub log_id: Bytes,
    /// Timestamp
    pub timestamp: u64,
    /// Extensions
    pub extensions: Bytes,
    /// Signature
    pub signature: Bytes,
}

/// CT log query request via STOQ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqCtQuery {
    /// Query type
    pub query_type: CtQueryType,
    /// Certificate fingerprint (for certificate lookup)
    pub certificate_fingerprint: Option<String>,
    /// Entry index (for entry retrieval)
    pub entry_index: Option<u64>,
    /// Start index (for range queries)
    pub start_index: Option<u64>,
    /// End index (for range queries)
    pub end_index: Option<u64>,
    /// Log ID to query
    pub log_id: String,
}

/// CT query types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CtQueryType {
    /// Get specific log entry by index
    GetEntry,
    /// Get entries in range
    GetEntries,
    /// Get proof by hash
    GetProofByHash,
    /// Get consistency proof
    GetConsistencyProof,
    /// Get STH (Signed Tree Head)
    GetSTH,
    /// Search by certificate fingerprint
    SearchByCertificate,
}

/// CT log query response via STOQ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqCtQueryResponse {
    /// Query result type
    pub result_type: CtQueryType,
    /// CT log entries (for entry queries)
    pub entries: Vec<StoqCtEntry>,
    /// Merkle proof (for proof queries)
    pub proof: Option<StoqMerkleProof>,
    /// Signed Tree Head (for STH queries)
    pub sth: Option<StoqSignedTreeHead>,
    /// Query timestamp
    pub queried_at: SystemTime,
}

/// CT log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqCtEntry {
    /// Entry index in log
    pub index: u64,
    /// Certificate (DER encoded)
    pub certificate: Bytes,
    /// Certificate chain (DER encoded)
    pub chain: Vec<Bytes>,
    /// Entry timestamp
    pub timestamp: u64,
    /// Extensions
    pub extensions: Bytes,
    /// Entry type
    pub entry_type: CtEntryType,
    /// Log ID
    pub log_id: String,
}

/// CT entry types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CtEntryType {
    /// X.509 certificate
    X509Certificate,
    /// Pre-certificate
    PreCertificate,
}

/// Merkle proof for CT entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqMerkleProof {
    /// Leaf index
    pub leaf_index: u64,
    /// Tree size
    pub tree_size: u64,
    /// Audit path
    pub audit_path: Vec<Bytes>,
}

/// Signed Tree Head
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqSignedTreeHead {
    /// Tree size
    pub tree_size: u64,
    /// Root hash
    pub root_hash: Bytes,
    /// Timestamp
    pub timestamp: u64,
    /// Signature
    pub signature: Bytes,
    /// Log ID
    pub log_id: String,
}

impl Default for CtStoqConfig {
    fn default() -> Self {
        Self {
            submission_timeout: Duration::from_secs(30),
            query_timeout: Duration::from_secs(10),
            cache_ttl: Duration::from_secs(1800), // 30 minutes
            max_cache_entries: 500,
            enable_auto_logging: true,
            ct_endpoints: vec![
                ServiceEndpoint::new(
                    ServiceType::CertificateTransparency,
                    Ipv6Addr::LOCALHOST,
                    6962
                ).with_service_name("ct.trustchain.local".to_string()),
            ],
            max_retries: 3,
            retry_delay: Duration::from_millis(1000),
        }
    }
}

impl CtStoqClient {
    /// Create new CT STOQ client
    pub async fn new(
        stoq_client: Arc<TrustChainStoqClient>,
        config: CtStoqConfig,
    ) -> TrustChainResult<Self> {
        info!("Initializing CT STOQ client with {} endpoints", config.ct_endpoints.len());

        let client = Self {
            stoq_client,
            ct_cache: Arc::new(DashMap::new()),
            config: config.clone(),
            metrics: Arc::new(CtStoqMetrics::default()),
            ct_endpoints: Arc::new(RwLock::new(config.ct_endpoints)),
        };

        info!("CT STOQ client initialized successfully");
        Ok(client)
    }

    /// Submit certificate to CT log via STOQ
    pub async fn submit_to_ct_log(&self, submission: StoqCtSubmission) -> TrustChainResult<StoqCtSubmissionResponse> {
        let start_time = std::time::Instant::now();
        
        let fingerprint = hex::encode(sha2::Sha256::digest(&submission.certificate));
        debug!("Submitting certificate to CT log via STOQ: {} (log: {})", fingerprint, submission.log_id);

        // Check cache first
        if let Some(cached_entry) = self.check_ct_cache(&fingerprint).await {
            debug!("Certificate already in CT log cache: {}", fingerprint);
            self.metrics.cache_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            
            // Convert cached entry to submission response
            return Ok(StoqCtSubmissionResponse {
                sct: StoqSct {
                    version: 1,
                    log_id: submission.log_id.clone().into(),
                    timestamp: cached_entry.entry.timestamp,
                    extensions: cached_entry.entry.extensions.clone(),
                    signature: Bytes::new(), // Would be actual signature
                },
                entry_index: cached_entry.entry.index,
                submitted_at: cached_entry.cached_at,
                log_id: submission.log_id,
            });
        }

        self.metrics.cache_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Perform submission with retries
        let mut last_error = None;
        
        for attempt in 0..=self.config.max_retries {
            match self.submit_with_stoq(&submission, attempt).await {
                Ok(response) => {
                    // Cache the result
                    self.cache_ct_submission(&submission, &response).await;
                    
                    // Update metrics
                    let latency = start_time.elapsed().as_micros() as u64;
                    self.metrics.log_submissions.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    self.metrics.scts_generated.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    self.update_avg_latency(latency);
                    
                    info!("Certificate submitted to CT log successfully: {} ({}μs)", fingerprint, latency);
                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.max_retries {
                        warn!("CT submission attempt {} failed for {}, retrying: {}", 
                              attempt + 1, fingerprint, last_error.as_ref().unwrap());
                        tokio::time::sleep(self.config.retry_delay).await;
                    }
                }
            }
        }

        // All retries failed
        self.metrics.failed_operations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        error!("CT submission failed after {} attempts: {}", 
               self.config.max_retries + 1, fingerprint);
        
        Err(last_error.unwrap_or_else(|| TrustChainError::NetworkError {
            operation: "ct_submission".to_string(),
            reason: "All retry attempts failed".to_string(),
        }))
    }

    /// Query CT log via STOQ
    pub async fn query_ct_log(&self, query: StoqCtQuery) -> TrustChainResult<StoqCtQueryResponse> {
        let start_time = std::time::Instant::now();
        
        debug!("Querying CT log via STOQ: {:?} (log: {})", query.query_type, query.log_id);

        // Select CT endpoint
        let ct_endpoint = self.select_ct_endpoint().await?;

        // Serialize query
        let query_data = bincode::serialize(&query)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "ct_query_serialize".to_string(),
                reason: e.to_string(),
            })?;

        // Send query via STOQ
        let response_data = self.send_ct_request(&ct_endpoint, &query_data).await?;

        // Deserialize response
        let response: StoqCtQueryResponse = bincode::deserialize(&response_data)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "ct_query_deserialize".to_string(),
                reason: e.to_string(),
            })?;

        // Update metrics
        let latency = start_time.elapsed().as_micros() as u64;
        self.metrics.log_queries.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.update_avg_latency(latency);

        debug!("CT query completed successfully: {:?} ({}μs)", query.query_type, latency);
        Ok(response)
    }

    /// Verify Merkle proof via STOQ
    pub async fn verify_merkle_proof(
        &self,
        proof: &StoqMerkleProof,
        leaf_hash: &[u8],
        tree_head: &StoqSignedTreeHead,
    ) -> TrustChainResult<bool> {
        let start_time = std::time::Instant::now();
        
        debug!("Verifying Merkle proof via STOQ: leaf_index={}, tree_size={}", 
               proof.leaf_index, proof.tree_size);

        // Create verification request
        let verification_request = StoqMerkleVerification {
            proof: proof.clone(),
            leaf_hash: Bytes::copy_from_slice(leaf_hash),
            tree_head: tree_head.clone(),
        };

        // Select CT endpoint
        let ct_endpoint = self.select_ct_endpoint().await?;

        // Serialize verification request
        let request_data = bincode::serialize(&verification_request)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "merkle_verification_serialize".to_string(),
                reason: e.to_string(),
            })?;

        // Send verification request via STOQ
        let response_data = self.send_ct_request(&ct_endpoint, &request_data).await?;

        // Deserialize response
        let is_valid: bool = bincode::deserialize(&response_data)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "merkle_verification_deserialize".to_string(),
                reason: e.to_string(),
            })?;

        // Update metrics
        let latency = start_time.elapsed().as_micros() as u64;
        self.metrics.proof_verifications.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.update_avg_latency(latency);

        debug!("Merkle proof verification completed: {} ({}μs)", is_valid, latency);
        Ok(is_valid)
    }

    /// Submit with retry logic
    async fn submit_with_stoq(&self, submission: &StoqCtSubmission, attempt: u32) -> TrustChainResult<StoqCtSubmissionResponse> {
        // Select CT endpoint (round-robin based on attempt)
        let endpoints = self.ct_endpoints.read().await;
        if endpoints.is_empty() {
            return Err(TrustChainError::ServiceDiscoveryError {
                service: "certificate_transparency".to_string(),
                reason: "No CT endpoints configured".to_string(),
            });
        }
        
        let endpoint_index = attempt as usize % endpoints.len();
        let ct_endpoint = &endpoints[endpoint_index];

        debug!("Using CT endpoint: [{}]:{} (attempt {})", 
               ct_endpoint.address, ct_endpoint.port, attempt + 1);

        // Serialize submission
        let submission_data = bincode::serialize(submission)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "ct_submission_serialize".to_string(),
                reason: e.to_string(),
            })?;

        // Send submission via STOQ
        let response_data = self.send_ct_request(ct_endpoint, &submission_data).await?;

        // Deserialize response
        let response: StoqCtSubmissionResponse = bincode::deserialize(&response_data)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "ct_submission_deserialize".to_string(),
                reason: e.to_string(),
            })?;

        Ok(response)
    }

    /// Send CT request via STOQ transport
    async fn send_ct_request(&self, endpoint: &ServiceEndpoint, data: &[u8]) -> TrustChainResult<Bytes> {
        // Create STOQ endpoint
        let stoq_endpoint = stoq::Endpoint::new(endpoint.address, endpoint.port)
            .with_server_name(endpoint.service_name.clone().unwrap_or_else(|| {
                "ct.trustchain.local".to_string()
            }));

        // Get connection
        let connection = self.stoq_client.transport().connect(&stoq_endpoint).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "ct_stoq_connection".to_string(),
                reason: e.to_string(),
            })?;

        // Send request
        self.stoq_client.transport().send(&connection, data).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "ct_request_send".to_string(),
                reason: e.to_string(),
            })?;

        // Receive response
        let response = self.stoq_client.transport().receive(&connection).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "ct_response_receive".to_string(),
                reason: e.to_string(),
            })?;

        Ok(response)
    }

    /// Select best CT endpoint
    async fn select_ct_endpoint(&self) -> TrustChainResult<ServiceEndpoint> {
        let endpoints = self.ct_endpoints.read().await;
        endpoints.first()
            .cloned()
            .ok_or_else(|| TrustChainError::ServiceDiscoveryError {
                service: "certificate_transparency".to_string(),
                reason: "No CT endpoints configured".to_string(),
            })
    }

    /// Check CT cache for existing entry
    async fn check_ct_cache(&self, fingerprint: &str) -> Option<CachedCtEntry> {
        if let Some(cached_entry) = self.ct_cache.get(fingerprint) {
            if cached_entry.expires_at > SystemTime::now() {
                return Some(cached_entry.clone());
            } else {
                self.ct_cache.remove(fingerprint);
            }
        }
        None
    }

    /// Cache CT submission result
    async fn cache_ct_submission(&self, submission: &StoqCtSubmission, response: &StoqCtSubmissionResponse) {
        let fingerprint = hex::encode(sha2::Sha256::digest(&submission.certificate));
        
        let ct_entry = StoqCtEntry {
            index: response.entry_index,
            certificate: submission.certificate.clone(),
            chain: submission.chain.clone(),
            timestamp: response.sct.timestamp,
            extensions: response.sct.extensions.clone(),
            entry_type: if submission.is_precertificate {
                CtEntryType::PreCertificate
            } else {
                CtEntryType::X509Certificate
            },
            log_id: response.log_id.clone(),
        };

        let cache_entry = CachedCtEntry {
            entry: ct_entry,
            cached_at: SystemTime::now(),
            expires_at: SystemTime::now() + self.config.cache_ttl,
            fingerprint: fingerprint.clone(),
        };

        // Ensure cache doesn't exceed max size
        if self.ct_cache.len() >= self.config.max_cache_entries {
            self.evict_oldest_cache_entry().await;
        }

        self.ct_cache.insert(fingerprint, cache_entry);
    }

    /// Evict oldest cache entry
    async fn evict_oldest_cache_entry(&self) {
        let mut oldest_key = String::new();
        let mut oldest_time = SystemTime::now();

        for entry in self.ct_cache.iter() {
            if entry.value().cached_at < oldest_time {
                oldest_time = entry.value().cached_at;
                oldest_key = entry.key().clone();
            }
        }

        if !oldest_key.is_empty() {
            self.ct_cache.remove(&oldest_key);
        }
    }

    /// Update average latency metric
    fn update_avg_latency(&self, latency_us: u64) {
        let current_avg = self.metrics.avg_latency_us.load(std::sync::atomic::Ordering::Relaxed);
        let new_avg = if current_avg == 0 {
            latency_us
        } else {
            (current_avg * 9 + latency_us) / 10 // Moving average
        };
        self.metrics.avg_latency_us.store(new_avg, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get CT client metrics
    pub fn get_metrics(&self) -> CtStoqMetrics {
        CtStoqMetrics {
            log_submissions: std::sync::atomic::AtomicU64::new(
                self.metrics.log_submissions.load(std::sync::atomic::Ordering::Relaxed)
            ),
            log_queries: std::sync::atomic::AtomicU64::new(
                self.metrics.log_queries.load(std::sync::atomic::Ordering::Relaxed)
            ),
            proof_verifications: std::sync::atomic::AtomicU64::new(
                self.metrics.proof_verifications.load(std::sync::atomic::Ordering::Relaxed)
            ),
            scts_generated: std::sync::atomic::AtomicU64::new(
                self.metrics.scts_generated.load(std::sync::atomic::Ordering::Relaxed)
            ),
            cache_hits: std::sync::atomic::AtomicU64::new(
                self.metrics.cache_hits.load(std::sync::atomic::Ordering::Relaxed)
            ),
            cache_misses: std::sync::atomic::AtomicU64::new(
                self.metrics.cache_misses.load(std::sync::atomic::Ordering::Relaxed)
            ),
            failed_operations: std::sync::atomic::AtomicU64::new(
                self.metrics.failed_operations.load(std::sync::atomic::Ordering::Relaxed)
            ),
            avg_latency_us: std::sync::atomic::AtomicU64::new(
                self.metrics.avg_latency_us.load(std::sync::atomic::Ordering::Relaxed)
            ),
        }
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> (usize, usize, f64) {
        let total_entries = self.ct_cache.len();
        let total_ops = self.metrics.log_submissions.load(std::sync::atomic::Ordering::Relaxed) +
                       self.metrics.log_queries.load(std::sync::atomic::Ordering::Relaxed);
        let cache_hits = self.metrics.cache_hits.load(std::sync::atomic::Ordering::Relaxed);
        
        let hit_ratio = if total_ops > 0 {
            cache_hits as f64 / total_ops as f64
        } else {
            0.0
        };

        (total_entries, self.config.max_cache_entries, hit_ratio)
    }

    /// Clean expired cache entries
    pub async fn cleanup_cache(&self) -> TrustChainResult<usize> {
        let now = SystemTime::now();
        let mut expired_keys = Vec::new();

        for entry in self.ct_cache.iter() {
            if entry.value().expires_at <= now {
                expired_keys.push(entry.key().clone());
            }
        }

        let count = expired_keys.len();
        for key in expired_keys {
            self.ct_cache.remove(&key);
        }

        debug!("Cleaned {} expired CT cache entries", count);
        Ok(count)
    }

    /// Add CT endpoint
    pub async fn add_ct_endpoint(&self, endpoint: ServiceEndpoint) -> TrustChainResult<()> {
        let mut endpoints = self.ct_endpoints.write().await;
        if !endpoints.contains(&endpoint) {
            endpoints.push(endpoint.clone());
            info!("Added CT endpoint: [{}]:{}", endpoint.address, endpoint.port);
        }
        Ok(())
    }

    /// Remove CT endpoint
    pub async fn remove_ct_endpoint(&self, endpoint: &ServiceEndpoint) -> TrustChainResult<()> {
        let mut endpoints = self.ct_endpoints.write().await;
        endpoints.retain(|e| e != endpoint);
        info!("Removed CT endpoint: [{}]:{}", endpoint.address, endpoint.port);
        Ok(())
    }
}

/// Merkle proof verification request
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoqMerkleVerification {
    proof: StoqMerkleProof,
    leaf_hash: Bytes,
    tree_head: StoqSignedTreeHead,
}

// Additional required imports
use sha2::Digest;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ct_stoq_config_default() {
        let config = CtStoqConfig::default();
        
        assert_eq!(config.submission_timeout, Duration::from_secs(30));
        assert_eq!(config.query_timeout, Duration::from_secs(10));
        assert_eq!(config.cache_ttl, Duration::from_secs(1800));
        assert_eq!(config.max_cache_entries, 500);
        assert!(config.enable_auto_logging);
        assert_eq!(config.ct_endpoints.len(), 1);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_ct_submission_serialization() {
        let submission = StoqCtSubmission {
            certificate: Bytes::from_static(b"test-certificate"),
            chain: vec![Bytes::from_static(b"test-chain-cert")],
            log_id: "test-log-id".to_string(),
            timestamp: SystemTime::now(),
            is_precertificate: false,
            issuer_key_hash: None,
            extensions: None,
        };

        let serialized = bincode::serialize(&submission).unwrap();
        let deserialized: StoqCtSubmission = bincode::deserialize(&serialized).unwrap();

        assert_eq!(submission.certificate, deserialized.certificate);
        assert_eq!(submission.chain, deserialized.chain);
        assert_eq!(submission.log_id, deserialized.log_id);
        assert_eq!(submission.is_precertificate, deserialized.is_precertificate);
    }

    #[tokio::test]
    async fn test_metrics_initialization() {
        let metrics = CtStoqMetrics::default();
        
        assert_eq!(metrics.log_submissions.load(std::sync::atomic::Ordering::Relaxed), 0);
        assert_eq!(metrics.log_queries.load(std::sync::atomic::Ordering::Relaxed), 0);
        assert_eq!(metrics.proof_verifications.load(std::sync::atomic::Ordering::Relaxed), 0);
        assert_eq!(metrics.scts_generated.load(std::sync::atomic::Ordering::Relaxed), 0);
    }
}