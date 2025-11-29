//! Privacy-aware allocation system
//!
//! User-configurable privacy levels for resource sharing with
//! appropriate access controls and economic incentives.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};

use super::AssetId;
use super::status::AssetStatus;

/// Privacy levels for asset sharing (from Proof of State patterns)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivacyLevel {
    /// Internal network only, no external access
    Private,
    /// Specific networks/groups only
    PrivateNetwork,
    /// Trusted peer sharing
    P2P,
    /// Specific public networks
    PublicNetwork,
    /// Maximum CAESAR rewards, full HyperMesh node/"pivot"
    FullPublic,
}

impl PrivacyLevel {
    /// Get privacy level priority (lower is more private)
    pub fn priority(&self) -> u8 {
        match self {
            PrivacyLevel::Private => 0,
            PrivacyLevel::PrivateNetwork => 1,
            PrivacyLevel::P2P => 2,
            PrivacyLevel::PublicNetwork => 3,
            PrivacyLevel::FullPublic => 4,
        }
    }
    
    /// Check if this privacy level allows access from another level
    pub fn allows_access_from(&self, requester_level: &PrivacyLevel) -> bool {
        match self {
            PrivacyLevel::Private => false, // No external access
            PrivacyLevel::PrivateNetwork => {
                matches!(requester_level, PrivacyLevel::PrivateNetwork)
            },
            PrivacyLevel::P2P => {
                matches!(requester_level, 
                    PrivacyLevel::PrivateNetwork | PrivacyLevel::P2P)
            },
            PrivacyLevel::PublicNetwork => {
                !matches!(requester_level, PrivacyLevel::Private)
            },
            PrivacyLevel::FullPublic => true, // Maximum accessibility
        }
    }
    
    /// Get expected CAESAR token reward multiplier
    pub fn caesar_reward_multiplier(&self) -> f32 {
        match self {
            PrivacyLevel::Private => 0.0, // No rewards for private resources
            PrivacyLevel::PrivateNetwork => 0.25,
            PrivacyLevel::P2P => 0.5,
            PrivacyLevel::PublicNetwork => 0.75,
            PrivacyLevel::FullPublic => 1.0, // Maximum rewards
        }
    }
    
    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            PrivacyLevel::Private => "Internal use only, no sharing",
            PrivacyLevel::PrivateNetwork => "Shared within trusted network groups",
            PrivacyLevel::P2P => "Shared with verified peers",
            PrivacyLevel::PublicNetwork => "Available on public networks",
            PrivacyLevel::FullPublic => "Fully public with maximum rewards",
        }
    }
    
    /// Check if privacy level supports specific features
    pub fn supports_remote_access(&self) -> bool {
        !matches!(self, PrivacyLevel::Private)
    }
    
    pub fn supports_proxy_addressing(&self) -> bool {
        matches!(self, 
            PrivacyLevel::P2P | 
            PrivacyLevel::PublicNetwork | 
            PrivacyLevel::FullPublic
        )
    }
}

impl std::fmt::Display for PrivacyLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrivacyLevel::Private => write!(f, "Private"),
            PrivacyLevel::PrivateNetwork => write!(f, "Private Network"),
            PrivacyLevel::P2P => write!(f, "P2P"),
            PrivacyLevel::PublicNetwork => write!(f, "Public Network"),
            PrivacyLevel::FullPublic => write!(f, "Full Public"),
        }
    }
}

/// Asset allocation result with privacy enforcement
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetAllocation {
    /// Allocated asset identifier
    pub asset_id: AssetId,
    /// Current asset status
    pub status: AssetStatus,
    /// Allocation configuration
    pub allocation_config: AllocationConfig,
    /// Access control configuration
    pub access_config: AccessConfig,
    /// Allocation timestamp
    pub allocated_at: SystemTime,
    /// Allocation expiry (if configured)
    pub expires_at: Option<SystemTime>,
}

