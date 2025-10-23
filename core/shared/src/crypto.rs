//! Cryptographic utilities for Nexus components

use ed25519_dalek::{Signer, Verifier, Signature, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use rand::rngs::OsRng;

/// Ed25519 key pair for node identity and signing
#[derive(Debug, Clone)]
pub struct KeyPair {
    signing_key: SigningKey,
    public_key: [u8; 32],
}

impl KeyPair {
    /// Generate a new random key pair
    pub fn generate() -> Result<Self, Box<dyn std::error::Error>> {
        let mut _csprng = OsRng;
        let secret_key_bytes: [u8; 32] = (0..32).map(|_| rand::random()).collect::<Vec<u8>>().try_into().unwrap();
        let signing_key = SigningKey::from_bytes(&secret_key_bytes);
        let public_key = signing_key.verifying_key().to_bytes();
        
        Ok(Self {
            signing_key,
            public_key,
        })
    }

    /// Load key pair from seed bytes
    pub fn from_bytes(seed: &[u8; 32]) -> Result<Self, Box<dyn std::error::Error>> {
        let signing_key = SigningKey::from_bytes(seed);
        let public_key = signing_key.verifying_key().to_bytes();
        
        Ok(Self {
            signing_key,
            public_key,
        })
    }

    /// Get the public key
    pub fn public_key(&self) -> &[u8; 32] {
        &self.public_key
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.signing_key.sign(message).to_bytes().to_vec()
    }

    /// Verify a signature
    pub fn verify(public_key: &[u8], message: &[u8], signature: &[u8]) -> bool {
        if let Ok(verifying_key) = VerifyingKey::from_bytes(public_key.try_into().unwrap_or(&[0u8; 32])) {
            if let Ok(sig_array) = <&[u8; 64]>::try_from(signature) {
                let sig = Signature::from_bytes(sig_array);
                verifying_key.verify(message, &sig).is_ok()
            } else {
                false
            }
        } else {
            false
        }
    }
}

/// Hash function using Blake3
pub fn hash(data: &[u8]) -> [u8; 32] {
    let digest = blake3::hash(data);
    *digest.as_bytes()
}

/// Cryptographically secure random number generation
pub fn random_bytes(len: usize) -> Vec<u8> {
    use ring::rand::{SystemRandom, SecureRandom};
    let rng = SystemRandom::new();
    let mut bytes = vec![0u8; len];
    rng.fill(&mut bytes).expect("RNG failure");
    bytes
}

/// Message authentication with timestamp and nonce
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedMessage {
    pub payload: Vec<u8>,
    pub timestamp: u64,
    pub nonce: [u8; 16],
    pub signature: Vec<u8>,
}

impl AuthenticatedMessage {
    /// Create a new authenticated message
    pub fn new(payload: Vec<u8>, key_pair: &KeyPair) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        
        let nonce_bytes = random_bytes(16);
        let mut nonce = [0u8; 16];
        nonce.copy_from_slice(&nonce_bytes);
        
        // Create message to sign: payload || timestamp || nonce
        let mut message_to_sign = payload.clone();
        message_to_sign.extend_from_slice(&timestamp.to_be_bytes());
        message_to_sign.extend_from_slice(&nonce);
        
        let signature = key_pair.sign(&message_to_sign);
        
        Self {
            payload,
            timestamp,
            nonce,
            signature,
        }
    }
    
    /// Verify the message authenticity
    pub fn verify(&self, public_key: &[u8]) -> bool {
        // Reconstruct the signed message
        let mut message_to_verify = self.payload.clone();
        message_to_verify.extend_from_slice(&self.timestamp.to_be_bytes());
        message_to_verify.extend_from_slice(&self.nonce);
        
        KeyPair::verify(public_key, &message_to_verify, &self.signature)
    }
    
    /// Check if the message is within the valid time window
    pub fn is_fresh(&self, max_age_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        
        now.saturating_sub(self.timestamp) <= max_age_seconds
    }
}

/// Certificate validation helpers
pub mod cert {
    use super::*;
    
    /// Simple certificate structure for testing
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Certificate {
        pub subject: String,
        pub public_key: [u8; 32],
        pub issuer: String,
        pub not_before: u64,
        pub not_after: u64,
        pub signature: Vec<u8>,
    }
    
    impl Certificate {
        /// Create a self-signed certificate
        pub fn self_signed(subject: String, key_pair: &KeyPair, validity_days: u64) -> Self {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs();
            
            let not_before = now;
            let not_after = now + (validity_days * 24 * 60 * 60);
            
            // Create certificate data to sign
            let mut cert_data = subject.as_bytes().to_vec();
            cert_data.extend_from_slice(key_pair.public_key());
            cert_data.extend_from_slice(subject.as_bytes()); // issuer = subject for self-signed
            cert_data.extend_from_slice(&not_before.to_be_bytes());
            cert_data.extend_from_slice(&not_after.to_be_bytes());
            
            let signature = key_pair.sign(&cert_data);
            
            Self {
                subject: subject.clone(),
                public_key: *key_pair.public_key(),
                issuer: subject,
                not_before,
                not_after,
                signature,
            }
        }
        
        /// Verify certificate signature
        pub fn verify(&self, issuer_public_key: &[u8]) -> bool {
            // Reconstruct the signed data
            let mut cert_data = self.subject.as_bytes().to_vec();
            cert_data.extend_from_slice(&self.public_key);
            cert_data.extend_from_slice(self.issuer.as_bytes());
            cert_data.extend_from_slice(&self.not_before.to_be_bytes());
            cert_data.extend_from_slice(&self.not_after.to_be_bytes());
            
            KeyPair::verify(issuer_public_key, &cert_data, &self.signature)
        }
        
        /// Check if certificate is currently valid
        pub fn is_valid(&self) -> bool {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs();
                
            now >= self.not_before && now <= self.not_after
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_keypair_generation() {
        let kp1 = KeyPair::generate().unwrap();
        let kp2 = KeyPair::generate().unwrap();
        
        assert_ne!(kp1.public_key(), kp2.public_key());
    }
    
    #[test]
    fn test_signing_and_verification() {
        let key_pair = KeyPair::generate().unwrap();
        let message = b"Hello, world!";
        
        let signature = key_pair.sign(message);
        assert!(KeyPair::verify(key_pair.public_key(), message, &signature));
        
        // Verify fails with different message
        let wrong_message = b"Goodbye, world!";
        assert!(!KeyPair::verify(key_pair.public_key(), wrong_message, &signature));
    }
    
    #[test]
    fn test_authenticated_message() {
        let key_pair = KeyPair::generate().unwrap();
        let payload = b"secret payload".to_vec();
        
        let msg = AuthenticatedMessage::new(payload.clone(), &key_pair);
        assert!(msg.verify(key_pair.public_key()));
        assert!(msg.is_fresh(60)); // 60 seconds
        assert_eq!(msg.payload, payload);
    }
    
    #[test]
    fn test_self_signed_certificate() {
        let key_pair = KeyPair::generate().unwrap();
        let cert = cert::Certificate::self_signed(
            "test-node".to_string(),
            &key_pair,
            365
        );
        
        assert!(cert.verify(key_pair.public_key()));
        assert!(cert.is_valid());
        assert_eq!(cert.subject, "test-node");
        assert_eq!(cert.issuer, "test-node");
    }
}