// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Post-Quantum Node Identity
//!
//! Each node has a dual-key identity:
//! - **FALCON-1024** keypair for signing (proves WHO created/sent something)
//! - **Kyber-1024** keypair for encryption (enables asset access control via KEM tokens)
//!
//! The node ID is derived as `BLAKE3(falcon_public_key)`.
//!
//! Keys are stored as raw DER bytes on disk and loaded on startup.
//!
//! This module implements [`hypermesh_lib::NodeSigner`] and [`hypermesh_lib::NodeEncryptor`]
//! so that STOQ and the asset pipeline can use the identity without depending
//! on TrustChain directly.

use anyhow::{anyhow, Result};
use hkdf::Hkdf;
use hypermesh_lib::{NodeEncryptor, NodeSigner};
use pqcrypto_falcon::falcon1024;
use pqcrypto_kyber::kyber1024;
use pqcrypto_traits::kem::{Ciphertext as KemCiphertext, PublicKey as KemPublicKey, SecretKey as KemSecretKey, SharedSecret};
use pqcrypto_traits::sign::{DetachedSignature, PublicKey, SecretKey};
use serde::{Deserialize, Serialize};
use sha2::Sha512;
use std::path::Path;
use tracing::info;

/// Persistent dual-key identity for a HyperMesh node.
///
/// Contains both FALCON-1024 (signing) and Kyber-1024 (encryption) keypairs:
/// - **FALCON**: Proves provenance — signs state proofs, handshake challenges, blocks
/// - **Kyber**: Enables access control — peers encrypt assets using this node's Kyber
///   pubkey, node decapsulates to recover shared secret, issues tokens to authorize
///   specific peers to decrypt
///
/// The node ID is `BLAKE3(falcon_public_key)`.
pub struct FalconIdentity {
    /// Raw FALCON-1024 public key bytes (signing/verification)
    pub public_key: Vec<u8>,
    /// Raw FALCON-1024 secret key bytes
    secret_key: Vec<u8>,
    /// Raw Kyber-1024 public key bytes (KEM encapsulation)
    pub kyber_public_key: Vec<u8>,
    /// Raw Kyber-1024 secret key bytes (KEM decapsulation)
    kyber_secret_key: Vec<u8>,
    /// BLAKE3 hex digest of FALCON public_key — used as node ID
    pub node_id: String,
}

impl FalconIdentity {
    /// Generate a fresh dual-key identity (FALCON-1024 + Kyber-1024).
    pub fn generate() -> Self {
        let (pk, sk) = falcon1024::keypair();
        let pk_bytes = pk.as_bytes().to_vec();
        let sk_bytes = sk.as_bytes().to_vec();

        let (kyber_pk, kyber_sk) = kyber1024::keypair();
        let kyber_pk_bytes = kyber_pk.as_bytes().to_vec();
        let kyber_sk_bytes = kyber_sk.as_bytes().to_vec();

        let node_id = blake3::hash(&pk_bytes).to_hex().to_string();
        Self {
            public_key: pk_bytes,
            secret_key: sk_bytes,
            kyber_public_key: kyber_pk_bytes,
            kyber_secret_key: kyber_sk_bytes,
            node_id,
        }
    }

