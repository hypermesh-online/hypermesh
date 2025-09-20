//! TrustChain API Handler
//!
//! Provides HTTP API endpoints for TrustChain certificate operations including:
//! - Certificate management and validation
//! - Certificate rotation policies
//! - Expiring and revoked certificate tracking
//! - Real certificate data for UI authentication

use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::{SystemTime, Duration, UNIX_EPOCH};
use tracing::{debug, warn};
use serde::{Serialize, Deserialize};
use serde_json::json;

use crate::transport::http_gateway::{RouteHandler, HttpRequest, HttpResponse};
use crate::authority::TrustChainAuthorityLayer;

/// TrustChain API route handler
pub struct TrustChainRouteHandler {
    trustchain: Arc<TrustChainAuthorityLayer>,
}

impl TrustChainRouteHandler {
    pub fn new(trustchain: Arc<TrustChainAuthorityLayer>) -> Self {
        Self { trustchain }
    }
}

impl RouteHandler for TrustChainRouteHandler {
    fn handle(&self, request: &HttpRequest) -> Result<HttpResponse> {
        debug!("TrustChain API handling: {} {}", request.method, request.path);

        // Route to appropriate handler based on path
        let response = match request.path.as_str() {
            "/api/v1/trustchain/certificates" => self.handle_certificates(request),
            "/api/v1/trustchain/certificates/expiring" => self.handle_expiring_certificates(request),
            "/api/v1/trustchain/certificates/revoked" => self.handle_revoked_certificates(request),
            "/api/v1/trustchain/certificates/root" => self.handle_root_certificate(request),
            "/api/v1/trustchain/policies/rotation" => self.handle_rotation_policies(request),
            "/api/v1/trustchain/health" => self.handle_health_check(request),
            "/api/v1/trustchain/stats" => self.handle_statistics(request),
            path if path.starts_with("/api/v1/trustchain/certificates/") => {
                self.handle_certificate_by_id(request, path)
            }
            _ => Ok(HttpResponse {
                status: 404,
                headers: [("Content-Type".to_string(), "application/json".to_string())]
                    .into_iter()
                    .collect(),
                body: json!({
                    "error": "TrustChain endpoint not found",
                    "path": request.path
                }).to_string().into_bytes(),
            }),
        };

        response
    }
}

