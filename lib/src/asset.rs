// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Canonical asset type definitions shared across all HyperMesh crates.
//!
//! Three-pillar design: **Kind** (classification) + **Status** (state machine) +
//! **Adapter** (programmable runtime interface).
//!
//! - [`AssetKind`] — two-level classification (system or user-defined).
//! - [`BaseState`] — infrastructure lifecycle states.
//! - [`AssetStatusTrait`] — programmable state machine trait (domain states map to [`BaseState`]).
//! - [`AssetAdapter`] — fully programmable runtime interface for asset behavior.
//! - [`AssetMetadata`] — common metadata for all assets.

use serde::{Deserialize, Serialize};
use std::fmt;

use super::protocol::HardwareCapabilities;
use super::types::{AssetAddress, AssetId, ContentHash};

// ---------------------------------------------------------------------------
// Pillar 1: AssetKind — two-level classification
// ---------------------------------------------------------------------------

/// Two-level asset classification: system-defined or user-defined (via Catalog).
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum AssetKind {
    /// Built-in system resource types.
    System(SystemAssetKind),
    /// Catalog-registered user-defined types.
    UserDefined(UserAssetKind),
}

impl fmt::Display for AssetKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetKind::System(s) => write!(f, "System({s})"),
            AssetKind::UserDefined(u) => write!(f, "UserDefined({})", u.type_name),
        }
    }
}

impl From<SystemAssetKind> for AssetKind {
    fn from(kind: SystemAssetKind) -> Self {
        AssetKind::System(kind)
    }
}

// ---------------------------------------------------------------------------
// SystemAssetKind — 9 built-in resource types
// ---------------------------------------------------------------------------

/// Built-in system asset types.
///
/// Union of blockmatrix's `BaseSystemType` (9 variants) plus `Dns` and `Transmission`.
/// Each variant has a stable `type_id` used in serialisation and eBPF maps.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum SystemAssetKind {
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
}

impl SystemAssetKind {
    /// Stable numeric identifier (0-9). Must never change once assigned.
    pub fn type_id(&self) -> u8 {
        match self {
            Self::Cpu => 0,
            Self::Gpu => 1,
            Self::Memory => 2,
            Self::Storage => 3,
            Self::Network => 4,
            Self::Container => 5,
            Self::Economic => 6,
            Self::Blockchain => 7,
            Self::Dns => 8,
            Self::Transmission => 9,
        }
    }

    /// Human-readable type name.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Cpu => "Cpu",
            Self::Gpu => "Gpu",
            Self::Memory => "Memory",
            Self::Storage => "Storage",
            Self::Network => "Network",
            Self::Container => "Container",
            Self::Economic => "Economic",
            Self::Blockchain => "Blockchain",
            Self::Dns => "Dns",
            Self::Transmission => "Transmission",
        }
    }
}

impl fmt::Display for SystemAssetKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.type_name())
    }
}

// ---------------------------------------------------------------------------
// UserAssetKind — Catalog-registered types
// ---------------------------------------------------------------------------

/// A user-defined asset type registered through the Catalog.
///
/// Identified by a human-readable name and a content-addressed hash
/// derived from its Catalog package definition.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct UserAssetKind {
    /// Human-readable type name (e.g. "MyCustomService").
    pub type_name: String,
    /// Content hash of the Catalog package that defines this type.
    pub type_hash: ContentHash,
}

// ---------------------------------------------------------------------------
// Pillar 2: BaseState + AssetStatus trait
// ---------------------------------------------------------------------------

/// Infrastructure lifecycle state. Every domain-specific state maps down to one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BaseState {
    /// Ready for allocation.
    Available,
    /// Reserved but not yet actively used.
    Allocated,
    /// Actively serving workload.
    InUse,
    /// Temporarily paused (can resume without maintenance).
    Suspended,
    /// Under repair or upgrade.
    Maintenance,
    /// Irrecoverable error state.
    Failed,
}

