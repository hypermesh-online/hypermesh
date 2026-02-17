// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Quantum-Resistant Security for Proxy System
//!
//! Implements FALCON-1024 signatures and Kyber encryption for quantum-resistant security

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use pqcrypto_falcon::falcon1024;
use pqcrypto_kyber::kyber1024;
use pqcrypto_traits::sign::{PublicKey as SignPublicKey, SecretKey as SignSecretKey, DetachedSignature};
use pqcrypto_traits::kem::{PublicKey as KemPublicKey, SecretKey as KemSecretKey, Ciphertext, SharedSecret};
use aes_gcm::{Aes256Gcm, Key, AeadCore, AeadInPlace, KeyInit};

use crate::assets::core::{AssetResult, AssetError, ProxyAddress};

/// Quantum-resistant security handler
pub struct QuantumSecurity {
    /// FALCON-1024 signer
    falcon_signer: FalconSigner,
    
    /// Kyber encryption handler
    kyber_encryption: KyberEncryption,
    
    /// Active security tokens
    active_tokens: HashMap<String, SecurityToken>,
    
    /// Security configuration
    config: SecurityConfig,
}

/// FALCON-1024 digital signature system
pub struct FalconSigner {
    /// FALCON-1024 secret key bytes
    secret_key_bytes: Vec<u8>,

    /// FALCON-1024 public key bytes
    public_key_bytes: Vec<u8>,
}

/// Kyber-1024 post-quantum encryption system (KEM + AES-GCM)
pub struct KyberEncryption {
    /// Kyber-1024 secret key bytes
    secret_key_bytes: Vec<u8>,

    /// Kyber-1024 public key bytes
    public_key_bytes: Vec<u8>,
}

/// Security token for quantum-resistant authentication
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityToken {
    /// Token identifier
    pub token_id: String,
    
    /// Associated proxy address
    pub proxy_address: ProxyAddress,
    
    /// FALCON-1024 signature
    pub signature: Vec<u8>,
    
    /// Kyber encrypted payload
    pub encrypted_payload: Vec<u8>,
    
    /// Token creation timestamp
    pub created_at: SystemTime,
    
    /// Token expiration timestamp
    pub expires_at: SystemTime,
    
    /// Token validation status
    pub validation_status: TokenValidationStatus,
}

/// Token validation status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TokenValidationStatus {
    /// Token is valid and active
    Valid,
    
    /// Token has expired
    Expired,
    
    /// Token signature is invalid
    InvalidSignature,
    
    /// Token encryption is invalid
    InvalidEncryption,
    
    /// Token has been revoked
    Revoked,
}

/// Security configuration
#[derive(Clone, Debug)]
pub struct SecurityConfig {
    /// Token lifetime duration
    token_lifetime: Duration,
    
    /// Signature validation timeout
    signature_timeout: Duration,
    
    /// Enable signature caching
    enable_signature_caching: bool,
    
    /// Enable encryption caching
    enable_encryption_caching: bool,
    
    /// Maximum cache size
    max_cache_size: usize,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            token_lifetime: Duration::from_secs(3600), // 1 hour
            signature_timeout: Duration::from_secs(30),
            enable_signature_caching: true,
            enable_encryption_caching: true,
            max_cache_size: 10000,
        }
    }
}

impl QuantumSecurity {
    /// Create new quantum security handler
    pub async fn new() -> AssetResult<Self> {
        Ok(Self {
            falcon_signer: FalconSigner::new()?,
            kyber_encryption: KyberEncryption::new()?,
            active_tokens: HashMap::new(),
            config: SecurityConfig::default(),
        })
    }
    
    /// Generate quantum-resistant access tokens
    pub async fn generate_access_tokens(&self, proxy_addr: &ProxyAddress) -> AssetResult<Vec<u8>> {
        // Create token payload
        let token_payload = self.create_token_payload(proxy_addr)?;
        
        // Sign with FALCON-1024
        let signature = self.falcon_signer.sign(&token_payload).await?;
        
        // Encrypt with Kyber
        let encrypted_payload = self.kyber_encryption.encrypt(&token_payload).await?;
        
        // Create security token
        let token_id = self.generate_token_id(proxy_addr)?;
        let token = SecurityToken {
            token_id: token_id.clone(),
            proxy_address: proxy_addr.clone(),
            signature,
            encrypted_payload,
            created_at: SystemTime::now(),
            expires_at: SystemTime::now() + self.config.token_lifetime,
            validation_status: TokenValidationStatus::Valid,
        };

        // Store active token and keep a clone for return
        let token_clone = token.clone();
        self.store_active_token(token_id, token).await;

        // Return combined: [4-byte sig_len][signature][encrypted_payload]
        let sig_len = token_clone.signature.len() as u32;
        let mut access_tokens = Vec::with_capacity(4 + token_clone.signature.len() + token_clone.encrypted_payload.len());
        access_tokens.extend_from_slice(&sig_len.to_be_bytes());
        access_tokens.extend_from_slice(&token_clone.signature);
        access_tokens.extend_from_slice(&token_clone.encrypted_payload);

        tracing::debug!("Generated quantum security tokens for proxy address: {}", proxy_addr);
        Ok(access_tokens)
    }
    