    /// Load an existing identity from `data_dir`, or generate and persist a new one.
    ///
    /// Persists 4 key files: FALCON (signing) + Kyber (encryption).
    /// If FALCON keys exist but Kyber keys don't (upgrade path), generates
    /// Kyber keys and persists them alongside the existing FALCON keys.
    pub fn load_or_create(data_dir: &Path) -> Result<Self> {
        let pk_path = data_dir.join("falcon_pubkey.der");
        let sk_path = data_dir.join("falcon_secretkey.der");
        let kyber_pk_path = data_dir.join("kyber_pubkey.der");
        let kyber_sk_path = data_dir.join("kyber_secretkey.der");

        if pk_path.exists() && sk_path.exists() {
            let pk_bytes = std::fs::read(&pk_path)
                .map_err(|e| anyhow!("Failed to read FALCON public key: {e}"))?;
            let sk_bytes = std::fs::read(&sk_path)
                .map_err(|e| anyhow!("Failed to read FALCON secret key: {e}"))?;

            // Validate FALCON key sizes
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

            // Load or generate Kyber keys (upgrade path for existing nodes)
            let (kyber_pk_bytes, kyber_sk_bytes) = if kyber_pk_path.exists() && kyber_sk_path.exists() {
                let kpk = std::fs::read(&kyber_pk_path)
                    .map_err(|e| anyhow!("Failed to read Kyber public key: {e}"))?;
                let ksk = std::fs::read(&kyber_sk_path)
                    .map_err(|e| anyhow!("Failed to read Kyber secret key: {e}"))?;

                if kpk.len() != kyber1024::public_key_bytes() {
                    return Err(anyhow!(
                        "Kyber public key size mismatch: expected {}, got {}",
                        kyber1024::public_key_bytes(),
                        kpk.len()
                    ));
                }
                if ksk.len() != kyber1024::secret_key_bytes() {
                    return Err(anyhow!(
                        "Kyber secret key size mismatch: expected {}, got {}",
                        kyber1024::secret_key_bytes(),
                        ksk.len()
                    ));
                }
                (kpk, ksk)
            } else {
                info!("Generating Kyber-1024 encryption keys (upgrade from FALCON-only identity)");
                let (kpk, ksk) = kyber1024::keypair();
                let kpk_bytes = kpk.as_bytes().to_vec();
                let ksk_bytes = ksk.as_bytes().to_vec();
                std::fs::write(&kyber_pk_path, &kpk_bytes)
                    .map_err(|e| anyhow!("Failed to write Kyber public key: {e}"))?;
                std::fs::write(&kyber_sk_path, &ksk_bytes)
                    .map_err(|e| anyhow!("Failed to write Kyber secret key: {e}"))?;
                (kpk_bytes, ksk_bytes)
            };

            let node_id = blake3::hash(&pk_bytes).to_hex().to_string();
            info!("Loaded dual-key identity from {}", data_dir.display());
            Ok(Self {
                public_key: pk_bytes,
                secret_key: sk_bytes,
                kyber_public_key: kyber_pk_bytes,
                kyber_secret_key: kyber_sk_bytes,
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
            std::fs::write(&kyber_pk_path, &identity.kyber_public_key)
                .map_err(|e| anyhow!("Failed to write Kyber public key: {e}"))?;
            std::fs::write(&kyber_sk_path, &identity.kyber_secret_key)
                .map_err(|e| anyhow!("Failed to write Kyber secret key: {e}"))?;
            info!(
                "Generated new dual-key identity at {}",
                data_dir.display()
            );
            Ok(identity)
        }
    }

    /// Return the raw FALCON secret key bytes.
    pub fn secret_key_bytes(&self) -> &[u8] {
        &self.secret_key
    }

    /// Return the raw Kyber secret key bytes.
    pub fn kyber_secret_key_bytes(&self) -> &[u8] {
        &self.kyber_secret_key
    }

    /// Rotate keys: generate new FALCON-1024 + Kyber-1024 keypair, sign the
    /// transition with the old FALCON key.
    ///
    /// Returns `(KeyRotationEntry, new_identity)`. The new identity's `node_id`
    /// will be `BLAKE3(new_falcon_pubkey)` -- the **caller** must override it
    /// to the genesis-derived node ID before persisting. The rotation entry
    /// records the cryptographic proof of authorized key change (§6.2.2).
    pub fn rotate_keys(
        &self,
        block_index: u64,
        reason: KeyRotationReason,
    ) -> Result<(KeyRotationEntry, FalconIdentity)> {
        let new_identity = FalconIdentity::generate();

        let old_pubkey_hash = *blake3::hash(&self.public_key).as_bytes();

        let entry = KeyRotationEntry {
            old_pubkey_hash,
            new_pubkey: new_identity.public_key.clone(),
            new_kyber_pubkey: new_identity.kyber_public_key.clone(),
            rotation_signature: Vec::new(), // placeholder, filled below
            block_index,
            reason,
        };

        let message = entry.rotation_message();
        let sk = falcon1024::SecretKey::from_bytes(&self.secret_key)
            .map_err(|e| anyhow!("Invalid old FALCON secret key: {e}"))?;
        let sig = falcon1024::detached_sign(&message, &sk);
        let entry = KeyRotationEntry {
            rotation_signature: sig.as_bytes().to_vec(),
            ..entry
        };

        Ok((entry, new_identity))
    }

    /// Verify a key rotation chain from genesis to current.
    ///
    /// Each entry must have a valid `rotation_signature` produced by the
    /// previous key. Returns `Ok(true)` if the entire chain is valid,
    /// `Ok(false)` if any signature or hash check fails.
    pub fn verify_rotation_chain(
        genesis_pubkey: &[u8],
        rotations: &[KeyRotationEntry],
    ) -> Result<bool> {
        let mut current_key = genesis_pubkey.to_vec();

        for (i, entry) in rotations.iter().enumerate() {
            // Verify old_pubkey_hash matches BLAKE3 of current key
            let expected_hash = *blake3::hash(&current_key).as_bytes();
            if entry.old_pubkey_hash != expected_hash {
                info!(
                    "Rotation chain entry {i}: old_pubkey_hash mismatch"
                );
                return Ok(false);
            }

            let message = entry.rotation_message();
            let pk = falcon1024::PublicKey::from_bytes(&current_key)
                .map_err(|e| anyhow!("Invalid FALCON public key at rotation {i}: {e}"))?;
            let sig = falcon1024::DetachedSignature::from_bytes(
                &entry.rotation_signature,
            )
            .map_err(|e| anyhow!("Invalid FALCON signature at rotation {i}: {e}"))?;

            if falcon1024::verify_detached_signature(&sig, &message, &pk).is_err() {
                info!("Rotation chain entry {i}: signature verification failed");
                return Ok(false);
            }

            current_key = entry.new_pubkey.clone();
        }

        Ok(true)
    }
}

/// Represents a key rotation event stored as a BlockAssetEntry.
///
/// Per §6.2.2: old key signs authorization of new key, recorded on-chain.
/// The node_id does NOT change -- it remains `BLAKE3(genesis_falcon_pubkey)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationEntry {
    /// BLAKE3 hash of the outgoing FALCON public key.
    pub old_pubkey_hash: [u8; 32],
    /// Full incoming FALCON-1024 public key (1793 bytes).
    pub new_pubkey: Vec<u8>,
    /// Full incoming Kyber-1024 public key (1568 bytes).
    pub new_kyber_pubkey: Vec<u8>,
    /// Old key signs `BLAKE3(old_pubkey_hash || new_pubkey || new_kyber_pubkey || block_index)`.
    pub rotation_signature: Vec<u8>,
    /// Block index at which this rotation was recorded.
    pub block_index: u64,
    /// Reason for the rotation.
    pub reason: KeyRotationReason,
}

impl KeyRotationEntry {
    /// Build the canonical message that is signed during rotation.
    ///
    /// Format: `old_pubkey_hash || new_pubkey || new_kyber_pubkey || block_index_le`
    pub fn rotation_message(&self) -> Vec<u8> {
        let mut msg = Vec::with_capacity(
            32 + self.new_pubkey.len() + self.new_kyber_pubkey.len() + 8,
        );
        msg.extend_from_slice(&self.old_pubkey_hash);
        msg.extend_from_slice(&self.new_pubkey);
        msg.extend_from_slice(&self.new_kyber_pubkey);
        msg.extend_from_slice(&self.block_index.to_le_bytes());
        msg
    }
}

/// Reason for a key rotation event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum KeyRotationReason {
    /// Periodic scheduled rotation.
    Scheduled,
    /// Key compromise detected.
    Compromise,
    /// Cryptographic upgrade.
    Upgrade,
    /// Recovery from lost key material.
    Recovery,
}

