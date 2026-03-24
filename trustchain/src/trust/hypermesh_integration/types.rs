// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Trust Integration types and data structures
//!
//! Domain-specific types for TrustChain's trust integration layer.
//! These wrap the canonical lib identifiers (`hypermesh_lib::NodeId`,
//! `hypermesh_lib::AssetId`) with TrustChain-specific context.

use serde::{Deserialize, Serialize};
use std::net::Ipv6Addr;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Asset kind for trust-layer classification
///
/// Aligned with `hypermesh_lib::SystemAssetKind` variants.
/// Uses the canonical asset taxonomy (Blockchain/Dns, not VirtualMachine/Library).
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum TrustAssetKind {
    Cpu,
    Gpu,
    Memory,
    Storage,
    Network,
    Container,
    Economic,
    Blockchain,
    Dns,
    /// Mesh relay bandwidth as a first-class asset (R10)
    Transmission,
    Dashboard,
    Identity,
    /// Key rotation event recorded on-chain (§6.2.2)
    KeyRotation,
    /// Share invitation registered on-chain
    Invitation,
    /// Direct message registered on-chain
    Message,
}

impl From<hypermesh_lib::asset::SystemAssetKind> for TrustAssetKind {
    fn from(kind: hypermesh_lib::asset::SystemAssetKind) -> Self {
        use hypermesh_lib::asset::SystemAssetKind;
        match kind {
            SystemAssetKind::Cpu => Self::Cpu,
            SystemAssetKind::Gpu => Self::Gpu,
            SystemAssetKind::Memory => Self::Memory,
            SystemAssetKind::Storage => Self::Storage,
            SystemAssetKind::Network => Self::Network,
            SystemAssetKind::Container => Self::Container,
            SystemAssetKind::Economic => Self::Economic,
            SystemAssetKind::Blockchain => Self::Blockchain,
            SystemAssetKind::Dns => Self::Dns,
            SystemAssetKind::Transmission => Self::Transmission,
            SystemAssetKind::Dashboard => Self::Dashboard,
            SystemAssetKind::Identity => Self::Identity,
            SystemAssetKind::KeyRotation => Self::KeyRotation,
            SystemAssetKind::Invitation => Self::Invitation,
            SystemAssetKind::Message => Self::Message,
        }
    }
}

impl From<TrustAssetKind> for hypermesh_lib::asset::SystemAssetKind {
    fn from(kind: TrustAssetKind) -> Self {
        match kind {
            TrustAssetKind::Cpu => Self::Cpu,
            TrustAssetKind::Gpu => Self::Gpu,
            TrustAssetKind::Memory => Self::Memory,
            TrustAssetKind::Storage => Self::Storage,
            TrustAssetKind::Network => Self::Network,
            TrustAssetKind::Container => Self::Container,
            TrustAssetKind::Economic => Self::Economic,
            TrustAssetKind::Blockchain => Self::Blockchain,
            TrustAssetKind::Dns => Self::Dns,
            TrustAssetKind::Transmission => Self::Transmission,
            TrustAssetKind::Dashboard => Self::Dashboard,
            TrustAssetKind::Identity => Self::Identity,
            TrustAssetKind::KeyRotation => Self::KeyRotation,
            TrustAssetKind::Invitation => Self::Invitation,
            TrustAssetKind::Message => Self::Message,
        }
    }
}

/// Authenticated asset in HyperMesh trust layer
///
/// Wraps `hypermesh_lib::AssetId` (the canonical string identifier) with
/// TrustChain-specific context: UUID, asset kind, and network membership.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AuthenticatedAsset {
    /// Canonical asset identifier from hypermesh_lib
    pub asset_id: hypermesh_lib::AssetId,
    /// UUID for this asset instance
    pub uuid: Uuid,
    /// Asset kind classification
    pub asset_kind: TrustAssetKind,
    /// Network this asset belongs to
    pub network_id: String,
}

