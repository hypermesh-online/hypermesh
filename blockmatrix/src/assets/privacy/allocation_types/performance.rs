// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Performance characteristics and integration settings for privacy allocation types

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Performance characteristics for allocation types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceCharacteristics {
    /// Expected latency ranges
    pub latency_characteristics: LatencyCharacteristics,

    /// Throughput characteristics
    pub throughput_characteristics: ThroughputCharacteristics,

    /// Scalability characteristics
    pub scalability_characteristics: ScalabilityCharacteristics,

    /// Reliability characteristics
    pub reliability_characteristics: ReliabilityCharacteristics,
}

/// Latency characteristics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LatencyCharacteristics {
    /// Minimum expected latency (ms)
    pub min_latency_ms: u32,

    /// Maximum acceptable latency (ms)
    pub max_latency_ms: u32,

    /// Average expected latency (ms)
    pub avg_latency_ms: u32,

    /// Latency variance tolerance
    pub latency_variance: f32,
}

/// Throughput characteristics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThroughputCharacteristics {
    /// Minimum throughput (MB/s)
    pub min_throughput_mbps: u32,

    /// Maximum throughput (MB/s)
    pub max_throughput_mbps: u32,

    /// Burst throughput capability
    pub burst_capability: bool,

    /// Sustained throughput guarantee
    pub sustained_guarantee: f32,
}

/// Scalability characteristics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScalabilityCharacteristics {
    /// Maximum concurrent connections
    pub max_concurrent_connections: u32,

    /// Horizontal scaling support
    pub horizontal_scaling: bool,

    /// Vertical scaling support
    pub vertical_scaling: bool,

    /// Auto-scaling triggers
    pub auto_scaling_triggers: Vec<String>,
}

/// Reliability characteristics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReliabilityCharacteristics {
    /// Target uptime percentage
    pub target_uptime: f32,

    /// Fault tolerance level
    pub fault_tolerance_level: FaultToleranceLevel,

    /// Recovery time objectives
    pub recovery_time_objective: Duration,

    /// Recovery point objectives
    pub recovery_point_objective: Duration,
}

/// Fault tolerance levels
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FaultToleranceLevel {
    None,
    Basic,
    High,
    Critical,
}

/// Integration settings for allocation types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntegrationSettings {
    /// Consensus system integration
    pub consensus_integration: ConsensusIntegrationSettings,

    /// Proxy system integration
    pub proxy_integration: ProxyIntegrationSettings,

    /// Reward system integration
    pub reward_integration: RewardIntegrationSettings,

    /// External system integrations
    pub external_integrations: Vec<ExternalIntegration>,
}

/// Consensus system integration settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusIntegrationSettings {
    /// Required consensus proofs
    pub required_proofs: Vec<String>,

    /// Proof validation frequency
    pub validation_frequency: Duration,

    /// Consensus participation requirements
    pub participation_requirements: ParticipationRequirements,
}

/// Consensus participation requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParticipationRequirements {
    /// Must participate in consensus
    pub must_participate: bool,

    /// Minimum participation level
    pub min_participation_level: f32,

    /// Contribution requirements
    pub contribution_requirements: Vec<String>,
}

/// Proxy system integration settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxyIntegrationSettings {
    /// Proxy usage requirements
    pub proxy_requirements: ProxyRequirements,

    /// NAT translation settings
    pub nat_settings: NatIntegrationSettings,

    /// Load balancing settings
    pub load_balancing_settings: LoadBalancingSettings,
}

/// Proxy usage requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxyRequirements {
    /// Require proxy for access
    pub require_proxy: bool,

    /// Allowed proxy types
    pub allowed_proxy_types: Vec<String>,

    /// Proxy chaining allowed
    pub allow_proxy_chaining: bool,

    /// Maximum proxy chain length
    pub max_chain_length: u32,
}

/// NAT integration settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NatIntegrationSettings {
    /// Enable NAT translation
    pub enable_nat: bool,

    /// NAT mapping persistence
    pub mapping_persistence: Duration,

    /// Port allocation strategy
    pub port_allocation_strategy: PortAllocationStrategy,
}

/// Port allocation strategies
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PortAllocationStrategy {
    Sequential,
    Random,
    UserDefined,
    LoadBalanced,
}

/// Load balancing settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoadBalancingSettings {
    /// Load balancing algorithm
    pub algorithm: LoadBalancingAlgorithm,

    /// Health check settings
    pub health_check: HealthCheckSettings,

    /// Failover settings
    pub failover: FailoverSettings,
}

/// Load balancing algorithms
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LoadBalancingAlgorithm {
    RoundRobin,
    WeightedRoundRobin,
    LeastConnections,
    IpHash,
    GeographicProximity,
    AuthenticationBased,
}

/// Health check settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthCheckSettings {
    /// Health check frequency
    pub frequency: Duration,

    /// Health check timeout
    pub timeout: Duration,

    /// Failure threshold
    pub failure_threshold: u32,

    /// Recovery threshold
    pub recovery_threshold: u32,
}

/// Failover settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FailoverSettings {
    /// Enable automatic failover
    pub enable_auto_failover: bool,

    /// Failover trigger conditions
    pub trigger_conditions: Vec<String>,

    /// Failover timeout
    pub failover_timeout: Duration,

    /// Rollback conditions
    pub rollback_conditions: Vec<String>,
}

/// Reward system integration settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RewardIntegrationSettings {
    /// Reward calculation method
    pub calculation_method: RewardCalculationMethod,

    /// Reward distribution settings
    pub distribution_settings: RewardDistributionSettings,

    /// Performance bonuses
    pub performance_bonuses: Vec<PerformanceBonus>,
}

/// Reward calculation methods
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RewardCalculationMethod {
    TimeBasedLinear,
    TimeBasedDecaying,
    UtilizationBased,
    PerformanceBased,
    Hybrid,
}

/// Reward distribution settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RewardDistributionSettings {
    /// Distribution frequency
    pub frequency: super::super::PayoutFrequency,

    /// Minimum payout threshold
    pub min_payout_threshold: f32,

    /// Auto-staking percentage
    pub auto_stake_percentage: f32,
}

/// Performance bonus configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceBonus {
    /// Metric being measured
    pub metric: String,

    /// Threshold for bonus
    pub threshold: f32,

    /// Bonus multiplier
    pub multiplier: f32,

    /// Maximum bonus cap
    pub max_bonus: f32,
}

/// External system integration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalIntegration {
    /// Integration name
    pub name: String,

    /// Integration type
    pub integration_type: ExternalIntegrationType,

    /// Configuration parameters
    pub config_params: std::collections::HashMap<String, String>,

    /// Required for allocation type
    pub required: bool,
}

/// Types of external integrations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExternalIntegrationType {
    Monitoring,
    Logging,
    Authentication,
    Storage,
    Networking,
    Security,
    Analytics,
}
