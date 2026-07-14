// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Kernel-map serializers for the XDP allowlist datapath.
//!
//! These functions encode userspace state into the exact wire layout the
//! kernel XDP program's BPF maps expect, and are consumed by `manager.rs`
//! under the `kernel-attach` feature.
//!
//! The former userspace EXT_* packet parser (`validate_packet` and the
//! per-header `validate_*` helpers) was removed with the F10 reframe: STOQ is
//! encrypted QUIC, so the plaintext extension headers that parser inspected
//! were never emitted on the wire. The honest kernel gate admits/drops by the
//! PoS-authenticated-peer allowlist populated through these serializers.

/// Serialize a `ValidationPolicy` to the 16-byte `struct policy_value`
/// wire format the kernel XDP program expects (all fields `u32` LE).
///
/// This MUST match the C `struct policy_value` in `hypermesh_xdp.c`:
/// ```c
/// struct policy_value {
///     __u32 requires_pos;
///     __u32 validate_asset_hash;
///     __u32 check_matrix_routing;
///     __u32 privacy_tier;
/// };
/// ```
/// Layout (16 bytes, little-endian):
///   `[0..4]`   requires_pos         (bool as 0/1)
///   `[4..8]`   validate_asset_hash  (bool as 0/1)
///   `[8..12]`  check_matrix_routing (bool as 0/1)
///   `[12..16]` privacy_tier         (u8 zero-extended)
///
/// `max_packet_size` / `rate_limit_per_sec` are userspace-only fields (the
/// kernel policy struct does not carry them).
#[cfg(any(feature = "kernel-attach", test))]
pub(crate) fn policy_to_bytes(policy: &crate::policy_maps::ValidationPolicy) -> [u8; 16] {
    let mut buf = [0u8; 16];
    buf[0..4].copy_from_slice(&(policy.requires_pos as u32).to_le_bytes());
    buf[4..8].copy_from_slice(&(policy.validate_asset_hash as u32).to_le_bytes());
    buf[8..12].copy_from_slice(&(policy.check_matrix_routing as u32).to_le_bytes());
    buf[12..16].copy_from_slice(&u32::from(policy.privacy_tier).to_le_bytes());
    buf
}

/// Serialize an asset-hash registry entry to the 40-byte
/// `struct asset_hash_entry` wire format the kernel expects.
///
/// Matches the C struct in `hypermesh_xdp.c`:
/// ```c
/// struct asset_hash_entry {
///     __u8  expected_hash[32];
///     __u32 shard_count;
///     __u8  registered;   /* padded to 4-byte alignment -> 40 bytes total */
/// };
/// ```
/// Layout (40 bytes, little-endian):
///   `[0..32]`  expected_hash (BLAKE3, 32 bytes)
///   `[32..36]` shard_count   (u32 LE)
///   `[36]`     registered    (1 = registered on blockchain)
///   `[37..40]` padding
#[cfg(any(feature = "kernel-attach", test))]
pub(crate) fn asset_hash_entry_to_bytes(
    expected_hash: &[u8; 32],
    shard_count: u32,
    registered: bool,
) -> [u8; 40] {
    let mut buf = [0u8; 40];
    buf[0..32].copy_from_slice(expected_hash);
    buf[32..36].copy_from_slice(&shard_count.to_le_bytes());
    buf[36] = registered as u8;
    buf
}

/// Serialize a PoS validation cache entry to the 24-byte
/// `struct pos_validation` wire format the kernel expects.
///
/// Matches the C struct in `hypermesh_xdp.c`:
/// ```c
/// struct pos_validation {
///     __u8  algorithm;
///     __u32 difficulty;
///     __u8  validated;
///     __u64 last_validated;   /* 8-byte aligned -> 24 bytes total */
/// };
/// ```
/// Layout (24 bytes, little-endian, natural C alignment):
///   `[0]`      algorithm      (0x01 FALCON / 0x02 Ed25519 / 0x03 ECDSA)
///   `[1..4]`   padding
///   `[4..8]`   difficulty     (u32 LE)
///   `[8]`      validated      (1 = passed cryptographic verification)
///   `[9..16]`  padding
///   `[16..24]` last_validated (u64 LE, bpf_ktime_get_ns() at write time)
#[cfg(any(feature = "kernel-attach", test))]
pub(crate) fn pos_validation_to_bytes(
    algorithm: u8,
    difficulty: u32,
    validated: bool,
    last_validated_ns: u64,
) -> [u8; 24] {
    let mut buf = [0u8; 24];
    buf[0] = algorithm;
    buf[4..8].copy_from_slice(&difficulty.to_le_bytes());
    buf[8] = validated as u8;
    buf[16..24].copy_from_slice(&last_validated_ns.to_le_bytes());
    buf
}

#[cfg(test)]
mod map_serializer_tests {
    use super::*;
    use crate::policy_maps::ValidationPolicy;

    // These byte-level tests verify the userspace map-population wire
    // format matches the C structs in `hypermesh_xdp.c` EXACTLY. They do
    // not require a running kernel — the kernel DROP == allowlist-reject
    // behaviour is verified separately on the remote (P8).

    #[test]
    fn asset_hash_entry_layout_matches_c_struct() {
        // C: __u8 expected_hash[32]; __u32 shard_count; __u8 registered;
        // padded to 40 bytes.
        let hash = [0x11u8; 32];
        let bytes = asset_hash_entry_to_bytes(&hash, 14, true);
        assert_eq!(bytes.len(), 40);
        assert_eq!(&bytes[0..32], &hash);
        assert_eq!(u32::from_le_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]), 14);
        assert_eq!(bytes[36], 1); // registered
        assert_eq!(&bytes[37..40], &[0u8; 3]); // padding
    }

    #[test]
    fn asset_hash_entry_unregistered() {
        let bytes = asset_hash_entry_to_bytes(&[0u8; 32], 0, false);
        assert_eq!(bytes[36], 0);
    }

    #[test]
    fn pos_validation_layout_matches_c_struct() {
        // C: __u8 algorithm; __u32 difficulty; __u8 validated;
        // __u64 last_validated; (natural alignment -> 24 bytes)
        let bytes = pos_validation_to_bytes(0x01, 8, true, 0xDEAD_BEEF_u64);
        assert_eq!(bytes.len(), 24);
        assert_eq!(bytes[0], 0x01); // algorithm = FALCON
        assert_eq!(u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]), 8);
        assert_eq!(bytes[8], 1); // validated
        let ts = u64::from_le_bytes([
            bytes[16], bytes[17], bytes[18], bytes[19],
            bytes[20], bytes[21], bytes[22], bytes[23],
        ]);
        assert_eq!(ts, 0xDEAD_BEEF);
    }

    #[test]
    fn pos_validation_not_validated() {
        let bytes = pos_validation_to_bytes(0x01, 8, false, 0);
        assert_eq!(bytes[8], 0);
    }

    #[test]
    fn policy_value_is_16_bytes_four_u32() {
        // Mirrors the kernel `struct policy_value` (4x u32).
        let strict = policy_to_bytes(&ValidationPolicy::strict());
        assert_eq!(strict.len(), 16);
        // strict => requires_pos=1, validate_asset_hash=1,
        // check_matrix_routing=1, privacy_tier=2.
        assert_eq!(u32::from_le_bytes([strict[0], strict[1], strict[2], strict[3]]), 1);
        assert_eq!(u32::from_le_bytes([strict[12], strict[13], strict[14], strict[15]]), 2);
    }
}
