//! FALCON-1024 Post-Quantum Signature Implementation
//!
//! Provides FALCON-1024 signature generation and verification for quantum-resistant
//! certificate authority operations and asset authentication in the TrustChain ecosystem.

use std::time::SystemTime;
use anyhow::{Result, anyhow};
use tracing::{info, debug, warn, error};
use sha2::{Sha256, Digest};

use pqcrypto_falcon::falcon1024;
use pqcrypto_traits::sign::{PublicKey, SecretKey, DetachedSignature};

use super::{
    FalconKeyPair, FalconPublicKey, FalconPrivateKey, FalconSignature,
    KeyUsage, PQCError
};

/// FALCON-1024 cryptographic operations handler
pub struct FalconCrypto {
    /// Algorithm identifier
    algorithm_id: String,
}

impl FalconCrypto {
    /// Initialize FALCON-1024 cryptographic system
    pub fn new() -> Result<Self> {
        info!("🔐 Initializing FALCON-1024 post-quantum signature system");
        
        Ok(Self {
            algorithm_id: "FALCON-1024".to_string(),
        })
    }
    
    /// Generate FALCON-1024 key pair
    pub async fn generate_keypair(&self, usage: KeyUsage) -> Result<FalconKeyPair> {
        info!("🔑 Generating FALCON-1024 key pair for usage: {:?}", usage);
        
        // Generate FALCON-1024 key pair
        let (public_key_native, secret_key_native) = falcon1024::keypair();
        
        // Extract raw bytes
        let public_key_bytes = public_key_native.as_bytes().to_vec();
        let secret_key_bytes = secret_key_native.as_bytes().to_vec();
        
        // Validate key sizes
        if public_key_bytes.len() != falcon1024::public_key_bytes() {
            return Err(anyhow!(
                "FALCON-1024 public key size mismatch: expected {}, got {}",
                falcon1024::public_key_bytes(),
                public_key_bytes.len()
            ));
        }
        
        if secret_key_bytes.len() != falcon1024::secret_key_bytes() {
            return Err(anyhow!(
                "FALCON-1024 secret key size mismatch: expected {}, got {}",
                falcon1024::secret_key_bytes(),
                secret_key_bytes.len()
            ));
        }
        
        // Generate key fingerprint
        let fingerprint = self.calculate_key_fingerprint(&public_key_bytes);
        
        // Generate salt for key strengthening
        let salt = self.generate_salt();
        
        let public_key = FalconPublicKey {
            key_bytes: public_key_bytes,
            fingerprint,
        };
        
        let private_key = FalconPrivateKey {
            key_bytes: secret_key_bytes,
            salt,
        };
        
        let keypair = FalconKeyPair {
            public_key,
            private_key,
            created_at: SystemTime::now(),
            key_usage: usage,
            ca_id: None,
        };
        
        info!("✅ FALCON-1024 key pair generated successfully");
        debug!("Public key size: {} bytes", keypair.public_key.key_bytes.len());
        debug!("Private key size: {} bytes", keypair.private_key.key_bytes.len());
        debug!("Key fingerprint: {}", hex::encode(&keypair.public_key.fingerprint[..8]));
        
        Ok(keypair)
    }
    
    /// Sign data with FALCON-1024 private key
    pub async fn sign(
        &self,
        data: &[u8],
        private_key: &FalconPrivateKey,
    ) -> Result<FalconSignature> {
        debug!("🔏 Signing {} bytes with FALCON-1024", data.len());
        
        // Validate private key size
        if private_key.key_bytes.len() != falcon1024::secret_key_bytes() {
            return Err(PQCError::InvalidKeyFormat {
                message: format!(
                    "Invalid FALCON-1024 private key size: expected {}, got {}",
                    falcon1024::secret_key_bytes(),
                    private_key.key_bytes.len()
                )
            }.into());
        }
        
        // Reconstruct secret key from bytes
        let secret_key_native = falcon1024::SecretKey::from_bytes(&private_key.key_bytes)
            .map_err(|e| PQCError::FalconSigningError {
                message: format!("Failed to reconstruct FALCON-1024 secret key: {}", e)
            })?;
        
        // Hash the data before signing (FALCON-1024 requirement)
        let message_hash = self.hash_message(data);
        
        // Create FALCON-1024 signature
        let signature_native = falcon1024::detached_sign(&message_hash, &secret_key_native);
        let signature_bytes = signature_native.as_bytes().to_vec();
        
        let signature = FalconSignature {
            signature_bytes,
            algorithm: self.algorithm_id.clone(),
            signed_at: SystemTime::now(),
            message_hash,
        };
        
        debug!("✅ FALCON-1024 signature created: {} bytes", signature.signature_bytes.len());
        Ok(signature)
    }
    
