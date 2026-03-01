// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Privacy-preserving network metrics streaming for HyperMesh routing intelligence.
//!
//! # Modules
//!
//! - [`protocol`] -- MetricsFrame wire format and payload types.
//! - [`privacy_filter`] -- Differential privacy via Laplace noise injection.
//! - [`publisher`] -- Local metrics collection and frame production.
//! - [`subscriber`] -- Rolling-window frame reception per source node.
//! - [`aggregator`] -- Multi-node regional aggregation for routing decisions.

pub mod aggregator;
pub mod privacy_filter;
pub mod protocol;
pub mod publisher;
pub mod subscriber;

// Re-export primary types at module root.
pub use aggregator::{RegionalAggregate, RegionalAggregator};
pub use privacy_filter::DifferentialPrivacyFilter;
pub use protocol::{
    CapacitySnapshot, CongestionSnapshot, EconomicSnapshot, MetricsFrame, MetricsPayload,
    ProtocolError, RoutingSnapshot, VerificationSnapshot,
};
pub use publisher::MetricsPublisher;
pub use subscriber::MetricsSubscriber;
