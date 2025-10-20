//! Post-Quantum Certificate Integration
//!
//! Integrates FALCON-1024 signatures and Kyber encryption with X.509 certificates
//! for quantum-resistant certificate authority operations in TrustChain.

use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{Result, anyhow};
use tracing::{info, debug, warn, error};
use serde::{Serialize, Deserialize};

use rcgen::{Certificate as RcgenCertificate, CertificateParams, DnType, SanType, Ia5String};
use x509_parser::parse_x509_certificate;

use super::{
    FalconKeyPair, FalconPublicKey, FalconPrivateKey, FalconSignature,
    KyberKeyPair, KyberPublicKey, KyberPrivateKey, PostQuantumCrypto, PQCError
};

/// Post-quantum X.509 certificate with embedded FALCON-1024 public key
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PQCertificate {
    /// DER-encoded certificate
    pub certificate_der: Vec<u8>,
    /// Certificate serial number
    pub serial_number: String,
    /// Subject common name
    pub common_name: String,
    /// Subject alternative names
    pub san_entries: Vec<String>,
    /// FALCON-1024 public key embedded in certificate
    pub falcon_public_key: FalconPublicKey,
    /// Kyber public key for encryption (optional)
    pub kyber_public_key: Option<KyberPublicKey>,
    /// Certificate validity period
    pub not_before: SystemTime,
    pub not_after: SystemTime,
    /// Certificate fingerprint (SHA-256)
    pub fingerprint: [u8; 32],
    /// Issuer information
    pub issuer_ca_id: String,
    /// Post-quantum signature from CA
    pub ca_signature: FalconSignature,
    /// Certificate extensions specific to post-quantum crypto
    pub pq_extensions: PQCertificateExtensions,
}

/// Post-quantum specific certificate extensions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PQCertificateExtensions {
    /// FALCON-1024 algorithm parameters
    pub falcon_parameters: String,
    /// Kyber algorithm parameters (if present)
    pub kyber_parameters: Option<String>,
    /// Quantum security level
    pub quantum_security_level: u32,
    /// Hybrid signature support
    pub hybrid_signature_support: bool,
    /// Migration signature support for transition
    pub migration_support: bool,
    /// Post-quantum certificate version
    pub pq_cert_version: String,
}

/// Post-quantum certificate signing request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PQCSR {
    /// CSR in DER format
    pub csr_der: Vec<u8>,
    /// Subject information
    pub subject: CertificateSubject,
    /// FALCON-1024 public key to be certified
    pub falcon_public_key: FalconPublicKey,
    /// Optional Kyber public key for encryption
    pub kyber_public_key: Option<KyberPublicKey>,
    /// Self-signature with FALCON-1024 private key
    pub self_signature: FalconSignature,
    /// CSR creation timestamp
    pub created_at: SystemTime,
    /// Additional attributes
    pub attributes: Vec<CSRAttribute>,
}

/// Certificate subject information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CertificateSubject {
    pub common_name: String,
    pub organization: Option<String>,
    pub organizational_unit: Option<String>,
    pub country: Option<String>,
    pub state: Option<String>,
    pub locality: Option<String>,
    pub email: Option<String>,
}

/// CSR attribute for additional information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CSRAttribute {
    pub name: String,
    pub value: String,
}

/// Post-quantum certificate operations
pub struct PQCertificateManager {
    /// Post-quantum crypto handler
    pqc: PostQuantumCrypto,
    /// CA configuration
    ca_config: PQCAConfig,
}

/// Post-quantum CA configuration
#[derive(Clone, Debug)]
pub struct PQCAConfig {
    pub ca_id: String,
    pub ca_keypair: FalconKeyPair,
    pub ca_kyber_keypair: Option<KyberKeyPair>,
    pub cert_validity_days: u32,
    pub quantum_security_level: u32,
    pub enable_hybrid_signatures: bool,
    pub enable_migration_support: bool,
}

impl PQCertificateManager {
    /// Initialize post-quantum certificate manager
    pub async fn new(ca_config: PQCAConfig) -> Result<Self> {
        info!("🔐 Initializing post-quantum certificate manager for CA: {}", ca_config.ca_id);
        
        let pqc = PostQuantumCrypto::new()?;
        
        // Validate CA keypair
        if !pqc.falcon.validate_keypair(&ca_config.ca_keypair)? {
            return Err(anyhow!("Invalid CA FALCON-1024 keypair"));
        }
        
        if let Some(ref kyber_keypair) = ca_config.ca_kyber_keypair {
            if !pqc.kyber.validate_keypair(kyber_keypair)? {
                return Err(anyhow!("Invalid CA Kyber keypair"));
            }
        }
        
        info!("✅ Post-quantum certificate manager initialized");
        Ok(Self {
            pqc,
            ca_config,
        })
    }
    
