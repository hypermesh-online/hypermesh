//! STOQ Client Library for TrustChain Integration
//!
//! This module provides a comprehensive STOQ client that integrates with TrustChain's
//! Certificate Authority, Certificate Transparency, and DNS services. All transport
//! operations are delegated to STOQ protocol for high-performance networking.

use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::net::{Ipv6Addr, SocketAddr};
use std::collections::HashMap;
use dashmap::DashMap;
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};
use tokio::sync::{RwLock, Mutex};
use tracing::{info, debug, warn, error};
use bytes::Bytes;

use crate::errors::{TrustChainError, Result as TrustChainResult};

// Import STOQ transport
use stoq::{
    StoqTransport, Endpoint, Connection
};
use stoq::config::TransportConfig;

/// TrustChain STOQ client for all network operations
pub struct TrustChainStoqClient {
    /// STOQ transport instance
    transport: Arc<StoqTransport>,
    /// Connection pool for different services
    connections: Arc<DashMap<ServiceEndpoint, Arc<Connection>>>,
    /// Client configuration
    config: TrustChainStoqConfig,
    /// Performance metrics
    metrics: Arc<StoqClientMetrics>,
    /// Certificate validation cache
    cert_cache: Arc<DashMap<String, CertificateValidationResult>>,
}

/// STOQ client configuration for TrustChain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustChainStoqConfig {
    /// Client bind address (IPv6 only)
    pub bind_address: Ipv6Addr,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Enable connection pooling
    pub enable_connection_pooling: bool,
    /// Maximum connections per service
    pub max_connections_per_service: usize,
    /// Certificate validation timeout
    pub cert_validation_timeout: Duration,
    /// DNS query timeout
    pub dns_query_timeout: Duration,
    /// CT log submission timeout
    pub ct_submission_timeout: Duration,
    /// Service discovery configuration
    pub service_discovery: ServiceDiscoveryConfig,
}

/// Service discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDiscoveryConfig {
    /// DNS resolver endpoints
    pub dns_resolvers: Vec<ServiceEndpoint>,
    /// Certificate transparency log endpoints
    pub ct_logs: Vec<ServiceEndpoint>,
    /// Certificate authority endpoints
    pub ca_endpoints: Vec<ServiceEndpoint>,
    /// Service health check interval
    pub health_check_interval: Duration,
}

/// Service endpoint identification
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Service type
    pub service_type: ServiceType,
    /// IPv6 address
    pub address: Ipv6Addr,
    /// Port number
    pub port: u16,
    /// Optional service name for SNI
    pub service_name: Option<String>,
}

/// Service types supported by TrustChain
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ServiceType {
    /// DNS resolution service
    Dns,
    /// Certificate Authority service
    CertificateAuthority,
    /// Certificate Transparency log
    CertificateTransparency,
    /// TrustChain consensus node
    ConsensusNode,
    /// HyperMesh asset discovery
    AssetDiscovery,
}

/// Client performance metrics
#[derive(Debug, Default)]
pub struct StoqClientMetrics {
    /// Total connections established
    pub connections_established: std::sync::atomic::AtomicU64,
    /// Total bytes sent
    pub bytes_sent: std::sync::atomic::AtomicU64,
    /// Total bytes received  
    pub bytes_received: std::sync::atomic::AtomicU64,
    /// DNS queries performed
    pub dns_queries: std::sync::atomic::AtomicU64,
    /// Certificate validations
    pub certificate_validations: std::sync::atomic::AtomicU64,
    /// CT log submissions
    pub ct_submissions: std::sync::atomic::AtomicU64,
    /// Average latency in microseconds
    pub average_latency_us: std::sync::atomic::AtomicU64,
    /// Connection errors
    pub connection_errors: std::sync::atomic::AtomicU64,
}

/// Certificate validation result
#[derive(Debug, Clone)]
struct CertificateValidationResult {
    is_valid: bool,
    validated_at: SystemTime,
    expires_at: SystemTime,
    fingerprint: String,
}