impl BaseState {
    /// Whether the asset is in a working state (Available, Allocated, or InUse).
    pub fn is_operational(&self) -> bool {
        matches!(self, Self::Available | Self::Allocated | Self::InUse)
    }

    /// Whether the asset is actively serving workload.
    pub fn is_active(&self) -> bool {
        matches!(self, Self::InUse)
    }

    /// Whether the asset is ready for new allocation.
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    /// Scheduling priority (higher = more preferred for work).
    ///
    /// Failed=0, Maintenance=1, Suspended=2, Available=3, Allocated=4, InUse=5.
    pub fn priority(&self) -> u8 {
        match self {
            Self::Failed => 0,
            Self::Maintenance => 1,
            Self::Suspended => 2,
            Self::Available => 3,
            Self::Allocated => 4,
            Self::InUse => 5,
        }
    }

    /// Human-readable description of this state.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Available => "Ready for allocation",
            Self::Allocated => "Reserved but not yet active",
            Self::InUse => "Actively serving workload",
            Self::Suspended => "Temporarily paused",
            Self::Maintenance => "Under repair or upgrade",
            Self::Failed => "Irrecoverable error state",
        }
    }
}

impl fmt::Display for BaseState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Available => "Available",
            Self::Allocated => "Allocated",
            Self::InUse => "InUse",
            Self::Suspended => "Suspended",
            Self::Maintenance => "Maintenance",
            Self::Failed => "Failed",
        };
        f.write_str(label)
    }
}

/// Backward-compatibility alias. Prefer [`BaseState`] in new code.
#[deprecated(note = "renamed to BaseState")]
pub type AssetState = BaseState;

/// Error returned when a state transition is invalid.
#[derive(Debug, Clone, thiserror::Error)]
pub enum StatusTransitionError {
    #[error("invalid transition from '{from}' to '{to}'")]
    InvalidTransition { from: String, to: String },
    #[error("unknown state: '{0}'")]
    UnknownState(String),
}

/// Programmable state machine for any asset type.
///
/// System assets map states 1:1 to [`BaseState`]. User-defined assets
/// (e.g. a Car: Designed->Manufacturing->QA->Shipped->InService->Retired)
/// define domain-specific states that each map to a BaseState.
pub trait AssetStatusTrait: Send + Sync {
    /// Domain-specific state name (e.g. "Manufacturing", "QA").
    fn current_state(&self) -> &str;
    /// The infrastructure-level state this maps to.
    fn base_state(&self) -> BaseState;
    /// Whether a transition to `target` is allowed from the current state.
    fn can_transition_to(&self, target: &str) -> bool;
    /// Execute the transition. Fails if not allowed.
    fn transition(&mut self, target: &str) -> Result<(), StatusTransitionError>;
    /// States reachable from the current state.
    fn available_transitions(&self) -> Vec<&str>;
}

// ---------------------------------------------------------------------------
// Pillar 3: AssetAdapter — fully programmable runtime interface
// ---------------------------------------------------------------------------

/// Error from adapter operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AdapterError {
    #[error("validation failed: {0}")]
    ValidationFailed(String),
    #[error("invalid state for operation: {0}")]
    InvalidState(String),
    #[error("unsupported operation: {0}")]
    UnsupportedOperation(String),
    #[error("security violation: {0}")]
    SecurityViolation(String),
    #[error("adapter error: {0}")]
    Internal(String),
}

/// Result of validating asset data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationOutcome {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Result of inspecting asset integrity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectionReport {
    pub integrity_valid: bool,
    pub state_consistent: bool,
    pub findings: Vec<String>,
}

/// Describes a command the adapter can execute (mutating operation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDescriptor {
    pub name: String,
    pub description: String,
}

/// Describes a query the adapter supports (read-only operation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryDescriptor {
    pub name: String,
    pub description: String,
}

