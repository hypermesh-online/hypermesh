//! Certificate Operations
//!
//! Certificate issuance, validation, and management operations.

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{info, debug, warn};
use sha2::{Sha256, Digest};
use x509_parser::prelude::*;
use rsa::{RsaPrivateKey, RsaPublicKey};
use rsa::pkcs1v15::SigningKey as Pkcs1v15SigningKey;
use rsa::signature::Signer;

use super::types::*;

/// Certificate operations trait
pub trait CertificateOperations {
    /// Issue a new certificate
    fn issue_certificate(
        &self,
        request: &CertificateRequest,
        ca_key: &RsaPrivateKey,
    ) -> Result<CaCertificate>;

    /// Validate a certificate
    fn validate_certificate(
        &self,
        certificate_der: &[u8],
    ) -> Result<CertificateValidationResult>;

    /// Revoke a certificate
    fn revoke_certificate(
        &self,
        serial_number: &str,
        reason: RevocationReason,
    ) -> Result<()>;

    /// Generate certificate serial number
    fn generate_serial_number(&self) -> String;

    /// Create certificate fingerprint
    fn create_fingerprint(&self, certificate_der: &[u8]) -> String;
}

/// Default certificate operations implementation
pub struct DefaultCertificateOperations;

impl DefaultCertificateOperations {
    /// Build X.509 distinguished name
    pub fn build_distinguished_name(request: &CertificateRequest) -> String {
        let mut dn_parts = Vec::new();

        if !request.common_name.is_empty() {
            dn_parts.push(format!("CN={}", request.common_name));
        }

        if let Some(org) = &request.organization {
            dn_parts.push(format!("O={}", org));
        }

        if let Some(ou) = &request.organizational_unit {
            dn_parts.push(format!("OU={}", ou));
        }

        if let Some(country) = &request.country {
            if country.len() == 2 {
                dn_parts.push(format!("C={}", country));
            }
        }

        if let Some(state) = &request.state {
            dn_parts.push(format!("ST={}", state));
        }

        if let Some(locality) = &request.locality {
            dn_parts.push(format!("L={}", locality));
        }

        if let Some(email) = &request.email {
            dn_parts.push(format!("emailAddress={}", email));
        }

        if dn_parts.is_empty() {
            request.subject_dn.clone()
        } else {
            dn_parts.join(", ")
        }
    }

    /// Generate RSA key pair
    pub fn generate_key_pair(bits: usize) -> Result<(RsaPrivateKey, RsaPublicKey)> {
        use rand::rngs::OsRng;

        let private_key = RsaPrivateKey::new(&mut OsRng, bits)?;
        let public_key = RsaPublicKey::from(&private_key);

        Ok((private_key, public_key))
    }

    /// Sign data with RSA private key
    pub fn sign_data(private_key: &RsaPrivateKey, data: &[u8]) -> Result<Vec<u8>> {
        let signing_key = Pkcs1v15SigningKey::<Sha256>::new(private_key.clone());
        let signature = signing_key.sign(data);
        Ok(signature.to_vec())
    }

    /// Verify RSA signature
    pub fn verify_signature(
        public_key: &RsaPublicKey,
        data: &[u8],
        signature: &[u8],
    ) -> Result<bool> {
        use rsa::pkcs1v15::VerifyingKey;
        use rsa::signature::Verifier;

        let verifying_key = VerifyingKey::<Sha256>::new(public_key.clone());
        let signature = rsa::pkcs1v15::Signature::try_from(signature)?;

        Ok(verifying_key.verify(data, &signature).is_ok())
    }

    /// Create basic constraints extension
    pub fn create_basic_constraints(is_ca: bool, path_len: Option<u32>) -> Vec<u8> {
        let mut extension = Vec::new();
        extension.push(if is_ca { 1 } else { 0 });

        if let Some(len) = path_len {
            extension.extend_from_slice(&len.to_be_bytes());
        }

        extension
    }

    /// Create key usage extension
    pub fn create_key_usage(usage: &super::types::KeyUsage) -> Vec<u8> {
        let mut flags = 0u16;

        if usage.digital_signature {
            flags |= 0x0080;
        }
        if usage.content_commitment {
            flags |= 0x0040;
        }
        if usage.key_encipherment {
            flags |= 0x0020;
        }
        if usage.data_encipherment {
            flags |= 0x0010;
        }
        if usage.key_agreement {
            flags |= 0x0008;
        }
        if usage.key_cert_sign {
            flags |= 0x0004;
        }
        if usage.crl_sign {
            flags |= 0x0002;
        }

        flags.to_be_bytes().to_vec()
    }

