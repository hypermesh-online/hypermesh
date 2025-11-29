//! HyperMesh Container System
//!
//! This module provides container runtime and orchestration capabilities
//! integrated with the HyperMesh asset management system.

pub mod runtime;
pub mod config;
pub mod lifecycle;
pub mod resources;
pub mod image;
pub mod network;
pub mod filesystem;
pub mod monitoring;
pub mod migration;
pub mod error;
pub mod types;
pub mod tests;

// Re-export main types
pub use runtime::{
    ContainerRuntime, RuntimeMetrics,
};

pub use config::{
    ContainerConfig,
};

// Re-export from types module
pub use types::{
    ContainerId, ContainerSpec, ContainerStatus, ContainerHandle,
};

pub use types::{
    CreateOptions, ContainerState, ResourceRequirements, ResourceLimits, ResourceUsage,
};

pub use error::{
    ContainerError,
};

pub use self::resources::{
    ResourceAllocation, ResourceConstraints,
};

pub use image::{
    ContainerImage, ImageManager,
};

pub use network::{
    NetworkConfig,
};

pub use monitoring::{
    ContainerMetrics,
};