    /// Generate post-quantum certificate signing request
    pub async fn generate_pq_csr(
        &self,
        subject: CertificateSubject,
        keypair: &FalconKeyPair,
        kyber_keypair: Option<&KyberKeyPair>,
        san_entries: Vec<String>,
    ) -> Result<PQCSR> {
        info!("🔏 Generating post-quantum CSR for: {}", subject.common_name);
        
        // Create certificate parameters (rcgen 0.13: returns Result)
        let mut params = CertificateParams::new(vec![subject.common_name.clone()])?;

        // Set subject information
        params.distinguished_name.push(DnType::CommonName, subject.common_name.clone());
        if let Some(ref org) = subject.organization {
            params.distinguished_name.push(DnType::OrganizationName, org.clone());
        }
        if let Some(ref country) = subject.country {
            params.distinguished_name.push(DnType::CountryName, country.clone());
        }
        
        // Add subject alternative names (rcgen 0.13: SanType uses Ia5String)
        for san in &san_entries {
            if san.contains('@') {
                params.subject_alt_names.push(SanType::Rfc822Name(
                    Ia5String::try_from(san.as_str())?
                ));
            } else if san.parse::<std::net::IpAddr>().is_ok() {
                params.subject_alt_names.push(SanType::IpAddress(san.parse().unwrap()));
            } else {
                params.subject_alt_names.push(SanType::DnsName(
                    Ia5String::try_from(san.as_str())?
                ));
            }
        }

        // Add FALCON-1024 public key as extension
        let falcon_extension = self.encode_falcon_public_key_extension(&keypair.public_key)?;
        params.custom_extensions.push(falcon_extension);

        // Add Kyber public key if present
        if let Some(kyber_kp) = kyber_keypair {
            let kyber_extension = self.encode_kyber_public_key_extension(&kyber_kp.public_key)?;
            params.custom_extensions.push(kyber_extension);
        }

        // rcgen 0.13: Generate key pair and create CSR
        let key_pair = rcgen::KeyPair::generate()?;
        let csr = params.serialize_request(&key_pair)?;
        let csr_der = csr.der().to_vec();

        // Create self-signature with FALCON-1024
        let self_signature = self.pqc.sign_with_falcon(&csr_der, &keypair.private_key).await?;

        let pq_csr = PQCSR {
            csr_der,
            subject,
            falcon_public_key: keypair.public_key.clone(),
            kyber_public_key: kyber_keypair.map(|kp| kp.public_key.clone()),
            self_signature,
            created_at: SystemTime::now(),
            attributes: vec![
                CSRAttribute {
                    name: "pqc-algorithm".to_string(),
                    value: "FALCON-1024".to_string(),
                },
                CSRAttribute {
                    name: "quantum-security-level".to_string(),
                    value: "128".to_string(),
                },
            ],
        };
        
        info!("✅ Post-quantum CSR generated for: {}", pq_csr.subject.common_name);
        Ok(pq_csr)
    }
    
