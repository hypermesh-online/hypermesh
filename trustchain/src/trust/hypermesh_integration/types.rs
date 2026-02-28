// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Trust Integration types and data structures

use std::time::{Duration, SystemTime};
use std::net::Ipv6Addr;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// Universal asset type enumeration
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum AssetType {
    Cpu,
    Gpu,
    Memory,
    Storage,
    Network,
    Container,
    Economic,
    VirtualMachine,
    Library,
}

/// Asset identification in HyperMesh
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId {
    pub uuid: Uuid,
    pub asset_type: AssetType,
    pub network_id: String,
}

/// Node identification in HyperMesh
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId {
    pub public_key: String,
    pub network_address: Ipv6Addr,
    pub node_type: NodeType,
}

/// Entity ID for authentication (assets or nodes)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityId {
    Asset(AssetId),
    Node(NodeId),
}

/// Proxy connection identifier
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProxyId {
    pub proxy_address: Ipv6Addr,
    pub target_address: Ipv6Addr,
    pub session_id: String,
}

/// Node types in HyperMesh network
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    Full,
    Light,
    Validator,
    Proxy,
    Bridge,
}

/// Binary authentication status for assets and nodes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticationStatus {
    /// Whether the entity passed authentication
    pub authenticated: bool,
    /// Whether the certificate is valid
    pub certificate_valid: bool,
    /// Whether consensus verification passed
    pub consensus_verified: bool,
    /// When this status was last checked
    pub last_checked: SystemTime,
    /// When this status expires
    pub expiry: SystemTime,
}

/// Byzantine fault detection report
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ByzantineReport {
    pub node_id: NodeId,
    pub detection_time: SystemTime,
    pub fault_type: ByzantineFaultType,
    pub evidence: Vec<ByzantineEvidence>,
    pub confidence: f64,
    pub recommended_action: RecommendedAction,
    pub alert_level: AlertLevel,
}

/// Types of Byzantine faults
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ByzantineFaultType {
    DoubleSigning,
    EquivocationAttack,
    NothingAtStake,
    LongRangeAttack,
    Censorship,
    DataWithholding,
    InvalidStateTransition,
    TimestampManipulation,
    ResourceExhaustion,
    IdentitySpoofing,
}

/// Evidence of Byzantine behavior
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ByzantineEvidence {
    pub evidence_type: EvidenceType,
    pub data: Vec<u8>,
    pub witness_nodes: Vec<NodeId>,
    pub timestamp: SystemTime,
    pub cryptographic_proof: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EvidenceType {
    ConflictingSignatures,
    InvalidProof,
    NetworkBehaviorLog,
    ConsensusViolation,
    CryptographicMismatch,
}

/// Recommended actions for Byzantine faults
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RecommendedAction {
    Monitor,
    Quarantine,
    Slash,
    Exclude,
    Investigate,
    EmergencyShutdown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AlertLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Proxy connection for remote asset access
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxyConnection {
    pub proxy_id: ProxyId,
    pub connection_type: ProxyType,
    /// Binary: is the proxy authenticated?
    pub is_authenticated: bool,
    pub established_at: SystemTime,
    pub last_activity: SystemTime,
    pub performance_metrics: ProxyPerformanceMetrics,
    pub security_context: SecurityContext,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProxyType {
    Direct,
    Encrypted,
    Federated,
    Anonymous,
}

/// Configuration for trust validator
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrustValidatorConfig {
    /// Whether authentication is required
    pub require_authentication: bool,
    /// Cache TTL for authentication results
    pub auth_cache_ttl: Duration,
    /// Maximum proxy hops
    pub max_proxy_hops: u32,
    /// Monitoring interval
    pub monitoring_interval: Duration,
    /// Alert thresholds
    pub alert_thresholds: AlertThresholds,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub byzantine_confidence: f64,
    pub performance_degradation: f64,
    pub availability_threshold: f64,
}

/// Performance metrics for authentication
#[derive(Default)]
pub struct TrustMetrics {
    pub auth_checks: std::sync::atomic::AtomicU64,
    pub byzantine_detections: std::sync::atomic::AtomicU64,
    pub proxy_connections: std::sync::atomic::AtomicU64,
    pub average_validation_time_ms: std::sync::atomic::AtomicU32,
    pub alert_count: std::sync::atomic::AtomicU64,
}

/// Validator performance metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrustValidatorMetrics {
    pub auth_checks: u64,
    pub byzantine_detections: u64,
    pub proxy_connections: u64,
    pub average_validation_time_ms: u32,
    pub alert_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxyPerformanceMetrics;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityContext;

/// Node behavior tracking (binary: authenticated or byzantine)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct NodeBehavior {
    pub(crate) is_authenticated: bool,
    pub(crate) is_byzantine: bool,
    pub(crate) last_seen: SystemTime,
}


// Supporting type stubs
pub(crate) struct HyperMeshNetworkClient;
pub(crate) struct AssetMetadata;
pub(crate) struct AssetVerificationEngine;
pub(crate) struct ByzantinePatterns;
pub(crate) struct DetectionAlgorithms;
pub(crate) struct AlertSystem;
pub(crate) struct ProxySelectionStrategy;
pub(crate) struct ProxyPerformanceMonitor;

/// Byzantine behavior analysis result
pub(crate) struct ByzantineBehaviorAnalysis {
    pub(crate) is_byzantine: bool,
    pub(crate) fault_type: ByzantineFaultType,
    pub(crate) evidence: Vec<ByzantineEvidence>,
    pub(crate) confidence: f64,
    pub(crate) recommended_action: RecommendedAction,
    pub(crate) alert_level: AlertLevel,
}

/// Proxy candidate for selection
pub(crate) struct ProxyCandidate {
    pub(crate) _node_id: NodeId,
    pub(crate) _is_authenticated: bool,
    pub(crate) _performance_metrics: ProxyPerformanceMetrics,
    pub(crate) _distance_hops: u32,
}

impl Default for TrustValidatorConfig {
    fn default() -> Self {
        Self {
            require_authentication: true,
            auth_cache_ttl: Duration::from_secs(3600),
            max_proxy_hops: 3,
            monitoring_interval: Duration::from_secs(60),
            alert_thresholds: AlertThresholds {
                byzantine_confidence: 0.8,
                performance_degradation: 0.5,
                availability_threshold: 0.95,
            },
        }
    }
}
