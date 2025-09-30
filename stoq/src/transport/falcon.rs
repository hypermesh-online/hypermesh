//! FALCON Quantum-Resistant Cryptography for STOQ Transport
//!
//! This module provides FALCON-1024 digital signatures for quantum-resistant security
//! at the QUIC transport layer. FALCON (Fast-Fourier Lattice-based Compact Signatures)
//! provides post-quantum security for STOQ transport protocols.
//!
//! Implementation based on NIST PQC FALCON specification.

use bytes::{BytesMut, BufMut};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use sha2::{Sha256, Digest};
use anyhow::{Result, anyhow};

// Real FALCON cryptography imports
use pqcrypto_falcon::{falcon512, falcon1024};
use pqcrypto_traits::sign::{PublicKey as _, SecretKey as _, DetachedSignature as _};

/// FALCON signature algorithm parameters for STOQ transport
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FalconVariant {
    /// FALCON-512 (NIST security level I)
    Falcon512,
    /// FALCON-1024 (NIST security level V) - Recommended for STOQ
    Falcon1024,
}

impl FalconVariant {
    /// Get the public key size in bytes
    pub fn public_key_size(&self) -> usize {
        match self {
            FalconVariant::Falcon512 => falcon512::public_key_bytes(),
            FalconVariant::Falcon1024 => falcon1024::public_key_bytes(),
        }
    }

    /// Get the private key size in bytes
    pub fn private_key_size(&self) -> usize {
        match self {
            FalconVariant::Falcon512 => falcon512::secret_key_bytes(),
            FalconVariant::Falcon1024 => falcon1024::secret_key_bytes(),
        }
    }

    /// Get the signature size in bytes
    pub fn signature_size(&self) -> usize {
        match self {
            FalconVariant::Falcon512 => falcon512::signature_bytes(),
            FalconVariant::Falcon1024 => falcon1024::signature_bytes(),
        }
    }

    /// Get the security level in bits
    pub fn security_level(&self) -> u32 {
        match self {
            FalconVariant::Falcon512 => 128,
            FalconVariant::Falcon1024 => 256,
        }
    }
}

/// FALCON public key for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FalconPublicKey {
    /// The public key variant
    pub variant: FalconVariant,
    /// Raw public key bytes
    pub key_data: Vec<u8>,
    /// Key generation timestamp
    pub created_at: u64,
    /// Optional key identifier
    pub key_id: Option<String>,
}

impl FalconPublicKey {
    /// Create a new FALCON public key
    pub fn new(variant: FalconVariant, key_data: Vec<u8>) -> Result<Self> {
        if key_data.len() != variant.public_key_size() {
            return Err(anyhow!(
                "Invalid public key size: expected {}, got {}",
                variant.public_key_size(),
                key_data.len()
            ));
        }

        Ok(Self {
            variant,
            key_data,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            key_id: None,
        })
    }

    /// Set the key identifier
    pub fn with_key_id(mut self, key_id: String) -> Self {
        self.key_id = Some(key_id);
        self
    }

    /// Get the key fingerprint
    pub fn fingerprint(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.key_data);
        hasher.finalize().into()
    }
}

/// FALCON private key for signing
#[derive(Debug, Clone)]
pub struct FalconPrivateKey {
    /// The private key variant
    pub variant: FalconVariant,
    /// Raw private key bytes (sensitive data)
    key_data: Vec<u8>,
    /// Associated public key
    pub public_key: FalconPublicKey,
}

impl FalconPrivateKey {
    /// Create a new FALCON private key
    pub fn new(variant: FalconVariant, key_data: Vec<u8>, public_key: FalconPublicKey) -> Result<Self> {
        if key_data.len() != variant.private_key_size() {
            return Err(anyhow!(
                "Invalid private key size: expected {}, got {}",
                variant.private_key_size(),
                key_data.len()
            ));
        }

        Ok(Self {
            variant,
            key_data,
            public_key,
        })
    }

    /// Get reference to the private key data (use carefully)
    pub(crate) fn key_data(&self) -> &[u8] {
        &self.key_data
    }
}

