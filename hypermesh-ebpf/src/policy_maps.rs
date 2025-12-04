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
            privacy_tier: 2, // Federated
            max_packet_size: 9000, // Jumbo frames
            rate_limit_per_sec: 100,
            _reserved: [0u8; 8],
        }
    }

    /// Create policy for specific privacy tier
    pub fn for_privacy_tier(tier: u8) -> Self {
        match tier {
            0 => Self {
                // Anonymous tier - minimal validation
                requires_pos: false,
                validate_asset_hash: false,
                check_matrix_routing: false,
                privacy_tier: 0,
                max_packet_size: 65535,
                rate_limit_per_sec: 100, // Lower rate limit
                _reserved: [0u8; 8],
            },
            1 => Self {
                // Private P2P tier - peer validation only
                requires_pos: false,
                validate_asset_hash: true,
                check_matrix_routing: false,
                privacy_tier: 1,
                max_packet_size: 65535,
                rate_limit_per_sec: 500,
                _reserved: [0u8; 8],
            },
            2 => Self {
                // Federated tier - cross-network validation
                requires_pos: true,
                validate_asset_hash: true,
                check_matrix_routing: true,
                privacy_tier: 2,
                max_packet_size: 9000,
                rate_limit_per_sec: 1000,
                _reserved: [0u8; 8],
            },
            3 => Self {
                // Public tier - full validation
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

    /// Update eBPF maps with current policies
    #[cfg(feature = "kernel-attach")]
    pub fn sync_to_kernel(&self) -> anyhow::Result<()> {
        // In production, this would update actual eBPF maps
        // For now, this is a placeholder
        tracing::debug!("Syncing {} policies to kernel", self.policy_count());
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
    fn test_privacy_tier_policies() {
        // Test Anonymous tier (0)
        let anon = ValidationPolicy::for_privacy_tier(0);
        assert!(!anon.requires_pos);
        assert!(!anon.validate_asset_hash);
        assert_eq!(anon.privacy_tier, 0);
        assert_eq!(anon.rate_limit_per_sec, 100);

        // Test Private P2P tier (1)
        let p2p = ValidationPolicy::for_privacy_tier(1);
        assert!(!p2p.requires_pos);
        assert!(p2p.validate_asset_hash);
        assert_eq!(p2p.privacy_tier, 1);
        assert_eq!(p2p.rate_limit_per_sec, 500);

        // Test Federated tier (2)
        let federated = ValidationPolicy::for_privacy_tier(2);
        assert!(federated.requires_pos);
        assert!(federated.validate_asset_hash);
        assert!(federated.check_matrix_routing);
        assert_eq!(federated.privacy_tier, 2);
        assert_eq!(federated.rate_limit_per_sec, 1000);

        // Test Public tier (3)
        let public = ValidationPolicy::for_privacy_tier(3);
        assert!(public.requires_pos);
        assert!(public.validate_asset_hash);
        assert!(public.check_matrix_routing);
        assert_eq!(public.privacy_tier, 3);
        assert_eq!(public.rate_limit_per_sec, 10000);
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
}
