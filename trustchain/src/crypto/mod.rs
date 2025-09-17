//! Post-Quantum Cryptography Module for TrustChain
//!
//! Implements FALCON-1024 post-quantum signatures and Kyber encryption
//! for quantum-resistant certificate authority operations and asset authentication.
//!
//! CRITICAL: This module replaces classical cryptography (ed25519, RSA) with
//! quantum-resistant alternatives to protect against future quantum attacks.

use std::fmt;
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};
use tracing::{info, debug, warn, error};

pub mod falcon;
pub mod kyber;
pub mod hybrid;
pub mod certificate;

pub use falcon::*;
pub use kyber::*;
pub use hybrid::*;
pub use certificate::*;

/// Post-quantum cryptographic key pair for FALCON-1024
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FalconKeyPair {
    /// FALCON-1024 public key (897 bytes)
    pub public_key: FalconPublicKey,
    /// FALCON-1024 private key (1281 bytes)
    pub private_key: FalconPrivateKey,
    /// Key generation timestamp
    pub created_at: std::time::SystemTime,
    /// Key usage purpose
    pub key_usage: KeyUsage,
    /// Associated certificate authority ID
    pub ca_id: Option<String>,
}

/// FALCON-1024 public key wrapper
#[derive(Clone, Serialize, Deserialize)]
pub struct FalconPublicKey {
    /// Raw FALCON-1024 public key bytes (897 bytes)
    pub key_bytes: Vec<u8>,
    /// Key fingerprint (SHA-256 of key_bytes)
    pub fingerprint: [u8; 32],
}

/// FALCON-1024 private key wrapper
#[derive(Clone, Serialize, Deserialize)]
pub struct FalconPrivateKey {
    /// Raw FALCON-1024 private key bytes (1281 bytes)
    pub key_bytes: Vec<u8>,
    /// Key derivation salt (for key strengthening)
    pub salt: [u8; 32],
}

/// Post-quantum signature created with FALCON-1024
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FalconSignature {
    /// FALCON-1024 signature bytes (variable length, ~700 bytes average)
    pub signature_bytes: Vec<u8>,
    /// Signature algorithm identifier
    pub algorithm: String,
    /// Signing timestamp
    pub signed_at: std::time::SystemTime,
    /// Message hash that was signed (SHA-256)
    pub message_hash: [u8; 32],
}

/// Kyber key pair for post-quantum encryption
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KyberKeyPair {
    /// Kyber public key
    pub public_key: KyberPublicKey,
    /// Kyber private key
    pub private_key: KyberPrivateKey,
    /// Key generation timestamp
    pub created_at: std::time::SystemTime,
}

/// Kyber public key wrapper
#[derive(Clone, Serialize, Deserialize)]
pub struct KyberPublicKey {
    /// Raw Kyber public key bytes
    pub key_bytes: Vec<u8>,
    /// Key fingerprint
    pub fingerprint: [u8; 32],
}

/// Kyber private key wrapper
#[derive(Clone, Serialize, Deserialize)]
pub struct KyberPrivateKey {
    /// Raw Kyber private key bytes
    pub key_bytes: Vec<u8>,
    /// Key derivation salt
    pub salt: [u8; 32],
}

/// Key usage enumeration
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum KeyUsage {
    /// Certificate Authority signing
    CertificateAuthority,
    /// Certificate signing
    CertificateSigning,
    /// Asset authentication
    AssetAuthentication,
    /// Remote proxy authentication
    RemoteProxyAuth,
    /// Consensus validation
    ConsensusValidation,
    /// Encryption/Decryption
    Encryption,
    /// General purpose signing
    GeneralSigning,
}

/// Post-quantum cryptography algorithms supported
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PQCAlgorithm {
    /// FALCON-1024 signature algorithm
    Falcon1024,
    /// Kyber encryption algorithm
    Kyber1024,
    /// Hybrid classical + post-quantum
    HybridFalconEd25519,
    /// Hybrid encryption
    HybridKyberAES,
}

/// Post-quantum cryptographic error types
#[derive(thiserror::Error, Debug)]
pub enum PQCError {
    #[error("FALCON-1024 key generation failed: {message}")]
    FalconKeyGeneration { message: String },
    
    #[error("FALCON-1024 signature creation failed: {message}")]
    FalconSigningError { message: String },
    
    #[error("FALCON-1024 signature verification failed: {message}")]
    FalconVerificationError { message: String },
    
