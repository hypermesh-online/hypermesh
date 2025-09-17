//! Hybrid Post-Quantum + Classical Cryptography
//!
//! Provides hybrid cryptographic schemes combining FALCON-1024/Kyber with classical
//! algorithms for transition period and defense-in-depth security strategies.

use std::time::SystemTime;
use anyhow::{Result, anyhow};
use tracing::{info, debug, warn, error};
use serde::{Serialize, Deserialize};

use super::{
    FalconPrivateKey, FalconPublicKey, FalconSignature,
    KyberPrivateKey, KyberPublicKey, PQCError
};

/// Hybrid signature combining FALCON-1024 and Ed25519
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HybridSignature {
    /// FALCON-1024 post-quantum signature
    pub falcon_signature: FalconSignature,
    /// Ed25519 classical signature
    pub ed25519_signature: Ed25519Signature,
    /// Signature creation timestamp
    pub signed_at: SystemTime,
    /// Message hash that was signed
    pub message_hash: [u8; 32],
    /// Hybrid algorithm identifier
    pub algorithm: String,
}

/// Ed25519 signature wrapper for hybrid schemes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ed25519Signature {
    /// Ed25519 signature bytes (64 bytes)
    #[serde(with = "serde_arrays")]
    pub signature_bytes: [u8; 64],
    /// Algorithm identifier
    pub algorithm: String,
    /// Signing timestamp
    pub signed_at: SystemTime,
}

/// Hybrid encryption result combining Kyber and AES
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HybridEncryption {
    /// Kyber-encrypted AES key
    pub kyber_ciphertext: Vec<u8>,
    /// AES-encrypted data
    pub aes_ciphertext: Vec<u8>,
    /// Encryption timestamp
    pub encrypted_at: SystemTime,
    /// Hybrid algorithm identifier
    pub algorithm: String,
    /// AES initialization vector
    #[serde(with = "serde_arrays")]
    pub aes_nonce: [u8; 12],
    /// AES authentication tag
    #[serde(with = "serde_arrays")]
    pub aes_tag: [u8; 16],
}

/// Hybrid cryptography operations handler
pub struct HybridCrypto {
    /// Algorithm identifier
    algorithm_id: String,
    /// FALCON-1024 handler
    falcon: super::falcon::FalconCrypto,
    /// Kyber handler
    kyber: super::kyber::KyberCrypto,
}

impl HybridCrypto {
    /// Initialize hybrid cryptography system
    pub fn new() -> Result<Self> {
        info!("🔐 Initializing hybrid post-quantum + classical cryptography");
        
        let falcon = super::falcon::FalconCrypto::new()?;
        let kyber = super::kyber::KyberCrypto::new()?;
        
        Ok(Self {
            algorithm_id: "Hybrid-PQC".to_string(),
            falcon,
            kyber,
        })
    }
    
    /// Create hybrid signature (FALCON-1024 + Ed25519)
    pub async fn create_hybrid_signature(
        &self,
        data: &[u8],
        falcon_key: &FalconPrivateKey,
        ed25519_key: &ed25519_dalek::SigningKey,
    ) -> Result<HybridSignature> {
        debug!("🔏 Creating hybrid signature for {} bytes", data.len());
        
        // Create FALCON-1024 signature
        let falcon_signature = self.falcon.sign(data, falcon_key).await?;
        
        // Create Ed25519 signature
        let ed25519_signature = self.create_ed25519_signature(data, ed25519_key)?;
        
        // Calculate message hash
        let message_hash = self.hash_message(data);
        
        let hybrid_signature = HybridSignature {
            falcon_signature,
            ed25519_signature,
            signed_at: SystemTime::now(),
            message_hash,
            algorithm: "FALCON-1024+Ed25519".to_string(),
        };
        
        debug!("✅ Hybrid signature created successfully");
        Ok(hybrid_signature)
    }
    
    /// Verify hybrid signature (both FALCON-1024 and Ed25519 must be valid)
    pub async fn verify_hybrid_signature(
        &self,
        data: &[u8],
        signature: &HybridSignature,
        falcon_pubkey: &FalconPublicKey,
        ed25519_pubkey: &ed25519_dalek::VerifyingKey,
    ) -> Result<bool> {
        debug!("🔍 Verifying hybrid signature");
        
        // Verify message hash
        let message_hash = self.hash_message(data);
        if message_hash != signature.message_hash {
            warn!("❌ Message hash mismatch in hybrid signature verification");
            return Ok(false);
        }
        
        // Verify FALCON-1024 signature
        let falcon_valid = self.falcon.verify(data, &signature.falcon_signature, falcon_pubkey).await?;
        if !falcon_valid {
            warn!("❌ FALCON-1024 signature verification failed");
            return Ok(false);
        }
        
        // Verify Ed25519 signature
        let ed25519_valid = self.verify_ed25519_signature(data, &signature.ed25519_signature, ed25519_pubkey)?;
        if !ed25519_valid {
            warn!("❌ Ed25519 signature verification failed");
            return Ok(false);
        }
        
        debug!("✅ Hybrid signature verification successful (both algorithms valid)");
        Ok(true)
    }
    
