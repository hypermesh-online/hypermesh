// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! ShareInvite — a signed invitation containing shard map + encrypted key.
//!
//! Sent peer-to-peer over STOQ with TAG_SHARE_INVITE (0x05).

use hypermesh_lib::NodeSigner;
use serde::{Deserialize, Serialize};

use crate::identity::FalconIdentity;

/// A signed share invitation containing shard map and encrypted decryption key.
///
/// The sender encrypts the asset's `DecryptionKey` for the recipient using
/// Kyber-1024 KEM (see `key_wrap`), includes the shard map so the recipient
/// knows where to fetch shards, and signs the entire payload with FALCON-1024.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareInvite {
    /// Unique invite identifier (UUID).
    pub invite_id: String,
    /// Asset being shared.
    pub asset_id: String,
    /// Sender's node ID (BLAKE3 of FALCON pubkey).
    pub sender_node_id: String,
    /// Optional human-readable sender name.
    pub sender_name: Option<String>,
    /// Recipient's node ID.
    pub recipient_node_id: String,
    /// Human-readable asset name.
    pub asset_name: String,
    /// Total asset size in bytes (pre-pipeline).
    pub asset_size: u64,
    /// Number of shards.
    pub shard_count: u32,
    /// JSON-encoded shard map (shard locations + hashes).
    #[serde(with = "serde_bytes")]
    pub shard_map_json: Vec<u8>,
    /// Kyber-1024 wrapped DecryptionKey (AES-GCM ciphertext).
    #[serde(with = "serde_bytes")]
    pub encrypted_key: Vec<u8>,
    /// Kyber KEM ciphertext for key unwrapping.
    #[serde(with = "serde_bytes")]
    pub key_kem_ciphertext: Vec<u8>,
    /// Unix timestamp (seconds) when the invite was created.
    pub created_at: i64,
    /// FALCON-1024 signature over the signing payload.
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,
}

impl ShareInvite {
    /// Create a new invite with all fields. Signature is empty until [`sign`] is called.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        invite_id: String,
        asset_id: String,
        sender_node_id: String,
        sender_name: Option<String>,
        recipient_node_id: String,
        asset_name: String,
        asset_size: u64,
        shard_count: u32,
        shard_map_json: Vec<u8>,
        encrypted_key: Vec<u8>,
        key_kem_ciphertext: Vec<u8>,
        created_at: i64,
    ) -> Self {
        Self {
            invite_id,
            asset_id,
            sender_node_id,
            sender_name,
            recipient_node_id,
            asset_name,
            asset_size,
            shard_count,
            shard_map_json,
            encrypted_key,
            key_kem_ciphertext,
            created_at,
            signature: Vec::new(),
        }
    }

    /// Sign this invite with the sender's FALCON-1024 identity.
    ///
    /// Replaces any existing signature.
    pub fn sign(&mut self, identity: &FalconIdentity) -> Result<(), anyhow::Error> {
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
        FalconIdentity::verify_signature(sender_falcon_pubkey, hash.as_bytes(), &self.signature)
            .unwrap_or(false)
    }

    /// Build the deterministic signing payload.
    ///
    /// Concatenates all fields except `signature` in a fixed order so that
    /// both sender and verifier produce the same bytes.
    fn signing_payload(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(
            self.invite_id.len()
                + self.asset_id.len()
                + self.sender_node_id.len()
                + self.recipient_node_id.len()
                + self.asset_name.len()
                + 8
                + 4
                + self.shard_map_json.len()
                + self.encrypted_key.len()
                + self.key_kem_ciphertext.len()
                + 8,
        );
        buf.extend_from_slice(self.invite_id.as_bytes());
        buf.extend_from_slice(self.asset_id.as_bytes());
        buf.extend_from_slice(self.sender_node_id.as_bytes());
        buf.extend_from_slice(self.recipient_node_id.as_bytes());
        buf.extend_from_slice(self.asset_name.as_bytes());
        buf.extend_from_slice(&self.asset_size.to_le_bytes());
        buf.extend_from_slice(&self.shard_count.to_le_bytes());
        buf.extend_from_slice(&self.shard_map_json);
        buf.extend_from_slice(&self.encrypted_key);
        buf.extend_from_slice(&self.key_kem_ciphertext);
        buf.extend_from_slice(&self.created_at.to_le_bytes());
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_invite() -> ShareInvite {
        ShareInvite::new(
            "inv-001".into(),
            "asset-abc".into(),
            "sender-node-id".into(),
            Some("Alice".into()),
            "recipient-node-id".into(),
            "photo.jpg".into(),
            1_048_576,
            14,
            b"[{\"shard\":0}]".to_vec(),
            vec![0xAA; 64],
            vec![0xBB; 128],
            1_700_000_000,
        )
    }

    #[test]
    fn test_share_invite_serialization_roundtrip() {
        let invite = make_test_invite();
        let json = serde_json::to_vec(&invite).expect("test: serialize");
        let restored: ShareInvite = serde_json::from_slice(&json).expect("test: deserialize");
        assert_eq!(restored.invite_id, invite.invite_id);
        assert_eq!(restored.asset_id, invite.asset_id);
        assert_eq!(restored.sender_name, Some("Alice".into()));
        assert_eq!(restored.shard_count, 14);
        assert_eq!(restored.encrypted_key, vec![0xAA; 64]);
        assert_eq!(restored.key_kem_ciphertext, vec![0xBB; 128]);
    }

    #[test]
    fn test_signing_payload_deterministic() {
        let invite = make_test_invite();
        let p1 = invite.signing_payload();
        let p2 = invite.signing_payload();
        assert_eq!(p1, p2, "signing payload must be deterministic");
        assert!(!p1.is_empty());
    }

    #[test]
    fn test_sign_and_verify() {
        let identity = FalconIdentity::generate();
        let mut invite = make_test_invite();
        invite.sender_node_id = identity.node_id.clone();

        invite.sign(&identity).expect("test: sign");
        assert!(!invite.signature.is_empty());

        assert!(
            invite.verify_signature(&identity.public_key),
            "valid signature must verify"
        );

        // Tamper and re-verify
        invite.asset_name = "tampered.jpg".into();
        assert!(
            !invite.verify_signature(&identity.public_key),
            "tampered invite must fail verification"
        );
    }

    #[test]
    fn test_verify_empty_signature_returns_false() {
        let identity = FalconIdentity::generate();
        let invite = make_test_invite();
        assert!(
            !invite.verify_signature(&identity.public_key),
            "empty signature must return false"
        );
    }
}
