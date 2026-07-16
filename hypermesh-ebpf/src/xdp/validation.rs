// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! XDP packet validation: userspace validation path, header checks, and the
//! kernel-map serializers for the XDP allowlist datapath.
//!
//! The userspace EXT_* packet parser (`validate_packet` and the per-header
//! `validate_*` helpers) is the documented §5.4 userspace pre-validation path.
//! It is LIVE again under A4-CORRECT because STOQ now emits the plaintext
//! HyperMesh extension header on the send path (`apply_extensions`) — it was
//! only ever "dead" because nothing emitted the header it parses. With
//! `kernel-attach`, the XDP program handles classification in-kernel; this path
//! is the userspace fallback / deep-validation companion.
//!
//! The serializer functions (`policy_to_bytes`, `pos_validation_to_bytes`,
//! `asset_hash_entry_to_bytes`) encode userspace state into the exact wire
//! layout the kernel XDP program's BPF maps expect, and are consumed by
//! `manager.rs` under the `kernel-attach` feature.

use crate::hypermesh_headers::*;
use crate::validation::FastValidationResult;

use super::manager::XdpManager;
use super::types::*;

// -----------------------------------------------------------------------
// Packet validation (userspace path)
// -----------------------------------------------------------------------

impl XdpManager {
    /// Validate a packet and return a decision (the three execution paths).
    ///
    /// This is the userspace validation path. With kernel-attach, the XDP
    /// program handles this at kernel level; this function serves as fallback
    /// and as the deep-validation companion to the kernel fast path.
    ///
    /// Enforces all policy flags:
    /// - `max_packet_size`: Drop oversized packets
    /// - `requires_pos`: Parse and validate PoS header from packet
    /// - `validate_asset_hash`: Check asset hash header in packet
    /// - `check_matrix_routing`: Verify matrix routing header in packet
    pub fn validate_packet(&self, connection_id: u64, packet_data: &[u8]) -> PacketDecision {
        let policy = self.policy_manager.get_policy(connection_id);

        // Check packet size
        if packet_data.len() > policy.max_packet_size as usize {
            return PacketDecision::Drop {
                reason: format!(
                    "Packet too large: {} > {}",
                    packet_data.len(),
                    policy.max_packet_size
                ),
            };
        }

        // Enforce PoS validation when required by policy
        if policy.requires_pos {
            if packet_data.len() < ProofOfStateHeader::SIZE {
                return PacketDecision::Drop {
                    reason: format!(
                        "Packet too short for PoS header: {} < {}",
                        packet_data.len(),
                        ProofOfStateHeader::SIZE
                    ),
                };
            }

            match ProofOfStateHeader::from_bytes(packet_data) {
                Some(header) => {
                    let result = self.pos_validator.validate_fast(&header);
                    if !result.all_ok() {
                        return PacketDecision::Drop {
                            reason: format!(
                                "PoS validation failed: timestamp={}, stake={}, work={}, space={}",
                                result.timestamp_ok,
                                result.stake_ok,
                                result.work_ok,
                                result.space_ok
                            ),
                        };
                    }
                }
                None => {
                    return PacketDecision::Drop {
                        reason: "Failed to parse PoS header".to_string(),
                    };
                }
            }
        }

        // Enforce asset hash validation when required by policy
        if policy.validate_asset_hash {
            // Asset hash header follows PoS header (or starts at offset 0
            // if PoS is not required).
            let offset = if policy.requires_pos {
                ProofOfStateHeader::SIZE
            } else {
                0
            };

            if packet_data.len() < offset + AssetHashHeader::SIZE {
                return PacketDecision::Drop {
                    reason: format!(
                        "Packet too short for asset hash header at offset {}: {} < {}",
                        offset,
                        packet_data.len(),
                        offset + AssetHashHeader::SIZE
                    ),
                };
            }

            match AssetHashHeader::from_bytes(&packet_data[offset..]) {
                Some(header) => {
                    if !header.validate_shard_indices() {
                        return PacketDecision::Drop {
                            reason: format!(
                                "Invalid shard indices: {}/{}",
                                header.shard_index, header.shard_count
                            ),
                        };
                    }
                }
                None => {
                    return PacketDecision::Drop {
                        reason: "Failed to parse asset hash header".to_string(),
                    };
                }
            }
        }

        // Enforce matrix routing validation when required by policy
        if policy.check_matrix_routing {
            // Routing header follows PoS + asset hash headers
            let mut offset = 0;
            if policy.requires_pos {
                offset += ProofOfStateHeader::SIZE;
            }
            if policy.validate_asset_hash {
                offset += AssetHashHeader::SIZE;
            }

            if packet_data.len() < offset + MatrixRoutingHeader::MIN_SIZE {
                return PacketDecision::Drop {
                    reason: format!(
                        "Packet too short for routing header at offset {}: {} < {}",
                        offset,
                        packet_data.len(),
                        offset + MatrixRoutingHeader::MIN_SIZE
                    ),
                };
            }

            match MatrixRoutingHeader::from_bytes(&packet_data[offset..]) {
                Some(routing) => {
                    // Use u16::MAX as matrix size bound (permissive)
                    if !routing.validate_path(u16::MAX) {
                        return PacketDecision::Drop {
                            reason: "Matrix routing path validation failed".to_string(),
                        };
                    }
                }
                None => {
                    return PacketDecision::Drop {
                        reason: "Failed to parse matrix routing header".to_string(),
                    };
                }
            }
        }

        // Default: pass to userspace for processing
        PacketDecision::Pass
    }

    /// Validate a packet returning legacy FilterAction for backward compatibility
    pub fn validate_packet_userspace(
        &self,
        connection_id: u64,
        packet_data: &[u8],
    ) -> FilterAction {
        match self.validate_packet(connection_id, packet_data) {
            PacketDecision::Pass => FilterAction::Pass,
            PacketDecision::Redirect { .. } => FilterAction::Redirect,
            PacketDecision::Forward { .. } => FilterAction::Pass,
            PacketDecision::Drop { .. } => FilterAction::Drop,
        }
    }

    /// Validate Proof of State extension header using the enhanced four-proof
    /// validator. Returns true only if all four proofs pass fast validation.
    pub fn validate_proof_of_state(&self, proof: &ProofOfStateHeader) -> bool {
        let result = self.pos_validator.validate_fast(proof);
        if !result.all_ok() {
            tracing::warn!(
                "Proof of State fast validation failed: timestamp={}, stake={}, work={}, space={}",
                result.timestamp_ok,
                result.stake_ok,
                result.work_ok,
                result.space_ok
            );
            return false;
        }
        true
    }

    /// Validate Proof of State with detailed per-proof results.
    pub fn validate_proof_of_state_detailed(
        &self,
        proof: &ProofOfStateHeader,
    ) -> FastValidationResult {
        self.pos_validator.validate_fast(proof)
    }

    /// Validate Asset Hash extension header
    pub fn validate_asset_hash(&self, header: &AssetHashHeader, _payload: &[u8]) -> bool {
        if !header.validate_shard_indices() {
            tracing::warn!("Invalid shard indices in asset hash header");
            return false;
        }
        true
    }

    /// Validate Matrix Routing extension header
    pub fn validate_matrix_routing(&self, routing: &MatrixRoutingHeader, matrix_size: u16) -> bool {
        if !routing.validate_path(matrix_size) {
            tracing::warn!("Invalid matrix routing path");
            return false;
        }
        true
    }
}

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
