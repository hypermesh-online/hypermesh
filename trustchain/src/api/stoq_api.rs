//! TrustChain STOQ API - Replaces HTTP servers
//!
//! Provides certificate authority, DNS, and trust validation services over STOQ protocol.

use async_trait::async_trait;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use tracing::{info, debug, warn, instrument};

use stoq::api::{ApiHandler, ApiRequest, ApiResponse, ApiError};
use stoq::{StoqApiServer, StoqApiClient};
use stoq::transport::{StoqTransport, TransportConfig};

use crate::ca::TrustChainCA;
use crate::dns::DnsResolver;

/// TrustChain STOQ API configuration
#[derive(Debug, Clone)]
pub struct TrustChainStoqConfig {
    /// STOQ bind address (IPv6)
    pub bind_address: String,
    /// Service name
    pub service_name: String,
    /// Enable request logging
    pub enable_logging: bool,
}

impl Default for TrustChainStoqConfig {
    fn default() -> Self {
        Self {
            bind_address: "[::1]:9293".to_string(), // TrustChain default port
            service_name: "trustchain".to_string(),
            enable_logging: true,
        }
    }
}

// === Request/Response Types ===

/// Certificate validation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateCertificateRequest {
    /// Certificate in PEM format
    pub certificate_pem: String,
    /// Certificate chain in PEM format
    pub chain_pem: Vec<String>,
}

/// Certificate validation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateCertificateResponse {
    /// Whether certificate is valid
    pub valid: bool,
    /// Validation error if any
    pub error: Option<String>,
    /// Certificate details
    pub details: Option<CertificateDetails>,
}

/// Certificate details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateDetails {
    /// Subject common name
    pub subject: String,
    /// Issuer common name
    pub issuer: String,
    /// Not valid before timestamp
    pub not_before: String,
    /// Not valid after timestamp
    pub not_after: String,
}

/// Certificate issuance request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueCertificateRequest {
    /// Certificate signing request (PEM)
    pub csr_pem: String,
    /// Certificate type (server, client, ca)
    pub cert_type: String,
    /// Validity duration in days
    pub validity_days: u32,
}

/// Certificate issuance response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueCertificateResponse {
    /// Issued certificate (PEM)
    pub certificate_pem: String,
    /// Certificate chain (PEM)
    pub chain_pem: Vec<String>,
}

/// DNS resolution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveDnsRequest {
    /// Domain name to resolve
    pub domain: String,
    /// Record type (A, AAAA, SRV, TXT, etc)
    pub record_type: String,
}

/// DNS resolution response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveDnsResponse {
    /// Resolved records
    pub records: Vec<String>,
    /// TTL in seconds
    pub ttl: u32,
}

// === Handlers ===

/// Certificate validation handler
pub struct ValidateCertificateHandler {
    ca: Arc<TrustChainCA>,
}

impl ValidateCertificateHandler {
    pub fn new(ca: Arc<TrustChainCA>) -> Self {
        Self { ca }
    }
}

#[async_trait]
impl ApiHandler for ValidateCertificateHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling certificate validation request: {}", request.id);

        // Deserialize request
        let cert_request: ValidateCertificateRequest = serde_json::from_slice(&request.payload)
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid certificate request: {}", e)))?;

        // Decode PEM certificate to DER for validation
        // TODO: Implement proper PEM parsing
        // For now, assume certificate_pem contains DER-encoded data
        let cert_der = cert_request.certificate_pem.as_bytes().to_vec();

        // Validate certificate through CA
        let is_valid = self.ca.validate_certificate_chain(&cert_der).await
            .map_err(|e| ApiError::HandlerError(e.to_string()))?;

        let response = ValidateCertificateResponse {
            valid: is_valid,
            error: if is_valid { None } else { Some("Certificate validation failed".to_string()) },
            details: None, // TODO: Extract certificate details from parsed cert
        };

        // Serialize response
        let payload = serde_json::to_vec(&response)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "trustchain/validate_certificate"
    }
}

/// Certificate issuance handler
pub struct IssueCertificateHandler {
    ca: Arc<TrustChainCA>,
}

impl IssueCertificateHandler {
    pub fn new(ca: Arc<TrustChainCA>) -> Self {
        Self { ca }
    }
}

#[async_trait]
impl ApiHandler for IssueCertificateHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling certificate issuance request: {}", request.id);

        // Deserialize request
        let issue_request: IssueCertificateRequest = serde_json::from_slice(&request.payload)
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid issuance request: {}", e)))?;

        // Build CertificateRequest for CA
        // TODO: Parse CSR to extract subject info, for now using placeholder values
        use crate::ca::CertificateRequest;
        use crate::consensus::ConsensusProof;

        let cert_request = CertificateRequest {
            common_name: "placeholder.trustchain.local".to_string(), // TODO: Extract from CSR
            san_entries: vec![],
            node_id: "api_node".to_string(),
            ipv6_addresses: vec![std::net::Ipv6Addr::LOCALHOST],
            consensus_proof: ConsensusProof::new_for_testing(), // TODO: Get actual proof
            timestamp: std::time::SystemTime::now(),
        };

        // Issue certificate through CA
        let cert_result = self.ca.issue_certificate(cert_request).await
            .map_err(|e| ApiError::HandlerError(e.to_string()))?;

        let response = IssueCertificateResponse {
            certificate_pem: cert_result.certificate_pem,
            chain_pem: vec![cert_result.chain_pem], // Wrap in vec for API compatibility
        };

        // Serialize response
        let payload = serde_json::to_vec(&response)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "trustchain/issue_certificate"
    }
}