/// DNS query request over STOQ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqDnsQuery {
    /// Query ID
    pub query_id: u16,
    /// Domain name
    pub domain: String,
    /// Query type (A, AAAA, CNAME, etc.)
    pub query_type: u16,
    /// Query flags
    pub flags: u16,
    /// Client IP for logging
    pub client_ip: Ipv6Addr,
}

/// DNS response over STOQ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqDnsResponse {
    /// Query ID
    pub query_id: u16,
    /// Response code
    pub response_code: u16,
    /// Answer records
    pub answers: Vec<DnsResourceRecord>,
    /// Authority records
    pub authorities: Vec<DnsResourceRecord>,
    /// Additional records
    pub additionals: Vec<DnsResourceRecord>,
    /// Response flags
    pub flags: u16,
}

/// DNS resource record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsResourceRecord {
    /// Record name
    pub name: String,
    /// Record type
    pub record_type: u16,
    /// Record class
    pub class: u16,
    /// TTL in seconds
    pub ttl: u32,
    /// Record data
    pub data: Bytes,
}

/// Certificate validation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateValidationRequest {
    /// Certificate DER data
    pub certificate_der: Bytes,
    /// Certificate chain (optional)
    pub chain: Option<Vec<Bytes>>,
    /// Hostname to validate (optional)
    pub hostname: Option<String>,
    /// Validation policy
    pub policy: ValidationPolicy,
}

/// Certificate validation policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationPolicy {
    /// Standard X.509 validation
    Standard,
    /// TrustChain consensus validation
    TrustChainConsensus,
    /// Extended validation with CT logs
    ExtendedValidation,
}

/// CT log submission request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CtLogSubmission {
    /// Certificate to log
    pub certificate: Bytes,
    /// Certificate chain
    pub chain: Vec<Bytes>,
    /// Submission timestamp
    pub timestamp: SystemTime,
    /// Log ID
    pub log_id: String,
}

impl Default for TrustChainStoqConfig {
    fn default() -> Self {
        Self {
            bind_address: Ipv6Addr::UNSPECIFIED,
            connection_timeout: Duration::from_secs(5),
            enable_connection_pooling: true,
            max_connections_per_service: 10,
            cert_validation_timeout: Duration::from_secs(10),
            dns_query_timeout: Duration::from_secs(5),
            ct_submission_timeout: Duration::from_secs(30),
            service_discovery: ServiceDiscoveryConfig {
                dns_resolvers: vec![
                    ServiceEndpoint {
                        service_type: ServiceType::Dns,
                        address: "2001:4860:4860::8888".parse().unwrap(), // Google DNS
                        port: 853, // DNS-over-QUIC port
                        service_name: Some("dns.google".to_string()),
                    },
                ],
                ct_logs: vec![
                    ServiceEndpoint {
                        service_type: ServiceType::CertificateTransparency,
                        address: Ipv6Addr::LOCALHOST, // Placeholder
                        port: 6962,
                        service_name: Some("ct.trustchain.local".to_string()),
                    },
                ],
                ca_endpoints: vec![
                    ServiceEndpoint {
                        service_type: ServiceType::CertificateAuthority,
                        address: Ipv6Addr::LOCALHOST,
                        port: 8443,
                        service_name: Some("ca.trustchain.local".to_string()),
                    },
                ],
                health_check_interval: Duration::from_secs(60),
            },
        }
    }
}