    /// Create hybrid encryption (Kyber + AES)
    pub async fn create_hybrid_encryption(
        &self,
        data: &[u8],
        kyber_pubkey: &KyberPublicKey,
    ) -> Result<HybridEncryption> {
        debug!("🔒 Creating hybrid encryption for {} bytes", data.len());
        
        // Generate random AES key
        let aes_key = self.generate_aes_key();
        
        // Encrypt data with AES-GCM
        let (aes_ciphertext, aes_nonce, aes_tag) = self.aes_encrypt(data, &aes_key)?;
        
        // Encrypt AES key with Kyber
        let kyber_ciphertext = self.kyber.encrypt(&aes_key, kyber_pubkey).await?;
        
        let hybrid_encryption = HybridEncryption {
            kyber_ciphertext,
            aes_ciphertext,
            encrypted_at: SystemTime::now(),
            algorithm: "Kyber-1024+AES-256-GCM".to_string(),
            aes_nonce,
            aes_tag,
        };
        
        debug!("✅ Hybrid encryption created successfully");
        Ok(hybrid_encryption)
    }
    
    /// Decrypt hybrid encryption (Kyber + AES)
    pub async fn decrypt_hybrid_encryption(
        &self,
        hybrid_ciphertext: &HybridEncryption,
        kyber_privkey: &KyberPrivateKey,
    ) -> Result<Vec<u8>> {
        debug!("🔓 Decrypting hybrid encryption");
        
        // Decrypt AES key with Kyber
        let aes_key_bytes = self.kyber.decrypt(&hybrid_ciphertext.kyber_ciphertext, kyber_privkey).await?;
        
        if aes_key_bytes.len() != 32 {
            return Err(PQCError::KyberDecryptionError {
                message: format!("Invalid AES key length: {}", aes_key_bytes.len())
            }.into());
        }
        
        let mut aes_key = [0u8; 32];
        aes_key.copy_from_slice(&aes_key_bytes);
        
        // Decrypt data with AES-GCM
        let plaintext = self.aes_decrypt(
            &hybrid_ciphertext.aes_ciphertext,
            &aes_key,
            &hybrid_ciphertext.aes_nonce,
            &hybrid_ciphertext.aes_tag,
        )?;
        
        debug!("✅ Hybrid decryption completed: {} bytes", plaintext.len());
        Ok(plaintext)
    }
    
    /// Create migration signature (supporting both old and new keys)
    pub async fn create_migration_signature(
        &self,
        data: &[u8],
        old_key: &ed25519_dalek::SigningKey,
        new_key: &FalconPrivateKey,
    ) -> Result<MigrationSignature> {
        debug!("🔄 Creating migration signature for key transition");
        
        let ed25519_signature = self.create_ed25519_signature(data, old_key)?;
        let falcon_signature = self.falcon.sign(data, new_key).await?;
        
        Ok(MigrationSignature {
            legacy_signature: ed25519_signature,
            quantum_signature: falcon_signature,
            migration_timestamp: SystemTime::now(),
            message_hash: self.hash_message(data),
        })
    }
    
    /// Verify migration signature (either key can validate)
    pub async fn verify_migration_signature(
        &self,
        data: &[u8],
        signature: &MigrationSignature,
        ed25519_pubkey: Option<&ed25519_dalek::VerifyingKey>,
        falcon_pubkey: Option<&FalconPublicKey>,
    ) -> Result<MigrationVerificationResult> {
        debug!("🔍 Verifying migration signature");
        
        let message_hash = self.hash_message(data);
        if message_hash != signature.message_hash {
            return Ok(MigrationVerificationResult::Invalid("Message hash mismatch".to_string()));
        }
        
        let mut legacy_valid = false;
        let mut quantum_valid = false;
        
        // Try legacy verification
        if let Some(ed25519_key) = ed25519_pubkey {
            legacy_valid = self.verify_ed25519_signature(data, &signature.legacy_signature, ed25519_key)?;
        }
        
        // Try quantum verification
        if let Some(falcon_key) = falcon_pubkey {
            quantum_valid = self.falcon.verify(data, &signature.quantum_signature, falcon_key).await?;
        }
        
        match (legacy_valid, quantum_valid) {
            (true, true) => Ok(MigrationVerificationResult::BothValid),
            (true, false) => Ok(MigrationVerificationResult::LegacyOnly),
            (false, true) => Ok(MigrationVerificationResult::QuantumOnly),
            (false, false) => Ok(MigrationVerificationResult::Invalid("No valid signatures".to_string())),
        }
    }
    
