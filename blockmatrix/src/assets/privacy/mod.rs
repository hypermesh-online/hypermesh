// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Privacy Management System for HyperMesh Assets
//!
//! User-configurable privacy levels and resource sharing controls with
//! integration to consensus system, remote proxy addressing, and CAESAR rewards.
//!
//! Based on Proof of State patterns with privacy allocation types:
//! - Private: Internal network only, no external access
//! - Public: Cross-network accessible, full discovery
//! - Anonymous: No identity tracking, privacy-first sharing
//! - Verified: Full consensus validation required (PoSp+PoSt+PoWk+PoTm)

pub mod advanced_config;
pub mod allocation_types;
pub mod config;
pub mod core;
pub mod enforcement;
pub mod keys;
pub mod manager;
pub mod retention;
pub mod rewards;

pub use manager::{
    CaesarRewardPreferences, PrivacyManager, PrivacyManagerConfig, RewardOptimizationPreferences,
};
// Import from modular config system
pub use allocation_types::{
    AllocationTypeConfig, AllocationTypeConstraints, PrivacyAllocationType, PrivacyTransition,
    TransitionValidation,
};
pub use config::{
    AdvancedPrivacyOptions, PrivacyConstraints, PrivacyPreset, PrivacySettings, PrivacyTemplate,
    PrivacyValidationRules, ResourcePrivacySettings, UserPrivacyConfig,
};
pub use enforcement::{
    AccessControlResult, EnforcementAction, PrivacyAuditLog, PrivacyEnforcer, PrivacyViolation,
};
pub use rewards::{CaesarRewardCalculator, RewardConfiguration, RewardTier};

use crate::assets::core::{AssetRegistration, PrivacyMode};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

// Type alias for compatibility
pub type ResourceAllocation = ResourceAllocationConfig;

/// Main privacy allocation result with complete configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacyAllocationResult {
    /// Asset being allocated
    pub asset_id: AssetRegistration,

    /// Privacy allocation type (from Proof of State patterns)
    pub allocation_type: PrivacyAllocationType,

    /// Privacy level assigned
    pub privacy_level: PrivacyMode,

    /// Resource allocation configuration
    pub resource_config: ResourceAllocationConfig,

    /// Consensus requirements for this allocation
    pub consensus_requirements: ConsensusRequirementConfig,

    /// CAESAR reward configuration
    pub reward_config: CaesarRewardConfig,

    /// Remote proxy configuration
    pub proxy_config: ProxyConfiguration,

    /// Allocation timestamp and expiry
    pub allocated_at: SystemTime,
    pub expires_at: Option<SystemTime>,

    /// Allocation unique identifier
    pub allocation_id: String,
}

/// Resource allocation configuration with user controls
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceAllocationConfig {
    /// CPU allocation percentage (0.0 - 1.0)
    pub cpu_percentage: f32,

    /// GPU allocation percentage (0.0 - 1.0)  
    pub gpu_percentage: f32,

    /// Memory allocation percentage (0.0 - 1.0)
    pub memory_percentage: f32,

    /// Storage allocation percentage (0.0 - 1.0)
    pub storage_percentage: f32,

    /// Network bandwidth allocation percentage (0.0 - 1.0)
    pub network_percentage: f32,

    /// Maximum concurrent users allowed
    pub max_concurrent_users: u32,

    /// Maximum concurrent processes allowed
    pub max_concurrent_processes: u32,

    /// Duration-based limits
    pub duration_config: DurationLimits,
}

/// Duration-based allocation limits
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DurationLimits {
    /// Maximum total allocation time
    pub max_total_duration: Option<Duration>,

    /// Maximum single session duration
    pub max_session_duration: Option<Duration>,

    /// Minimum required break between sessions
    pub cooldown_duration: Duration,

    /// Grace period before forced termination
    pub grace_period: Duration,

    /// Auto-renewal configuration
    pub auto_renewal: Option<AutoRenewalConfig>,
}

