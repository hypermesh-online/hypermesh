// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Key derivation for the segmented asset pipeline.
//!
//! One Kyber KEM operation per asset produces a shared secret.
//! BLAKE3 `derive_key` (domain-separated HKDF) turns that into:
//!   - A 32-byte master key (one per asset)
//!   - A unique 32-byte AES-256 key + 12-byte nonce per segment
//!
//! This avoids per-segment KEM operations while giving every segment
//! a unique, deterministic key+nonce pair derived from the master key.

/// Derive a 32-byte master key from a Kyber shared secret.
/// Uses BLAKE3's derive_key function with a domain separation string.
pub fn derive_master_key(shared_secret: &[u8]) -> [u8; 32] {
    blake3::derive_key("HYPERMESH-ASSET-KEY-V1", shared_secret)
}

/// Derive a per-segment AES-256 key and 12-byte nonce from the master key and segment index.
/// Each segment gets a unique, deterministic key+nonce pair.
pub fn derive_segment_key(master_key: &[u8; 32], segment_index: u32) -> ([u8; 32], [u8; 12]) {
    let mut input = Vec::with_capacity(36);
    input.extend_from_slice(master_key);
    input.extend_from_slice(&segment_index.to_le_bytes());

    let key = blake3::derive_key("HYPERMESH-SEGMENT-KEY-V1", &input);

    let nonce_full = blake3::derive_key("HYPERMESH-SEGMENT-NONCE-V1", &input);
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&nonce_full[..12]);

    (key, nonce)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_master_key_deterministic() {
        let secret = b"test-shared-secret-from-kyber-kem";
        let key1 = derive_master_key(secret);
        let key2 = derive_master_key(secret);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_derive_master_key_different_inputs() {
        let key_a = derive_master_key(b"secret-a");
        let key_b = derive_master_key(b"secret-b");
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn test_derive_segment_key_deterministic() {
        let master = derive_master_key(b"determinism-test");
        let (key1, nonce1) = derive_segment_key(&master, 42);
        let (key2, nonce2) = derive_segment_key(&master, 42);
        assert_eq!(key1, key2);
        assert_eq!(nonce1, nonce2);
    }

    #[test]
    fn test_segment_keys_differ_per_index() {
        let master = derive_master_key(b"per-index-test");
        let (key0, nonce0) = derive_segment_key(&master, 0);
        let (key1, nonce1) = derive_segment_key(&master, 1);
        let (key2, nonce2) = derive_segment_key(&master, 2);

        assert_ne!(key0, key1);
        assert_ne!(key1, key2);
        assert_ne!(key0, key2);

        assert_ne!(nonce0, nonce1);
        assert_ne!(nonce1, nonce2);
        assert_ne!(nonce0, nonce2);
    }

    #[test]
    fn test_nonce_is_12_bytes() {
        let master = derive_master_key(b"nonce-len-test");
        let (_key, nonce) = derive_segment_key(&master, 0);
        assert_eq!(nonce.len(), 12);
    }

    #[test]
    fn test_key_is_32_bytes() {
        let master = derive_master_key(b"key-len-test");
        let (key, _nonce) = derive_segment_key(&master, 0);
        assert_eq!(key.len(), 32);
    }
}