    /// Verify FALCON-1024 signature
    pub async fn verify(
        &self,
        data: &[u8],
        signature: &FalconSignature,
        public_key: &FalconPublicKey,
    ) -> Result<bool> {
        debug!("🔍 Verifying FALCON-1024 signature ({} bytes)", signature.signature_bytes.len());
        
        // Validate algorithm
        if signature.algorithm != self.algorithm_id {
            warn!("❌ Algorithm mismatch: expected {}, got {}", self.algorithm_id, signature.algorithm);
            return Ok(false);
        }
        
        // Validate public key size
        if public_key.key_bytes.len() != falcon1024::public_key_bytes() {
            return Err(PQCError::InvalidKeyFormat {
                message: format!(
                    "Invalid FALCON-1024 public key size: expected {}, got {}",
                    falcon1024::public_key_bytes(),
                    public_key.key_bytes.len()
                )
            }.into());
        }
        
        // Reconstruct public key from bytes
        let public_key_native = falcon1024::PublicKey::from_bytes(&public_key.key_bytes)
            .map_err(|e| PQCError::FalconVerificationError {
                message: format!("Failed to reconstruct FALCON-1024 public key: {}", e)
            })?;
        
        // Hash the data (same as during signing)
        let message_hash = self.hash_message(data);
        
        // Verify the message hash matches
        if message_hash != signature.message_hash {
            warn!("❌ Message hash mismatch during FALCON-1024 verification");
            return Ok(false);
        }
        
        // Reconstruct signature from bytes
        let signature_native = falcon1024::DetachedSignature::from_bytes(&signature.signature_bytes)
            .map_err(|e| PQCError::FalconVerificationError {
                message: format!("Failed to reconstruct FALCON-1024 signature: {}", e)
            })?;
        
        // Verify FALCON-1024 signature
        let verification_result = falcon1024::verify_detached_signature(
            &signature_native,
            &message_hash,
            &public_key_native,
        );
        
        match verification_result {
            Ok(_) => {
                debug!("✅ FALCON-1024 signature verification successful");
                Ok(true)
            }
            Err(e) => {
                debug!("❌ FALCON-1024 signature verification failed: {}", e);
                Ok(false)
            }
        }
    }
    
    /// Validate FALCON-1024 key pair consistency
    pub fn validate_keypair(&self, keypair: &FalconKeyPair) -> Result<bool> {
        debug!("🔍 Validating FALCON-1024 key pair consistency");
        
        // Check key sizes
        if keypair.public_key.key_bytes.len() != falcon1024::public_key_bytes() {
            warn!("❌ Invalid FALCON-1024 public key size: {}", keypair.public_key.key_bytes.len());
            return Ok(false);
        }
        
        if keypair.private_key.key_bytes.len() != falcon1024::secret_key_bytes() {
            warn!("❌ Invalid FALCON-1024 private key size: {}", keypair.private_key.key_bytes.len());
            return Ok(false);
        }
        
        // Verify fingerprint
        let calculated_fingerprint = self.calculate_key_fingerprint(&keypair.public_key.key_bytes);
        if calculated_fingerprint != keypair.public_key.fingerprint {
            warn!("❌ FALCON-1024 key fingerprint mismatch");
            return Ok(false);
        }
        
        // Test signature round-trip
        let test_data = b"FALCON-1024 key pair validation test";
        let test_signature = match falcon1024::detached_sign(
            test_data,
            &falcon1024::SecretKey::from_bytes(&keypair.private_key.key_bytes)
                .map_err(|e| PQCError::InvalidKeyFormat {
                    message: format!("Invalid secret key: {}", e)
                })?
        ) {
            sig => sig,
        };
        
        let verification_result = falcon1024::verify_detached_signature(
            &test_signature,
            test_data,
            &falcon1024::PublicKey::from_bytes(&keypair.public_key.key_bytes)
                .map_err(|e| PQCError::InvalidKeyFormat {
                    message: format!("Invalid public key: {}", e)
                })?
        );
        
        match verification_result {
            Ok(_) => {
                debug!("✅ FALCON-1024 key pair validation successful");
                Ok(true)
            }
            Err(e) => {
                warn!("❌ FALCON-1024 key pair validation failed: {}", e);
                Ok(false)
            }
        }
    }
    