    /// Issue post-quantum certificate from CSR
    pub async fn issue_pq_certificate(&self, csr: PQCSR) -> Result<PQCertificate> {
        info!("🔐 Issuing post-quantum certificate for: {}", csr.subject.common_name);
        
        // Verify CSR self-signature
        let csr_signature_valid = self.pqc.verify_falcon_signature(
            &csr.csr_der,
            &csr.self_signature,
            &csr.falcon_public_key,
        ).await?;
        
        if !csr_signature_valid {
            return Err(anyhow!("CSR self-signature verification failed"));
        }
        
        // Create certificate parameters (rcgen 0.13: returns Result)
        let mut params = CertificateParams::new(vec![csr.subject.common_name.clone()])?;

        // Set validity period
        let now = SystemTime::now();
        let validity_duration = std::time::Duration::from_secs(
            self.ca_config.cert_validity_days as u64 * 24 * 60 * 60
        );
        let not_after = now + validity_duration;

        params.not_before = now.into();
        params.not_after = not_after.into();
        
        // Set subject
        params.distinguished_name.push(DnType::CommonName, csr.subject.common_name.clone());
        
        // Add post-quantum extensions
        let pq_extensions = PQCertificateExtensions {
            falcon_parameters: "FALCON-1024, 128-bit quantum security".to_string(),
            kyber_parameters: csr.kyber_public_key.as_ref().map(|_| "Kyber-1024, NIST PQC standard".to_string()),
            quantum_security_level: self.ca_config.quantum_security_level,
            hybrid_signature_support: self.ca_config.enable_hybrid_signatures,
            migration_support: self.ca_config.enable_migration_support,
            pq_cert_version: "1.0".to_string(),
        };
        
        // Embed FALCON-1024 public key
        let falcon_extension = self.encode_falcon_public_key_extension(&csr.falcon_public_key)?;
        params.custom_extensions.push(falcon_extension);
        
        // Embed Kyber public key if present
        if let Some(ref kyber_pubkey) = csr.kyber_public_key {
            let kyber_extension = self.encode_kyber_public_key_extension(kyber_pubkey)?;
            params.custom_extensions.push(kyber_extension);
        }
        
        // rcgen 0.13: Generate key pair for the certificate
        let key_pair = rcgen::KeyPair::generate()?;

        // rcgen 0.13: Create self-signed certificate (TODO: should be CA-signed)
        let cert = params.self_signed(&key_pair)?;

        // Sign with CA FALCON-1024 key (this is a simplified approach)
        // In a full implementation, we would need to modify rcgen to support FALCON-1024 signing
        // For now, we create the certificate with standard algorithms and add FALCON signature separately
        let certificate_der = cert.der().to_vec();
        
        // Create CA signature with FALCON-1024
        let ca_signature = self.pqc.sign_with_falcon(&certificate_der, &self.ca_config.ca_keypair.private_key).await?;
        
        // Calculate certificate fingerprint
        let fingerprint = self.calculate_certificate_fingerprint(&certificate_der);
        
        // Generate serial number
        let serial_number = hex::encode(&fingerprint[..16]);
        
        let pq_certificate = PQCertificate {
            certificate_der,
            serial_number,
            common_name: csr.subject.common_name,
            san_entries: vec![], // TODO: Extract from certificate
            falcon_public_key: csr.falcon_public_key,
            kyber_public_key: csr.kyber_public_key,
            not_before: now,
            not_after,
            fingerprint,
            issuer_ca_id: self.ca_config.ca_id.clone(),
            ca_signature,
            pq_extensions,
        };
        
        info!("✅ Post-quantum certificate issued: {}", pq_certificate.serial_number);
        Ok(pq_certificate)
    }
    
    /// Verify post-quantum certificate
    pub async fn verify_pq_certificate(&self, certificate: &PQCertificate) -> Result<bool> {
        debug!("🔍 Verifying post-quantum certificate: {}", certificate.serial_number);
        
        // Verify certificate fingerprint
        let calculated_fingerprint = self.calculate_certificate_fingerprint(&certificate.certificate_der);
        if calculated_fingerprint != certificate.fingerprint {
            warn!("❌ Certificate fingerprint mismatch");
            return Ok(false);
        }
        
        // Verify CA signature
        let ca_signature_valid = self.pqc.verify_falcon_signature(
            &certificate.certificate_der,
            &certificate.ca_signature,
            &self.ca_config.ca_keypair.public_key,
        ).await?;
        
        if !ca_signature_valid {
            warn!("❌ CA signature verification failed");
            return Ok(false);
        }
        
        // Check validity period
        let now = SystemTime::now();
        if now < certificate.not_before || now > certificate.not_after {
            warn!("❌ Certificate is not within validity period");
            return Ok(false);
        }
        
        // Verify embedded FALCON-1024 public key
        if !self.verify_embedded_falcon_key(&certificate.certificate_der, &certificate.falcon_public_key)? {
            warn!("❌ Embedded FALCON-1024 public key verification failed");
            return Ok(false);
        }
        
        debug!("✅ Post-quantum certificate verification successful");
        Ok(true)
    }
    
    /// Extract FALCON-1024 public key from certificate
    pub fn extract_falcon_public_key(&self, cert_der: &[u8]) -> Result<FalconPublicKey> {
        debug!("🔍 Extracting FALCON-1024 public key from certificate");
        
        // Parse X.509 certificate
        let (_remainder, cert) = parse_x509_certificate(cert_der)
            .map_err(|e| anyhow!("Failed to parse X.509 certificate: {}", e))?;
        
        // Search for FALCON-1024 extension
        for extension in cert.extensions() {
            if self.is_falcon_extension(&extension.oid) {
                return self.decode_falcon_public_key_extension(&extension.value);
            }
        }
        
        Err(anyhow!("FALCON-1024 public key not found in certificate"))
    }
    
