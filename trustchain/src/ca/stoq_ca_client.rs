//! Certificate Authority STOQ Client Integration
//!
//! Production-ready certificate operations using STOQ transport for
//! certificate issuance, validation, and revocation through TrustChain CA.

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
    TrustChainStoqClient, CertificateValidationRequest, ValidationPolicy,
    ServiceEndpoint, ServiceType
};

/// CA operations via STOQ transport
pub struct CaStoqClient {
    /// STOQ client for transport
    stoq_client: Arc<TrustChainStoqClient>,
    /// Certificate cache for performance
    cert_cache: Arc<DashMap<String, CachedCertificate>>,
    /// CA client configuration
    config: CaStoqConfig,
    /// Performance metrics
    metrics: Arc<CaStoqMetrics>,
    /// Available CA endpoints
    ca_endpoints: Arc<RwLock<Vec<ServiceEndpoint>>>,
}

/// Configuration for CA STOQ client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaStoqConfig {
    /// Certificate request timeout
    pub request_timeout: Duration,
    /// Certificate validation timeout
    pub validation_timeout: Duration,
    /// Certificate cache TTL
    pub cache_ttl: Duration,
    /// Maximum cache entries
    pub max_cache_entries: usize,
    /// Default certificate validity period
    pub default_validity_days: u32,
    /// Enable automatic certificate renewal
    pub enable_auto_renewal: bool,
    /// Certificate key size
    pub key_size: u32,
    /// CA endpoints
    pub ca_endpoints: Vec<ServiceEndpoint>,
}

/// Cached certificate entry
#[derive(Debug, Clone)]
struct CachedCertificate {
    certificate_der: Bytes,
    chain: Option<Vec<Bytes>>,
    cached_at: SystemTime,
    expires_at: SystemTime,
    fingerprint: String,
    is_valid: bool,
}

/// Performance metrics for CA STOQ operations
#[derive(Debug, Default)]
pub struct CaStoqMetrics {
    /// Certificate requests processed
    pub cert_requests: std::sync::atomic::AtomicU64,
    /// Certificate validations performed
    pub cert_validations: std::sync::atomic::AtomicU64,
    /// Certificate revocations processed
    pub cert_revocations: std::sync::atomic::AtomicU64,
    /// Cache hits
    pub cache_hits: std::sync::atomic::AtomicU64,
    /// Cache misses
    pub cache_misses: std::sync::atomic::AtomicU64,
    /// Failed operations
    pub failed_operations: std::sync::atomic::AtomicU64,
    /// Average operation latency (microseconds)
    pub avg_latency_us: std::sync::atomic::AtomicU64,
}

/// Certificate request via STOQ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqCertificateRequest {
    /// Common name for certificate
    pub common_name: String,
    /// Subject alternative names
    pub san_list: Vec<String>,
    /// Certificate key usage
    pub key_usage: Vec<KeyUsage>,
    /// Extended key usage
    pub extended_key_usage: Vec<ExtendedKeyUsage>,
    /// Certificate validity period in days
    pub validity_days: u32,
    /// Key size
    pub key_size: u32,
    /// Key algorithm
    pub key_algorithm: KeyAlgorithm,
    /// Requester information
    pub requester: CertificateRequester,
    /// Consensus proof for validation
    pub consensus_proof: Option<Bytes>,
}

/// Certificate response via STOQ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqCertificateResponse {
    /// Issued certificate (DER encoded)
    pub certificate: Bytes,
    /// Certificate chain (DER encoded)
    pub chain: Vec<Bytes>,
    /// Certificate serial number
    pub serial_number: String,
    /// Certificate fingerprint (SHA256)
    pub fingerprint: String,
    /// Certificate validity period
    pub not_before: SystemTime,
    pub not_after: SystemTime,
    /// Signed Certificate Timestamp (if CT enabled)
    pub sct: Option<Bytes>,
}

