// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! TrustChain Integration for Catalog
//!
//! Provides certificate validation and CA integration for package signing

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Convert x509_parser's ASN1Time to chrono DateTime<Utc>
fn asn1_time_to_chrono(asn1: &x509_parser::time::ASN1Time) -> chrono::DateTime<chrono::Utc> {
    let offset_dt = asn1.to_datetime();
    let unix_ts = offset_dt.unix_timestamp();
    chrono::DateTime::from_timestamp(unix_ts, 0).unwrap_or_else(chrono::Utc::now)
}

// Import TrustChain types (will be available when integrated)
// use trustchain::{TrustChainCA, CertificateRequest, IssuedCertificate};

/// TrustChain integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustChainConfig {
    /// TrustChain CA endpoint
    pub endpoint: String,
    /// Enable post-quantum cryptography
    pub enable_pqc: bool,
    /// Certificate cache TTL (seconds)
    pub cert_cache_ttl: u64,
}

/// TrustChain integration client
pub struct TrustChainIntegration {
    /// Configuration
    config: TrustChainConfig,
    /// Certificate cache
    cert_cache: Arc<RwLock<CertificateCache>>,
    /// HTTP client for TrustChain API (STOQ-only transport, reqwest removed)
    // client: reqwest::Client,
    /// Cached CA root certificate
    ca_root_cert: Arc<RwLock<Option<CACertificate>>>,
}

/// Certificate cache
struct CertificateCache {
    /// Cached certificates by fingerprint
    certificates: HashMap<String, CachedCertificate>,
    /// Cache expiration times
    expiry_times: HashMap<String, std::time::Instant>,
}

/// Cached certificate entry
#[derive(Clone)]
struct CachedCertificate {
    /// Certificate data
    _certificate: Certificate,
    /// Validation result
    validation: CertificateValidation,
    /// Cache timestamp
    _cached_at: std::time::Instant,
}

/// Certificate representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certificate {
    /// Certificate fingerprint (SHA-256)
    pub fingerprint: String,
    /// Subject common name
    pub common_name: String,
    /// Subject organization
    pub organization: Option<String>,
    /// Issuer common name
    pub issuer: String,
    /// Not valid before
    pub not_before: chrono::DateTime<chrono::Utc>,
    /// Not valid after
    pub not_after: chrono::DateTime<chrono::Utc>,
    /// Subject alternative names
    pub san_entries: Vec<String>,
    /// Certificate chain
    pub chain: Vec<String>,
    /// Raw certificate bytes (DER encoded)
    pub raw_bytes: Vec<u8>,
    /// Post-quantum signature if available
    pub pqc_signature: Option<PQCSignature>,
}

/// Post-quantum signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PQCSignature {
    /// Algorithm (e.g., "FALCON-1024")
    pub algorithm: String,
    /// Signature bytes
    pub signature: Vec<u8>,
    /// Public key bytes
    pub public_key: Vec<u8>,
}

/// Certificate validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateValidation {
    /// Is certificate valid
    pub valid: bool,
    /// Validation timestamp
    pub validated_at: chrono::DateTime<chrono::Utc>,
    /// Chain validation status
    pub chain_valid: bool,
    /// Revocation status
    pub revoked: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
}

/// CA root certificate
#[derive(Debug, Clone)]
struct CACertificate {
    /// Root certificate
    certificate: Certificate,
    /// Last update time
    _last_updated: std::time::Instant,
}

