//! Kyber Post-Quantum Encryption Implementation
//!
//! Provides Kyber-1024 key encapsulation mechanism (KEM) for quantum-resistant
//! encryption in the TrustChain ecosystem, supporting secure data transmission
//! and storage protection.

use std::time::SystemTime;
use anyhow::{Result, anyhow};
use tracing::{info, debug, warn, error};
use sha2::{Sha256, Digest};

use pqcrypto_kyber::kyber1024;
use pqcrypto_traits::kem::{PublicKey, SecretKey, Ciphertext, SharedSecret};

use super::{
    KyberKeyPair, KyberPublicKey, KyberPrivateKey, PQCError
};

/// Kyber-1024 cryptographic operations handler
pub struct KyberCrypto {
    /// Algorithm identifier
    algorithm_id: String,
}

/// Kyber encryption result containing ciphertext and shared secret
#[derive(Clone, Debug)]
pub struct KyberEncryptionResult {
    /// Encapsulated ciphertext
    pub ciphertext: Vec<u8>,
    /// Shared secret for symmetric encryption
    pub shared_secret: Vec<u8>,
    /// Encryption timestamp
    pub encrypted_at: SystemTime,
    /// Algorithm used
    pub algorithm: String,
}

/// Kyber decryption result
#[derive(Clone, Debug)]
pub struct KyberDecryptionResult {
    /// Decrypted plaintext
    pub plaintext: Vec<u8>,
    /// Decryption timestamp
    pub decrypted_at: SystemTime,
    /// Verification status
    pub verified: bool,
}

impl KyberCrypto {
    /// Initialize Kyber-1024 cryptographic system
    pub fn new() -> Result<Self> {
        info!("🔐 Initializing Kyber-1024 post-quantum encryption system");
        
        Ok(Self {
            algorithm_id: "Kyber-1024".to_string(),
        })
    }
    
    /// Generate Kyber-1024 key pair
    pub async fn generate_keypair(&self) -> Result<KyberKeyPair> {
        info!("🔑 Generating Kyber-1024 encryption key pair");
        
        // Generate Kyber-1024 key pair
        let (public_key_native, secret_key_native) = kyber1024::keypair();
        
        // Extract raw bytes
        let public_key_bytes = public_key_native.as_bytes().to_vec();
        let secret_key_bytes = secret_key_native.as_bytes().to_vec();
        
        // Validate key sizes
        if public_key_bytes.len() != kyber1024::public_key_bytes() {
            return Err(anyhow!(
                "Kyber-1024 public key size mismatch: expected {}, got {}",
                kyber1024::public_key_bytes(),
                public_key_bytes.len()
            ));
        }
        
        if secret_key_bytes.len() != kyber1024::secret_key_bytes() {
            return Err(anyhow!(
                "Kyber-1024 secret key size mismatch: expected {}, got {}",
                kyber1024::secret_key_bytes(),
                secret_key_bytes.len()
            ));
        }
        
        // Generate key fingerprint
        let fingerprint = self.calculate_key_fingerprint(&public_key_bytes);
        
        // Generate salt for key strengthening
        let salt = self.generate_salt();
        
        let public_key = KyberPublicKey {
            key_bytes: public_key_bytes,
            fingerprint,
        };
        
        let private_key = KyberPrivateKey {
            key_bytes: secret_key_bytes,
            salt,
        };
        
        let keypair = KyberKeyPair {
            public_key,
            private_key,
            created_at: SystemTime::now(),
        };
        
        info!("✅ Kyber-1024 key pair generated successfully");
        debug!("Public key size: {} bytes", keypair.public_key.key_bytes.len());
        debug!("Private key size: {} bytes", keypair.private_key.key_bytes.len());
        debug!("Key fingerprint: {}", hex::encode(&keypair.public_key.fingerprint[..8]));
        
        Ok(keypair)
    }
    
