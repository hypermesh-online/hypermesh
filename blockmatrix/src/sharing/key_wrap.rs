// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Key wrapping for P2P file sharing.
//!
//! Encrypts a [`DecryptionKey`] for a specific recipient using their
//! Kyber-1024 public key (KEM + AES-256-GCM), and decrypts it back
//! using the recipient's Kyber secret key.

use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit};
use pqcrypto_kyber::kyber1024;
use pqcrypto_traits::kem::{Ciphertext, PublicKey, SecretKey, SharedSecret};
use serde::{Deserialize, Serialize};

use crate::assets::pipeline::orchestrator::DecryptionKey;
use crate::assets::pipeline::PipelineError;

/// A recipient-bound key envelope: the AES-GCM-wrapped [`DecryptionKey`]
/// plus the Kyber KEM ciphertext needed to recover it.
///
/// This is the *custody* half of a transmission payload (R6). It NEVER
/// contains a cleartext Kyber secret key — the wrapped bytes are AES-GCM
/// ciphertext under a shared secret only the holder of the matching Kyber
/// secret key can derive.
///
/// The same envelope shape is used in three places:
/// - `ShareInvite` — wrapped for a *remote* recipient (P2P sharing).
/// - the owner-local shard map — wrapped for the owner's *own* node Kyber
///   identity (self-custody, so a stripped-of-secret map can still be
///   self-fetched without ever persisting a raw secret key).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEnvelope {
    /// AES-256-GCM ciphertext of the JSON-serialized `DecryptionKey`.
    #[serde(with = "serde_bytes")]
    pub encrypted_key: Vec<u8>,
    /// Kyber-1024 KEM ciphertext for unwrapping.
    #[serde(with = "serde_bytes")]
    pub key_kem_ciphertext: Vec<u8>,
}

impl KeyEnvelope {
    /// Wrap a `DecryptionKey` for the holder of `recipient_kyber_pubkey`.
    ///
    /// The recipient may be a remote peer or the owner itself (self-custody).
    pub fn wrap_for(
        decryption_key: &DecryptionKey,
        recipient_kyber_pubkey: &[u8],
    ) -> Result<Self, PipelineError> {
        let (encrypted_key, key_kem_ciphertext) =
            wrap_key_for_recipient(decryption_key, recipient_kyber_pubkey)?;
        Ok(Self {
            encrypted_key,
            key_kem_ciphertext,
        })
    }

    /// Unwrap this envelope using our Kyber-1024 secret key.
    pub fn unwrap_with(&self, our_kyber_secret_key: &[u8]) -> Result<DecryptionKey, PipelineError> {
        unwrap_key(
            &self.encrypted_key,
            &self.key_kem_ciphertext,
            our_kyber_secret_key,
        )
    }
}

/// Encrypt a `DecryptionKey` for a specific recipient.
///
/// Uses Kyber-1024 KEM to establish a shared secret with the recipient,
/// derives an AES-256-GCM key via BLAKE3, and encrypts the JSON-serialized
/// `DecryptionKey`.
///
/// Returns `(encrypted_key_bytes, kem_ciphertext_bytes)`.
pub fn wrap_key_for_recipient(
    decryption_key: &DecryptionKey,
    recipient_kyber_pubkey: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), PipelineError> {
    // 1. Serialize the DecryptionKey to JSON
    let key_json = serde_json::to_vec(decryption_key)
        .map_err(|e| PipelineError::EncryptionFailed(format!("key serialization: {e}")))?;

    // 2. Kyber KEM encapsulate with recipient's public key
    let pk = kyber1024::PublicKey::from_bytes(recipient_kyber_pubkey)
        .map_err(|_| PipelineError::EncryptionFailed("invalid recipient Kyber public key".into()))?;
    let (shared_secret, kem_ciphertext) = kyber1024::encapsulate(&pk);

    // 3. Derive AES-256 key and nonce from shared secret via BLAKE3
    let aes_key = blake3::derive_key("HYPERMESH-SHARE-KEY-WRAP-V1", shared_secret.as_bytes());
    let nonce_full =
        blake3::derive_key("HYPERMESH-SHARE-KEY-NONCE-V1", shared_secret.as_bytes());
    let nonce = &nonce_full[..12];

    // 4. AES-256-GCM encrypt the serialized key
    let cipher = Aes256Gcm::new(aes_gcm::Key::<Aes256Gcm>::from_slice(&aes_key));
    let encrypted = cipher
        .encrypt(aes_gcm::Nonce::from_slice(nonce), key_json.as_ref())
        .map_err(|_| PipelineError::EncryptionFailed("AES-GCM key wrap failed".into()))?;

    Ok((encrypted, kem_ciphertext.as_bytes().to_vec()))
}

