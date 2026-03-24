// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! DirectMessage -- a post-quantum encrypted message between two nodes.
//!
//! Encrypted with recipient's Kyber-1024 public key, signed with sender's
//! FALCON-1024 key. Sent peer-to-peer over STOQ with TAG_DIRECT_MESSAGE (0x06).

use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit};
use hypermesh_lib::NodeSigner;
use pqcrypto_kyber::kyber1024;
use pqcrypto_traits::kem::{Ciphertext, PublicKey, SecretKey, SharedSecret};
use serde::{Deserialize, Serialize};

use crate::identity::FalconIdentity;

/// A post-quantum encrypted direct message between two nodes.
///
/// The sender encrypts the message body for the recipient using Kyber-1024
/// KEM (establishing a shared secret for AES-256-GCM), then signs the entire
/// payload with FALCON-1024. The recipient decapsulates the KEM ciphertext
/// to recover the shared secret and decrypt the body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectMessage {
    /// Unique message identifier (UUID-style).
    pub message_id: String,
    /// Sender's node ID (BLAKE3 of FALCON pubkey).
    pub sender_node_id: String,
    /// Optional human-readable sender name.
    pub sender_name: Option<String>,
    /// Recipient's node ID.
    pub recipient_node_id: String,
    /// Kyber KEM + AES-GCM encrypted body.
    #[serde(with = "serde_bytes")]
    pub encrypted_body: Vec<u8>,
    /// Kyber KEM ciphertext for recipient to decapsulate.
    #[serde(with = "serde_bytes")]
    pub kem_ciphertext: Vec<u8>,
    /// Message ID of parent message (for threading).
    pub reply_to: Option<String>,
    /// MIME content type: "text/plain", "application/asset-ref", etc.
    pub content_type: String,
    /// Unix timestamp (seconds) when the message was created.
    pub created_at: i64,
    /// FALCON-1024 signature over the signing payload.
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,
}

impl DirectMessage {
    /// Create a new message. Call [`encrypt_body`] and [`sign`] before sending.
    pub fn new(
        sender_node_id: String,
        sender_name: Option<String>,
        recipient_node_id: String,
        content_type: String,
        reply_to: Option<String>,
    ) -> Self {
        let message_id = format!(
            "msg-{}",
            &blake3::hash(
                format!(
                    "{}:{}:{}",
                    sender_node_id,
                    recipient_node_id,
                    chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                )
                .as_bytes(),
            )
            .to_hex()[..16]
        );
        let created_at = chrono::Utc::now().timestamp();
        Self {
            message_id,
            sender_node_id,
            sender_name,
            recipient_node_id,
            encrypted_body: Vec::new(),
            kem_ciphertext: Vec::new(),
            reply_to,
            content_type,
            created_at,
            signature: Vec::new(),
        }
    }

    /// Encrypt plaintext body for recipient using their Kyber-1024 public key.
    ///
    /// Uses Kyber KEM to establish a shared secret, derives an AES-256-GCM
    /// key via BLAKE3 with message-specific domain separation, and encrypts
    /// the body. Stores the encrypted body and KEM ciphertext on `self`.
    pub fn encrypt_body(
        &mut self,
        plaintext: &[u8],
        recipient_kyber_pubkey: &[u8],
    ) -> Result<(), anyhow::Error> {
        let pk = kyber1024::PublicKey::from_bytes(recipient_kyber_pubkey)
            .map_err(|_| anyhow::anyhow!("invalid recipient Kyber public key"))?;
        let (shared_secret, kem_ct) = kyber1024::encapsulate(&pk);

        let aes_key =
            blake3::derive_key("HYPERMESH-MESSAGE-KEY-V1", shared_secret.as_bytes());
        let nonce_full =
            blake3::derive_key("HYPERMESH-MESSAGE-NONCE-V1", shared_secret.as_bytes());
        let nonce = &nonce_full[..12];

        let cipher = Aes256Gcm::new(aes_gcm::Key::<Aes256Gcm>::from_slice(&aes_key));
        let encrypted = cipher
            .encrypt(aes_gcm::Nonce::from_slice(nonce), plaintext)
            .map_err(|_| anyhow::anyhow!("AES-GCM message encryption failed"))?;

        self.encrypted_body = encrypted;
        self.kem_ciphertext = kem_ct.as_bytes().to_vec();
        Ok(())
    }

    /// Decrypt message body using our Kyber-1024 secret key.
    pub fn decrypt_body(
        &self,
        our_kyber_secret_key: &[u8],
    ) -> Result<Vec<u8>, anyhow::Error> {
        let sk = kyber1024::SecretKey::from_bytes(our_kyber_secret_key)
            .map_err(|_| anyhow::anyhow!("invalid Kyber secret key"))?;
        let ct = kyber1024::Ciphertext::from_bytes(&self.kem_ciphertext)
            .map_err(|_| anyhow::anyhow!("invalid Kyber ciphertext"))?;
        let shared_secret = kyber1024::decapsulate(&ct, &sk);

        let aes_key =
            blake3::derive_key("HYPERMESH-MESSAGE-KEY-V1", shared_secret.as_bytes());
        let nonce_full =
            blake3::derive_key("HYPERMESH-MESSAGE-NONCE-V1", shared_secret.as_bytes());
        let nonce = &nonce_full[..12];

        let cipher = Aes256Gcm::new(aes_gcm::Key::<Aes256Gcm>::from_slice(&aes_key));
        cipher
            .decrypt(aes_gcm::Nonce::from_slice(nonce), self.encrypted_body.as_ref())
            .map_err(|_| anyhow::anyhow!("AES-GCM message decryption failed"))
    }