    /// Encrypt data using Kyber-1024 KEM + AES-GCM
    pub async fn encrypt(
        &self,
        data: &[u8],
        public_key: &KyberPublicKey,
    ) -> Result<Vec<u8>> {
        debug!("🔒 Encrypting {} bytes with Kyber-1024 KEM", data.len());
        
        // Validate public key size
        if public_key.key_bytes.len() != kyber1024::public_key_bytes() {
            return Err(PQCError::InvalidKeyFormat {
                message: format!(
                    "Invalid Kyber-1024 public key size: expected {}, got {}",
                    kyber1024::public_key_bytes(),
                    public_key.key_bytes.len()
                )
            }.into());
        }
        
        // Reconstruct public key from bytes
        let public_key_native = kyber1024::PublicKey::from_bytes(&public_key.key_bytes)
            .map_err(|e| PQCError::KyberEncryptionError {
                message: format!("Failed to reconstruct Kyber-1024 public key: {}", e)
            })?;
        
        // Perform Kyber-1024 key encapsulation
        let (ciphertext_kem, shared_secret) = kyber1024::encapsulate(&public_key_native);
        
        // Use shared secret as AES key for symmetric encryption
        let symmetric_key = self.derive_symmetric_key(shared_secret.as_bytes());
        let encrypted_data = self.aes_encrypt(data, &symmetric_key)?;
        
        // Combine KEM ciphertext with encrypted data
        let mut combined_ciphertext = Vec::new();
        combined_ciphertext.extend_from_slice(&(ciphertext_kem.as_bytes().len() as u32).to_be_bytes());
        combined_ciphertext.extend_from_slice(ciphertext_kem.as_bytes());
        combined_ciphertext.extend_from_slice(&encrypted_data);
        
        debug!("✅ Kyber-1024 encryption completed: {} bytes total", combined_ciphertext.len());
        Ok(combined_ciphertext)
    }
    
    /// Decrypt data using Kyber-1024 KEM + AES-GCM
    pub async fn decrypt(
        &self,
        ciphertext: &[u8],
        private_key: &KyberPrivateKey,
    ) -> Result<Vec<u8>> {
        debug!("🔓 Decrypting {} bytes with Kyber-1024 KEM", ciphertext.len());
        
        // Validate private key size
        if private_key.key_bytes.len() != kyber1024::secret_key_bytes() {
            return Err(PQCError::InvalidKeyFormat {
                message: format!(
                    "Invalid Kyber-1024 private key size: expected {}, got {}",
                    kyber1024::secret_key_bytes(),
                    private_key.key_bytes.len()
                )
            }.into());
        }
        
        // Parse combined ciphertext
        if ciphertext.len() < 4 {
            return Err(PQCError::KyberDecryptionError {
                message: "Ciphertext too short".to_string()
            }.into());
        }
        
        let kem_length = u32::from_be_bytes([ciphertext[0], ciphertext[1], ciphertext[2], ciphertext[3]]) as usize;
        
        if ciphertext.len() < 4 + kem_length {
            return Err(PQCError::KyberDecryptionError {
                message: "Invalid ciphertext format".to_string()
            }.into());
        }
        
        let kem_ciphertext = &ciphertext[4..4 + kem_length];
        let encrypted_data = &ciphertext[4 + kem_length..];
        
        // Validate KEM ciphertext size
        if kem_ciphertext.len() != kyber1024::ciphertext_bytes() {
            return Err(PQCError::KyberDecryptionError {
                message: format!(
                    "Invalid Kyber-1024 ciphertext size: expected {}, got {}",
                    kyber1024::ciphertext_bytes(),
                    kem_ciphertext.len()
                )
            }.into());
        }
        
        // Reconstruct private key and ciphertext from bytes
        let secret_key_native = kyber1024::SecretKey::from_bytes(&private_key.key_bytes)
            .map_err(|e| PQCError::KyberDecryptionError {
                message: format!("Failed to reconstruct Kyber-1024 secret key: {}", e)
            })?;
        
        let ciphertext_native = kyber1024::Ciphertext::from_bytes(kem_ciphertext)
            .map_err(|e| PQCError::KyberDecryptionError {
                message: format!("Failed to reconstruct Kyber-1024 ciphertext: {}", e)
            })?;
        
        // Perform Kyber-1024 key decapsulation
        let shared_secret = kyber1024::decapsulate(&ciphertext_native, &secret_key_native);
        
        // Derive symmetric key and decrypt data
        let symmetric_key = self.derive_symmetric_key(shared_secret.as_bytes());
        let decrypted_data = self.aes_decrypt(encrypted_data, &symmetric_key)?;
        
        debug!("✅ Kyber-1024 decryption completed: {} bytes", decrypted_data.len());
        Ok(decrypted_data)
    }
    
