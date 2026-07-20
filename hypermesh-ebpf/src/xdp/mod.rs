// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Unified XDP (eXpress Data Path) Management
//!
//! Provides kernel-level packet classification, filtering, and routing
//! for the HyperMesh node. Combines XDP program attachment/management with
//! the PoS-authenticated-peer allowlist datapath.
//!
//! This is THE single XDP manager for the entire HyperMesh stack.
//! STOQ and blockmatrix are consumers via the `HyperMeshEbpf` orchestrator.

mod manager;
mod types;
mod validation;

pub use manager::*;
pub use types::*;
// The `*_to_bytes` serializers are consumed directly by `manager.rs` (via
// `super::validation::...`) under kernel-attach and by the tests below; no
// crate-level re-export needed.
#[cfg(test)]
use validation::policy_to_bytes;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy_maps::{PolicyManager, ValidationPolicy};

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
    fn test_sync_policies_to_bpf_no_kernel() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        pm.set_policy(1, ValidationPolicy::strict());
        pm.set_policy(2, ValidationPolicy::permissive());
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        // Without kernel-attach, this should succeed as a no-op
        assert!(mgr.sync_policies_to_bpf().is_ok());
    }

    // -------------------------------------------------------------------
    // Kernel map serializer tests (allowlist wire format)
    // -------------------------------------------------------------------

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
            max_timestamp_skew_ns: 300_000_000_000, // 5 min in ns
            validation_ttl_ns: 3_600_000_000_000,   // 1 hour in ns
            enabled: true,
        };

        let bytes = cfg.to_bytes();
        assert_eq!(bytes.len(), 24);

        // Verify field layout — C `struct pos_config` NATURAL (non-packed)
        // alignment: u64 fields are 8-byte aligned, 4B trailing pad.
        let skew = u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        assert_eq!(skew, 300_000_000_000);

        let ttl = u64::from_le_bytes([
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]);
        assert_eq!(ttl, 3_600_000_000_000);

        let enabled = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        assert_eq!(enabled, 1);
    }

    #[test]
    fn test_kernel_pos_config_serialization_roundtrip() {
        let original = KernelPosConfig {
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
            max_timestamp_skew_ns: 0,
            validation_ttl_ns: 0,
            enabled: false,
        };
        let bytes = cfg.to_bytes();
        // All bytes zero. 24 bytes = C `struct pos_config` NATURAL (non-packed)
        // layout — u64 fields are 8-byte aligned (see hypermesh_xdp.c).
        assert_eq!(bytes, [0u8; 24]);
    }

    #[test]
    fn test_kernel_pos_config_carries_no_difficulty_field() {
        // Regression guard for the PoW mining-difficulty removal: the kernel
        // PoS config is exactly 24 bytes of skew + TTL + enabled. If a
        // difficulty word were ever re-added, the struct would grow to 32 and
        // this assertion would fail.
        assert_eq!(KernelPosConfig::SIZE, 24);
        assert_eq!(KernelPosConfig::default().to_bytes().len(), 24);
    }

    #[test]
    fn test_kernel_pos_config_field_offsets_match_c_natural_layout() {
        // Guards the byte layout against the C `struct pos_config` (24 bytes,
        // natural alignment). A field at the wrong offset here = kernel reads
        // garbage = silent PoS bypass, so pin the exact offsets.
        let cfg = KernelPosConfig {
            max_timestamp_skew_ns: 0x0102_0304_0506_0708,
            validation_ttl_ns: 0x1112_1314_1516_1718,
            enabled: true,
        };
        let b = cfg.to_bytes();
        assert_eq!(&b[0..8], &0x0102_0304_0506_0708u64.to_le_bytes()); // skew @0
        assert_eq!(&b[8..16], &0x1112_1314_1516_1718u64.to_le_bytes()); // ttl @8
        assert_eq!(&b[16..20], &1u32.to_le_bytes()); // enabled @16
        assert_eq!(&b[20..24], &[0u8; 4]); // trailing padding
        // round-trip
        let rt = KernelPosConfig::from_bytes(&b).expect("test: from_bytes 24B");
        assert_eq!(rt.max_timestamp_skew_ns, cfg.max_timestamp_skew_ns);
        assert_eq!(rt.validation_ttl_ns, cfg.validation_ttl_ns);
        assert_eq!(rt.enabled, cfg.enabled);
    }
}