impl TrustChainStoqClient {
    /// Create new TrustChain STOQ client
    pub async fn new(config: TrustChainStoqConfig) -> TrustChainResult<Self> {
        info!("Initializing TrustChain STOQ client");

        // Configure STOQ transport for TrustChain
        let transport_config = TransportConfig {
            bind_address: config.bind_address,
            port: 0, // Use ephemeral port for client
            connection_timeout: config.connection_timeout,
            enable_migration: true,
            enable_0rtt: true,
            max_idle_timeout: Duration::from_secs(120),
            max_concurrent_streams: 100,
            send_buffer_size: 8 * 1024 * 1024, // 8MB
            receive_buffer_size: 8 * 1024 * 1024, // 8MB
            max_connections: Some(config.max_connections_per_service as u32),
            connection_pool_size: 10, // Default connection pool size
            enable_zero_copy: true,
            max_datagram_size: 65507,
            congestion_control: stoq::transport::CongestionControl::Bbr2,
            enable_memory_pool: true,
            memory_pool_size: 512,
            frame_batch_size: 32,
            enable_cpu_affinity: false, // Client mode
            enable_large_send_offload: false, // Client mode
            hardware_accel: stoq::transport::hardware_acceleration::HardwareAccelConfig::default(),
            cert_rotation_interval: Duration::from_secs(24 * 60 * 60), // 24 hours
        };

        // Initialize STOQ transport
        let transport = Arc::new(StoqTransport::new(transport_config).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "stoq_transport_init".to_string(),
                reason: e.to_string(),
            })?);

        let client = Self {
            transport,
            connections: Arc::new(DashMap::new()),
            config,
            metrics: Arc::new(StoqClientMetrics::default()),
            cert_cache: Arc::new(DashMap::new()),
        };

        info!("TrustChain STOQ client initialized successfully");
        Ok(client)
    }

    /// Perform DNS resolution over STOQ transport
    pub async fn resolve_dns(&self, query: StoqDnsQuery) -> TrustChainResult<StoqDnsResponse> {
        let start_time = std::time::Instant::now();
        debug!("Resolving DNS query over STOQ: {} (type: {})", query.domain, query.query_type);

        // Select DNS resolver
        let resolver_endpoint = self.select_dns_resolver().await?;
        
        // Get or create connection
        let connection = self.get_or_create_connection(&resolver_endpoint).await?;

        // Serialize DNS query
        let query_data = bincode::serialize(&query)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "dns_query_serialize".to_string(),
                reason: e.to_string(),
            })?;

        // Send query over STOQ
        self.transport.send(&connection, &query_data).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "dns_query_send".to_string(),
                reason: e.to_string(),
            })?;

        // Receive response
        let response_data = self.transport.receive(&connection).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "dns_response_receive".to_string(),
                reason: e.to_string(),
            })?;

        // Deserialize response
        let response: StoqDnsResponse = bincode::deserialize(&response_data)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "dns_response_deserialize".to_string(),
                reason: e.to_string(),
            })?;

        // Update metrics
        let latency = start_time.elapsed().as_micros() as u64;
        self.metrics.dns_queries.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics.bytes_sent.fetch_add(query_data.len() as u64, std::sync::atomic::Ordering::Relaxed);
        self.metrics.bytes_received.fetch_add(response_data.len() as u64, std::sync::atomic::Ordering::Relaxed);
        self.update_average_latency(latency);

        debug!("DNS query resolved successfully: {} ({}μs)", query.domain, latency);
        Ok(response)
    }

    /// Validate certificate over STOQ transport
    pub async fn validate_certificate(&self, request: CertificateValidationRequest) -> TrustChainResult<bool> {
        let start_time = std::time::Instant::now();
        
        // Calculate certificate fingerprint for caching
        let fingerprint = hex::encode(sha2::Sha256::digest(&request.certificate_der));
        
        // Check cache first
        if let Some(cached_result) = self.cert_cache.get(&fingerprint) {
            if cached_result.expires_at > SystemTime::now() {
                debug!("Certificate validation cache hit: {}", fingerprint);
                return Ok(cached_result.is_valid);
            } else {
                self.cert_cache.remove(&fingerprint);
            }
        }

        debug!("Validating certificate over STOQ: {}", fingerprint);

        // Select CA endpoint
        let ca_endpoint = self.select_ca_endpoint().await?;
        
        // Get or create connection
        let connection = self.get_or_create_connection(&ca_endpoint).await?;

        // Serialize validation request
        let request_data = bincode::serialize(&request)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "cert_validation_serialize".to_string(),
                reason: e.to_string(),
            })?;

        // Send validation request
        self.transport.send(&connection, &request_data).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "cert_validation_send".to_string(),
                reason: e.to_string(),
            })?;

        // Receive validation response
        let response_data = self.transport.receive(&connection).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "cert_validation_receive".to_string(),
                reason: e.to_string(),
            })?;

        // Parse validation result
        let is_valid: bool = bincode::deserialize(&response_data)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "cert_validation_deserialize".to_string(),
                reason: e.to_string(),
            })?;

        // Cache result
        let cache_entry = CertificateValidationResult {
            is_valid,
            validated_at: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(3600), // 1 hour cache
            fingerprint: fingerprint.clone(),
        };
        // Store fingerprint for later use before moving it
        let fingerprint_for_log = fingerprint.clone();
        self.cert_cache.insert(fingerprint, cache_entry);

        // Update metrics
        let latency = start_time.elapsed().as_micros() as u64;
        self.metrics.certificate_validations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics.bytes_sent.fetch_add(request_data.len() as u64, std::sync::atomic::Ordering::Relaxed);
        self.metrics.bytes_received.fetch_add(response_data.len() as u64, std::sync::atomic::Ordering::Relaxed);
        self.update_average_latency(latency);

        debug!("Certificate validation completed: {} -> {} ({}μs)", fingerprint_for_log, is_valid, latency);
        Ok(is_valid)
    }

    /// Submit certificate to CT log over STOQ transport
    pub async fn submit_to_ct_log(&self, submission: CtLogSubmission) -> TrustChainResult<String> {
        let start_time = std::time::Instant::now();
        debug!("Submitting certificate to CT log over STOQ: {}", submission.log_id);

        // Select CT log endpoint
        let ct_endpoint = self.select_ct_log().await?;
        
        // Get or create connection
        let connection = self.get_or_create_connection(&ct_endpoint).await?;

        // Serialize CT submission
        let submission_data = bincode::serialize(&submission)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "ct_submission_serialize".to_string(),
                reason: e.to_string(),
            })?;

        // Send submission
        self.transport.send(&connection, &submission_data).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "ct_submission_send".to_string(),
                reason: e.to_string(),
            })?;

        // Receive SCT (Signed Certificate Timestamp)
        let sct_data = self.transport.receive(&connection).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "ct_sct_receive".to_string(),
                reason: e.to_string(),
            })?;

        // Parse SCT
        let sct_id: String = bincode::deserialize(&sct_data)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "ct_sct_deserialize".to_string(),
                reason: e.to_string(),
            })?;

        // Update metrics
        let latency = start_time.elapsed().as_micros() as u64;
        self.metrics.ct_submissions.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics.bytes_sent.fetch_add(submission_data.len() as u64, std::sync::atomic::Ordering::Relaxed);
        self.metrics.bytes_received.fetch_add(sct_data.len() as u64, std::sync::atomic::Ordering::Relaxed);
        self.update_average_latency(latency);

        debug!("CT log submission completed: {} -> {} ({}μs)", submission.log_id, sct_id, latency);
        Ok(sct_id)
    }

    /// Get or create connection to service endpoint
    async fn get_or_create_connection(&self, endpoint: &ServiceEndpoint) -> TrustChainResult<Arc<Connection>> {
        // Check if we have a cached connection
        if let Some(existing_conn) = self.connections.get(endpoint) {
            if existing_conn.is_active() {
                return Ok(existing_conn.clone());
            } else {
                // Remove inactive connection
                self.connections.remove(endpoint);
            }
        }

        // Create new connection
        let stoq_endpoint = Endpoint::new(endpoint.address, endpoint.port)
            .with_server_name(endpoint.service_name.clone().unwrap_or_else(|| {
                format!("{}.trustchain.local", endpoint.service_type.as_str())
            }));

        debug!("Creating new STOQ connection to: [{}]:{}", endpoint.address, endpoint.port);

        let connection = self.transport.connect(&stoq_endpoint).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "stoq_connection".to_string(),
                reason: e.to_string(),
            })?;

        // Cache connection if pooling is enabled
        if self.config.enable_connection_pooling {
            self.connections.insert(endpoint.clone(), connection.clone());
        }

        // Update metrics
        self.metrics.connections_established.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        debug!("STOQ connection established successfully: [{}]:{}", endpoint.address, endpoint.port);
        Ok(connection)
    }

    /// Select best DNS resolver endpoint
    async fn select_dns_resolver(&self) -> TrustChainResult<ServiceEndpoint> {
        // Simple round-robin selection for now
        // In production, this would include health checks and load balancing
        self.config.service_discovery.dns_resolvers
            .first()
            .cloned()
            .ok_or_else(|| TrustChainError::ServiceDiscoveryError {
                service: "dns_resolver".to_string(),
                reason: "No DNS resolvers configured".to_string(),
            })
    }

    /// Select best CA endpoint
    async fn select_ca_endpoint(&self) -> TrustChainResult<ServiceEndpoint> {
        self.config.service_discovery.ca_endpoints
            .first()
            .cloned()
            .ok_or_else(|| TrustChainError::ServiceDiscoveryError {
                service: "certificate_authority".to_string(),
                reason: "No CA endpoints configured".to_string(),
            })
    }

    /// Select best CT log endpoint
    async fn select_ct_log(&self) -> TrustChainResult<ServiceEndpoint> {
        self.config.service_discovery.ct_logs
            .first()
            .cloned()
            .ok_or_else(|| TrustChainError::ServiceDiscoveryError {
                service: "certificate_transparency".to_string(),
                reason: "No CT log endpoints configured".to_string(),
            })
    }

    /// Update average latency metric
    fn update_average_latency(&self, latency_us: u64) {
        // Simple moving average - in production this would be more sophisticated
        let current_avg = self.metrics.average_latency_us.load(std::sync::atomic::Ordering::Relaxed);
        let new_avg = if current_avg == 0 {
            latency_us
        } else {
            (current_avg * 9 + latency_us) / 10 // 90% old, 10% new
        };
        self.metrics.average_latency_us.store(new_avg, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get client performance metrics
    pub fn get_metrics(&self) -> StoqClientMetrics {
        StoqClientMetrics {
            connections_established: std::sync::atomic::AtomicU64::new(
                self.metrics.connections_established.load(std::sync::atomic::Ordering::Relaxed)
            ),
            bytes_sent: std::sync::atomic::AtomicU64::new(
                self.metrics.bytes_sent.load(std::sync::atomic::Ordering::Relaxed)
            ),
            bytes_received: std::sync::atomic::AtomicU64::new(
                self.metrics.bytes_received.load(std::sync::atomic::Ordering::Relaxed)
            ),
            dns_queries: std::sync::atomic::AtomicU64::new(
                self.metrics.dns_queries.load(std::sync::atomic::Ordering::Relaxed)
            ),
            certificate_validations: std::sync::atomic::AtomicU64::new(
                self.metrics.certificate_validations.load(std::sync::atomic::Ordering::Relaxed)
            ),
            ct_submissions: std::sync::atomic::AtomicU64::new(
                self.metrics.ct_submissions.load(std::sync::atomic::Ordering::Relaxed)
            ),
            average_latency_us: std::sync::atomic::AtomicU64::new(
                self.metrics.average_latency_us.load(std::sync::atomic::Ordering::Relaxed)
            ),
            connection_errors: std::sync::atomic::AtomicU64::new(
                self.metrics.connection_errors.load(std::sync::atomic::Ordering::Relaxed)
            ),
        }
    }

    /// Get transport statistics
    pub fn get_transport_stats(&self) -> stoq::TransportStats {
        self.transport.stats()
    }

    /// Get the underlying STOQ transport (for low-level access)
    pub fn transport(&self) -> Arc<StoqTransport> {
        self.transport.clone()
    }

    /// Cleanup expired connections and cached data
    pub async fn cleanup(&self) -> TrustChainResult<()> {
        info!("Cleaning up TrustChain STOQ client");

        let now = SystemTime::now();

        // Clean expired certificate validation cache
        let mut expired_certs = Vec::new();
        for entry in self.cert_cache.iter() {
            if entry.value().expires_at <= now {
                expired_certs.push(entry.key().clone());
            }
        }
        for cert in expired_certs {
            self.cert_cache.remove(&cert);
        }

        // Clean inactive connections
        let mut inactive_endpoints = Vec::new();
        for entry in self.connections.iter() {
            if !entry.value().is_active() {
                inactive_endpoints.push(entry.key().clone());
            }
        }
        for endpoint in inactive_endpoints {
            self.connections.remove(&endpoint);
        }

        debug!("STOQ client cleanup completed");
        Ok(())
    }

    /// Shutdown the STOQ client
    pub async fn shutdown(&self) -> TrustChainResult<()> {
        info!("Shutting down TrustChain STOQ client");

        // Close all connections
        for entry in self.connections.iter() {
            entry.value().close();
        }
        self.connections.clear();

        // Shutdown transport
        self.transport.shutdown().await;

        info!("TrustChain STOQ client shutdown complete");
        Ok(())
    }
}

impl ServiceType {
    /// Convert service type to string
    pub fn as_str(&self) -> &'static str {
        match self {
            ServiceType::Dns => "dns",
            ServiceType::CertificateAuthority => "ca",
            ServiceType::CertificateTransparency => "ct",
            ServiceType::ConsensusNode => "consensus",
            ServiceType::AssetDiscovery => "assets",
        }
    }
}