/// Auto-renewal configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutoRenewalConfig {
    /// Enable automatic renewal
    pub enabled: bool,

    /// Maximum number of renewals
    pub max_renewals: u32,

    /// Time before expiry to trigger renewal
    pub renewal_threshold: Duration,

    /// Require user confirmation for renewal
    pub require_confirmation: bool,
}

/// Consensus requirements configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusRequirementConfig {
    /// Require Proof of Space (PoSp) - WHERE
    pub require_proof_of_space: bool,

    /// Require Proof of Stake (PoSt) - WHO
    pub require_proof_of_stake: bool,

    /// Require Proof of Work (PoWk) - WHAT/HOW
    pub require_proof_of_work: bool,

    /// Require Proof of Time (PoTm) - WHEN
    pub require_proof_of_time: bool,

    /// Minimum stake amount required (in CAESAR tokens)
    pub minimum_stake: u64,

    /// Maximum allowed time offset
    pub max_time_offset: Duration,

    /// Proof validation frequency
    pub validation_frequency: Duration,

    /// Required proof difficulty levels
    pub difficulty_requirements: DifficultyRequirements,
}

/// Proof difficulty requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DifficultyRequirements {
    /// PoW difficulty requirement
    pub work_difficulty: u32,

    /// PoSp space commitment requirement (bytes)
    pub space_commitment: u64,

    /// PoSt minimum stake multiplier
    pub stake_multiplier: f32,

    /// PoTm temporal precision requirement
    pub time_precision_ms: u64,
}

/// CAESAR reward configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaesarRewardConfig {
    /// Base reward rate (tokens per hour)
    pub base_reward_rate: f32,

    /// Privacy level multiplier
    pub privacy_multiplier: f32,

    /// Resource utilization multiplier
    pub utilization_multiplier: f32,

    /// Consensus proof bonus
    pub consensus_bonus: f32,

    /// Maximum reward cap per allocation
    pub max_reward_cap: f32,

    /// Reward distribution configuration
    pub distribution_config: RewardDistributionConfig,
}

/// Reward distribution configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RewardDistributionConfig {
    /// Pay rewards immediately on completion
    pub immediate_payout: bool,

    /// Percentage paid immediately (rest staked)
    pub immediate_percentage: f32,

    /// Auto-stake remaining rewards
    pub auto_stake_remainder: bool,

    /// Minimum payout threshold
    pub minimum_payout_threshold: f32,

    /// Payout frequency
    pub payout_frequency: PayoutFrequency,
}

/// Payout frequency options
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PayoutFrequency {
    Immediate,
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

/// Remote proxy configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxyConfiguration {
    /// Enable remote proxy addressing
    pub enabled: bool,

    /// NAT-like addressing preferences
    pub nat_preferences: NatAddressingPreferences,

    /// Proxy node selection criteria
    pub node_selection: ProxyNodeSelection,

    /// Quantum security requirements
    pub quantum_security: QuantumSecurityConfig,

    /// Trust-based proxy selection
    pub trust_requirements: TrustRequirements,
}

/// NAT-like addressing preferences
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NatAddressingPreferences {
    /// Preferred network address ranges
    pub preferred_networks: Vec<String>,

    /// Port allocation preferences
    pub port_preferences: PortAllocationPreferences,

    /// IPv6 preference over IPv4
    pub prefer_ipv6: bool,

    /// Enable UPnP port mapping
    pub enable_upnp: bool,

    /// Connection persistence settings
    pub persistence_config: ConnectionPersistenceConfig,
}

/// Port allocation preferences
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PortAllocationPreferences {
    /// Preferred port ranges
    pub preferred_ranges: Vec<PortRange>,

    /// Avoid well-known ports
    pub avoid_well_known: bool,

    /// Use random port allocation
    pub use_random_allocation: bool,

    /// Port binding timeout
    pub binding_timeout: Duration,
}

/// Port range specification
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PortRange {
    /// Start port (inclusive)
    pub start: u16,
    /// End port (inclusive)
    pub end: u16,
}

/// Connection persistence configuration
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConnectionPersistenceConfig {
    /// Keep connections alive
    pub keep_alive: bool,

    /// Connection timeout
    pub connection_timeout: Duration,

    /// Maximum idle time
    pub max_idle_time: Duration,

    /// Reconnection attempts
    pub max_reconnect_attempts: u32,
}

