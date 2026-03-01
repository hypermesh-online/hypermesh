// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Container System
//!
//! This module provides container runtime and orchestration capabilities
//! integrated with the HyperMesh asset management system.

pub mod config;
pub mod error;
pub mod filesystem;
pub mod image;
pub mod lifecycle;
pub mod migration;
pub mod monitoring;
pub mod network;
pub mod process;
pub mod resources;
pub mod runtime;
pub mod types;

// Re-export main types
pub use runtime::{ContainerRuntime, RuntimeMetrics};

pub use config::ContainerConfig;

// Re-export from types module
pub use types::{ContainerHandle, ContainerId, ContainerSpec, ContainerStatus};

pub use types::{
    ContainerState, CreateOptions, ResourceLimits, ResourceRequirements, ResourceUsage,
};

pub use error::ContainerError;

pub use self::resources::{ResourceAllocation, ResourceConstraints};

pub use image::{ContainerImage, ImageManager};

pub use network::NetworkConfig;

pub use monitoring::ContainerMetrics;