    #[error("Kyber encryption failed: {message}")]
    KyberEncryptionError { message: String },
    
    #[error("Kyber decryption failed: {message}")]
    KyberDecryptionError { message: String },
    
    #[error("Invalid key format: {message}")]
    InvalidKeyFormat { message: String },
    
    #[error("Cryptographic operation failed: {message}")]
    CryptographicFailure { message: String },
    
    #[error("Quantum resistance validation failed: {message}")]
    QuantumResistanceFailure { message: String },
}

impl fmt::Display for FalconPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FALCON-1024 PubKey: {}", hex::encode(&self.fingerprint[..8]))
    }
}

impl fmt::Debug for FalconPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FalconPublicKey")
            .field("key_bytes_len", &self.key_bytes.len())
            .field("fingerprint", &hex::encode(&self.fingerprint[..8]))
            .finish()
    }
}

impl fmt::Debug for FalconPrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FalconPrivateKey")
            .field("key_bytes_len", &self.key_bytes.len())
            .field("salt", &hex::encode(&self.salt[..8]))
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for KyberPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KyberPublicKey")
            .field("key_bytes_len", &self.key_bytes.len())
            .field("fingerprint", &hex::encode(&self.fingerprint[..8]))
            .finish()
    }
}

impl fmt::Debug for KyberPrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KyberPrivateKey")
            .field("key_bytes_len", &self.key_bytes.len())
            .field("salt", &hex::encode(&self.salt[..8]))
            .finish_non_exhaustive()
    }
}

/// Post-quantum cryptography manager for TrustChain
pub struct PostQuantumCrypto {
    /// Falcon implementation handler
    falcon: falcon::FalconCrypto,
    /// Kyber implementation handler
    kyber: kyber::KyberCrypto,
    /// Hybrid crypto handler
    hybrid: hybrid::HybridCrypto,
}

impl PostQuantumCrypto {
    /// Initialize post-quantum cryptography system
    pub fn new() -> Result<Self> {
        info!("🔐 Initializing post-quantum cryptography with FALCON-1024");
        
        let falcon = falcon::FalconCrypto::new()?;
        let kyber = kyber::KyberCrypto::new()?;
        let hybrid = hybrid::HybridCrypto::new()?;
        
        info!("✅ Post-quantum cryptography initialized successfully");
        Ok(Self {
            falcon,
            kyber,
            hybrid,
        })
    }
    
    /// Generate FALCON-1024 key pair for certificate authority
    pub async fn generate_ca_keypair(&self, ca_id: &str) -> Result<FalconKeyPair> {
        info!("🔑 Generating FALCON-1024 CA key pair for: {}", ca_id);
        
        let keypair = self.falcon.generate_keypair(KeyUsage::CertificateAuthority).await?;
        
        info!("✅ FALCON-1024 CA key pair generated: {}", keypair.public_key);
        Ok(FalconKeyPair {
            ca_id: Some(ca_id.to_string()),
            ..keypair
        })
    }
    
    /// Generate asset authentication key pair
    pub async fn generate_asset_keypair(&self) -> Result<FalconKeyPair> {
        info!("🔑 Generating FALCON-1024 asset authentication key pair");
        
        let keypair = self.falcon.generate_keypair(KeyUsage::AssetAuthentication).await?;
        
        info!("✅ FALCON-1024 asset key pair generated: {}", keypair.public_key);
        Ok(keypair)
    }
    
    /// Generate remote proxy authentication key pair
    pub async fn generate_proxy_keypair(&self) -> Result<FalconKeyPair> {
        info!("🔑 Generating FALCON-1024 remote proxy authentication key pair");
        
        let keypair = self.falcon.generate_keypair(KeyUsage::RemoteProxyAuth).await?;
        
        info!("✅ FALCON-1024 proxy key pair generated: {}", keypair.public_key);
        Ok(keypair)
    }
    
    /// Sign data with FALCON-1024
    pub async fn sign_with_falcon(
        &self,
        data: &[u8],
        private_key: &FalconPrivateKey,
    ) -> Result<FalconSignature> {
        debug!("🔏 Signing data with FALCON-1024 ({} bytes)", data.len());
        
        let signature = self.falcon.sign(data, private_key).await?;
        
        debug!("✅ FALCON-1024 signature created ({} bytes)", signature.signature_bytes.len());
        Ok(signature)
    }
    
