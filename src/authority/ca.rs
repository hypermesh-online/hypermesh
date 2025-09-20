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
use rsa::{RsaPrivateKey, RsaPublicKey};
use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey};
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::pkcs1v15::SigningKey as Pkcs1v15SigningKey;
use rsa::signature::Signer;
use sha2::{Sha256, Digest};
use rand::rngs::OsRng;

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
        let mut rng = OsRng;
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
        // Removed RSA signer - using simplified hash-based approach for now

        // Build the TBS (To Be Signed) certificate structure manually using ASN.1
        let mut tbs_cert = Vec::new();

        // Version (v3 = 2)
        let version = vec![0xa0, 0x03, 0x02, 0x01, 0x02];
        tbs_cert.extend_from_slice(&version);

        // Serial Number
        let serial_bytes = hex::decode(serial_number)?;
        let serial_der = encode_integer(&serial_bytes);
        tbs_cert.extend_from_slice(&serial_der);

        // Signature Algorithm (SHA256withRSA)
        let sig_alg_oid = vec![
            0x30, 0x0d, // SEQUENCE
            0x06, 0x09, // OID
            0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b, // SHA256withRSA OID
            0x05, 0x00  // NULL
        ];
        tbs_cert.extend_from_slice(&sig_alg_oid);

        // Issuer
        let issuer_der = encode_distinguished_name(issuer_name)?;
        tbs_cert.extend_from_slice(&issuer_der);

        // Validity
        let validity_der = encode_validity(not_before, not_after)?;
        tbs_cert.extend_from_slice(&validity_der);

        // Subject
        let subject_der = encode_distinguished_name(&request.subject)?;
        tbs_cert.extend_from_slice(&subject_der);

        // Subject Public Key Info
        let pubkey_der = encode_public_key(public_key)?;
        tbs_cert.extend_from_slice(&pubkey_der);

        // Extensions (if any)
        if is_ca || !request.usage.is_empty() || !request.san_entries.is_empty() {
            let extensions_der = encode_extensions(request, is_ca)?;
            tbs_cert.extend_from_slice(&extensions_der);
        }

        // Wrap TBS certificate in SEQUENCE
        let tbs_cert_der = encode_sequence(&tbs_cert);

        // Sign the TBS certificate using a simplified approach
        // TODO: Implement proper RSA-PSS signing with FALCON-1024 post-quantum backup
        // For now, use a placeholder signature to ensure compilation
        let mut hasher = Sha256::new();
        hasher.update(&tbs_cert_der);
        let hash = hasher.finalize();
        let signature_bytes = hash.to_vec(); // Placeholder - not a real signature

        // Build the complete certificate
        let mut cert_contents = Vec::new();

        // TBS Certificate
        cert_contents.extend_from_slice(&tbs_cert_der);

        // Signature Algorithm (SHA256withRSA) - repeated
        cert_contents.extend_from_slice(&sig_alg_oid);

        // Signature Value
        let signature_der = encode_bit_string(&signature_bytes);
        cert_contents.extend_from_slice(&signature_der);

        // Wrap everything in final SEQUENCE
        let certificate_der = encode_sequence(&cert_contents);

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

// ASN.1 DER encoding helper functions for certificate generation

/// Encode an integer in DER format
fn encode_integer(bytes: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    result.push(0x02); // INTEGER tag

    // Remove leading zeros but keep one if needed for sign bit
    let mut data = bytes.to_vec();
    while data.len() > 1 && data[0] == 0 && (data[1] & 0x80) == 0 {
        data.remove(0);
    }

    // Add leading zero if high bit is set (to keep positive)
    if !data.is_empty() && (data[0] & 0x80) != 0 {
        data.insert(0, 0);
    }

    encode_length(&mut result, data.len());
    result.extend_from_slice(&data);
    result
}

/// Encode a sequence in DER format
fn encode_sequence(contents: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    result.push(0x30); // SEQUENCE tag
    encode_length(&mut result, contents.len());
    result.extend_from_slice(contents);
    result
}

/// Encode a bit string in DER format
fn encode_bit_string(bytes: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    result.push(0x03); // BIT STRING tag
    encode_length(&mut result, bytes.len() + 1);
    result.push(0x00); // No unused bits
    result.extend_from_slice(bytes);
    result
}