/// DNS resolution handler
pub struct ResolveDnsHandler {
    resolver: Arc<DnsResolver>,
}

impl ResolveDnsHandler {
    pub fn new(resolver: Arc<DnsResolver>) -> Self {
        Self { resolver }
    }
}

#[async_trait]
impl ApiHandler for ResolveDnsHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling DNS resolution request: {}", request.id);

        // Deserialize request
        let dns_request: ResolveDnsRequest = serde_json::from_slice(&request.payload)
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid DNS request: {}", e)))?;

        // Parse record type from string
        let record_type = match dns_request.record_type.as_str() {
            "A" => trust_dns_proto::rr::RecordType::A,
            "AAAA" => trust_dns_proto::rr::RecordType::AAAA,
            "CNAME" => trust_dns_proto::rr::RecordType::CNAME,
            "MX" => trust_dns_proto::rr::RecordType::MX,
            "TXT" => trust_dns_proto::rr::RecordType::TXT,
            "SRV" => trust_dns_proto::rr::RecordType::SRV,
            _ => return Err(ApiError::InvalidRequest(format!("Unsupported record type: {}", dns_request.record_type))),
        };

        // Build DnsQuery for resolver
        use crate::dns::DnsQuery;
        use trust_dns_proto::rr::DNSClass;

        // Parse request.id (String) to u16, use 0 if parsing fails
        let query_id = request.id.parse::<u64>().unwrap_or(0) as u16;

        let query = DnsQuery {
            id: query_id,
            name: dns_request.domain,
            record_type,
            class: DNSClass::IN,
            client_addr: std::net::Ipv6Addr::LOCALHOST, // TODO: Get actual client address
            timestamp: std::time::SystemTime::now(),
        };

        // Resolve DNS through resolver
        let dns_result = self.resolver.resolve_query(&query).await
            .map_err(|e| ApiError::HandlerError(e.to_string()))?;

        // Extract records as strings
        let records: Vec<String> = dns_result.answers.iter()
            .map(|record| format!("{:?}", record.data))
            .collect();

        let response = ResolveDnsResponse {
            records,
            ttl: dns_result.ttl,
        };

        // Serialize response
        let payload = serde_json::to_vec(&response)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "trustchain/resolve_dns"
    }
}

/// Health check handler
pub struct TrustChainHealthHandler;

#[async_trait]
impl ApiHandler for TrustChainHealthHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        #[derive(Serialize)]
        struct HealthStatus {
            status: String,
            service: String,
            version: String,
        }

        let health = HealthStatus {
            status: "healthy".to_string(),
            service: "trustchain".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let payload = serde_json::to_vec(&health)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "trustchain/health"
    }
}

// === Server ===

/// TrustChain STOQ API Server
pub struct TrustChainStoqApi {
    server: Arc<StoqApiServer>,
    config: TrustChainStoqConfig,
}

impl TrustChainStoqApi {
    /// Create new TrustChain API server over STOQ
    #[instrument(skip(ca, resolver))]
    pub async fn new(
        ca: Arc<TrustChainCA>,
        resolver: Arc<DnsResolver>,
        config: TrustChainStoqConfig,
    ) -> Result<Self> {
        info!("Creating TrustChain STOQ API server on {}", config.bind_address);

        // Parse bind address
        let bind_addr: std::net::Ipv6Addr = config.bind_address.split(':')
            .next()
            .and_then(|addr| addr.trim_matches(|c| c == '[' || c == ']').parse().ok())
            .ok_or_else(|| anyhow!("Invalid IPv6 bind address"))?;

        let port: u16 = config.bind_address.split(':')
            .nth(1)
            .and_then(|p| p.parse().ok())
            .ok_or_else(|| anyhow!("Invalid port"))?;

        // Create STOQ transport
        let transport_config = TransportConfig {
            bind_address: bind_addr,
            port,
            ..Default::default()
        };

        let transport = Arc::new(StoqTransport::new(transport_config).await?);

        // Create API server
        let server = Arc::new(StoqApiServer::new(transport));

        // Register handlers
        server.register_handler(Arc::new(ValidateCertificateHandler::new(Arc::clone(&ca))));
        server.register_handler(Arc::new(IssueCertificateHandler::new(Arc::clone(&ca))));
        server.register_handler(Arc::new(ResolveDnsHandler::new(Arc::clone(&resolver))));
        server.register_handler(Arc::new(TrustChainHealthHandler));

        info!("TrustChain STOQ API handlers registered");

        Ok(Self { server, config })
    }

    /// Start the API server
    #[instrument(skip(self))]
    pub async fn serve(self: Arc<Self>) -> Result<()> {
        info!("Starting TrustChain STOQ API server...");
        self.server.listen().await
    }

    /// Stop the server gracefully
    pub fn stop(&self) {
        info!("Stopping TrustChain STOQ API server");
        self.server.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Add TrustChain STOQ API integration tests
}
