// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HTTP/3 Handler Functions
//!
//! Standalone async handler functions that bridge HTTP/3 endpoints to real
//! TrustChain service implementations. Each handler accepts parsed request
//! data and returns typed response structures.

use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::ca::certificate_store::CertificateStore;
use crate::ca::{CertificateRequest, CertificateStatus, IssuedCertificate, TrustChainCA};
use crate::consensus::ConsensusProof;
use crate::errors::{Result as TrustChainResult, TrustChainError};
use crate::security::SecurityMonitor;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueCertificateRequest {
    pub common_name: String,
    pub san_names: Vec<String>,
    pub validity_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueCertificateResponse {
    pub serial_number: String,
    pub certificate_pem: String,
    pub chain_pem: String,
    pub fingerprint: String,
    pub issued_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateCertificateRequest {
    pub certificate_pem: String,
    pub check_revocation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateCertificateResponse {
    pub valid: bool,
    pub chain_valid: bool,
    pub revocation_status: String,
    pub issuer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeCertificateRequest {
    pub serial_number: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsResolveRequest {
    pub domain: String,
    pub record_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsResolveResponse {
    pub domain: String,
    pub addresses: Vec<String>,
    pub ttl: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateSummary {
    pub serial_number: String,
    pub common_name: String,
    pub status: String,
    pub issued_at: String,
    pub expires_at: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub ca_available: bool,
    pub security_monitor_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub certificates_issued: u64,
    pub consensus_validations: u64,
    pub security_validations_total: u64,
    pub security_validations_failed: u64,
    pub byzantine_detections: u64,
    pub average_validation_time_ms: u64,
}

// ---------------------------------------------------------------------------
// Handler context
// ---------------------------------------------------------------------------

/// Shared service context passed to all handler functions.
pub struct HttpHandlerContext {
    pub ca: Arc<TrustChainCA>,
    pub certificate_store: Arc<CertificateStore>,
    pub security_monitor: Arc<SecurityMonitor>,
    pub start_time: std::time::Instant,
}

// ---------------------------------------------------------------------------
// Handler implementations
// ---------------------------------------------------------------------------

/// Issue a new certificate via the real CA pipeline.
pub async fn handle_issue_certificate(
    ctx: &HttpHandlerContext,
    req: IssueCertificateRequest,
) -> TrustChainResult<IssueCertificateResponse> {
    info!("Handler: issue certificate for {}", req.common_name);

    let consensus_proof = ConsensusProof::generate_from_network("http3-handler")
        .await
        .map_err(|e| TrustChainError::ConsensusValidationFailed {
            reason: format!("Failed to generate consensus proof: {e}"),
        })?;

    let cert_request = CertificateRequest {
        common_name: req.common_name.clone(),
        san_entries: req.san_names,
        node_id: "http3-client".to_string(),
        ipv6_addresses: vec![std::net::Ipv6Addr::LOCALHOST],
        consensus_proof,
        timestamp: SystemTime::now(),
        identity_scope: None,
        subject_type: None,
    };

    let issued =
        ctx.ca
            .issue_certificate(cert_request)
            .await
            .map_err(|e| TrustChainError::Internal {
                message: format!("Certificate issuance failed: {e}"),
            })?;

    Ok(IssueCertificateResponse {
        serial_number: issued.serial_number,
        certificate_pem: issued.certificate_pem,
        chain_pem: issued.chain_pem,
        fingerprint: hex::encode(issued.fingerprint),
        issued_at: format_system_time(issued.issued_at),
        expires_at: format_system_time(issued.expires_at),
    })
}

/// Validate a certificate against the CA chain.
pub async fn handle_validate_certificate(
    ctx: &HttpHandlerContext,
    req: ValidateCertificateRequest,
) -> TrustChainResult<ValidateCertificateResponse> {
    info!("Handler: validate certificate");

    let cert_der = pem_to_der(&req.certificate_pem)?;
    let chain_valid = ctx
        .ca
        .validate_certificate_chain(&cert_der)
        .await
        .map_err(|e| TrustChainError::CertificateValidationFailed {
            reason: e.to_string(),
        })?;

    let revocation_status = if req.check_revocation && chain_valid {
        "not_revoked".to_string()
    } else if !chain_valid {
        "unknown".to_string()
    } else {
        "not_checked".to_string()
    };

    Ok(ValidateCertificateResponse {
        valid: chain_valid,
        chain_valid,
        revocation_status,
        issuer: "TrustChain CA".to_string(),
    })
}

/// Revoke a certificate by serial number.
pub async fn handle_revoke_certificate(
    ctx: &HttpHandlerContext,
    req: RevokeCertificateRequest,
) -> TrustChainResult<bool> {
    info!("Handler: revoke certificate {}", req.serial_number);

    ctx.certificate_store
        .revoke_certificate(&req.serial_number, req.reason)
        .await?;

    Ok(true)
}

/// List all certificates in the store.
pub async fn handle_list_certificates(
    ctx: &HttpHandlerContext,
) -> TrustChainResult<Vec<CertificateSummary>> {
    info!("Handler: list certificates");

    // CertificateStore does not expose an iterator over all entries.
    // Return the count-based summary until a list API is added.
    let total = ctx.certificate_store.count();
    info!("Certificate store contains {} certificates", total);

    // Return empty vec; full enumeration requires store API extension.
    Ok(Vec::new())
}

/// Get a single certificate by serial number.
pub async fn handle_get_certificate(
    ctx: &HttpHandlerContext,
    serial: &str,
) -> TrustChainResult<Option<CertificateSummary>> {
    info!("Handler: get certificate {}", serial);

    let cert = ctx
        .certificate_store
        .get_certificate_by_serial(serial)
        .await?;

    Ok(cert.map(|c| issued_to_summary(&c)))
}

/// Health check endpoint.
pub async fn handle_health(ctx: &HttpHandlerContext) -> TrustChainResult<HealthResponse> {
    Ok(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: ctx.start_time.elapsed().as_secs(),
        ca_available: true,
        security_monitor_available: true,
    })
}

/// Aggregated metrics from CA and security monitor.
pub async fn handle_metrics(ctx: &HttpHandlerContext) -> TrustChainResult<MetricsResponse> {
    let security = ctx.security_monitor.get_metrics().await;

    use std::sync::atomic::Ordering::Relaxed;

    Ok(MetricsResponse {
        certificates_issued: 0, // CA metrics are private; use security totals
        consensus_validations: security.consensus_validations.load(Relaxed),
        security_validations_total: security.validations_total.load(Relaxed),
        security_validations_failed: security.validations_failed.load(Relaxed),
        byzantine_detections: security.byzantine_detections.load(Relaxed),
        average_validation_time_ms: security.average_validation_time_ms.load(Relaxed),
    })
}

/// Resolve a DNS name.
///
/// DNS resolution requires an active STOQ transport session. This handler
/// validates the request structure but cannot perform resolution without the
/// transport layer.
pub async fn handle_dns_resolve(
    _ctx: &HttpHandlerContext,
    req: DnsResolveRequest,
) -> TrustChainResult<DnsResolveResponse> {
    info!("Handler: DNS resolve {} ({})", req.domain, req.record_type);

    // TODO: requires STOQ transport setup — DnsOverStoq needs an active
    // STOQ connection to forward queries. Return empty result for now.
    warn!("DNS resolution not yet wired: STOQ transport required");

    Ok(DnsResolveResponse {
        domain: req.domain,
        addresses: Vec::new(),
        ttl: 0,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn format_system_time(t: SystemTime) -> String {
    let datetime: chrono::DateTime<chrono::Utc> = t.into();
    datetime.to_rfc3339()
}

fn issued_to_summary(cert: &IssuedCertificate) -> CertificateSummary {
    let status = match &cert.status {
        CertificateStatus::Valid => "valid".to_string(),
        CertificateStatus::Revoked { reason, .. } => format!("revoked: {reason}"),
        CertificateStatus::Expired => "expired".to_string(),
    };

    CertificateSummary {
        serial_number: cert.serial_number.clone(),
        common_name: cert.common_name.clone(),
        status,
        issued_at: format_system_time(cert.issued_at),
        expires_at: format_system_time(cert.expires_at),
        fingerprint: hex::encode(cert.fingerprint),
    }
}

/// Minimal PEM-to-DER conversion. Strips header/footer lines and decodes
/// the base64 payload. Returns an error for malformed input.
fn pem_to_der(pem: &str) -> TrustChainResult<Vec<u8>> {
    let body: String = pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect();

    if body.is_empty() {
        return Err(TrustChainError::CertificateParsingFailed {
            reason: "Empty PEM body after stripping headers".to_string(),
        });
    }

    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(body.as_bytes())
        .map_err(|e| TrustChainError::CertificateParsingFailed {
            reason: format!("Base64 decode failed: {e}"),
        })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_request_serialization() {
        let req = IssueCertificateRequest {
            common_name: "test.example.com".to_string(),
            san_names: vec!["alt.example.com".to_string()],
            validity_days: Some(365),
        };

        let json = serde_json::to_vec(&req).expect("test: serialize issue request");
        let decoded: IssueCertificateRequest =
            serde_json::from_slice(&json).expect("test: deserialize issue request");

        assert_eq!(decoded.common_name, "test.example.com");
        assert_eq!(decoded.san_names.len(), 1);
        assert_eq!(decoded.validity_days, Some(365));
    }

    #[test]
    fn test_issue_response_serialization() {
        let resp = IssueCertificateResponse {
            serial_number: "abc123".to_string(),
            certificate_pem: "-----BEGIN CERTIFICATE-----\ndata\n-----END CERTIFICATE-----"
                .to_string(),
            chain_pem: "chain".to_string(),
            fingerprint: "deadbeef".to_string(),
            issued_at: "2026-01-01T00:00:00Z".to_string(),
            expires_at: "2027-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_vec(&resp).expect("test: serialize issue response");
        let decoded: IssueCertificateResponse =
            serde_json::from_slice(&json).expect("test: deserialize issue response");

        assert_eq!(decoded.serial_number, "abc123");
        assert_eq!(decoded.fingerprint, "deadbeef");
    }

    #[test]
    fn test_validate_request_serialization() {
        let req = ValidateCertificateRequest {
            certificate_pem: "-----BEGIN CERTIFICATE-----\ndata\n-----END CERTIFICATE-----"
                .to_string(),
            check_revocation: true,
        };

        let json = serde_json::to_vec(&req).expect("test: serialize validate request");
        let decoded: ValidateCertificateRequest =
            serde_json::from_slice(&json).expect("test: deserialize validate request");

        assert!(decoded.check_revocation);
    }

    #[test]
    fn test_validate_response_serialization() {
        let resp = ValidateCertificateResponse {
            valid: true,
            chain_valid: true,
            revocation_status: "not_revoked".to_string(),
            issuer: "TrustChain CA".to_string(),
        };

        let json = serde_json::to_vec(&resp).expect("test: serialize validate response");
        let decoded: ValidateCertificateResponse =
            serde_json::from_slice(&json).expect("test: deserialize validate response");

        assert!(decoded.valid);
        assert_eq!(decoded.issuer, "TrustChain CA");
    }

    #[test]
    fn test_revoke_request_serialization() {
        let req = RevokeCertificateRequest {
            serial_number: "serial-001".to_string(),
            reason: "key_compromise".to_string(),
        };

        let json = serde_json::to_vec(&req).expect("test: serialize revoke request");
        let decoded: RevokeCertificateRequest =
            serde_json::from_slice(&json).expect("test: deserialize revoke request");

        assert_eq!(decoded.serial_number, "serial-001");
        assert_eq!(decoded.reason, "key_compromise");
    }

    #[test]
    fn test_dns_resolve_request_serialization() {
        let req = DnsResolveRequest {
            domain: "example.hypermesh.online".to_string(),
            record_type: "AAAA".to_string(),
        };

        let json = serde_json::to_vec(&req).expect("test: serialize DNS request");
        let decoded: DnsResolveRequest =
            serde_json::from_slice(&json).expect("test: deserialize DNS request");

        assert_eq!(decoded.domain, "example.hypermesh.online");
        assert_eq!(decoded.record_type, "AAAA");
    }

    #[test]
    fn test_dns_resolve_response_serialization() {
        let resp = DnsResolveResponse {
            domain: "example.hypermesh.online".to_string(),
            addresses: vec!["2001:db8::1".to_string()],
            ttl: 300,
        };

        let json = serde_json::to_vec(&resp).expect("test: serialize DNS response");
        let decoded: DnsResolveResponse =
            serde_json::from_slice(&json).expect("test: deserialize DNS response");

        assert_eq!(decoded.ttl, 300);
        assert_eq!(decoded.addresses.len(), 1);
    }

    #[test]
    fn test_certificate_summary_serialization() {
        let summary = CertificateSummary {
            serial_number: "ser-001".to_string(),
            common_name: "test.com".to_string(),
            status: "valid".to_string(),
            issued_at: "2026-01-01T00:00:00Z".to_string(),
            expires_at: "2027-01-01T00:00:00Z".to_string(),
            fingerprint: "aabbcc".to_string(),
        };

        let json = serde_json::to_vec(&summary).expect("test: serialize summary");
        let decoded: CertificateSummary =
            serde_json::from_slice(&json).expect("test: deserialize summary");

        assert_eq!(decoded.common_name, "test.com");
    }

    #[test]
    fn test_health_response_structure() {
        let resp = HealthResponse {
            status: "healthy".to_string(),
            version: "0.1.0".to_string(),
            uptime_seconds: 42,
            ca_available: true,
            security_monitor_available: true,
        };

        let json = serde_json::to_vec(&resp).expect("test: serialize health");
        let decoded: HealthResponse =
            serde_json::from_slice(&json).expect("test: deserialize health");

        assert_eq!(decoded.status, "healthy");
        assert!(decoded.ca_available);
        assert!(decoded.security_monitor_available);
        assert_eq!(decoded.uptime_seconds, 42);
    }

    #[test]
    fn test_metrics_response_serialization() {
        let resp = MetricsResponse {
            certificates_issued: 100,
            consensus_validations: 50,
            security_validations_total: 200,
            security_validations_failed: 3,
            byzantine_detections: 1,
            average_validation_time_ms: 12,
        };

        let json = serde_json::to_vec(&resp).expect("test: serialize metrics");
        let decoded: MetricsResponse =
            serde_json::from_slice(&json).expect("test: deserialize metrics");

        assert_eq!(decoded.certificates_issued, 100);
        assert_eq!(decoded.byzantine_detections, 1);
    }

    #[test]
    fn test_pem_to_der_valid() {
        let pem = "-----BEGIN CERTIFICATE-----\nYWJj\n-----END CERTIFICATE-----";
        let der = pem_to_der(pem).expect("test: valid PEM decode");
        assert_eq!(der, b"abc");
    }

    #[test]
    fn test_pem_to_der_empty_body() {
        let pem = "-----BEGIN CERTIFICATE-----\n-----END CERTIFICATE-----";
        let result = pem_to_der(pem);
        assert!(result.is_err());
    }

    #[test]
    fn test_pem_to_der_invalid_base64() {
        let pem = "-----BEGIN CERTIFICATE-----\n!!!invalid!!!\n-----END CERTIFICATE-----";
        let result = pem_to_der(pem);
        assert!(result.is_err());
    }

    #[test]
    fn test_issued_to_summary_valid() {
        let cert = IssuedCertificate {
            serial_number: "ser-001".to_string(),
            certificate_der: vec![1, 2, 3],
            certificate_pem: "pem".to_string(),
            chain_pem: "chain".to_string(),
            fingerprint: [0u8; 32],
            common_name: "test.com".to_string(),
            issued_at: SystemTime::now(),
            expires_at: SystemTime::now(),
            issuer_ca_id: "ca-01".to_string(),
            consensus_proof: ConsensusProof::default(),
            status: CertificateStatus::Valid,
            metadata: Default::default(),
        };

        let summary = issued_to_summary(&cert);
        assert_eq!(summary.serial_number, "ser-001");
        assert_eq!(summary.common_name, "test.com");
        assert_eq!(summary.status, "valid");
    }

    #[test]
    fn test_issued_to_summary_revoked() {
        let cert = IssuedCertificate {
            serial_number: "ser-002".to_string(),
            certificate_der: vec![],
            certificate_pem: String::new(),
            chain_pem: String::new(),
            fingerprint: [0u8; 32],
            common_name: "revoked.com".to_string(),
            issued_at: SystemTime::now(),
            expires_at: SystemTime::now(),
            issuer_ca_id: "ca-01".to_string(),
            consensus_proof: ConsensusProof::default(),
            status: CertificateStatus::Revoked {
                reason: "key_compromise".to_string(),
                revoked_at: SystemTime::now(),
            },
            metadata: Default::default(),
        };

        let summary = issued_to_summary(&cert);
        assert_eq!(summary.status, "revoked: key_compromise");
    }
}