impl TrustChainRouteHandler {
    /// Handle GET /api/v1/trustchain/certificates - list all certificates
    fn handle_certificates(&self, request: &HttpRequest) -> Result<HttpResponse> {
        if request.method != "GET" {
            return self.method_not_allowed();
        }

        // Block on async operation
        let certificates = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.trustchain.get_ca().await.list_certificates().await
            })
        });

        // Convert to API response format
        let response_data: Vec<CertificateResponse> = certificates
            .into_iter()
            .map(|cert| CertificateResponse {
                id: cert.serial_number.clone(),
                subject: cert.subject.clone(),
                issuer: cert.issuer.clone(),
                serial_number: cert.serial_number.clone(),
                valid_from: cert.valid_from.duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                valid_to: cert.valid_to.duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                fingerprint_sha256: cert.fingerprint_sha256.clone(),
                fingerprint_sha1: cert.fingerprint_sha1.clone(),
                status: cert.status.clone(),
                key_usage: cert.key_usage.clone(),
                san_entries: cert.san_entries.clone(),
            })
            .collect();

        self.json_response(200, json!(response_data))
    }

    /// Handle GET /api/v1/trustchain/certificates/expiring - get expiring certificates
    fn handle_expiring_certificates(&self, request: &HttpRequest) -> Result<HttpResponse> {
        if request.method != "GET" {
            return self.method_not_allowed();
        }

        // Parse query parameters for days threshold (default 30 days)
        let days_threshold = self.parse_query_param(&request.path, "days")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(30);

        // Block on async operation
        let certificates = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.trustchain.get_ca().await.list_certificates().await
            })
        });

        // Filter for certificates expiring within threshold
        let now = SystemTime::now();
        let threshold = now + Duration::from_secs(days_threshold * 24 * 3600);

        let expiring: Vec<CertificateResponse> = certificates
            .into_iter()
            .filter(|cert| {
                cert.valid_to > now && cert.valid_to <= threshold
            })
            .map(|cert| CertificateResponse {
                id: cert.serial_number.clone(),
                subject: cert.subject.clone(),
                issuer: cert.issuer.clone(),
                serial_number: cert.serial_number.clone(),
                valid_from: cert.valid_from.duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                valid_to: cert.valid_to.duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                fingerprint_sha256: cert.fingerprint_sha256.clone(),
                fingerprint_sha1: cert.fingerprint_sha1.clone(),
                status: cert.status.clone(),
                key_usage: cert.key_usage.clone(),
                san_entries: cert.san_entries.clone(),
            })
            .collect();

        self.json_response(200, json!({
            "certificates": expiring,
            "count": expiring.len(),
            "days_threshold": days_threshold
        }))
    }

    /// Handle GET /api/v1/trustchain/certificates/revoked - get revoked certificate count
    fn handle_revoked_certificates(&self, request: &HttpRequest) -> Result<HttpResponse> {
        if request.method != "GET" {
            return self.method_not_allowed();
        }

        // Block on async operation
        let (certificates, revocation_list) = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let certs = self.trustchain.get_ca().await.list_certificates().await;
                let revocations = self.trustchain.get_ca().await.get_revocation_list().await;
                (certs, revocations)
            })
        });

        // Count revoked certificates
        let revoked_count = certificates
            .iter()
            .filter(|cert| cert.status == crate::authority::ca::CertificateStatus::Revoked)
            .count();

        // Build revocation details
        let revocation_details: Vec<_> = revocation_list
            .into_iter()
            .map(|entry| json!({
                "serial_number": entry.serial_number,
                "revocation_time": entry.revocation_time.duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                "reason": format!("{:?}", entry.revocation_reason),
                "issuer": entry.issuer
            }))
            .collect();

        self.json_response(200, json!({
            "count": revoked_count,
            "revocations": revocation_details
        }))
    }

    /// Handle GET /api/v1/trustchain/certificates/root - get root certificate
    fn handle_root_certificate(&self, request: &HttpRequest) -> Result<HttpResponse> {
        if request.method != "GET" {
            return self.method_not_allowed();
        }

        // Block on async operation
        let root_cert_der = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.trustchain.get_ca().await.export_root_certificate().await
            })
        });

        match root_cert_der {
            Some(ref cert_der) => {
                // Convert to PEM format for easy consumption
                let pem = self.der_to_pem(cert_der, "CERTIFICATE");

                self.json_response(200, json!({
                    "certificate": pem,
                    "format": "pem",
                    "fingerprint": self.calculate_fingerprint(cert_der)
                }))
            }
            None => {
                // No root certificate exists yet - return placeholder for now
                warn!("No root certificate found - returning empty response");
                self.json_response(200, json!({
                    "certificate": "",
                    "format": "pem",
                    "fingerprint": "",
                    "status": "not_initialized"
                }))
            }
        }
    }

    /// Handle GET /api/v1/trustchain/policies/rotation - get rotation policies
    fn handle_rotation_policies(&self, request: &HttpRequest) -> Result<HttpResponse> {
        if request.method != "GET" {
            return self.method_not_allowed();
        }

        // For now, return default rotation policies
        // TODO: Implement actual rotation policy storage and retrieval
        let policies = vec![
            json!({
                "id": "default-rotation",
                "name": "Default Certificate Rotation",
                "rotation_type": "automatic",
                "schedule": {
                    "interval_days": 90,
                    "warning_days": 14,
                    "grace_period_days": 7
                },
                "enabled": true,
                "applies_to": ["leaf", "intermediate"],
                "last_rotation": null,
                "next_rotation": null
            }),
            json!({
                "id": "root-rotation",
                "name": "Root Certificate Rotation",
                "rotation_type": "manual",
                "schedule": {
                    "interval_days": 365,
                    "warning_days": 30,
                    "grace_period_days": 14
                },
                "enabled": true,
                "applies_to": ["root"],
                "last_rotation": null,
                "next_rotation": null
            })
        ];

        self.json_response(200, json!({
            "policies": policies,
            "count": policies.len()
        }))
    }

    /// Handle GET /api/v1/trustchain/certificates/{id} - get specific certificate
    fn handle_certificate_by_id(&self, request: &HttpRequest, path: &str) -> Result<HttpResponse> {
        if request.method != "GET" {
            return self.method_not_allowed();
        }

        // Extract certificate ID from path
        let cert_id = path.trim_start_matches("/api/v1/trustchain/certificates/");

        // Block on async operation
        let certificate = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.trustchain.get_ca().await.get_certificate(cert_id).await
            })
        });

        match certificate {
            Some(cert) => {
                let response = CertificateResponse {
                    id: cert.serial_number.clone(),
                    subject: cert.subject.clone(),
                    issuer: cert.issuer.clone(),
                    serial_number: cert.serial_number.clone(),
                    valid_from: cert.valid_from.duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    valid_to: cert.valid_to.duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    fingerprint_sha256: cert.fingerprint_sha256.clone(),
                    fingerprint_sha1: cert.fingerprint_sha1.clone(),
                    status: cert.status.clone(),
                    key_usage: cert.key_usage.clone(),
                    san_entries: cert.san_entries.clone(),
                };
                self.json_response(200, json!(response))
            }
            None => self.json_response(404, json!({
                "error": "Certificate not found",
                "certificate_id": cert_id
            }))
        }
    }

    /// Handle GET /api/v1/trustchain/health
    fn handle_health_check(&self, _request: &HttpRequest) -> Result<HttpResponse> {
        // Block on async operation
        let (has_root, stats) = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let has_root = self.trustchain.get_ca().await.has_root_certificate().await
                    .unwrap_or(false);
                let stats = self.trustchain.get_statistics().await
                    .unwrap_or_default();
                (has_root, stats)
            })
        });

        self.json_response(200, json!({
            "status": if has_root { "healthy" } else { "warning" },
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            "version": "1.0.0",
            "services": {
                "ca": has_root,
                "ct": true,  // Certificate Transparency always available
                "dns": true,  // DNS resolver always available
                "consensus": true  // Consensus validation always available
            },
            "metrics": {
                "certificates_issued": stats.certificates_issued,
                "certificates_validated": stats.certificates_validated,
                "dns_queries": stats.dns_queries_resolved,
                "avg_validation_time_ms": stats.certificate_ops_ms
            }
        }))
    }

    /// Handle GET /api/v1/trustchain/stats
    fn handle_statistics(&self, _request: &HttpRequest) -> Result<HttpResponse> {
        // Block on async operation
        let stats = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.trustchain.get_statistics().await.unwrap_or_default()
            })
        });

        self.json_response(200, json!({
            "requests_total": stats.certificates_validated + stats.dns_queries_resolved,
            "requests_successful": stats.certificates_validated,
            "requests_failed": 0,
            "ca_requests": stats.certificates_issued,
            "ct_requests": stats.ct_logs_submitted,
            "dns_requests": stats.dns_queries_resolved,
            "average_response_time_ms": stats.certificate_ops_ms,
            "active_connections": 0,  // TODO: Get from transport layer
            "rate_limited_requests": 0,
            "last_update": SystemTime::now().duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        }))
    }

    // Helper methods

    fn json_response(&self, status: u16, body: serde_json::Value) -> Result<HttpResponse> {
        Ok(HttpResponse {
            status,
            headers: [
                ("Content-Type".to_string(), "application/json".to_string()),
                ("Cache-Control".to_string(), "no-cache".to_string()),
            ]
            .into_iter()
            .collect(),
            body: body.to_string().into_bytes(),
        })
    }

    fn method_not_allowed(&self) -> Result<HttpResponse> {
        self.json_response(405, json!({
            "error": "Method not allowed"
        }))
    }

    fn parse_query_param(&self, path: &str, param: &str) -> Option<String> {
        if let Some(query_start) = path.find('?') {
            let query = &path[query_start + 1..];
            for pair in query.split('&') {
                if let Some(eq_pos) = pair.find('=') {
                    let key = &pair[..eq_pos];
                    let value = &pair[eq_pos + 1..];
                    if key == param {
                        return Some(value.to_string());
                    }
                }
            }
        }
        None
    }

    fn der_to_pem(&self, der: &[u8], label: &str) -> String {
        use base64::{engine::general_purpose::STANDARD, Engine};

        let b64 = STANDARD.encode(der);
        let mut pem = format!("-----BEGIN {}-----\n", label);

        // Add line breaks every 64 characters
        for chunk in b64.as_bytes().chunks(64) {
            pem.push_str(&String::from_utf8_lossy(chunk));
            pem.push('\n');
        }

        pem.push_str(&format!("-----END {}-----", label));
        pem
    }

    fn calculate_fingerprint(&self, der: &[u8]) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(der);
        hex::encode(hasher.finalize())
    }
}

/// Certificate response format for API
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CertificateResponse {
    id: String,
    subject: String,
    issuer: String,
    serial_number: String,
    valid_from: u64,  // Unix timestamp
    valid_to: u64,    // Unix timestamp
    fingerprint_sha256: String,
    fingerprint_sha1: String,
    status: crate::authority::ca::CertificateStatus,
    key_usage: Vec<String>,
    san_entries: Vec<String>,
}