/// Proxy node selection criteria
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProxyNodeSelection {
    /// Whether authentication is required for proxy nodes
    pub require_authentication: bool,

    /// Required capabilities
    pub required_capabilities: Vec<String>,

    /// Geographic preferences
    pub geographic_preferences: Vec<String>,

    /// Bandwidth requirements
    pub min_bandwidth_mbps: u32,

    /// Latency requirements
    pub max_latency_ms: u32,

    /// Load balancing preferences
    pub load_balancing: LoadBalancingPreferences,
}

/// Load balancing preferences
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LoadBalancingPreferences {
    /// Preferred load balancing strategy
    pub strategy: LoadBalancingStrategy,

    /// Maximum node utilization threshold
    pub max_utilization_threshold: f32,

    /// Enable automatic failover
    pub enable_failover: bool,

    /// Health check frequency
    pub health_check_frequency: Duration,
}

/// Load balancing strategies
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    #[default]
    RoundRobin,
    LeastConnections,
    WeightedRandom,
    LatencyBased,
    AuthenticationBased,
}

/// Quantum security configuration
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct QuantumSecurityConfig {
    /// Enable quantum-resistant encryption
    pub enabled: bool,

    /// Use FALCON-1024 signatures
    pub use_falcon_signatures: bool,

    /// Use Kyber encryption
    pub use_kyber_encryption: bool,

    /// Quantum key distribution
    pub qkd_enabled: bool,

    /// Security level requirements
    pub security_level: QuantumSecurityLevel,
}

/// Quantum security levels
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum QuantumSecurityLevel {
    #[default]
    Basic, // Standard quantum resistance
    Enhanced, // Higher security parameters
    Maximum,  // Highest available security
}

/// Trust requirements for proxy selection (binary authentication)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TrustRequirements {
    /// Require certificate validation
    pub require_certificate_validation: bool,

    /// Require consensus proof validation
    pub require_consensus_validation: bool,

    /// Re-authentication interval
    pub reauth_interval: Duration,

    /// Minimum successful operations before full access
    pub min_successful_operations: u64,
}

/// Default implementations
impl Default for ResourceAllocationConfig {
    fn default() -> Self {
        Self {
            cpu_percentage: 1.0,
            gpu_percentage: 1.0,
            memory_percentage: 1.0,
            storage_percentage: 1.0,
            network_percentage: 1.0,
            max_concurrent_users: 10,
            max_concurrent_processes: 100,
            duration_config: DurationLimits::default(),
        }
    }
}

impl Default for DurationLimits {
    fn default() -> Self {
        Self {
            max_total_duration: Some(Duration::from_secs(24 * 60 * 60)), // 24 hours
            max_session_duration: Some(Duration::from_secs(4 * 60 * 60)), // 4 hours
            cooldown_duration: Duration::from_secs(5 * 60),              // 5 minutes
            grace_period: Duration::from_secs(5 * 60),                   // 5 minutes
            auto_renewal: Some(AutoRenewalConfig::default()),
        }
    }
}

impl Default for AutoRenewalConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_renewals: 3,
            renewal_threshold: Duration::from_secs(10 * 60), // 10 minutes
            require_confirmation: true,
        }
    }
}

impl Default for ConsensusRequirementConfig {
    fn default() -> Self {
        Self {
            require_proof_of_space: true,
            require_proof_of_stake: true,
            require_proof_of_work: true,
            require_proof_of_time: true,
            minimum_stake: 1000, // 1000 CAESAR tokens
            max_time_offset: Duration::from_secs(30),
            validation_frequency: Duration::from_secs(5 * 60), // 5 minutes
            difficulty_requirements: DifficultyRequirements::default(),
        }
    }
}

impl Default for DifficultyRequirements {
    fn default() -> Self {
        Self {
            work_difficulty: 16,             // 16-bit difficulty
            space_commitment: 1_000_000_000, // 1GB
            stake_multiplier: 1.0,
            time_precision_ms: 100,
        }
    }
}