/// Authenticated node in HyperMesh trust layer
///
/// Wraps `hypermesh_lib::NodeId` (the canonical string identifier) with
/// TrustChain-specific context: public key, network address, and node role.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AuthenticatedNode {
    /// Canonical node identifier from hypermesh_lib
    pub node_id: hypermesh_lib::NodeId,
    /// Node's public key for cryptographic verification
    pub public_key: String,
    /// Node's IPv6 network address
    pub network_address: Ipv6Addr,
    /// Node's role in the network
    pub node_type: NodeType,
}

impl AuthenticatedNode {
    /// Convert to a `ScopedIdentity` with the given scope.
    ///
    /// The `AuthenticatedNode` carries the cryptographic `NodeId`; callers
    /// supply the `IdentityScope` that matches the context the node is
    /// operating in (device-local, private network, public network, etc.).
    pub fn to_scoped_identity(
        &self,
        scope: hypermesh_lib::IdentityScope,
    ) -> hypermesh_lib::ScopedIdentity {
        hypermesh_lib::ScopedIdentity::new_node(self.node_id, scope)
    }
}

/// Entity ID for authentication (assets or nodes)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityId {
    Asset(AuthenticatedAsset),
    Node(AuthenticatedNode),
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
    /// Whether state proof verification passed
    pub state_verified: bool,
    /// When this status was last checked
    pub last_checked: SystemTime,
    /// When this status expires
    pub expiry: SystemTime,
}

/// Byzantine fault detection report
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ByzantineReport {
    pub node: AuthenticatedNode,
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
    pub witness_nodes: Vec<AuthenticatedNode>,
    pub timestamp: SystemTime,
    pub cryptographic_proof: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EvidenceType {
    ConflictingSignatures,
    InvalidProof,
    NetworkBehaviorLog,
    StateProofViolation,
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

// ---------------------------------------------------------------------------
// Workload Identity Types (Items 1.6-1.8)
// ---------------------------------------------------------------------------

/// A node's complete identity record in TrustChain.
///
/// Combines the authenticated node with its blockchain/privacy scope and
/// an optional certificate fingerprint linking it to a TrustChain certificate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeIdentity {
    pub node: AuthenticatedNode,
    pub scope: hypermesh_lib::IdentityScope,
    pub certificate_fingerprint: Option<[u8; 32]>,
}

/// A service running on a node.
///
/// Services are identified by an `AssetId` (since everything in HyperMesh
/// is an asset) and scoped to a host node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceIdentity {
    pub service_id: hypermesh_lib::AssetId,
    pub host_node: hypermesh_lib::NodeId,
    pub scope: hypermesh_lib::IdentityScope,
    pub service_name: String,
}

/// An autonomous agent acting on behalf of a node.
///
/// Agents carry a list of capability strings describing what operations
/// they are authorized to perform on the controlling node's behalf.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    pub agent_id: hypermesh_lib::AssetId,
    pub controlling_node: hypermesh_lib::NodeId,
    pub scope: hypermesh_lib::IdentityScope,
    pub capabilities: Vec<String>,
}

/// What entity a certificate authenticates.
///
/// Embedded in X.509 certificates via [`IdentityScopeExtension`] so that
/// relying parties know what kind of workload they are speaking to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CertificateSubjectType {
    Node,
    Service,
    Agent,
}

impl From<hypermesh_lib::WorkloadType> for CertificateSubjectType {
    fn from(wt: hypermesh_lib::WorkloadType) -> Self {
        match wt {
            hypermesh_lib::WorkloadType::Node => Self::Node,
            hypermesh_lib::WorkloadType::Service => Self::Service,
            hypermesh_lib::WorkloadType::Agent => Self::Agent,
        }
    }
}

impl From<CertificateSubjectType> for hypermesh_lib::WorkloadType {
    fn from(cst: CertificateSubjectType) -> Self {
        match cst {
            CertificateSubjectType::Node => Self::Node,
            CertificateSubjectType::Service => Self::Service,
            CertificateSubjectType::Agent => Self::Agent,
        }
    }
}

