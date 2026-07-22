// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Caesar Economic Mode -- opt-in resource leasing and content push for
//! the HyperMesh capacity marketplace.
//!
//! # Modules
//!
//! - [`resource_pool`] -- Node operator resource allocation configuration.
//! - [`lease_contract`] -- Time-bounded resource access agreements.
//! - [`lease_manager`] -- Active lease management and supply/demand matching.
//! - [`pricing`] -- Governor-adjusted pricing engine.
//! - [`content_push`] -- Paid content distribution with explicit opt-in.

pub mod content_push;
pub mod lease_contract;
pub mod lease_manager;
pub mod pricing;
pub mod resource_pool;

pub use content_push::{ContentPushManager, ContentPushRequest, PushError, PushStatus};
pub use lease_contract::{LeaseContract, LeaseError, LeaseState};
pub use lease_manager::{LeaseManager, ManagerError};
pub use pricing::PricingEngine;
pub use resource_pool::{AllocationConfig, LeaseableResource, PoolError, ResourcePool};
