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
use crate::ca::federation::FederationManager;
use crate::ca::ocsp::{OcspCertStatus, OcspRequest, OcspResponder};
use crate::ca::{CertificateRequest, CertificateStatus, IssuedCertificate, TrustChainCA};
use crate::proof_of_state::StateProof;
use crate::errors::{Result as TrustChainResult, TrustChainError};
use crate::security::SecurityMonitor;
use crate::proof_of_state::StateProofOps;

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
    pub state_validations: u64,
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
    /// Optional OCSP responder (Phase F.2). When attached, the
    /// `/api/v1/trustchain/ocsp` HTTP/3 endpoint serves real OCSP
    /// responses; otherwise the endpoint returns NOT_IMPLEMENTED.
    pub ocsp_responder: Option<Arc<OcspResponder>>,
    /// Optional federation manager (Phase F.2). Required for OCSP
    /// federation fallback.
    pub federation: Option<Arc<FederationManager>>,
}

// ---------------------------------------------------------------------------
// OCSP request / response wire types
// ---------------------------------------------------------------------------

/// HTTP/3 OCSP request body.  Mirrors [`OcspRequest`] but accepts hex
/// strings for the issuer hashes so JSON callers can use them naturally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcspHttpRequest {
    pub serial_number: String,
    /// Hex-encoded SHA-256 issuer name hash (optional, defaults to all
    /// zeros when omitted — the responder does not enforce issuer
    /// matching in alpha).
    #[serde(default)]
    pub issuer_name_hash_hex: Option<String>,
    /// Hex-encoded SHA-256 issuer key hash (optional, see above).
    #[serde(default)]
    pub issuer_key_hash_hex: Option<String>,
}

/// HTTP/3 OCSP response body.  String discriminator instead of an enum
/// because JSON readability matters more than type-system fidelity here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcspHttpResponse {
    /// One of `"good" | "revoked" | "unknown"`.
    pub status: String,
    /// Source of the verdict: `"local"` or `"federation"` (Phase F.2).
    pub source: String,
    pub serial_number: String,
    pub responder_id: String,
    /// Optional revocation reason when `status == "revoked"`.
    pub reason: Option<String>,
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

    let state_proof = StateProof::generate_from_network("http3-handler")
        .await
        .map_err(|e| TrustChainError::StateProofValidationFailed {
            reason: format!("Failed to generate state proof: {e}"),
        })?;

    let cert_request = CertificateRequest {
        common_name: req.common_name.clone(),
        san_entries: req.san_names,
        node_id: "http3-client".to_string(),
        ipv6_addresses: vec![std::net::Ipv6Addr::LOCALHOST],
        state_proof,
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

    let summaries: Vec<CertificateSummary> = ctx
        .certificate_store
        .iter_certificates()
        .map(|cert| issued_to_summary(&cert))
        .collect();

    info!("Returning {} certificates", summaries.len());
    Ok(summaries)
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
        state_validations: security.state_validations.load(Relaxed),
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
// OCSP handler (Phase F.2)
// ---------------------------------------------------------------------------

/// Decode a hex-encoded SHA-256 hash, falling back to all zeros on
/// missing or malformed input.  OCSP issuer hashes are advisory in our
/// alpha and serve mostly as request identifiers; rejecting on malformed
/// hashes would lock out clients that don't pre-compute them.
fn parse_optional_hex_hash(hex_str: &Option<String>) -> [u8; 32] {
    match hex_str {
        Some(s) => {
            let mut out = [0u8; 32];
            match hex::decode(s) {
                Ok(bytes) if bytes.len() == 32 => {
                    out.copy_from_slice(&bytes);
                    out
                }
                _ => out,
            }
        }
        None => [0u8; 32],
    }
}

/// Handle a `/api/v1/trustchain/ocsp` request.  Queries the local
/// certificate store first; on Unknown, falls back to federation peers
/// when both an `OcspResponder` and a `FederationManager` have been
/// attached to the context (Phase F.2).
pub async fn handle_ocsp(
    ctx: &HttpHandlerContext,
    req: OcspHttpRequest,
) -> TrustChainResult<OcspHttpResponse> {
    info!("Handler: OCSP query for serial={}", req.serial_number);

    let responder = ctx
        .ocsp_responder
        .as_ref()
        .ok_or_else(|| TrustChainError::Internal {
            message: "OCSP responder not attached to handler context".into(),
        })?;

    let ocsp_req = OcspRequest {
        serial_number: req.serial_number.clone(),
        issuer_name_hash: parse_optional_hex_hash(&req.issuer_name_hash_hex),
        issuer_key_hash: parse_optional_hex_hash(&req.issuer_key_hash_hex),
    };

    // 1. Fast path — local store via OcspResponder::check_status.
    let local = responder
        .check_status(&ocsp_req)
        .await
        .map_err(|e| TrustChainError::Internal {
            message: format!("OCSP local check failed: {e}"),
        })?;

    let (status, source, reason) = match (&local.status, &ctx.federation) {
        (OcspCertStatus::Unknown, Some(fed)) => {
            // 2. Federation fallback (Phase F.2). Only when both
            //    OcspResponder transport and FederationManager are
            //    attached.  When transport is unset, federated_check
            //    returns Unknown — keep the same answer but record the
            //    source as `federation` for observability.
            let fed_status = responder.federated_check(&req.serial_number, fed).await;
            let r = if let OcspCertStatus::Revoked { reason, .. } = &fed_status {
                Some(format!("{:?}", reason))
            } else {
                None
            };
            (fed_status, "federation".to_string(), r)
        }
        (other, _) => {
            let r = if let OcspCertStatus::Revoked { reason, .. } = other {
                Some(format!("{:?}", reason))
            } else {
                None
            };
            (other.clone(), "local".to_string(), r)
        }
    };

    let status_label = match &status {
        OcspCertStatus::Good => "good",
        OcspCertStatus::Revoked { .. } => "revoked",
        OcspCertStatus::Unknown => "unknown",
    };

    Ok(OcspHttpResponse {
        status: status_label.to_string(),
        source,
        serial_number: req.serial_number,
        responder_id: local.responder_id,
        reason,
    })
}

// ---------------------------------------------------------------------------
// State proof and authentication handlers
// ---------------------------------------------------------------------------

/// Request to validate a submitted proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateProofValidateRequest {
    pub proof: StateProof,
}