    /// Extract public key from certificate request or existing certificate
    pub fn extract_public_key_from_certificate(&self, cert_der: &[u8]) -> Result<FalconPublicKey> {
        // Parse certificate and extract FALCON-1024 public key
        // This would involve X.509 certificate parsing specific to FALCON-1024 keys
        // For now, return a placeholder implementation
        
        error!("🚧 FALCON-1024 certificate public key extraction not yet implemented");
        Err(anyhow!("FALCON-1024 certificate integration under development"))
    }
    
    /// Create certificate signing request with FALCON-1024 key
    pub fn create_csr_with_falcon(
        &self,
        keypair: &FalconKeyPair,
        subject: &str,
        san_entries: &[String],
    ) -> Result<Vec<u8>> {
        // Generate CSR with FALCON-1024 public key
        // This would involve integrating FALCON-1024 keys into X.509 CSR format
        
        info!("🚧 Creating FALCON-1024 certificate signing request for: {}", subject);
        warn!("🚧 FALCON-1024 CSR generation integration under development");
        
        // Return placeholder CSR
        Ok(format!(
            "FALCON-1024 CSR Placeholder for subject: {}, SAN: {:?}, Key: {}",
            subject,
            san_entries,
            keypair.public_key
        ).into_bytes())
    }
    
    /// Get FALCON-1024 algorithm parameters
    pub fn get_algorithm_parameters(&self) -> FalconParameters {
        FalconParameters {
            name: "FALCON-1024".to_string(),
            security_level: 128, // 128-bit quantum security
            public_key_size: falcon1024::public_key_bytes(),
            secret_key_size: falcon1024::secret_key_bytes(),
            signature_size_average: 700, // Variable, ~700 bytes average
            signature_size_max: falcon1024::signature_bytes(),
            nist_pqc_round: 3,
            standardized: false, // FALCON did not win NIST PQC, but is still secure
            lattice_based: true,
            performance_tier: "Fast".to_string(),
        }
    }
    
    /// Internal: Hash message for signing
    fn hash_message(&self, data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }
    
    /// Internal: Calculate key fingerprint
    fn calculate_key_fingerprint(&self, key_bytes: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"FALCON-1024-KEY:");
        hasher.update(key_bytes);
        hasher.finalize().into()
    }
    
    /// Internal: Generate cryptographic salt
    fn generate_salt(&self) -> [u8; 32] {
        use rand::RngCore;
        let mut salt = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut salt);
        salt
    }
}

/// FALCON-1024 algorithm parameters
#[derive(Clone, Debug)]
pub struct FalconParameters {
    pub name: String,
    pub security_level: u32,
    pub public_key_size: usize,
    pub secret_key_size: usize,
    pub signature_size_average: usize,
    pub signature_size_max: usize,
    pub nist_pqc_round: u32,
    pub standardized: bool,
    pub lattice_based: bool,
    pub performance_tier: String,
}

