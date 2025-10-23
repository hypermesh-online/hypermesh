//! Privacy Allocation Types - Based on NKrypt patterns
//!
//! Implements the four allocation types from NKrypt with enhanced
//! constraints and transition validation.

use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};
use crate::assets::core::{AssetResult, AssetError, PrivacyLevel};
use crate::consensus::proof::ConsensusProof;

/// Privacy allocation types from NKrypt patterns
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivacyAllocationType {
    /// Internal use only, no sharing, no external access
    Private,
    /// Cross-network accessible, full discovery
    Public,
    /// No identity tracking, privacy-first sharing
    Anonymous,
    /// Full consensus validation required (PoSp+PoSt+PoWk+PoTm)
    Verified,
}

impl PrivacyAllocationType {
    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            PrivacyAllocationType::Private => "Internal use only, no external access or sharing",
            PrivacyAllocationType::Public => "Cross-network accessible with full discovery capabilities", 
            PrivacyAllocationType::Anonymous => "Privacy-first sharing with no identity tracking",
            PrivacyAllocationType::Verified => "Maximum security with full consensus proof validation",
        }
    }
    
    /// Check if allocation type requires consensus proof
    pub fn requires_consensus_proof(&self) -> bool {
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
        matches!(self, 
            PrivacyAllocationType::Public | 
            PrivacyAllocationType::Verified
        )
    }
    
    /// Get minimum required privacy level
    pub fn minimum_privacy_level(&self) -> PrivacyLevel {
        match self {
            PrivacyAllocationType::Private => PrivacyLevel::Private,
            PrivacyAllocationType::Public => PrivacyLevel::PublicNetwork,
            PrivacyAllocationType::Anonymous => PrivacyLevel::P2P,
            PrivacyAllocationType::Verified => PrivacyLevel::FullPublic,
        }
    }
    
    /// Get maximum allowed privacy level
    pub fn maximum_privacy_level(&self) -> PrivacyLevel {
        match self {
            PrivacyAllocationType::Private => PrivacyLevel::PrivateNetwork,
            PrivacyAllocationType::Public => PrivacyLevel::FullPublic,
            PrivacyAllocationType::Anonymous => PrivacyLevel::PublicNetwork,
            PrivacyAllocationType::Verified => PrivacyLevel::FullPublic,
        }
    }
    
    /// Get base CAESAR reward multiplier
    pub fn base_reward_multiplier(&self) -> f32 {
        match self {
            PrivacyAllocationType::Private => 0.0, // No rewards for private allocation
            PrivacyAllocationType::Public => 0.75,
            PrivacyAllocationType::Anonymous => 0.5, // Lower rewards for anonymous
            PrivacyAllocationType::Verified => 1.0, // Maximum rewards
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
            PrivacyAllocationType::Private => vec![
                "local_access".to_string(),
                "memory_isolation".to_string(),
            ],
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
                "consensus_validation".to_string(),
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

/// Security requirements for allocation types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityRequirements {
    /// Encryption requirements
    pub encryption_requirements: EncryptionRequirements,
    
    /// Authentication requirements
    pub authentication_requirements: AuthenticationRequirements,
    
    /// Audit logging requirements
    pub audit_requirements: AuditRequirements,
    
    /// Data protection requirements
    pub data_protection: DataProtectionRequirements,
}

/// Encryption requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncryptionRequirements {
    /// Require encryption in transit
    pub require_transport_encryption: bool,
    
    /// Require encryption at rest
    pub require_storage_encryption: bool,
    
    /// Minimum encryption strength
    pub minimum_key_length: u32,
    
    /// Allowed encryption algorithms
    pub allowed_algorithms: Vec<String>,
    
    /// Require quantum-resistant encryption
    pub require_quantum_resistant: bool,
}

/// Authentication requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticationRequirements {
    /// Authentication methods required
    pub required_methods: Vec<AuthenticationMethod>,
    
    /// Multi-factor authentication required
    pub require_mfa: bool,
    
    /// Certificate validation required
    pub require_certificate_validation: bool,
    
    /// Biometric authentication required
    pub require_biometric: bool,
    
    /// Session management requirements
    pub session_requirements: SessionRequirements,
}

/// Authentication method types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthenticationMethod {
    Password,
    Certificate,
    Token,
    Biometric,
    Hardware,
    ConsensusProof,
}

/// Session management requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionRequirements {
    /// Maximum session duration
    pub max_session_duration: Duration,
    
    /// Session idle timeout
    pub idle_timeout: Duration,
    
    /// Require session renewal
    pub require_renewal: bool,
    
    /// Session binding requirements
    pub binding_requirements: Vec<String>,
}

/// Audit logging requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditRequirements {
    /// Events that must be logged
    pub required_events: Vec<String>,
    
    /// Log retention period
    pub retention_period: Duration,
    
    /// Real-time monitoring required
    pub require_realtime_monitoring: bool,
    
    /// External audit system integration
    pub external_audit_integration: bool,
}

/// Data protection requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataProtectionRequirements {
    /// Data classification requirements
    pub classification_requirements: Vec<String>,
    
    /// Data retention policies
    pub retention_policies: Vec<RetentionPolicy>,
    
    /// Data anonymization requirements
    pub anonymization_requirements: AnonymizationRequirements,
    
    /// Cross-border transfer restrictions
    pub transfer_restrictions: Vec<String>,
}

