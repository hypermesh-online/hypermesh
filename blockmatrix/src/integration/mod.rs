//! HyperMesh Integration Layer
//!
//! This module provides integration services for connecting HyperMesh
//! with external systems and protocols.

// REMOVED: HTTP API bridge (replaced with STOQ)
// pub mod api_bridge;
pub mod stoq_bridge;
pub mod bootstrap;
pub mod config;
pub mod coordinator;
pub mod lifecycle;
pub mod metrics;
pub mod services;
pub mod benches;
pub mod tests;

// Re-export main types from stoq_bridge instead
pub use stoq_bridge::{
    UnifiedApiBridge,
    ApiConfig,
    ServiceInfo,
    EndpointInfo,
    AssetRequest,
    AssetResponse,
    CertificateRequest,
    CertificateResponse,
    TransactionRequest,
    TransactionResponse,
};

pub use bootstrap::{
    BootstrapManager,
    BootstrapConfig,
    BootstrapPhase,
    ComponentState,
    ComponentStatus,
    ServiceDiscovery,
    CertificateProvider,
    TransportProvider,
    ConsensusProvider,
};

pub use config::{
    IntegrationConfig,
};

pub use self::coordinator::{
    IntegrationCoordinator,
};

// Additional types for integration
pub use crate::assets::core::adapter::AssetAdapter;

/// Blockchain integration trait
pub trait BlockchainIntegration: Send + Sync {
    /// Get blockchain name
    fn name(&self) -> &str;

    /// Check if blockchain is connected
    fn is_connected(&self) -> bool;
}

/// P2P router trait
pub trait P2PRouter: Send + Sync {
    /// Route message to peer
    fn route(&self, peer_id: &str, message: &[u8]) -> anyhow::Result<()>;

    /// Get peer count
    fn peer_count(&self) -> usize;
}

/// Integration manager
pub struct IntegrationManager {
    /// Placeholder
    _private: (),
}

impl IntegrationManager {
    /// Create new integration manager
    pub fn new() -> Self {
        Self { _private: () }
    }
}

// Common types for integration
pub type NodeId = String;
pub type ServiceId = String;

// MFN Bridge types (temporary exports for compatibility)
// These are actually in orchestration/integration/mfn_bridge.rs
pub struct MfnBridge;
pub struct MfnOperation;
pub struct LayerResponse;

pub use self::lifecycle::{
    LifecycleManager,
};

pub use self::metrics::{
    IntegrationMetrics,
};

pub use self::services::{
    ServiceRegistry,
};