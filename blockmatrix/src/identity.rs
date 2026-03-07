// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! FALCON-1024 Node Identity
//!
//! Each node has a persistent FALCON-1024 keypair used to sign state proofs
//! for bilateral authentication during handshakes. The node ID is derived
//! as BLAKE3(public_key).
//!
//! Keys are stored as raw DER bytes on disk and loaded on startup.

use anyhow::{anyhow, Result};
use pqcrypto_falcon::falcon1024;
use pqcrypto_traits::sign::{DetachedSignature, PublicKey, SecretKey};
use std::path::Path;
use tracing::info;

/// Persistent FALCON-1024 identity for a HyperMesh node.
pub struct FalconIdentity {
    /// Raw FALCON-1024 public key bytes
    pub public_key: Vec<u8>,
    /// Raw FALCON-1024 secret key bytes
    secret_key: Vec<u8>,
    /// BLAKE3 hex digest of public_key — used as node ID
    pub node_id: String,
}

impl FalconIdentity {
    /// Generate a fresh FALCON-1024 keypair.
    pub fn generate() -> Self {
        let (pk, sk) = falcon1024::keypair();
        let pk_bytes = pk.as_bytes().to_vec();
        let sk_bytes = sk.as_bytes().to_vec();
        let node_id = blake3::hash(&pk_bytes).to_hex().to_string();
        Self {
            public_key: pk_bytes,
            secret_key: sk_bytes,
            node_id,
        }
    }

    /// Load an existing identity from `data_dir`, or generate and persist a new one.
    pub fn load_or_create(data_dir: &Path) -> Result<Self> {
        let pk_path = data_dir.join("falcon_pubkey.der");
        let sk_path = data_dir.join("falcon_secretkey.der");

        if pk_path.exists() && sk_path.exists() {
            let pk_bytes = std::fs::read(&pk_path)
                .map_err(|e| anyhow!("Failed to read FALCON public key: {e}"))?;
            let sk_bytes = std::fs::read(&sk_path)
                .map_err(|e| anyhow!("Failed to read FALCON secret key: {e}"))?;

            // Validate key sizes before accepting
            if pk_bytes.len() != falcon1024::public_key_bytes() {
                return Err(anyhow!(
                    "FALCON public key size mismatch: expected {}, got {}",
                    falcon1024::public_key_bytes(),
                    pk_bytes.len()
                ));
            }
            if sk_bytes.len() != falcon1024::secret_key_bytes() {
                return Err(anyhow!(
                    "FALCON secret key size mismatch: expected {}, got {}",
                    falcon1024::secret_key_bytes(),
                    sk_bytes.len()
                ));
            }

            let node_id = blake3::hash(&pk_bytes).to_hex().to_string();
            info!("Loaded FALCON-1024 identity from {}", data_dir.display());
            Ok(Self {
                public_key: pk_bytes,
                secret_key: sk_bytes,
                node_id,
            })
        } else {
            let identity = Self::generate();
            std::fs::create_dir_all(data_dir)
                .map_err(|e| anyhow!("Failed to create identity dir: {e}"))?;
            std::fs::write(&pk_path, &identity.public_key)
                .map_err(|e| anyhow!("Failed to write FALCON public key: {e}"))?;
            std::fs::write(&sk_path, &identity.secret_key)
                .map_err(|e| anyhow!("Failed to write FALCON secret key: {e}"))?;
            info!(
                "Generated new FALCON-1024 identity at {}",
                data_dir.display()
            );
            Ok(identity)
        }
    }

    /// Sign arbitrary data with this identity's FALCON-1024 secret key.
    pub fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        let sk = falcon1024::SecretKey::from_bytes(&self.secret_key)
            .map_err(|e| anyhow!("Invalid FALCON secret key: {e}"))?;
        let sig = falcon1024::detached_sign(data, &sk);
        Ok(sig.as_bytes().to_vec())
    }

    /// Return the raw secret key bytes (needed for SignedStateProof::sign).
    pub fn secret_key_bytes(&self) -> &[u8] {
        &self.secret_key
    }

    /// Verify a FALCON-1024 signature against a public key.
    pub fn verify(pubkey: &[u8], data: &[u8], signature: &[u8]) -> Result<bool> {
        let pk = falcon1024::PublicKey::from_bytes(pubkey)
            .map_err(|e| anyhow!("Invalid FALCON public key: {e}"))?;
        let sig = falcon1024::DetachedSignature::from_bytes(signature)
            .map_err(|e| anyhow!("Invalid FALCON signature: {e}"))?;
        Ok(falcon1024::verify_detached_signature(&sig, data, &pk).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_identity() {
        let id = FalconIdentity::generate();
        assert_eq!(id.public_key.len(), falcon1024::public_key_bytes());
        assert!(!id.node_id.is_empty());
        assert_eq!(id.node_id.len(), 64); // BLAKE3 hex = 64 chars
    }

    #[test]
    fn test_sign_verify() {
        let id = FalconIdentity::generate();
        let message = b"test message for FALCON-1024 signing";
        let sig = id.sign(message).expect("test: signing");

        let valid = FalconIdentity::verify(&id.public_key, message, &sig)
            .expect("test: verification");
        assert!(valid, "Signature should verify");

        // Tampered message should fail
        let invalid = FalconIdentity::verify(&id.public_key, b"tampered", &sig)
            .expect("test: verification");
        assert!(!invalid, "Tampered message should fail verification");
    }

    #[test]
    fn test_load_or_create_persists() {
        let dir = tempfile::tempdir().expect("test: tempdir");
        let path = dir.path().join("identity");

        // First call creates
        let id1 = FalconIdentity::load_or_create(&path).expect("test: create");

        // Second call loads
        let id2 = FalconIdentity::load_or_create(&path).expect("test: load");

        assert_eq!(id1.node_id, id2.node_id);
        assert_eq!(id1.public_key, id2.public_key);
    }

    #[test]
    fn test_node_id_is_blake3_of_pubkey() {
        let id = FalconIdentity::generate();
        let expected = blake3::hash(&id.public_key).to_hex().to_string();
        assert_eq!(id.node_id, expected);
    }
}
