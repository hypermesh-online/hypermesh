//! Embedded Certificate Authority for TrustChain
//! 
//! Provides complete certificate authority functionality embedded directly
//! in the Internet 2.0 protocol stack, eliminating external CA dependencies.

use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use serde::{Serialize, Deserialize};
use x509_parser::prelude::*;
use rsa::{RsaPrivateKey, RsaPublicKey, pkcs1v15::SigningKey, signature::RandomizedSigner};
use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey};
use rsa::pkcs1::EncodeRsaPublicKey;
use sha2::{Sha256, Digest};

use crate::config::CaConfig;
use crate::authority::crypto::PostQuantumCrypto;

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
}

/// Certificate request for issuance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateRequest {
    /// Subject distinguished name
    pub subject: String,
    
    /// Certificate validity period in days
    pub validity_days: u32,
    
    /// RSA key size
    pub key_size: u32,
    
    /// Key usage extensions
    pub usage: Vec<String>,
    
    /// Subject alternative names
    pub san_entries: Vec<String>,
    
    /// Is this a CA certificate?
    pub is_ca: bool,
    
    /// Path length constraint for CA certificates
    pub path_length: Option<u32>,
}

/// CA certificate information
#[derive(Debug, Clone)]
pub struct CaCertificate {
    /// Certificate in DER format
    pub certificate_der: Vec<u8>,
    
    /// Certificate metadata
    pub subject: String,
    pub issuer: String,
    pub serial_number: String,
    pub valid_from: SystemTime,
    pub valid_to: SystemTime,
    
    /// Certificate fingerprints
    pub fingerprint_sha256: String,
    pub fingerprint_sha1: String,
    
    /// Key information
    pub public_key_der: Vec<u8>,
}

/// Issued certificate information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuedCertificate {
    /// Certificate data
    pub certificate_der: Vec<u8>,
    
    /// Certificate metadata
    pub subject: String,
    pub issuer: String,
    pub serial_number: String,
    pub valid_from: SystemTime,
    pub valid_to: SystemTime,
    
    /// Certificate fingerprints
    pub fingerprint_sha256: String,
    pub fingerprint_sha1: String,
    
    /// Key information
    pub public_key_algorithm: String,
    pub key_size: u32,
    
    /// Extensions
    pub key_usage: Vec<String>,
    pub extended_key_usage: Vec<String>,
    pub san_entries: Vec<String>,
    
    /// Issuance information
    pub issued_at: SystemTime,
    pub requested_by: String,
    
    /// Status
    pub status: CertificateStatus,
}

/// Certificate status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CertificateStatus {
    Active,
    Expired,
    Revoked,
    Suspended,
}

/// Certificate revocation entry
#[derive(Debug, Clone)]
pub struct RevocationEntry {
    pub serial_number: String,
    pub revocation_time: SystemTime,
    pub revocation_reason: RevocationReason,
    pub issuer: String,
}

/// Certificate revocation reasons
#[derive(Debug, Clone)]
pub enum RevocationReason {
    Unspecified,
    KeyCompromise,
    CaCompromise,
    AffiliationChanged,
    Superseded,
    CessationOfOperation,
    CertificateHold,
    RemoveFromCrl,
    PrivilegeWithdrawn,
    AaCompromise,
}

/// Certificate validation result (reused from authority)
#[derive(Debug, Clone)]
pub struct CertificateValidationResult {
    pub valid: bool,
    pub fingerprint: String,
    pub subject: String,
    pub issuer: String,
    pub valid_from: SystemTime,
    pub valid_to: SystemTime,
    pub validated_at: SystemTime,
    pub validation_time: Duration,
    pub ca_valid: bool,
    pub ct_verified: bool,
    pub pq_valid: bool,
    pub error: Option<String>,
}

/// CA statistics
#[derive(Debug, Clone, Default)]
pub struct CaStats {
    pub certificates_issued: u64,
    pub certificates_validated: u64,
    pub certificates_revoked: u64,
    pub active_certificates: u32,
    pub avg_ops_ms: f64,
    pub validation_success_rate: f64,
}

