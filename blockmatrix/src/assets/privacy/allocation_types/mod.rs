// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Privacy Allocation Types - Based on Proof of State patterns
//!
//! Implements the four allocation types from Proof of State with enhanced
//! constraints and transition validation.

pub mod performance;
pub mod security;
pub mod transitions;

pub use performance::*;
pub use security::*;
pub use transitions::*;

use crate::assets::core::PrivacyMode;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Privacy allocation types from Proof of State patterns
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivacyAllocationType {
    /// Internal use only, no sharing, no external access
    Private,
    /// Cross-network accessible, full discovery
    Public,
    /// No identity tracking, privacy-first sharing
    Anonymous,
    /// Full state proof validation required (PoSp+PoSt+PoWk+PoTm)
    Verified,
}

impl PrivacyAllocationType {
    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            PrivacyAllocationType::Private => "Internal use only, no external access or sharing",
            PrivacyAllocationType::Public => {
                "Cross-network accessible with full discovery capabilities"
            }
            PrivacyAllocationType::Anonymous => "Privacy-first sharing with no identity tracking",
            PrivacyAllocationType::Verified => {
                "Maximum security with full state proof validation"
            }
        }
    }

    /// Check if allocation type requires state proof
    pub fn requires_state_proof(&self) -> bool {
        matches!(self, PrivacyAllocationType::Verified)
    }

    /// Check if allocation type supports remote access
    pub fn supports_remote_access(&self) -> bool {
        !matches!(self, PrivacyAllocationType::Private)
    }

    /// Check if allocation type supports identity tracking
    pub fn supports_identity_tracking(&self) -> bool {
        !matches!(self, PrivacyAllocationType::Anonymous)
    }

    /// Check if allocation type supports public discovery
    pub fn supports_public_discovery(&self) -> bool {
        matches!(
            self,
            PrivacyAllocationType::Public | PrivacyAllocationType::Verified
        )
    }

    /// Get minimum required privacy level
    pub fn minimum_privacy_level(&self) -> PrivacyMode {
        match self {
            PrivacyAllocationType::Private => PrivacyMode::PRIVATE,
            PrivacyAllocationType::Public => PrivacyMode::PUBLIC,
            PrivacyAllocationType::Anonymous => PrivacyMode::PRIVATE,
            PrivacyAllocationType::Verified => PrivacyMode::PUBLIC,
        }
    }

    /// Get maximum allowed privacy level
    pub fn maximum_privacy_level(&self) -> PrivacyMode {
        match self {
            PrivacyAllocationType::Private => PrivacyMode::PRIVATE,
            PrivacyAllocationType::Public => PrivacyMode::PUBLIC,
            PrivacyAllocationType::Anonymous => PrivacyMode::PUBLIC,
            PrivacyAllocationType::Verified => PrivacyMode::PUBLIC,
        }
    }

    /// Get base CAESAR reward multiplier for resource allocation.
    ///
    /// Distinct from [`PrivacyMode::caesar_multiplier()`] which rewards identity transparency
    /// (Public=1.0, Anonymous=0.0). This multiplier rewards resource-sharing generosity:
    /// Anonymous shares most freely (0.5), Private shares nothing (0.0), and Verified (1.0)
    /// is unique to allocation -- requiring identity verification for maximum rewards.
    pub fn base_reward_multiplier(&self) -> f32 {
        match self {
            PrivacyAllocationType::Private => 0.0, // No rewards for private allocation
            PrivacyAllocationType::Public => 0.75,
            PrivacyAllocationType::Anonymous => 0.5, // Lower rewards for anonymous
            PrivacyAllocationType::Verified => 1.0,  // Maximum rewards
        }
    }

    /// Check if transition to another allocation type is allowed
    pub fn can_transition_to(&self, target: &PrivacyAllocationType) -> bool {
        match (self, target) {
            // Private can only transition to public types
            (PrivacyAllocationType::Private, PrivacyAllocationType::Private) => true,
            (PrivacyAllocationType::Private, PrivacyAllocationType::Public) => true,
            (PrivacyAllocationType::Private, PrivacyAllocationType::Anonymous) => false,
            (PrivacyAllocationType::Private, PrivacyAllocationType::Verified) => true,

            // Public can transition to any type
            (PrivacyAllocationType::Public, _) => true,

            // Anonymous can transition to anonymous, private, or verified
            (PrivacyAllocationType::Anonymous, PrivacyAllocationType::Anonymous) => true,
            (PrivacyAllocationType::Anonymous, PrivacyAllocationType::Private) => true,
            (PrivacyAllocationType::Anonymous, PrivacyAllocationType::Public) => false, // No identity->public
            (PrivacyAllocationType::Anonymous, PrivacyAllocationType::Verified) => true,

            // Verified can transition to any type
            (PrivacyAllocationType::Verified, _) => true,
        }
    }

    /// Get required capabilities for this allocation type
    pub fn required_capabilities(&self) -> Vec<String> {
        match self {
            PrivacyAllocationType::Private => {
                vec!["local_access".to_string(), "memory_isolation".to_string()]
            }
            PrivacyAllocationType::Public => vec![
                "remote_access".to_string(),
                "public_discovery".to_string(),
                "load_balancing".to_string(),
            ],
            PrivacyAllocationType::Anonymous => vec![
                "anonymous_routing".to_string(),
                "identity_masking".to_string(),
                "encrypted_communication".to_string(),
            ],
            PrivacyAllocationType::Verified => vec![
                "state_validation".to_string(),
                "proof_verification".to_string(),
                "quantum_security".to_string(),
                "trust_scoring".to_string(),
            ],
        }
    }
}

