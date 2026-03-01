// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! FALCON-1024 TrustChain Client
//!
//! Production implementation of `TrustChainClient` using pqcrypto_falcon
//! for post-quantum signature verification.

use anyhow::Result;

/// TrustChain client trait for integration
pub trait TrustChainClient: Send + Sync {
    /// Verify a signature using TrustChain CA
    fn verify_signature(&self, pubkey: &[u8], data: &[u8], signature: &[u8]) -> Result<bool>;

    /// Check if a public key is trusted
    fn is_trusted(&self, pubkey: &[u8]) -> Result<bool>;
}

/// FALCON-1024 TrustChain client for real signature verification
///
/// Uses pqcrypto_falcon for post-quantum signature verification.
/// This is the production implementation of TrustChainClient.
pub struct FalconTrustChainClient {
    /// Set of trusted public key fingerprints (SHA-256 of pubkey bytes)
    trusted_keys: std::sync::RwLock<std::collections::HashSet<[u8; 32]>>,
}

impl Default for FalconTrustChainClient {
    fn default() -> Self {
        Self::new()
    }
}

impl FalconTrustChainClient {
    /// Create a new FALCON-1024 TrustChain client
    pub fn new() -> Self {
        Self {
            trusted_keys: std::sync::RwLock::new(std::collections::HashSet::new()),
        }
    }

    /// Register a trusted public key
    pub fn add_trusted_key(&self, pubkey: &[u8]) {
        let fingerprint = Self::fingerprint(pubkey);
        self.trusted_keys
            .write()
            .expect("trusted_keys lock poisoned")
            .insert(fingerprint);
    }

    /// Remove a trusted public key
    pub fn remove_trusted_key(&self, pubkey: &[u8]) {
        let fingerprint = Self::fingerprint(pubkey);
        self.trusted_keys
            .write()
            .expect("trusted_keys lock poisoned")
            .remove(&fingerprint);
    }

    /// Calculate SHA-256 fingerprint of a public key
    fn fingerprint(pubkey: &[u8]) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(b"FALCON-1024-KEY:");
        hasher.update(pubkey);
        hasher.finalize().into()
    }
}

impl TrustChainClient for FalconTrustChainClient {
    fn verify_signature(&self, pubkey: &[u8], data: &[u8], signature: &[u8]) -> Result<bool> {
        use pqcrypto_falcon::falcon1024;
        use pqcrypto_traits::sign::{DetachedSignature, PublicKey};
        use sha2::{Digest, Sha256};

        // Validate public key size
        if pubkey.len() != falcon1024::public_key_bytes() {
            return Ok(false);
        }

        // Reconstruct public key
        let public_key = falcon1024::PublicKey::from_bytes(pubkey)
            .map_err(|e| anyhow::anyhow!("Invalid FALCON-1024 public key: {e}"))?;

        // Reconstruct signature
        let detached_sig = match falcon1024::DetachedSignature::from_bytes(signature) {
            Ok(sig) => sig,
            Err(_) => return Ok(false),
        };

        // Hash the data (same as TrustChain's FalconCrypto::hash_message)
        let mut hasher = Sha256::new();
        hasher.update(data);
        let message_hash: [u8; 32] = hasher.finalize().into();

        // Verify FALCON-1024 signature against message hash
        match falcon1024::verify_detached_signature(&detached_sig, &message_hash, &public_key) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn is_trusted(&self, pubkey: &[u8]) -> Result<bool> {
        let fingerprint = Self::fingerprint(pubkey);
        Ok(self
            .trusted_keys
            .read()
            .expect("trusted_keys lock poisoned")
            .contains(&fingerprint))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_falcon_trustchain_client_creation() {
        let client = FalconTrustChainClient::new();
        // No trusted keys initially
        assert!(!client.is_trusted(&[1, 2, 3]).expect("test: is_trusted"));
    }

    #[test]
    fn test_falcon_trustchain_client_trusted_keys() {
        let client = FalconTrustChainClient::new();
        let pubkey = vec![42u8; 64]; // Not a real FALCON key, just testing trust tracking

        client.add_trusted_key(&pubkey);
        assert!(client.is_trusted(&pubkey).expect("test: is_trusted"));

        client.remove_trusted_key(&pubkey);
        assert!(!client.is_trusted(&pubkey).expect("test: is_trusted"));
    }

    #[test]
    fn test_falcon_verify_invalid_key_size() {
        let client = FalconTrustChainClient::new();
        // Wrong key size should return Ok(false), not error
        let result = client
            .verify_signature(&[1, 2, 3], b"data", &[4, 5, 6])
            .expect("test: verify");
        assert!(!result);
    }
}
