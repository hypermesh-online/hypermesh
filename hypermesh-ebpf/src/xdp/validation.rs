// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! XDP packet validation: userspace validation path and header checks.

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
    /// program handles this at kernel level; this function serves as fallback.
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

/// Serialize a `ValidationPolicy` to a 24-byte `#[repr(C)]` byte array
/// suitable for writing into a BPF hash map.
///
/// Layout (24 bytes):
///   [0]     requires_pos (bool as u8)
///   [1]     validate_asset_hash (bool as u8)
///   [2]     check_matrix_routing (bool as u8)
///   [3]     privacy_tier (u8)
///   [4..8]  max_packet_size (u32 little-endian)
///   [8..12] rate_limit_per_sec (u32 little-endian)
///   [12..20] _reserved (8 bytes)
///   [20..24] padding (zeros)
#[cfg(any(feature = "kernel-attach", test))]
pub(crate) fn policy_to_bytes(policy: &crate::policy_maps::ValidationPolicy) -> [u8; 24] {
    let mut buf = [0u8; 24];
    buf[0] = policy.requires_pos as u8;
    buf[1] = policy.validate_asset_hash as u8;
    buf[2] = policy.check_matrix_routing as u8;
    buf[3] = policy.privacy_tier;
    buf[4..8].copy_from_slice(&policy.max_packet_size.to_le_bytes());
    buf[8..12].copy_from_slice(&policy.rate_limit_per_sec.to_le_bytes());
    // bytes 12..24 remain zero (reserved + padding)
    buf
}