/// Compute a recovery commitment from a passphrase.
///
/// The commitment is stored in the genesis block's Identity asset entry
/// (`AssetData.config`). During recovery, the operator provides the
/// passphrase and the system verifies it against this commitment.
///
/// Uses HKDF-SHA512 for key derivation then BLAKE3 for the final commitment
/// hash, binding the commitment to the specific `node_id`.
pub fn compute_recovery_commitment(passphrase: &str, node_id: &str) -> [u8; 32] {
    let hk = Hkdf::<Sha512>::new(Some(node_id.as_bytes()), passphrase.as_bytes());
    let mut okm = [0u8; 32];
    hk.expand(b"hypermesh-recovery-v1", &mut okm)
        .expect("HKDF expand cannot fail for 32-byte output");
    *blake3::hash(&okm).as_bytes()
}

impl NodeEncryptor for FalconIdentity {
    fn encryption_public_key(&self) -> &[u8] {
        &self.kyber_public_key
    }

    fn decapsulate(&self, kem_ciphertext: &[u8]) -> Result<Vec<u8>> {
        let sk = kyber1024::SecretKey::from_bytes(&self.kyber_secret_key)
            .map_err(|e| anyhow!("Invalid Kyber secret key: {e}"))?;
        let ct = kyber1024::Ciphertext::from_bytes(kem_ciphertext)
            .map_err(|e| anyhow!("Invalid Kyber KEM ciphertext: {e}"))?;
        let shared_secret = kyber1024::decapsulate(&ct, &sk);
        Ok(shared_secret.as_bytes().to_vec())
    }
}

impl NodeSigner for FalconIdentity {
    // rotation_chain() returns empty (default) — rotation history is on the
    // blockchain, not in the identity struct. A higher-level wrapper (in
    // blockmatrix) should implement NodeSigner by querying the chain for
    // KeyRotationEntry items and serializing them as JSON strings.

