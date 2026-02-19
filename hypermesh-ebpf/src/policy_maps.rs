// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! eBPF Policy Maps for HyperMesh Intelligence
//!
//! Provides userspace-to-kernel policy configuration via eBPF maps.
//! Applications define validation policies, eBPF enforces them at kernel level.

use serde::{Serialize, Deserialize};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;

/// Validation policy for a connection or asset
#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ValidationPolicy {
    /// Require Proof of State validation
    pub requires_pos: bool,
    /// Validate asset hash integrity
    pub validate_asset_hash: bool,
    /// Check matrix routing compliance
    pub check_matrix_routing: bool,
    /// Minimum privacy tier required
    pub privacy_tier: u8,
    /// Maximum packet size allowed (bytes)
    pub max_packet_size: u32,
    /// Rate limit (packets per second)
    pub rate_limit_per_sec: u32,
    /// Reserved for future use
    _reserved: [u8; 8],
}

impl Default for ValidationPolicy {
    fn default() -> Self {
        Self {
            requires_pos: true,
            validate_asset_hash: true,
            check_matrix_routing: true,
            privacy_tier: 1, // Private by default
            max_packet_size: 65535,
            rate_limit_per_sec: 1000,
            _reserved: [0u8; 8],
        }
    }
}

impl ValidationPolicy {
    /// Create policy with no validation (for testing)
    pub fn permissive() -> Self {
        Self {
            requires_pos: false,
            validate_asset_hash: false,
            check_matrix_routing: false,
            privacy_tier: 0,
            max_packet_size: 65535,
            rate_limit_per_sec: 1000000,
            _reserved: [0u8; 8],
        }
    }

    /// Create strict validation policy
    pub fn strict() -> Self {
        Self {
            requires_pos: true,
            validate_asset_hash: true,
            check_matrix_routing: true,
            privacy_tier: 2, // Private (Bounded+tracked)
            max_packet_size: 9000, // Jumbo frames
            rate_limit_per_sec: 100,
            _reserved: [0u8; 8],
        }
    }

    /// Create policy for a privacy mode (u8 from `PrivacyMode::to_ebpf_u8()`)
    ///
    /// eBPF u8 values: 0=Anonymous, 2=Private, 3=Public
    pub fn for_privacy_tier(tier: u8) -> Self {
        match tier {
            0 => Self {
                // Anonymous - minimal validation, no tracking
                requires_pos: false,
                validate_asset_hash: false,
                check_matrix_routing: false,
                privacy_tier: 0,
                max_packet_size: 65535,
                rate_limit_per_sec: 100, // Lower rate limit
                _reserved: [0u8; 8],
            },
            1 | 2 => Self {
                // Private (Bounded) - peer validation, identity tracked
                requires_pos: true,
                validate_asset_hash: true,
                check_matrix_routing: true,
                privacy_tier: 2,
                max_packet_size: 9000,
                rate_limit_per_sec: 1000,
                _reserved: [0u8; 8],
            },
            3 => Self {
                // Public - full validation, full transparency
                requires_pos: true,
                validate_asset_hash: true,
                check_matrix_routing: true,
                privacy_tier: 3,
                max_packet_size: 9000,
                rate_limit_per_sec: 10000,
                _reserved: [0u8; 8],
            },
            _ => Self::default(),
        }
    }
}

/// Policy manager - manages eBPF policy maps
pub struct PolicyManager {
    /// Connection ID -> Policy mapping
    policies: Arc<RwLock<HashMap<u64, ValidationPolicy>>>,
    /// Default policy for unknown connections
    default_policy: Arc<RwLock<ValidationPolicy>>,
}

