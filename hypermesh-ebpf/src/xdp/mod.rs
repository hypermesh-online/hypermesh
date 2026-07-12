// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Unified XDP (eXpress Data Path) Management
//!
//! Provides kernel-level packet classification, filtering, and routing
//! for the HyperMesh node. Merges HyperMesh intelligence validation
//! (PoS, asset hash, routing) with XDP program attachment and management.
//!
//! This is THE single XDP manager for the entire HyperMesh stack.
//! STOQ and blockmatrix are consumers via the `HyperMeshEbpf` orchestrator.

mod manager;
mod types;
mod validation;

pub use manager::*;
pub use types::*;
// `policy_to_bytes` / the other `*_to_bytes` serializers are consumed
// directly by `manager.rs` (via `super::validation::...`) under
// kernel-attach and by the tests below; no crate-level re-export needed.
#[cfg(test)]
use validation::policy_to_bytes;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hypermesh_headers::*;
    use crate::policy_maps::{PolicyManager, ValidationPolicy};
    use crate::validation::ALG_FALCON_1024;

    /// Build a valid `who` field: FALCON-1024 algorithm indicator + 8 non-zero prefix bytes.
    fn valid_who() -> [u8; 32] {
        let mut who = [0xABu8; 32];
        who[0] = ALG_FALCON_1024;
        who
    }

    /// Build a valid `what` field: first byte zero (8 leading zero bits meets default difficulty).
    fn valid_what() -> [u8; 32] {
        let mut what = [0xFFu8; 32];
        what[0] = 0x00;
        what
    }

    /// Build a valid `where_` field: IPv6 global unicast prefix (0x20).
    fn valid_where() -> [u8; 16] {
        let mut w = [0x01u8; 16];
        w[0] = 0x20;
        w
    }

    fn now_micros() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("test: get system time")
            .as_micros() as u64
    }

    #[test]
    fn test_xdp_manager_creation() {
        let policy = PolicyManager::new().expect("test: create policy manager");
        let manager = XdpManager::new(policy);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_filter_config_default() {
        let config = XdpFilterConfig::default();
        assert!(config.filter_quic_only);
        assert!(config.drop_ipv4);
        assert_eq!(config.max_packet_size, 65535);
    }

    #[test]
    fn test_packet_decision_pass() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        pm.set_default_policy(ValidationPolicy::permissive());
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let packet = vec![0u8; 1500];
        let decision = mgr.validate_packet(123, &packet);
        assert_eq!(decision, PacketDecision::Pass);
    }

    #[test]
    fn test_packet_decision_drop_oversized() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let large_packet = vec![0u8; 70000];
        let decision = mgr.validate_packet(123, &large_packet);
        match decision {
            PacketDecision::Drop { reason } => {
                assert!(reason.contains("too large"));
            }
            other => unreachable!("test: expected Drop, got {:?}", other),
        }
    }

    #[test]
    fn test_userspace_validation_compat() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        pm.set_default_policy(ValidationPolicy::permissive());
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let packet = vec![0u8; 1500];
        let action = mgr.validate_packet_userspace(123, &packet);
        assert_eq!(action, FilterAction::Pass);
    }

    #[test]
    fn test_proof_of_state_validation_valid() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let valid_proof = ProofOfStateHeader {
            who: valid_who(),
            what: valid_what(),
            when: now_micros(),
            where_: valid_where(),
        };
        assert!(mgr.validate_proof_of_state(&valid_proof));
    }

    #[test]
    fn test_proof_of_state_validation_future_timestamp() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let future_proof = ProofOfStateHeader {
            who: valid_who(),
            what: valid_what(),
            when: now_micros() + 10 * 60 * 1_000_000, // 10 min in future
            where_: valid_where(),
        };
        assert!(!mgr.validate_proof_of_state(&future_proof));
    }

    #[test]
    fn test_proof_of_state_detailed_results() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let proof = ProofOfStateHeader {
            who: valid_who(),
            what: valid_what(),
            when: now_micros(),
            where_: valid_where(),
        };

        let result = mgr.validate_proof_of_state_detailed(&proof);
        assert!(result.all_ok());
        assert!(result.timestamp_ok);
        assert!(result.stake_ok);
        assert!(result.work_ok);
        assert!(result.space_ok);
    }

    #[test]
    fn test_sync_policies_to_bpf_no_kernel() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        pm.set_policy(1, ValidationPolicy::strict());
        pm.set_policy(2, ValidationPolicy::permissive());
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        // Without kernel-attach, this should succeed as a no-op
        assert!(mgr.sync_policies_to_bpf().is_ok());
    }

    #[test]
    fn test_policy_to_bytes_matches_kernel_policy_value() {
        // policy_to_bytes must produce the 16-byte `struct policy_value`
        // (4x u32 LE) the kernel XDP program reads. See hypermesh_xdp.c.
        let policy = ValidationPolicy::strict();
        let bytes = policy_to_bytes(&policy);
        assert_eq!(bytes.len(), 16);
        // requires_pos = true -> 1u32 LE
        assert_eq!(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]), 1);
        // validate_asset_hash = true -> 1u32 LE
        assert_eq!(u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]), 1);
        // check_matrix_routing = true -> 1u32 LE
        assert_eq!(u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]), 1);
        // privacy_tier = 2 -> 2u32 LE
        assert_eq!(u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]), 2);
    }

    #[test]
    fn test_permissive_policy_to_bytes_all_zero_flags() {
        let bytes = policy_to_bytes(&ValidationPolicy::permissive());
        assert_eq!(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]), 0);
        assert_eq!(u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]), 0);
        assert_eq!(u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]), 0);
    }

    // -------------------------------------------------------------------
    // Policy enforcement tests (validate_packet with policy flags)
    // -------------------------------------------------------------------

    /// Build a valid PoS header as raw bytes.
    fn valid_pos_bytes() -> Vec<u8> {
        let header = ProofOfStateHeader {
            who: valid_who(),
            what: valid_what(),
            when: now_micros(),
            where_: valid_where(),
        };
        header.to_bytes()
    }

    /// Build a valid asset hash header as raw bytes.
    fn valid_asset_hash_bytes() -> Vec<u8> {
        let header = AssetHashHeader {
            asset_id: [0x01; 32],
            hash: [0x02; 32],
            shard_count: 10,
            shard_index: 3,
        };
        header.to_bytes()
    }

    /// Build a valid matrix routing header as raw bytes.
    fn valid_routing_bytes() -> Vec<u8> {
        let header = MatrixRoutingHeader {
            source: MatrixCoordinate { x: 0, y: 0, z: 0 },
            destination: MatrixCoordinate { x: 5, y: 5, z: 0 },
            path: vec![],
        };
        header.to_bytes()
    }

    #[test]
    fn test_policy_pos_required_drops_short_packet() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut policy = ValidationPolicy::permissive();
        policy.requires_pos = true;
        pm.set_default_policy(policy);
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        // Too short for PoS header (88 bytes needed)
        let packet = vec![0u8; 50];
        let decision = mgr.validate_packet(0, &packet);
        match decision {
            PacketDecision::Drop { reason } => {
                assert!(reason.contains("too short for PoS"));
            }
            other => unreachable!("test: expected Drop, got {:?}", other),
        }
    }

    #[test]
    fn test_policy_pos_required_passes_valid_packet() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut policy = ValidationPolicy::permissive();
        policy.requires_pos = true;
        pm.set_default_policy(policy);
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let packet = valid_pos_bytes();
        let decision = mgr.validate_packet(0, &packet);
        assert_eq!(decision, PacketDecision::Pass);
    }

    #[test]
    fn test_policy_pos_required_drops_invalid_proof() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut policy = ValidationPolicy::permissive();
        policy.requires_pos = true;
        pm.set_default_policy(policy);
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        // Build a PoS header with invalid who (bad algorithm indicator)
        let mut bad_who = [0xFF; 32];
        bad_who[0] = 0x99; // invalid algorithm
        let header = ProofOfStateHeader {
            who: bad_who,
            what: valid_what(),
            when: now_micros(),
            where_: valid_where(),
        };
        let packet = header.to_bytes();
        let decision = mgr.validate_packet(0, &packet);
        match decision {
            PacketDecision::Drop { reason } => {
                assert!(reason.contains("PoS validation failed"));
            }
            other => unreachable!("test: expected Drop, got {:?}", other),
        }
    }

    #[test]
    fn test_policy_asset_hash_required_drops_short_packet() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut policy = ValidationPolicy::permissive();
        policy.validate_asset_hash = true;
        pm.set_default_policy(policy);
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        // Too short for asset hash header (72 bytes needed, 0 offset)
        let packet = vec![0u8; 50];
        let decision = mgr.validate_packet(0, &packet);
        match decision {
            PacketDecision::Drop { reason } => {
                assert!(reason.contains("too short for asset hash"));
            }
            other => unreachable!("test: expected Drop, got {:?}", other),
        }
    }

    #[test]
    fn test_policy_asset_hash_required_passes_valid() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut policy = ValidationPolicy::permissive();
        policy.validate_asset_hash = true;
        pm.set_default_policy(policy);
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let packet = valid_asset_hash_bytes();
        let decision = mgr.validate_packet(0, &packet);
        assert_eq!(decision, PacketDecision::Pass);
    }

    #[test]
    fn test_policy_asset_hash_drops_invalid_shard() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut policy = ValidationPolicy::permissive();
        policy.validate_asset_hash = true;
        pm.set_default_policy(policy);
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let bad_header = AssetHashHeader {
            asset_id: [0x01; 32],
            hash: [0x02; 32],
            shard_count: 10,
            shard_index: 10, // >= shard_count
        };
        let packet = bad_header.to_bytes();
        let decision = mgr.validate_packet(0, &packet);
        match decision {
            PacketDecision::Drop { reason } => {
                assert!(reason.contains("Invalid shard indices"));
            }
            other => unreachable!("test: expected Drop, got {:?}", other),
        }
    }

    #[test]
    fn test_policy_routing_required_drops_short_packet() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut policy = ValidationPolicy::permissive();
        policy.check_matrix_routing = true;
        pm.set_default_policy(policy);
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let packet = vec![0u8; 5];
        let decision = mgr.validate_packet(0, &packet);
        match decision {
            PacketDecision::Drop { reason } => {
                assert!(reason.contains("too short for routing"));
            }
            other => unreachable!("test: expected Drop, got {:?}", other),
        }
    }

    #[test]
    fn test_policy_routing_required_passes_valid() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut policy = ValidationPolicy::permissive();
        policy.check_matrix_routing = true;
        pm.set_default_policy(policy);
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let packet = valid_routing_bytes();
        let decision = mgr.validate_packet(0, &packet);
        assert_eq!(decision, PacketDecision::Pass);
    }

    #[test]
    fn test_policy_all_flags_combined() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        pm.set_default_policy(ValidationPolicy::strict());
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        // Build combined packet: PoS (88) + AssetHash (72) + Routing (12+)
        let mut packet = valid_pos_bytes();
        packet.extend_from_slice(&valid_asset_hash_bytes());
        packet.extend_from_slice(&valid_routing_bytes());

        let decision = mgr.validate_packet(0, &packet);
        assert_eq!(decision, PacketDecision::Pass);
    }

    #[test]
    fn test_policy_all_flags_drops_when_pos_invalid() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        pm.set_default_policy(ValidationPolicy::strict());
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        // Build combined packet with bad PoS
        let mut bad_who = [0xFF; 32];
        bad_who[0] = 0x99;
        let bad_pos = ProofOfStateHeader {
            who: bad_who,
            what: valid_what(),
            when: now_micros(),
            where_: valid_where(),
        };
        let mut packet = bad_pos.to_bytes();
        packet.extend_from_slice(&valid_asset_hash_bytes());
        packet.extend_from_slice(&valid_routing_bytes());

        let decision = mgr.validate_packet(0, &packet);
        match decision {
            PacketDecision::Drop { reason } => {
                assert!(reason.contains("PoS validation failed"));
            }
            other => unreachable!("test: expected Drop, got {:?}", other),
        }
    }

    #[test]
    fn test_permissive_policy_skips_all_checks() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        pm.set_default_policy(ValidationPolicy::permissive());
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        // Small garbage packet passes with permissive policy
        let packet = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let decision = mgr.validate_packet(0, &packet);
        assert_eq!(decision, PacketDecision::Pass);
    }

    #[test]
    fn test_pos_offset_for_asset_hash_check() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut policy = ValidationPolicy::permissive();
        policy.requires_pos = true;
        policy.validate_asset_hash = true;
        pm.set_default_policy(policy);
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        // PoS header (88 bytes) + Asset hash header (72 bytes) = 160 bytes needed
        let mut packet = valid_pos_bytes();
        packet.extend_from_slice(&valid_asset_hash_bytes());
        let decision = mgr.validate_packet(0, &packet);
        assert_eq!(decision, PacketDecision::Pass);
    }

    // -------------------------------------------------------------------
    // Offload policy tests
    // -------------------------------------------------------------------

    #[test]
    fn test_offload_policy_default() {
        assert_eq!(OffloadPolicy::default(), OffloadPolicy::Disabled);
    }

    #[test]
    fn test_offload_policy_set() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut mgr = XdpManager::new(pm).expect("test: create xdp manager");
        assert_eq!(mgr.offload_policy, OffloadPolicy::Disabled);

        mgr.set_offload_policy(OffloadPolicy::Opportunistic);
        assert_eq!(mgr.offload_policy, OffloadPolicy::Opportunistic);

        mgr.set_offload_policy(OffloadPolicy::Required);
        assert_eq!(mgr.offload_policy, OffloadPolicy::Required);
    }

    #[test]
    fn test_attach_offload_falls_back_to_native() {
        // loopback does not support XDP offload, so Offload mode should
        // fall back to Native when policy is Disabled (default)
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let result = mgr.attach_with_mode("lo", XdpAttachMode::Offload);
        assert!(
            result.is_ok(),
            "test: offload on lo should fall back, not error"
        );

        // Verify attachment was tracked
        let attached = mgr.attached.read();
        assert!(attached.contains_key("lo"));
    }

    #[test]
    fn test_attach_offload_required_fails_on_unsupported_nic() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut mgr = XdpManager::new(pm).expect("test: create xdp manager");
        mgr.set_offload_policy(OffloadPolicy::Required);

        // loopback does not support XDP offload
        let result = mgr.attach_with_mode("lo", XdpAttachMode::Offload);
        assert!(result.is_err(), "test: required offload on lo should fail");

        let err_msg = result.expect_err("test: should be error").to_string();
        assert!(
            err_msg.contains("does not support XDP offload"),
            "test: error should mention offload unsupported, got: {err_msg}"
        );
    }

    #[test]
    fn test_attach_offload_opportunistic_falls_back() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut mgr = XdpManager::new(pm).expect("test: create xdp manager");
        mgr.set_offload_policy(OffloadPolicy::Opportunistic);

        // loopback does not support offload - should succeed with fallback
        let result = mgr.attach_with_mode("lo", XdpAttachMode::Offload);
        assert!(
            result.is_ok(),
            "test: opportunistic offload on lo should fall back"
        );
    }

    #[test]
    fn test_attach_native_mode_unaffected_by_offload_policy() {
        // Native and Generic modes should not be affected by offload policy
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut mgr = XdpManager::new(pm).expect("test: create xdp manager");
        mgr.set_offload_policy(OffloadPolicy::Required);

        let result = mgr.attach_with_mode("lo", XdpAttachMode::Native);
        assert!(result.is_ok(), "test: native mode should not check offload");

        let _ = mgr.detach("lo");

        let result = mgr.attach_with_mode("lo", XdpAttachMode::Generic);
        assert!(
            result.is_ok(),
            "test: generic mode should not check offload"
        );
    }

    // -------------------------------------------------------------------
    // KernelPosConfig tests
    // -------------------------------------------------------------------

    #[test]
    fn test_kernel_pos_config_default() {
        let cfg = KernelPosConfig::default();
        assert_eq!(cfg.min_difficulty, 8);
        assert_eq!(cfg.max_timestamp_skew_ns, 5 * 60 * 1_000_000_000);
        assert_eq!(cfg.validation_ttl_ns, 60 * 60 * 1_000_000_000);
        assert!(cfg.enabled);
    }

    #[test]
    fn test_set_kernel_pos_config_no_xdp() {
        // Calling set_kernel_pos_config without XDP attached should succeed
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let cfg = KernelPosConfig::default();
        let result = mgr.set_kernel_pos_config(&cfg);
        assert!(result.is_ok());
    }

    #[test]
    fn test_kernel_pos_config_serialization() {
        let cfg = KernelPosConfig {
            min_difficulty: 16,
            max_timestamp_skew_ns: 300_000_000_000, // 5 min in ns
            validation_ttl_ns: 3_600_000_000_000,   // 1 hour in ns
            enabled: true,
        };

        let bytes = cfg.to_bytes();
        assert_eq!(bytes.len(), 32);

        // Verify field layout — C `struct pos_config` NATURAL (non-packed) alignment:
        // u64 fields are 8-byte aligned (4B pad after min_difficulty, 4B trailing pad).
        let difficulty = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(difficulty, 16);

        let skew = u64::from_le_bytes([
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]);
        assert_eq!(skew, 300_000_000_000);

        let ttl = u64::from_le_bytes([
            bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
        ]);
        assert_eq!(ttl, 3_600_000_000_000);

        let enabled = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
        assert_eq!(enabled, 1);
    }

    #[test]
    fn test_kernel_pos_config_serialization_roundtrip() {
        let original = KernelPosConfig {
            min_difficulty: 24,
            max_timestamp_skew_ns: 42_000_000,
            validation_ttl_ns: 99_000_000_000,
            enabled: false,
        };

        let bytes = original.to_bytes();
        let decoded = KernelPosConfig::from_bytes(&bytes)
            .expect("test: from_bytes should succeed with 24 bytes");

        assert_eq!(decoded, original);
    }

    #[test]
    fn test_kernel_pos_config_from_bytes_too_short() {
        let short = [0u8; 23];
        assert!(KernelPosConfig::from_bytes(&short).is_none());
    }

    #[test]
    fn test_kernel_pos_config_disabled_serialization() {
        let cfg = KernelPosConfig {
            min_difficulty: 0,
            max_timestamp_skew_ns: 0,
            validation_ttl_ns: 0,
            enabled: false,
        };
        let bytes = cfg.to_bytes();
        // All bytes zero. 32 bytes = C `struct pos_config` NATURAL (non-packed)
        // layout — u64 fields are 8-byte aligned (see hypermesh_xdp.c).
        assert_eq!(bytes, [0u8; 32]);
    }

    #[test]
    fn test_kernel_pos_config_field_offsets_match_c_natural_layout() {
        // Guards the byte layout against the C `struct pos_config` (32 bytes,
        // natural alignment). A field at the wrong offset here = kernel reads
        // garbage = silent PoS bypass, so pin the exact offsets.
        let cfg = KernelPosConfig {
            min_difficulty: 0x1122_3344,
            max_timestamp_skew_ns: 0x0102_0304_0506_0708,
            validation_ttl_ns: 0x1112_1314_1516_1718,
            enabled: true,
        };
        let b = cfg.to_bytes();
        assert_eq!(&b[0..4], &0x1122_3344u32.to_le_bytes()); // min_difficulty @0
        assert_eq!(&b[4..8], &[0u8; 4]); // padding
        assert_eq!(&b[8..16], &0x0102_0304_0506_0708u64.to_le_bytes()); // skew @8
        assert_eq!(&b[16..24], &0x1112_1314_1516_1718u64.to_le_bytes()); // ttl @16
        assert_eq!(&b[24..28], &1u32.to_le_bytes()); // enabled @24
        assert_eq!(&b[28..32], &[0u8; 4]); // trailing padding
        // round-trip
        let rt = KernelPosConfig::from_bytes(&b).expect("test: from_bytes 32B");
        assert_eq!(rt.min_difficulty, cfg.min_difficulty);
        assert_eq!(rt.max_timestamp_skew_ns, cfg.max_timestamp_skew_ns);
        assert_eq!(rt.validation_ttl_ns, cfg.validation_ttl_ns);
        assert_eq!(rt.enabled, cfg.enabled);
    }
}