/// Data retention policy
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Data type
    pub data_type: String,
    
    /// Retention period
    pub retention_period: Duration,
    
    /// Automatic deletion
    pub auto_delete: bool,
    
    /// Archive policy
    pub archive_policy: Option<ArchivePolicy>,
}

/// Archive policy for retained data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArchivePolicy {
    /// Archive after period
    pub archive_after: Duration,
    
    /// Archive location
    pub archive_location: String,
    
    /// Archive encryption
    pub archive_encryption: bool,
}

/// Data anonymization requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnonymizationRequirements {
    /// Anonymization techniques required
    pub required_techniques: Vec<AnonymizationTechnique>,
    
    /// K-anonymity level
    pub k_anonymity_level: Option<u32>,
    
    /// Differential privacy parameters
    pub differential_privacy: Option<DifferentialPrivacyParams>,
}

/// Data anonymization techniques
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AnonymizationTechnique {
    Hashing,
    Tokenization,
    Generalization,
    Suppression,
    Noise,
    DifferentialPrivacy,
}

/// Differential privacy parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DifferentialPrivacyParams {
    /// Privacy budget (epsilon)
    pub epsilon: f32,
    
    /// Delta parameter
    pub delta: f32,
    
    /// Sensitivity
    pub sensitivity: f32,
}

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
    TrustScore,
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
    pub frequency: super::PayoutFrequency,
    
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

/// Privacy transition validation and management
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacyTransition {
    /// Current allocation type
    pub from_type: PrivacyAllocationType,
    
    /// Target allocation type
    pub to_type: PrivacyAllocationType,
    
    /// Transition timestamp
    pub transition_time: SystemTime,
    
    /// Transition reason
    pub reason: String,
    
    /// Validation requirements
    pub validation_requirements: TransitionValidation,
    
    /// Transition impact
    pub impact_assessment: TransitionImpact,
}

/// Validation requirements for privacy transitions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransitionValidation {
    /// Require user consent
    pub require_user_consent: bool,
    
    /// Require consensus proof
    pub require_consensus_proof: bool,
    
    /// Require administrator approval
    pub require_admin_approval: bool,
    
    /// Cooling off period
    pub cooling_off_period: Duration,
    
    /// Validation criteria
    pub validation_criteria: Vec<ValidationCriterion>,
}

/// Validation criteria for transitions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationCriterion {
    /// Criterion name
    pub name: String,
    
    /// Criterion type
    pub criterion_type: ValidationCriterionType,
    
    /// Required value or threshold
    pub required_value: String,
    
    /// Validation method
    pub validation_method: String,
}

/// Types of validation criteria
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ValidationCriterionType {
    TrustScore,
    StakeAmount,
    HistoryCheck,
    PerformanceMetric,
    SecurityCheck,
    ComplianceCheck,
}

/// Impact assessment for privacy transitions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransitionImpact {
    /// Privacy level change impact
    pub privacy_impact: PrivacyImpact,
    
    /// Performance impact
    pub performance_impact: PerformanceImpact,
    
    /// Security impact
    pub security_impact: SecurityImpact,
    
    /// Economic impact
    pub economic_impact: EconomicImpact,
}

/// Privacy impact assessment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacyImpact {
    /// Privacy increase/decrease
    pub privacy_delta: i8,
    
    /// Anonymity change
    pub anonymity_change: AnonymityChange,
    
    /// Data exposure change
    pub exposure_change: ExposureChange,
}

/// Anonymity change assessment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AnonymityChange {
    Increased,
    Decreased,
    NoChange,
}

/// Data exposure change assessment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExposureChange {
    Increased,
    Decreased,
    NoChange,
}

/// Performance impact assessment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceImpact {
    /// Latency change (ms)
    pub latency_delta: i32,
    
    /// Throughput change (%)
    pub throughput_delta: f32,
    
    /// Reliability change
    pub reliability_delta: f32,
}

/// Security impact assessment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityImpact {
    /// Security level change
    pub security_level_delta: i8,
    
    /// Attack surface change
    pub attack_surface_change: AttackSurfaceChange,
    
    /// Compliance impact
    pub compliance_impact: Vec<String>,
}

/// Attack surface change assessment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AttackSurfaceChange {
    Increased,
    Decreased,
    NoChange,
}

/// Economic impact assessment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EconomicImpact {
    /// Reward rate change
    pub reward_rate_delta: f32,
    
    /// Cost change
    pub cost_delta: f32,
    
    /// Stake requirement change
    pub stake_requirement_delta: i64,
}

impl PrivacyTransition {
    /// Validate if transition is allowed and safe
    pub async fn validate_transition(&self) -> AssetResult<bool> {
        // Check if base transition is allowed
        if !self.from_type.can_transition_to(&self.to_type) {
            return Ok(false);
        }
        
        // Validate specific requirements
        if self.validation_requirements.require_consensus_proof {
            // Check if consensus proof is available and valid
            // This would integrate with the consensus system
        }
        
        if self.validation_requirements.require_user_consent {
            // Check if user consent has been obtained
            // This would integrate with user consent management
        }
        
        // Validate each criterion
        for criterion in &self.validation_requirements.validation_criteria {
            if !self.validate_criterion(criterion).await? {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    async fn validate_criterion(&self, _criterion: &ValidationCriterion) -> AssetResult<bool> {
        // Implementation would validate specific criteria
        // For now, return true as placeholder
        Ok(true)
    }
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