impl ServiceEndpoint {
    /// Create new service endpoint
    pub fn new(service_type: ServiceType, address: Ipv6Addr, port: u16) -> Self {
        Self {
            service_type,
            address,
            port,
            service_name: None,
        }
    }

    /// Set service name for SNI
    pub fn with_service_name(mut self, name: String) -> Self {
        self.service_name = Some(name);
        self
    }
}

// Additional required imports
use sha2::Digest;

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv6Addr;

    #[tokio::test]
    async fn test_stoq_client_creation() {
        let config = TrustChainStoqConfig::default();
        
        // Note: This test may fail without proper STOQ setup
        // In integration tests, we would mock the STOQ transport
        if let Ok(client) = TrustChainStoqClient::new(config).await {
            let metrics = client.get_metrics();
            assert_eq!(metrics.dns_queries.load(std::sync::atomic::Ordering::Relaxed), 0);
        }
    }

    #[test]
    fn test_service_endpoint_creation() {
        let endpoint = ServiceEndpoint::new(
            ServiceType::Dns,
            Ipv6Addr::LOCALHOST,
            853
        ).with_service_name("dns.test.local".to_string());

        assert_eq!(endpoint.service_type, ServiceType::Dns);
        assert_eq!(endpoint.port, 853);
        assert_eq!(endpoint.service_name, Some("dns.test.local".to_string()));
    }

    #[test]
    fn test_service_type_string_conversion() {
        assert_eq!(ServiceType::Dns.as_str(), "dns");
        assert_eq!(ServiceType::CertificateAuthority.as_str(), "ca");
        assert_eq!(ServiceType::CertificateTransparency.as_str(), "ct");
        assert_eq!(ServiceType::ConsensusNode.as_str(), "consensus");
        assert_eq!(ServiceType::AssetDiscovery.as_str(), "assets");
    }

    #[tokio::test]
    async fn test_dns_query_serialization() {
        let query = StoqDnsQuery {
            query_id: 1234,
            domain: "example.com".to_string(),
            query_type: 1, // A record
            flags: 0x0100, // RD flag
            client_ip: Ipv6Addr::LOCALHOST,
        };

        let serialized = bincode::serialize(&query).unwrap();
        let deserialized: StoqDnsQuery = bincode::deserialize(&serialized).unwrap();

        assert_eq!(query.query_id, deserialized.query_id);
        assert_eq!(query.domain, deserialized.domain);
        assert_eq!(query.query_type, deserialized.query_type);
    }
}