/// FALCON digital signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FalconSignature {
    /// The signature variant used
    pub variant: FalconVariant,
    /// Raw signature bytes
    pub signature_data: Vec<u8>,
    /// Message hash that was signed
    pub message_hash: [u8; 32],
    /// Timestamp when signature was created
    pub signed_at: u64,
}

impl FalconSignature {
    /// Create a new FALCON signature
    pub fn new(variant: FalconVariant, signature_data: Vec<u8>, message_hash: [u8; 32]) -> Result<Self> {
        if signature_data.len() > variant.signature_size() {
            return Err(anyhow!(
                "Invalid signature size: expected <= {}, got {}",
                variant.signature_size(),
                signature_data.len()
            ));
        }

        Ok(Self {
            variant,
            signature_data,
            message_hash,
            signed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        })
    }
}

/// FALCON cryptographic engine for STOQ transport
pub struct FalconEngine {
    /// Default variant to use
    variant: FalconVariant,
    /// Key cache for performance
    key_cache: HashMap<String, FalconPublicKey>,
}

impl FalconEngine {
    /// Create a new FALCON engine
    pub fn new(variant: FalconVariant) -> Self {
        Self {
            variant,
            key_cache: HashMap::new(),
        }
    }

    /// Generate a new FALCON key pair
    pub fn generate_keypair(&self) -> Result<(FalconPrivateKey, FalconPublicKey)> {
        // Generate real FALCON keypair using pqcrypto library
        let (public_key_data, private_key_data) = match self.variant {
            FalconVariant::Falcon512 => {
                let (pk, sk) = falcon512::keypair();
                (pk.as_bytes().to_vec(), sk.as_bytes().to_vec())
            },
            FalconVariant::Falcon1024 => {
                let (pk, sk) = falcon1024::keypair();
                (pk.as_bytes().to_vec(), sk.as_bytes().to_vec())
            },
        };

        let public_key = FalconPublicKey::new(self.variant, public_key_data)?;
        let private_key = FalconPrivateKey::new(self.variant, private_key_data, public_key.clone())?;

        Ok((private_key, public_key))
    }

    /// Sign data with a FALCON private key
    pub fn sign(&self, private_key: &FalconPrivateKey, data: &[u8]) -> Result<FalconSignature> {
        // Hash the data
        let mut hasher = Sha256::new();
        hasher.update(data);
        let message_hash: [u8; 32] = hasher.finalize().into();

        // Sign with real FALCON algorithm
        let signature_data = match private_key.variant {
            FalconVariant::Falcon512 => {
                let sk = falcon512::SecretKey::from_bytes(&private_key.key_data())
                    .map_err(|e| anyhow!("Failed to reconstruct Falcon512 secret key: {}", e))?;
                let sig = falcon512::detached_sign(&message_hash, &sk);
                sig.as_bytes().to_vec()
            },
            FalconVariant::Falcon1024 => {
                let sk = falcon1024::SecretKey::from_bytes(&private_key.key_data())
                    .map_err(|e| anyhow!("Failed to reconstruct Falcon1024 secret key: {}", e))?;
                let sig = falcon1024::detached_sign(&message_hash, &sk);
                sig.as_bytes().to_vec()
            },
        };

        FalconSignature::new(private_key.variant, signature_data, message_hash)
    }

    /// Verify a FALCON signature
    pub fn verify(&self, public_key: &FalconPublicKey, signature: &FalconSignature, data: &[u8]) -> Result<bool> {
        // Verify signature variant matches key variant
        if public_key.variant != signature.variant {
            return Ok(false);
        }

        // Hash the data and verify it matches the signature
        let mut hasher = Sha256::new();
        hasher.update(data);
        let computed_hash: [u8; 32] = hasher.finalize().into();

        if computed_hash != signature.message_hash {
            return Ok(false);
        }

        // Verify with real FALCON algorithm
        let result = match signature.variant {
            FalconVariant::Falcon512 => {
                let pk = falcon512::PublicKey::from_bytes(&public_key.key_data)
                    .map_err(|e| anyhow!("Failed to reconstruct Falcon512 public key: {}", e))?;
                let sig = falcon512::DetachedSignature::from_bytes(&signature.signature_data)
                    .map_err(|e| anyhow!("Failed to reconstruct Falcon512 signature: {}", e))?;
                falcon512::verify_detached_signature(&sig, &computed_hash, &pk).is_ok()
            },
            FalconVariant::Falcon1024 => {
                let pk = falcon1024::PublicKey::from_bytes(&public_key.key_data)
                    .map_err(|e| anyhow!("Failed to reconstruct Falcon1024 public key: {}", e))?;
                let sig = falcon1024::DetachedSignature::from_bytes(&signature.signature_data)
                    .map_err(|e| anyhow!("Failed to reconstruct Falcon1024 signature: {}", e))?;
                falcon1024::verify_detached_signature(&sig, &computed_hash, &pk).is_ok()
            },
        };

        Ok(result)
    }

