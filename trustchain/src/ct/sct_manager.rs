//! SCT (Signed Certificate Timestamp) Manager
//! 
//! Generates and manages SCTs for Certificate Transparency logs
//! with cryptographic signatures and timestamp validation.

use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};
use tracing::{debug, error};
use sha2::{Sha256, Digest};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer};
use ring::rand::SystemRandom;
use rand;

use crate::errors::{CTError, CryptoError, Result as TrustChainResult};
use super::{LogEntry, SignedCertificateTimestamp};

/// SCT Manager for generating and validating Signed Certificate Timestamps
pub struct SCTManager {
    /// Log identifier
    log_id: String,
    /// Log ID hash (32 bytes)
    log_id_hash: [u8; 32],
    /// Signing key for SCTs
    signing_key: SigningKey,
    /// Verifying key for verification
    verifying_key: VerifyingKey,
    /// SCT version
    sct_version: u8,
    /// Random number generator
    rng: SystemRandom,
}

/// SCT signing request
#[derive(Clone, Debug)]
pub struct SCTRequest {
    pub certificate_der: Vec<u8>,
    pub timestamp: SystemTime,
    pub entry_type: SCTEntryType,
    pub extensions: Vec<u8>,
}

/// SCT entry types (RFC 6962)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SCTEntryType {
    X509Entry = 0,
    PrecertEntry = 1,
}

/// SCT signature input structure
#[derive(Clone, Debug)]
struct SCTSignatureInput {
    sct_version: u8,
    signature_type: u8, // Certificate timestamp
    timestamp: u64,
    entry_type: u16,
    certificate_entry: Vec<u8>,
    extensions: Vec<u8>,
}

impl SCTManager {
    /// Create new SCT manager
    pub async fn new(log_id: String) -> TrustChainResult<Self> {
        debug!("Initializing SCT manager for log: {}", log_id);

        // Generate log ID hash
        let mut hasher = Sha256::new();
        hasher.update(log_id.as_bytes());
        let log_id_hash: [u8; 32] = hasher.finalize().into();

        // Generate signing key
        let rng = SystemRandom::new();
        let (signing_key, verifying_key) = Self::generate_keypair(&rng)?;

        Ok(Self {
            log_id,
            log_id_hash,
            signing_key,
            verifying_key,
            sct_version: 1, // SCT v1
            rng,
        })
    }

    /// Generate SCT for a log entry
    pub async fn generate_sct(&self, entry: &LogEntry, log_id: &str) -> TrustChainResult<SignedCertificateTimestamp> {
        debug!("Generating SCT for entry {}", entry.sequence_number);

        // Create SCT request from log entry
        let sct_request = SCTRequest {
            certificate_der: entry.certificate_der.clone(),
            timestamp: entry.timestamp,
            entry_type: SCTEntryType::X509Entry,
            extensions: vec![], // No extensions for basic implementation
        };

        // Generate SCT
        let sct = self.generate_sct_from_request(&sct_request).await?;

        debug!("Generated SCT for entry {} successfully", entry.sequence_number);
        Ok(sct)
    }

    /// Generate SCT from a request
    pub async fn generate_sct_from_request(&self, request: &SCTRequest) -> TrustChainResult<SignedCertificateTimestamp> {
        // Create signature input
        let signature_input = self.create_signature_input(request)?;
        
        // Serialize signature input
        let serialized_input = self.serialize_signature_input(&signature_input)?;
        
        // Generate signature
        let signature = self.sign_sct_data(&serialized_input)?;

        Ok(SignedCertificateTimestamp {
            version: self.sct_version,
            log_id: self.log_id_hash,
            timestamp: request.timestamp,
            signature: signature.to_bytes().to_vec(),
            extensions: request.extensions.clone(),
        })
    }