    /// Verify FALCON-1024 signature
    pub async fn verify_falcon_signature(
        &self,
        data: &[u8],
        signature: &FalconSignature,
        public_key: &FalconPublicKey,
    ) -> Result<bool> {
        debug!("🔍 Verifying FALCON-1024 signature ({} bytes)", signature.signature_bytes.len());
        
        let is_valid = self.falcon.verify(data, signature, public_key).await?;
        
        if is_valid {
            debug!("✅ FALCON-1024 signature verification successful");
        } else {
            warn!("❌ FALCON-1024 signature verification failed");
        }
        
        Ok(is_valid)
    }
    
    /// Generate Kyber encryption key pair
    pub async fn generate_encryption_keypair(&self) -> Result<KyberKeyPair> {
        info!("🔑 Generating Kyber encryption key pair");
        
        let keypair = self.kyber.generate_keypair().await?;
        
        info!("✅ Kyber encryption key pair generated: fingerprint {}", hex::encode(&keypair.public_key.fingerprint[..8]));
        Ok(keypair)
    }
    
    /// Encrypt data with Kyber
    pub async fn encrypt_with_kyber(
        &self,
        data: &[u8],
        public_key: &KyberPublicKey,
    ) -> Result<Vec<u8>> {
        debug!("🔒 Encrypting data with Kyber ({} bytes)", data.len());
        
        let ciphertext = self.kyber.encrypt(data, public_key).await?;
        
        debug!("✅ Kyber encryption completed ({} bytes)", ciphertext.len());
        Ok(ciphertext)
    }
    
    /// Decrypt data with Kyber
    pub async fn decrypt_with_kyber(
        &self,
        ciphertext: &[u8],
        private_key: &KyberPrivateKey,
    ) -> Result<Vec<u8>> {
        debug!("🔓 Decrypting data with Kyber ({} bytes)", ciphertext.len());
        
        let plaintext = self.kyber.decrypt(ciphertext, private_key).await?;
        
        debug!("✅ Kyber decryption completed ({} bytes)", plaintext.len());
        Ok(plaintext)
    }
    
    /// Create hybrid signature (FALCON-1024 + classical for transition)
    pub async fn create_hybrid_signature(
        &self,
        data: &[u8],
        falcon_key: &FalconPrivateKey,
        ed25519_key: &ed25519_dalek::SigningKey,
    ) -> Result<hybrid::HybridSignature> {
        debug!("🔏 Creating hybrid signature (FALCON-1024 + Ed25519)");
        
        let signature = self.hybrid.create_hybrid_signature(data, falcon_key, ed25519_key).await?;
        
        debug!("✅ Hybrid signature created");
        Ok(signature)
    }
    
    /// Verify hybrid signature
    pub async fn verify_hybrid_signature(
        &self,
        data: &[u8],
        signature: &hybrid::HybridSignature,
        falcon_pubkey: &FalconPublicKey,
        ed25519_pubkey: &ed25519_dalek::VerifyingKey,
    ) -> Result<bool> {
        debug!("🔍 Verifying hybrid signature");
        
        let is_valid = self.hybrid.verify_hybrid_signature(
            data,
            signature,
            falcon_pubkey,
            ed25519_pubkey,
        ).await?;
        
        if is_valid {
            debug!("✅ Hybrid signature verification successful");
        } else {
            warn!("❌ Hybrid signature verification failed");
        }
        
        Ok(is_valid)
    }
    
    /// Validate quantum resistance of a key
    pub fn validate_quantum_resistance(&self, algorithm: &PQCAlgorithm) -> Result<bool> {
        match algorithm {
            PQCAlgorithm::Falcon1024 => {
                info!("✅ FALCON-1024 is quantum resistant (NIST PQC Round 3 finalist)");
                Ok(true)
            }
            PQCAlgorithm::Kyber1024 => {
                info!("✅ Kyber-1024 is quantum resistant (NIST PQC standard)");
                Ok(true)
            }
            PQCAlgorithm::HybridFalconEd25519 | PQCAlgorithm::HybridKyberAES => {
                info!("⚠️  Hybrid algorithm provides quantum resistance through PQC component");
                Ok(true)
            }
        }
    }
    