    /// Extract Kyber public key from certificate (if present)
    pub fn extract_kyber_public_key(&self, cert_der: &[u8]) -> Result<Option<KyberPublicKey>> {
        debug!("🔍 Extracting Kyber public key from certificate");
        
        let (_remainder, cert) = parse_x509_certificate(cert_der)
            .map_err(|e| anyhow!("Failed to parse X.509 certificate: {}", e))?;
        
        for extension in cert.extensions() {
            if self.is_kyber_extension(&extension.oid) {
                let kyber_key = self.decode_kyber_public_key_extension(&extension.value)?;
                return Ok(Some(kyber_key));
            }
        }
        
        Ok(None)
    }
    
    /// Create CA certificate with FALCON-1024
    pub async fn create_ca_certificate(&self) -> Result<PQCertificate> {
        info!("🔐 Creating post-quantum CA certificate for: {}", self.ca_config.ca_id);
        
        let subject = CertificateSubject {
            common_name: format!("TrustChain PQ-CA {}", self.ca_config.ca_id),
            organization: Some("TrustChain Quantum-Resistant CA".to_string()),
            organizational_unit: Some("Post-Quantum Cryptography Division".to_string()),
            country: Some("US".to_string()),
            state: None,
            locality: None,
            email: None,
        };
        
        // Generate self-signed CSR
        let csr = self.generate_pq_csr(
            subject,
            &self.ca_config.ca_keypair,
            self.ca_config.ca_kyber_keypair.as_ref(),
            vec![format!("ca.{}", self.ca_config.ca_id)],
        ).await?;
        
        // Issue self-signed certificate
        let mut ca_cert = self.issue_pq_certificate(csr).await?;
        
        // Mark as CA certificate
        ca_cert.pq_extensions.pq_cert_version = "CA-1.0".to_string();
        
        info!("✅ Post-quantum CA certificate created: {}", ca_cert.serial_number);
        Ok(ca_cert)
    }
    
    /// Internal: Encode FALCON-1024 public key as X.509 extension
    fn encode_falcon_public_key_extension(&self, public_key: &FalconPublicKey) -> Result<rcgen::CustomExtension> {
        let extension_data = bincode::serialize(&public_key)
            .map_err(|e| anyhow!("Failed to serialize FALCON public key: {}", e))?;
        
        Ok(rcgen::CustomExtension::from_oid_content(
            &[1, 3, 6, 1, 4, 1, 99999, 1], // Custom OID for FALCON-1024
            extension_data,
        ))
    }
    
    /// Internal: Encode Kyber public key as X.509 extension
    fn encode_kyber_public_key_extension(&self, public_key: &KyberPublicKey) -> Result<rcgen::CustomExtension> {
        let extension_data = bincode::serialize(&public_key)
            .map_err(|e| anyhow!("Failed to serialize Kyber public key: {}", e))?;
        
        Ok(rcgen::CustomExtension::from_oid_content(
            &[1, 3, 6, 1, 4, 1, 99999, 2], // Custom OID for Kyber
            extension_data,
        ))
    }
    
    /// Internal: Decode FALCON-1024 public key from extension
    fn decode_falcon_public_key_extension(&self, extension_data: &[u8]) -> Result<FalconPublicKey> {
        bincode::deserialize(extension_data)
            .map_err(|e| anyhow!("Failed to deserialize FALCON public key: {}", e))
    }
    
    /// Internal: Decode Kyber public key from extension
    fn decode_kyber_public_key_extension(&self, extension_data: &[u8]) -> Result<KyberPublicKey> {
        bincode::deserialize(extension_data)
            .map_err(|e| anyhow!("Failed to deserialize Kyber public key: {}", e))
    }
    
    /// Internal: Check if OID is FALCON extension
    fn is_falcon_extension(&self, oid: &x509_parser::der_parser::oid::Oid) -> bool {
        oid.to_string() == "1.3.6.1.4.1.99999.1"
    }
    
    /// Internal: Check if OID is Kyber extension
    fn is_kyber_extension(&self, oid: &x509_parser::der_parser::oid::Oid) -> bool {
        oid.to_string() == "1.3.6.1.4.1.99999.2"
    }
    
    /// Internal: Verify embedded FALCON key matches expected
    fn verify_embedded_falcon_key(&self, cert_der: &[u8], expected_key: &FalconPublicKey) -> Result<bool> {
        let extracted_key = self.extract_falcon_public_key(cert_der)?;
        Ok(extracted_key.key_bytes == expected_key.key_bytes)
    }
    
    /// Internal: Calculate certificate fingerprint
    fn calculate_certificate_fingerprint(&self, cert_der: &[u8]) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(cert_der);
        hasher.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyUsage;
    