/// Allocation configuration settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AllocationConfig {
    /// Privacy level for this allocation
    pub privacy_level: PrivacyLevel,
    /// Resource allocation percentages (0-100% per resource type)
    pub resource_allocation: ResourceAllocationConfig,
    /// Concurrent usage limits
    pub concurrency_limits: ConcurrencyLimits,
    /// Duration-based allocation settings
    pub duration_config: DurationConfig,
    /// Consensus requirements for access
    pub consensus_requirements: ConsensusRequirements,
}

/// Resource allocation percentages
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceAllocationConfig {
    /// CPU allocation percentage (0.0 - 1.0)
    pub cpu_allocation: f32,
    /// GPU allocation percentage (0.0 - 1.0)
    pub gpu_allocation: f32,
    /// Memory allocation percentage (0.0 - 1.0)
    pub memory_allocation: f32,
    /// Storage allocation percentage (0.0 - 1.0)
    pub storage_allocation: f32,
    /// Network bandwidth allocation percentage (0.0 - 1.0)
    pub network_allocation: f32,
}

/// Concurrent usage limitations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConcurrencyLimits {
    /// Maximum concurrent users
    pub max_users: u32,
    /// Maximum concurrent processes
    pub max_processes: u32,
    /// Maximum concurrent connections
    pub max_connections: u32,
    /// Maximum queue length for pending requests
    pub max_queue_length: u32,
}

/// Duration-based allocation settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DurationConfig {
    /// Maximum allocation duration
    pub max_duration: Option<Duration>,
    /// Minimum allocation duration
    pub min_duration: Option<Duration>,
    /// Automatic renewal policy
    pub auto_renewal: bool,
    /// Grace period before forced deallocation
    pub grace_period: Duration,
}

/// Consensus requirements for access
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusRequirements {
    /// Require Proof of Space
    pub require_space_proof: bool,
    /// Require Proof of Stake
    pub require_stake_proof: bool,
    /// Require Proof of Work
    pub require_work_proof: bool,
    /// Require Proof of Time
    pub require_time_proof: bool,
    /// Minimum stake amount required
    pub minimum_stake: u64,
    /// Maximum time offset allowed
    pub max_time_offset: Duration,
}

/// Access control configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessConfig {
    /// Allowed certificate fingerprints
    pub allowed_certificates: Vec<String>,
    /// Allowed network groups
    pub allowed_networks: Vec<String>,
    /// Access permissions
    pub permissions: AccessPermissions,
    /// Rate limiting configuration
    pub rate_limits: RateLimits,
    /// Authentication requirements
    pub auth_requirements: AuthRequirements,
}

/// Access permissions granularity
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessPermissions {
    /// Can read resource status
    pub can_read: bool,
    /// Can execute operations
    pub can_execute: bool,
    /// Can modify configuration
    pub can_configure: bool,
    /// Can monitor performance
    pub can_monitor: bool,
    /// Can share with others
    pub can_share: bool,
}

/// Rate limiting configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimits {
    /// Requests per second limit
    pub requests_per_second: u32,
    /// Data transfer rate limit in MB/s
    pub bandwidth_mbps: u32,
    /// CPU usage rate limit (0.0 - 1.0)
    pub cpu_usage_limit: f32,
    /// Memory usage rate limit in bytes
    pub memory_usage_limit: u64,
}

/// Authentication requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthRequirements {
    /// Require TLS certificate validation
    pub require_certificate: bool,
    /// Require multi-factor authentication
    pub require_mfa: bool,
    /// Require consensus proof validation
    pub require_consensus_proof: bool,
    /// Session timeout in seconds
    pub session_timeout: u32,
}

/// User privacy preferences configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserPrivacyConfig {
    /// User's default privacy level
    pub default_privacy_level: PrivacyLevel,
    /// Per-resource type privacy settings
    pub resource_privacy: HashMap<String, PrivacyLevel>,
    /// CAESAR token earning preferences
    pub caesar_preferences: CaesarPreferences,
    /// Remote proxy preferences
    pub proxy_preferences: ProxyPreferences,
}

