//! Embedded Certificate Authority Implementation
//!
//! Core certificate authority functionality with post-quantum support.

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use rsa::RsaPrivateKey;

use crate::config::CaConfig;
use crate::authority::crypto::PostQuantumCrypto;
use super::types::*;
use super::operations::{CertificateOperations, DefaultCertificateOperations};

/// Embedded Certificate Authority
pub struct EmbeddedCertificateAuthority {
    /// Configuration
    config: CaConfig,

    /// Post-quantum cryptography integration
    pqc_crypto: Arc<PostQuantumCrypto>,

    /// CA root certificate and key
    root_certificate: Arc<RwLock<Option<CaCertificate>>>,
    root_private_key: Arc<RwLock<Option<RsaPrivateKey>>>,

    /// Issued certificates registry
    issued_certificates: Arc<RwLock<HashMap<String, IssuedCertificate>>>,

    /// Certificate revocation list
    revocation_list: Arc<RwLock<HashMap<String, RevocationEntry>>>,

    /// CA statistics
    stats: Arc<RwLock<CaStats>>,

    /// Certificate operations handler
    operations: Arc<dyn CertificateOperations + Send + Sync>,
}

impl EmbeddedCertificateAuthority {
    /// Create new Certificate Authority
    pub async fn new(config: CaConfig) -> Result<Self> {
        info!("Initializing Embedded Certificate Authority");

        let pqc_crypto = Arc::new(PostQuantumCrypto::new()?);
        let operations = Arc::new(DefaultCertificateOperations);

        let ca = Self {
            config: config.clone(),
            pqc_crypto,
            root_certificate: Arc::new(RwLock::new(None)),
            root_private_key: Arc::new(RwLock::new(None)),
            issued_certificates: Arc::new(RwLock::new(HashMap::new())),
            revocation_list: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CaStats::default())),
            operations,
        };

        // Initialize root CA certificate if configured
        if config.auto_generate_root {
            ca.initialize_root_ca().await?;
        }

        Ok(ca)
    }

    /// Initialize root CA certificate
    async fn initialize_root_ca(&self) -> Result<()> {
        info!("Generating root CA certificate");

        // Generate RSA key pair
        let (private_key, public_key) =
            DefaultCertificateOperations::generate_key_pair(4096)?;

        // Create root certificate request
        let request = CertificateRequest {
            subject_dn: format!(
                "CN={}, O={}, C={}",
                self.config.root_cn,
                self.config.root_org,
                self.config.root_country
            ),
            common_name: self.config.root_cn.clone(),
            organization: Some(self.config.root_org.clone()),
            country: Some(self.config.root_country.clone()),
            validity_days: self.config.root_validity_days,
            key_usage: KeyUsage {
                digital_signature: true,
                key_cert_sign: true,
                crl_sign: true,
                ..Default::default()
            },
            extended_key_usage: ExtendedKeyUsage::default(),
            ..Default::default()
        };

        // Self-sign root certificate
        let root_cert = self.operations.issue_certificate(&request, &private_key)?;

        // Store root certificate and key
        *self.root_certificate.write().await = Some(root_cert);
        *self.root_private_key.write().await = Some(private_key);

        info!("Root CA certificate generated successfully");
        Ok(())
    }

    /// Issue a new certificate
    pub async fn issue_certificate(
        &self,
        request: CertificateRequest,
    ) -> Result<IssuedCertificate> {
        debug!("Issuing certificate for: {}", request.common_name);

        // Get root key
        let root_key = self.root_private_key.read().await;
        let root_key = root_key.as_ref()
            .ok_or_else(|| anyhow!("Root CA not initialized"))?;

        // Issue certificate
        let certificate = self.operations.issue_certificate(&request, root_key)?;

        // Create fingerprint
        let fingerprint = self.operations.create_fingerprint(&certificate.certificate_der);

        // Create issued certificate record
        let issued_cert = IssuedCertificate {
            certificate: certificate.clone(),
            issued_at: SystemTime::now(),
            requestor: request.common_name.clone(),
            status: CertificateStatus::Active,
            domains: request.dns_names.clone(),
            ip_addresses: request.ip_addresses.clone(),
            fingerprint: fingerprint.clone(),
            pq_algorithm: request.pq_algorithm,
            renewal_count: 0,
            last_validated: None,
        };

        // Store in registry
        self.issued_certificates.write().await.insert(
            certificate.serial_number.clone(),
            issued_cert.clone(),
        );

        // Update statistics
        self.stats.write().await.certificates_issued += 1;

        info!(
            "Certificate issued: {} (fingerprint: {})",
            certificate.serial_number,
            fingerprint
        );

        Ok(issued_cert)
    }

    /// Validate a certificate
    pub async fn validate_certificate(
        &self,
        certificate_der: &[u8],
    ) -> Result<CertificateValidationResult> {
        debug!("Validating certificate");

        // Update statistics
        self.stats.write().await.validation_requests += 1;

        // Perform validation
        let result = self.operations.validate_certificate(certificate_der)?;

        if !result.is_valid {
            self.stats.write().await.validation_failures += 1;
        }

        // Update last validated timestamp if certificate is in registry
        if let Some(info) = &result.certificate_info {
            let mut certificates = self.issued_certificates.write().await;
            if let Some(cert) = certificates.get_mut(&info.serial_number) {
                cert.last_validated = Some(SystemTime::now());
            }
        }

        Ok(result)
    }

    /// Revoke a certificate
    pub async fn revoke_certificate(
        &self,
        serial_number: String,
        reason: RevocationReason,
    ) -> Result<()> {
        info!(
            "Revoking certificate {} for reason: {}",
            serial_number,
            reason.description()
        );

        // Check if certificate exists
        let mut certificates = self.issued_certificates.write().await;
        let cert = certificates.get_mut(&serial_number)
            .ok_or_else(|| anyhow!("Certificate not found"))?;

        // Update certificate status
        cert.status = CertificateStatus::Revoked;

        // Add to revocation list
        let entry = RevocationEntry {
            serial_number: serial_number.clone(),
            revoked_at: SystemTime::now(),
            reason,
        };

        self.revocation_list.write().await.insert(
            serial_number.clone(),
            entry,
        );

        // Update statistics
        self.stats.write().await.certificates_revoked += 1;

        Ok(())
    }

    /// Renew a certificate
    pub async fn renew_certificate(
        &self,
        serial_number: String,
    ) -> Result<IssuedCertificate> {
        info!("Renewing certificate: {}", serial_number);

        // Get existing certificate
        let certificates = self.issued_certificates.read().await;
        let existing = certificates.get(&serial_number)
            .ok_or_else(|| anyhow!("Certificate not found"))?;

        // Create renewal request
        let request = CertificateRequest {
            subject_dn: existing.certificate.subject_dn.clone(),
            common_name: existing.requestor.clone(),
            dns_names: existing.domains.clone(),
            ip_addresses: existing.ip_addresses.clone(),
            pq_algorithm: existing.pq_algorithm.clone(),
            ..Default::default()
        };

        drop(certificates);

        // Issue new certificate
        let mut new_cert = self.issue_certificate(request).await?;
        new_cert.renewal_count = existing.renewal_count + 1;

        // Update statistics
        self.stats.write().await.certificates_renewed += 1;

        Ok(new_cert)
    }

    /// Get certificate by serial number
    pub async fn get_certificate(
        &self,
        serial_number: &str,
    ) -> Result<IssuedCertificate> {
        self.issued_certificates
            .read()
            .await
            .get(serial_number)
            .cloned()
            .ok_or_else(|| anyhow!("Certificate not found"))
    }

    /// List all active certificates
    pub async fn list_active_certificates(&self) -> Vec<IssuedCertificate> {
        self.issued_certificates
            .read()
            .await
            .values()
            .filter(|cert| cert.status == CertificateStatus::Active)
            .cloned()
            .collect()
    }

    /// Get CA statistics
    pub async fn get_stats(&self) -> CaStats {
        self.stats.read().await.clone()
    }

    /// Check if certificate is revoked
    pub async fn is_revoked(&self, serial_number: &str) -> bool {
        self.revocation_list.read().await.contains_key(serial_number)
    }

    /// Get certificates expiring soon
    pub async fn get_expiring_certificates(&self, days: u64) -> Vec<IssuedCertificate> {
        let certificates = self.issued_certificates.read().await;
        let mut expiring = Vec::new();

        for cert in certificates.values() {
            if cert.status == CertificateStatus::Active {
                if let Some(days_left) =
                    DefaultCertificateOperations::days_until_expiry(&cert.certificate)
                {
                    if days_left <= days {
                        expiring.push(cert.clone());
                    }
                }
            }
        }

        expiring
    }

    /// Export root certificate
    pub async fn export_root_certificate(&self) -> Result<Vec<u8>> {
        let root = self.root_certificate.read().await;
        root.as_ref()
            .map(|cert| cert.certificate_der.clone())
            .ok_or_else(|| anyhow!("Root CA not initialized"))
    }

    /// Cleanup expired certificates
    pub async fn cleanup_expired(&self) -> Result<usize> {
        let mut certificates = self.issued_certificates.write().await;
        let mut removed = 0;

        certificates.retain(|_, cert| {
            let expired = !DefaultCertificateOperations::check_expiry(&cert.certificate);
            if expired && cert.status == CertificateStatus::Active {
                removed += 1;
                false
            } else {
                true
            }
        });

        info!("Cleaned up {} expired certificates", removed);
        Ok(removed)
    }
}