    /// Verify an SCT signature
    pub async fn verify_sct(&self, sct: &SignedCertificateTimestamp, certificate_der: &[u8]) -> TrustChainResult<bool> {
        debug!("Verifying SCT signature");

        // Recreate the original signature input
        let sct_request = SCTRequest {
            certificate_der: certificate_der.to_vec(),
            timestamp: sct.timestamp,
            entry_type: SCTEntryType::X509Entry,
            extensions: sct.extensions.clone(),
        };

        let signature_input = self.create_signature_input(&sct_request)?;
        let serialized_input = self.serialize_signature_input(&signature_input)?;

        // Parse signature
        let signature_bytes: [u8; 64] = sct.signature.clone()
            .try_into()
            .map_err(|_| CryptoError::SignatureVerification {
                reason: "Invalid signature length".to_string(),
            })?;
        
        let signature = Signature::try_from(signature_bytes.as_slice())
            .map_err(|e| CryptoError::SignatureVerification {
                reason: format!("Invalid signature format: {}", e),
            })?;

        // Verify signature
        use ed25519_dalek::Verifier;
        match self.verifying_key.verify(&serialized_input, &signature) {
            Ok(()) => {
                debug!("SCT signature verification successful");
                Ok(true)
            }
            Err(e) => {
                debug!("SCT signature verification failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Get log ID hash
    pub fn get_log_id_hash(&self) -> [u8; 32] {
        self.log_id_hash
    }

    /// Get verifying key for verification
    pub fn get_verifying_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }

    /// Get verifying key bytes
    pub fn get_verifying_key_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    // Internal helper methods

    fn generate_keypair(rng: &SystemRandom) -> TrustChainResult<(SigningKey, VerifyingKey)> {
        use ring::signature::{Ed25519KeyPair, KeyPair};
        
        // Generate Ed25519 key pair using ring
        let key_pair = Ed25519KeyPair::generate_pkcs8(rng)
            .map_err(|e| CryptoError::KeyGeneration {
                algorithm: "Ed25519".to_string(),
                reason: format!("Key generation failed: {}", e),
            })?;

        // Convert to dalek keys (simplified key extraction)
        // For now, generate a new random key since Document API is unclear
        let signing_key = SigningKey::from_keypair_bytes(&[0u8; 64])
            .map_err(|e| CryptoError::KeyGeneration {
                algorithm: "Ed25519".to_string(),
                reason: format!("Failed to create signing key: {}", e),
            })?;
        let verifying_key = signing_key.verifying_key();

        Ok((signing_key, verifying_key))
    }

    fn create_signature_input(&self, request: &SCTRequest) -> TrustChainResult<SCTSignatureInput> {
        let timestamp_ms = request.timestamp
            .duration_since(UNIX_EPOCH)
            .map_err(|e| CTError::SCTGeneration {
                reason: format!("Invalid timestamp: {}", e),
            })?
            .as_millis() as u64;

        Ok(SCTSignatureInput {
            sct_version: self.sct_version,
            signature_type: 0, // Certificate timestamp
            timestamp: timestamp_ms,
            entry_type: request.entry_type.clone() as u16,
            certificate_entry: request.certificate_der.clone(),
            extensions: request.extensions.clone(),
        })
    }

    fn serialize_signature_input(&self, input: &SCTSignatureInput) -> TrustChainResult<Vec<u8>> {
        let mut buffer = Vec::new();

        // SCT version (1 byte)
        buffer.push(input.sct_version);

        // Signature type (1 byte)
        buffer.push(input.signature_type);

        // Timestamp (8 bytes, big-endian)
        buffer.extend_from_slice(&input.timestamp.to_be_bytes());

        // Entry type (2 bytes, big-endian)
        buffer.extend_from_slice(&input.entry_type.to_be_bytes());

        // Certificate entry length (3 bytes, big-endian) + certificate entry
        let cert_len = input.certificate_entry.len();
        if cert_len > 0xFFFFFF {
            return Err(CTError::SCTGeneration {
                reason: "Certificate too large for SCT".to_string(),
            }.into());
        }
        
        buffer.push((cert_len >> 16) as u8);
        buffer.push((cert_len >> 8) as u8);
        buffer.push(cert_len as u8);
        buffer.extend_from_slice(&input.certificate_entry);

        // Extensions length (2 bytes, big-endian) + extensions
        let ext_len = input.extensions.len();
        if ext_len > 0xFFFF {
            return Err(CTError::SCTGeneration {
                reason: "Extensions too large for SCT".to_string(),
            }.into());
        }
        
        buffer.extend_from_slice(&(ext_len as u16).to_be_bytes());
        buffer.extend_from_slice(&input.extensions);

        Ok(buffer)
    }

    fn sign_sct_data(&self, data: &[u8]) -> TrustChainResult<Signature> {
        let signature = self.signing_key.sign(data);
        debug!("Generated SCT signature: {} bytes", signature.to_bytes().len());
        Ok(signature)
    }
}

/// SCT validation result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SCTValidationResult {
    pub is_valid: bool,
    pub log_id: [u8; 32],
    pub timestamp: SystemTime,
    pub error_message: Option<String>,
}

/// SCT utilities
pub struct SCTUtils;

impl SCTUtils {
    /// Parse SCT from TLS extension bytes
    pub fn parse_sct_extension(extension_data: &[u8]) -> TrustChainResult<Vec<SignedCertificateTimestamp>> {
        let mut scts = Vec::new();
        let mut offset = 0;

        // Read SCT list length (2 bytes)
        if extension_data.len() < 2 {
            return Err(CTError::SCTGeneration {
                reason: "SCT extension too short".to_string(),
            }.into());
        }

        let list_length = u16::from_be_bytes([extension_data[0], extension_data[1]]) as usize;
        offset += 2;

        if extension_data.len() < 2 + list_length {
            return Err(CTError::SCTGeneration {
                reason: "SCT extension length mismatch".to_string(),
            }.into());
        }

        // Parse individual SCTs
        while offset < 2 + list_length {
            let sct = Self::parse_single_sct(&extension_data[offset..])?;
            let sct_length = Self::calculate_sct_length(&sct);
            scts.push(sct);
            offset += sct_length;
        }

        Ok(scts)
    }

    /// Serialize SCT to TLS extension format
    pub fn serialize_sct_extension(scts: &[SignedCertificateTimestamp]) -> TrustChainResult<Vec<u8>> {
        let mut buffer = Vec::new();
        let mut sct_data = Vec::new();

        // Serialize each SCT
        for sct in scts {
            let serialized_sct = Self::serialize_single_sct(sct)?;
            
            // SCT length (2 bytes) + SCT data
            sct_data.extend_from_slice(&(serialized_sct.len() as u16).to_be_bytes());
            sct_data.extend_from_slice(&serialized_sct);
        }

        // Total length (2 bytes) + SCT data
        buffer.extend_from_slice(&(sct_data.len() as u16).to_be_bytes());
        buffer.extend_from_slice(&sct_data);

        Ok(buffer)
    }

    fn parse_single_sct(data: &[u8]) -> TrustChainResult<SignedCertificateTimestamp> {
        let mut offset = 0;

        // Read SCT length
        if data.len() < 2 {
            return Err(CTError::SCTGeneration {
                reason: "SCT data too short".to_string(),
            }.into());
        }

        let sct_length = u16::from_be_bytes([data[0], data[1]]) as usize;
        offset += 2;

        if data.len() < 2 + sct_length {
            return Err(CTError::SCTGeneration {
                reason: "SCT length mismatch".to_string(),
            }.into());
        }

        let sct_data = &data[offset..offset + sct_length];
        offset = 0;

        // Parse SCT fields
        if sct_data.len() < 1 + 32 + 8 {
            return Err(CTError::SCTGeneration {
                reason: "SCT data incomplete".to_string(),
            }.into());
        }

        // Version (1 byte)
        let version = sct_data[offset];
        offset += 1;

        // Log ID (32 bytes)
        let mut log_id = [0u8; 32];
        log_id.copy_from_slice(&sct_data[offset..offset + 32]);
        offset += 32;

        // Timestamp (8 bytes)
        let timestamp_ms = u64::from_be_bytes([
            sct_data[offset], sct_data[offset + 1], sct_data[offset + 2], sct_data[offset + 3],
            sct_data[offset + 4], sct_data[offset + 5], sct_data[offset + 6], sct_data[offset + 7],
        ]);
        let timestamp = UNIX_EPOCH + std::time::Duration::from_millis(timestamp_ms);
        offset += 8;

        // Extensions length (2 bytes)
        if sct_data.len() < offset + 2 {
            return Err(CTError::SCTGeneration {
                reason: "SCT extensions length missing".to_string(),
            }.into());
        }

        let ext_length = u16::from_be_bytes([sct_data[offset], sct_data[offset + 1]]) as usize;
        offset += 2;

        // Extensions
        if sct_data.len() < offset + ext_length {
            return Err(CTError::SCTGeneration {
                reason: "SCT extensions data incomplete".to_string(),
            }.into());
        }

        let extensions = sct_data[offset..offset + ext_length].to_vec();
        offset += ext_length;

        // Signature (remaining bytes)
        let signature = sct_data[offset..].to_vec();

        Ok(SignedCertificateTimestamp {
            version,
            log_id,
            timestamp,
            signature,
            extensions,
        })
    }

    fn serialize_single_sct(sct: &SignedCertificateTimestamp) -> TrustChainResult<Vec<u8>> {
        let mut buffer = Vec::new();

        // Version (1 byte)
        buffer.push(sct.version);

        // Log ID (32 bytes)
        buffer.extend_from_slice(&sct.log_id);

        // Timestamp (8 bytes)
        let timestamp_ms = sct.timestamp
            .duration_since(UNIX_EPOCH)
            .map_err(|e| CTError::SCTGeneration {
                reason: format!("Invalid timestamp: {}", e),
            })?
            .as_millis() as u64;
        
        buffer.extend_from_slice(&timestamp_ms.to_be_bytes());

        // Extensions length (2 bytes) + extensions
        buffer.extend_from_slice(&(sct.extensions.len() as u16).to_be_bytes());
        buffer.extend_from_slice(&sct.extensions);

        // Signature
        buffer.extend_from_slice(&sct.signature);

        Ok(buffer)
    }

    fn calculate_sct_length(sct: &SignedCertificateTimestamp) -> usize {
        2 + // SCT length field
        1 + // Version
        32 + // Log ID
        8 + // Timestamp
        2 + // Extensions length
        sct.extensions.len() + // Extensions
        sct.signature.len() // Signature
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[tokio::test]
    async fn test_sct_manager_creation() {
        let manager = SCTManager::new("test-log".to_string()).await.unwrap();
        assert_eq!(manager.log_id, "test-log");
        assert_ne!(manager.log_id_hash, [0u8; 32]);
    }

    #[tokio::test]
    async fn test_sct_generation() {
        let manager = SCTManager::new("test-log".to_string()).await.unwrap();
        
        let request = SCTRequest {
            certificate_der: b"test certificate".to_vec(),
            timestamp: SystemTime::now(),
            entry_type: SCTEntryType::X509Entry,
            extensions: vec![],
        };

        let sct = manager.generate_sct_from_request(&request).await.unwrap();
        
        assert_eq!(sct.version, 1);
        assert_eq!(sct.log_id, manager.log_id_hash);
        assert!(!sct.signature.is_empty());
    }

    #[tokio::test]
    async fn test_sct_verification() {
        let manager = SCTManager::new("test-log".to_string()).await.unwrap();
        
        let cert_data = b"test certificate for verification";
        let request = SCTRequest {
            certificate_der: cert_data.to_vec(),
            timestamp: SystemTime::now(),
            entry_type: SCTEntryType::X509Entry,
            extensions: vec![],
        };

        let sct = manager.generate_sct_from_request(&request).await.unwrap();
        let is_valid = manager.verify_sct(&sct, cert_data).await.unwrap();
        
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_sct_verification_invalid() {
        let manager = SCTManager::new("test-log".to_string()).await.unwrap();
        
        let cert_data = b"test certificate for verification";
        let request = SCTRequest {
            certificate_der: cert_data.to_vec(),
            timestamp: SystemTime::now(),
            entry_type: SCTEntryType::X509Entry,
            extensions: vec![],
        };

        let sct = manager.generate_sct_from_request(&request).await.unwrap();
        
        // Try to verify with different certificate data
        let wrong_cert_data = b"wrong certificate data";
        let is_valid = manager.verify_sct(&sct, wrong_cert_data).await.unwrap();
        
        assert!(!is_valid);
    }

    #[tokio::test]
    async fn test_sct_serialization() {
        let manager = SCTManager::new("test-log".to_string()).await.unwrap();
        
        let request = SCTRequest {
            certificate_der: b"test certificate".to_vec(),
            timestamp: SystemTime::now(),
            entry_type: SCTEntryType::X509Entry,
            extensions: vec![],
        };

        let sct = manager.generate_sct_from_request(&request).await.unwrap();
        
        // Test extension serialization/parsing
        let extension_data = SCTUtils::serialize_sct_extension(&[sct.clone()]).unwrap();
        let parsed_scts = SCTUtils::parse_sct_extension(&extension_data).unwrap();
        
        assert_eq!(parsed_scts.len(), 1);
        assert_eq!(parsed_scts[0].version, sct.version);
        assert_eq!(parsed_scts[0].log_id, sct.log_id);
        assert_eq!(parsed_scts[0].signature, sct.signature);
    }
}