/// Self-description of what an adapter can do.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterCapabilities {
    /// Commands this adapter can execute.
    pub commands: Vec<CommandDescriptor>,
    /// Queries this adapter supports.
    pub queries: Vec<QueryDescriptor>,
    /// Whether this adapter supports streaming data.
    pub supports_streaming: bool,
    /// Whether this adapter can be composed with others.
    pub supports_composition: bool,
    /// Maximum input data size in bytes (None = unlimited).
    pub max_input_bytes: Option<u64>,
    /// System resource types this adapter depends on.
    pub resource_dependencies: Vec<SystemAssetKind>,
}

/// Fully programmable asset runtime interface.
///
/// Every asset in HyperMesh implements this trait. The adapter IS the asset's
/// behavior -- its executable logic, lifecycle hooks, and command interface.
///
/// # Security Model
///
/// The trait defines the INTERFACE. Security enforcement happens at the
/// runtime level (blockmatrix): state proofs before execution, resource
/// limits, sandboxing. The adapter declares its capabilities; the runtime
/// decides what to allow.
pub trait AssetAdapter: Send + Sync {
    // --- Identity ---

    /// The asset's two-level classification.
    fn kind(&self) -> &AssetKind;
    /// Unique identifier.
    fn id(&self) -> &AssetId;

    // --- Lifecycle hooks ---

    /// Called when the asset is first created/registered.
    fn on_create(&mut self) -> Result<(), AdapterError>;
    /// Called on every state transition (from AssetStatus).
    fn on_transition(&mut self, from: &str, to: &str) -> Result<(), AdapterError>;
    /// Called when the asset is destroyed/deregistered.
    fn on_destroy(&mut self) -> Result<(), AdapterError>;

    // --- Programmable interface ---

    /// Execute a mutating command with input data, returning output.
    fn execute(&mut self, command: &str, input: &[u8]) -> Result<Vec<u8>, AdapterError>;
    /// Run a read-only query with parameters, returning results.
    fn query(&self, query: &str, params: &[u8]) -> Result<Vec<u8>, AdapterError>;

    // --- Validation ---

    /// Validate asset data against this type's rules.
    fn validate(&self, data: &[u8]) -> Result<ValidationOutcome, AdapterError>;

    // --- Self-description ---

    /// Declare what this adapter can do.
    fn capabilities(&self) -> AdapterCapabilities;
}

/// Backward-compatibility alias. Prefer [`AssetAdapter`] in new code.
#[deprecated(note = "replaced by AssetAdapter")]
pub trait AssetDescriptor {
    /// The two-level classification of this asset.
    fn kind(&self) -> &AssetKind;
    /// Current lifecycle state.
    fn state(&self) -> BaseState;
    /// Unique identifier.
    fn id(&self) -> &AssetId;
}

// ---------------------------------------------------------------------------
// AssetMetadata — common metadata
// ---------------------------------------------------------------------------

/// Metadata common to all assets regardless of kind.
///
/// This is the canonical shared metadata. Domain-specific crates may embed
/// this struct or define their own richer metadata types that contain
/// an `AssetMetadata` field (e.g. catalog's `PackageMetadata`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetadata {
    /// Human-readable asset name.
    pub name: String,
    /// Optional longer description.
    pub description: Option<String>,
    /// Semantic version string (e.g. "1.0.0").
    pub version: String,
    /// Content hash of the asset payload.
    pub content_hash: ContentHash,
    /// Size of the asset payload in bytes.
    pub size_bytes: u64,
    /// Two-level asset classification.
    pub kind: AssetKind,
    /// Creation timestamp (UTC milliseconds since epoch).
    pub created_at: i64,
    /// Last-modified timestamp (UTC milliseconds since epoch).
    pub updated_at: i64,
    /// Free-form tags for discovery and filtering.
    pub tags: Vec<String>,
}

// ---------------------------------------------------------------------------
// GenesisAssetRecord — sovereign genesis (R1, R10)
// ---------------------------------------------------------------------------