    /// Get cryptographic algorithm info
    pub fn get_algorithm_info(&self, algorithm: &PQCAlgorithm) -> String {
        match algorithm {
            PQCAlgorithm::Falcon1024 => {
                "FALCON-1024: Lattice-based signature scheme, NIST PQC finalist, ~700 byte signatures".to_string()
            }
            PQCAlgorithm::Kyber1024 => {
                "Kyber-1024: Lattice-based KEM, NIST PQC standard, 128-bit quantum security".to_string()
            }
            PQCAlgorithm::HybridFalconEd25519 => {
                "Hybrid FALCON-1024 + Ed25519: Quantum-resistant with classical fallback".to_string()
            }
            PQCAlgorithm::HybridKyberAES => {
                "Hybrid Kyber + AES: Quantum-resistant encryption with classical components".to_string()
            }
        }
    }
    
    /// Get performance characteristics
    pub fn get_performance_info(&self, algorithm: &PQCAlgorithm) -> String {
        match algorithm {
            PQCAlgorithm::Falcon1024 => {
                "Fast signing (~0.1ms), Fast verification (~0.05ms), Compact keys (897+1281 bytes)".to_string()
            }
            PQCAlgorithm::Kyber1024 => {
                "Fast encapsulation (~0.1ms), Fast decapsulation (~0.1ms), Moderate key sizes".to_string()
            }
            PQCAlgorithm::HybridFalconEd25519 => {
                "Combined performance of both algorithms, ~2x signature size".to_string()
            }
            PQCAlgorithm::HybridKyberAES => {
                "Near-native AES performance with quantum-resistant key exchange".to_string()
            }
        }
    }
}

impl Default for PostQuantumCrypto {
    fn default() -> Self {
        Self::new().expect("Failed to initialize post-quantum cryptography")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_post_quantum_crypto_initialization() {
        let pqc = PostQuantumCrypto::new().unwrap();
        
        // Verify quantum resistance validation
        assert!(pqc.validate_quantum_resistance(&PQCAlgorithm::Falcon1024).unwrap());
        assert!(pqc.validate_quantum_resistance(&PQCAlgorithm::Kyber1024).unwrap());
    }
    
    #[tokio::test]
    async fn test_falcon_keypair_generation() {
        let pqc = PostQuantumCrypto::new().unwrap();
        
        let ca_keypair = pqc.generate_ca_keypair("test-ca").await.unwrap();
        assert_eq!(ca_keypair.key_usage, KeyUsage::CertificateAuthority);
        assert_eq!(ca_keypair.ca_id, Some("test-ca".to_string()));
        
        let asset_keypair = pqc.generate_asset_keypair().await.unwrap();
        assert_eq!(asset_keypair.key_usage, KeyUsage::AssetAuthentication);
    }
    
    #[tokio::test]
    async fn test_falcon_sign_verify() {
        let pqc = PostQuantumCrypto::new().unwrap();
        let keypair = pqc.generate_ca_keypair("test-ca").await.unwrap();
        
        let test_data = b"Hello, Post-Quantum World!";
        let signature = pqc.sign_with_falcon(test_data, &keypair.private_key).await.unwrap();
        
        let is_valid = pqc.verify_falcon_signature(
            test_data,
            &signature,
            &keypair.public_key,
        ).await.unwrap();
        
        assert!(is_valid);
        
        // Test with tampered data
        let tampered_data = b"Hello, Tampered World!";
        let is_invalid = pqc.verify_falcon_signature(
            tampered_data,
            &signature,
            &keypair.public_key,
        ).await.unwrap();
        
        assert!(!is_invalid);
    }
    
    #[tokio::test]
    async fn test_kyber_encrypt_decrypt() {
        let pqc = PostQuantumCrypto::new().unwrap();
        let keypair = pqc.generate_encryption_keypair().await.unwrap();
        
        let test_data = b"Secret quantum-resistant message";
        let ciphertext = pqc.encrypt_with_kyber(test_data, &keypair.public_key).await.unwrap();
        
        let decrypted = pqc.decrypt_with_kyber(&ciphertext, &keypair.private_key).await.unwrap();
        
        assert_eq!(test_data, decrypted.as_slice());
    }
    
    #[tokio::test]
    async fn test_algorithm_info() {
        let pqc = PostQuantumCrypto::new().unwrap();
        
        let falcon_info = pqc.get_algorithm_info(&PQCAlgorithm::Falcon1024);
        assert!(falcon_info.contains("FALCON-1024"));
        assert!(falcon_info.contains("Lattice-based"));
        
        let kyber_info = pqc.get_algorithm_info(&PQCAlgorithm::Kyber1024);
        assert!(kyber_info.contains("Kyber-1024"));
        assert!(kyber_info.contains("NIST PQC"));
    }
}