    /// Sign this message with the sender's FALCON-1024 identity.
    ///
    /// Replaces any existing signature.
    pub fn sign(
        &mut self,
        identity: &FalconIdentity,
    ) -> Result<(), anyhow::Error> {
        let payload = self.signing_payload();
        let hash = blake3::hash(&payload);
        self.signature = NodeSigner::sign(identity, hash.as_bytes())?;
        Ok(())
    }

    /// Verify the FALCON-1024 signature using the sender's public key.
    pub fn verify_signature(&self, sender_falcon_pubkey: &[u8]) -> bool {
        if self.signature.is_empty() {
            return false;
        }
        let payload = self.signing_payload();
        let hash = blake3::hash(&payload);
        FalconIdentity::verify_signature(
            sender_falcon_pubkey,
            hash.as_bytes(),
            &self.signature,
        )
        .unwrap_or(false)
    }

    /// Build the deterministic signing payload.
    ///
    /// Concatenates all fields except `signature` in a fixed order so that
    /// both sender and verifier produce the same bytes.
    fn signing_payload(&self) -> Vec<u8> {
        let reply = self.reply_to.as_deref().unwrap_or("");
        let mut buf = Vec::with_capacity(
            self.message_id.len()
                + self.sender_node_id.len()
                + self.recipient_node_id.len()
                + self.encrypted_body.len()
                + self.kem_ciphertext.len()
                + reply.len()
                + self.content_type.len()
                + 8,
        );
        buf.extend_from_slice(self.message_id.as_bytes());
        buf.extend_from_slice(self.sender_node_id.as_bytes());
        buf.extend_from_slice(self.recipient_node_id.as_bytes());
        buf.extend_from_slice(&self.encrypted_body);
        buf.extend_from_slice(&self.kem_ciphertext);
        buf.extend_from_slice(reply.as_bytes());
        buf.extend_from_slice(self.content_type.as_bytes());
        buf.extend_from_slice(&self.created_at.to_le_bytes());
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_message() -> DirectMessage {
        DirectMessage {
            message_id: "msg-abc123".into(),
            sender_node_id: "sender-node-id".into(),
            sender_name: Some("Alice".into()),
            recipient_node_id: "recipient-node-id".into(),
            encrypted_body: vec![0xAA; 64],
            kem_ciphertext: vec![0xBB; 128],
            reply_to: None,
            content_type: "text/plain".into(),
            created_at: 1_700_000_000,
            signature: Vec::new(),
        }
    }

    #[test]
    fn test_direct_message_serialization_roundtrip() {
        let msg = make_test_message();
        let json = serde_json::to_vec(&msg).expect("test: serialize");
        let restored: DirectMessage =
            serde_json::from_slice(&json).expect("test: deserialize");
        assert_eq!(restored.message_id, msg.message_id);
        assert_eq!(restored.sender_node_id, msg.sender_node_id);
        assert_eq!(restored.sender_name, Some("Alice".into()));
        assert_eq!(restored.content_type, "text/plain");
        assert_eq!(restored.encrypted_body, vec![0xAA; 64]);
        assert_eq!(restored.kem_ciphertext, vec![0xBB; 128]);
    }

    #[test]
    fn test_encrypt_decrypt_body_roundtrip() {
        let (recipient_pk, recipient_sk) = kyber1024::keypair();
        let mut msg = make_test_message();

        let plaintext = b"Hello, HyperMesh!";
        msg.encrypt_body(plaintext, recipient_pk.as_bytes())
            .expect("test: encrypt");

        assert!(!msg.encrypted_body.is_empty());
        assert!(!msg.kem_ciphertext.is_empty());

        let decrypted = msg
            .decrypt_body(recipient_sk.as_bytes())
            .expect("test: decrypt");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_wrong_key_fails() {
        let (alice_pk, _alice_sk) = kyber1024::keypair();
        let (_bob_pk, bob_sk) = kyber1024::keypair();

        let mut msg = make_test_message();
        msg.encrypt_body(b"secret", alice_pk.as_bytes())
            .expect("test: encrypt with alice");

        let result = msg.decrypt_body(bob_sk.as_bytes());
        assert!(result.is_err(), "decrypt with wrong key must fail");
    }

    #[test]
    fn test_sign_verify_roundtrip() {
        let identity = FalconIdentity::generate();
        let mut msg = make_test_message();
        msg.sender_node_id = identity.node_id.clone();

        msg.sign(&identity).expect("test: sign");
        assert!(!msg.signature.is_empty());

        assert!(
            msg.verify_signature(&identity.public_key),
            "valid signature must verify"
        );
    }

    #[test]
    fn test_tampered_message_signature_fails() {
        let identity = FalconIdentity::generate();
        let mut msg = make_test_message();
        msg.sender_node_id = identity.node_id.clone();
        msg.sign(&identity).expect("test: sign");

        // Tamper with the message
        msg.content_type = "application/tampered".into();
        assert!(
            !msg.verify_signature(&identity.public_key),
            "tampered message must fail verification"
        );
    }

    #[test]
    fn test_signing_payload_deterministic() {
        let msg = make_test_message();
        let p1 = msg.signing_payload();
        let p2 = msg.signing_payload();
        assert_eq!(p1, p2, "signing payload must be deterministic");
        assert!(!p1.is_empty());
    }
}