    /// Cache a public key for performance
    pub fn cache_public_key(&mut self, key_id: String, public_key: FalconPublicKey) {
        self.key_cache.insert(key_id, public_key);
    }

    /// Get a cached public key
    pub fn get_cached_public_key(&self, key_id: &str) -> Option<&FalconPublicKey> {
        self.key_cache.get(key_id)
    }

    /// Clear the key cache
    pub fn clear_cache(&mut self) {
        self.key_cache.clear();
    }
}

impl Default for FalconEngine {
    fn default() -> Self {
        Self::new(FalconVariant::Falcon1024)
    }
}

/// FALCON transport integration for QUIC handshake
pub struct FalconTransport {
    /// FALCON cryptographic engine
    engine: FalconEngine,
    /// Local private key for signing
    private_key: Option<FalconPrivateKey>,
    /// Local public key
    public_key: Option<FalconPublicKey>,
    /// Trusted public keys for verification
    trusted_keys: HashMap<String, FalconPublicKey>,
}

impl FalconTransport {
    /// Create a new FALCON transport
    pub fn new(variant: FalconVariant) -> Self {
        Self {
            engine: FalconEngine::new(variant),
            private_key: None,
            public_key: None,
            trusted_keys: HashMap::new(),
        }
    }

    /// Generate and set local key pair
    pub fn generate_local_keypair(&mut self) -> Result<()> {
        let (private_key, public_key) = self.engine.generate_keypair()?;
        self.private_key = Some(private_key);
        self.public_key = Some(public_key);
        Ok(())
    }

    /// Set local key pair
    pub fn set_local_keypair(&mut self, private_key: FalconPrivateKey, public_key: FalconPublicKey) {
        self.private_key = Some(private_key);
        self.public_key = Some(public_key);
    }

    /// Add a trusted public key
    pub fn add_trusted_key(&mut self, key_id: String, public_key: FalconPublicKey) {
        self.trusted_keys.insert(key_id.clone(), public_key.clone());
        self.engine.cache_public_key(key_id, public_key);
    }

    /// Sign QUIC handshake data
    pub fn sign_handshake_data(&self, data: &[u8]) -> Result<FalconSignature> {
        let private_key = self.private_key.as_ref()
            .ok_or_else(|| anyhow!("No private key available for signing"))?;
        self.engine.sign(private_key, data)
    }

    /// Verify QUIC handshake signature
    pub fn verify_handshake_signature(&self, key_id: &str, signature: &FalconSignature, data: &[u8]) -> Result<bool> {
        let public_key = self.trusted_keys.get(key_id)
            .ok_or_else(|| anyhow!("Unknown public key: {}", key_id))?;
        self.engine.verify(public_key, signature, data)
    }

    /// Get local public key
    pub fn get_local_public_key(&self) -> Option<&FalconPublicKey> {
        self.public_key.as_ref()
    }

    /// Export FALCON signature for QUIC wire format
    pub fn export_signature(&self, signature: &FalconSignature) -> Vec<u8> {
        let mut buffer = BytesMut::new();

        // Write variant (1 byte)
        buffer.put_u8(match signature.variant {
            FalconVariant::Falcon512 => 0,
            FalconVariant::Falcon1024 => 1,
        });

        // Write signature length (2 bytes)
        buffer.put_u16(signature.signature_data.len() as u16);

        // Write signature data
        buffer.put_slice(&signature.signature_data);

        // Write message hash (32 bytes)
        buffer.put_slice(&signature.message_hash);

        // Write timestamp (8 bytes)
        buffer.put_u64(signature.signed_at);

        buffer.freeze().to_vec()
    }