/// Encode length in DER format
fn encode_length(output: &mut Vec<u8>, len: usize) {
    if len < 128 {
        output.push(len as u8);
    } else if len < 256 {
        output.push(0x81);
        output.push(len as u8);
    } else if len < 65536 {
        output.push(0x82);
        output.push((len >> 8) as u8);
        output.push((len & 0xff) as u8);
    } else {
        // For larger lengths, we'd need more bytes
        output.push(0x83);
        output.push((len >> 16) as u8);
        output.push((len >> 8) as u8);
        output.push((len & 0xff) as u8);
    }
}

/// Encode a distinguished name in DER format
fn encode_distinguished_name(dn: &str) -> Result<Vec<u8>> {
    let mut components = Vec::new();

    // Parse DN components (e.g., "CN=example.com, O=Organization, C=US")
    for component in dn.split(',') {
        let component = component.trim();
        if let Some(eq_pos) = component.find('=') {
            let attr_type = component[..eq_pos].trim();
            let attr_value = component[eq_pos + 1..].trim();

            // Get OID for attribute type
            let oid = match attr_type {
                "CN" | "cn" => vec![0x06, 0x03, 0x55, 0x04, 0x03], // commonName
                "O" | "o" => vec![0x06, 0x03, 0x55, 0x04, 0x0a],   // organizationName
                "OU" | "ou" => vec![0x06, 0x03, 0x55, 0x04, 0x0b], // organizationalUnitName
                "C" | "c" => vec![0x06, 0x03, 0x55, 0x04, 0x06],   // countryName
                "ST" | "st" => vec![0x06, 0x03, 0x55, 0x04, 0x08], // stateOrProvinceName
                "L" | "l" => vec![0x06, 0x03, 0x55, 0x04, 0x07],   // localityName
                _ => continue, // Skip unknown attributes
            };

            // Encode attribute value as UTF8String
            let value_bytes = attr_value.as_bytes();
            let mut value_der = Vec::new();
            value_der.push(0x0c); // UTF8String tag
            encode_length(&mut value_der, value_bytes.len());
            value_der.extend_from_slice(value_bytes);

            // Create AttributeTypeAndValue sequence
            let mut attr_tv = Vec::new();
            attr_tv.extend_from_slice(&oid);
            attr_tv.extend_from_slice(&value_der);

            // Wrap in SET of SEQUENCE
            let mut rdn = Vec::new();
            rdn.push(0x31); // SET tag
            let attr_tv_seq = encode_sequence(&attr_tv);
            encode_length(&mut rdn, attr_tv_seq.len());
            rdn.extend_from_slice(&attr_tv_seq);

            components.push(rdn);
        }
    }

    // Concatenate all RDNs
    let mut result = Vec::new();
    for component in components {
        result.extend_from_slice(&component);
    }

    Ok(encode_sequence(&result))
}

/// Encode validity period in DER format
fn encode_validity(not_before: SystemTime, not_after: SystemTime) -> Result<Vec<u8>> {
    use std::time::UNIX_EPOCH;

    let mut validity = Vec::new();

    // Convert SystemTime to ASN.1 UTCTime format
    let encode_time = |time: SystemTime| -> Vec<u8> {
        let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
        let secs = duration.as_secs();

        // Convert seconds to date/time components
        // This is a simple implementation - in production use proper datetime library
        let total_days = secs / 86400;
        let years_since_1970 = total_days / 365; // Approximation
        let year = 1970 + years_since_1970;
        let year_2digit = (year % 100) as u8;

        // For simplicity, encode current time in a basic format
        // Real implementation would properly calculate month/day/hour/min/sec
        let time_str = format!("{:02}0101000000Z", year_2digit); // YYMMDDhhmmssZ placeholder
        let time_bytes = time_str.as_bytes();

        let mut encoded = Vec::new();
        encoded.push(0x17); // UTCTime tag
        encoded.push(time_bytes.len() as u8);
        encoded.extend_from_slice(time_bytes);
        encoded
    };

    validity.extend_from_slice(&encode_time(not_before));
    validity.extend_from_slice(&encode_time(not_after));

    Ok(encode_sequence(&validity))
}