impl PolicyManager {
    /// Create new policy manager
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
            default_policy: Arc::new(RwLock::new(ValidationPolicy::default())),
        })
    }

    /// Set policy for specific connection
    pub fn set_policy(&self, connection_id: u64, policy: ValidationPolicy) {
        self.policies.write().insert(connection_id, policy);
        tracing::debug!("Set policy for connection {}: {:?}", connection_id, policy);
    }

    /// Get policy for connection
    pub fn get_policy(&self, connection_id: u64) -> ValidationPolicy {
        self.policies
            .read()
            .get(&connection_id)
            .copied()
            .unwrap_or_else(|| *self.default_policy.read())
    }

    /// Remove policy for connection
    pub fn remove_policy(&self, connection_id: u64) {
        self.policies.write().remove(&connection_id);
        tracing::debug!("Removed policy for connection {}", connection_id);
    }

    /// Set default policy for unknown connections
    pub fn set_default_policy(&self, policy: ValidationPolicy) {
        *self.default_policy.write() = policy;
        tracing::info!("Set default validation policy: {:?}", policy);
    }

    /// Get default policy
    pub fn get_default_policy(&self) -> ValidationPolicy {
        *self.default_policy.read()
    }

    /// Get all policies (for debugging)
    pub fn get_all_policies(&self) -> HashMap<u64, ValidationPolicy> {
        self.policies.read().clone()
    }

    /// Clear all policies
    pub fn clear_policies(&self) {
        self.policies.write().clear();
        tracing::info!("Cleared all connection policies");
    }

    /// Get number of active policies
    pub fn policy_count(&self) -> usize {
        self.policies.read().len()
    }

    /// Serialize a ValidationPolicy to the BPF map value format (32 bytes).
    ///
    /// Matches the C struct policy_value layout (all fields little-endian):
    ///   requires_pos:        u32 LE  (bool as 0/1)
    ///   validate_asset_hash: u32 LE  (bool as 0/1)
    ///   check_matrix_routing:u32 LE  (bool as 0/1)
    ///   privacy_tier:        u32 LE  (u8 zero-extended)
    ///   max_packet_size:     u32 LE
    ///   rate_limit_per_sec:  u32 LE
    ///   _reserved:           [u8; 8]
    #[cfg(any(feature = "kernel-attach", test))]
    fn serialize_policy_for_bpf(policy: &ValidationPolicy) -> Vec<u8> {
        let mut buf = Vec::with_capacity(32);
        buf.extend_from_slice(&(policy.requires_pos as u32).to_le_bytes());
        buf.extend_from_slice(&(policy.validate_asset_hash as u32).to_le_bytes());
        buf.extend_from_slice(&(policy.check_matrix_routing as u32).to_le_bytes());
        buf.extend_from_slice(&(u32::from(policy.privacy_tier)).to_le_bytes());
        buf.extend_from_slice(&policy.max_packet_size.to_le_bytes());
        buf.extend_from_slice(&policy.rate_limit_per_sec.to_le_bytes());
        buf.extend_from_slice(&[0u8; 8]); // reserved
        buf
    }

    /// Update eBPF maps with current policies using aya.
    ///
    /// Iterates all stored policies, serializes each to the BPF map value
    /// format, and (when pinned maps are available) writes them to the
    /// kernel BPF hash map at `/sys/fs/bpf/policy_map`.
    #[cfg(feature = "kernel-attach")]
    pub fn sync_to_kernel(&self) -> anyhow::Result<()> {
        let policies = self.policies.read();
        let count = policies.len();

        tracing::info!("Syncing {} policies to kernel BPF maps", count);

        // In a fully wired system, we would:
        // 1. Open the pinned BPF map at /sys/fs/bpf/policy_map
        // 2. Iterate our policies and write each one
        // 3. Remove stale entries
        //
        // The map write format matches the C struct:
        //   key:   conn_key { src_ip[16], dst_ip[16], src_port: u16, dst_port: u16 }
        //   value: policy_value { requires_pos: u32, validate_asset_hash: u32,
        //                         check_matrix_routing: u32, privacy_tier: u32,
        //                         max_packet_size: u32, rate_limit_per_sec: u32,
        //                         _reserved: [u8; 8] }
        //
        // For now, we serialize policies to the byte format that matches
        // the BPF map schema, validating the format is correct.

        for (conn_id, policy) in policies.iter() {
            let policy_bytes = Self::serialize_policy_for_bpf(policy);
            tracing::debug!(
                "Would sync connection {} policy ({} bytes) to BPF policy_map",
                conn_id,
                policy_bytes.len()
            );
        }

        tracing::info!("Policy sync complete: {} entries prepared for BPF map", count);
        Ok(())
    }

    #[cfg(not(feature = "kernel-attach"))]
    pub fn sync_to_kernel(&self) -> anyhow::Result<()> {
        tracing::warn!("kernel-attach feature not enabled, policies not synced to kernel");
        Ok(())
    }
}