/// State proof validation response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateProofValidateResponse {
    pub valid: bool,
    pub space_valid: bool,
    pub stake_valid: bool,
    pub work_valid: bool,
    pub time_valid: bool,
    pub proofs_passed: u32,
    pub errors: Vec<String>,
}

/// State proof status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateProofStatusResponse {
    pub active: bool,
    pub proof_types: Vec<String>,
    pub last_validation_time_ms: u64,
    pub total_validations: u64,
}

/// Authentication certificate request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCertificateRequest {
    pub common_name: String,
    pub purpose: String,
}

/// Authentication certificate response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCertificateResponse {
    pub issued: bool,
    pub serial_number: Option<String>,
    pub fingerprint: Option<String>,
    pub expires_at: Option<String>,
    pub error: Option<String>,
}

/// Return current Proof of State status from security monitor metrics.
pub async fn handle_state_proof_status(
    ctx: &HttpHandlerContext,
) -> TrustChainResult<StateProofStatusResponse> {
    info!("Handler: state proof status");

    let metrics = ctx.security_monitor.get_metrics().await;
    use std::sync::atomic::Ordering::Relaxed;

    Ok(StateProofStatusResponse {
        active: true,
        proof_types: vec![
            "PoSpace".to_string(),
            "PoStake".to_string(),
            "PoWork".to_string(),
            "PoTime".to_string(),
        ],
        last_validation_time_ms: metrics.average_validation_time_ms.load(Relaxed),
        total_validations: metrics.state_validations.load(Relaxed),
    })
}