impl std::fmt::Display for FalconParameters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, 
            "{}: Security Level {}, Pub:{} bytes, Sec:{} bytes, Sig:~{} bytes, Lattice-based: {}", 
            self.name, 
            self.security_level, 
            self.public_key_size, 
            self.secret_key_size, 
            self.signature_size_average,
            self.lattice_based
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyUsage;
    
    #[tokio::test]
    async fn test_falcon_crypto_initialization() {
        let falcon = FalconCrypto::new().unwrap();
        assert_eq!(falcon.algorithm_id, "FALCON-1024");
    }
    
    #[tokio::test]
    async fn test_falcon_keypair_generation() {
        let falcon = FalconCrypto::new().unwrap();
        
        let keypair = falcon.generate_keypair(KeyUsage::CertificateAuthority).await.unwrap();
        
        assert_eq!(keypair.key_usage, KeyUsage::CertificateAuthority);
        assert_eq!(keypair.public_key.key_bytes.len(), falcon1024::public_key_bytes());
        assert_eq!(keypair.private_key.key_bytes.len(), falcon1024::secret_key_bytes());
        assert_ne!(keypair.public_key.fingerprint, [0u8; 32]);
    }
    
    #[tokio::test]
    async fn test_falcon_sign_verify_roundtrip() {
        let falcon = FalconCrypto::new().unwrap();
        let keypair = falcon.generate_keypair(KeyUsage::AssetAuthentication).await.unwrap();
        
        let test_data = b"Test data for FALCON-1024 signature";
        
        // Sign the data
        let signature = falcon.sign(test_data, &keypair.private_key).await.unwrap();
        
        assert_eq!(signature.algorithm, "FALCON-1024");
        assert!(!signature.signature_bytes.is_empty());
        assert_ne!(signature.message_hash, [0u8; 32]);
        
        // Verify the signature
        let is_valid = falcon.verify(test_data, &signature, &keypair.public_key).await.unwrap();
        assert!(is_valid);
        
        // Test with tampered data
        let tampered_data = b"Tampered data for FALCON-1024 signature";
        let is_invalid = falcon.verify(tampered_data, &signature, &keypair.public_key).await.unwrap();
        assert!(!is_invalid);
    }
    
    #[tokio::test]
    async fn test_falcon_keypair_validation() {
        let falcon = FalconCrypto::new().unwrap();
        let keypair = falcon.generate_keypair(KeyUsage::RemoteProxyAuth).await.unwrap();
        
        let is_valid = falcon.validate_keypair(&keypair).unwrap();
        assert!(is_valid);
        
        // Test with corrupted keypair
        let mut corrupted_keypair = keypair.clone();
        corrupted_keypair.public_key.key_bytes[0] ^= 0xFF; // Flip bits
        
        let is_invalid = falcon.validate_keypair(&corrupted_keypair).unwrap();
        assert!(!is_invalid);
    }
    
    #[tokio::test]
    async fn test_falcon_algorithm_parameters() {
        let falcon = FalconCrypto::new().unwrap();
        let params = falcon.get_algorithm_parameters();
        
        assert_eq!(params.name, "FALCON-1024");
        assert_eq!(params.security_level, 128);
        assert_eq!(params.public_key_size, falcon1024::public_key_bytes());
        assert_eq!(params.secret_key_size, falcon1024::secret_key_bytes());
        assert!(params.lattice_based);
        assert_eq!(params.nist_pqc_round, 3);
    }
    
    #[tokio::test]
    async fn test_falcon_signature_sizes() {
        let falcon = FalconCrypto::new().unwrap();
        let keypair = falcon.generate_keypair(KeyUsage::GeneralSigning).await.unwrap();
        
        let test_data = b"Size test for FALCON-1024 signatures";
        let signature = falcon.sign(test_data, &keypair.private_key).await.unwrap();
        
        // FALCON-1024 signatures are variable length but should be reasonable
        assert!(signature.signature_bytes.len() > 400);  // Minimum reasonable size
        assert!(signature.signature_bytes.len() <= falcon1024::signature_bytes()); // Maximum size
        
        println!("FALCON-1024 signature size: {} bytes", signature.signature_bytes.len());
    }
    
    #[tokio::test]
    async fn test_multiple_keypairs() {
        let falcon = FalconCrypto::new().unwrap();
        
        let keypair1 = falcon.generate_keypair(KeyUsage::CertificateAuthority).await.unwrap();
        let keypair2 = falcon.generate_keypair(KeyUsage::AssetAuthentication).await.unwrap();
        
        // Keypairs should be different
        assert_ne!(keypair1.public_key.key_bytes, keypair2.public_key.key_bytes);
        assert_ne!(keypair1.private_key.key_bytes, keypair2.private_key.key_bytes);
        assert_ne!(keypair1.public_key.fingerprint, keypair2.public_key.fingerprint);
        
        // Cross-verification should fail
        let test_data = b"Cross-verification test";
        let signature1 = falcon.sign(test_data, &keypair1.private_key).await.unwrap();
        
        let is_cross_valid = falcon.verify(test_data, &signature1, &keypair2.public_key).await.unwrap();
        assert!(!is_cross_valid);
    }
}