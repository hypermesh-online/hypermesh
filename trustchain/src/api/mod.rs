// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! TrustChain API Module
//!
//! **STOQ Protocol Only** - HTTP version REMOVED
//! All TrustChain services now communicate via STOQ (QUIC transport).

use serde::{Deserialize, Serialize};
use std::net::Ipv6Addr;
use std::time::SystemTime;

use crate::ca::IssuedCertificate;
use crate::consensus::ConsensusProof;
use crate::ct::SignedCertificateTimestamp;
use crate::dns::DnsResponse;

// STOQ API (Primary interface)
pub mod stoq_api;

// HTTP modules (DEPRECATED - kept for reference only)
// These modules are commented out to prevent build errors
// See stoq_api.rs for the STOQ-based replacement
// pub mod handlers;
// pub mod middleware_auth;
// pub mod security_handlers;

// Supporting modules (HTTP-independent)
pub mod rate_limiter;
pub mod validators;

// Re-export STOQ API as primary interface
pub use stoq_api::{TrustChainStoqApi, TrustChainStoqConfig};

// Re-export supporting modules
pub use rate_limiter::*;
pub use validators::*;

/// API server statistics (shared between STOQ and legacy HTTP)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiStats {
    pub requests_total: u64,
    pub requests_successful: u64,
    pub requests_failed: u64,
    pub ca_requests: u64,
    pub ct_requests: u64,
    pub dns_requests: u64,
    pub average_response_time_ms: f64,
    pub active_connections: u64,
    pub rate_limited_requests: u64,
    pub last_update: SystemTime,
}

impl Default for ApiStats {
    fn default() -> Self {
        Self {
            requests_total: 0,
            requests_successful: 0,
            requests_failed: 0,
            ca_requests: 0,
            ct_requests: 0,
            dns_requests: 0,
            average_response_time_ms: 0.0,
            active_connections: 0,
            rate_limited_requests: 0,
            last_update: SystemTime::now(),
        }
    }
}

/// Service health status
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub ca: bool,
    pub ct: bool,
    pub dns: bool,
    pub consensus: bool,
}

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: SystemTime,
    pub version: String,
    pub services: ServiceHealth,
}

/// Certificate issuance request
#[derive(Debug, Serialize, Deserialize)]
pub struct CertificateIssueRequest {
    pub common_name: String,
    pub san_entries: Vec<String>,
    pub node_id: String,
    pub ipv6_addresses: Vec<Ipv6Addr>,
    pub consensus_proof: ConsensusProof,
}

/// Certificate response
#[derive(Debug, Serialize, Deserialize)]
pub struct CertificateResponse {
    pub certificate: IssuedCertificate,
    pub sct: Option<SignedCertificateTimestamp>,
}

/// DNS resolve request
#[derive(Debug, Serialize, Deserialize)]
pub struct DnsResolveRequest {
    pub name: String,
    pub record_type: String, // "A", "AAAA", "CNAME", etc.
}

/// Bulk DNS resolve request
#[derive(Debug, Serialize, Deserialize)]
pub struct BulkDnsResolveRequest {
    pub queries: Vec<DnsResolveRequest>,
}

/// Bulk DNS resolve response
#[derive(Debug, Serialize, Deserialize)]
pub struct BulkDnsResolveResponse {
    pub responses: Vec<DnsResponse>,
    pub failed_queries: Vec<String>,
}

/// Certificate validation request
#[derive(Debug, Serialize, Deserialize)]
pub struct CertificateValidationRequest {
    pub certificate_der: String, // Base64 encoded
    pub domain: Option<String>,
}

/// Certificate validation response
#[derive(Debug, Serialize, Deserialize)]
pub struct CertificateValidationResponse {
    pub is_valid: bool,
    pub reason: Option<String>,
    pub ct_verified: bool,
    pub ca_verified: bool,
}

/// Consensus validation request
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsensusValidationRequest {
    pub consensus_proof: ConsensusProof,
    pub operation: String,
}

/// Consensus validation response
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsensusValidationResponse {
    pub is_valid: bool,
    pub validation_details: ConsensusValidationDetails,
}

/// Consensus validation details
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsensusValidationDetails {
    pub stake_valid: bool,
    pub time_valid: bool,
    pub space_valid: bool,
    pub work_valid: bool,
    pub overall_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_stats_default() {
        let stats = ApiStats::default();
        assert_eq!(stats.requests_total, 0);
        assert_eq!(stats.requests_successful, 0);
        assert_eq!(stats.requests_failed, 0);
    }

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            timestamp: SystemTime::now(),
            version: "1.0.0".to_string(),
            services: ServiceHealth {
                ca: true,
                ct: true,
                dns: true,
                consensus: true,
            },
        };

        let json = serde_json::to_string(&response).expect("test: serialization");
        assert!(json.contains("healthy"));
        assert!(json.contains("services"));
    }

    #[test]
    fn test_certificate_request_deserialization() {
        use crate::consensus::ConsensusProof;

        // Test deserialization with simplified approach
        let request = CertificateIssueRequest {
            common_name: "test.example.com".to_string(),
            san_entries: vec![
                "test.example.com".to_string(),
                "alt.example.com".to_string(),
            ],
            node_id: "node123".to_string(),
            ipv6_addresses: vec![std::net::Ipv6Addr::LOCALHOST],
            consensus_proof: ConsensusProof::default_for_testing(),
        };

        // Test serialization/deserialization roundtrip
        let json = serde_json::to_string(&request).expect("test: serialization");
        let deserialized: CertificateIssueRequest = serde_json::from_str(&json).expect("test: deserialization");

        assert_eq!(request.common_name, deserialized.common_name);
        assert_eq!(request.san_entries.len(), deserialized.san_entries.len());
        assert_eq!(request.node_id, deserialized.node_id);
    }
}
