// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Integration Layer
//!
//! This module provides integration services for connecting HyperMesh
//! with external systems and protocols.

// REMOVED: HTTP API bridge (replaced with STOQ)
// pub mod api_bridge;
pub mod bootstrap;
pub mod config;
pub mod coordinator;
pub mod lifecycle;
pub mod metrics;
pub mod services;
pub mod stoq_bridge;

pub mod phase1_foundation;

// Re-export main types from stoq_bridge instead
pub use stoq_bridge::{
    ApiConfig, AssetRequest, AssetResponse, CertificateRequest, CertificateResponse, EndpointInfo,
    ServiceInfo, TransactionRequest, TransactionResponse, UnifiedApiBridge,
};

use async_trait::async_trait;
pub use bootstrap::{
    BootstrapConfig, BootstrapManager, BootstrapPhase, CertificateProvider, ComponentState,
    ComponentStatus, ConsensusProvider, ServiceDiscovery, TransportProvider,
};

pub use config::IntegrationConfig;

pub use self::coordinator::IntegrationCoordinator;

// Error types for integration layer
#[derive(Debug, thiserror::Error)]
pub enum IntegrationError {
    /// Component initialization failed
    #[error("Component {component} initialization failed: {message}")]
    ComponentInit { component: String, message: String },

    /// Component communication failure
    #[error("Communication failure between {source_component} and {target}: {message}")]
    ComponentCommunication {
        source_component: String,
        target: String,
        message: String,
    },

    /// Configuration validation error
    #[error("Configuration validation failed: {message}")]
    ConfigValidation { message: String },

    /// Platform lifecycle error
    #[error("Platform lifecycle error in {phase}: {message}")]
    Lifecycle { phase: String, message: String },

    /// Service registry error
    #[error("Service registry operation failed: {message}")]
    ServiceRegistry { message: String },

    /// Underlying component error
    #[error("Component error: {0}")]
    Component(#[from] anyhow::Error),
}

// Additional types for integration
pub use crate::assets::core::adapter::AssetAdapter;

/// Blockchain integration trait
#[async_trait]
pub trait BlockchainIntegration: Send + Sync {
    /// Get blockchain name
    fn name(&self) -> &str;

    /// Check if blockchain is connected
    fn is_connected(&self) -> bool;
}

/// P2P router trait
#[async_trait]
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

impl Default for IntegrationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl IntegrationManager {
    /// Create new integration manager
    pub fn new() -> Self {
        Self { _private: () }
    }
}

// Common types for integration
pub use crate::ServiceId;
pub use hypermesh_lib::NodeId;

pub use self::lifecycle::LifecycleManager;

pub use self::metrics::IntegrationMetrics;

pub use self::services::ServiceRegistry;

// Phase 1 Foundation - Sprint 1.6 Integration Layer
pub use self::phase1_foundation::{
    MatrixFoundation, MatrixFoundationConfig, MatrixNode, NetworkStats, Phase1Error, Phase1Result,
};