    fn node_id(&self) -> &str {
        &self.node_id
    }

    fn public_key_bytes(&self) -> &[u8] {
        &self.public_key
    }

    fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        let sk = falcon1024::SecretKey::from_bytes(&self.secret_key)
            .map_err(|e| anyhow!("Invalid FALCON secret key: {e}"))?;
        let sig = falcon1024::detached_sign(data, &sk);
        Ok(sig.as_bytes().to_vec())
    }

    fn verify_signature(pubkey: &[u8], data: &[u8], signature: &[u8]) -> Result<bool> {
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
        assert_eq!(id.kyber_public_key.len(), kyber1024::public_key_bytes());
        assert!(!id.node_id.is_empty());
        assert_eq!(id.node_id.len(), 64); // BLAKE3 hex = 64 chars
    }

    #[test]
    fn test_sign_verify_via_trait() {
        let id = FalconIdentity::generate();
        let message = b"test message for FALCON-1024 signing";
        let sig = NodeSigner::sign(&id, message).expect("test: signing");

        let valid = FalconIdentity::verify_signature(&id.public_key, message, &sig)
            .expect("test: verification");
        assert!(valid, "Signature should verify");

        let invalid = FalconIdentity::verify_signature(&id.public_key, b"tampered", &sig)
            .expect("test: verification");
        assert!(!invalid, "Tampered message should fail verification");
    }

    #[test]
    fn test_load_or_create_persists() {
        let dir = tempfile::tempdir().expect("test: tempdir");
        let path = dir.path().join("identity");

        let id1 = FalconIdentity::load_or_create(&path).expect("test: create");
        let id2 = FalconIdentity::load_or_create(&path).expect("test: load");

        assert_eq!(id1.node_id, id2.node_id);
        assert_eq!(id1.public_key, id2.public_key);
        assert_eq!(id1.kyber_public_key, id2.kyber_public_key);
    }

    #[test]
    fn test_node_id_is_blake3_of_pubkey() {
        let id = FalconIdentity::generate();
        let expected = blake3::hash(&id.public_key).to_hex().to_string();
        assert_eq!(id.node_id, expected);
    }

    #[test]
    fn test_node_signer_trait_node_id() {
        let id = FalconIdentity::generate();
        assert_eq!(NodeSigner::node_id(&id), id.node_id.as_str());
    }

    #[test]
    fn test_node_signer_trait_pubkey() {
        let id = FalconIdentity::generate();
        assert_eq!(NodeSigner::public_key_bytes(&id), id.public_key.as_slice());
    }

    #[test]
    fn test_kyber_encapsulate_decapsulate() {
        let id = FalconIdentity::generate();

        // Encapsulate using the node's Kyber public key (what a peer would do)
        let pk = kyber1024::PublicKey::from_bytes(&id.kyber_public_key)
            .expect("test: valid kyber pubkey");
        let (shared_secret_sender, kem_ciphertext) = kyber1024::encapsulate(&pk);

        // Decapsulate using the trait (what this node does to recover the secret)
        let shared_secret_receiver = id
            .decapsulate(kem_ciphertext.as_bytes())
            .expect("test: decapsulation");

        // Both sides must derive the same shared secret
        assert_eq!(shared_secret_sender.as_bytes(), shared_secret_receiver.as_slice());
    }

    #[test]
    fn test_node_encryptor_trait() {
        let id = FalconIdentity::generate();
        assert_eq!(
            NodeEncryptor::encryption_public_key(&id),
            id.kyber_public_key.as_slice()
        );
    }

    #[test]
    fn test_key_rotation_creates_valid_entry() {
        let id = FalconIdentity::generate();
        let (entry, new_id) = id
            .rotate_keys(42, KeyRotationReason::Scheduled)
            .expect("test: rotate_keys");

        assert_eq!(entry.old_pubkey_hash, *blake3::hash(&id.public_key).as_bytes());
        assert_eq!(entry.new_pubkey, new_id.public_key);
        assert_eq!(entry.new_kyber_pubkey, new_id.kyber_public_key);
        assert_eq!(entry.block_index, 42);
        assert_eq!(entry.reason, KeyRotationReason::Scheduled);
        assert!(!entry.rotation_signature.is_empty());
        // New identity has different keys
        assert_ne!(id.public_key, new_id.public_key);
        assert_ne!(id.kyber_public_key, new_id.kyber_public_key);
    }

    #[test]
    fn test_rotation_signature_verification() {
        let id = FalconIdentity::generate();
        let (entry, _new_id) = id
            .rotate_keys(100, KeyRotationReason::Upgrade)
            .expect("test: rotate_keys");

        // Manually verify the signature using the old pubkey
        let mut message = Vec::new();
        message.extend_from_slice(&entry.old_pubkey_hash);
        message.extend_from_slice(&entry.new_pubkey);
        message.extend_from_slice(&entry.new_kyber_pubkey);
        message.extend_from_slice(&100u64.to_le_bytes());

        let valid = FalconIdentity::verify_signature(&id.public_key, &message, &entry.rotation_signature)
            .expect("test: verify_signature");
        assert!(valid, "Rotation signature should verify with old pubkey");
    }

    #[test]
    fn test_verify_rotation_chain_single() {
        let id = FalconIdentity::generate();
        let (entry, _new_id) = id
            .rotate_keys(1, KeyRotationReason::Scheduled)
            .expect("test: rotate_keys");

        let valid = FalconIdentity::verify_rotation_chain(&id.public_key, &[entry])
            .expect("test: verify_rotation_chain");
        assert!(valid, "Single-rotation chain should verify");
    }

    #[test]
    fn test_verify_rotation_chain_multiple() {
        let id0 = FalconIdentity::generate();
        let (entry1, id1) = id0
            .rotate_keys(1, KeyRotationReason::Scheduled)
            .expect("test: rotate 1");
        let (entry2, id2) = id1
            .rotate_keys(5, KeyRotationReason::Upgrade)
            .expect("test: rotate 2");
        let (entry3, _id3) = id2
            .rotate_keys(10, KeyRotationReason::Scheduled)
            .expect("test: rotate 3");

        let valid = FalconIdentity::verify_rotation_chain(
            &id0.public_key,
            &[entry1, entry2, entry3],
        )
        .expect("test: verify_rotation_chain");
        assert!(valid, "Three-rotation chain should verify");
    }

    #[test]
    fn test_verify_rotation_chain_tampered() {
        let id0 = FalconIdentity::generate();
        let (entry1, id1) = id0
            .rotate_keys(1, KeyRotationReason::Scheduled)
            .expect("test: rotate 1");
        let (mut entry2, _id2) = id1
            .rotate_keys(5, KeyRotationReason::Upgrade)
            .expect("test: rotate 2");

        // Tamper with the second entry's new pubkey
        if let Some(byte) = entry2.new_pubkey.get_mut(0) {
            *byte ^= 0xFF;
        }

        let valid = FalconIdentity::verify_rotation_chain(
            &id0.public_key,
            &[entry1, entry2],
        )
        .expect("test: verify_rotation_chain");
        assert!(!valid, "Tampered chain should fail verification");
    }

    #[test]
    fn test_recovery_commitment_deterministic() {
        let c1 = compute_recovery_commitment("my-secret-phrase", "node-abc123");
        let c2 = compute_recovery_commitment("my-secret-phrase", "node-abc123");
        assert_eq!(c1, c2, "Same inputs should produce same commitment");
    }

    #[test]
    fn test_recovery_commitment_different_passphrase() {
        let c1 = compute_recovery_commitment("phrase-one", "node-abc123");
        let c2 = compute_recovery_commitment("phrase-two", "node-abc123");
        assert_ne!(c1, c2, "Different passphrases should produce different commitments");

        let c3 = compute_recovery_commitment("my-secret-phrase", "node-111");
        let c4 = compute_recovery_commitment("my-secret-phrase", "node-222");
        assert_ne!(c3, c4, "Different node_ids should produce different commitments");
    }

    #[test]
    fn test_load_or_create_upgrades_falcon_only() {
        // Simulate an existing FALCON-only identity (no Kyber keys on disk)
        let dir = tempfile::tempdir().expect("test: tempdir");
        let path = dir.path().join("identity");

        // Create FALCON-only identity manually
        let id1 = FalconIdentity::generate();
        std::fs::create_dir_all(&path).expect("test: mkdir");
        std::fs::write(path.join("falcon_pubkey.der"), &id1.public_key).expect("test: write pk");
        std::fs::write(path.join("falcon_secretkey.der"), &id1.secret_key).expect("test: write sk");
        // Deliberately NOT writing kyber keys

        // load_or_create should generate Kyber keys on upgrade
        let id2 = FalconIdentity::load_or_create(&path).expect("test: upgrade");
        assert_eq!(id2.node_id, id1.node_id); // Same FALCON identity
        assert_eq!(id2.kyber_public_key.len(), kyber1024::public_key_bytes());

        // Subsequent load should return same Kyber keys
        let id3 = FalconIdentity::load_or_create(&path).expect("test: reload");
        assert_eq!(id3.kyber_public_key, id2.kyber_public_key);
    }
}