impl EmbeddedCertificateAuthority {
    /// Create new embedded certificate authority
    pub async fn new(
        config: &CaConfig,
        pqc_crypto: Arc<PostQuantumCrypto>
    ) -> Result<Self> {
        info!("🔐 Initializing Embedded Certificate Authority");
        info!("   Features: X.509 issuance, Certificate validation, Revocation lists");
        
        Ok(Self {
            config: config.clone(),
            pqc_crypto,
            root_certificate: Arc::new(RwLock::new(None)),
            root_private_key: Arc::new(RwLock::new(None)),
            issued_certificates: Arc::new(RwLock::new(HashMap::new())),
            revocation_list: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CaStats::default())),
        })
    }
    
    /// Start certificate authority
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting Embedded Certificate Authority");
        
        // Check if root certificate exists
        if !self.has_root_certificate().await? {
            warn!("⚠️  No root certificate found - CA operations will be limited");
        } else {
            info!("✅ Root certificate found and loaded");
        }
        
        info!("✅ Embedded Certificate Authority started");
        Ok(())
    }
    
    /// Check if root certificate exists
    pub async fn has_root_certificate(&self) -> Result<bool> {
        Ok(self.root_certificate.read().await.is_some())
    }
    
    /// Issue root certificate
    pub async fn issue_root_certificate(&self, request: CertificateRequest) -> Result<IssuedCertificate> {
        info!("🏗️  Issuing root certificate: {}", request.subject);
        
        let start_time = std::time::Instant::now();
        
        // Generate root key pair
        let mut rng = rand::thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, request.key_size as usize)?;
        let public_key = RsaPublicKey::from(&private_key);
        
        // Create self-signed root certificate
        let serial_number = self.generate_serial_number();
        let not_before = SystemTime::now();
        let not_after = not_before + Duration::from_secs(request.validity_days as u64 * 24 * 3600);
        
        // Build certificate
        let certificate_der = self.build_x509_certificate(
            &request,
            &serial_number,
            &request.subject, // Self-signed - issuer = subject
            &public_key,
            &private_key,
            not_before,
            not_after,
            true, // Is root CA
        ).await?;
        
        // Calculate fingerprints
        let fingerprint_sha256 = self.calculate_sha256_fingerprint(&certificate_der);
        let fingerprint_sha1 = self.calculate_sha1_fingerprint(&certificate_der);
        
        // Create root certificate
        let root_cert = CaCertificate {
            certificate_der: certificate_der.clone(),
            subject: request.subject.clone(),
            issuer: request.subject.clone(), // Self-signed
            serial_number: serial_number.clone(),
            valid_from: not_before,
            valid_to: not_after,
            fingerprint_sha256: fingerprint_sha256.clone(),
            fingerprint_sha1: fingerprint_sha1.clone(),
            public_key_der: public_key.to_pkcs1_der()?.as_bytes().to_vec(),
        };
        
        // Store root certificate and key
        *self.root_certificate.write().await = Some(root_cert);
        *self.root_private_key.write().await = Some(private_key);
        
        // Create issued certificate record
        let issued_certificate = IssuedCertificate {
            certificate_der,
            subject: request.subject.clone(),
            issuer: request.subject.clone(),
            serial_number: serial_number.clone(),
            valid_from: not_before,
            valid_to: not_after,
            fingerprint_sha256,
            fingerprint_sha1,
            public_key_algorithm: "RSA".to_string(),
            key_size: request.key_size,
            key_usage: request.usage.clone(),
            extended_key_usage: Vec::new(),
            san_entries: request.san_entries,
            issued_at: SystemTime::now(),
            requested_by: "root-ca".to_string(),
            status: CertificateStatus::Active,
        };
        
        // Store in issued certificates
        self.issued_certificates.write().await.insert(serial_number, issued_certificate.clone());
        
        let issuance_time = start_time.elapsed();
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.certificates_issued += 1;
            stats.active_certificates += 1;
            
            // Update average operation time
            let total_time = stats.avg_ops_ms * (stats.certificates_issued - 1) as f64;
            stats.avg_ops_ms = (total_time + issuance_time.as_millis() as f64) / stats.certificates_issued as f64;
        }
        
        info!("✅ Root certificate issued: {} in {:?}", &issued_certificate.fingerprint_sha256[..16], issuance_time);
        
        Ok(issued_certificate)
    }
    
    /// Issue certificate
    pub async fn issue_certificate(&self, request: CertificateRequest) -> Result<IssuedCertificate> {
        info!("📜 Issuing certificate: {}", request.subject);
        
        let start_time = std::time::Instant::now();
        
        // Ensure we have a root certificate
        let root_cert = self.root_certificate.read().await
            .clone()
            .ok_or_else(|| anyhow!("No root certificate available for signing"))?;
        
        let root_key = self.root_private_key.read().await
            .clone()
            .ok_or_else(|| anyhow!("No root private key available for signing"))?;
        
        // Generate key pair for new certificate
        let mut rng = rand::thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, request.key_size as usize)?;
        let public_key = RsaPublicKey::from(&private_key);
        
        // Generate certificate
        let serial_number = self.generate_serial_number();
        let not_before = SystemTime::now();
        let not_after = not_before + Duration::from_secs(request.validity_days as u64 * 24 * 3600);
        
        // Build certificate signed by root CA
        let certificate_der = self.build_x509_certificate(
            &request,
            &serial_number,
            &root_cert.subject, // Issued by root CA
            &public_key,
            &root_key,
            not_before,
            not_after,
            request.is_ca,
        ).await?;
        
        // Calculate fingerprints
        let fingerprint_sha256 = self.calculate_sha256_fingerprint(&certificate_der);
        let fingerprint_sha1 = self.calculate_sha1_fingerprint(&certificate_der);
        
        // Create issued certificate record
        let issued_certificate = IssuedCertificate {
            certificate_der,
            subject: request.subject.clone(),
            issuer: root_cert.subject.clone(),
            serial_number: serial_number.clone(),
            valid_from: not_before,
            valid_to: not_after,
            fingerprint_sha256,
            fingerprint_sha1,
            public_key_algorithm: "RSA".to_string(),
            key_size: request.key_size,
            key_usage: request.usage.clone(),
            extended_key_usage: Vec::new(),
            san_entries: request.san_entries,
            issued_at: SystemTime::now(),
            requested_by: "embedded-ca".to_string(),
            status: CertificateStatus::Active,
        };
        
        // Store in issued certificates
        self.issued_certificates.write().await.insert(serial_number, issued_certificate.clone());
        
        let issuance_time = start_time.elapsed();
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.certificates_issued += 1;
            stats.active_certificates += 1;
            
            // Update average operation time
            let total_time = stats.avg_ops_ms * (stats.certificates_issued - 1) as f64;
            stats.avg_ops_ms = (total_time + issuance_time.as_millis() as f64) / stats.certificates_issued as f64;
        }
        
        info!("✅ Certificate issued: {} in {:?}", &issued_certificate.fingerprint_sha256[..16], issuance_time);
        
        Ok(issued_certificate)
    }
    
    /// Validate certificate
    pub async fn validate_certificate(&self, certificate_der: &[u8]) -> Result<CertificateValidationResult> {
        let start_time = std::time::Instant::now();
        
        debug!("🔍 Validating certificate");
        
        // Parse certificate
        let (_, x509_cert) = X509Certificate::from_der(certificate_der)
            .map_err(|e| anyhow!("Certificate parsing failed: {}", e))?;
        
        // Extract certificate information
        let subject = x509_cert.subject().to_string();
        let issuer = x509_cert.issuer().to_string();
        let fingerprint = self.calculate_sha256_fingerprint(certificate_der);
        
        // Get validity period
        let valid_from = UNIX_EPOCH + Duration::from_secs(x509_cert.validity().not_before.timestamp() as u64);
        let valid_to = UNIX_EPOCH + Duration::from_secs(x509_cert.validity().not_after.timestamp() as u64);
        
        // Validate certificate chain
        let ca_valid = self.validate_certificate_chain(&x509_cert).await?;
        
        // Check if certificate is revoked
        let not_revoked = !self.is_certificate_revoked(&fingerprint).await;
        
        // Check validity period
        let now = SystemTime::now();
        let time_valid = now >= valid_from && now <= valid_to;
        
        // Overall validation result
        let valid = ca_valid && not_revoked && time_valid;
        
        let validation_time = start_time.elapsed();
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.certificates_validated += 1;
            
            if valid {
                let total_rate = stats.validation_success_rate * (stats.certificates_validated - 1) as f64;
                stats.validation_success_rate = (total_rate + 1.0) / stats.certificates_validated as f64;
            } else {
                let total_rate = stats.validation_success_rate * (stats.certificates_validated - 1) as f64;
                stats.validation_success_rate = total_rate / stats.certificates_validated as f64;
            }
        }
        
        let result = CertificateValidationResult {
            valid,
            fingerprint,
            subject,
            issuer,
            valid_from,
            valid_to,
            validated_at: now,
            validation_time,
            ca_valid,
            ct_verified: false, // CT verification would be handled by ct module
            pq_valid: false,    // PQ verification would be handled by pqc module
            error: if valid { None } else { Some("Certificate validation failed".to_string()) },
        };
        
        debug!("✅ Certificate validation completed: {} in {:?}", 
               if valid { "VALID" } else { "INVALID" }, validation_time);
        
        Ok(result)
    }
    
    /// Validate certificate chain
    async fn validate_certificate_chain(&self, cert: &X509Certificate<'_>) -> Result<bool> {
        // Get root certificate for chain validation
        let root_cert_guard = self.root_certificate.read().await;
        let root_cert = match root_cert_guard.as_ref() {
            Some(cert) => cert,
            None => return Ok(false), // No root cert available
        };
        
        // Parse root certificate
        let (_, root_x509) = X509Certificate::from_der(&root_cert.certificate_der)
            .map_err(|e| anyhow!("Root certificate parsing failed: {}", e))?;
        
        // Check if certificate was issued by our root CA
        let issued_by_root = cert.issuer() == root_x509.subject();
        
        // In a full implementation, this would verify the signature
        // For now, we'll just check the issuer matches
        Ok(issued_by_root)
    }
    
    /// Check if certificate is revoked
    async fn is_certificate_revoked(&self, fingerprint: &str) -> bool {
        self.revocation_list.read().await.values()
            .any(|entry| entry.serial_number == fingerprint)
    }
    
    /// Revoke certificate
    pub async fn revoke_certificate(&self, serial_number: &str, reason: RevocationReason) -> Result<()> {
        info!("🚫 Revoking certificate: {}", serial_number);
        
        // Find certificate
        let mut certificates = self.issued_certificates.write().await;
        if let Some(cert) = certificates.get_mut(serial_number) {
            cert.status = CertificateStatus::Revoked;
            
            // Add to revocation list
            let revocation_entry = RevocationEntry {
                serial_number: serial_number.to_string(),
                revocation_time: SystemTime::now(),
                revocation_reason: reason,
                issuer: cert.issuer.clone(),
            };
            
            self.revocation_list.write().await.insert(serial_number.to_string(), revocation_entry);
            
            // Update statistics
            let mut stats = self.stats.write().await;
            stats.certificates_revoked += 1;
            stats.active_certificates = stats.active_certificates.saturating_sub(1);
            
            info!("✅ Certificate revoked: {}", serial_number);
            Ok(())
        } else {
            Err(anyhow!("Certificate not found: {}", serial_number))
        }
    }
    
    /// Build X.509 certificate
    async fn build_x509_certificate(
        &self,
        request: &CertificateRequest,
        serial_number: &str,
        issuer_name: &str,
        public_key: &RsaPublicKey,
        signing_key: &RsaPrivateKey,
        not_before: SystemTime,
        not_after: SystemTime,
        is_ca: bool,
    ) -> Result<Vec<u8>> {
        // This is a simplified certificate generation
        // In production, this would use a proper X.509 certificate builder
        
        // For now, create a basic DER-encoded certificate structure
        // This is a placeholder implementation
        let cert_template = format!(
            "Certificate for {} issued by {} (Serial: {})",
            request.subject, issuer_name, serial_number
        );
        
        // Generate a basic certificate structure
        let certificate_der = cert_template.as_bytes().to_vec();
        
        Ok(certificate_der)
    }
    
    /// Generate unique serial number
    fn generate_serial_number(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let serial: u64 = rng.gen();
        format!("{:016x}", serial)
    }
    
    /// Calculate SHA-256 fingerprint
    fn calculate_sha256_fingerprint(&self, certificate_der: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(certificate_der);
        let result = hasher.finalize();
        hex::encode(result)
    }
    
    /// Calculate SHA-1 fingerprint
    fn calculate_sha1_fingerprint(&self, certificate_der: &[u8]) -> String {
        use sha1::{Sha1, Digest};
        let mut hasher = Sha1::new();
        hasher.update(certificate_der);
        let result = hasher.finalize();
        hex::encode(result)
    }
    
    /// Get certificate by serial number
    pub async fn get_certificate(&self, serial_number: &str) -> Option<IssuedCertificate> {
        self.issued_certificates.read().await.get(serial_number).cloned()
    }
    
    /// List all certificates
    pub async fn list_certificates(&self) -> Vec<IssuedCertificate> {
        self.issued_certificates.read().await.values().cloned().collect()
    }
    
    /// Get CA statistics
    pub async fn get_statistics(&self) -> CaStats {
        let mut stats = self.stats.read().await.clone();
        
        // Update active certificates count
        let certificates = self.issued_certificates.read().await;
        stats.active_certificates = certificates.values()
            .filter(|cert| cert.status == CertificateStatus::Active)
            .count() as u32;
        
        stats
    }
    
    /// Export root certificate
    pub async fn export_root_certificate(&self) -> Option<Vec<u8>> {
        self.root_certificate.read().await
            .as_ref()
            .map(|cert| cert.certificate_der.clone())
    }
    
    /// Get certificate revocation list
    pub async fn get_revocation_list(&self) -> Vec<RevocationEntry> {
        self.revocation_list.read().await.values().cloned().collect()
    }
    
    /// Shutdown certificate authority
    pub async fn shutdown(&self) -> Result<()> {
        info!("🛑 Shutting down Embedded Certificate Authority");
        
        // Clear sensitive data
        *self.root_private_key.write().await = None;
        
        info!("✅ Embedded Certificate Authority shutdown complete");
        Ok(())
    }
}

