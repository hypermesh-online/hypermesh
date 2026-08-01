// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase 2 Intelligence Layer Integration
//!
//! This module integrates all Phase 2 revolutionary concepts into a unified
//! intelligence layer for BlockMatrix. It brings together:
//!
//! - Sprint 2.1: STOQ Protocol Intelligence (PoS validation at protocol level)
//! - Sprint 2.2: Four Privacy Tiers (Anonymous, Private P2P, Federated, Public)
//! - Sprint 2.3: Multi-Network Participation (isolated networks with cross-validation)
//! - Sprint 2.4: Asset Pipeline (compression → encryption → sharding → distribution)
//! - Sprint 2.5: Content-Addressed Storage (hash buckets with O(1) deduplication)
//!
//! ## Architecture
//!
//! The IntelligenceLayer orchestrates all Phase 2 components to provide:
//! - Intelligent asset processing with privacy-aware pipeline configuration
//! - Multi-network asset distribution with complete isolation
//! - Content deduplication across networks while maintaining privacy
//! - STOQ protocol integration for matrix-aware routing
//! - End-to-end asset lifecycle management

// Sub-modules for integration layer
#[cfg(feature = "intelligence")]
pub mod ebpf_feedback;
pub mod ngauge_bridge;
#[cfg(feature = "intelligence")]
pub mod ngauge_trust_adapter;
pub mod integration;
pub mod metrics_bridge;
pub mod performance;
pub mod types;
pub mod validation;
pub mod workflows;

#[cfg(feature = "intelligence")]
pub use ebpf_feedback::EbpfFeedbackAdapter;
#[cfg(feature = "intelligence")]
pub use ngauge_trust_adapter::NGaugeTrustAdapter;

// Re-exports from types module
pub use types::{AssetHandle, IntelligenceLayerConfig, IntelligenceMetrics};

// Re-exports for external use
pub use integration::{
    ComponentIntegration, ComponentStatus, HealthCheck, IntegrationConfig, IntegrationHealth,
};
pub use performance::{
    LatencyMetrics, PerformanceMetrics, PerformanceMonitor, PerformanceReport, ThroughputMetrics,
};
pub use validation::{
    ComponentValidation, E2EValidation, IntegrationValidator, ValidationReport, ValidationResult,
};
pub use workflows::{
    AssetWorkflow, ProcessingWorkflow, RetrievalWorkflow, WorkflowMetrics, WorkflowResult,
};