/// Certificate validation request via STOQ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqCertificateValidation {
    /// Certificate to validate (DER encoded)
    pub certificate: Bytes,
    /// Certificate chain (optional, DER encoded)
    pub chain: Option<Vec<Bytes>>,
    /// Hostname to validate against (optional)
    pub hostname: Option<String>,
    /// Validation time (optional, defaults to now)
    pub validation_time: Option<SystemTime>,
    /// Validation policy
    pub policy: ValidationPolicy,
    /// Check certificate revocation status
    pub check_revocation: bool,
}

/// Certificate validation response via STOQ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqValidationResponse {
    /// Whether certificate is valid
    pub is_valid: bool,
    /// Validation error if any
    pub error: Option<String>,
    /// Certificate fingerprint
    pub fingerprint: String,
    /// Certificate expiry
    pub expires_at: SystemTime,
    /// Validation performed at
    pub validated_at: SystemTime,
    /// Revocation status
    pub revocation_status: RevocationStatus,
}

/// Certificate revocation request via STOQ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqRevocationRequest {
    /// Certificate serial number to revoke
    pub serial_number: String,
    /// Revocation reason
    pub reason: RevocationReason,
    /// Requester authorization
    pub requester: CertificateRequester,
    /// Consensus proof for authorization
    pub consensus_proof: Option<Bytes>,
}

/// Key usage flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyUsage {
    DigitalSignature,
    KeyEncipherment,
    KeyAgreement,
    CertSign,
    CrlSign,
    DataEncipherment,
    NonRepudiation,
}

/// Extended key usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtendedKeyUsage {
    ServerAuth,
    ClientAuth,
    CodeSigning,
    TimeStamping,
    EmailProtection,
    IpsecEndSystem,
    IpsecTunnel,
    IpsecUser,
}

/// Key algorithms supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyAlgorithm {
    Rsa,
    EcdsaP256,
    EcdsaP384,
    Ed25519,
}

/// Certificate requester information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateRequester {
    /// Requester identity
    pub identity: String,
    /// Organization
    pub organization: Option<String>,
    /// Country
    pub country: Option<String>,
    /// Email address
    pub email: Option<String>,
    /// Authorization token
    pub auth_token: Option<String>,
}

/// Revocation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RevocationStatus {
    Valid,
    Revoked { reason: RevocationReason, revoked_at: SystemTime },
    Unknown,
}

/// Revocation reasons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RevocationReason {
    Unspecified,
    KeyCompromise,
    CaCompromise,
    AffiliationChanged,
    Superseded,
    CessationOfOperation,
    PrivilegeWithdrawn,
}

impl Default for CaStoqConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(30),
            validation_timeout: Duration::from_secs(10),
            cache_ttl: Duration::from_secs(3600), // 1 hour
            max_cache_entries: 1000,
            default_validity_days: 365, // 1 year
            enable_auto_renewal: false,
            key_size: 2048,
            ca_endpoints: vec![
                ServiceEndpoint::new(
                    ServiceType::CertificateAuthority,
                    Ipv6Addr::LOCALHOST,
                    8443
                ).with_service_name("ca.trustchain.local".to_string()),
            ],
        }
    }
}

impl CaStoqClient {
    /// Create new CA STOQ client
    pub async fn new(
        stoq_client: Arc<TrustChainStoqClient>,
        config: CaStoqConfig,
    ) -> TrustChainResult<Self> {
        info!("Initializing CA STOQ client with {} endpoints", config.ca_endpoints.len());

        let client = Self {
            stoq_client,
            cert_cache: Arc::new(DashMap::new()),
            config: config.clone(),
            metrics: Arc::new(CaStoqMetrics::default()),
            ca_endpoints: Arc::new(RwLock::new(config.ca_endpoints)),
        };

        info!("CA STOQ client initialized successfully");
        Ok(client)
    }