/// CAESAR token earning configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaesarPreferences {
    /// Enable CAESAR token earning
    pub enabled: bool,
    /// Minimum reward rate threshold
    pub min_reward_rate: f32,
    /// Preferred reward token types
    pub preferred_tokens: Vec<String>,
    /// Auto-stake earned tokens
    pub auto_stake: bool,
}

/// Remote proxy addressing preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxyPreferences {
    /// Enable remote proxy addressing
    pub enabled: bool,
    /// Preferred proxy node characteristics
    pub preferred_proxy_nodes: Vec<String>,
    /// NAT-like addressing preferences
    pub nat_preferences: NatPreferences,
    /// Trust-based proxy selection
    pub trust_based_selection: bool,
}

/// NAT-like addressing preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NatPreferences {
    /// Preferred network address ranges
    pub preferred_networks: Vec<String>,
    /// Port range preferences
    pub port_ranges: Vec<PortRange>,
    /// IPv6 preference over IPv4
    pub prefer_ipv6: bool,
}

/// Port range specification
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortRange {
    /// Start port (inclusive)
    pub start: u16,
    /// End port (inclusive)
    pub end: u16,
}

impl Default for ResourceAllocationConfig {
    fn default() -> Self {
        Self {
            cpu_allocation: 1.0,
            gpu_allocation: 1.0,
            memory_allocation: 1.0,
            storage_allocation: 1.0,
            network_allocation: 1.0,
        }
    }
}

impl Default for ConcurrencyLimits {
    fn default() -> Self {
        Self {
            max_users: 10,
            max_processes: 100,
            max_connections: 1000,
            max_queue_length: 50,
        }
    }
}

impl Default for DurationConfig {
    fn default() -> Self {
        Self {
            max_duration: Some(Duration::from_secs(24 * 60 * 60)), // 24 hours default
            min_duration: Some(Duration::from_secs(5 * 60)), // 5 minutes minimum
            auto_renewal: false,
            grace_period: Duration::from_secs(5 * 60), // 5 minutes
        }
    }
}

impl Default for ConsensusRequirements {
    fn default() -> Self {
        Self {
            require_space_proof: true,
            require_stake_proof: true,
            require_work_proof: true,
            require_time_proof: true,
            minimum_stake: 1000,
            max_time_offset: Duration::from_secs(30),
        }
    }
}

impl Default for AccessPermissions {
    fn default() -> Self {
        Self {
            can_read: true,
            can_execute: true,
            can_configure: false,
            can_monitor: true,
            can_share: false,
        }
    }
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            requests_per_second: 100,
            bandwidth_mbps: 100,
            cpu_usage_limit: 0.8,
            memory_usage_limit: 1024 * 1024 * 1024, // 1GB
        }
    }
}

impl Default for AuthRequirements {
    fn default() -> Self {
        Self {
            require_certificate: true,
            require_mfa: false,
            require_consensus_proof: true,
            session_timeout: 3600, // 1 hour
        }
    }
}

impl AssetAllocation {
    /// Create new asset allocation
    pub fn new(
        asset_id: AssetId,
        status: AssetStatus,
        privacy_level: PrivacyLevel,
    ) -> Self {
        let allocation_config = AllocationConfig {
            privacy_level: privacy_level.clone(),
            resource_allocation: ResourceAllocationConfig::default(),
            concurrency_limits: ConcurrencyLimits::default(),
            duration_config: DurationConfig::default(),
            consensus_requirements: ConsensusRequirements::default(),
        };
        
        let access_config = AccessConfig {
            allowed_certificates: vec![status.owner_certificate_fingerprint.clone()],
            allowed_networks: Vec::new(),
            permissions: AccessPermissions::default(),
            rate_limits: RateLimits::default(),
            auth_requirements: AuthRequirements::default(),
        };
        
        Self {
            asset_id,
            status,
            allocation_config,
            access_config,
            allocated_at: SystemTime::now(),
            expires_at: None,
        }
    }
    