    /// Validate quantum-resistant access tokens
    pub async fn validate_access_tokens(&self, tokens: &[u8]) -> AssetResult<bool> {
        // Format: [4-byte sig_len][signature][encrypted_payload]
        if tokens.len() < 4 {
            return Ok(false);
        }

        let sig_len = u32::from_be_bytes([tokens[0], tokens[1], tokens[2], tokens[3]]) as usize;
        if tokens.len() < 4 + sig_len {
            return Ok(false);
        }

        let signature = &tokens[4..4 + sig_len];
        let encrypted_payload = &tokens[4 + sig_len..];
        
        // Decrypt payload
        let payload = match self.kyber_encryption.decrypt(encrypted_payload).await {
            Ok(p) => p,
            Err(_) => return Ok(false),
        };
        
        // Verify signature
        let signature_valid = match self.falcon_signer.verify(&payload, signature).await {
            Ok(valid) => valid,
            Err(_) => false,
        };
        
        if !signature_valid {
            tracing::warn!("Quantum security token signature validation failed");
            return Ok(false);
        }
        
        // Validate payload structure and expiration
        let token_valid = self.validate_token_payload(&payload).await?;
        
        tracing::debug!("Quantum security token validation result: {}", token_valid);
        Ok(token_valid)
    }
    
    /// Create token payload for signing/encryption
    fn create_token_payload(&self, proxy_addr: &ProxyAddress) -> AssetResult<Vec<u8>> {
        let mut payload = Vec::new();
        
        // Add proxy address components
        payload.extend_from_slice(&proxy_addr.network_id);
        payload.extend_from_slice(&proxy_addr.node_id);
        payload.extend_from_slice(&proxy_addr.asset_port.to_le_bytes());
        payload.extend_from_slice(&proxy_addr.access_token);
        
        // Add timestamp
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| AssetError::AdapterError {
                message: "Invalid system time".to_string()
            })?
            .as_secs();
        payload.extend_from_slice(&timestamp.to_le_bytes());
        
        // Add random nonce
        let nonce: u64 = fastrand::u64(..);
        payload.extend_from_slice(&nonce.to_le_bytes());
        