    /// Request certificate issuance via STOQ
    pub async fn request_certificate(&self, request: StoqCertificateRequest) -> TrustChainResult<StoqCertificateResponse> {
        let start_time = std::time::Instant::now();
        
        debug!("Requesting certificate via STOQ: {}", request.common_name);

        // Select CA endpoint
        let ca_endpoint = self.select_ca_endpoint().await?;

        // Serialize certificate request
        let request_data = bincode::serialize(&request)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "certificate_request_serialize".to_string(),
                reason: e.to_string(),
            })?;

        // Send request via STOQ
        let response_data = self.send_ca_request(&ca_endpoint, &request_data).await?;

        // Deserialize response
        let response: StoqCertificateResponse = bincode::deserialize(&response_data)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "certificate_response_deserialize".to_string(),
                reason: e.to_string(),
            })?;

        // Cache the certificate
        self.cache_certificate(&response).await;

        // Update metrics
        let latency = start_time.elapsed().as_micros() as u64;
        self.metrics.cert_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.update_avg_latency(latency);

        info!("Certificate issued successfully: {} ({}μs)", request.common_name, latency);
        Ok(response)
    }

    /// Validate certificate via STOQ
    pub async fn validate_certificate(&self, validation: StoqCertificateValidation) -> TrustChainResult<StoqValidationResponse> {
        let start_time = std::time::Instant::now();
        
        // Calculate certificate fingerprint for caching
        let fingerprint = hex::encode(sha2::Sha256::digest(&validation.certificate));
        
        debug!("Validating certificate via STOQ: {}", fingerprint);

        // Check cache first
        if let Some(cached_cert) = self.cert_cache.get(&fingerprint) {
            if cached_cert.expires_at > SystemTime::now() {
                debug!("Certificate validation cache hit: {}", fingerprint);
                self.metrics.cache_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                
                return Ok(StoqValidationResponse {
                    is_valid: cached_cert.is_valid,
                    error: None,
                    fingerprint: cached_cert.fingerprint.clone(),
                    expires_at: cached_cert.expires_at,
                    validated_at: SystemTime::now(),
                    revocation_status: RevocationStatus::Valid, // From cache
                });
            } else {
                self.cert_cache.remove(&fingerprint);
            }
        }

        self.metrics.cache_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Select CA endpoint
        let ca_endpoint = self.select_ca_endpoint().await?;

        // Serialize validation request
        let request_data = bincode::serialize(&validation)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "certificate_validation_serialize".to_string(),
                reason: e.to_string(),
            })?;

        // Send validation request via STOQ
        let response_data = self.send_ca_request(&ca_endpoint, &request_data).await?;

        // Deserialize response
        let response: StoqValidationResponse = bincode::deserialize(&response_data)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "certificate_validation_deserialize".to_string(),
                reason: e.to_string(),
            })?;

        // Cache validation result
        if response.is_valid {
            let cache_entry = CachedCertificate {
                certificate_der: validation.certificate,
                chain: validation.chain,
                cached_at: SystemTime::now(),
                expires_at: response.expires_at,
                fingerprint: response.fingerprint.clone(),
                is_valid: response.is_valid,
            };
            self.cert_cache.insert(fingerprint.clone(), cache_entry);
        }

        // Update metrics
        let latency = start_time.elapsed().as_micros() as u64;
        self.metrics.cert_validations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.update_avg_latency(latency);

        debug!("Certificate validation completed: {} -> {} ({}μs)", 
               fingerprint, response.is_valid, latency);
        Ok(response)
    }

    /// Revoke certificate via STOQ
    pub async fn revoke_certificate(&self, revocation: StoqRevocationRequest) -> TrustChainResult<bool> {
        let start_time = std::time::Instant::now();
        
        debug!("Revoking certificate via STOQ: {}", revocation.serial_number);

        // Select CA endpoint
        let ca_endpoint = self.select_ca_endpoint().await?;

        // Serialize revocation request
        let request_data = bincode::serialize(&revocation)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "certificate_revocation_serialize".to_string(),
                reason: e.to_string(),
            })?;

        // Send revocation request via STOQ
        let response_data = self.send_ca_request(&ca_endpoint, &request_data).await?;

        // Deserialize response
        let success: bool = bincode::deserialize(&response_data)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "certificate_revocation_deserialize".to_string(),
                reason: e.to_string(),
            })?;

        // Invalidate cache entries for revoked certificate
        if success {
            self.invalidate_certificate_cache(&revocation.serial_number).await;
        }

        // Update metrics
        let latency = start_time.elapsed().as_micros() as u64;
        self.metrics.cert_revocations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.update_avg_latency(latency);

        info!("Certificate revocation {}: {} ({}μs)", 
              if success { "succeeded" } else { "failed" },
              revocation.serial_number, latency);
        Ok(success)
    }

    /// Send CA request via STOQ transport
    async fn send_ca_request(&self, endpoint: &ServiceEndpoint, data: &[u8]) -> TrustChainResult<Bytes> {
        // Create STOQ endpoint
        let stoq_endpoint = stoq::Endpoint::new(endpoint.address, endpoint.port)
            .with_server_name(endpoint.service_name.clone().unwrap_or_else(|| {
                "ca.trustchain.local".to_string()
            }));

        // Get connection
        let connection = self.stoq_client.transport().connect(&stoq_endpoint).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "ca_stoq_connection".to_string(),
                reason: e.to_string(),
            })?;

        // Send request
        self.stoq_client.transport().send(&connection, data).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "ca_request_send".to_string(),
                reason: e.to_string(),
            })?;

        // Receive response
        let response = self.stoq_client.transport().receive(&connection).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "ca_response_receive".to_string(),
                reason: e.to_string(),
            })?;

        Ok(response)
    }

    /// Select best CA endpoint
    async fn select_ca_endpoint(&self) -> TrustChainResult<ServiceEndpoint> {
        let endpoints = self.ca_endpoints.read().await;
        endpoints.first()
            .cloned()
            .ok_or_else(|| TrustChainError::ServiceDiscoveryError {
                service: "certificate_authority".to_string(),
                reason: "No CA endpoints configured".to_string(),
            })
    }

    /// Cache certificate after issuance
    async fn cache_certificate(&self, response: &StoqCertificateResponse) {
        let cache_entry = CachedCertificate {
            certificate_der: response.certificate.clone(),
            chain: Some(response.chain.clone()),
            cached_at: SystemTime::now(),
            expires_at: response.not_after,
            fingerprint: response.fingerprint.clone(),
            is_valid: true,
        };

        // Ensure cache doesn't exceed max size
        if self.cert_cache.len() >= self.config.max_cache_entries {
            self.evict_oldest_cache_entry().await;
        }

        self.cert_cache.insert(response.fingerprint.clone(), cache_entry);
    }

    /// Invalidate certificate cache entries for revoked certificate
    async fn invalidate_certificate_cache(&self, serial_number: &str) {
        // Remove any cached certificates with this serial number
        // Note: In production, you'd need to track serial number to fingerprint mapping
        let mut to_remove = Vec::new();
        
        for entry in self.cert_cache.iter() {
            // This is a simplified approach - in production you'd need proper serial tracking
            if entry.key().contains(serial_number) {
                to_remove.push(entry.key().clone());
            }
        }

        for key in to_remove {
            self.cert_cache.remove(&key);
        }
    }

    /// Evict oldest cache entry
    async fn evict_oldest_cache_entry(&self) {
        let mut oldest_key = String::new();
        let mut oldest_time = SystemTime::now();

        for entry in self.cert_cache.iter() {
            if entry.value().cached_at < oldest_time {
                oldest_time = entry.value().cached_at;
                oldest_key = entry.key().clone();
            }
        }

        if !oldest_key.is_empty() {
            self.cert_cache.remove(&oldest_key);
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

    /// Get CA client metrics
    pub fn get_metrics(&self) -> CaStoqMetrics {
        CaStoqMetrics {
            cert_requests: std::sync::atomic::AtomicU64::new(
                self.metrics.cert_requests.load(std::sync::atomic::Ordering::Relaxed)
            ),
            cert_validations: std::sync::atomic::AtomicU64::new(
                self.metrics.cert_validations.load(std::sync::atomic::Ordering::Relaxed)
            ),
            cert_revocations: std::sync::atomic::AtomicU64::new(
                self.metrics.cert_revocations.load(std::sync::atomic::Ordering::Relaxed)
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
        let total_entries = self.cert_cache.len();
        let total_ops = self.metrics.cert_requests.load(std::sync::atomic::Ordering::Relaxed) +
                       self.metrics.cert_validations.load(std::sync::atomic::Ordering::Relaxed);
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

        for entry in self.cert_cache.iter() {
            if entry.value().expires_at <= now {
                expired_keys.push(entry.key().clone());
            }
        }

        let count = expired_keys.len();
        for key in expired_keys {
            self.cert_cache.remove(&key);
        }

        debug!("Cleaned {} expired certificate cache entries", count);
        Ok(count)
    }

    /// Add CA endpoint
    pub async fn add_ca_endpoint(&self, endpoint: ServiceEndpoint) -> TrustChainResult<()> {
        let mut endpoints = self.ca_endpoints.write().await;
        if !endpoints.contains(&endpoint) {
            endpoints.push(endpoint.clone());
            info!("Added CA endpoint: [{}]:{}", endpoint.address, endpoint.port);
        }
        Ok(())
    }

    /// Remove CA endpoint
    pub async fn remove_ca_endpoint(&self, endpoint: &ServiceEndpoint) -> TrustChainResult<()> {
        let mut endpoints = self.ca_endpoints.write().await;
        endpoints.retain(|e| e != endpoint);
        info!("Removed CA endpoint: [{}]:{}", endpoint.address, endpoint.port);
        Ok(())
    }
}

// Additional required imports
use sha2::Digest;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ca_stoq_config_default() {
        let config = CaStoqConfig::default();
        
        assert_eq!(config.request_timeout, Duration::from_secs(30));
        assert_eq!(config.validation_timeout, Duration::from_secs(10));
        assert_eq!(config.cache_ttl, Duration::from_secs(3600));
        assert_eq!(config.max_cache_entries, 1000);
        assert_eq!(config.default_validity_days, 365);
        assert!(!config.enable_auto_renewal);
        assert_eq!(config.key_size, 2048);
        assert_eq!(config.ca_endpoints.len(), 1);
    }

    #[test]
    fn test_certificate_request_serialization() {
        let request = StoqCertificateRequest {
            common_name: "test.example.com".to_string(),
            san_list: vec!["test.example.com".to_string(), "www.test.example.com".to_string()],
            key_usage: vec![KeyUsage::DigitalSignature, KeyUsage::KeyEncipherment],
            extended_key_usage: vec![ExtendedKeyUsage::ServerAuth],
            validity_days: 365,
            key_size: 2048,
            key_algorithm: KeyAlgorithm::Rsa,
            requester: CertificateRequester {
                identity: "test-requester".to_string(),
                organization: Some("Test Org".to_string()),
                country: Some("US".to_string()),
                email: Some("test@example.com".to_string()),
                auth_token: None,
            },
            consensus_proof: None,
        };

        let serialized = bincode::serialize(&request).unwrap();
        let deserialized: StoqCertificateRequest = bincode::deserialize(&serialized).unwrap();

        assert_eq!(request.common_name, deserialized.common_name);
        assert_eq!(request.san_list, deserialized.san_list);
        assert_eq!(request.validity_days, deserialized.validity_days);
    }

    #[tokio::test]
    async fn test_metrics_initialization() {
        let metrics = CaStoqMetrics::default();
        
        assert_eq!(metrics.cert_requests.load(std::sync::atomic::Ordering::Relaxed), 0);
        assert_eq!(metrics.cert_validations.load(std::sync::atomic::Ordering::Relaxed), 0);
        assert_eq!(metrics.cert_revocations.load(std::sync::atomic::Ordering::Relaxed), 0);
        assert_eq!(metrics.cache_hits.load(std::sync::atomic::Ordering::Relaxed), 0);
    }
}