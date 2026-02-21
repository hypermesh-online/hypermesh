// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Multi-Path QUIC Transport
//!
//! Logical multi-path QUIC via multiple independent QUIC connections.
//! Provides scope/privacy/federation constraint enforcement at the
//! connection management layer.
//!
//! Quinn does not support native QUIC-LR multi-path, so this module
//! manages multiple independent QUIC connections as logical "paths"
//! with policy-driven validation, scheduling, and federation boundary
//! enforcement.

pub mod connection;
pub mod policy;
pub mod scheduler;

pub use connection::{MultiPathConnection, MultiPathMetrics, PathInfo, PathSnapshot};
pub use policy::{
    FederationPolicy, PathPolicy, PathRejectionReason, PathValidation, PosValidationLevel,
    PrivacyPolicy, ScopePolicy, SendContext,
};
pub use scheduler::{PathCandidate, PathScheduler, PathSelector};
