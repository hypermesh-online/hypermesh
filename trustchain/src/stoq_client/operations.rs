// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Client operations and implementations

use std::sync::Arc;
use std::time::{Duration, SystemTime};
use dashmap::DashMap;
use tracing::{info, debug};
use sha2::Digest;

use crate::errors::{TrustChainError, Result as TrustChainResult};

use stoq::{
    StoqTransport, Endpoint, Connection
};
use stoq::transport::TransportConfig;

use super::types::*;

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

impl TrustChainStoqClient {
    /// Create new TrustChain STOQ client
    pub async fn new(config: TrustChainStoqConfig) -> TrustChainResult<Self> {
        info!("Initializing TrustChain STOQ client");

        let transport_config = TransportConfig {
            bind_address: config.bind_address,
            port: 0,
            connection_timeout: config.connection_timeout,
            enable_migration: true,
            enable_0rtt: true,
            max_idle_timeout: Duration::from_secs(120),
            max_concurrent_streams: 100,
            send_buffer_size: 8 * 1024 * 1024,
            receive_buffer_size: 8 * 1024 * 1024,
            max_connections: Some(config.max_connections_per_service as u32),
            connection_pool_size: 10,
            enable_zero_copy: true,
            max_datagram_size: 65507,
            congestion_control: stoq::transport::CongestionControl::Bbr2,
            health_check_interval: 10,
            connection_idle_timeout: 30,
            enable_memory_pool: true,
            memory_pool_size: 512,
            frame_batch_size: 32,
            enable_cpu_affinity: false,
            enable_large_send_offload: false,
            cert_rotation_interval: Duration::from_secs(24 * 60 * 60),
            enable_falcon_crypto: true,
            falcon_variant: stoq::FalconVariant::Falcon1024,
            ebpf_interface: None,
        };

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

        let resolver_endpoint = self.select_dns_resolver().await?;
        let connection = self.get_or_create_connection(&resolver_endpoint).await?;

        let query_data = bincode::serialize(&query)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "dns_query_serialize".to_string(),
                reason: e.to_string(),
            })?;

        self.transport.send(&connection, &query_data).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "dns_query_send".to_string(),
                reason: e.to_string(),
            })?;

        let response_data = self.transport.receive(&connection).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "dns_response_receive".to_string(),
                reason: e.to_string(),
            })?;

        let response: StoqDnsResponse = bincode::deserialize(&response_data)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "dns_response_deserialize".to_string(),
                reason: e.to_string(),
            })?;

        let latency = start_time.elapsed().as_micros() as u64;
        self.metrics.dns_queries.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics.bytes_sent.fetch_add(query_data.len() as u64, std::sync::atomic::Ordering::Relaxed);
        self.metrics.bytes_received.fetch_add(response_data.len() as u64, std::sync::atomic::Ordering::Relaxed);
        self.update_average_latency(latency);

        debug!("DNS query resolved successfully: {} ({}us)", query.domain, latency);
        Ok(response)
    }

    /// Validate certificate over STOQ transport
    pub async fn validate_certificate(&self, request: CertificateValidationRequest) -> TrustChainResult<bool> {
        let start_time = std::time::Instant::now();

        let fingerprint = hex::encode(sha2::Sha256::digest(&request.certificate_der));

        if let Some(cached_result) = self.cert_cache.get(&fingerprint) {
            if cached_result.expires_at > SystemTime::now() {
                debug!("Certificate validation cache hit: {}", fingerprint);
                return Ok(cached_result.is_valid);
            } else {
                self.cert_cache.remove(&fingerprint);
            }
        }

        debug!("Validating certificate over STOQ: {}", fingerprint);

        let ca_endpoint = self.select_ca_endpoint().await?;
        let connection = self.get_or_create_connection(&ca_endpoint).await?;

        let request_data = bincode::serialize(&request)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "cert_validation_serialize".to_string(),
                reason: e.to_string(),
            })?;

        self.transport.send(&connection, &request_data).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "cert_validation_send".to_string(),
                reason: e.to_string(),
            })?;

        let response_data = self.transport.receive(&connection).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "cert_validation_receive".to_string(),
                reason: e.to_string(),
            })?;

        let is_valid: bool = bincode::deserialize(&response_data)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "cert_validation_deserialize".to_string(),
                reason: e.to_string(),
            })?;

        let cache_entry = CertificateValidationResult {
            is_valid,
            validated_at: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(3600),
            fingerprint: fingerprint.clone(),
        };
        let fingerprint_for_log = fingerprint.clone();
        self.cert_cache.insert(fingerprint, cache_entry);

        let latency = start_time.elapsed().as_micros() as u64;
        self.metrics.certificate_validations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics.bytes_sent.fetch_add(request_data.len() as u64, std::sync::atomic::Ordering::Relaxed);
        self.metrics.bytes_received.fetch_add(response_data.len() as u64, std::sync::atomic::Ordering::Relaxed);
        self.update_average_latency(latency);

        debug!("Certificate validation completed: {} -> {} ({}us)", fingerprint_for_log, is_valid, latency);
        Ok(is_valid)
    }

    /// Submit certificate to CT log over STOQ transport
    pub async fn submit_to_ct_log(&self, submission: CtLogSubmission) -> TrustChainResult<String> {
        let start_time = std::time::Instant::now();
        debug!("Submitting certificate to CT log over STOQ: {}", submission.log_id);

        let ct_endpoint = self.select_ct_log().await?;
        let connection = self.get_or_create_connection(&ct_endpoint).await?;

        let submission_data = bincode::serialize(&submission)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "ct_submission_serialize".to_string(),
                reason: e.to_string(),
            })?;

        self.transport.send(&connection, &submission_data).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "ct_submission_send".to_string(),
                reason: e.to_string(),
            })?;

        let sct_data = self.transport.receive(&connection).await
            .map_err(|e| TrustChainError::NetworkError {
                operation: "ct_sct_receive".to_string(),
                reason: e.to_string(),
            })?;

        let sct_id: String = bincode::deserialize(&sct_data)
            .map_err(|e| TrustChainError::SerializationError {
                operation: "ct_sct_deserialize".to_string(),
                reason: e.to_string(),
            })?;

        let latency = start_time.elapsed().as_micros() as u64;
        self.metrics.ct_submissions.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics.bytes_sent.fetch_add(submission_data.len() as u64, std::sync::atomic::Ordering::Relaxed);
        self.metrics.bytes_received.fetch_add(sct_data.len() as u64, std::sync::atomic::Ordering::Relaxed);
        self.update_average_latency(latency);

        debug!("CT log submission completed: {} -> {} ({}us)", submission.log_id, sct_id, latency);
        Ok(sct_id)
    }

    /// Get or create connection to service endpoint
    async fn get_or_create_connection(&self, endpoint: &ServiceEndpoint) -> TrustChainResult<Arc<Connection>> {
        if let Some(existing_conn) = self.connections.get(endpoint) {
            if existing_conn.is_active() {
                return Ok(existing_conn.clone());
            } else {
                self.connections.remove(endpoint);
            }
        }

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

        if self.config.enable_connection_pooling {
            self.connections.insert(endpoint.clone(), connection.clone());
        }

        self.metrics.connections_established.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        debug!("STOQ connection established successfully: [{}]:{}", endpoint.address, endpoint.port);
        Ok(connection)
    }

    /// Select best DNS resolver endpoint
    async fn select_dns_resolver(&self) -> TrustChainResult<ServiceEndpoint> {
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
        let current_avg = self.metrics.average_latency_us.load(std::sync::atomic::Ordering::Relaxed);
        let new_avg = if current_avg == 0 {
            latency_us
        } else {
            (current_avg * 9 + latency_us) / 10
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

        let mut expired_certs = Vec::new();
        for entry in self.cert_cache.iter() {
            if entry.value().expires_at <= now {
                expired_certs.push(entry.key().clone());
            }
        }
        for cert in expired_certs {
            self.cert_cache.remove(&cert);
        }

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

        for entry in self.connections.iter() {
            entry.value().close();
        }
        self.connections.clear();

        self.transport.shutdown().await;

        info!("TrustChain STOQ client shutdown complete");
        Ok(())
    }
}