    /// Get hybrid algorithm security assessment
    pub fn get_security_assessment(&self, algorithm: &str) -> SecurityAssessment {
        match algorithm {
            "FALCON-1024+Ed25519" => SecurityAssessment {
                quantum_resistant: true,
                classical_secure: true,
                combined_security_level: 128,
                recommended_use: "Certificate authority operations during quantum transition".to_string(),
                performance_impact: "Moderate (2x signature size, 1.5x verification time)".to_string(),
                transition_timeline: "Phase out Ed25519 by 2030".to_string(),
            },
            "Kyber-1024+AES-256-GCM" => SecurityAssessment {
                quantum_resistant: true,
                classical_secure: true,
                combined_security_level: 256,
                recommended_use: "High-security data encryption and key exchange".to_string(),
                performance_impact: "Low (minimal overhead over AES-256)".to_string(),
                transition_timeline: "Immediate adoption recommended".to_string(),
            },
            _ => SecurityAssessment {
                quantum_resistant: false,
                classical_secure: false,
                combined_security_level: 0,
                recommended_use: "Unknown algorithm".to_string(),
                performance_impact: "Unknown".to_string(),
                transition_timeline: "Not assessed".to_string(),
            }
        }
    }
    
    /// Internal: Create Ed25519 signature
    fn create_ed25519_signature(
        &self,
        data: &[u8],
        secret_key: &ed25519_dalek::SigningKey,
    ) -> Result<Ed25519Signature> {
        use ed25519_dalek::Signer;
        
        let signature = secret_key.sign(data);
        
        Ok(Ed25519Signature {
            signature_bytes: signature.to_bytes(),
            algorithm: "Ed25519".to_string(),
            signed_at: SystemTime::now(),
        })
    }
    
    /// Internal: Verify Ed25519 signature
    fn verify_ed25519_signature(
        &self,
        data: &[u8],
        signature: &Ed25519Signature,
        public_key: &ed25519_dalek::VerifyingKey,
    ) -> Result<bool> {
        use ed25519_dalek::{Signature, Verifier};
        
        let signature_obj = Signature::from_bytes(&signature.signature_bytes);
        
        match public_key.verify(data, &signature_obj) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    /// Internal: Generate AES-256 key
    fn generate_aes_key(&self) -> [u8; 32] {
        use rand::RngCore;
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        key
    }
    
    /// Internal: AES-GCM encryption
    fn aes_encrypt(&self, data: &[u8], key: &[u8; 32]) -> Result<(Vec<u8>, [u8; 12], [u8; 16])> {
        use aes_gcm::{Aes256Gcm, Key, Nonce, AeadCore, AeadInPlace, KeyInit};
        use rand::RngCore;
        
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let nonce = Aes256Gcm::generate_nonce(rand::thread_rng());
        
        let mut buffer = data.to_vec();
        let tag = cipher.encrypt_in_place_detached(&nonce, b"", &mut buffer)
            .map_err(|e| PQCError::KyberEncryptionError {
                message: format!("AES-GCM encryption failed: {}", e)
            })?;
        
        Ok((buffer, nonce.into(), tag.into()))
    }
    
    /// Internal: AES-GCM decryption
    fn aes_decrypt(
        &self,
        ciphertext: &[u8],
        key: &[u8; 32],
        nonce: &[u8; 12],
        tag: &[u8; 16],
    ) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, Key, Nonce, AeadInPlace, KeyInit, Tag};
        
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let nonce_obj = Nonce::from_slice(nonce);
        let tag_obj = Tag::from_slice(tag);
        
        let mut buffer = ciphertext.to_vec();
        cipher.decrypt_in_place_detached(nonce_obj, b"", &mut buffer, tag_obj)
            .map_err(|e| PQCError::KyberDecryptionError {
                message: format!("AES-GCM decryption failed: {}", e)
            })?;
        
        Ok(buffer)
    }
    
    /// Internal: Hash message
    fn hash_message(&self, data: &[u8]) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }
}

/// Migration signature for key transition periods
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MigrationSignature {
    /// Legacy Ed25519 signature
    pub legacy_signature: Ed25519Signature,
    /// New FALCON-1024 signature
    pub quantum_signature: FalconSignature,
    /// Migration timestamp
    pub migration_timestamp: SystemTime,
    /// Message hash
    pub message_hash: [u8; 32],
}

/// Migration verification result
#[derive(Clone, Debug, PartialEq)]
pub enum MigrationVerificationResult {
    /// Both signatures are valid
    BothValid,
    /// Only legacy signature is valid
    LegacyOnly,
    /// Only quantum signature is valid
    QuantumOnly,
    /// No valid signatures
    Invalid(String),
}