    /// Create extended key usage extension
    pub fn create_extended_key_usage(usage: &super::types::ExtendedKeyUsage) -> Vec<u8> {
        let mut oids = Vec::new();

        if usage.server_auth {
            oids.push("1.3.6.1.5.5.7.3.1");
        }
        if usage.client_auth {
            oids.push("1.3.6.1.5.5.7.3.2");
        }
        if usage.code_signing {
            oids.push("1.3.6.1.5.5.7.3.3");
        }
        if usage.email_protection {
            oids.push("1.3.6.1.5.5.7.3.4");
        }
        if usage.time_stamping {
            oids.push("1.3.6.1.5.5.7.3.8");
        }
        if usage.ocsp_signing {
            oids.push("1.3.6.1.5.5.7.3.9");
        }

        oids.join(",").into_bytes()
    }

    /// Create subject alternative names extension
    pub fn create_san_extension(
        dns_names: &[String],
        ip_addresses: &[String],
    ) -> Vec<u8> {
        let mut san = Vec::new();

        for dns in dns_names {
            san.extend_from_slice(b"DNS:");
            san.extend_from_slice(dns.as_bytes());
            san.push(b',');
        }

        for ip in ip_addresses {
            san.extend_from_slice(b"IP:");
            san.extend_from_slice(ip.as_bytes());
            san.push(b',');
        }

        if !san.is_empty() {
            san.pop(); // Remove trailing comma
        }

        san
    }

    /// Check certificate expiry
    pub fn check_expiry(certificate: &CaCertificate) -> bool {
        let now = SystemTime::now();
        certificate.not_before <= now && now <= certificate.not_after
    }

    /// Calculate days until expiry
    pub fn days_until_expiry(certificate: &CaCertificate) -> Option<u64> {
        let now = SystemTime::now();

        if now > certificate.not_after {
            return None;
        }

        certificate.not_after
            .duration_since(now)
            .ok()
            .map(|d| d.as_secs() / 86400)
    }
}

impl CertificateOperations for DefaultCertificateOperations {
    fn issue_certificate(
        &self,
        request: &CertificateRequest,
        ca_key: &RsaPrivateKey,
    ) -> Result<CaCertificate> {
        let serial_number = self.generate_serial_number();
        let subject_dn = Self::build_distinguished_name(request);

        // For now, return a simple mock certificate
        // In real implementation, this would build a proper X.509 certificate

        let not_before = SystemTime::now();
        let not_after = not_before + Duration::from_secs(
            request.validity_days as u64 * 86400
        );

        let certificate = CaCertificate {
            serial_number,
            subject_dn: subject_dn.clone(),
            issuer_dn: "CN=TrustChain Root CA, O=HyperMesh, C=US".to_string(),
            not_before,
            not_after,
            public_key: Vec::new(), // Would be filled with actual public key
            signature: Vec::new(),  // Would be filled with actual signature
            pq_signature: None,
            extensions: HashMap::new(),
            certificate_der: Vec::new(), // Would be filled with DER encoding
        };

        Ok(certificate)
    }

    fn validate_certificate(
        &self,
        certificate_der: &[u8],
    ) -> Result<CertificateValidationResult> {
        // Parse X.509 certificate
        let (_, cert) = X509Certificate::from_der(certificate_der)
            .map_err(|e| anyhow!("Failed to parse certificate: {:?}", e))?;

        let now = SystemTime::now();
        let now_asn1 = ASN1Time::from_timestamp(
            now.duration_since(UNIX_EPOCH)?.as_secs() as i64
        )?;

        let not_expired = cert.validity.not_after >= now_asn1
            && cert.validity.not_before <= now_asn1;

        let certificate_info = CertificateInfo {
            serial_number: format!("{:x}", cert.raw_serial_as_string()),
            subject_dn: cert.subject.to_string(),
            issuer_dn: cert.issuer.to_string(),
            not_before: SystemTime::UNIX_EPOCH + Duration::from_secs(
                cert.validity.not_before.timestamp() as u64
            ),
            not_after: SystemTime::UNIX_EPOCH + Duration::from_secs(
                cert.validity.not_after.timestamp() as u64
            ),
            fingerprint: self.create_fingerprint(certificate_der),
        };

        Ok(CertificateValidationResult {
            is_valid: not_expired,
            chain_valid: true, // Would verify chain in real implementation
            signature_valid: true, // Would verify signature in real implementation
            not_expired,
            not_revoked: true, // Would check CRL in real implementation
            domain_match: None,
            errors: if not_expired {
                Vec::new()
            } else {
                vec!["Certificate expired".to_string()]
            },
            certificate_info: Some(certificate_info),
        })
    }

    fn revoke_certificate(
        &self,
        serial_number: &str,
        reason: RevocationReason,
    ) -> Result<()> {
        info!(
            "Revoking certificate {} for reason: {}",
            serial_number,
            reason.description()
        );

        // In real implementation, this would update the CRL
        Ok(())
    }

    fn generate_serial_number(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: [u8; 16] = rng.gen();
        hex::encode(bytes)
    }

    fn create_fingerprint(&self, certificate_der: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(certificate_der);
        let result = hasher.finalize();
        hex::encode(result)
    }
}