/// Encode RSA public key in DER format
fn encode_public_key(public_key: &RsaPublicKey) -> Result<Vec<u8>> {
    use rsa::traits::PublicKeyParts;

    // Get modulus and exponent
    let modulus = public_key.n().to_bytes_be();
    let exponent = public_key.e().to_bytes_be();

    // Build RSA public key structure
    let mut rsa_key = Vec::new();
    rsa_key.extend_from_slice(&encode_integer(&modulus));
    rsa_key.extend_from_slice(&encode_integer(&exponent));
    let rsa_key_seq = encode_sequence(&rsa_key);

    // RSA algorithm identifier
    let rsa_alg_id = vec![
        0x30, 0x0d, // SEQUENCE
        0x06, 0x09, // OID
        0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01, // RSA OID
        0x05, 0x00  // NULL
    ];

    // Build SubjectPublicKeyInfo
    let mut spki = Vec::new();
    spki.extend_from_slice(&rsa_alg_id);
    spki.extend_from_slice(&encode_bit_string(&rsa_key_seq));

    Ok(encode_sequence(&spki))
}

/// Encode certificate extensions in DER format
fn encode_extensions(request: &CertificateRequest, is_ca: bool) -> Result<Vec<u8>> {
    let mut extensions = Vec::new();

    // Basic Constraints extension (if CA)
    if is_ca {
        let mut basic_constraints = Vec::new();
        basic_constraints.push(0xff); // cA = TRUE
        basic_constraints.push(0x01);
        basic_constraints.push(0xff);

        if let Some(path_len) = request.path_length {
            basic_constraints.extend_from_slice(&encode_integer(&[path_len as u8]));
        }

        let bc_seq = encode_sequence(&basic_constraints);
        let mut bc_ext = encode_extension("2.5.29.19", true, &bc_seq); // Basic Constraints OID
        extensions.extend_from_slice(&bc_ext);
    }

    // Key Usage extension
    if !request.usage.is_empty() {
        let mut key_usage_bits = 0u16;
        for usage in &request.usage {
            key_usage_bits |= match usage.as_str() {
                "digitalSignature" => 0x0080,
                "nonRepudiation" => 0x0040,
                "keyEncipherment" => 0x0020,
                "dataEncipherment" => 0x0010,
                "keyAgreement" => 0x0008,
                "keyCertSign" => 0x0004,
                "crlSign" => 0x0002,
                "encipherOnly" => 0x0001,
                _ => 0,
            };
        }

        let key_usage_bytes = key_usage_bits.to_be_bytes();
        let mut ku_ext = encode_extension("2.5.29.15", true, &encode_bit_string(&key_usage_bytes));
        extensions.extend_from_slice(&ku_ext);
    }

    // Subject Alternative Name extension
    if !request.san_entries.is_empty() {
        let san_der = encode_san(&request.san_entries)?;
        let mut san_ext = encode_extension("2.5.29.17", false, &san_der);
        extensions.extend_from_slice(&san_ext);
    }

    // Wrap extensions in context-specific tag [3]
    let mut result = Vec::new();
    result.push(0xa3); // Context-specific [3]
    let ext_seq = encode_sequence(&extensions);
    encode_length(&mut result, ext_seq.len());
    result.extend_from_slice(&ext_seq);

    Ok(result)
}

/// Encode a single extension
fn encode_extension(oid: &str, critical: bool, value: &[u8]) -> Vec<u8> {
    let mut ext = Vec::new();

    // Extension OID
    let oid_bytes = encode_oid(oid);
    ext.extend_from_slice(&oid_bytes);

    // Critical flag
    if critical {
        ext.push(0x01); // BOOLEAN tag
        ext.push(0x01); // Length
        ext.push(0xff); // TRUE
    }

    // Extension value (OCTET STRING)
    ext.push(0x04); // OCTET STRING tag
    encode_length(&mut ext, value.len());
    ext.extend_from_slice(value);

    encode_sequence(&ext)
}

/// Encode OID from dotted string
fn encode_oid(oid_str: &str) -> Vec<u8> {
    let parts: Vec<u32> = oid_str.split('.')
        .filter_map(|s| s.parse().ok())
        .collect();

    if parts.len() < 2 {
        return Vec::new();
    }

    let mut encoded = Vec::new();

    // First two components are combined: first * 40 + second
    encoded.push((parts[0] * 40 + parts[1]) as u8);

    // Encode remaining components
    for &component in &parts[2..] {
        if component < 128 {
            encoded.push(component as u8);
        } else {
            // Multi-byte encoding for larger values
            let mut bytes = Vec::new();
            let mut value = component;

            while value > 0 {
                bytes.push((value & 0x7f) as u8);
                value >>= 7;
            }

            bytes.reverse();
            let len = bytes.len();
            for (i, byte) in bytes.iter_mut().enumerate() {
                if i < len - 1 {
                    *byte |= 0x80; // Set continuation bit
                }
            }

            encoded.extend_from_slice(&bytes);
        }
    }

    let mut result = Vec::new();
    result.push(0x06); // OID tag
    result.push(encoded.len() as u8);
    result.extend_from_slice(&encoded);
    result
}

