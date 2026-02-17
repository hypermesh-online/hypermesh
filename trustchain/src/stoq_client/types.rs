// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Client types, configuration, and data structures

use std::net::Ipv6Addr;
use std::time::{Duration, SystemTime};
use serde::{Serialize, Deserialize};
use bytes::Bytes;

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
#[allow(dead_code)]
pub(crate) struct CertificateValidationResult {
    pub(crate) is_valid: bool,
    pub(crate) validated_at: SystemTime,
    pub(crate) expires_at: SystemTime,
    pub(crate) fingerprint: String,
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
                        // Safe: hardcoded valid IPv6 address
                        address: "2001:4860:4860::8888".parse()
                            .expect("hardcoded valid IPv6 address"), // Google DNS
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