/// Security assessment for hybrid algorithms
#[derive(Clone, Debug)]
pub struct SecurityAssessment {
    pub quantum_resistant: bool,
    pub classical_secure: bool,
    pub combined_security_level: u32,
    pub recommended_use: String,
    pub performance_impact: String,
    pub transition_timeline: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    
    #[tokio::test]
    async fn test_hybrid_crypto_initialization() {
        let hybrid = HybridCrypto::new().unwrap();
        assert_eq!(hybrid.algorithm_id, "Hybrid-PQC");
    }
    
    #[tokio::test]
    async fn test_hybrid_signature_creation_and_verification() {
        let hybrid = HybridCrypto::new().unwrap();
        
        // Generate keys
        let falcon_keypair = hybrid.falcon.generate_keypair(super::KeyUsage::CertificateAuthority).await.unwrap();
        let ed25519_key = SigningKey::from_bytes(&rand::random::<[u8; 32]>());
        let ed25519_pubkey = ed25519_key.verifying_key();
        
        let test_data = b"Hybrid signature test data";
        
        // Create hybrid signature
        let signature = hybrid.create_hybrid_signature(
            test_data,
            &falcon_keypair.private_key,
            &ed25519_key,
        ).await.unwrap();
        
        // Verify hybrid signature
        let is_valid = hybrid.verify_hybrid_signature(
            test_data,
            &signature,
            &falcon_keypair.public_key,
            &ed25519_pubkey,
        ).await.unwrap();
        
        assert!(is_valid);
    }
    
    #[tokio::test]
    async fn test_hybrid_encryption_decryption() {
        let hybrid = HybridCrypto::new().unwrap();
        
        let kyber_keypair = hybrid.kyber.generate_keypair().await.unwrap();
        let test_data = b"Secret hybrid encryption test data";
        
        // Create hybrid encryption
        let encrypted = hybrid.create_hybrid_encryption(test_data, &kyber_keypair.public_key).await.unwrap();
        
        // Decrypt hybrid encryption
        let decrypted = hybrid.decrypt_hybrid_encryption(&encrypted, &kyber_keypair.private_key).await.unwrap();
        
        assert_eq!(decrypted, test_data.to_vec());
    }
    
    #[tokio::test]
    async fn test_migration_signature() {
        let hybrid = HybridCrypto::new().unwrap();
        
        let ed25519_key = SigningKey::from_bytes(&rand::random::<[u8; 32]>());
        let ed25519_pubkey = ed25519_key.verifying_key();
        let falcon_keypair = hybrid.falcon.generate_keypair(super::KeyUsage::CertificateAuthority).await.unwrap();
        
        let test_data = b"Migration signature test";
        
        // Create migration signature
        let migration_sig = hybrid.create_migration_signature(
            test_data,
            &ed25519_key,
            &falcon_keypair.private_key,
        ).await.unwrap();
        
        // Verify with both keys
        let result = hybrid.verify_migration_signature(
            test_data,
            &migration_sig,
            Some(&ed25519_pubkey),
            Some(&falcon_keypair.public_key),
        ).await.unwrap();
        
        assert_eq!(result, MigrationVerificationResult::BothValid);
        
        // Verify with only legacy key
        let legacy_result = hybrid.verify_migration_signature(
            test_data,
            &migration_sig,
            Some(&ed25519_pubkey),
            None,
        ).await.unwrap();
        
        assert_eq!(legacy_result, MigrationVerificationResult::LegacyOnly);
        
        // Verify with only quantum key
        let quantum_result = hybrid.verify_migration_signature(
            test_data,
            &migration_sig,
            None,
            Some(&falcon_keypair.public_key),
        ).await.unwrap();
        
        assert_eq!(quantum_result, MigrationVerificationResult::QuantumOnly);
    }
    
    #[tokio::test]
    async fn test_security_assessments() {
        let hybrid = HybridCrypto::new().unwrap();
        
        let falcon_assessment = hybrid.get_security_assessment("FALCON-1024+Ed25519");
        assert!(falcon_assessment.quantum_resistant);
        assert!(falcon_assessment.classical_secure);
        assert_eq!(falcon_assessment.combined_security_level, 128);
        
        let kyber_assessment = hybrid.get_security_assessment("Kyber-1024+AES-256-GCM");
        assert!(kyber_assessment.quantum_resistant);
        assert!(kyber_assessment.classical_secure);
        assert_eq!(kyber_assessment.combined_security_level, 256);
        
        let unknown_assessment = hybrid.get_security_assessment("Unknown");
        assert!(!unknown_assessment.quantum_resistant);
        assert!(!unknown_assessment.classical_secure);
        assert_eq!(unknown_assessment.combined_security_level, 0);
    }
}