/// Configuration for allocation type behavior
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AllocationTypeConfig {
    /// Allocation type
    pub allocation_type: PrivacyAllocationType,

    /// Type-specific constraints
    pub constraints: AllocationTypeConstraints,

    /// Security requirements
    pub security_requirements: SecurityRequirements,

    /// Performance characteristics
    pub performance_characteristics: PerformanceCharacteristics,

    /// Integration settings
    pub integration_settings: IntegrationSettings,
}

/// Constraints specific to allocation types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AllocationTypeConstraints {
    /// Maximum allocation duration for this type
    pub max_duration: Option<Duration>,

    /// Maximum concurrent allocations of this type per user
    pub max_concurrent_per_user: u32,

    /// Maximum resource allocation percentage
    pub max_resource_allocation: f32,

    /// Required minimum stake for this type
    pub required_minimum_stake: u64,

    /// Access restrictions
    pub access_restrictions: AccessRestrictions,

    /// Network restrictions
    pub network_restrictions: NetworkRestrictions,
}

/// Access restrictions for allocation types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessRestrictions {
    /// Allowed access patterns
    pub allowed_access_patterns: Vec<AccessPattern>,

    /// Forbidden operations
    pub forbidden_operations: Vec<String>,

    /// Time-based restrictions
    pub time_restrictions: Vec<TimeRestriction>,

    /// Geographic restrictions
    pub geographic_restrictions: Vec<String>,
}

/// Access patterns allowed for allocation types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AccessPattern {
    LocalOnly,
    NetworkLocal,
    P2PDirect,
    ProxyRouted,
    PublicAccess,
    VerifiedAccess,
}

/// Time-based access restrictions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeRestriction {
    /// Days of week (0=Sunday, 6=Saturday)
    pub allowed_days: Vec<u8>,

    /// Hours of day (0-23)
    pub allowed_hours: Vec<u8>,

    /// Maximum duration per time window
    pub max_duration_per_window: Duration,

    /// Time window size
    pub time_window: Duration,
}

/// Network restrictions for allocation types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkRestrictions {
    /// Allowed network ranges (CIDR notation)
    pub allowed_networks: Vec<String>,

    /// Blocked network ranges
    pub blocked_networks: Vec<String>,

    /// VPN/Proxy policies
    pub vpn_proxy_policy: VpnProxyPolicy,

    /// Tor network policy
    pub tor_policy: TorPolicy,
}

/// VPN/Proxy access policies
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VpnProxyPolicy {
    Allowed,
    Blocked,
    WhitelistOnly,
    RequiredForAccess,
}

/// Tor network access policies
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TorPolicy {
    Allowed,
    Blocked,
    AnonymousOnly, // Only for Anonymous allocation type
    VerifiedOnly,  // Only with additional verification
}

/// Default implementations
impl Default for AllocationTypeConstraints {
    fn default() -> Self {
        Self {
            max_duration: Some(Duration::from_secs(24 * 60 * 60)), // 24 hours
            max_concurrent_per_user: 10,
            max_resource_allocation: 1.0, // 100%
            required_minimum_stake: 0,
            access_restrictions: AccessRestrictions::default(),
            network_restrictions: NetworkRestrictions::default(),
        }
    }
}

impl Default for AccessRestrictions {
    fn default() -> Self {
        Self {
            allowed_access_patterns: vec![AccessPattern::LocalOnly],
            forbidden_operations: Vec::new(),
            time_restrictions: Vec::new(),
            geographic_restrictions: Vec::new(),
        }
    }
}

impl Default for NetworkRestrictions {
    fn default() -> Self {
        Self {
            allowed_networks: Vec::new(),
            blocked_networks: Vec::new(),
            vpn_proxy_policy: VpnProxyPolicy::Allowed,
            tor_policy: TorPolicy::Allowed,
        }
    }
}