    /// Validate Kyber-1024 key pair consistency
    pub fn validate_keypair(&self, keypair: &KyberKeyPair) -> Result<bool> {
        debug!("🔍 Validating Kyber-1024 key pair consistency");
        
        // Check key sizes
        if keypair.public_key.key_bytes.len() != kyber1024::public_key_bytes() {
            warn!("❌ Invalid Kyber-1024 public key size: {}", keypair.public_key.key_bytes.len());
            return Ok(false);
        }
        
        if keypair.private_key.key_bytes.len() != kyber1024::secret_key_bytes() {
            warn!("❌ Invalid Kyber-1024 private key size: {}", keypair.private_key.key_bytes.len());
            return Ok(false);
        }
        
        // Verify fingerprint
        let calculated_fingerprint = self.calculate_key_fingerprint(&keypair.public_key.key_bytes);
        if calculated_fingerprint != keypair.public_key.fingerprint {
            warn!("❌ Kyber-1024 key fingerprint mismatch");
            return Ok(false);
        }
        
        // Test encryption/decryption round-trip
        let test_data = b"Kyber-1024 key pair validation test data";
        
        // Reconstruct keys for testing
        let public_key_native = kyber1024::PublicKey::from_bytes(&keypair.public_key.key_bytes)
            .map_err(|e| PQCError::InvalidKeyFormat {
                message: format!("Invalid public key: {}", e)
            })?;
        
        let secret_key_native = kyber1024::SecretKey::from_bytes(&keypair.private_key.key_bytes)
            .map_err(|e| PQCError::InvalidKeyFormat {
                message: format!("Invalid secret key: {}", e)
            })?;
        
        // Test KEM round-trip
        let (shared_secret1, ciphertext) = kyber1024::encapsulate(&public_key_native);
        let shared_secret2 = kyber1024::decapsulate(&ciphertext, &secret_key_native);
        
        if shared_secret1.as_bytes() == shared_secret2.as_bytes() {
            debug!("✅ Kyber-1024 key pair validation successful");
            Ok(true)
        } else {
            warn!("❌ Kyber-1024 KEM round-trip failed");
            Ok(false)
        }
    }
    
    /// Get Kyber-1024 algorithm parameters
    pub fn get_algorithm_parameters(&self) -> KyberParameters {
        KyberParameters {
            name: "Kyber-1024".to_string(),
            security_level: 128, // 128-bit quantum security
            public_key_size: kyber1024::public_key_bytes(),
            secret_key_size: kyber1024::secret_key_bytes(),
            ciphertext_size: kyber1024::ciphertext_bytes(),
            shared_secret_size: kyber1024::shared_secret_bytes(),
            nist_pqc_standard: true, // Kyber is the NIST PQC standard
            lattice_based: true,
            performance_tier: "Fast".to_string(),
        }
    }
    
    /// Internal: Derive AES key from Kyber shared secret
    fn derive_symmetric_key(&self, shared_secret: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"KYBER-1024-AES-KEY:");
        hasher.update(shared_secret);
        hasher.finalize().into()
    }
    
    /// Internal: AES-GCM encryption
    fn aes_encrypt(&self, data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, Key, Nonce, AeadCore, AeadInPlace, KeyInit};
        use rand::RngCore;
        
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let nonce = Aes256Gcm::generate_nonce(rand::thread_rng());
        
        let mut buffer = data.to_vec();
        let tag = cipher.encrypt_in_place_detached(&nonce, b"", &mut buffer)
            .map_err(|e| PQCError::KyberEncryptionError {
                message: format!("AES-GCM encryption failed: {}", e)
            })?;
        
        // Combine nonce + tag + ciphertext
        let mut result = Vec::new();
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&tag);
        result.extend_from_slice(&buffer);
        
        Ok(result)
    }
    
    /// Internal: AES-GCM decryption
    fn aes_decrypt(&self, ciphertext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, Key, Nonce, AeadInPlace, KeyInit, Tag};
        
        if ciphertext.len() < 12 + 16 { // nonce + tag minimum
            return Err(PQCError::KyberDecryptionError {
                message: "Encrypted data too short".to_string()
            }.into());
        }
        
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        
        let (nonce_bytes, rest) = ciphertext.split_at(12);
        let (tag_bytes, encrypted_data) = rest.split_at(16);
        
        let nonce = Nonce::from_slice(nonce_bytes);
        let tag = Tag::from_slice(tag_bytes);
        
        let mut buffer = encrypted_data.to_vec();
        cipher.decrypt_in_place_detached(nonce, b"", &mut buffer, tag)
            .map_err(|e| PQCError::KyberDecryptionError {
                message: format!("AES-GCM decryption failed: {}", e)
            })?;
        
        Ok(buffer)
    }
    
    /// Internal: Calculate key fingerprint
    fn calculate_key_fingerprint(&self, key_bytes: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"KYBER-1024-KEY:");
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

/// Kyber-1024 algorithm parameters
#[derive(Clone, Debug)]
pub struct KyberParameters {
    pub name: String,
    pub security_level: u32,
    pub public_key_size: usize,
    pub secret_key_size: usize,
    pub ciphertext_size: usize,
    pub shared_secret_size: usize,
    pub nist_pqc_standard: bool,
    pub lattice_based: bool,
    pub performance_tier: String,
}