        Ok(payload)
    }
    
    /// Generate unique token ID
    fn generate_token_id(&self, proxy_addr: &ProxyAddress) -> AssetResult<String> {
        let mut hasher = Sha256::new();
        hasher.update(&proxy_addr.network_id);
        hasher.update(&proxy_addr.node_id);
        hasher.update(&proxy_addr.asset_port.to_le_bytes());
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| AssetError::AdapterError {
                message: "Invalid system time for token ID generation".to_string()
            })?
            .as_nanos();
        hasher.update(&nanos.to_le_bytes());

        let hash = hasher.finalize();
        Ok(hex::encode(&hash[..16])) // Use first 16 bytes as token ID
    }
    
    /// Store active token
    async fn store_active_token(&self, token_id: String, token: SecurityToken) {
        // TODO: In real implementation, this would be thread-safe
        // For now, we'll simulate token storage
        tracing::debug!("Stored security token: {}", token_id);
    }
    
    /// Validate token payload structure and expiration
    async fn validate_token_payload(&self, payload: &[u8]) -> AssetResult<bool> {
        if payload.len() < 32 { // Minimum expected size
            return Ok(false);
        }
        
        // Extract timestamp from payload (last 8 bytes before nonce)
        if payload.len() >= 16 {
            let timestamp_bytes = &payload[payload.len() - 16..payload.len() - 8];
            let timestamp = u64::from_le_bytes(timestamp_bytes.try_into().map_err(|_| AssetError::AdapterError {
                message: "Invalid timestamp bytes in token payload".to_string()
            })?);
            
            let token_time = SystemTime::UNIX_EPOCH + Duration::from_secs(timestamp);
            let now = SystemTime::now();
            
            // Check if token has expired
            if token_time + self.config.token_lifetime < now {
                return Ok(false);
            }
            
            // Check if token is from the future (clock skew protection)
            if token_time > now + Duration::from_secs(300) { // 5 minute tolerance
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    /// Revoke security token
    pub async fn revoke_token(&self, token_id: &str) -> AssetResult<()> {
        // TODO: Implement token revocation
        tracing::info!("Revoked security token: {}", token_id);
        Ok(())
    }
    
    /// Cleanup expired tokens
    pub async fn cleanup_expired_tokens(&self) -> AssetResult<u64> {
        // TODO: Implement expired token cleanup
        tracing::debug!("Cleaned up expired security tokens");
        Ok(0)
    }
}

impl FalconSigner {
    /// Create new FALCON-1024 signer with a real keypair
    fn new() -> AssetResult<Self> {
        let (pk, sk) = falcon1024::keypair();

        Ok(Self {
            public_key_bytes: pk.as_bytes().to_vec(),
            secret_key_bytes: sk.as_bytes().to_vec(),
        })
    }

    /// Sign data with FALCON-1024
    pub async fn sign(&self, data: &[u8]) -> AssetResult<Vec<u8>> {
        let sk = falcon1024::SecretKey::from_bytes(&self.secret_key_bytes)
            .map_err(|e| AssetError::AdapterError {
                message: format!("Invalid FALCON-1024 secret key: {}", e),
            })?;

        let sig = falcon1024::detached_sign(data, &sk);
        let sig_bytes = sig.as_bytes().to_vec();

        tracing::debug!("Created FALCON-1024 signature ({} bytes)", sig_bytes.len());
        Ok(sig_bytes)
    }

    /// Verify FALCON-1024 signature
    pub async fn verify(&self, data: &[u8], signature: &[u8]) -> AssetResult<bool> {
        let pk = falcon1024::PublicKey::from_bytes(&self.public_key_bytes)
            .map_err(|e| AssetError::AdapterError {
                message: format!("Invalid FALCON-1024 public key: {}", e),
            })?;

        let sig = match falcon1024::DetachedSignature::from_bytes(signature) {
            Ok(s) => s,
            Err(_) => {
                tracing::debug!("FALCON-1024 signature verification: invalid signature format");
                return Ok(false);
            }
        };

        let valid = falcon1024::verify_detached_signature(&sig, data, &pk).is_ok();
        tracing::debug!("FALCON-1024 signature verification result: {}", valid);
        Ok(valid)
    }

    /// Get public key bytes for verification
    pub fn get_public_key(&self) -> &[u8] {
        &self.public_key_bytes
    }
}

impl KyberEncryption {
    /// Create new Kyber-1024 encryption handler with a real keypair
    fn new() -> AssetResult<Self> {
        let (pk, sk) = kyber1024::keypair();

        Ok(Self {
            public_key_bytes: pk.as_bytes().to_vec(),
            secret_key_bytes: sk.as_bytes().to_vec(),
        })
    }

    /// Encrypt data with Kyber-1024 KEM + AES-GCM
    ///
    /// Output format: [4-byte KEM ciphertext length][KEM ciphertext][12-byte nonce][16-byte tag][AES ciphertext]
    pub async fn encrypt(&self, data: &[u8]) -> AssetResult<Vec<u8>> {
        let pk = kyber1024::PublicKey::from_bytes(&self.public_key_bytes)
            .map_err(|e| AssetError::AdapterError {
                message: format!("Invalid Kyber-1024 public key: {}", e),
            })?;

        // KEM encapsulation produces a shared secret and ciphertext
        let (shared_secret, kem_ct) = kyber1024::encapsulate(&pk);

        // Derive AES-256 key from shared secret
        let aes_key = Self::derive_aes_key(shared_secret.as_bytes());

        // AES-GCM encrypt the data
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&aes_key));
        let nonce = Aes256Gcm::generate_nonce(rand::thread_rng());

        let mut buffer = data.to_vec();
        let tag = cipher.encrypt_in_place_detached(&nonce, b"", &mut buffer)
            .map_err(|e| AssetError::AdapterError {
                message: format!("AES-GCM encryption failed: {}", e),
            })?;

        // Build output: [kem_ct_len (4 bytes)][kem_ct][nonce (12)][tag (16)][aes_ct]
        let kem_ct_bytes = kem_ct.as_bytes();
        let mut output = Vec::with_capacity(4 + kem_ct_bytes.len() + 12 + 16 + buffer.len());
        output.extend_from_slice(&(kem_ct_bytes.len() as u32).to_be_bytes());
        output.extend_from_slice(kem_ct_bytes);
        output.extend_from_slice(&nonce);
        output.extend_from_slice(&tag);
        output.extend_from_slice(&buffer);

        tracing::debug!("Kyber-1024 encrypted {} bytes -> {} bytes", data.len(), output.len());
        Ok(output)
    }

    /// Decrypt data with Kyber-1024 KEM + AES-GCM
    pub async fn decrypt(&self, encrypted_data: &[u8]) -> AssetResult<Vec<u8>> {
        // Parse header
        if encrypted_data.len() < 4 {
            return Err(AssetError::AdapterError {
                message: "Kyber ciphertext too short for length header".to_string(),
            });
        }

        let kem_ct_len = u32::from_be_bytes([
            encrypted_data[0], encrypted_data[1],
            encrypted_data[2], encrypted_data[3],
        ]) as usize;

        let min_len = 4 + kem_ct_len + 12 + 16; // header + kem_ct + nonce + tag
        if encrypted_data.len() < min_len {
            return Err(AssetError::AdapterError {
                message: "Kyber ciphertext too short".to_string(),
            });
        }

        let kem_ct_bytes = &encrypted_data[4..4 + kem_ct_len];
        let nonce_bytes = &encrypted_data[4 + kem_ct_len..4 + kem_ct_len + 12];
        let tag_bytes = &encrypted_data[4 + kem_ct_len + 12..4 + kem_ct_len + 28];
        let aes_ct = &encrypted_data[4 + kem_ct_len + 28..];

        // KEM decapsulation
        let sk = kyber1024::SecretKey::from_bytes(&self.secret_key_bytes)
            .map_err(|e| AssetError::AdapterError {
                message: format!("Invalid Kyber-1024 secret key: {}", e),
            })?;

        let kem_ct = kyber1024::Ciphertext::from_bytes(kem_ct_bytes)
            .map_err(|e| AssetError::AdapterError {
                message: format!("Invalid Kyber-1024 ciphertext: {}", e),
            })?;

        let shared_secret = kyber1024::decapsulate(&kem_ct, &sk);
        let aes_key = Self::derive_aes_key(shared_secret.as_bytes());

        // AES-GCM decrypt
        use aes_gcm::{Nonce, Tag};
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&aes_key));
        let nonce = Nonce::from_slice(nonce_bytes);
        let tag = Tag::from_slice(tag_bytes);

        let mut buffer = aes_ct.to_vec();
        cipher.decrypt_in_place_detached(nonce, b"", &mut buffer, tag)
            .map_err(|e| AssetError::AdapterError {
                message: format!("AES-GCM decryption failed: {}", e),
            })?;

        tracing::debug!("Kyber-1024 decrypted {} bytes", buffer.len());
        Ok(buffer)
    }

    /// Derive AES-256 key from Kyber shared secret
    fn derive_aes_key(shared_secret: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"KYBER-1024-AES-KEY:");
        hasher.update(shared_secret);
        hasher.finalize().into()
    }

    /// Get public key bytes for encryption
    pub fn get_public_key(&self) -> &[u8] {
        &self.public_key_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::ProxyAddress;
    
    #[tokio::test]
    async fn test_quantum_security_creation() {
        let security = QuantumSecurity::new().await.expect("Failed to create QuantumSecurity");
        assert_eq!(security.active_tokens.len(), 0);
    }
    
    #[tokio::test]
    async fn test_falcon_signer() {
        let signer = FalconSigner::new().expect("Failed to create FalconSigner");
        let test_data = b"test message for signing";

        let signature = signer.sign(test_data).await.expect("Failed to sign data");
        assert!(!signature.is_empty());

        let valid = signer.verify(test_data, &signature).await.expect("Failed to verify signature");
        assert!(valid);

        // Test with different data - should not verify
        let invalid = signer.verify(b"different message", &signature).await.expect("Failed to verify invalid signature");
        assert!(!invalid);
    }
    
    #[tokio::test]
    async fn test_kyber_encryption() {
        let kyber = KyberEncryption::new().expect("Failed to create KyberEncryption");
        let test_data = b"sensitive data for encryption";

        let encrypted = kyber.encrypt(test_data).await.expect("Failed to encrypt data");
        assert_ne!(encrypted, test_data);

        let decrypted = kyber.decrypt(&encrypted).await.expect("Failed to decrypt data");
        assert_eq!(decrypted, test_data);
    }
    
    #[tokio::test]
    async fn test_access_token_generation_and_validation() {
        let security = QuantumSecurity::new().await.expect("Failed to create QuantumSecurity");
        let proxy_addr = ProxyAddress::new([1u8; 16], [2u8; 8], 8080);

        let tokens = security.generate_access_tokens(&proxy_addr).await.expect("Failed to generate access tokens");
        assert!(!tokens.is_empty());

        let valid = security.validate_access_tokens(&tokens).await.expect("Failed to validate access tokens");
        assert!(valid);
    }
}