/// Record of an asset created during genesis — hardware assessed and instantiated
/// as an IPv6-addressed asset with Proof of State (R1, R10).
///
/// Each device resource (CPU, GPU, RAM, Storage, Network, Transmission) gets a
/// genesis record when the node first boots and assesses its hardware.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAssetRecord {
    /// The asset's unique content-addressed identifier.
    pub asset_id: AssetId,
    /// The kind of system asset.
    pub kind: SystemAssetKind,
    /// IPv6-encoded asset address (matrix position + content hash).
    pub address: AssetAddress,
    /// Assessed hardware capabilities for this asset.
    pub capabilities: HardwareCapabilities,
    /// BLAKE3 hash of the genesis block that created this asset.
    pub genesis_block_hash: ContentHash,
    /// Timestamp of genesis assessment (UTC milliseconds since epoch).
    pub assessed_at: u64,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_asset_kind_type_ids() {
        let all = [
            SystemAssetKind::Cpu,
            SystemAssetKind::Gpu,
            SystemAssetKind::Memory,
            SystemAssetKind::Storage,
            SystemAssetKind::Network,
            SystemAssetKind::Container,
            SystemAssetKind::Economic,
            SystemAssetKind::Blockchain,
            SystemAssetKind::Dns,
            SystemAssetKind::Transmission,
        ];

        // Verify expected IDs
        assert_eq!(all[0].type_id(), 0, "Cpu");
        assert_eq!(all[1].type_id(), 1, "Gpu");
        assert_eq!(all[2].type_id(), 2, "Memory");
        assert_eq!(all[3].type_id(), 3, "Storage");
        assert_eq!(all[4].type_id(), 4, "Network");
        assert_eq!(all[5].type_id(), 5, "Container");
        assert_eq!(all[6].type_id(), 6, "Economic");
        assert_eq!(all[7].type_id(), 7, "Blockchain");
        assert_eq!(all[8].type_id(), 8, "Dns");
        assert_eq!(all[9].type_id(), 9, "Transmission");

        // Verify uniqueness
        let mut ids: Vec<u8> = all.iter().map(|k| k.type_id()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 10, "All 10 type IDs must be unique");
    }

    #[test]
    fn test_asset_kind_serde_roundtrip() {
        // System variant
        let system = AssetKind::System(SystemAssetKind::Gpu);
        let json = serde_json::to_string(&system).expect("test: serialize System");
        let back: AssetKind = serde_json::from_str(&json).expect("test: deserialize System");
        assert_eq!(system, back);

        // UserDefined variant
        let user = AssetKind::UserDefined(UserAssetKind {
            type_name: "MyService".to_string(),
            type_hash: ContentHash::zeroed(),
        });
        let json = serde_json::to_string(&user).expect("test: serialize UserDefined");
        let back: AssetKind = serde_json::from_str(&json).expect("test: deserialize UserDefined");
        assert_eq!(user, back);
    }

    #[test]
    fn test_base_state_methods() {
        let states = [
            BaseState::Available,
            BaseState::Allocated,
            BaseState::InUse,
            BaseState::Suspended,
            BaseState::Maintenance,
            BaseState::Failed,
        ];

        // is_operational: Available, Allocated, InUse
        assert!(states[0].is_operational());
        assert!(states[1].is_operational());
        assert!(states[2].is_operational());
        assert!(!states[3].is_operational());
        assert!(!states[4].is_operational());
        assert!(!states[5].is_operational());

        // is_active: only InUse
        assert!(!states[0].is_active());
        assert!(!states[1].is_active());
        assert!(states[2].is_active());
        assert!(!states[3].is_active());
        assert!(!states[4].is_active());
        assert!(!states[5].is_active());

        // is_available: only Available
        assert!(states[0].is_available());
        assert!(!states[1].is_available());
        assert!(!states[2].is_available());
        assert!(!states[3].is_available());
        assert!(!states[4].is_available());
        assert!(!states[5].is_available());

        // priority: Failed=0 < Maintenance=1 < Suspended=2 < Available=3 < Allocated=4 < InUse=5
        assert_eq!(states[0].priority(), 3); // Available
        assert_eq!(states[1].priority(), 4); // Allocated
        assert_eq!(states[2].priority(), 5); // InUse
        assert_eq!(states[3].priority(), 2); // Suspended
        assert_eq!(states[4].priority(), 1); // Maintenance
        assert_eq!(states[5].priority(), 0); // Failed
    }

    #[test]
    fn test_asset_kind_display() {
        assert_eq!(
            AssetKind::System(SystemAssetKind::Cpu).to_string(),
            "System(Cpu)"
        );
        assert_eq!(
            AssetKind::System(SystemAssetKind::Dns).to_string(),
            "System(Dns)"
        );
        assert_eq!(
            AssetKind::UserDefined(UserAssetKind {
                type_name: "Widget".to_string(),
                type_hash: ContentHash::zeroed(),
            })
            .to_string(),
            "UserDefined(Widget)"
        );

        // SystemAssetKind Display
        assert_eq!(SystemAssetKind::Blockchain.to_string(), "Blockchain");
        assert_eq!(SystemAssetKind::Storage.to_string(), "Storage");
        assert_eq!(SystemAssetKind::Transmission.to_string(), "Transmission");

        // BaseState Display
        assert_eq!(BaseState::Available.to_string(), "Available");
        assert_eq!(BaseState::Suspended.to_string(), "Suspended");
        assert_eq!(BaseState::Failed.to_string(), "Failed");
    }

    #[test]
    fn test_asset_metadata_serde() {
        let meta = AssetMetadata {
            name: "test-asset".to_string(),
            description: Some("A test asset".to_string()),
            version: "1.0.0".to_string(),
            content_hash: ContentHash::from_bytes([42u8; 32]),
            size_bytes: 1024,
            kind: AssetKind::System(SystemAssetKind::Storage),
            created_at: 1_700_000_000_000,
            updated_at: 1_700_000_001_000,
            tags: vec!["test".to_string(), "example".to_string()],
        };

        let json = serde_json::to_string(&meta).expect("test: serialize AssetMetadata");
        let back: AssetMetadata =
            serde_json::from_str(&json).expect("test: deserialize AssetMetadata");

        assert_eq!(back.name, "test-asset");
        assert_eq!(back.description.as_deref(), Some("A test asset"));
        assert_eq!(back.version, "1.0.0");
        assert_eq!(back.content_hash, ContentHash::from_bytes([42u8; 32]));
        assert_eq!(back.size_bytes, 1024);
        assert_eq!(back.kind, AssetKind::System(SystemAssetKind::Storage));
        assert_eq!(back.created_at, 1_700_000_000_000);
        assert_eq!(back.updated_at, 1_700_000_001_000);
        assert_eq!(back.tags, vec!["test", "example"]);
    }

    #[test]
    fn test_adapter_capabilities_serde() {
        let caps = AdapterCapabilities {
            commands: vec![CommandDescriptor {
                name: "process".into(),
                description: "Process the asset".into(),
            }],
            queries: vec![QueryDescriptor {
                name: "status".into(),
                description: "Get current status".into(),
            }],
            supports_streaming: true,
            supports_composition: false,
            max_input_bytes: Some(1024 * 1024),
            resource_dependencies: vec![SystemAssetKind::Cpu, SystemAssetKind::Memory],
        };
        let json = serde_json::to_string(&caps).expect("test: serialize capabilities");
        let back: AdapterCapabilities =
            serde_json::from_str(&json).expect("test: deserialize capabilities");
        assert_eq!(back.commands.len(), 1);
        assert_eq!(back.queries.len(), 1);
        assert!(back.supports_streaming);
        assert_eq!(back.resource_dependencies.len(), 2);
    }

    #[test]
    fn test_validation_outcome_serde() {
        let outcome = ValidationOutcome {
            valid: false,
            errors: vec!["missing field: vin".into()],
            warnings: vec!["deprecated field: model_year".into()],
        };
        let json = serde_json::to_string(&outcome).expect("test: serialize");
        let back: ValidationOutcome = serde_json::from_str(&json).expect("test: deserialize");
        assert!(!back.valid);
        assert_eq!(back.errors.len(), 1);
        assert_eq!(back.warnings.len(), 1);
    }
}