    /// Import FALCON signature from QUIC wire format
    pub fn import_signature(&self, data: &[u8]) -> Result<FalconSignature> {
        if data.len() < 43 {  // Minimum: 1 (variant) + 2 (length) + 32 (hash) + 8 (timestamp)
            return Err(anyhow!("Signature data too short: {} bytes", data.len()));
        }

        let variant = match data[0] {
            0 => FalconVariant::Falcon512,
            1 => FalconVariant::Falcon1024,
            v => return Err(anyhow!("Unknown FALCON variant: {}", v)),
        };

        let sig_len = u16::from_be_bytes([data[1], data[2]]) as usize;

        if data.len() < 3 + sig_len + 40 {  // 3 (header) + sig_len + 32 (hash) + 8 (timestamp)
            return Err(anyhow!("Signature data truncated"));
        }

        let signature_data = data[3..3+sig_len].to_vec();
        let message_hash: [u8; 32] = data[3+sig_len..3+sig_len+32]
            .try_into()
            .map_err(|_| anyhow!("Invalid message hash"))?;
        let signed_at = u64::from_be_bytes(
            data[3+sig_len+32..3+sig_len+40]
                .try_into()
                .map_err(|_| anyhow!("Invalid timestamp"))?
        );

        Ok(FalconSignature {
            variant,
            signature_data,
            message_hash,
            signed_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_falcon_keypair_generation() {
        let engine = FalconEngine::new(FalconVariant::Falcon1024);
        let result = engine.generate_keypair();
        assert!(result.is_ok());

        let (private_key, public_key) = result.unwrap();
        assert_eq!(private_key.variant, FalconVariant::Falcon1024);
        assert_eq!(public_key.variant, FalconVariant::Falcon1024);
        assert_eq!(public_key.key_data.len(), falcon1024::public_key_bytes());
        assert_eq!(private_key.key_data().len(), falcon1024::secret_key_bytes());
    }

    #[test]
    fn test_falcon_sign_verify() -> Result<(), Box<dyn std::error::Error>> {
        let engine = FalconEngine::new(FalconVariant::Falcon1024);
        let (private_key, public_key) = engine.generate_keypair()?;

        let message = b"Test message for FALCON signature";
        let signature = engine.sign(&private_key, message)?;

        assert_eq!(signature.variant, FalconVariant::Falcon1024);
        assert!(signature.signature_data.len() <= falcon1024::signature_bytes());

        let verification = engine.verify(&public_key, &signature, message)?;
        assert!(verification, "Signature verification should succeed");

        // Test with wrong message
        let wrong_message = b"Different message";
        let verification = engine.verify(&public_key, &signature, wrong_message)?;
        assert!(!verification, "Signature verification should fail for wrong message");
        Ok(())
    }

    #[test]
    fn test_falcon_transport_handshake() -> Result<(), Box<dyn std::error::Error>> {
        let mut transport = FalconTransport::new(FalconVariant::Falcon1024);
        transport.generate_local_keypair()?;

        let handshake_data = b"QUIC handshake data";
        let signature = transport.sign_handshake_data(handshake_data)?;

        // Add local public key as trusted for testing
        let public_key = transport.get_local_public_key()
            .ok_or("No local public key")?
            .clone();
        transport.add_trusted_key("test_key".to_string(), public_key);

        let verification = transport.verify_handshake_signature("test_key", &signature, handshake_data)?;
        assert!(verification, "Handshake signature verification should succeed");
        Ok(())
    }

    #[test]
    fn test_signature_wire_format() -> Result<(), Box<dyn std::error::Error>> {
        let engine = FalconEngine::new(FalconVariant::Falcon512);
        let (private_key, _) = engine.generate_keypair()?;

        let message = b"Wire format test";
        let signature = engine.sign(&private_key, message)?;

        let transport = FalconTransport::new(FalconVariant::Falcon512);
        let exported = transport.export_signature(&signature);
        let imported = transport.import_signature(&exported)?;

        assert_eq!(signature.variant, imported.variant);
        assert_eq!(signature.signature_data, imported.signature_data);
        assert_eq!(signature.message_hash, imported.message_hash);
        assert_eq!(signature.signed_at, imported.signed_at);
        Ok(())
    }
}