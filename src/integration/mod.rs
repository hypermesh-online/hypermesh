//! HyperMesh Integration Layer
//!
//! This module provides integration services for connecting HyperMesh
//! with external systems and protocols.

pub mod api_bridge;
pub mod bootstrap;
pub mod config;
pub mod coordinator;
pub mod lifecycle;
pub mod metrics;
pub mod services;
pub mod benches;
pub mod tests;

// Re-export main types
pub use api_bridge::{
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

pub use coordinator::{
    IntegrationCoordinator,
};

// Common types for integration
pub type NodeId = String;
pub type ServiceId = String;

// MFN Bridge types (temporary exports for compatibility)
// These are actually in orchestration/integration/mfn_bridge.rs
pub struct MfnBridge;
pub struct MfnOperation;
pub struct LayerResponse;

pub use lifecycle::{
    LifecycleManager,
};

pub use metrics::{
    IntegrationMetrics,
};

pub use services::{
    ServiceRegistry,
};