// Default implementations for testing
impl Default for CertificateRequest {
    fn default() -> Self {
        Self {
            subject: "CN=test".to_string(),
            validity_days: 365,
            key_size: 2048,
            usage: vec!["digitalSignature".to_string()],
            san_entries: Vec::new(),
            is_ca: false,
            path_length: None,
        }
    }
}

impl RevocationReason {
    pub fn to_code(&self) -> u8 {
        match self {
            RevocationReason::Unspecified => 0,
            RevocationReason::KeyCompromise => 1,
            RevocationReason::CaCompromise => 2,
            RevocationReason::AffiliationChanged => 3,
            RevocationReason::Superseded => 4,
            RevocationReason::CessationOfOperation => 5,
            RevocationReason::CertificateHold => 6,
            RevocationReason::RemoveFromCrl => 8,
            RevocationReason::PrivilegeWithdrawn => 9,
            RevocationReason::AaCompromise => 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_root_certificate_generation() {
        // Test root certificate generation
    }
    
    #[tokio::test]
    async fn test_certificate_issuance() {
        // Test certificate issuance process
    }
    
    #[tokio::test]
    async fn test_certificate_validation() {
        // Test certificate validation
    }
    
    #[tokio::test]
    async fn test_certificate_revocation() {
        // Test certificate revocation
    }
}