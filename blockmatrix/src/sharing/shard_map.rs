// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! On-disk shard map — the locate + integrity record for a stored asset.
//!
//! This is the instruction half of R6 instruction-based retrieval: it tells a
//! fetcher WHICH shards make up an asset, WHERE they are, and how to verify
//! them (BLAKE3 hashes). It carries NO raw decryption key.
//!
//! ## Custody model (F5)
//!
//! Encrypted (Private) assets attach a [`KeyEnvelope`] — the asset's
//! `DecryptionKey` wrapped for a Kyber public key. On the owner's disk the
//! envelope is wrapped for the owner's *own* node Kyber identity (self-custody)
//! so a self-fetch can recover the key by decapsulating with the node Kyber
//! secret held in the keystore. The raw Kyber secret key is NEVER serialized
//! into this map.
//!
//! Public/Anonymous assets are content-addressed cleartext shards and carry no
//! envelope at all (`key_envelope: None`).

use serde::{Deserialize, Serialize};

use crate::assets::pipeline::ShardMetadata;
use crate::sharing::key_wrap::KeyEnvelope;

/// The persisted shard map for a stored asset.
///
/// Replaces the previous on-disk representation that serialized a cleartext
/// `DecryptionKey` (including a raw Kyber secret key). Reconstruction reads
/// this map, fetches shards by hash, and — if `key_envelope` is present —
/// unwraps the decryption key using the local node's Kyber secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMap {
    /// Content-addressed asset identifier (BLAKE3 hex of the original bytes).
    pub asset_id: String,
    /// BLAKE3 hex of each shard, in order (data shards then parity).
    pub shard_hashes: Vec<String>,
    /// Wrapped decryption key for encrypted (Private) assets.
    ///
    /// `None` means the shards are cleartext (Public/Anonymous) — no key is
    /// needed to reconstruct, only BLAKE3 integrity verification.
    #[serde(default)]
    pub key_envelope: Option<KeyEnvelope>,
    /// Number of shards.
    pub shard_count: usize,
    /// Pre-pipeline original size in bytes.
    pub original_size: usize,
    /// Per-shard metadata (index/parity/size), needed for RS reconstruction.
    pub shard_metadata: Vec<ShardMetadata>,
}

impl ShardMap {
    /// Whether this asset's shards are cleartext (no decryption key).
    pub fn is_cleartext(&self) -> bool {
        self.key_envelope.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::pipeline::orchestrator::DecryptionKey;

    fn sample_metadata(n: usize) -> Vec<ShardMetadata> {
        (0..n)
            .map(|i| ShardMetadata {
                index: i,
                is_parity: i >= 10,
                size: 1024,
                original_size: 1024,
                hash: format!("{i:064x}"),
            })
            .collect()
    }

    #[test]
    fn test_cleartext_map_has_no_envelope() {
        let map = ShardMap {
            asset_id: "abc".into(),
            shard_hashes: vec!["0".repeat(64)],
            key_envelope: None,
            shard_count: 1,
            original_size: 10,
            shard_metadata: sample_metadata(1),
        };
        assert!(map.is_cleartext());
        let json = serde_json::to_string(&map).expect("test: serialize");
        // No decryption-key field name should be present in cleartext maps.
        assert!(!json.contains("secret_key"));
    }

    /// Regression (F5): a serialized encrypted `ShardMap` must contain no raw
    /// Kyber secret-key bytes — only the wrapped envelope.
    #[test]
    fn test_encrypted_map_hides_raw_secret_key() {
        let (owner_pk, owner_sk) = pqcrypto_kyber::kyber1024::keypair();
        use pqcrypto_traits::kem::{PublicKey, SecretKey};

        let raw_secret = vec![0xAB; 3168];
        let dk = DecryptionKey::Kyber {
            ciphertext_kem: vec![0x01; 100],
            nonce: vec![0x02; 12],
            original_size: 5,
            secret_key: raw_secret.clone(),
        };
        let envelope =
            KeyEnvelope::wrap_for(&dk, owner_pk.as_bytes()).expect("test: wrap for owner");

        let map = ShardMap {
            asset_id: "def".into(),
            shard_hashes: vec!["1".repeat(64)],
            key_envelope: Some(envelope),
            shard_count: 1,
            original_size: 5,
            shard_metadata: sample_metadata(1),
        };

        let serialized = serde_json::to_vec(&map).expect("test: serialize map");
        let needle = &raw_secret[..64];
        assert!(
            !serialized.windows(needle.len()).any(|w| w == needle),
            "raw Kyber secret bytes leaked into serialized ShardMap"
        );

        // And the owner can still recover the key by unwrapping.
        let recovered = map
            .key_envelope
            .as_ref()
            .expect("test: envelope present")
            .unwrap_with(owner_sk.as_bytes())
            .expect("test: unwrap");
        let orig_json = serde_json::to_string(&dk).expect("test: ser dk");
        let rec_json = serde_json::to_string(&recovered).expect("test: ser rec");
        assert_eq!(orig_json, rec_json);
    }
}