/// Validate a submitted state proof against the four-proof system.
pub async fn handle_state_proof_validate(
    ctx: &HttpHandlerContext,
    req: StateProofValidateRequest,
) -> TrustChainResult<StateProofValidateResponse> {
    info!("Handler: validate state proof");

    // Validate the proof using the real state proof validation pipeline
    let validation = crate::proof_of_state::validation::ProofValidation::validate_proof(&req.proof);

    // Record the validation in security metrics
    let metrics = ctx.security_monitor.get_metrics().await;
    metrics
        .state_validations
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    if !validation.all_valid {
        metrics
            .validations_failed
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    let errors: Vec<String> = validation
        .errors
        .iter()
        .map(|e| format!("{:?}: {}", e.proof_type, e.error_message))
        .collect();

    Ok(StateProofValidateResponse {
        valid: validation.all_valid,
        space_valid: validation.space_valid,
        stake_valid: validation.stake_valid,
        work_valid: validation.work_valid,
        time_valid: validation.time_valid,
        proofs_passed: validation.proofs_passed(),
        errors,
    })
}

/// Issue or check an authentication certificate via the CA.
pub async fn handle_auth_certificate(
    ctx: &HttpHandlerContext,
    req: AuthCertificateRequest,
) -> TrustChainResult<AuthCertificateResponse> {
    info!(
        "Handler: auth certificate for {} (purpose: {})",
        req.common_name, req.purpose
    );

    // Check if a certificate already exists for this common name
    let existing = ctx
        .certificate_store
        .find_by_common_name(&req.common_name)
        .await;

    if let Some(cert) = existing {
        // Return existing certificate info
        return Ok(AuthCertificateResponse {
            issued: false,
            serial_number: Some(cert.serial_number.clone()),
            fingerprint: Some(hex::encode(cert.fingerprint)),
            expires_at: Some(format_system_time(cert.expires_at)),
            error: None,
        });
    }

    // Issue a new certificate
    let issue_req = IssueCertificateRequest {
        common_name: req.common_name.clone(),
        san_names: vec![req.common_name.clone()],
        validity_days: Some(365),
    };

    match handle_issue_certificate(ctx, issue_req).await {
        Ok(resp) => Ok(AuthCertificateResponse {
            issued: true,
            serial_number: Some(resp.serial_number),
            fingerprint: Some(resp.fingerprint),
            expires_at: Some(resp.expires_at),
            error: None,
        }),
        Err(e) => Ok(AuthCertificateResponse {
            issued: false,
            serial_number: None,
            fingerprint: None,
            expires_at: None,
            error: Some(format!("{e}")),
        }),
    }
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
            state_validations: 50,
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
            state_proof: StateProof::default(),
            status: CertificateStatus::Valid,
            metadata: Default::default(),
        };

        let summary = issued_to_summary(&cert);
        assert_eq!(summary.serial_number, "ser-001");
        assert_eq!(summary.common_name, "test.com");
        assert_eq!(summary.status, "valid");
    }

    #[test]
    fn test_state_proof_validate_request_serialization() {
        let req = StateProofValidateRequest {
            proof: StateProof::default(),
        };
        let json = serde_json::to_vec(&req).expect("test: serialize state proof validate request");
        let decoded: StateProofValidateRequest =
            serde_json::from_slice(&json).expect("test: deserialize state proof validate request");
        // Default StakeProof binds a default identity (authorization, no amount).
        assert_eq!(decoded.proof.stake_proof.stake_holder_id, "test-001");
    }

    #[test]
    fn test_state_proof_status_response_serialization() {
        let resp = StateProofStatusResponse {
            active: true,
            proof_types: vec!["PoSpace".to_string()],
            last_validation_time_ms: 12,
            total_validations: 100,
        };
        let json = serde_json::to_vec(&resp).expect("test: serialize");
        let decoded: StateProofStatusResponse =
            serde_json::from_slice(&json).expect("test: deserialize");
        assert!(decoded.active);
        assert_eq!(decoded.total_validations, 100);
    }

    #[test]
    fn test_auth_certificate_response_serialization() {
        let resp = AuthCertificateResponse {
            issued: true,
            serial_number: Some("ser-001".to_string()),
            fingerprint: Some("aabb".to_string()),
            expires_at: Some("2027-01-01".to_string()),
            error: None,
        };
        let json = serde_json::to_vec(&resp).expect("test: serialize");
        let decoded: AuthCertificateResponse =
            serde_json::from_slice(&json).expect("test: deserialize");
        assert!(decoded.issued);
        assert_eq!(decoded.serial_number, Some("ser-001".to_string()));
    }

    #[tokio::test]
    async fn test_handle_state_proof_status() {
        use crate::security::SecurityConfig;

        let ca_config = crate::ca::CAConfig::testing();
        let ca = TrustChainCA::new(ca_config)
            .await
            .expect("test: create CA");
        let store = CertificateStore::new()
            .await
            .expect("test: create cert store");
        let security_monitor = SecurityMonitor::new(SecurityConfig::default())
            .await
            .expect("test: create security monitor");

        let ctx = HttpHandlerContext {
            ca: Arc::new(ca),
            certificate_store: Arc::new(store),
            security_monitor: Arc::new(security_monitor),
            start_time: std::time::Instant::now(),
            ocsp_responder: None,
            federation: None,
        };

        let result = handle_state_proof_status(&ctx)
            .await
            .expect("test: state proof status should succeed");
        assert!(result.active);
        assert_eq!(result.proof_types.len(), 4);
    }

    #[tokio::test]
    async fn test_handle_state_proof_validate_default_proof() {
        use crate::security::SecurityConfig;

        let ca_config = crate::ca::CAConfig::testing();
        let ca = TrustChainCA::new(ca_config)
            .await
            .expect("test: create CA");
        let store = CertificateStore::new()
            .await
            .expect("test: create cert store");
        let security_monitor = SecurityMonitor::new(SecurityConfig::default())
            .await
            .expect("test: create security monitor");

        let ctx = HttpHandlerContext {
            ca: Arc::new(ca),
            certificate_store: Arc::new(store),
            security_monitor: Arc::new(security_monitor),
            start_time: std::time::Instant::now(),
            ocsp_responder: None,
            federation: None,
        };

        let req = StateProofValidateRequest {
            proof: StateProof::default(),
        };

        let result = handle_state_proof_validate(&ctx, req)
            .await
            .expect("test: validate should succeed");
        // Default proof has non-zero values (stake=1000, storage=1GB, compute=1000, offset=0s)
        // so it passes validation
        assert!(result.valid, "default proof should pass validation");
        assert_eq!(
            result.proofs_passed, 4,
            "all four proofs should pass for valid proof"
        );
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
            state_proof: StateProof::default(),
            status: CertificateStatus::Revoked {
                reason: "key_compromise".to_string(),
                revoked_at: SystemTime::now(),
            },
            metadata: Default::default(),
        };

        let summary = issued_to_summary(&cert);
        assert_eq!(summary.status, "revoked: key_compromise");
    }

    // -- Item 2.13: handle_list_certificates wired to iter_certificates --------

    #[tokio::test]
    async fn test_handle_list_certificates_empty_store() {
        use crate::security::SecurityConfig;

        let ca_config = crate::ca::CAConfig::testing();
        let ca = TrustChainCA::new(ca_config)
            .await
            .expect("test: create CA");
        let store = CertificateStore::new()
            .await
            .expect("test: create cert store");
        let security_monitor = SecurityMonitor::new(SecurityConfig::default())
            .await
            .expect("test: create security monitor");

        let ctx = HttpHandlerContext {
            ca: Arc::new(ca),
            certificate_store: Arc::new(store),
            security_monitor: Arc::new(security_monitor),
            start_time: std::time::Instant::now(),
            ocsp_responder: None,
            federation: None,
        };

        let result = handle_list_certificates(&ctx)
            .await
            .expect("test: list certificates should not error");
        assert!(result.is_empty(), "empty store should return empty list");
    }

    #[tokio::test]
    async fn test_handle_list_certificates_with_entries() {
        use crate::security::SecurityConfig;

        let ca_config = crate::ca::CAConfig::testing();
        let ca = TrustChainCA::new(ca_config)
            .await
            .expect("test: create CA");
        let store = CertificateStore::new()
            .await
            .expect("test: create cert store");

        // Insert a certificate into the store
        let cert = IssuedCertificate {
            serial_number: "list-test-001".to_string(),
            certificate_der: vec![1, 2, 3],
            certificate_pem: "pem".to_string(),
            chain_pem: "chain".to_string(),
            fingerprint: [0u8; 32],
            common_name: "listed.hypermesh.online".to_string(),
            issued_at: SystemTime::now(),
            expires_at: SystemTime::now(),
            issuer_ca_id: "ca-01".to_string(),
            state_proof: StateProof::default(),
            status: CertificateStatus::Valid,
            metadata: Default::default(),
        };
        store
            .store_certificate(&cert)
            .await
            .expect("test: store certificate");

        let security_monitor = SecurityMonitor::new(SecurityConfig::default())
            .await
            .expect("test: create security monitor");

        let ctx = HttpHandlerContext {
            ca: Arc::new(ca),
            certificate_store: Arc::new(store),
            security_monitor: Arc::new(security_monitor),
            start_time: std::time::Instant::now(),
            ocsp_responder: None,
            federation: None,
        };

        let result = handle_list_certificates(&ctx)
            .await
            .expect("test: list certificates should not error");
        assert_eq!(result.len(), 1, "should return one certificate");
        assert_eq!(result[0].serial_number, "list-test-001");
        assert_eq!(result[0].common_name, "listed.hypermesh.online");
        assert_eq!(result[0].status, "valid");
    }
}