/// Custom X.509 extension for HyperMesh identity scope.
///
/// OID: 1.3.6.1.4.1.XXXXX.1 (placeholder pending IANA private enterprise number).
///
/// Encodes the workload type, blockchain scope, tracking flag, and workload
/// type into a compact binary format suitable for embedding in X.509
/// certificate extensions.
///
/// Wire format (4 bytes):
/// ```text
/// byte 0: subject_type  (0=Node, 1=Service, 2=Agent)
/// byte 1: blockchain_scope (0=Device, 1=Network)
/// byte 2: tracked (0=false, 1=true)
/// byte 3: workload_type (0=Node, 1=Service, 2=Agent)
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityScopeExtension {
    pub subject_type: CertificateSubjectType,
    pub blockchain_scope: hypermesh_lib::BlockchainScope,
    pub tracked: bool,
    pub workload_type: hypermesh_lib::WorkloadType,
}

/// Placeholder OID for HyperMesh identity scope extension.
/// Format: 1.3.6.1.4.1.{PEN}.1 where PEN is the private enterprise number.
pub const IDENTITY_SCOPE_EXTENSION_OID: &str = "1.3.6.1.4.1.99999.1";

impl IdentityScopeExtension {
    /// Encode this extension to a 4-byte representation for X.509 embedding.
    pub fn to_bytes(&self) -> [u8; 4] {
        let subject = match self.subject_type {
            CertificateSubjectType::Node => 0u8,
            CertificateSubjectType::Service => 1u8,
            CertificateSubjectType::Agent => 2u8,
        };
        let scope = match self.blockchain_scope {
            hypermesh_lib::BlockchainScope::Device => 0u8,
            hypermesh_lib::BlockchainScope::Network => 1u8,
        };
        let tracked = u8::from(self.tracked);
        let workload = match self.workload_type {
            hypermesh_lib::WorkloadType::Node => 0u8,
            hypermesh_lib::WorkloadType::Service => 1u8,
            hypermesh_lib::WorkloadType::Agent => 2u8,
        };
        [subject, scope, tracked, workload]
    }

    /// Decode from a 4-byte representation.
    ///
    /// Returns `None` if any byte value is out of range.
    pub fn from_bytes(bytes: &[u8; 4]) -> Option<Self> {
        let subject_type = match bytes[0] {
            0 => CertificateSubjectType::Node,
            1 => CertificateSubjectType::Service,
            2 => CertificateSubjectType::Agent,
            _ => return None,
        };
        let blockchain_scope = match bytes[1] {
            0 => hypermesh_lib::BlockchainScope::Device,
            1 => hypermesh_lib::BlockchainScope::Network,
            _ => return None,
        };
        let tracked = match bytes[2] {
            0 => false,
            1 => true,
            _ => return None,
        };
        let workload_type = match bytes[3] {
            0 => hypermesh_lib::WorkloadType::Node,
            1 => hypermesh_lib::WorkloadType::Service,
            2 => hypermesh_lib::WorkloadType::Agent,
            _ => return None,
        };
        Some(Self {
            subject_type,
            blockchain_scope,
            tracked,
            workload_type,
        })
    }

    /// Create from an `IdentityScope` and `WorkloadType`.
    pub fn from_scope(
        scope: &hypermesh_lib::IdentityScope,
        workload_type: hypermesh_lib::WorkloadType,
    ) -> Self {
        Self {
            subject_type: CertificateSubjectType::from(workload_type),
            blockchain_scope: scope.blockchain_scope,
            tracked: scope.tracked,
            workload_type,
        }
    }
}

// Supporting type stubs
pub(crate) struct HyperMeshNetworkClient;
pub(crate) struct AssetVerificationRecord;
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
    pub(crate) _node: AuthenticatedNode,
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