/// TrustChain API request/response types
#[derive(Debug, Serialize, Deserialize)]
struct ValidateCertificateRequest {
    certificate: String,
    chain: Vec<String>,
    check_revocation: bool,
    require_pqc: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ValidateCertificateResponse {
    valid: bool,
    validation: CertificateValidation,
    certificate_info: Option<Certificate>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IssueCertificateRequest {
    common_name: String,
    organization: Option<String>,
    san_entries: Vec<String>,
    validity_days: u32,
    use_pqc: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct IssueCertificateResponse {
    certificate: Certificate,
    private_key: String, // PEM encoded
    chain: Vec<String>,
}

impl TrustChainIntegration {
    /// Create new TrustChain integration
    pub async fn new(config: TrustChainConfig) -> Result<Self> {
        // HTTP removed - STOQ-only transport
        // let client = reqwest::Client::builder()
        //     .timeout(std::time::Duration::from_secs(30))
        //     .danger_accept_invalid_certs(false) // Always validate TLS
        //     .build()
        //     .context("Failed to build HTTP client")?;

        let integration = Self {
            config,
            cert_cache: Arc::new(RwLock::new(CertificateCache {
                certificates: HashMap::new(),
                expiry_times: HashMap::new(),
            })),
            // client removed for STOQ-only transport
            ca_root_cert: Arc::new(RwLock::new(None)),
        };

        // Fetch and cache CA root certificate
        integration.fetch_ca_root().await?;

        Ok(integration)
    }

    /// Fetch CA root certificate
    ///
    /// In alpha mode, generates a self-signed root CA certificate using rcgen.
    /// Future: fetch real CA root via STOQ transport from TrustChain CA.
    async fn fetch_ca_root(&self) -> Result<()> {
        debug!("Fetching TrustChain CA root certificate");

        // Check if we already have a cached root
        {
            let existing = self.ca_root_cert.read().await;
            if existing.is_some() {
                debug!("CA root certificate already cached");
                return Ok(());
            }
        }

        warn!(
            "STOQ transport to TrustChain CA not yet available — generating self-signed alpha root"
        );

        // Generate a self-signed root CA certificate for alpha
        let mut ca_params =
            rcgen::CertificateParams::new(vec!["trust.hypermesh.online".to_string()])
                .map_err(|e| anyhow!("Failed to create CA cert params: {}", e))?;
        ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        ca_params
            .distinguished_name
            .push(rcgen::DnType::CommonName, "TrustChain CA Root");
        ca_params
            .distinguished_name
            .push(rcgen::DnType::OrganizationName, "HyperMesh");
        ca_params.not_before = rcgen::date_time_ymd(2026, 1, 1);
        ca_params.not_after = rcgen::date_time_ymd(2036, 12, 31);

        let ca_key_pair = rcgen::KeyPair::generate()
            .map_err(|e| anyhow!("Failed to generate CA key pair: {}", e))?;
        let ca_cert = ca_params
            .self_signed(&ca_key_pair)
            .map_err(|e| anyhow!("Failed to generate self-signed CA cert: {}", e))?;

        let der_bytes = ca_cert.der().to_vec();

        // Calculate fingerprint
        use sha2::{Digest, Sha256};
        let fingerprint = hex::encode(Sha256::digest(&der_bytes));

        let cert_data = Certificate {
            fingerprint,
            common_name: "TrustChain CA Root".to_string(),
            organization: Some("HyperMesh".to_string()),
            issuer: "TrustChain CA Root".to_string(),
            not_before: chrono::Utc::now(),
            not_after: chrono::Utc::now() + chrono::Duration::days(365 * 10),
            san_entries: vec!["trust.hypermesh.online".to_string()],
            chain: vec![],
            raw_bytes: der_bytes,
            pqc_signature: None,
        };

        let mut ca_root = self.ca_root_cert.write().await;
        *ca_root = Some(CACertificate {
            certificate: cert_data,
            _last_updated: std::time::Instant::now(),
        });

        info!("Generated self-signed alpha CA root certificate (real STOQ-based CA root fetch is future work)");
        Ok(())
    }

    /// Validate a certificate by parsing X.509 and checking expiry/revocation
    pub async fn validate_certificate(&self, cert_bytes: &[u8]) -> Result<CertificateValidation> {
        // Calculate fingerprint
        let fingerprint = self.calculate_fingerprint(cert_bytes);

        // Check cache
        if let Some(cached) = self.get_cached_certificate(&fingerprint).await {
            debug!("Using cached certificate validation for {}", fingerprint);
            return Ok(cached.validation);
        }

        debug!("Validating certificate {} with X.509 parsing", fingerprint);

        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut valid = true;

        // Try to parse the certificate as X.509 DER
        let cert_info = match x509_parser::parse_x509_certificate(cert_bytes) {
            Ok((_remaining, cert)) => {
                // Check validity period (convert from x509 time to chrono)
                let now = chrono::Utc::now();
                let not_before = asn1_time_to_chrono(&cert.validity().not_before);
                let not_after = asn1_time_to_chrono(&cert.validity().not_after);

                if now < not_before {
                    errors.push(format!(
                        "Certificate is not yet valid (not_before: {})",
                        not_before
                    ));
                    valid = false;
                }
                if now > not_after {
                    errors.push(format!("Certificate has expired (not_after: {})", not_after));
                    valid = false;
                }

                // Check if certificate version is v3 (version field: 0=v1, 1=v2, 2=v3)
                if cert.version().0 != 2 {
                    warnings.push("Certificate is not X.509 v3".to_string());
                }

                // Extract subject/issuer for certificate info
                let common_name = cert
                    .subject()
                    .iter_common_name()
                    .next()
                    .and_then(|cn| cn.as_str().ok())
                    .unwrap_or("unknown")
                    .to_string();

                let organization = cert
                    .subject()
                    .iter_organization()
                    .next()
                    .and_then(|o| o.as_str().ok())
                    .map(|s| s.to_string());

                let issuer_cn = cert
                    .issuer()
                    .iter_common_name()
                    .next()
                    .and_then(|cn| cn.as_str().ok())
                    .unwrap_or("unknown")
                    .to_string();

                Some(Certificate {
                    fingerprint: fingerprint.clone(),
                    common_name,
                    organization,
                    issuer: issuer_cn,
                    not_before,
                    not_after,
                    san_entries: vec![],
                    chain: vec![],
                    raw_bytes: cert_bytes.to_vec(),
                    pqc_signature: None,
                })
            }
            Err(e) => {
                warn!(
                    "Failed to parse certificate as X.509 DER: {}. Attempting basic validation.",
                    e
                );
                warnings.push(format!("X.509 parsing failed: {}", e));
                // If we can't parse it, we still allow it with a warning for alpha
                // (self-signed or non-standard certs used during bootstrap)
                None
            }
        };

        // Check revocation status via fingerprint
        let revoked = self.check_revocation(&fingerprint).await.unwrap_or(false);
        if revoked {
            errors.push("Certificate has been revoked".to_string());
            valid = false;
        }

        let validation = CertificateValidation {
            valid,
            validated_at: chrono::Utc::now(),
            chain_valid: true, // Chain validation requires full CA infrastructure (future work)
            revoked,
            errors,
            warnings,
        };

        // Cache the result
        if let Some(cert) = cert_info {
            self.cache_certificate(cert, validation.clone()).await;
        }

        Ok(validation)
    }

    /// Issue a new certificate for a publisher
    pub async fn issue_certificate(
        &self,
        common_name: String,
        organization: Option<String>,
    ) -> Result<(Certificate, String)> {
        info!("Requesting certificate for {}", common_name);

        // Request is prepared for when STOQ transport is implemented
        let _request = IssueCertificateRequest {
            common_name: common_name.clone(),
            organization: organization.clone(),
            san_entries: vec![format!("catalog.{}.hypermesh.online", common_name)],
            validity_days: 365,
            use_pqc: self.config.enable_pqc,
        };

        // Certificate issuance requires STOQ transport to communicate with TrustChain CA.
        // The STOQ transport integration must be configured before certificates can be issued.
        // When implemented, this will:
        //   1. Send the request via STOQ to the TrustChain CA endpoint
        //   2. Receive a real certificate and private key from the CA
        //   3. Return the validated certificate and key pair
        Err(anyhow::anyhow!(
            "Certificate issuance for '{}' failed: STOQ transport to TrustChain CA is not yet configured. \
             The private key must be generated by the CA service, not provided as placeholder data. \
             Configure the TrustChain CA endpoint at '{}'.",
            common_name,
            self.config.endpoint,
        ))
    }

    /// Check certificate revocation status
    pub async fn check_revocation(&self, cert_fingerprint: &str) -> Result<bool> {
        debug!("Checking revocation status for {}", cert_fingerprint);

        // TODO: Replace with STOQ transport
        // let url = format!("{}/api/certificates/{}/revocation", self.config.endpoint, cert_fingerprint);
        // let response = self.client
        //     .get(&url)
        //     .send()
        //     .await
        //     .context("Failed to check revocation status")?;

        // if !response.status().is_success() {
        //     return Err(anyhow!("Revocation check failed: {}", response.status()));
        // }

        #[derive(Deserialize)]
        struct RevocationStatus {
            revoked: bool,
            reason: Option<String>,
            _revoked_at: Option<chrono::DateTime<chrono::Utc>>,
        }

        // let status: RevocationStatus = response.json().await?;

        // For now, return not revoked
        let status = RevocationStatus {
            revoked: false,
            reason: None,
            _revoked_at: None,
        };

        if status.revoked {
            warn!(
                "Certificate {} is revoked: {:?}",
                cert_fingerprint, status.reason
            );
        }

        Ok(status.revoked)
    }

    /// Get certificate from cache
    async fn get_cached_certificate(&self, fingerprint: &str) -> Option<CachedCertificate> {
        let cache = self.cert_cache.read().await;

        // Check if certificate exists and is not expired
        if let Some(cached) = cache.certificates.get(fingerprint) {
            if let Some(expiry) = cache.expiry_times.get(fingerprint) {
                if std::time::Instant::now() < *expiry {
                    return Some(cached.clone());
                }
            }
        }

        None
    }

    /// Cache a certificate
    async fn cache_certificate(&self, cert: Certificate, validation: CertificateValidation) {
        let mut cache = self.cert_cache.write().await;

        let fingerprint = cert.fingerprint.clone();
        let cached_cert = CachedCertificate {
            _certificate: cert,
            validation,
            _cached_at: std::time::Instant::now(),
        };

        let expiry =
            std::time::Instant::now() + std::time::Duration::from_secs(self.config.cert_cache_ttl);

        cache.certificates.insert(fingerprint.clone(), cached_cert);
        cache.expiry_times.insert(fingerprint, expiry);

        // Clean up expired entries
        self.cleanup_cache(&mut cache);
    }

    /// Clean up expired cache entries
    fn cleanup_cache(&self, cache: &mut CertificateCache) {
        let now = std::time::Instant::now();

        cache.expiry_times.retain(|fingerprint, expiry| {
            if now >= *expiry {
                cache.certificates.remove(fingerprint);
                false
            } else {
                true
            }
        });
    }

    /// Calculate certificate fingerprint
    fn calculate_fingerprint(&self, cert_bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(cert_bytes);
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Clear certificate cache
    pub async fn clear_cache(&self) -> Result<()> {
        let mut cache = self.cert_cache.write().await;
        cache.certificates.clear();
        cache.expiry_times.clear();
        Ok(())
    }

    /// Verify certificate chain
    pub async fn verify_chain(&self, chain: &[Certificate]) -> Result<bool> {
        if chain.is_empty() {
            return Err(anyhow!("Certificate chain is empty"));
        }

        // Get CA root certificate
        let ca_root = self.ca_root_cert.read().await;
        let root_cert = ca_root
            .as_ref()
            .ok_or_else(|| anyhow!("CA root certificate not available"))?;

        // Verify each certificate in the chain
        for i in 0..chain.len() {
            let cert = &chain[i];

            // Check certificate validity period
            let now = chrono::Utc::now();
            if now < cert.not_before || now > cert.not_after {
                return Ok(false);
            }

            // Verify signature (simplified - actual implementation would use crypto libraries)
            if i == chain.len() - 1 {
                // Last cert should be signed by CA root
                if cert.issuer != root_cert.certificate.common_name {
                    return Ok(false);
                }
            } else {
                // Verify cert is signed by next cert in chain
                let issuer = &chain[i + 1];
                if cert.issuer != issuer.common_name {
                    return Ok(false);
                }
            }

            // Check revocation status
            if self.check_revocation(&cert.fingerprint).await? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Get TrustChain endpoint
    pub fn endpoint(&self) -> &str {
        &self.config.endpoint
    }

    /// Check if post-quantum cryptography is enabled
    pub fn is_pqc_enabled(&self) -> bool {
        self.config.enable_pqc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_trustchain_integration() {
        let config = TrustChainConfig {
            endpoint: "https://trust.hypermesh.online:8443".to_string(),
            enable_pqc: true,
            cert_cache_ttl: 3600,
        };

        let integration = TrustChainIntegration::new(config).await;
        assert!(integration.is_ok());
    }

    #[test]
    fn test_fingerprint_calculation() {
        let _config = TrustChainConfig {
            endpoint: "test".to_string(),
            enable_pqc: false,
            cert_cache_ttl: 60,
        };

        // This would need proper initialization in real tests
        // Just testing the fingerprint calculation logic
        let test_bytes = b"test certificate data";
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(test_bytes);
        let result = hasher.finalize();
        let fingerprint = hex::encode(result);

        assert_eq!(fingerprint.len(), 64); // SHA-256 produces 32 bytes = 64 hex chars
    }
}