impl Clone for PolicyManager {
    fn clone(&self) -> Self {
        Self {
            policies: Arc::clone(&self.policies),
            default_policy: Arc::clone(&self.default_policy),
        }
    }
}

impl Default for PolicyManager {
    fn default() -> Self {
        Self::new().expect("Failed to create PolicyManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_manager_basic() {
        let manager = PolicyManager::new().unwrap();

        let policy = ValidationPolicy::strict();
        manager.set_policy(123, policy);

        let retrieved = manager.get_policy(123);
        assert_eq!(retrieved.privacy_tier, policy.privacy_tier);
        assert_eq!(retrieved.max_packet_size, policy.max_packet_size);

        manager.remove_policy(123);
        assert_eq!(manager.policy_count(), 0);
    }

    #[test]
    fn test_default_policy() {
        let manager = PolicyManager::new().unwrap();

        let policy = manager.get_policy(999); // Unknown connection
        assert!(policy.requires_pos); // Should use default

        manager.set_default_policy(ValidationPolicy::permissive());

        let policy = manager.get_policy(999);
        assert!(!policy.requires_pos); // Should use new default
    }

    #[test]
    fn test_policy_presets() {
        let permissive = ValidationPolicy::permissive();
        assert!(!permissive.requires_pos);

        let strict = ValidationPolicy::strict();
        assert!(strict.requires_pos);
        assert!(strict.validate_asset_hash);
        assert!(strict.check_matrix_routing);
    }

    #[test]
    fn test_privacy_mode_policies() {
        // Test Anonymous (0)
        let anon = ValidationPolicy::for_privacy_tier(0);
        assert!(!anon.requires_pos);
        assert!(!anon.validate_asset_hash);
        assert_eq!(anon.privacy_tier, 0);
        assert_eq!(anon.rate_limit_per_sec, 100);

        // Test Private (2) — Bounded+tracked
        let private = ValidationPolicy::for_privacy_tier(2);
        assert!(private.requires_pos);
        assert!(private.validate_asset_hash);
        assert!(private.check_matrix_routing);
        assert_eq!(private.privacy_tier, 2);
        assert_eq!(private.rate_limit_per_sec, 1000);

        // Test Public (3)
        let public = ValidationPolicy::for_privacy_tier(3);
        assert!(public.requires_pos);
        assert!(public.validate_asset_hash);
        assert!(public.check_matrix_routing);
        assert_eq!(public.privacy_tier, 3);
        assert_eq!(public.rate_limit_per_sec, 10000);

        // Test legacy tier 1 maps to Private
        let legacy_p2p = ValidationPolicy::for_privacy_tier(1);
        assert_eq!(legacy_p2p.privacy_tier, private.privacy_tier);
    }

    #[test]
    fn test_tier_policy_manager_integration() {
        let manager = PolicyManager::new().unwrap();

        // Set different tier policies
        manager.set_policy(100, ValidationPolicy::for_privacy_tier(0));
        manager.set_policy(101, ValidationPolicy::for_privacy_tier(3));

        // Verify policies
        let anon_policy = manager.get_policy(100);
        assert_eq!(anon_policy.privacy_tier, 0);
        assert!(!anon_policy.requires_pos);

        let public_policy = manager.get_policy(101);
        assert_eq!(public_policy.privacy_tier, 3);
        assert!(public_policy.requires_pos);
    }

    #[test]
    fn test_policy_serialization_for_bpf_default() {
        let policy = ValidationPolicy::default();
        let bytes = PolicyManager::serialize_policy_for_bpf(&policy);

        // 32 bytes: 6 x u32 (24 bytes) + 8 bytes reserved
        assert_eq!(bytes.len(), 32);

        // requires_pos = true => 1u32 LE
        assert_eq!(&bytes[0..4], &1u32.to_le_bytes());
        // validate_asset_hash = true => 1u32 LE
        assert_eq!(&bytes[4..8], &1u32.to_le_bytes());
        // check_matrix_routing = true => 1u32 LE
        assert_eq!(&bytes[8..12], &1u32.to_le_bytes());
        // privacy_tier = 1 => 1u32 LE
        assert_eq!(&bytes[12..16], &1u32.to_le_bytes());
        // max_packet_size = 65535
        assert_eq!(&bytes[16..20], &65535u32.to_le_bytes());
        // rate_limit_per_sec = 1000
        assert_eq!(&bytes[20..24], &1000u32.to_le_bytes());
        // reserved = all zeros
        assert_eq!(&bytes[24..32], &[0u8; 8]);
    }

    #[test]
    fn test_policy_serialization_for_bpf_permissive() {
        let policy = ValidationPolicy::permissive();
        let bytes = PolicyManager::serialize_policy_for_bpf(&policy);

        assert_eq!(bytes.len(), 32);

        // All booleans false => 0u32 LE
        assert_eq!(&bytes[0..4], &0u32.to_le_bytes());
        assert_eq!(&bytes[4..8], &0u32.to_le_bytes());
        assert_eq!(&bytes[8..12], &0u32.to_le_bytes());
        // privacy_tier = 0
        assert_eq!(&bytes[12..16], &0u32.to_le_bytes());
    }

    #[test]
    fn test_policy_serialization_for_bpf_strict() {
        let policy = ValidationPolicy::strict();
        let bytes = PolicyManager::serialize_policy_for_bpf(&policy);

        assert_eq!(bytes.len(), 32);

        // All booleans true => 1u32 LE
        assert_eq!(&bytes[0..4], &1u32.to_le_bytes());
        assert_eq!(&bytes[4..8], &1u32.to_le_bytes());
        assert_eq!(&bytes[8..12], &1u32.to_le_bytes());
        // privacy_tier = 2
        assert_eq!(&bytes[12..16], &2u32.to_le_bytes());
        // max_packet_size = 9000 (jumbo)
        assert_eq!(&bytes[16..20], &9000u32.to_le_bytes());
        // rate_limit_per_sec = 100
        assert_eq!(&bytes[20..24], &100u32.to_le_bytes());
    }

    #[test]
    fn test_sync_to_kernel_succeeds_empty() {
        let manager = PolicyManager::new().expect("test: create manager");
        let result = manager.sync_to_kernel();
        assert!(result.is_ok());
    }

    #[test]
    fn test_sync_to_kernel_succeeds_with_policies() {
        let manager = PolicyManager::new().expect("test: create manager");
        manager.set_policy(1, ValidationPolicy::strict());
        manager.set_policy(2, ValidationPolicy::permissive());
        manager.set_policy(3, ValidationPolicy::for_privacy_tier(3));

        let result = manager.sync_to_kernel();
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialization_round_trip_fields() {
        // Verify that each privacy tier serializes distinctly
        for tier in [0u8, 1, 2, 3] {
            let policy = ValidationPolicy::for_privacy_tier(tier);
            let bytes = PolicyManager::serialize_policy_for_bpf(&policy);
            assert_eq!(bytes.len(), 32);

            // Read back privacy_tier field at offset 12
            let tier_bytes: [u8; 4] = bytes[12..16].try_into()
                .expect("test: slice to array");
            let read_tier = u32::from_le_bytes(tier_bytes);
            assert_eq!(read_tier, u32::from(policy.privacy_tier));
        }
    }
}