    #[tokio::test]
    async fn test_pq_certificate_manager_initialization() {
        let pqc = PostQuantumCrypto::new().unwrap();
        let ca_keypair = pqc.generate_ca_keypair("test-ca").await.unwrap();
        
        let ca_config = PQCAConfig {
            ca_id: "test-ca".to_string(),
            ca_keypair,
            ca_kyber_keypair: None,
            cert_validity_days: 1,
            quantum_security_level: 128,
            enable_hybrid_signatures: true,
            enable_migration_support: true,
        };
        
        let cert_manager = PQCertificateManager::new(ca_config).await.unwrap();
        assert_eq!(cert_manager.ca_config.ca_id, "test-ca");
    }
    
    #[tokio::test]
    async fn test_pq_csr_generation() {
        let pqc = PostQuantumCrypto::new().unwrap();
        let ca_keypair = pqc.generate_ca_keypair("test-ca").await.unwrap();
        let user_keypair = pqc.generate_asset_keypair().await.unwrap();
        
        let ca_config = PQCAConfig {
            ca_id: "test-ca".to_string(),
            ca_keypair,
            ca_kyber_keypair: None,
            cert_validity_days: 1,
            quantum_security_level: 128,
            enable_hybrid_signatures: true,
            enable_migration_support: true,
        };
        
        let cert_manager = PQCertificateManager::new(ca_config).await.unwrap();
        
        let subject = CertificateSubject {
            common_name: "test.example.com".to_string(),
            organization: Some("Test Org".to_string()),
            organizational_unit: None,
            country: Some("US".to_string()),
            state: None,
            locality: None,
            email: None,
        };
        
        let csr = cert_manager.generate_pq_csr(
            subject,
            &user_keypair,
            None,
            vec!["test.example.com".to_string()],
        ).await.unwrap();
        
        assert_eq!(csr.subject.common_name, "test.example.com");
        assert!(!csr.csr_der.is_empty());
    }
    
    #[tokio::test]
    async fn test_pq_certificate_issuance_and_verification() {
        let pqc = PostQuantumCrypto::new().unwrap();
        let ca_keypair = pqc.generate_ca_keypair("test-ca").await.unwrap();
        let user_keypair = pqc.generate_asset_keypair().await.unwrap();
        
        let ca_config = PQCAConfig {
            ca_id: "test-ca".to_string(),
            ca_keypair,
            ca_kyber_keypair: None,
            cert_validity_days: 1,
            quantum_security_level: 128,
            enable_hybrid_signatures: true,
            enable_migration_support: true,
        };
        
        let cert_manager = PQCertificateManager::new(ca_config).await.unwrap();
        
        let subject = CertificateSubject {
            common_name: "test.example.com".to_string(),
            organization: Some("Test Org".to_string()),
            organizational_unit: None,
            country: Some("US".to_string()),
            state: None,
            locality: None,
            email: None,
        };
        
        // Generate CSR
        let csr = cert_manager.generate_pq_csr(
            subject,
            &user_keypair,
            None,
            vec!["test.example.com".to_string()],
        ).await.unwrap();
        
        // Issue certificate
        let certificate = cert_manager.issue_pq_certificate(csr).await.unwrap();
        
        assert_eq!(certificate.common_name, "test.example.com");
        assert!(!certificate.certificate_der.is_empty());
        
        // Verify certificate
        let is_valid = cert_manager.verify_pq_certificate(&certificate).await.unwrap();
        assert!(is_valid);
    }
    
    #[tokio::test]
    async fn test_ca_certificate_creation() {
        let pqc = PostQuantumCrypto::new().unwrap();
        let ca_keypair = pqc.generate_ca_keypair("test-ca").await.unwrap();
        
        let ca_config = PQCAConfig {
            ca_id: "test-ca".to_string(),
            ca_keypair,
            ca_kyber_keypair: None,
            cert_validity_days: 365,
            quantum_security_level: 128,
            enable_hybrid_signatures: true,
            enable_migration_support: true,
        };
        
        let cert_manager = PQCertificateManager::new(ca_config).await.unwrap();
        
        let ca_cert = cert_manager.create_ca_certificate().await.unwrap();
        
        assert!(ca_cert.common_name.contains("TrustChain PQ-CA"));
        assert_eq!(ca_cert.pq_extensions.pq_cert_version, "CA-1.0");
        
        let is_valid = cert_manager.verify_pq_certificate(&ca_cert).await.unwrap();
        assert!(is_valid);
    }
}