impl std::fmt::Display for KyberParameters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, 
            "{}: Security Level {}, Pub:{} bytes, Sec:{} bytes, CT:{} bytes, NIST Standard: {}", 
            self.name, 
            self.security_level, 
            self.public_key_size, 
            self.secret_key_size, 
            self.ciphertext_size,
            self.nist_pqc_standard
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_kyber_crypto_initialization() {
        let kyber = KyberCrypto::new().unwrap();
        assert_eq!(kyber.algorithm_id, "Kyber-1024");
    }
    
    #[tokio::test]
    async fn test_kyber_keypair_generation() {
        let kyber = KyberCrypto::new().unwrap();
        
        let keypair = kyber.generate_keypair().await.unwrap();
        
        assert_eq!(keypair.public_key.key_bytes.len(), kyber1024::public_key_bytes());
        assert_eq!(keypair.private_key.key_bytes.len(), kyber1024::secret_key_bytes());
        assert_ne!(keypair.public_key.fingerprint, [0u8; 32]);
    }
    
    #[tokio::test]
    async fn test_kyber_encrypt_decrypt_roundtrip() {
        let kyber = KyberCrypto::new().unwrap();
        let keypair = kyber.generate_keypair().await.unwrap();
        
        let test_data = b"Secret message for Kyber-1024 encryption test";
        
        // Encrypt the data
        let ciphertext = kyber.encrypt(test_data, &keypair.public_key).await.unwrap();
        
        assert!(!ciphertext.is_empty());
        assert_ne!(ciphertext, test_data.to_vec());
        
        // Decrypt the data
        let decrypted = kyber.decrypt(&ciphertext, &keypair.private_key).await.unwrap();
        
        assert_eq!(decrypted, test_data.to_vec());
    }
    
    #[tokio::test]
    async fn test_kyber_keypair_validation() {
        let kyber = KyberCrypto::new().unwrap();
        let keypair = kyber.generate_keypair().await.unwrap();
        
        let is_valid = kyber.validate_keypair(&keypair).unwrap();
        assert!(is_valid);
        
        // Test with corrupted keypair
        let mut corrupted_keypair = keypair.clone();
        corrupted_keypair.public_key.key_bytes[0] ^= 0xFF; // Flip bits
        
        let is_invalid = kyber.validate_keypair(&corrupted_keypair).unwrap();
        assert!(!is_invalid);
    }
    
    #[tokio::test]
    async fn test_kyber_algorithm_parameters() {
        let kyber = KyberCrypto::new().unwrap();
        let params = kyber.get_algorithm_parameters();
        
        assert_eq!(params.name, "Kyber-1024");
        assert_eq!(params.security_level, 128);
        assert_eq!(params.public_key_size, kyber1024::public_key_bytes());
        assert_eq!(params.secret_key_size, kyber1024::secret_key_bytes());
        assert!(params.nist_pqc_standard);
        assert!(params.lattice_based);
    }
    
    #[tokio::test]
    async fn test_large_data_encryption() {
        let kyber = KyberCrypto::new().unwrap();
        let keypair = kyber.generate_keypair().await.unwrap();
        
        // Test with larger data (1MB)
        let large_data = vec![0xAA; 1024 * 1024];
        
        let ciphertext = kyber.encrypt(&large_data, &keypair.public_key).await.unwrap();
        let decrypted = kyber.decrypt(&ciphertext, &keypair.private_key).await.unwrap();
        
        assert_eq!(decrypted, large_data);
    }
    
    #[tokio::test]
    async fn test_multiple_keypairs() {
        let kyber = KyberCrypto::new().unwrap();
        
        let keypair1 = kyber.generate_keypair().await.unwrap();
        let keypair2 = kyber.generate_keypair().await.unwrap();
        
        // Keypairs should be different
        assert_ne!(keypair1.public_key.key_bytes, keypair2.public_key.key_bytes);
        assert_ne!(keypair1.private_key.key_bytes, keypair2.private_key.key_bytes);
        assert_ne!(keypair1.public_key.fingerprint, keypair2.public_key.fingerprint);
        
        // Cross-decryption should fail
        let test_data = b"Cross-decryption test";
        let ciphertext1 = kyber.encrypt(test_data, &keypair1.public_key).await.unwrap();
        
        // Attempting to decrypt with wrong key should fail
        let decrypt_result = kyber.decrypt(&ciphertext1, &keypair2.private_key).await;
        assert!(decrypt_result.is_err());
    }
}