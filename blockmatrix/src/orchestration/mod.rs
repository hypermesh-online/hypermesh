//! HyperMesh Orchestration System
//!
//! This module provides orchestration capabilities for containers and services
//! within the HyperMesh distributed computing platform.

pub mod hypermesh_integration;
pub mod container;
pub mod integration;
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