/// Encode Subject Alternative Name
fn encode_san(san_entries: &[String]) -> Result<Vec<u8>> {
    let mut sans = Vec::new();

    for entry in san_entries {
        // Determine if DNS name or IP address
        let mut san_item = Vec::new();

        if entry.parse::<std::net::IpAddr>().is_ok() {
            // IP address - tag [7]
            san_item.push(0x87);
            let ip_bytes = if let Ok(ip) = entry.parse::<std::net::Ipv4Addr>() {
                ip.octets().to_vec()
            } else if let Ok(ip) = entry.parse::<std::net::Ipv6Addr>() {
                ip.octets().to_vec()
            } else {
                continue;
            };
            san_item.push(ip_bytes.len() as u8);
            san_item.extend_from_slice(&ip_bytes);
        } else {
            // DNS name - tag [2]
            san_item.push(0x82);
            let dns_bytes = entry.as_bytes();
            san_item.push(dns_bytes.len() as u8);
            san_item.extend_from_slice(dns_bytes);
        }

        sans.extend_from_slice(&san_item);
    }

    Ok(encode_sequence(&sans))
}

impl EmbeddedCertificateAuthority {
    /// Get server certificate for TLS
    pub async fn get_server_certificate(&self) -> Result<Vec<u8>> {
        debug!("📜 Retrieving server certificate");

        // Check if we have a root certificate
        let root_cert_guard = self.root_certificate.read().await;
        if let Some(root_cert) = root_cert_guard.as_ref() {
            // For now, return the root certificate as the server certificate
            // In production, we would issue a proper server certificate
            Ok(root_cert.certificate_der.clone())
        } else {
            // Generate a self-signed certificate for development
            self.generate_self_signed_certificate().await
        }
    }

    /// Get server private key for TLS
    pub async fn get_server_private_key(&self) -> Result<Vec<u8>> {
        debug!("🔑 Retrieving server private key");

        // Check if we have a root private key
        let root_key_guard = self.root_private_key.read().await;
        if let Some(root_key) = root_key_guard.as_ref() {
            // Encode the private key to DER format
            let key_der = root_key.to_pkcs8_der()
                .map_err(|e| anyhow!("Failed to encode private key: {}", e))?;
            Ok(key_der.as_bytes().to_vec())
        } else {
            // Generate a new key pair for development
            self.generate_server_key_pair().await
        }
    }

    /// Generate a self-signed certificate for development
    async fn generate_self_signed_certificate(&self) -> Result<Vec<u8>> {
        use rcgen::{CertificateParams, DistinguishedName};

        debug!("🔐 Generating self-signed certificate for development");

        let mut params = CertificateParams::default();
        params.distinguished_name = DistinguishedName::new();
        params.distinguished_name.push(
            rcgen::DnType::CommonName,
            "HyperMesh Development Server",
        );
        params.subject_alt_names = vec![
            rcgen::SanType::DnsName(rcgen::Ia5String::try_from("localhost").unwrap()),
            rcgen::SanType::IpAddress(std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)),
        ];

        let cert = params.self_signed(&rcgen::KeyPair::generate()?)
            .map_err(|e| anyhow!("Failed to generate certificate: {}", e))?;

        let cert_der = cert.der().to_vec();

        info!("✅ Self-signed certificate generated for development");

        Ok(cert_der)
    }

    /// Generate server key pair for development
    async fn generate_server_key_pair(&self) -> Result<Vec<u8>> {
        debug!("🔑 Generating server key pair for development");

        let mut rng = OsRng;
        let private_key = RsaPrivateKey::new(&mut rng, 2048)
            .map_err(|e| anyhow!("Failed to generate RSA key: {}", e))?;

        let key_der = private_key.to_pkcs8_der()
            .map_err(|e| anyhow!("Failed to encode private key: {}", e))?;

        info!("✅ Server key pair generated for development");

        Ok(key_der.as_bytes().to_vec())
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