    /// Check if allocation has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            SystemTime::now() >= expires_at
        } else {
            false
        }
    }
    
    /// Set allocation expiry
    pub fn set_expiry(&mut self, duration: Duration) {
        self.expires_at = Some(self.allocated_at + duration);
    }
    
    /// Check if access is allowed for a certificate
    pub fn allows_access(&self, certificate_fingerprint: &str) -> bool {
        self.access_config.allowed_certificates.contains(&certificate_fingerprint.to_string())
    }
    
    /// Add allowed certificate
    pub fn add_allowed_certificate(&mut self, certificate_fingerprint: String) {
        if !self.access_config.allowed_certificates.contains(&certificate_fingerprint) {
            self.access_config.allowed_certificates.push(certificate_fingerprint);
        }
    }
    
    /// Get remaining allocation time
    pub fn remaining_time(&self) -> Option<Duration> {
        if let Some(expires_at) = self.expires_at {
            expires_at.duration_since(SystemTime::now()).ok()
        } else {
            None
        }
    }
    
    /// Calculate CAESAR token reward rate
    pub fn caesar_reward_rate(&self) -> f32 {
        let base_rate = self.allocation_config.privacy_level.caesar_reward_multiplier();
        
        // Adjust based on resource allocation
        let resource_factor = (
            self.allocation_config.resource_allocation.cpu_allocation +
            self.allocation_config.resource_allocation.gpu_allocation +
            self.allocation_config.resource_allocation.memory_allocation +
            self.allocation_config.resource_allocation.storage_allocation +
            self.allocation_config.resource_allocation.network_allocation
        ) / 5.0;
        
        base_rate * resource_factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{AssetId, AssetType};
    use super::status::AssetStatus;
    
    #[test]
    fn test_privacy_level_access_control() {
        assert!(!PrivacyLevel::Private.allows_access_from(&PrivacyLevel::FullPublic));
        assert!(PrivacyLevel::FullPublic.allows_access_from(&PrivacyLevel::Private));
        assert!(PrivacyLevel::P2P.allows_access_from(&PrivacyLevel::PrivateNetwork));
    }
    
    #[test]
    fn test_privacy_level_rewards() {
        assert_eq!(PrivacyLevel::Private.caesar_reward_multiplier(), 0.0);
        assert_eq!(PrivacyLevel::FullPublic.caesar_reward_multiplier(), 1.0);
        assert!(PrivacyLevel::P2P.caesar_reward_multiplier() > 0.0);
    }
    
    #[test]
    fn test_asset_allocation_creation() {
        let asset_id = AssetId::new(AssetType::Cpu);
        let status = AssetStatus::new(
            asset_id.clone(),
            "test-cert".to_string(),
            PrivacyLevel::P2P,
        );
        
        let allocation = AssetAllocation::new(asset_id.clone(), status, PrivacyLevel::P2P);
        
        assert_eq!(allocation.asset_id, asset_id);
        assert_eq!(allocation.allocation_config.privacy_level, PrivacyLevel::P2P);
        assert!(!allocation.is_expired());
    }
    
    #[test]
    fn test_allocation_expiry() {
        let asset_id = AssetId::new(AssetType::Memory);
        let status = AssetStatus::new(
            asset_id.clone(),
            "test-cert".to_string(),
            PrivacyLevel::Private,
        );
        
        let mut allocation = AssetAllocation::new(asset_id, status, PrivacyLevel::Private);
        allocation.set_expiry(Duration::from_secs(1));
        
        // Should not be expired immediately
        assert!(!allocation.is_expired());
        
        // Should have remaining time
        assert!(allocation.remaining_time().is_some());
    }
    
    #[test]
    fn test_caesar_reward_calculation() {
        let asset_id = AssetId::new(AssetType::Storage);
        let status = AssetStatus::new(
            asset_id.clone(),
            "test-cert".to_string(),
            PrivacyLevel::FullPublic,
        );
        
        let allocation = AssetAllocation::new(asset_id, status, PrivacyLevel::FullPublic);
        let reward_rate = allocation.caesar_reward_rate();
        
        // Full public with full resource allocation should give maximum rate
        assert_eq!(reward_rate, 1.0);
    }
}