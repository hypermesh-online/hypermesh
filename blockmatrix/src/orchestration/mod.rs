// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Orchestration System
//!
//! This module provides orchestration capabilities for containers and services
//! within the HyperMesh distributed computing platform.

pub mod hypermesh_integration;
pub mod container;
// integration module removed - was MFN bridge simulation layer
pub mod service_mesh;

// Re-export main types
pub use hypermesh_integration::{
    HyperMeshContainerOrchestrator, HyperMeshContainerSpec,
    ContainerDeploymentResult, OrchestrationMetrics,
    HyperMeshIntegrationConfig,
};

pub use container::{
    ContainerOrchestrator,
};

pub use crate::integration::{
    IntegrationManager,
};

pub use service_mesh::{
    ServiceMesh,
};