/// Decrypt a wrapped `DecryptionKey` using our Kyber-1024 secret key.
pub fn unwrap_key(
    encrypted_key: &[u8],
    kem_ciphertext: &[u8],
    our_kyber_secret_key: &[u8],
) -> Result<DecryptionKey, PipelineError> {
    // 1. Kyber KEM decapsulate
    let sk = kyber1024::SecretKey::from_bytes(our_kyber_secret_key)
        .map_err(|_| PipelineError::EncryptionFailed("invalid Kyber secret key".into()))?;
    let ct = kyber1024::Ciphertext::from_bytes(kem_ciphertext)
        .map_err(|_| PipelineError::EncryptionFailed("invalid Kyber ciphertext".into()))?;
    let shared_secret = kyber1024::decapsulate(&ct, &sk);

    // 2. Derive same AES-256 key and nonce
    let aes_key = blake3::derive_key("HYPERMESH-SHARE-KEY-WRAP-V1", shared_secret.as_bytes());
    let nonce_full =
        blake3::derive_key("HYPERMESH-SHARE-KEY-NONCE-V1", shared_secret.as_bytes());
    let nonce = &nonce_full[..12];

    // 3. AES-256-GCM decrypt
    let cipher = Aes256Gcm::new(aes_gcm::Key::<Aes256Gcm>::from_slice(&aes_key));
    let decrypted = cipher
        .decrypt(aes_gcm::Nonce::from_slice(nonce), encrypted_key.as_ref())
        .map_err(|_| PipelineError::EncryptionFailed("AES-GCM key unwrap failed".into()))?;

    // 4. Deserialize DecryptionKey
    serde_json::from_slice(&decrypted)
        .map_err(|e| PipelineError::EncryptionFailed(format!("key deserialization: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::pipeline::encryption::{AesKey, Encryptor};

    #[test]
    fn test_key_wrap_unwrap_roundtrip() {
        let encryptor = Encryptor::default();
        let asset_keypair = encryptor.generate_keypair().expect("test: asset keypair");
        let dk = DecryptionKey::Kyber {
            ciphertext_kem: asset_keypair.public_key.clone(),
            nonce: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
            original_size: 42,
            secret_key: asset_keypair.secret_key.clone(),
        };

        // Generate recipient Kyber keypair
        let (recipient_pk, recipient_sk) = pqcrypto_kyber::kyber1024::keypair();

        let (encrypted, kem_ct) =
            wrap_key_for_recipient(&dk, recipient_pk.as_bytes()).expect("test: wrap");
        let unwrapped =
            unwrap_key(&encrypted, &kem_ct, recipient_sk.as_bytes()).expect("test: unwrap");

        // Compare via JSON since DecryptionKey doesn't impl PartialEq
        let original_json = serde_json::to_string(&dk).expect("test: ser original");
        let unwrapped_json = serde_json::to_string(&unwrapped).expect("test: ser unwrapped");
        assert_eq!(original_json, unwrapped_json);
    }

    #[test]
    fn test_key_wrap_wrong_key_fails() {
        let dk = DecryptionKey::Aes(AesKey {
            key: vec![0xAA; 32],
            nonce: vec![0xBB; 12],
        });

        let (alice_pk, _alice_sk) = pqcrypto_kyber::kyber1024::keypair();
        let (_bob_pk, bob_sk) = pqcrypto_kyber::kyber1024::keypair();

        let (encrypted, kem_ct) =
            wrap_key_for_recipient(&dk, alice_pk.as_bytes()).expect("test: wrap with alice");

        // Try unwrapping with bob's secret key -- must fail
        let result = unwrap_key(&encrypted, &kem_ct, bob_sk.as_bytes());
        assert!(result.is_err(), "unwrap with wrong key must fail");
    }

    #[test]
    fn test_key_wrap_preserves_decryption_key_type_kyber() {
        let (recipient_pk, recipient_sk) = pqcrypto_kyber::kyber1024::keypair();

        let dk = DecryptionKey::Kyber {
            ciphertext_kem: vec![0x11; 100],
            nonce: vec![0x22; 12],
            original_size: 999,
            secret_key: vec![0x33; 100],
        };
        let (enc, kem) =
            wrap_key_for_recipient(&dk, recipient_pk.as_bytes()).expect("test: wrap Kyber");
        let restored =
            unwrap_key(&enc, &kem, recipient_sk.as_bytes()).expect("test: unwrap Kyber");
        assert!(matches!(restored, DecryptionKey::Kyber { .. }));
    }

    #[test]
    fn test_key_wrap_preserves_decryption_key_type_aes() {
        let (recipient_pk, recipient_sk) = pqcrypto_kyber::kyber1024::keypair();

        let dk = DecryptionKey::Aes(AesKey {
            key: vec![0xCC; 32],
            nonce: vec![0xDD; 12],
        });
        let (enc, kem) =
            wrap_key_for_recipient(&dk, recipient_pk.as_bytes()).expect("test: wrap Aes");
        let restored =
            unwrap_key(&enc, &kem, recipient_sk.as_bytes()).expect("test: unwrap Aes");
        assert!(matches!(restored, DecryptionKey::Aes(_)));
    }

    #[test]
    fn test_key_wrap_preserves_decryption_key_type_kyber_segmented() {
        let (recipient_pk, recipient_sk) = pqcrypto_kyber::kyber1024::keypair();

        let dk = DecryptionKey::KyberSegmented {
            ciphertext_kem: vec![0x44; 80],
            secret_key: vec![0x55; 80],
            segment_count: 7,
            original_size: 12345,
        };
        let (enc, kem) = wrap_key_for_recipient(&dk, recipient_pk.as_bytes())
            .expect("test: wrap KyberSegmented");
        let restored = unwrap_key(&enc, &kem, recipient_sk.as_bytes())
            .expect("test: unwrap KyberSegmented");
        assert!(matches!(restored, DecryptionKey::KyberSegmented { .. }));
    }

    #[test]
    fn test_key_envelope_roundtrip() {
        let (recipient_pk, recipient_sk) = pqcrypto_kyber::kyber1024::keypair();
        let dk = DecryptionKey::Kyber {
            ciphertext_kem: vec![0x11; 100],
            nonce: vec![0x22; 12],
            original_size: 4096,
            secret_key: vec![0x33; 100],
        };

        let env = KeyEnvelope::wrap_for(&dk, recipient_pk.as_bytes())
            .expect("test: wrap envelope");
        let restored = env
            .unwrap_with(recipient_sk.as_bytes())
            .expect("test: unwrap envelope");

        let original_json = serde_json::to_string(&dk).expect("test: ser dk");
        let restored_json = serde_json::to_string(&restored).expect("test: ser restored");
        assert_eq!(original_json, restored_json);
    }

    /// Regression (F5): a serialized `KeyEnvelope` must NOT contain the raw
    /// Kyber secret-key bytes in cleartext. The secret is only recoverable by
    /// decapsulating with the recipient's Kyber secret key.
    #[test]
    fn test_key_envelope_hides_raw_secret_key() {
        let (recipient_pk, _recipient_sk) = pqcrypto_kyber::kyber1024::keypair();

        // A distinctive secret-key byte pattern we can search for.
        let raw_secret = vec![0xEE; 3168]; // kyber1024 secret key size
        let dk = DecryptionKey::Kyber {
            ciphertext_kem: vec![0x01; 100],
            nonce: vec![0x02; 12],
            original_size: 1,
            secret_key: raw_secret.clone(),
        };

        let env = KeyEnvelope::wrap_for(&dk, recipient_pk.as_bytes())
            .expect("test: wrap envelope");
        let serialized = serde_json::to_vec(&env).expect("test: serialize envelope");

        // The raw secret window must not appear anywhere in the serialized
        // envelope — it is AES-GCM ciphertext, not plaintext.
        let needle = &raw_secret[..64];
        assert!(
            !serialized
                .windows(needle.len())
                .any(|w| w == needle),
            "raw Kyber secret bytes leaked into serialized KeyEnvelope"
        );
    }
}
